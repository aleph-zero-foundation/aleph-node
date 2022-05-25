use frame_support::{pallet_prelude::Get, traits::Currency};
use sp_staking::{EraIndex, SessionIndex};
use sp_std::vec::Vec;

pub trait SessionInfoProvider<T: frame_system::Config> {
    /// Returns index of the current session
    fn current_session_index() -> SessionIndex;
    /// Returns list containing validators that in the current session produce&finalize blocks.
    fn current_committee() -> Vec<T::AccountId>;
}

impl<T> SessionInfoProvider<T> for pallet_session::Pallet<T>
where
    T: pallet_session::Config,
    T::ValidatorId: Into<T::AccountId>,
{
    fn current_session_index() -> SessionIndex {
        pallet_session::Pallet::<T>::current_index()
    }

    fn current_committee() -> Vec<T::AccountId> {
        pallet_session::Validators::<T>::get()
            .into_iter()
            .map(|a| a.into())
            .collect()
    }
}

pub trait ValidatorRewardsHandler<T: frame_system::Config> {
    /// Returns all validators for the `era`.
    fn all_era_validators(era: EraIndex) -> Vec<T::AccountId>;
    /// Returns total exposure of validators for the `era`
    fn validator_totals(era: EraIndex) -> Vec<(T::AccountId, u128)>;
    /// Add reward for validators
    fn add_rewards(rewards: impl IntoIterator<Item = (T::AccountId, u32)>);
}

impl<T> ValidatorRewardsHandler<T> for pallet_staking::Pallet<T>
where
    T: pallet_staking::Config,
    <T::Currency as Currency<T::AccountId>>::Balance: Into<u128>,
{
    fn all_era_validators(era: EraIndex) -> Vec<T::AccountId> {
        pallet_staking::ErasStakers::<T>::iter_key_prefix(era).collect()
    }

    fn validator_totals(era: EraIndex) -> Vec<(T::AccountId, u128)> {
        pallet_staking::ErasStakers::<T>::iter_prefix(era)
            .map(|(validator, exposure)| (validator, exposure.total.into()))
            .collect()
    }

    fn add_rewards(rewards: impl IntoIterator<Item = (T::AccountId, u32)>) {
        pallet_staking::Pallet::<T>::reward_by_ids(rewards);
    }
}

pub trait EraInfoProvider {
    /// Returns `Some(idx)` where idx is the current active era index otherwise
    /// if no era is active returns `None`.
    fn active_era() -> Option<EraIndex>;
    /// Returns the index of the starting session of the `era` if possible. Otherwise returns `None`.
    fn era_start_session_index(era: EraIndex) -> Option<SessionIndex>;
    /// Returns how many sessions are in single era.
    fn sessions_per_era() -> SessionIndex;
}

impl<T> EraInfoProvider for pallet_staking::Pallet<T>
where
    T: pallet_staking::Config,
{
    fn active_era() -> Option<EraIndex> {
        pallet_staking::ActiveEra::<T>::get().map(|ae| ae.index)
    }

    fn era_start_session_index(era: EraIndex) -> Option<SessionIndex> {
        pallet_staking::ErasStartSessionIndex::<T>::get(era)
    }

    fn sessions_per_era() -> SessionIndex {
        T::SessionsPerEra::get()
    }
}
