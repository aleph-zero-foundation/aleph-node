use crate::{try_send_xt, wait_for_event, AnyConnection, RootConnection, SignedConnection};
use ac_primitives::ExtrinsicParams;
use codec::Decode;
use frame_support::PalletId;
use primitives::{Balance, MILLISECS_PER_BLOCK};
use sp_core::{Pair, H256};
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use std::{thread, thread::sleep, time::Duration};
use substrate_api_client::{compose_extrinsic, ApiResult, GenericAddress, XtStatus};

type AnyResult<T> = anyhow::Result<T>;

/// Returns the account of the treasury.
pub fn treasury_account() -> AccountId32 {
    PalletId(*b"a0/trsry").into_account_truncating()
}

/// Returns how many treasury proposals have ever been created.
pub fn proposals_counter<C: AnyConnection>(connection: &C) -> u32 {
    connection
        .as_connection()
        .get_storage_value("Treasury", "ProposalCount", None)
        .expect("Key `Treasury::ProposalCount` should be present in storage")
        .unwrap_or(0)
}

/// Calculates how much balance will be paid out to the treasury after each era.
pub fn staking_treasury_payout<C: AnyConnection>(connection: &C) -> Balance {
    let sessions_per_era = connection
        .as_connection()
        .get_constant::<u32>("Staking", "SessionsPerEra")
        .expect("Constant `Staking::SessionsPerEra` should be present");
    let session_period = connection
        .as_connection()
        .get_constant::<u32>("Elections", "SessionPeriod")
        .expect("Constant `Elections::SessionPeriod` should be present");
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
        "Treasury",
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
        ("Treasury", "Rejected"),
        |e: ProposalRejectedEvent| proposal_id.eq(&e.proposal_id),
    )
    .map(|_| ())
}

fn send_rejection(connection: &RootConnection, proposal_id: u32) -> ApiResult<Option<H256>> {
    let xt = compose_extrinsic!(
        connection.as_connection(),
        "Treasury",
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
        "Treasury",
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

fn wait_for_approval<C: AnyConnection>(connection: &C, proposal_id: u32) -> AnyResult<()> {
    loop {
        let approvals: Vec<u32> = connection
            .as_connection()
            .get_storage_value("Treasury", "Approvals", None)
            .expect("Key `Treasury::Approvals` should be present in storage")
            .unwrap();
        if approvals.contains(&proposal_id) {
            return Ok(());
        } else {
            sleep(Duration::from_millis(500))
        }
    }
}
