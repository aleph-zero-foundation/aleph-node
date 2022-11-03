#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod weights;
use frame_support::pallet_prelude::StorageVersion;
pub use pallet::*;
pub use weights::{AlephWeight, WeightInfo};

/// The current storage version.
const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

/// We store verification keys under short identifiers.
pub type VerificationKeyIdentifier = [u8; 4];

mod systems;
pub use systems::ProvingSystem;

#[frame_support::pallet]
pub mod pallet {
    use ark_serialize::CanonicalDeserialize;
    use frame_support::{log, pallet_prelude::*};
    use frame_system::pallet_prelude::OriginFor;
    use sp_std::prelude::Vec;

    use super::*;
    use crate::systems::{Gm17, Groth16, Marlin, VerifyingSystem};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type MaximumVerificationKeyLength: Get<u32>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// This verification key identifier is already taken.
        IdentifierAlreadyInUse,
        /// There is no verification key available under this identifier.
        UnknownVerificationKeyIdentifier,
        /// Provided verification key is longer than `MaximumVerificationKeyLength` limit.
        VerificationKeyTooLong,
        /// Couldn't deserialize proof.
        DeserializingProofFailed,
        /// Couldn't deserialize public input.
        DeserializingPublicInputFailed,
        /// Couldn't deserialize verification key from storage.
        DeserializingVerificationKeyFailed,
        /// Verification procedure has failed. Proof still can be correct.
        VerificationFailed,
        /// Proof has been found as incorrect.
        IncorrectProof,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Verification key has been successfully stored.
        VerificationKeyStored,
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
        #[pallet::weight(T::WeightInfo::store_key(key.len() as u32))]
        pub fn store_key(
            _origin: OriginFor<T>,
            identifier: VerificationKeyIdentifier,
            key: Vec<u8>,
        ) -> DispatchResult {
            Self::bare_store_key(identifier, key).map_err(|e| e.into())
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
        #[pallet::weight(T::WeightInfo::verify())]
        pub fn verify(
            _origin: OriginFor<T>,
            verification_key_identifier: VerificationKeyIdentifier,
            proof: Vec<u8>,
            public_input: Vec<u8>,
            system: ProvingSystem,
        ) -> DispatchResult {
            Self::bare_verify(verification_key_identifier, proof, public_input, system)
                .map_err(|e| e.into())
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
        ) -> Result<(), Error<T>> {
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
        ) -> Result<(), Error<T>> {
            let proof: S::Proof = CanonicalDeserialize::deserialize(&*proof).map_err(|e| {
                log::error!("Deserializing proof failed: {:?}", e);
                Error::<T>::DeserializingProofFailed
            })?;

            let public_input: Vec<S::CircuitField> =
                CanonicalDeserialize::deserialize(&*public_input).map_err(|e| {
                    log::error!("Deserializing public input failed: {:?}", e);
                    Error::<T>::DeserializingPublicInputFailed
                })?;

            let verification_key = VerificationKeys::<T>::get(verification_key_identifier)
                .ok_or(Error::<T>::UnknownVerificationKeyIdentifier)?;
            let verification_key: S::VerifyingKey =
                CanonicalDeserialize::deserialize(&**verification_key).map_err(|e| {
                    log::error!("Deserializing verification key failed: {:?}", e);
                    Error::<T>::DeserializingVerificationKeyFailed
                })?;

            // At some point we should enhance error type from `S::verify` and be more verbose here.
            let valid_proof = S::verify(&verification_key, &public_input, &proof)
                .map_err(|_| Error::<T>::VerificationFailed)?;

            ensure!(valid_proof, Error::<T>::IncorrectProof);

            Self::deposit_event(Event::VerificationSucceeded);
            Ok(())
        }
    }
}
