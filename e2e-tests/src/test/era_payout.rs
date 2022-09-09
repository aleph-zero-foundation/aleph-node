use aleph_client::{
    create_connection, get_current_era, get_payout_for_era, staking_force_new_era,
    wait_for_next_era, wait_for_session, ReadStorage, XtStatus,
};
use primitives::{
    staking::era_payout, Balance, EraIndex, DEFAULT_SESSIONS_PER_ERA, DEFAULT_SESSION_PERIOD,
    MILLISECS_PER_BLOCK,
};

use crate::Config;

pub fn era_payouts_calculated_correctly(config: &Config) -> anyhow::Result<()> {
    normal_era_payout(config)?;
    force_era_payout(config)?;

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

fn wait_to_second_era<C: ReadStorage>(connection: &C) -> EraIndex {
    let current_era = get_current_era(connection);
    if current_era < 2 {
        wait_for_next_era(connection).expect("Era is active");
        wait_for_next_era(connection).expect("Era is active");
    }
    get_current_era(connection)
}

fn force_era_payout(config: &Config) -> anyhow::Result<()> {
    let root_connection = config.create_root_connection();
    let current_era = wait_to_second_era(&root_connection);
    wait_for_next_era(&root_connection)?;
    let current_era = current_era + 1;

    let starting_session = current_era * DEFAULT_SESSIONS_PER_ERA;
    wait_for_session(&root_connection, starting_session + 1)?;

    // new era will start in the session `starting_session + 3`
    staking_force_new_era(&root_connection, XtStatus::InBlock);
    wait_for_session(&root_connection, starting_session + 3)?;

    let payout = get_payout_for_era(&root_connection, current_era);
    let expected_payout = era_payout((3 * DEFAULT_SESSION_PERIOD) as u64 * MILLISECS_PER_BLOCK).0;

    payout_within_two_block_delta(expected_payout, payout);

    Ok(())
}

fn normal_era_payout(config: &Config) -> anyhow::Result<()> {
    let connection = create_connection(&config.node);

    let current_era = wait_to_second_era(&connection);
    let payout = get_payout_for_era(&connection, current_era - 1);
    let expected_payout = era_payout(
        (DEFAULT_SESSIONS_PER_ERA * DEFAULT_SESSION_PERIOD) as u64 * MILLISECS_PER_BLOCK,
    )
    .0;

    payout_within_two_block_delta(expected_payout, payout);

    Ok(())
}
