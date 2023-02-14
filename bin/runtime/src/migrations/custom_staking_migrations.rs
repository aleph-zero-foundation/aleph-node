use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    log,
    pallet_prelude::{Get, TypeInfo},
    storage_alias,
    traits::OnRuntimeUpgrade,
    RuntimeDebug,
};
use pallet_staking::Config;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

use crate::Weight;

#[storage_alias]
type StorageVersion = StorageValue<Staking, Releases>;

// copied from pallet staking, hack for that fact that original struct is not exported
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
enum Releases {
    V1_0_0Ancient,
    V2_0_0,
    V3_0_0,
    V4_0_0,
    V5_0_0,  // blockable validators.
    V6_0_0,  // removal of all storage associated with offchain phragmen.
    V7_0_0,  // keep track of number of nominators / validators in map
    V8_0_0,  // populate `VoterList`.
    V9_0_0,  // inject validators into `VoterList` as well.
    V10_0_0, // remove `EarliestUnappliedSlash`.
    V11_0_0, // Move pallet storage prefix, e.g. BagsList -> VoterBagsList
    V12_0_0, // remove `HistoryDepth`.
}

pub struct BumpStorageVersionFromV7ToV11<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for BumpStorageVersionFromV7ToV11<T> {
    fn on_runtime_upgrade() -> Weight {
        if let Some(Releases::V7_0_0) = StorageVersion::get() {
            log::info!(
                target: "runtime::staking",
                "💸 Migrating storage to Releases::V7_0_0 from Releases::V11_0_0"
            );
            StorageVersion::put(Releases::V11_0_0);
            T::DbWeight::get().reads_writes(1, 1)
        } else {
            log::warn!(
                target: "runtime::staking",
                "💸 Migration being executed on the wrong storage \
                version, expected Releases::V7_0_0"
            );
            T::DbWeight::get().reads(1)
        }
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        frame_support::ensure!(
            StorageVersion::get() == Some(Releases::V7_0_0),
            "💸 Migration being executed on the wrong storage \
				version, expected Releases::V7_0_0"
        );

        Ok(Vec::new())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_prev_count: Vec<u8>) -> Result<(), &'static str> {
        frame_support::ensure!(
            StorageVersion::get() == Some(Releases::V11_0_0),
            "💸 must upgrade to Releases::V11_0_0"
        );
        Ok(())
    }
}
