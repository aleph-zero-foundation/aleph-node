#![allow(clippy::let_unit_value)]

use frame_benchmarking::{account, benchmarks};
use frame_support::{
    sp_runtime::traits::Bounded,
    traits::{Currency, Get},
    BoundedVec,
};
use frame_system::RawOrigin;
use sp_std::{vec, vec::Vec};

use crate::{
    BalanceOf, Call, Config, Pallet, VerificationKeyDeposits, VerificationKeyIdentifier,
    VerificationKeyOwners, VerificationKeys,
};

const SEED: u32 = 41;
const IDENTIFIER: VerificationKeyIdentifier = [0; 8];

fn caller<T: Config>() -> RawOrigin<<T as frame_system::Config>::AccountId> {
    let caller_account = account("caller", 0, SEED);
    T::Currency::make_free_balance_be(&caller_account, BalanceOf::<T>::max_value());
    RawOrigin::Signed(caller_account)
}

fn insert_key<T: Config>(key: Vec<u8>) {
    let owner: T::AccountId = account("caller", 0, SEED);
    let deposit = BalanceOf::<T>::from(0u32);
    VerificationKeys::<T>::insert(IDENTIFIER, BoundedVec::try_from(key).unwrap());
    VerificationKeyOwners::<T>::insert(IDENTIFIER, &owner);
    VerificationKeyDeposits::<T>::insert((&owner, IDENTIFIER), deposit);
}

benchmarks! {

    store_key {
        let l in 1 .. T::MaximumVerificationKeyLength::get();
        let key = vec![0u8; l as usize];
    } : _(caller::<T>(), IDENTIFIER, key)

    overwrite_equal_key {
        let l in 1 .. T::MaximumVerificationKeyLength::get();
        let key = vec![0u8; l as usize];
        let _ = insert_key::<T>(key.clone ());
    } : overwrite_key(caller::<T>(), IDENTIFIER, key)

    overwrite_key {
        let l in 1 .. T::MaximumVerificationKeyLength::get() - 1;
        let _ = insert_key::<T>(vec![0u8; l as usize]);
        let longer_key = vec![0u8; (l + 1) as usize];
    } : overwrite_key(caller::<T>(), IDENTIFIER, longer_key)

    delete_key {
        let l in 1 .. T::MaximumVerificationKeyLength::get();
        let key = vec![0u8; l as usize];
        let _ = insert_key::<T>(key);
    } : _(caller::<T>(), IDENTIFIER)

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::TestRuntime);
}
