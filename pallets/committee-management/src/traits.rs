use frame_support::pallet_prelude::Get;
use sp_staking::{EraIndex, SessionIndex};
use sp_std::vec::Vec;

pub trait EraInfoProvider {
    type AccountId;

    /// Returns `Some(idx)` where idx is the active era index otherwise
    /// if no era is active returns `None`.
    fn active_era() -> Option<EraIndex>;
    /// Returns `Some(idx)` where idx is the current era index which is latest
    /// planed era otherwise if no era has started returns `None`.
    fn current_era() -> Option<EraIndex>;
    /// Returns the index of the starting session of the `era` if possible. Otherwise returns `None`.
    fn era_start_session_index(era: EraIndex) -> Option<SessionIndex>;
    /// Returns how many sessions are in single era.
    fn sessions_per_era() -> SessionIndex;
    /// Returns the elected authorities for provided era.
    fn elected_validators(era: EraIndex) -> Vec<Self::AccountId>;
}

impl<T> EraInfoProvider for pallet_staking::Pallet<T>
where
    T: pallet_staking::Config,
{
    type AccountId = T::AccountId;

    fn active_era() -> Option<EraIndex> {
        pallet_staking::ActiveEra::<T>::get().map(|ae| ae.index)
    }

    fn current_era() -> Option<EraIndex> {
        pallet_staking::CurrentEra::<T>::get()
    }

    fn era_start_session_index(era: EraIndex) -> Option<SessionIndex> {
        pallet_staking::ErasStartSessionIndex::<T>::get(era)
    }

    fn sessions_per_era() -> SessionIndex {
        T::SessionsPerEra::get()
    }

    fn elected_validators(era: EraIndex) -> Vec<Self::AccountId> {
        pallet_staking::ErasStakers::<T>::iter_key_prefix(era).collect()
    }
}

pub trait ValidatorRewardsHandler {
    type AccountId;
    /// Returns total exposure of validators for the `era`
    fn validator_totals(era: EraIndex) -> Vec<(Self::AccountId, u128)>;
    /// Add reward for validators
    fn add_rewards(rewards: impl IntoIterator<Item = (Self::AccountId, u32)>);
}

impl<T> ValidatorRewardsHandler for pallet_staking::Pallet<T>
where
    T: pallet_staking::Config,
    <T as pallet_staking::Config>::CurrencyBalance: Into<u128>,
{
    type AccountId = T::AccountId;
    fn validator_totals(era: EraIndex) -> Vec<(Self::AccountId, u128)> {
        pallet_staking::ErasStakers::<T>::iter_prefix(era)
            .map(|(validator, exposure)| (validator, exposure.total.into()))
            .collect()
    }

    fn add_rewards(rewards: impl IntoIterator<Item = (Self::AccountId, u32)>) {
        pallet_staking::Pallet::<T>::reward_by_ids(rewards);
    }
}

pub trait ValidatorExtractor {
    type AccountId;

    /// Removes given validator from pallet's staking validators list
    fn remove_validator(who: &Self::AccountId);
}

impl<T> ValidatorExtractor for pallet_staking::Pallet<T>
where
    T: pallet_staking::Config,
{
    type AccountId = T::AccountId;

    fn remove_validator(who: &Self::AccountId) {
        pallet_staking::Pallet::<T>::do_remove_validator(who);
    }
}
