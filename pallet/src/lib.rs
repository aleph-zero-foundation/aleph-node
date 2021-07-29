#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use frame_support::Parameter;
use sp_std::prelude::*;

use frame_support::{sp_runtime::BoundToRuntimeAppPublic, traits::OneSessionHandler};
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        sp_runtime::{traits::OpaqueKeys, RuntimeAppPublic},
        sp_std,
    };
    use frame_system::pallet_prelude::*;
    use pallet_session::Pallet as Session;
    use primitives::{
        ApiError as AlephApiError, DEFAULT_MILLISECS_PER_BLOCK, DEFAULT_SESSION_PERIOD,
    };

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_session::Config {
        type AuthorityId: Member
            + Parameter
            + RuntimeAppPublic
            + Default
            + MaybeSerializeDeserialize;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn authorities)]
    pub(super) type Authorities<T: Config> = StorageValue<_, Vec<T::AuthorityId>, ValueQuery>;

    #[pallet::type_value]
    pub(super) fn DefaultForSessionPeriod() -> u32 {
        DEFAULT_SESSION_PERIOD
    }

    #[pallet::storage]
    #[pallet::getter(fn session_period)]
    pub(super) type SessionPeriod<T: Config> =
        StorageValue<_, u32, ValueQuery, DefaultForSessionPeriod>;

    #[pallet::type_value]
    pub(super) fn DefaultForMillisecsPerBlock() -> u64 {
        DEFAULT_MILLISECS_PER_BLOCK
    }

    #[pallet::storage]
    #[pallet::getter(fn millisecs_per_block)]
    pub(super) type MillisecsPerBlock<T: Config> =
        StorageValue<_, u64, ValueQuery, DefaultForMillisecsPerBlock>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub authorities: Vec<T::AuthorityId>,
        pub session_period: u32,
        pub millisecs_per_block: u64,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                authorities: Vec::new(),
                session_period: DEFAULT_SESSION_PERIOD,
                millisecs_per_block: DEFAULT_MILLISECS_PER_BLOCK,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <SessionPeriod<T>>::put(&self.session_period);
            <MillisecsPerBlock<T>>::put(&self.millisecs_per_block);
        }
    }

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

        pub fn next_session_authorities() -> Result<Vec<T::AuthorityId>, AlephApiError> {
            Session::<T>::queued_keys()
                .iter()
                .map(|(_, key)| key.get(T::AuthorityId::ID).ok_or(AlephApiError::DecodeKey))
                .collect::<Result<Vec<T::AuthorityId>, AlephApiError>>()
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
            let authorities = validators.map(|(_, key)| key).collect::<Vec<_>>();
            Self::initialize_authorities(authorities.as_slice());
        }

        fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_validators: I)
        where
            I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
            T::AccountId: 'a,
        {
            let authorities = validators.map(|(_, key)| key).collect::<Vec<_>>();
            Self::update_authorities(authorities.as_slice());
        }

        fn on_disabled(_validator_index: usize) {}
    }
}
