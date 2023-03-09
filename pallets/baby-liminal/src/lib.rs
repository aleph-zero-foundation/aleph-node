#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod systems;
#[cfg(test)]
mod tests;
mod weights;

use frame_support::pallet_prelude::StorageVersion;
use frame_system::ensure_root;
pub use pallet::*;
pub use systems::{ProvingSystem, VerificationError};
pub use weights::{AlephWeight, WeightInfo};

/// The current storage version.
const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

/// We store verification keys under short identifiers.
pub type VerificationKeyIdentifier = [u8; 4];

#[frame_support::pallet]
pub mod pallet {
    use ark_serialize::CanonicalDeserialize;
    use frame_support::{
        dispatch::PostDispatchInfo, log, pallet_prelude::*, sp_runtime::DispatchErrorWithPostInfo,
    };
    use frame_system::pallet_prelude::OriginFor;
    use sp_std::prelude::Vec;

    use super::*;
    use crate::systems::{Gm17, Groth16, Marlin, VerificationError, VerifyingSystem};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;

        /// Limits how many bytes verification key can have.
        ///
        /// Verification keys are stored, therefore this is separated from the limits on proof or
        /// public input.
        #[pallet::constant]
        type MaximumVerificationKeyLength: Get<u32>;

        /// Limits how many bytes proof or public input can have.
        #[pallet::constant]
        type MaximumDataLength: Get<u32>;
    }

    #[pallet::error]
    #[derive(Clone, Eq, PartialEq)]
    pub enum Error<T> {
        /// This verification key identifier is already taken.
        IdentifierAlreadyInUse,
        /// There is no verification key available under this identifier.
        UnknownVerificationKeyIdentifier,
        /// Provided verification key is longer than `MaximumVerificationKeyLength` limit.
        VerificationKeyTooLong,

        /// Either proof or public input is longer than `MaximumDataLength` limit.
        DataTooLong,
        /// Couldn't deserialize proof.
        DeserializingProofFailed,
        /// Couldn't deserialize public input.
        DeserializingPublicInputFailed,
        /// Couldn't deserialize verification key from storage.
        DeserializingVerificationKeyFailed,
        /// Verification procedure has failed. Proof still can be correct.
        VerificationFailed(VerificationError),
        /// Proof has been found as incorrect.
        IncorrectProof,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Verification key has been successfully stored.
        VerificationKeyStored,

        /// Verification key has been successfully deleted.
        VerificationKeyDeleted,

        /// Verification key has been successfully overwritten.
        VerificationKeyOverwritten,

        /// Proof has been successfully verified.
        VerificationSucceeded,
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type VerificationKeys<T: Config> = StorageMap<
        _,
        Twox64Concat,
        VerificationKeyIdentifier,
        BoundedVec<u8, T::MaximumVerificationKeyLength>,
    >;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Stores `key` under `identifier` in `VerificationKeys` map.
        ///
        /// Fails if:
        /// - `key.len()` is greater than `MaximumVerificationKeyLength`, or
        /// - `identifier` has been already used
        ///
        /// `key` can come from any proving system - there are no checks that verify it, in
        /// particular, `key` can contain just trash bytes.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::store_key(key.len() as u32))]
        pub fn store_key(
            _origin: OriginFor<T>,
            identifier: VerificationKeyIdentifier,
            key: Vec<u8>,
        ) -> DispatchResult {
            Self::bare_store_key(identifier, key).map_err(|e| e.into())
        }

        /// Deletes a key stored under `identifier` in `VerificationKeys` map.
        ///
        /// Can only be called by a root account.
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn delete_key(
            origin: OriginFor<T>,
            identifier: VerificationKeyIdentifier,
        ) -> DispatchResult {
            ensure_root(origin)?;
            VerificationKeys::<T>::remove(identifier);
            Self::deposit_event(Event::VerificationKeyDeleted);
            Ok(())
        }

        /// Overwrites a key stored under `identifier` in `VerificationKeys` map with a new value `key`
        ///
        /// Fails if `key.len()` is greater than `MaximumVerificationKeyLength`.
        /// Can only be called by a root account.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::overwrite_key(key.len() as u32))]
        pub fn overwrite_key(
            origin: OriginFor<T>,
            identifier: VerificationKeyIdentifier,
            key: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(
                key.len() <= T::MaximumVerificationKeyLength::get() as usize,
                Error::<T>::VerificationKeyTooLong
            );

            VerificationKeys::<T>::try_mutate_exists(identifier, |value| -> DispatchResult {
                // should never fail, since length is checked above
                *value = Some(BoundedVec::try_from(key).unwrap());

                Ok(())
            })?;

            Self::deposit_event(Event::VerificationKeyOverwritten);
            Ok(())
        }

        /// Verifies `proof` against `public_input` with a key that has been stored under
        /// `verification_key_identifier`. All is done within `system` proving system.
        ///
        /// Fails if:
        /// - there is no verification key under `verification_key_identifier`
        /// - verification key under `verification_key_identifier` cannot be deserialized
        /// (e.g. it has been produced for another proving system)
        /// - `proof` cannot be deserialized (e.g. it has been produced for another proving system)
        /// - `public_input` cannot be deserialized (e.g. it has been produced for another proving
        /// system)
        /// - verifying procedure fails (e.g. incompatible verification key and proof)
        /// - proof is incorrect
        #[pallet::weight(
            match system {
                ProvingSystem::Groth16 => T::WeightInfo::verify_groth16(),
                ProvingSystem::Gm17 => T::WeightInfo::verify_gm17(),
                ProvingSystem::Marlin => T::WeightInfo::verify_marlin(),
            }
        )]
        #[pallet::call_index(3)]
        pub fn verify(
            _origin: OriginFor<T>,
            verification_key_identifier: VerificationKeyIdentifier,
            proof: Vec<u8>,
            public_input: Vec<u8>,
            system: ProvingSystem,
        ) -> DispatchResultWithPostInfo {
            Self::bare_verify(verification_key_identifier, proof, public_input, system)
                .map(|_| ().into())
                .map_err(|(error, actual_weight)| DispatchErrorWithPostInfo {
                    post_info: PostDispatchInfo {
                        pays_fee: Pays::Yes,
                        actual_weight,
                    },
                    error: error.into(),
                })
        }
    }

    impl<T: Config> Pallet<T> {
        /// This is the inner logic behind `Self::store_key`, however it is free from account lookup
        /// or other dispatchable-related overhead. Thus, it is more suited to call directly from
        /// runtime, like from a chain extension.
        pub fn bare_store_key(
            identifier: VerificationKeyIdentifier,
            key: Vec<u8>,
        ) -> Result<(), Error<T>> {
            ensure!(
                key.len() <= T::MaximumVerificationKeyLength::get() as usize,
                Error::<T>::VerificationKeyTooLong
            );

            ensure!(
                !VerificationKeys::<T>::contains_key(identifier),
                Error::<T>::IdentifierAlreadyInUse
            );

            VerificationKeys::<T>::insert(
                identifier,
                BoundedVec::try_from(key).unwrap(), // must succeed since we've just check length
            );

            Self::deposit_event(Event::VerificationKeyStored);
            Ok(())
        }

        /// This is the inner logic behind `Self::verify`, however it is free from account lookup
        /// or other dispatchable-related overhead. Thus, it is more suited to call directly from
        /// runtime, like from a chain extension.
        pub fn bare_verify(
            verification_key_identifier: VerificationKeyIdentifier,
            proof: Vec<u8>,
            public_input: Vec<u8>,
            system: ProvingSystem,
        ) -> Result<(), (Error<T>, Option<Weight>)> {
            match system {
                ProvingSystem::Groth16 => {
                    Self::_bare_verify::<Groth16>(verification_key_identifier, proof, public_input)
                }
                ProvingSystem::Gm17 => {
                    Self::_bare_verify::<Gm17>(verification_key_identifier, proof, public_input)
                }
                ProvingSystem::Marlin => {
                    Self::_bare_verify::<Marlin>(verification_key_identifier, proof, public_input)
                }
            }
        }

        fn _bare_verify<S: VerifyingSystem>(
            verification_key_identifier: VerificationKeyIdentifier,
            proof: Vec<u8>,
            public_input: Vec<u8>,
        ) -> Result<(), (Error<T>, Option<Weight>)> {
            let data_length_limit = T::MaximumDataLength::get() as usize;
            let data_length_excess = proof.len().saturating_sub(data_length_limit)
                + public_input.len().saturating_sub(data_length_limit);
            ensure!(
                data_length_excess == 0,
                (
                    Error::<T>::DataTooLong,
                    Some(T::WeightInfo::verify_data_too_long(
                        data_length_excess as u32
                    ))
                )
            );

            let proof_len = proof.len() as u32;
            let proof: S::Proof = CanonicalDeserialize::deserialize(&*proof).map_err(|e| {
                log::error!("Deserializing proof failed: {:?}", e);
                (
                    Error::<T>::DeserializingProofFailed,
                    Some(T::WeightInfo::verify_data_deserializing_fails(proof_len)),
                )
            })?;

            let public_input: Vec<S::CircuitField> =
                CanonicalDeserialize::deserialize(&*public_input).map_err(|e| {
                    log::error!("Deserializing public input failed: {:?}", e);
                    (
                        Error::<T>::DeserializingPublicInputFailed,
                        Some(T::WeightInfo::verify_data_deserializing_fails(
                            proof_len + public_input.len() as u32,
                        )),
                    )
                })?;

            let verification_key =
                VerificationKeys::<T>::get(verification_key_identifier).ok_or((
                    Error::<T>::UnknownVerificationKeyIdentifier,
                    Some(T::WeightInfo::verify_key_deserializing_fails(0)),
                ))?;
            let verification_key: S::VerifyingKey =
                CanonicalDeserialize::deserialize(&**verification_key).map_err(|e| {
                    log::error!("Deserializing verification key failed: {:?}", e);
                    (
                        Error::<T>::DeserializingVerificationKeyFailed,
                        Some(T::WeightInfo::verify_key_deserializing_fails(
                            verification_key.len() as u32,
                        )),
                    )
                })?;

            let parent_hash = <frame_system::Pallet<T>>::parent_hash();
            let valid_proof = S::verify(
                &verification_key,
                &public_input,
                &proof,
                parent_hash.as_ref(),
            )
            .map_err(|err| (Error::<T>::VerificationFailed(err), None))?;

            ensure!(valid_proof, (Error::<T>::IncorrectProof, None));

            Self::deposit_event(Event::VerificationSucceeded);
            Ok(())
        }
    }
}
