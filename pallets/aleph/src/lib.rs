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
mod traits;

use frame_support::{
    log,
    sp_runtime::BoundToRuntimeAppPublic,
    traits::{OneSessionHandler, StorageVersion},
};
pub use pallet::*;
#[cfg(feature = "std")]
use primitives::LEGACY_FINALITY_VERSION;
use primitives::{SessionIndex, Version, VersionChange, DEFAULT_FINALITY_VERSION};
use sp_std::prelude::*;

/// The current storage version.
const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
pub(crate) const LOG_TARGET: &str = "pallet-aleph";

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, sp_runtime::RuntimeAppPublic};
    use frame_system::{ensure_root, pallet_prelude::OriginFor};
    use pallet_session::SessionManager;
    use sp_std::collections::btree_set::BTreeSet;
    #[cfg(feature = "std")]
    use sp_std::marker::PhantomData;

    use super::*;
    use crate::traits::{NextSessionAuthorityProvider, SessionInfoProvider};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type AuthorityId: Member + Parameter + RuntimeAppPublic + MaybeSerializeDeserialize;
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type SessionInfoProvider: SessionInfoProvider;
        type SessionManager: SessionManager<<Self as frame_system::Config>::AccountId>;
        type NextSessionAuthorityProvider: NextSessionAuthorityProvider<Self>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        ChangeEmergencyFinalizer(T::AuthorityId),
        ScheduleFinalityVersionChange(VersionChange),
        FinalityVersionChange(VersionChange),
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Default finality version. Relevant for sessions before the first version change occurs.
    #[pallet::type_value]
    pub(crate) fn DefaultFinalityVersion<T: Config>() -> Version {
        DEFAULT_FINALITY_VERSION
    }

    /// Default value for `NextAuthorities` storage.
    #[pallet::type_value]
    pub(crate) fn DefaultNextAuthorities<T: Config>() -> Vec<T::AuthorityId> {
        T::NextSessionAuthorityProvider::next_authorities()
    }

    #[pallet::storage]
    #[pallet::getter(fn authorities)]
    pub(super) type Authorities<T: Config> = StorageValue<_, Vec<T::AuthorityId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn next_authorities)]
    pub(super) type NextAuthorities<T: Config> =
        StorageValue<_, Vec<T::AuthorityId>, ValueQuery, DefaultNextAuthorities<T>>;

    /// Set of account ids that will be used as authorities in the next session
    #[pallet::storage]
    pub type NextFinalityCommittee<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

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
        pub(crate) fn initialize_authorities(
            authorities: &[T::AuthorityId],
            next_authorities: &[T::AuthorityId],
        ) {
            if !authorities.is_empty() {
                if !<Authorities<T>>::get().is_empty() {
                    log::error!(target: LOG_TARGET, "Authorities are already initialized!");
                } else {
                    <Authorities<T>>::put(authorities);
                }
            }
            if !next_authorities.is_empty() {
                // Storage NextAuthorities has default value so should never be empty.
                <NextAuthorities<T>>::put(next_authorities);
            }
        }

        fn get_authorities_for_next_session(
            next_authorities: Vec<(&T::AccountId, T::AuthorityId)>,
        ) -> Vec<T::AuthorityId> {
            let next_committee_ids: BTreeSet<_> =
                NextFinalityCommittee::<T>::get().into_iter().collect();

            let next_committee_authorities: Vec<_> = next_authorities
                .into_iter()
                .filter_map(|(account_id, auth_id)| {
                    if next_committee_ids.contains(account_id) {
                        Some(auth_id)
                    } else {
                        None
                    }
                })
                .collect();

            if next_committee_authorities.len() != next_committee_ids.len() {
                log::error!(
                    target: LOG_TARGET,
                    "Not all committee members were converted to keys."
                );
            }

            next_committee_authorities
        }

        pub(crate) fn update_authorities(next_authorities: Vec<(&T::AccountId, T::AuthorityId)>) {
            let next_authorities = Self::get_authorities_for_next_session(next_authorities);

            <Authorities<T>>::put(<NextAuthorities<T>>::get());
            <NextAuthorities<T>>::put(next_authorities);
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
        #[pallet::call_index(0)]
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
        #[pallet::call_index(1)]
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
            // it is guaranteed that the first validator set will also be used in the next session
            Self::initialize_authorities(authorities.as_slice(), authorities.as_slice());
        }

        fn on_new_session<'a, I: 'a>(changed: bool, _: I, queued_validators: I)
        where
            I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
            T::AccountId: 'a,
        {
            Self::update_emergency_finalizer();
            if changed {
                Self::update_authorities(queued_validators.collect());
            }
        }

        fn on_disabled(_validator_index: u32) {}
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub finality_version: Version,
        pub _marker: PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                finality_version: LEGACY_FINALITY_VERSION as u32,
                _marker: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <FinalityVersion<T>>::put(&self.finality_version);
        }
    }
}
