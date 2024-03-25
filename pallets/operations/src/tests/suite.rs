use frame_support::{
    assert_ok,
    traits::{Currency, LockableCurrency, ReservableCurrency, WithdrawReasons},
};
use pallet_staking::RewardDestination;

use super::setup::*;
use crate::VESTING_ID;

fn total_balance(account_id: u64) -> u128 {
    pallet_balances::Pallet::<TestRuntime>::total_balance(&account_id)
}

fn free_balance(account_id: u64) -> u128 {
    pallet_balances::Pallet::<TestRuntime>::free_balance(account_id)
}

fn reserved_balance(account_id: u64) -> u128 {
    pallet_balances::Pallet::<TestRuntime>::reserved_balance(account_id)
}

fn usable_balance(account_id: u64) -> u128 {
    pallet_balances::Pallet::<TestRuntime>::usable_balance(account_id)
}

fn ed() -> u128 {
    <TestRuntime as pallet_balances::Config>::ExistentialDeposit::get()
}

fn providers(account_id: u64) -> u32 {
    frame_system::Pallet::<TestRuntime>::providers(&account_id)
}

fn consumers(account_id: u64) -> u32 {
    frame_system::Pallet::<TestRuntime>::consumers(&account_id)
}

fn pallet_operations_events() -> Vec<crate::Event<TestRuntime>> {
    frame_system::Pallet::<TestRuntime>::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let RuntimeEvent::Operations(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn given_accounts_with_initial_balance_then_balances_data_and_counters_are_valid() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        assert_eq!(total_balance(authority_id), total_balance_authority);
        assert_eq!(free_balance(authority_id), total_balance_authority);
        assert_eq!(reserved_balance(authority_id), 0);
        // there are > 0 consumers, so we can't transfer everything
        assert_eq!(usable_balance(authority_id), total_balance_authority - ed());

        assert_eq!(providers(authority_id), 1);
        // +1 consumers due to session keys are set
        assert_eq!(consumers(authority_id), 1);

        assert_eq!(total_balance(non_authority_id), total_balance_non_authority);
        assert_eq!(free_balance(non_authority_id), total_balance_non_authority);
        assert_eq!(reserved_balance(non_authority_id), 0);
        // consumers == 0 so we can transfer everything
        assert_eq!(
            usable_balance(non_authority_id),
            total_balance_non_authority
        );
        assert_eq!(providers(non_authority_id), 1);
        assert_eq!(consumers(non_authority_id), 0);
    });
}

#[test]
fn given_accounts_with_initial_balance_when_reserving_then_balances_data_and_counters_are_valid() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let reserved_amount = 3_u128;
        assert_ok!(pallet_balances::Pallet::<TestRuntime>::reserve(
            &authority_id,
            reserved_amount
        ));
        assert_eq!(total_balance(authority_id), total_balance_authority);
        assert_eq!(
            free_balance(authority_id),
            total_balance_authority - reserved_amount
        );
        assert_eq!(reserved_balance(authority_id), reserved_amount);
        // since consumers > 0
        assert_eq!(
            usable_balance(authority_id),
            total_balance_authority - reserved_amount - ed()
        );
        assert_eq!(providers(authority_id), 1);
        // +1 consumers due to session keys are set
        // +1 consumers since there is reserved balance
        assert_eq!(consumers(authority_id), 2);

        assert_ok!(pallet_balances::Pallet::<TestRuntime>::reserve(
            &non_authority_id,
            reserved_amount
        ));
        assert_eq!(total_balance(non_authority_id), total_balance_non_authority);
        assert_eq!(
            free_balance(non_authority_id),
            total_balance_non_authority - reserved_amount
        );
        assert_eq!(reserved_balance(non_authority_id), reserved_amount);
        // free - ed - reserved since consumers > 0
        assert_eq!(
            usable_balance(non_authority_id),
            total_balance_non_authority - reserved_amount - ed()
        );
        assert_eq!(providers(non_authority_id), 1);
        // +1 consumers since there is reserved balance
        assert_eq!(consumers(authority_id), 2);
    });
}

#[test]
fn given_account_with_initial_balance_when_bonding_then_balances_data_and_counters_are_valid() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let bonded = 123_u128;
        assert_ok!(pallet_staking::Pallet::<TestRuntime>::bond(
            RuntimeOrigin::signed(authority_id),
            bonded,
            RewardDestination::Controller
        ));
        assert_eq!(total_balance(authority_id), total_balance_authority);
        assert_eq!(free_balance(authority_id), total_balance_authority);
        assert_eq!(reserved_balance(authority_id), 0_u128);
        assert_eq!(
            usable_balance(authority_id),
            total_balance_authority - bonded
        );
        assert_eq!(providers(authority_id), 1);
        // +1 consumers due to session keys are set
        // +1 consumers since there is frozen balance
        // +1 consumers since there is at least one lock
        // +1 consumers from bond()
        assert_eq!(consumers(authority_id), 4);

        assert_ok!(pallet_staking::Pallet::<TestRuntime>::bond(
            RuntimeOrigin::signed(non_authority_id),
            bonded,
            RewardDestination::Controller
        ));
        assert_eq!(total_balance(non_authority_id), total_balance_non_authority);
        assert_eq!(free_balance(non_authority_id), total_balance_non_authority);
        assert_eq!(reserved_balance(non_authority_id), 0_u128);
        // free - max(frozen, ed)
        assert_eq!(
            usable_balance(non_authority_id),
            total_balance_non_authority - bonded
        );
        assert_eq!(providers(non_authority_id), 1);
        // +1 consumers since there is frozen balance
        // +1 consumers since there is at least one lock
        // +1 consumers from bond()
        assert_eq!(consumers(non_authority_id), 3);
    });
}

#[test]
fn given_accounts_with_initial_balance_when_fixing_consumers_then_accounts_do_not_change() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        assert_eq!(consumers(authority_id), 1);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                authority_id
            )
        );
        assert_eq!(consumers(authority_id), 1);

        assert_eq!(consumers(non_authority_id), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(consumers(non_authority_id), 0);
    });
}

#[test]
fn given_accounts_with_reserved_balance_when_fixing_consumers_then_accounts_do_not_change() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let reserved_amount = 3_u128;
        assert_ok!(pallet_balances::Pallet::<TestRuntime>::reserve(
            &authority_id,
            reserved_amount
        ));
        assert_eq!(consumers(authority_id), 2);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                authority_id
            )
        );
        assert_eq!(consumers(authority_id), 2);

        assert_ok!(pallet_balances::Pallet::<TestRuntime>::reserve(
            &non_authority_id,
            reserved_amount
        ));
        assert_eq!(consumers(non_authority_id), 1);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(consumers(non_authority_id), 1);
    });
}

#[test]
fn given_bonded_accounts_balance_when_fixing_consumers_then_accounts_do_not_change() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let bonded = 123_u128;
        assert_ok!(pallet_staking::Pallet::<TestRuntime>::bond(
            RuntimeOrigin::signed(authority_id),
            bonded,
            RewardDestination::Controller
        ));

        assert_eq!(consumers(authority_id), 4);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                authority_id
            )
        );
        assert_eq!(consumers(authority_id), 4);

        assert_ok!(pallet_staking::Pallet::<TestRuntime>::bond(
            RuntimeOrigin::signed(non_authority_id),
            bonded,
            RewardDestination::Controller
        ));
        assert_eq!(consumers(non_authority_id), 3);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(consumers(non_authority_id), 3);
    });
}

#[test]
fn given_account_zero_consumers_some_reserved_when_fixing_consumers_then_consumers_increase() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let reserved_amount = 3_u128;
        assert_ok!(pallet_balances::Pallet::<TestRuntime>::reserve(
            &non_authority_id,
            reserved_amount
        ));
        frame_system::Pallet::<TestRuntime>::dec_consumers(&non_authority_id);
        assert_eq!(consumers(non_authority_id), 0);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersUnderflowFixed {
                who: non_authority_id
            }]
        );
        assert_eq!(consumers(non_authority_id), 1);
    });
}

#[test]
fn given_non_staking_account_with_vesting_lock_when_fixing_consumers_then_consumers_increase() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let locked = 3_u128;
        pallet_balances::Pallet::<TestRuntime>::set_lock(
            VESTING_ID,
            &non_authority_id,
            locked,
            WithdrawReasons::all(),
        );
        frame_system::Pallet::<TestRuntime>::dec_consumers(&non_authority_id);
        assert_eq!(consumers(non_authority_id), 1);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersUnderflowFixed {
                who: non_authority_id
            }]
        );

        assert_eq!(consumers(non_authority_id), 2);
    });
}

#[test]
fn given_nominator_account_with_staking_lock_when_fixing_consumers_then_consumers_increase() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let bonded = 123_u128;
        assert_ok!(pallet_staking::Pallet::<TestRuntime>::bond(
            RuntimeOrigin::signed(non_authority_id),
            bonded,
            RewardDestination::Controller
        ));
        frame_system::Pallet::<TestRuntime>::dec_consumers(&non_authority_id);
        assert_eq!(consumers(non_authority_id), 2);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersUnderflowFixed {
                who: non_authority_id
            }]
        );

        assert_eq!(consumers(non_authority_id), 3);
    });
}

#[test]
fn given_validator_with_stash_equal_to_consumer_when_fixing_consumers_then_consumers_increases() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let bonded = 123_u128;
        assert_ok!(pallet_staking::Pallet::<TestRuntime>::bond(
            RuntimeOrigin::signed(authority_id),
            bonded,
            RewardDestination::Controller
        ));
        frame_system::Pallet::<TestRuntime>::dec_consumers(&authority_id);
        assert_eq!(consumers(authority_id), 3);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(non_authority_id),
                authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersUnderflowFixed { who: authority_id }]
        );

        assert_eq!(consumers(authority_id), 4);
    });
}

#[test]
fn given_validator_with_stash_not_equal_to_controller_when_fixing_consumers_then_consumers_increases_on_controller_only(
) {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let bonded = 123_u128;
        assert_ok!(pallet_staking::Pallet::<TestRuntime>::bond(
            RuntimeOrigin::signed(authority_id),
            bonded,
            RewardDestination::Controller
        ));
        // direct manipulation on pallet storage is not possible from end user perspective,
        // but to mimic that scenario we need to directly set Bonded so stash != controller,
        // that is not possible to do via pallet staking API anymore
        // below procedure mimic what set_controller did back in 11 version, ie no manipulations
        // on consumers counter
        pallet_staking::Bonded::<TestRuntime>::set(authority_id, Some(non_authority_id));
        let ledger = pallet_staking::Ledger::<TestRuntime>::take(authority_id).unwrap();
        pallet_staking::Ledger::<TestRuntime>::set(non_authority_id, Some(ledger));

        frame_system::Pallet::<TestRuntime>::dec_consumers(&authority_id);
        assert_eq!(consumers(authority_id), 3);
        assert_eq!(consumers(non_authority_id), 0);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(non_authority_id),
                authority_id
            )
        );
        assert_eq!(pallet_operations_events().len(), 0);
        assert_eq!(consumers(authority_id), 3);

        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_underflow(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersUnderflowFixed {
                who: non_authority_id
            }]
        );
        assert_eq!(consumers(non_authority_id), 1);
    });
}
