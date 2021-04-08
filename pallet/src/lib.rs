#![cfg_attr(not(feature = "std"), no_std)]


use sp_std::prelude::*;
use frame_support::{
    Parameter,
};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use frame_support::sp_std;
    use frame_support::sp_runtime::RuntimeAppPublic;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + MaybeSerializeDeserialize;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn authorities)]
    pub(super) type Authorities<T: Config> = StorageValue<_, Vec<T::AuthorityId>, ValueQuery>;


    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub authorities: Vec<T::AuthorityId>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { authorities: Vec::new() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            Pallet::<T>::initialize_authorities(&self.authorities);
        }
    }
}

impl<T: Config> Pallet<T> {
    fn initialize_authorities(authorities: &[T::AuthorityId]) {
        if !authorities.is_empty() {
            assert!(<Authorities<T>>::get().is_empty(), "Authorities are already initialized!");
            <Authorities<T>>::put(authorities);
        }
    }
}
