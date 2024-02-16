//! # Feature control pallet
//!
//! This pallet provides a way of turning on/off features in the runtime that cannot be controlled with runtime
//! configuration. It maintains a simple map of feature identifiers together with their status (enabled/disabled). It is
//! supposed to be modified only by the specified origin, but read by any runtime code.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
#![deny(missing_docs)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod tests;
mod weights;

use frame_support::pallet_prelude::StorageVersion;
pub use pallet::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::RuntimeDebug;
pub use weights::{AlephWeight, WeightInfo};

/// All available optional features for the Aleph Zero runtime.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    RuntimeDebug,
    Encode,
    Decode,
    MaxEncodedLen,
    TypeInfo,
    Serialize,
    Deserialize,
)]
pub enum Feature {
    /// The on-chain verifier feature involves:
    /// - VkStorage pallet (for storing verification keys)
    /// - smart contract chain extension exposing `verify` function
    /// - SnarkVerifier runtime interface
    #[codec(index = 0)]
    OnChainVerifier,
}

const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::OriginFor;
    use sp_std::vec::Vec;

    use super::{weights::WeightInfo, *};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Item required for emitting events.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Weight information for the pallet's extrinsics.
        type WeightInfo: WeightInfo;
        /// The origin that can modify the feature map.
        type Supervisor: EnsureOrigin<Self::RuntimeOrigin>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A feature has been enabled.
        FeatureEnabled(Feature),
        /// A feature has been disabled.
        FeatureDisabled(Feature),
    }

    #[pallet::storage]
    pub type ActiveFeatures<T: Config> = StorageMap<_, Twox64Concat, Feature, ()>;

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    /// We can set active features right away in the genesis config.
    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// Features to be activated from the very beginning.
        pub active_features: Vec<Feature>,
        /// Generic marker.
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            for feature in &self.active_features {
                ActiveFeatures::<T>::insert(feature, ());
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Enable a feature.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::enable())]
        pub fn enable(origin: OriginFor<T>, feature: Feature) -> DispatchResult {
            T::Supervisor::ensure_origin(origin)?;
            ActiveFeatures::<T>::insert(feature, ());
            Self::deposit_event(Event::FeatureEnabled(feature));
            Ok(())
        }

        /// Disable a feature.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::disable())]
        pub fn disable(origin: OriginFor<T>, feature: Feature) -> DispatchResult {
            T::Supervisor::ensure_origin(origin)?;
            ActiveFeatures::<T>::remove(feature);
            Self::deposit_event(Event::FeatureDisabled(feature));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Check if a feature is enabled.
        pub fn is_feature_enabled(feature: Feature) -> bool {
            ActiveFeatures::<T>::contains_key(feature)
        }
    }
}
