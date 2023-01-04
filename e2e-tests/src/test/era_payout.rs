use aleph_client::{
    pallets::staking::{StakingApi, StakingSudoApi},
    waiting::{AlephWaiting, BlockStatus, WaitingExt},
    TxStatus,
};
use primitives::{
    staking::era_payout, Balance, EraIndex, DEFAULT_SESSIONS_PER_ERA, DEFAULT_SESSION_PERIOD,
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

fn payout_within_two_block_delta(expected_payout: Balance, payout: Balance) {
    let one_block = era_payout(2 * MILLISECS_PER_BLOCK).0;

    let start = expected_payout - one_block;
    let end = expected_payout + one_block;
    let within_delta = start <= payout && payout <= end;
    assert!(
        within_delta,
        "payout should fall within range: [{}, {}] but was {}",
        start, end, payout
    );
}

async fn wait_to_second_era<C: StakingApi + WaitingExt>(connection: &C) -> EraIndex {
    let active_era = connection.get_active_era(None).await;
    if active_era < 2 {
        connection.wait_for_n_eras(2, BlockStatus::Best).await;
    }
    connection.get_active_era(None).await
}

async fn force_era_payout(config: &Config) -> anyhow::Result<()> {
    let root_connection = config.create_root_connection().await;
    let active_era = wait_to_second_era(&root_connection).await;
    root_connection.wait_for_n_eras(1, BlockStatus::Best).await;
    let active_era = active_era + 1;

    let starting_session = active_era * DEFAULT_SESSIONS_PER_ERA;
    root_connection
        .wait_for_session(starting_session + 1, BlockStatus::Best)
        .await;

    // new era will start in the session `starting_session + 3`
    root_connection.force_new_era(TxStatus::InBlock).await?;
    root_connection
        .wait_for_session(starting_session + 3, BlockStatus::Best)
        .await;

    let payout = root_connection.get_payout_for_era(active_era, None).await;
    let expected_payout = era_payout((3 * DEFAULT_SESSION_PERIOD) as u64 * MILLISECS_PER_BLOCK).0;

    payout_within_two_block_delta(expected_payout, payout);

    Ok(())
}

async fn normal_era_payout(config: &Config) -> anyhow::Result<()> {
    let root_connection = config.create_root_connection().await;

    let active_era = wait_to_second_era(&root_connection).await;
    let payout = root_connection
        .get_payout_for_era(active_era - 1, None)
        .await;
    let expected_payout = era_payout(
        (DEFAULT_SESSIONS_PER_ERA * DEFAULT_SESSION_PERIOD) as u64 * MILLISECS_PER_BLOCK,
    )
    .0;

    payout_within_two_block_delta(expected_payout, payout);

    Ok(())
}
