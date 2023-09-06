use aleph_client::{
    pallets::{
        staking::{StakingApi, StakingSudoApi},
        timestamp::TimestampApi,
    },
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus},
    TxStatus,
};
use primitives::{
    staking::era_payout, EraIndex, DEFAULT_SESSIONS_PER_ERA, DEFAULT_SESSION_PERIOD,
    MILLISECS_PER_BLOCK,
};

use crate::config::{setup_test, Config};

#[tokio::test]
pub async fn era_payouts_calculated_correctly() -> anyhow::Result<()> {
    let config = setup_test();
    normal_era_payout(config).await?;
    force_era_payout(config).await?;

    Ok(())
}

async fn get_era_duration<C: TimestampApi + BlocksApi>(era: EraIndex, connection: &C) -> u64 {
    let current_era_first_block = era * DEFAULT_SESSIONS_PER_ERA * DEFAULT_SESSION_PERIOD;
    let next_era_first_block = (era + 1) * DEFAULT_SESSIONS_PER_ERA * DEFAULT_SESSION_PERIOD;

    let current_era_first_block_hash = connection
        .get_block_hash(current_era_first_block)
        .await
        .unwrap();
    let next_era_first_block_hash = connection
        .get_block_hash(next_era_first_block)
        .await
        .unwrap();

    return connection
        .get_timestamp(next_era_first_block_hash)
        .await
        .unwrap()
        - connection
            .get_timestamp(current_era_first_block_hash)
            .await
            .unwrap();
}

async fn force_era_payout(config: &Config) -> anyhow::Result<()> {
    let root_connection = config.create_root_connection().await;
    root_connection.wait_for_era(3, BlockStatus::Best).await;
    let active_era = root_connection.get_active_era(None).await;

    let starting_session = active_era * DEFAULT_SESSIONS_PER_ERA;
    root_connection
        .wait_for_session(starting_session + 1, BlockStatus::Best)
        .await;

    // new era will start in the session `starting_session + 3`
    root_connection.force_new_era(TxStatus::InBlock).await?;
    root_connection
        .wait_for_session(starting_session + 3, BlockStatus::Best)
        .await;

    let actual_duration = get_era_duration(active_era - 1, &root_connection).await;
    let payout = root_connection.get_payout_for_era(active_era, None).await;

    let expected_era_duration = (3 * DEFAULT_SESSION_PERIOD) as u64 * MILLISECS_PER_BLOCK;
    let expected_payout = era_payout(expected_era_duration).0;

    assert_within_delta_interval(
        expected_era_duration,
        actual_duration,
        MILLISECS_PER_BLOCK,
        "era duration",
        "Probably chain hasn't started correctly, try rerunning the test",
    );
    assert_within_delta_interval(
        expected_payout,
        payout,
        era_payout(2 * MILLISECS_PER_BLOCK).0,
        "payout",
        "",
    );
    Ok(())
}

async fn normal_era_payout(config: &Config) -> anyhow::Result<()> {
    let root_connection = config.create_root_connection().await;

    root_connection.wait_for_era(2, BlockStatus::Best).await;
    let active_era = root_connection.get_active_era(None).await;

    let payout = root_connection
        .get_payout_for_era(active_era - 1, None)
        .await;
    let actual_duration = get_era_duration(active_era - 1, &root_connection).await;

    let expected_era_duration =
        (DEFAULT_SESSIONS_PER_ERA * DEFAULT_SESSION_PERIOD) as u64 * MILLISECS_PER_BLOCK;
    let expected_payout = era_payout(expected_era_duration).0;

    assert_within_delta_interval(
        expected_era_duration,
        actual_duration,
        MILLISECS_PER_BLOCK,
        "era duration",
        "Probably chain hasn't started correctly, try rerunning the test",
    );
    assert_within_delta_interval(
        expected_payout,
        payout,
        era_payout(2 * MILLISECS_PER_BLOCK).0,
        "payout",
        "",
    );

    Ok(())
}

fn assert_within_delta_interval<T>(
    expected: T,
    actual: T,
    delta: T,
    quantity_name: &str,
    extra_msg_on_fail: &str,
) where
    T: std::fmt::Display
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + Copy,
{
    let start = expected - delta;
    let end = expected + delta;
    let within_delta = start <= expected && expected <= end;
    assert!(
        within_delta,
        "{quantity_name} should fall within range: [{start}, {end}] but was {actual}. {extra_msg_on_fail}",
    );
}
