use crate::{
    config::Config,
    test::{
        authorities_are_staking as test_authorities_are_staking,
        ban_automatic as test_ban_automatic, batch_transactions as test_batch_transactions,
        change_stake_and_force_new_era as test_change_stake_and_force_new_era,
        change_validators as test_change_validators,
        channeling_fee_and_tip as test_channeling_fee_and_tip,
        clearing_session_count as test_clearing_session_count, disable_node as test_disable_node,
        era_payouts_calculated_correctly as test_era_payout, era_validators as test_era_validators,
        fee_calculation as test_fee_calculation, finalization as test_finalization,
        force_new_era as test_force_new_era, points_basic as test_points_basic,
        points_stake_change as test_points_stake_change,
        schedule_version_change as test_schedule_version_change,
        staking_era_payouts as test_staking_era_payouts,
        staking_new_validator as test_staking_new_validator, token_transfer as test_token_transfer,
        treasury_access as test_treasury_access, validators_rotate as test_validators_rotate,
    },
};

pub type TestCase = fn(&Config) -> anyhow::Result<()>;

pub type PossibleTestCases = Vec<(&'static str, TestCase)>;

/// Get a Vec with test cases that the e2e suite is able to handle.
/// The order of items is important for tests when more than one case is handled in order.
/// This comes up in local tests.
pub fn possible_test_cases() -> PossibleTestCases {
    vec![
        ("finalization", test_finalization as TestCase),
        ("version_upgrade", test_schedule_version_change),
        ("rewards_disable_node", test_disable_node as TestCase),
        ("token_transfer", test_token_transfer as TestCase),
        (
            "channeling_fee_and_tip",
            test_channeling_fee_and_tip as TestCase,
        ),
        ("treasury_access", test_treasury_access as TestCase),
        ("batch_transactions", test_batch_transactions as TestCase),
        ("staking_era_payouts", test_staking_era_payouts as TestCase),
        ("validators_rotate", test_validators_rotate as TestCase),
        (
            "staking_new_validator",
            test_staking_new_validator as TestCase,
        ),
        ("change_validators", test_change_validators as TestCase),
        ("fee_calculation", test_fee_calculation as TestCase),
        ("era_payout", test_era_payout as TestCase),
        ("era_validators", test_era_validators as TestCase),
        (
            "rewards_change_stake_and_force_new_era",
            test_change_stake_and_force_new_era as TestCase,
        ),
        ("points_basic", test_points_basic as TestCase),
        ("rewards_force_new_era", test_force_new_era as TestCase),
        ("rewards_stake_change", test_points_stake_change as TestCase),
        (
            "authorities_are_staking",
            test_authorities_are_staking as TestCase,
        ),
        (
            "clearing_session_count",
            test_clearing_session_count as TestCase,
        ),
        ("ban_automatic", test_ban_automatic as TestCase),
    ]
}
