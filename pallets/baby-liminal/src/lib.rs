#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod tests;
mod weights;

use frame_support::{
    fail,
    pallet_prelude::StorageVersion,
    traits::{Currency, ReservableCurrency},
};
use frame_system::ensure_signed;
pub use pallet::*;
pub use weights::{AlephWeight, WeightInfo};

/// The current storage version.
const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

/// We store verification keys under short identifiers.
pub type VerificationKeyIdentifier = [u8; 8];
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::OriginFor;
    use sp_std::{
        cmp::Ordering::{Equal, Greater, Less},
        prelude::Vec,
    };

    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Limits how many bytes verification key can have.
        ///
        /// Verification keys are stored, therefore this is separated from the limits on proof or
        /// public input.
        #[pallet::constant]
        type MaximumVerificationKeyLength: Get<u32>;

        /// Deposit amount for storing a verification key
        ///
        /// Will get locked and returned upon deleting the key by the owner
        #[pallet::constant]
        type VerificationKeyDepositPerByte: Get<BalanceOf<Self>>;
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

        /// Unsigned request
        BadOrigin,
        /// User has insufficient funds to lock the deposit for storing verification key
        CannotAffordDeposit,
        /// Caller is not the owner of the key
        NotOwner,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Verification key has been successfully stored.
        ///
        /// \[ account_id, identifier \]
        VerificationKeyStored(T::AccountId, VerificationKeyIdentifier),

        /// Verification key has been successfully deleted.
        ///
        /// \[ identifier \]
        VerificationKeyDeleted(T::AccountId, VerificationKeyIdentifier),

        /// Verification key has been successfully overwritten.
        ///
        /// \[ identifier \]
        VerificationKeyOverwritten(VerificationKeyIdentifier),
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_verification_key)]
    pub type VerificationKeys<T: Config> = StorageMap<
        _,
        Twox64Concat,
        VerificationKeyIdentifier,
        BoundedVec<u8, T::MaximumVerificationKeyLength>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_verification_key_owner)]
    pub type VerificationKeyOwners<T: Config> =
        StorageMap<_, Twox64Concat, VerificationKeyIdentifier, T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn get_verification_key_deposit)]
    pub type VerificationKeyDeposits<T: Config> =
        StorageMap<_, Twox64Concat, (T::AccountId, VerificationKeyIdentifier), BalanceOf<T>>;

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
            origin: OriginFor<T>,
            identifier: VerificationKeyIdentifier,
            key: Vec<u8>,
        ) -> DispatchResult {
            Self::bare_store_key(origin, identifier, key).map_err(|e| e.into())
        }

        /// Deletes a key stored under `identifier` in `VerificationKeys` map.
        ///
        /// Returns the deposit locked. Can only be called by the key owner.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::delete_key(T::MaximumVerificationKeyLength::get()))]
        pub fn delete_key(
            origin: OriginFor<T>,
            identifier: VerificationKeyIdentifier,
        ) -> DispatchResult {
            let who = ensure_signed(origin).map_err(|_| Error::<T>::BadOrigin)?;
            let owner = VerificationKeyOwners::<T>::get(identifier)
                .ok_or(Error::<T>::UnknownVerificationKeyIdentifier)?;

            ensure!(who == owner, Error::<T>::NotOwner);

            let deposit = VerificationKeyDeposits::<T>::take((&owner, &identifier)).unwrap(); // cannot fail since the key has owner and owner must have made a deposit
            T::Currency::unreserve(&owner, deposit);

            VerificationKeys::<T>::remove(identifier);
            Self::deposit_event(Event::VerificationKeyDeleted(who, identifier));
            Ok(())
        }

        /// Overwrites a key stored under `identifier` in `VerificationKeys` map with a new value `key`
        ///
        /// Fails if `key.len()` is greater than `MaximumVerificationKeyLength`.
        /// Can only be called by the original owner of the key.
        /// It will require the caller to lock up additional funds (if the new key occupies more storage)
        /// or reimburse the difference if it is shorter in its byte-length.
        #[pallet::call_index(2)]
        #[pallet::weight(
            T::WeightInfo::overwrite_key(key.len() as u32)
                .max (T::WeightInfo::overwrite_equal_key(key.len() as u32))
        )]
        pub fn overwrite_key(
            origin: OriginFor<T>,
            identifier: VerificationKeyIdentifier,
            key: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin).map_err(|_| Error::<T>::BadOrigin)?;
            let owner = VerificationKeyOwners::<T>::get(identifier);

            match owner {
                Some(owner) => ensure!(who == owner, Error::<T>::NotOwner),
                None => fail!(Error::<T>::UnknownVerificationKeyIdentifier),
            };

            ensure!(
                key.len() <= T::MaximumVerificationKeyLength::get() as usize,
                Error::<T>::VerificationKeyTooLong
            );

            VerificationKeys::<T>::try_mutate_exists(identifier, |value| -> DispatchResult {
                // should never fail, since length is checked above
                *value = Some(BoundedVec::try_from(key.clone()).unwrap());
                Ok(())
            })?;

            VerificationKeyDeposits::<T>::try_mutate_exists(
                (&who, &identifier),
                |maybe_previous_deposit| -> DispatchResult {
                    let previous_deposit = maybe_previous_deposit
                        .ok_or(Error::<T>::UnknownVerificationKeyIdentifier)?;

                    let deposit = T::VerificationKeyDepositPerByte::get()
                        * BalanceOf::<T>::from(key.len() as u32);

                    match deposit.cmp(&previous_deposit) {
                        Less => {
                            // reimburse the prev - deposit difference
                            // we know that the caller is the owner because we have checked that
                            let difference = previous_deposit - deposit;
                            T::Currency::unreserve(&who, difference);
                            *maybe_previous_deposit = Some(deposit);
                        }
                        Equal => {
                            // do nothing
                        }
                        Greater => {
                            // lock the difference deposit - prev
                            let difference = deposit - previous_deposit;
                            T::Currency::reserve(&who, difference)
                                .map_err(|_| Error::<T>::CannotAffordDeposit)?;
                            *maybe_previous_deposit = Some(deposit);
                        }
                    };

                    Self::deposit_event(Event::VerificationKeyOverwritten(identifier));
                    Ok(())
                },
            )
        }
    }

    impl<T: Config> Pallet<T> {
        /// This is the inner logic behind `Self::store_key`, however it is free from account lookup
        /// or other dispatchable-related overhead. Thus, it is more suited to call directly from
        /// runtime, like from a chain extension.
        pub fn bare_store_key(
            origin: OriginFor<T>,
            identifier: VerificationKeyIdentifier,
            key: Vec<u8>,
        ) -> Result<(), Error<T>> {
            let who = ensure_signed(origin).map_err(|_| Error::<T>::BadOrigin)?;

            ensure!(
                key.len() <= T::MaximumVerificationKeyLength::get() as usize,
                Error::<T>::VerificationKeyTooLong
            );

            ensure!(
                !VerificationKeys::<T>::contains_key(identifier),
                Error::<T>::IdentifierAlreadyInUse
            );

            // make a locked deposit that will be returned when the key is deleted
            // deposit is calculated per byte of occupied storage
            let deposit =
                T::VerificationKeyDepositPerByte::get() * BalanceOf::<T>::from(key.len() as u32);
            T::Currency::reserve(&who, deposit).map_err(|_| Error::<T>::CannotAffordDeposit)?;

            VerificationKeys::<T>::insert(
                identifier,
                BoundedVec::try_from(key).unwrap(), // must succeed since we've just check length
            );

            // will never overwrite anything since we already check the VerificationKeys map
            VerificationKeyOwners::<T>::insert(identifier, &who);
            VerificationKeyDeposits::<T>::insert((&who, &identifier), deposit);

            Self::deposit_event(Event::VerificationKeyStored(who, identifier));
            Ok(())
        }
    }
}
