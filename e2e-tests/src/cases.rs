use crate::{
    config::Config,
    test::{
        authorities_are_staking as test_authorities_are_staking,
        ban_automatic as test_ban_automatic, ban_manual as test_ban_manual,
        ban_threshold as test_ban_threshold, batch_transactions as test_batch_transactions,
        change_stake_and_force_new_era as test_change_stake_and_force_new_era,
        change_validators as test_change_validators,
        channeling_fee_and_tip as test_channeling_fee_and_tip,
        clearing_session_count as test_clearing_session_count, disable_node as test_disable_node,
        era_payouts_calculated_correctly as test_era_payout, era_validators as test_era_validators,
        fee_calculation as test_fee_calculation, finalization as test_finalization,
        force_new_era as test_force_new_era, points_basic as test_points_basic,
        points_stake_change as test_points_stake_change,
        schedule_doomed_version_change_and_verify_finalization_stopped as test_schedule_doomed_version_change_and_verify_finalization_stopped,
        schedule_version_change as test_schedule_version_change,
        staking_era_payouts as test_staking_era_payouts,
        staking_new_validator as test_staking_new_validator, token_transfer as test_token_transfer,
        treasury_access as test_treasury_access, validators_rotate as test_validators_rotate,
    },
};

pub async fn run_testcase(id: &str, config: &Config) -> anyhow::Result<()> {
    match id {
        "finalization" => test_finalization(config).await,
        "version_upgrade" => test_schedule_version_change(config).await,
        "rewards_disable_node" => test_disable_node(config).await,
        "token_transfer" => test_token_transfer(config).await,
        "channeling_fee_and_tip" => test_channeling_fee_and_tip(config).await,
        "treasury_access" => test_treasury_access(config).await,
        "batch_transactions" => test_batch_transactions(config).await,
        "staking_era_payouts" => test_staking_era_payouts(config).await,
        "validators_rotate" => test_validators_rotate(config).await,
        "staking_new_validator" => test_staking_new_validator(config).await,
        "change_validators" => test_change_validators(config).await,
        "fee_calculation" => test_fee_calculation(config).await,
        "era_payout" => test_era_payout(config).await,
        "era_validators" => test_era_validators(config).await,
        "rewards_change_stake_and_force_new_era" => {
            test_change_stake_and_force_new_era(config).await
        }
        "points_basic" => test_points_basic(config).await,
        "rewards_force_new_era" => test_force_new_era(config).await,
        "rewards_stake_change" => test_points_stake_change(config).await,
        "authorities_are_staking" => test_authorities_are_staking(config).await,

        "clearing_session_count" => test_clearing_session_count(config).await,
        "ban_automatic" => test_ban_automatic(config).await,
        "ban_manual" => test_ban_manual(config).await,
        "ban_threshold" => test_ban_threshold(config).await,
        "doomed_version_upgrade" => {
            test_schedule_doomed_version_change_and_verify_finalization_stopped(config).await
        }
        _ => panic!("unknown testcase"),
    }
}

pub async fn run_all_testcases(config: &Config) -> anyhow::Result<()> {
    let all = vec![
        "finalization",
        "version_upgrade",
        "rewards_disable_node",
        "token_transfer",
        "channeling_fee_and_tip",
        "treasury_access",
        "batch_transactions",
        "staking_era_payouts",
        "validators_rotate",
        "staking_new_validator",
        "change_validators",
        "fee_calculation",
        "era_payout",
        "era_validators",
        "rewards_change_stake_and_force_new_era",
        "points_basic",
        "rewards_force_new_era",
        "rewards_stake_change",
        "authorities_are_staking",
        "clearing_session_count",
        "ban_automatic",
        "ban_manual",
        "ban_threshold",
        "doomed_version_upgrade",
    ];

    for testcase in all {
        run_testcase(testcase, config).await?;
    }
    Ok(())
}
