use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    log,
    pallet_prelude::{Get, TypeInfo},
    storage_alias,
    traits::OnRuntimeUpgrade,
    RuntimeDebug,
};
use pallet_transaction_payment::Config;
#[cfg(feature = "try-runtime")]
use pallet_transaction_payment::NextFeeMultiplier;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

use crate::Weight;

const TARGET: &str = "runtime::transaction_payment::migration";

#[storage_alias]
type StorageVersion = StorageValue<TransactionPayment, Releases>;

// copied from transaction payment code as it is not pub there
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
enum Releases {
    /// Original version of the pallet.
    V1Ancient,
    /// One that bumps the usage to FixedU128 from FixedI128.
    V2,
}

/// Custom migrations the transaction payment pallet from V1 to V2 that only bumps StorageVersion to 2
pub struct BumpTransactionVersionToV2<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for BumpTransactionVersionToV2<T> {
    fn on_runtime_upgrade() -> Weight {
        if StorageVersion::get() == Some(Releases::V2) {
            log::warn!(
                target: TARGET,
                "skipping V1Ancient to V2 migration: executed on wrong storage version.\
				Expected version V1Ancient, found V2"
            );
            return T::DbWeight::get().reads(1);
        }

        StorageVersion::put(Releases::V2);
        T::DbWeight::get().reads_writes(1, 1)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        if let Some(version) = StorageVersion::get() {
            assert_eq!(
                version,
                Releases::V1Ancient,
                "Can only upgrade from version V1Ancient!"
            );
        }

        let next_fee_multiplier = NextFeeMultiplier::<T>::get();
        let inner_fee_multiplier = next_fee_multiplier.into_inner();
        let old_inner_fee_multiplier = inner_fee_multiplier as i128;

        assert_eq!(
            inner_fee_multiplier, old_inner_fee_multiplier as u128,
            "Old and new NextFeeMultiplier must be the same!"
        );

        Ok(Vec::new())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_: Vec<u8>) -> Result<(), &'static str> {
        assert_eq!(
            StorageVersion::get(),
            Some(Releases::V2),
            "Must upgrade StorageVersion"
        );

        log::info!(
            target: TARGET,
            "NextFeeMultiplier remains the same. Bumped StorageVersion to V2"
        );

        Ok(())
    }
}
