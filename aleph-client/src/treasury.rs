use std::{thread, thread::sleep, time::Duration};

use ac_primitives::ExtrinsicParams;
use codec::Decode;
use frame_support::PalletId;
use primitives::{Balance, MILLISECS_PER_BLOCK};
use sp_core::{Pair, H256};
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use substrate_api_client::{compose_extrinsic, ApiResult, GenericAddress, XtStatus};

use crate::{
    try_send_xt, wait_for_event, AnyConnection, ReadStorage, RootConnection, SignedConnection,
};

const PALLET: &str = "Treasury";

type AnyResult<T> = anyhow::Result<T>;

/// Returns the account of the treasury.
pub fn treasury_account() -> AccountId32 {
    PalletId(*b"a0/trsry").into_account_truncating()
}

/// Returns how many treasury proposals have ever been created.
pub fn proposals_counter<C: ReadStorage>(connection: &C) -> u32 {
    connection.read_storage_value_or_default(PALLET, "ProposalCount")
}

/// Calculates how much balance will be paid out to the treasury after each era.
pub fn staking_treasury_payout<C: ReadStorage>(connection: &C) -> Balance {
    let sessions_per_era: u32 = connection.read_constant("Staking", "SessionsPerEra");
    let session_period: u32 = connection.read_constant("Elections", "SessionPeriod");
    let millisecs_per_era = MILLISECS_PER_BLOCK * session_period as u64 * sessions_per_era as u64;
    primitives::staking::era_payout(millisecs_per_era).1
}

/// Creates a proposal of spending treasury's funds.
///
/// The intention is to transfer `value` balance to `beneficiary`. The signer of `connection` is the
/// proposer.
pub fn propose(
    connection: &SignedConnection,
    value: Balance,
    beneficiary: &AccountId32,
) -> ApiResult<Option<H256>> {
    let xt = compose_extrinsic!(
        connection.as_connection(),
        PALLET,
        "propose_spend",
        Compact(value),
        GenericAddress::Id(beneficiary.clone())
    );
    try_send_xt(connection, xt, Some("treasury spend"), XtStatus::Finalized)
}

/// Approves proposal with id `proposal_id` and waits (in a loop) until pallet storage is updated.
///
/// Unfortunately, pallet treasury does not emit any event (like while rejecting), so we have to
/// keep reading storage to be sure. Hence, it may be an expensive call.
///
/// Be careful, since execution might never end.
pub fn approve(connection: &RootConnection, proposal_id: u32) -> AnyResult<()> {
    send_approval(connection, proposal_id)?;
    wait_for_approval(connection, proposal_id)
}

/// Rejects proposal with id `proposal_id` and waits for the corresponding event.
///
/// Be careful, since execution might never end (we may stuck on waiting for the event forever).
pub fn reject(connection: &RootConnection, proposal_id: u32) -> AnyResult<()> {
    let listener = {
        let (c, p) = (connection.clone(), proposal_id);
        thread::spawn(move || wait_for_rejection(&c, p))
    };
    send_rejection(connection, proposal_id)?;
    listener
        .join()
        .expect("Corresponding event should have been emitted")
}

#[derive(Debug, Decode, Copy, Clone)]
struct ProposalRejectedEvent {
    proposal_id: u32,
    _slashed: Balance,
}

fn wait_for_rejection<C: AnyConnection>(connection: &C, proposal_id: u32) -> AnyResult<()> {
    wait_for_event(
        connection,
        (PALLET, "Rejected"),
        |e: ProposalRejectedEvent| proposal_id.eq(&e.proposal_id),
    )
    .map(|_| ())
}

fn send_rejection(connection: &RootConnection, proposal_id: u32) -> ApiResult<Option<H256>> {
    let xt = compose_extrinsic!(
        connection.as_connection(),
        PALLET,
        "reject_proposal",
        Compact(proposal_id)
    );
    try_send_xt(
        connection,
        xt,
        Some("treasury rejection"),
        XtStatus::Finalized,
    )
}

fn send_approval(connection: &RootConnection, proposal_id: u32) -> ApiResult<Option<H256>> {
    let xt = compose_extrinsic!(
        connection.as_connection(),
        PALLET,
        "approve_proposal",
        Compact(proposal_id)
    );
    try_send_xt(
        connection,
        xt,
        Some("treasury approval"),
        XtStatus::Finalized,
    )
}

fn wait_for_approval<C: ReadStorage>(connection: &C, proposal_id: u32) -> AnyResult<()> {
    loop {
        let approvals: Vec<u32> = connection.read_storage_value(PALLET, "Approvals");
        if approvals.contains(&proposal_id) {
            return Ok(());
        } else {
            sleep(Duration::from_millis(500))
        }
    }
}
