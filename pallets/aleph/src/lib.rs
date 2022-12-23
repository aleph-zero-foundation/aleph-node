//! This pallet is the runtime companion of the Aleph finality gadget.
//!
//! Currently, it only provides support for changing sessions but in the future
//! it will allow reporting equivocation in AlephBFT.
//!
//! This pallet relies on an extension of the `AlephSessionApi` Runtime API to handle the finality
//! version. The scheduled version change is persisted as `FinalityScheduledVersionChange`. This
//! value stores the information about a scheduled finality version change, where `version_incoming`
//! is the version to be set and `session` is the session on which the new version will be set.
//! A `pallet_session::Session_Manager` checks whether a scheduled version change has moved into
//! the past and, if so, records it as the current version represented as `FinalityVersion`,
//! and clears `FinalityScheduledVersionChange`.
//! It is always possible to reschedule a version change. In order to cancel a scheduled version
//! change rather than reschedule it, a new version change should be scheduled with
//! `version_incoming` set to the current value of `FinalityVersion`.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod impls;
mod migrations;
mod traits;

use frame_support::{
    log,
    sp_runtime::BoundToRuntimeAppPublic,
    traits::{OneSessionHandler, StorageVersion},
};
pub use pallet::*;
use primitives::{SessionIndex, Version, VersionChange};
use sp_std::prelude::*;

/// The current storage version.
const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

const DEFAULT_FINALITY_VERSION: Version = 1;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, sp_runtime::RuntimeAppPublic};
    use frame_system::{
        ensure_root,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };
    use pallet_session::SessionManager;
    use pallets_support::StorageMigration;

    use super::*;
    use crate::traits::SessionInfoProvider;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type AuthorityId: Member + Parameter + RuntimeAppPublic + MaybeSerializeDeserialize;
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type SessionInfoProvider: SessionInfoProvider<Self>;
        type SessionManager: SessionManager<<Self as frame_system::Config>::AccountId>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ChangeEmergencyFinalizer(T::AuthorityId),
        ScheduleFinalityVersionChange(VersionChange),
        FinalityVersionChange(VersionChange),
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain = <Pallet<T> as GetStorageVersion>::on_chain_storage_version();
            T::DbWeight::get().reads(1)
                + match on_chain {
                    _ if on_chain == STORAGE_VERSION => Weight::zero(),
                    _ if on_chain == StorageVersion::new(1) => {
                        migrations::v1_to_v2::Migration::<T, Self>::migrate()
                    }
                    _ if on_chain == StorageVersion::new(0) => {
                        migrations::v0_to_v1::Migration::<T, Self>::migrate()
                            + migrations::v1_to_v2::Migration::<T, Self>::migrate()
                    }
                    _ => {
                        log::warn!(
                            target: "pallet_aleph",
                            "On chain storage version of pallet aleph is {:?} but it should not be bigger than 2",
                            on_chain
                        );
                        Weight::zero()
                    }
                }
        }
    }

    /// Default finality version. Relevant for sessions before the first version change occurs.
    #[pallet::type_value]
    pub(crate) fn DefaultFinalityVersion<T: Config>() -> Version {
        DEFAULT_FINALITY_VERSION
    }

    #[pallet::storage]
    #[pallet::getter(fn authorities)]
    pub(super) type Authorities<T: Config> = StorageValue<_, Vec<T::AuthorityId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn emergency_finalizer)]
    pub(super) type EmergencyFinalizer<T: Config> = StorageValue<_, T::AuthorityId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn queued_emergency_finalizer)]
    pub(super) type QueuedEmergencyFinalizer<T: Config> =
        StorageValue<_, T::AuthorityId, OptionQuery>;

    #[pallet::storage]
    type NextEmergencyFinalizer<T: Config> = StorageValue<_, T::AuthorityId, OptionQuery>;

    /// Current finality version.
    #[pallet::storage]
    #[pallet::getter(fn finality_version)]
    pub(super) type FinalityVersion<T: Config> =
        StorageValue<_, Version, ValueQuery, DefaultFinalityVersion<T>>;

    /// Scheduled finality version change.
    #[pallet::storage]
    #[pallet::getter(fn finality_version_change)]
    pub(super) type FinalityScheduledVersionChange<T: Config> =
        StorageValue<_, VersionChange, OptionQuery>;

    impl<T: Config> Pallet<T> {
        pub(crate) fn initialize_authorities(authorities: &[T::AuthorityId]) {
            if !authorities.is_empty() {
                assert!(
                    <Authorities<T>>::get().is_empty(),
                    "Authorities are already initialized!"
                );
                <Authorities<T>>::put(authorities);
            }
        }

        pub(crate) fn update_authorities(authorities: &[T::AuthorityId]) {
            <Authorities<T>>::put(authorities);
        }

        pub(crate) fn update_emergency_finalizer() {
            if let Some(emergency_finalizer) = <QueuedEmergencyFinalizer<T>>::get() {
                <EmergencyFinalizer<T>>::put(emergency_finalizer)
            }

            if let Some(emergency_finalizer) = <NextEmergencyFinalizer<T>>::get() {
                <QueuedEmergencyFinalizer<T>>::put(emergency_finalizer)
            }
        }

        pub(crate) fn set_next_emergency_finalizer(emergency_finalizer: T::AuthorityId) {
            <NextEmergencyFinalizer<T>>::put(emergency_finalizer);
        }

        pub(crate) fn current_session() -> u32 {
            T::SessionInfoProvider::current_session()
        }

        // If a scheduled future version change is rescheduled to a different session,
        // it is possible to reschedule it with the same version as initially.
        // To cancel a future version change, reschedule it with the current version.
        // If a scheduled version change has moved into the past, `SessionManager` records it
        // as the current version.
        pub(crate) fn do_schedule_finality_version_change(
            version_change: VersionChange,
        ) -> Result<(), &'static str> {
            let current_session = Self::current_session();

            let session_to_schedule = version_change.session;

            if session_to_schedule < current_session {
                return Err("Cannot schedule finality version changes for sessions in the past!");
            } else if session_to_schedule < current_session + 2 {
                return Err(
                    "Tried to schedule an finality version change less than 2 sessions in advance!",
                );
            }

            // Update the scheduled version change with the supplied version change.
            <FinalityScheduledVersionChange<T>>::put(version_change);

            Ok(())
        }

        pub fn next_session_finality_version() -> Version {
            let next_session = Self::current_session() + 1;
            let scheduled_version_change = Self::finality_version_change();

            if let Some(version_change) = scheduled_version_change {
                if next_session == version_change.session {
                    return version_change.version_incoming;
                }
            }

            Self::finality_version()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets the emergency finalization key. If called in session `N` the key can be used to
        /// finalize blocks from session `N+2` onwards, until it gets overridden.
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn set_emergency_finalizer(
            origin: OriginFor<T>,
            emergency_finalizer: T::AuthorityId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Self::set_next_emergency_finalizer(emergency_finalizer.clone());
            Self::deposit_event(Event::ChangeEmergencyFinalizer(emergency_finalizer));
            Ok(())
        }

        /// Schedules a finality version change for a future session. If such a scheduled future
        /// version is already set, it is replaced with the provided one.
        /// Any rescheduling of a future version change needs to occur at least 2 sessions in
        /// advance of the provided session of the version change.
        /// In order to cancel a scheduled version change, a new version change should be scheduled
        /// with the same version as the current one.
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn schedule_finality_version_change(
            origin: OriginFor<T>,
            version_incoming: Version,
            session: SessionIndex,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let version_change = VersionChange {
                version_incoming,
                session,
            };

            if let Err(e) = Self::do_schedule_finality_version_change(version_change.clone()) {
                return Err(DispatchError::Other(e));
            }

            Self::deposit_event(Event::ScheduleFinalityVersionChange(version_change));
            Ok(())
        }
    }

    impl<T: Config> BoundToRuntimeAppPublic for Pallet<T> {
        type Public = T::AuthorityId;
    }

    impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
        type Key = T::AuthorityId;

        fn on_genesis_session<'a, I: 'a>(validators: I)
        where
            I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
            T::AccountId: 'a,
        {
            let (_, authorities): (Vec<_>, Vec<_>) = validators.unzip();
            Self::initialize_authorities(authorities.as_slice());
        }

        fn on_new_session<'a, I: 'a>(changed: bool, validators: I, _queued_validators: I)
        where
            I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
            T::AccountId: 'a,
        {
            Self::update_emergency_finalizer();
            if changed {
                let (_, authorities): (Vec<_>, Vec<_>) = validators.unzip();
                Self::update_authorities(authorities.as_slice());
            }
        }

        fn on_disabled(_validator_index: u32) {}
    }
}
