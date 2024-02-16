//! # VK Storage pallet
//!
//! This pallet provides a way to store verification keys that can be used in the SNARK verification process. Anybody
//! can register a verification key. A key is stored in a map under its Blake256 hash. Pallet doesn't provide any way
//! for removing keys from the map, so it's a good idea to impose some costs on storing a key (see `StorageCharge`) to
//! avoid bloating the storage.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod tests;
mod weights;

use frame_support::{
    dispatch::PostDispatchInfo,
    pallet_prelude::{StorageVersion, Weight},
    sp_runtime::traits::BlakeTwo256,
};
pub use pallet::*;
use parity_scale_codec::{Decode, Encode};
use sp_core::H256;
pub use weights::{AlephWeight, WeightInfo};

/// Hashing algorithm used for computing key hashes.
pub type KeyHasher = BlakeTwo256;
/// Hash type used for storing keys.
pub type KeyHash = H256;

/// The current storage version.
const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, sp_runtime::traits::Hash};
    use frame_system::{ensure_signed, pallet_prelude::OriginFor};
    use sp_std::prelude::Vec;

    use super::*;
    use crate::StorageCharge;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Item required for emitting events.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for the pallet's extrinsics.
        type WeightInfo: WeightInfo;

        /// Limits how many bytes verification key can have.
        #[pallet::constant]
        type MaximumKeyLength: Get<u32>;

        /// The policy on charging for storing a key (in addition to the standard operation costs).
        #[pallet::constant]
        type StorageCharge: Get<StorageCharge>;
    }

    #[pallet::error]
    #[derive(Clone, Eq, PartialEq)]
    pub enum Error<T> {
        /// Provided verification key is longer than `MaximumKeyLength` limit.
        VerificationKeyTooLong,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Verification key has been successfully stored.
        VerificationKeyStored(KeyHash),
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type VerificationKeys<T: Config> =
        StorageMap<_, Twox64Concat, KeyHash, BoundedVec<u8, T::MaximumKeyLength>>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Stores `key` under its Blake256 hash in `VerificationKeys` map.
        ///
        /// # Errors
        ///
        /// This call will return an error if `key.len()` is greater than `MaximumKeyLength` limit.
        ///
        /// # Notes
        ///
        /// 1. `key` can come from any proving system - there are no checks that verify it, in
        /// particular, `key` can contain just trash bytes.
        ///
        /// 2. If the key is already stored, this call will succeed and charge the full weight, even though the whole
        /// work could have been avoided.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::store_key(key.len() as u32))]
        pub fn store_key(origin: OriginFor<T>, key: Vec<u8>) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            ensure!(
                key.len() <= T::MaximumKeyLength::get() as usize,
                Error::<T>::VerificationKeyTooLong
            );

            let hash = KeyHasher::hash(&key);
            let key_len = key.len();
            VerificationKeys::<T>::insert(
                hash,
                BoundedVec::try_from(key)
                    .expect("Key is already guaranteed to be within length limits."),
            );

            Self::deposit_event(Event::VerificationKeyStored(hash));
            Ok(PostDispatchInfo {
                actual_weight: Some(
                    T::WeightInfo::store_key(key_len as u32)
                        .saturating_add(T::StorageCharge::get().charge_for(key_len)),
                ),
                pays_fee: Pays::Yes,
            })
        }
    }
}

/// A simple linear model for charging for storing a key.
///
/// This should be used to impose higher costs on storing anything in this pallet (since there is no way of clearing
/// the storage). The costs should be charged in addition to the standard operation costs (i.e., database costs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, scale_info::TypeInfo)]
pub struct StorageCharge {
    base: u64,
    per_byte: u64,
}

impl StorageCharge {
    /// Creates a new charge model of a fixed cost.
    pub const fn constant(base: u64) -> Self {
        Self { base, per_byte: 0 }
    }

    /// Creates a new charge model of a linear cost.
    pub const fn linear(base: u64, per_byte: u64) -> Self {
        Self { base, per_byte }
    }

    /// Computes the fee for storing `bytes` bytes.
    pub fn charge_for(&self, bytes: usize) -> Weight {
        Weight::from_parts(
            self.base
                .saturating_add(self.per_byte.saturating_mul(bytes as u64)),
            0,
        )
    }
}
