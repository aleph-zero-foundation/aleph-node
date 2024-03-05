use frame_support::{traits::StoredMap, WeakBoundedVec};
use pallet_balances::BalanceLock;
use sp_runtime::traits::Zero;

pub trait AccountInfoProvider {
    type AccountId;
    type RefCount;

    fn get_consumers(who: &Self::AccountId) -> Self::RefCount;
}

impl<T> AccountInfoProvider for frame_system::Pallet<T>
where
    T: frame_system::Config,
{
    type AccountId = T::AccountId;
    type RefCount = frame_system::RefCount;

    fn get_consumers(who: &Self::AccountId) -> Self::RefCount {
        frame_system::Pallet::<T>::consumers(who)
    }
}

pub trait BalancesProvider {
    type AccountId;
    type Balance;
    type MaxLocks;

    fn is_reserved_not_zero(who: &Self::AccountId) -> bool;

    fn locks(who: &Self::AccountId) -> WeakBoundedVec<BalanceLock<Self::Balance>, Self::MaxLocks>;
}

impl<T: pallet_balances::Config<I>, I: 'static> BalancesProvider for pallet_balances::Pallet<T, I> {
    type AccountId = T::AccountId;
    type Balance = T::Balance;
    type MaxLocks = T::MaxLocks;

    fn is_reserved_not_zero(who: &Self::AccountId) -> bool {
        !T::AccountStore::get(who).reserved.is_zero()
    }

    fn locks(who: &Self::AccountId) -> WeakBoundedVec<BalanceLock<Self::Balance>, Self::MaxLocks> {
        pallet_balances::Locks::<T, I>::get(who)
    }
}

pub trait NextKeysSessionProvider {
    type AccountId;

    fn has_next_session_keys(who: &Self::AccountId) -> bool;
}

impl<T> NextKeysSessionProvider for pallet_session::Pallet<T>
where
    T: pallet_session::Config<ValidatorId = <T as frame_system::Config>::AccountId>,
{
    type AccountId = T::AccountId;

    fn has_next_session_keys(who: &Self::AccountId) -> bool {
        pallet_session::NextKeys::<T>::get(who).is_some()
    }
}
