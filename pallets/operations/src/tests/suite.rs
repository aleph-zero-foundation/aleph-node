use std::{env::var, path::PathBuf};

use frame_support::{
    assert_ok,
    traits::{Currency, LockableCurrency, ReservableCurrency, WithdrawReasons},
    weights::Weight,
};
use pallet_contracts::{Code, CollectEvents, DebugInfo};
use pallet_staking::RewardDestination;
use sp_runtime::traits::Hash;

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

fn wat_root_dir() -> PathBuf {
    match var("CARGO_MANIFEST_DIR") {
        // When `CARGO_MANIFEST_DIR` is not set, Rust resolves relative paths from the root folder
        Err(_) => "pallets/operations/src/tests/data".into(),
        Ok(path) => PathBuf::from(path).join("src/tests/data"),
    }
}

/// Load a given wasm module represented by a .wat file and returns a wasm binary contents along
/// with its hash.
///
/// The fixture files are located under the `fixtures/` directory.
fn compile_module<T>(fixture_name: &str) -> anyhow::Result<(Vec<u8>, <T::Hashing as Hash>::Output)>
where
    T: frame_system::Config,
{
    let fixture_path = wat_root_dir().join(format!("{fixture_name}.wat"));
    let wasm_binary = wat::parse_file(fixture_path)?;
    let code_hash = T::Hashing::hash(&wasm_binary);
    Ok((wasm_binary, code_hash))
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

        assert_eq!(providers(authority_id), 1);
        // +1 consumers due to session keys are set
        // +1 consumers due to frozen > 0
        // +1 consumers due to bond()
        assert_eq!(consumers(authority_id), 3);
        // since there are > 0 consumers, so we can't transfer everything - half ot total
        // balance is locked due to bond()
        assert_eq!(usable_balance(authority_id), total_balance_authority / 2);

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
            &non_authority_id,
            reserved_amount
        ));
        assert_eq!(total_balance(non_authority_id), total_balance_non_authority);
        assert_eq!(
            free_balance(non_authority_id),
            total_balance_non_authority - reserved_amount
        );
        assert_eq!(reserved_balance(non_authority_id), reserved_amount);
        assert_eq!(providers(non_authority_id), 1);
        // +1 consumers since there is reserved balance
        assert_eq!(consumers(non_authority_id), 1);
        // since consumers > 0
        assert_eq!(
            usable_balance(non_authority_id),
            total_balance_non_authority - reserved_amount - ed()
        );
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
            RuntimeOrigin::signed(non_authority_id),
            bonded,
            RewardDestination::Stash
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
        // +1 consumers from bond()
        assert_eq!(consumers(non_authority_id), 2);
    });
}

#[test]
fn given_accounts_with_initial_balance_when_fixing_consumers_then_counters_are_valid() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        assert_eq!(consumers(authority_id), 3);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                authority_id
            )
        );
        assert_eq!(consumers(authority_id), 3);

        assert_eq!(consumers(non_authority_id), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(consumers(non_authority_id), 0);

        assert_eq!(pallet_operations_events().len(), 0);
    });
}

#[test]
fn given_accounts_with_reserved_balance_when_fixing_consumers_then_counters_are_valid() {
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
        assert_eq!(consumers(authority_id), 3);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                authority_id
            )
        );
        assert_eq!(consumers(authority_id), 3);

        assert_ok!(pallet_balances::Pallet::<TestRuntime>::reserve(
            &non_authority_id,
            reserved_amount
        ));
        assert_eq!(consumers(non_authority_id), 1);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(consumers(non_authority_id), 1);

        assert_eq!(pallet_operations_events().len(), 0);
    });
}

#[test]
fn given_bonded_accounts_balance_when_fixing_consumers_then_counters_are_valid() {
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
            RewardDestination::Stash
        ));
        assert_eq!(consumers(non_authority_id), 2);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(consumers(non_authority_id), 2);

        assert_eq!(pallet_operations_events().len(), 0);
    });
}

#[test]
fn given_contract_accounts_when_fixing_consumers_then_counters_are_valid() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = UNITS * 100;
    let total_balance_non_authority = UNITS * 100;

    let (wasm, _code_hash) = compile_module::<TestRuntime>("transfer_return_code").unwrap();

    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        let contract_addr = pallet_contracts::Pallet::<TestRuntime>::bare_instantiate(
            non_authority_id,
            100,
            Weight::from_parts(100_000_000_000, 3 * 1024 * 1024),
            None,
            Code::Upload(wasm),
            vec![],
            vec![],
            DebugInfo::Skip,
            CollectEvents::Skip,
        )
        .result
        .unwrap()
        .account_id;

        // +1 since it's a contract account
        // +1 since it has some reserved funds
        assert_eq!(consumers(contract_addr), 2);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(consumers(contract_addr), 2);

        assert_eq!(pallet_operations_events().len(), 0);
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
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersCounterIncremented {
                who: non_authority_id
            }]
        );
        assert_eq!(consumers(non_authority_id), 1);
    });
}

#[test]
fn given_non_staking_account_with_vesting_lock_when_fixing_consumers_then_counters_are_valid() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        assert_eq!(consumers(non_authority_id), 0);
        let locked = 3_u128;
        pallet_balances::Pallet::<TestRuntime>::set_lock(
            VESTING_ID,
            &non_authority_id,
            locked,
            WithdrawReasons::all(),
        );
        // +1 due to frozen > 0
        // locks does not contribute to consumers counter anymore
        assert_eq!(consumers(non_authority_id), 1);

        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(pallet_operations_events().len(), 0);
        assert_eq!(consumers(non_authority_id), 1);
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
            RewardDestination::Stash
        ));
        // +1 due to frozen > 0
        // +1 due to bond()
        assert_eq!(consumers(non_authority_id), 2);
        frame_system::Pallet::<TestRuntime>::dec_consumers(&non_authority_id);
        assert_eq!(consumers(non_authority_id), 1);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersCounterIncremented {
                who: non_authority_id
            }]
        );

        assert_eq!(consumers(non_authority_id), 2);
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
        // +1 due to frozen > 0
        // +1 due to bond()
        // +1 due to session keys
        assert_eq!(consumers(authority_id), 3);
        frame_system::Pallet::<TestRuntime>::dec_consumers(&authority_id);
        assert_eq!(consumers(authority_id), 2);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(non_authority_id),
                authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersCounterIncremented { who: authority_id }]
        );

        assert_eq!(consumers(authority_id), 3);
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
        // +1 due to frozen > 0
        // +1 due to bond()
        // +1 due to session keys
        assert_eq!(consumers(authority_id), 3);
        // direct manipulation on pallet storage is not possible from end user perspective,
        // but to mimic that scenario we need to directly set Bonded so stash != controller,
        // that is not possible to do via pallet staking API anymore
        // below procedure mimic what set_controller did back in 11 version, ie no manipulations
        // on consumers counter
        pallet_staking::Bonded::<TestRuntime>::set(authority_id, Some(non_authority_id));
        let ledger = pallet_staking::Ledger::<TestRuntime>::take(authority_id).unwrap();
        pallet_staking::Ledger::<TestRuntime>::set(non_authority_id, Some(ledger));

        frame_system::Pallet::<TestRuntime>::dec_consumers(&authority_id);
        assert_eq!(consumers(authority_id), 2);
        assert_eq!(consumers(non_authority_id), 0);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(non_authority_id),
                authority_id
            )
        );
        assert_eq!(pallet_operations_events().len(), 0);
        assert_eq!(consumers(authority_id), 2);

        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersCounterIncremented {
                who: non_authority_id
            }]
        );
        assert_eq!(consumers(non_authority_id), 1);
    });
}

#[test]
fn given_account_zero_consumers_when_fixing_consumers_then_nothing_changes() {
    let authority_id = 1_u64;
    let non_authority_id = 2_u64;
    let total_balance_authority = 1000_u128;
    let total_balance_non_authority = 999_u128;
    new_test_ext(&[
        (authority_id, true, total_balance_authority),
        (non_authority_id, false, total_balance_non_authority),
    ])
    .execute_with(|| {
        assert_eq!(consumers(non_authority_id), 0);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(authority_id),
                non_authority_id
            )
        );
        assert_eq!(pallet_operations_events().len(), 0);
        assert_eq!(consumers(non_authority_id), 0);
    });
}

#[test]
fn given_nominator_account_with_staking_lock_and_consumer_overflow_when_fixing_consumers_then_consumers_decrease(
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
        // +1 from bond
        // +1 from frozen > 0
        // +1 from session keys
        // locks does not contribute to consumers counter anymore
        assert_eq!(consumers(authority_id), 3);
        frame_system::Pallet::<TestRuntime>::inc_consumers_without_limit(&authority_id).unwrap();
        assert_eq!(consumers(authority_id), 4);
        frame_system::Pallet::<TestRuntime>::reset_events();
        assert_eq!(pallet_operations_events().len(), 0);
        assert_ok!(
            crate::Pallet::<TestRuntime>::fix_accounts_consumers_counter(
                RuntimeOrigin::signed(non_authority_id),
                authority_id
            )
        );
        assert_eq!(
            pallet_operations_events(),
            [crate::Event::ConsumersCounterDecremented { who: authority_id }]
        );

        assert_eq!(consumers(authority_id), 3);
    });
}
