use sp_staking::EraIndex;
use sp_std::vec::Vec;

pub trait ValidatorProvider {
    type AccountId;
    fn elected_validators(era: EraIndex) -> Vec<Self::AccountId>;
}

impl<T: pallet_staking::Config> ValidatorProvider for pallet_staking::Pallet<T> {
    type AccountId = T::AccountId;

    fn elected_validators(era: EraIndex) -> Vec<Self::AccountId> {
        pallet_staking::ErasStakers::<T>::iter_key_prefix(era).collect()
    }
}
