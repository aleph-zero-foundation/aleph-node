use std::{collections::HashSet, iter::empty};

use aleph_client::{
    pallets::session::SessionApi,
    primitives::{CommitteeSeats, EraValidators},
    utility::BlocksApi,
    AccountId,
};
use log::debug;
use primitives::SessionIndex;

pub async fn get_and_test_members_for_session<C: SessionApi + BlocksApi>(
    connection: &C,
    seats: CommitteeSeats,
    era_validators: &EraValidators<AccountId>,
    session: SessionIndex,
) -> anyhow::Result<(Vec<AccountId>, Vec<AccountId>)> {
    let reserved_members_for_session =
        get_members_subset_for_session(seats.reserved_seats, &era_validators.reserved, session);
    let non_reserved_members_for_session = get_members_subset_for_session(
        seats.non_reserved_seats,
        &era_validators.non_reserved,
        session,
    );

    let reserved_members_bench =
        get_bench_members(&era_validators.reserved, &reserved_members_for_session);
    let non_reserved_members_bench = get_bench_members(
        &era_validators.non_reserved,
        &non_reserved_members_for_session,
    );
    let members_bench = empty()
        .chain(reserved_members_bench)
        .chain(non_reserved_members_bench)
        .collect();

    let members_active: Vec<_> = empty()
        .chain(reserved_members_for_session)
        .chain(non_reserved_members_for_session)
        .collect();

    let members_active_set: HashSet<_> = members_active.iter().cloned().collect();
    let block = connection.first_block_of_session(session).await?;
    let network_members: HashSet<_> = connection.get_validators(block).await.into_iter().collect();

    debug!(
        "expected era validators for session {}: reserved - {:?}, non-reserved - {:?}",
        session, era_validators.reserved, era_validators.non_reserved
    );
    debug!("Seats for session {}: {:?}", session, seats);
    debug!(
        "members for session - computed {:?} ; retrieved {:?}",
        members_active, network_members
    );

    assert_eq!(members_active_set, network_members);

    Ok((members_active, members_bench))
}

/// Computes a list of validators that should be elected for a given session, based on description in our elections pallet.
/// Panics if `nodes_per_session` is greater than length of `era_validators`.
pub fn get_members_subset_for_session(
    nodes_per_session: u32,
    era_validators: &[AccountId],
    session: SessionIndex,
) -> Vec<AccountId> {
    let validators_len = era_validators.len();
    let session: usize = session.try_into().unwrap();
    let nodes_per_session: usize = nodes_per_session.try_into().unwrap();
    assert!(nodes_per_session <= validators_len);
    let first_index = session.saturating_mul(nodes_per_session) % validators_len;

    era_validators
        .iter()
        .cycle()
        .skip(first_index)
        .take(nodes_per_session)
        .cloned()
        .collect()
}

fn get_bench_members(all_members: &[AccountId], members_active: &[AccountId]) -> Vec<AccountId> {
    all_members
        .iter()
        .filter(|account_id| !members_active.contains(account_id))
        .cloned()
        .collect()
}
