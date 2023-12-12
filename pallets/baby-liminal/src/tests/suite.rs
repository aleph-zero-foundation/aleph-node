use frame_support::{assert_err, assert_ok, sp_runtime, traits::ReservableCurrency, BoundedVec};
use frame_system::{pallet_prelude::OriginFor, Config};
use sp_runtime::traits::Get;

use super::setup::*;
use crate::{
    Error, VerificationKeyDeposits, VerificationKeyIdentifier, VerificationKeyOwners,
    VerificationKeys,
};

type BabyLiminal = crate::Pallet<TestRuntime>;

const IDENTIFIER: VerificationKeyIdentifier = [0; 8];

fn vk() -> Vec<u8> {
    vec![41; 1000]
}

fn owner() -> OriginFor<TestRuntime> {
    <TestRuntime as Config>::RuntimeOrigin::signed(1)
}

fn not_owner() -> OriginFor<TestRuntime> {
    <TestRuntime as Config>::RuntimeOrigin::signed(2)
}

fn reserved_balance(account_id: u128) -> u64 {
    <TestRuntime as crate::Config>::Currency::reserved_balance(&account_id)
}

fn free_balance(account_id: u128) -> u64 {
    <TestRuntime as crate::Config>::Currency::free_balance(&account_id)
}

fn put_key() -> u64 {
    let owner = 1;
    let key = vk();
    let per_byte_fee: u64 = <TestRuntime as crate::Config>::VerificationKeyDepositPerByte::get();
    let deposit = key.len() as u64 * per_byte_fee;
    VerificationKeys::<TestRuntime>::insert(IDENTIFIER, BoundedVec::try_from(key).unwrap());
    VerificationKeyOwners::<TestRuntime>::insert(IDENTIFIER, owner);
    VerificationKeyDeposits::<TestRuntime>::insert((owner, IDENTIFIER), deposit);
    <TestRuntime as crate::Config>::Currency::reserve(&owner, deposit)
        .expect("Could not reserve a deposit");
    deposit
}

#[test]
fn stores_vk_with_fresh_identifier() {
    new_test_ext().execute_with(|| {
        assert_ok!(BabyLiminal::store_key(owner(), IDENTIFIER, vk()));

        let stored_key = VerificationKeys::<TestRuntime>::get(IDENTIFIER);
        assert!(stored_key.is_some());
        assert_eq!(stored_key.unwrap().to_vec(), vk());
    });
}

#[test]
fn does_not_overwrite_registered_key() {
    new_test_ext().execute_with(|| {
        put_key();

        assert_err!(
            BabyLiminal::store_key(owner(), IDENTIFIER, vk()),
            Error::<TestRuntime>::IdentifierAlreadyInUse
        );
    });
}

#[test]
fn not_owner_cannot_delete_key() {
    new_test_ext().execute_with(|| {
        put_key();
        assert_err!(
            BabyLiminal::delete_key(not_owner(), IDENTIFIER),
            Error::<TestRuntime>::NotOwner
        );
    });
}

#[test]
fn owner_can_delete_key() {
    new_test_ext().execute_with(|| {
        put_key();
        assert_ok!(BabyLiminal::delete_key(owner(), IDENTIFIER));
    });
}

#[test]
fn not_owner_cannot_overwrite_key() {
    new_test_ext().execute_with(|| {
        put_key();
        assert_err!(
            BabyLiminal::overwrite_key(not_owner(), IDENTIFIER, vk()),
            Error::<TestRuntime>::NotOwner
        );
    });
}

#[test]
fn owner_can_overwrite_key() {
    new_test_ext().execute_with(|| {
        put_key();
        assert_ok!(BabyLiminal::overwrite_key(owner(), IDENTIFIER, vk()));
    });
}

#[test]
fn key_deposits() {
    new_test_ext().execute_with(|| {
        let per_byte_fee: u64 =
            <TestRuntime as crate::Config>::VerificationKeyDepositPerByte::get();

        let reserved_balance_begin = reserved_balance(1);
        let deposit = put_key();
        let reserved_balance_after = reserved_balance(1);

        assert_eq!(reserved_balance_begin + deposit, reserved_balance_after);

        let long_key_size = 2 * vk().len();
        let long_key = vec![0u8; long_key_size];

        let free_balance_before = free_balance(1);
        assert_ok!(BabyLiminal::overwrite_key(owner(), IDENTIFIER, long_key));
        assert_eq!(
            free_balance_before - free_balance(1),
            (long_key_size as u64 * per_byte_fee) - deposit
        );

        let short_key_size = vk().len() / 2;
        let short_key = vec![0u8; short_key_size];

        let reserved_balance_before = reserved_balance(1);
        assert_ok!(BabyLiminal::overwrite_key(owner(), IDENTIFIER, short_key));
        let reserved_balance_after = reserved_balance(1);
        assert_eq!(
            reserved_balance_before - reserved_balance_after,
            ((long_key_size - short_key_size) as u64 * per_byte_fee)
        );

        assert_ok!(BabyLiminal::delete_key(owner(), IDENTIFIER));
        assert_eq!(reserved_balance_begin, reserved_balance(1));
    });
}

#[test]
fn does_not_store_too_long_key() {
    new_test_ext().execute_with(|| {
        let limit: u32 = <TestRuntime as crate::Config>::MaximumVerificationKeyLength::get();

        assert_err!(
            BabyLiminal::store_key(owner(), IDENTIFIER, vec![0; (limit + 1) as usize]),
            Error::<TestRuntime>::VerificationKeyTooLong
        );
    });
}
