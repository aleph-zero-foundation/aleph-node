use crate::{
    config::Config,
    test::{
        batch_transactions as test_batch_transactions, change_validators as test_change_validators,
        channeling_fee as test_channeling_fee, fee_calculation as test_fee_calculation,
        finalization as test_finalization, staking_era_payouts as test_staking_era_payouts,
        staking_new_validator as test_staking_new_validator, token_transfer as test_token_transfer,
        treasury_access as test_treasury_access, validators_rotate as test_elections,
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
        ("token_transfer", test_token_transfer as TestCase),
        ("channeling_fee", test_channeling_fee as TestCase),
        ("treasury_access", test_treasury_access as TestCase),
        ("batch_transactions", test_batch_transactions as TestCase),
        ("staking_era_payouts", test_staking_era_payouts as TestCase),
        ("members_rotate", test_elections as TestCase),
        (
            "staking_new_validator",
            test_staking_new_validator as TestCase,
        ),
        ("change_validators", test_change_validators as TestCase),
        ("fee_calculation", test_fee_calculation as TestCase),
    ]
}
