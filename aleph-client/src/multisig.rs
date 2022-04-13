use std::collections::HashSet;

use anyhow::{ensure, Result};
use codec::{Decode, Encode};
use log::{error, info};
use primitives::Balance;
use sp_core::{blake2_256, crypto::AccountId32, Pair};
use substrate_api_client::{compose_extrinsic, XtStatus::Finalized};
use thiserror::Error;

use crate::{
    account_from_keypair, try_send_xt, AccountId, BlockNumber, Connection, KeyPair,
    UncheckedExtrinsicV4, H256,
};

/// `MAX_WEIGHT` is the extrinsic parameter specifying upperbound for executing approved call.
/// Unless the approval is final, it has no effect. However, if due to your approval the
/// threshold is reached, you will be charged for execution process. By setting `max_weight`
/// low enough, you can avoid paying and left it for another member.
///
/// However, passing such parameter everytime is cumbersome and introduces the need of either
/// estimating call weight or setting very high universal bound at every caller side.
/// Thus, we keep a fairly high limit, which should cover almost any call (0.05 token).
const MAX_WEIGHT: u64 = 500_000_000;

/// Gathers all possible errors from this module.
#[derive(Debug, Error)]
pub enum MultisigError {
    #[error("👪❌ Threshold should be between 2 and {0}.")]
    IncorrectThreshold(usize),
    #[error("👪❌ There should be at least 2 unique members.")]
    TooFewMembers,
    #[error("👪❌ There is no available member at the provided index.")]
    IncorrectMemberIndex,
    #[error("👪❌ There is no such member in the party.")]
    NoSuchMember,
    #[error("👪❌ There is no entry for this multisig aggregation in the pallet storage.")]
    NoAggregationFound,
    #[error("👪❌ Trying to report approval for a different call that already registered.")]
    CallConflict,
    #[error("👪❌ Only the author can cancel aggregation.")]
    NotAuthor,
}

type CallHash = [u8; 32];
type Call = Vec<u8>;
type Timepoint = pallet_multisig::Timepoint<BlockNumber>;

type ApproveAsMultiCall = UncheckedExtrinsicV4<(
    [u8; 2],           // call index
    u16,               // threshold
    Vec<AccountId32>,  // other signatories
    Option<Timepoint>, // timepoint, `None` for initiating
    CallHash,          // call hash
    u64,               // max weight
)>;

type AsMultiCall = UncheckedExtrinsicV4<(
    [u8; 2],           // call index
    u16,               // threshold
    Vec<AccountId32>,  // other signatories
    Option<Timepoint>, // timepoint, `None` for initiating
    Call,              // call
    bool,              // whether to store call
    u64,               // max weight
)>;

type CancelAsMultiCall = UncheckedExtrinsicV4<(
    [u8; 2],          // call index
    u16,              // threshold
    Vec<AccountId32>, // other signatories
    Timepoint,        // timepoint, `None` for initiating
    CallHash,         // call hash
)>;

pub fn compute_call_hash<CallDetails: Encode>(
    call: &UncheckedExtrinsicV4<CallDetails>,
) -> CallHash {
    blake2_256(&call.function.encode())
}

/// Unfortunately, we have to copy this struct from the pallet. We can get such object from storage
/// but there is no way of accessing the info within nor interacting in any manner 💩.
#[derive(Clone, Decode)]
#[allow(dead_code)]
struct Multisig {
    when: Timepoint,
    deposit: Balance,
    depositor: AccountId,
    approvals: Vec<AccountId>,
}

/// This represents the ongoing procedure of aggregating approvals among members
/// of multisignature party.
#[derive(Debug)]
pub struct SignatureAggregation {
    /// The point in 'time' when the aggregation was initiated on the chain.
    /// Internally it is a pair: number of the block containing initial call and the position
    /// of the corresponding extrinsic within block.
    ///
    /// It is actually the easiest (and the chosen) way of distinguishing between
    /// independent aggregations within the same party for the same call.
    timepoint: Timepoint,
    /// The member, who initiated the aggregation. They also had to deposit money, and they
    /// are the only person with power of canceling the procedure.
    ///
    /// We keep just their index within the (sorted) set of members.
    author: usize,
    /// The hash of the target call.
    call_hash: CallHash,
    /// The call to be dispatched. Maybe.
    call: Option<Call>,
    /// We keep counting approvals, just for information.
    approvers: HashSet<AccountId>,
}

impl SignatureAggregation {
    /// How many approvals has already been aggregated.
    pub fn num_of_approvals(&self) -> usize {
        self.approvers.len()
    }
}

/// `MultisigParty` is representing a multiparty entity constructed from
/// a group of accounts (`members`) and a threshold (`threshold`).
pub struct MultisigParty {
    /// Derived multiparty account (public key).
    account: AccountId,
    /// *Sorted* collection of members.
    members: Vec<KeyPair>,
    /// Minimum required approvals.
    threshold: u16,
}

impl MultisigParty {
    /// Creates new party. `members` does *not* have to be already sorted. Also:
    /// - `members` must be of length between 2 and `pallet_multisig::MaxSignatories`;
    ///    since checking the upperbound is expensive, it is the caller's responsibility
    ///    to ensure it is not exceeded
    /// - `members` may contain duplicates, but they are ignored and not counted to the cardinality
    /// - `threshold` must be between 2 and `members.len()`
    pub fn new(members: Vec<KeyPair>, threshold: u16) -> Result<Self> {
        let mut members = members
            .iter()
            .map(|m| (m.clone(), account_from_keypair(m)))
            .collect::<Vec<_>>();

        members.sort_by_key(|(_, a)| a.clone());
        members.dedup_by(|(_, a1), (_, a2)| a1 == a2);

        ensure!(2 <= members.len(), MultisigError::TooFewMembers);
        ensure!(
            2 <= threshold && threshold <= members.len() as u16,
            MultisigError::IncorrectThreshold(members.len())
        );

        let (keypairs, accounts): (Vec<_>, Vec<_>) = members.iter().cloned().unzip();
        let account = Self::multi_account_id(&accounts, threshold);
        Ok(Self {
            account,
            members: keypairs,
            threshold,
        })
    }

    /// This method generates deterministic account id for a given set of members and a threshold.
    /// `who` must be sorted, otherwise the result will be incorrect.
    ///
    /// It comes from pallet multisig. However, since it is an associated method for a struct
    /// `pallet_multisig::Pallet<T: pallet_multisig::Config>` it is easier to just copy
    /// these two lines.
    ///
    /// *Note:* this function is a little different in the newer Substrate versions.
    /// After update, this code should be adjusted.
    pub fn multi_account_id(who: &[AccountId], threshold: u16) -> AccountId {
        let entropy = (b"modlpy/utilisuba", who, threshold).using_encoded(blake2_256);
        AccountId::decode(&mut &entropy[..]).unwrap_or_default()
    }

    /// Provide the address corresponding to the party (and the threshold).
    pub fn get_account(&self) -> AccountId {
        self.account.clone()
    }

    /// This is a convenience method, as usually you may want to perform an action
    /// as a particular member, without sorting their public keys on the callee side.
    pub fn get_member_index(&self, member: AccountId) -> Result<usize> {
        self.members
            .binary_search_by_key(&member, account_from_keypair)
            .map_err(|_| MultisigError::NoSuchMember.into())
    }

    /// For all extrinsics we have to sign it with the caller (representative) and pass
    /// accounts of the other party members.
    fn designate_representative_and_represented(&self, idx: usize) -> (KeyPair, Vec<AccountId>) {
        let mut members = self.members.clone();
        let member = members.remove(idx);
        let others = members.iter().map(account_from_keypair).collect();
        (member, others)
    }

    /// Compose extrinsic for `multisig::approve_as_multi` call. Assumes that `author_idx` is correct.
    fn construct_approve_as_multi(
        &self,
        connection: &Connection,
        timepoint: Option<Timepoint>,
        author_idx: usize,
        call_hash: CallHash,
    ) -> (ApproveAsMultiCall, Connection) {
        let (author, other_signatories) = self.designate_representative_and_represented(author_idx);
        let connection = connection.clone().set_signer(author);
        let xt = compose_extrinsic!(
            connection,
            "Multisig",
            "approve_as_multi",
            self.threshold,
            other_signatories,
            timepoint,
            call_hash,
            MAX_WEIGHT
        );
        (xt, connection)
    }

    /// Tries sending `xt` with `connection` and waits for its finalization. Returns the hash
    /// of the containing block.
    fn finalize_xt<T: Encode>(
        &self,
        connection: &Connection,
        xt: UncheckedExtrinsicV4<T>,
        description: &'static str,
    ) -> Result<H256> {
        Ok(try_send_xt(connection, xt, Some(description), Finalized)?
            .expect("For `Finalized` status a block hash should be returned"))
    }

    /// Reads the pallet storage and takes the timestamp regarding procedure for the `self` party
    /// initiated at `block_hash`.
    fn get_timestamp(
        &self,
        connection: &Connection,
        call_hash: &CallHash,
        block_hash: H256,
    ) -> Result<Timepoint> {
        let multisig: Multisig = connection
            .get_storage_double_map(
                "Multisig",
                "Multisigs",
                self.account.clone(),
                *call_hash,
                Some(block_hash),
            )?
            .ok_or(MultisigError::NoAggregationFound)?;
        Ok(multisig.when)
    }

    /// Checks whether `member_idx` is a proper position for `self.members`.
    fn ensure_index(&self, member_idx: usize) -> Result<()> {
        ensure!(
            member_idx < self.members.len(),
            MultisigError::IncorrectMemberIndex
        );
        Ok(())
    }

    /// Effectively starts the aggregation process by calling `approveAsMulti`. Does *not* expect
    /// that `connection` is already signed by the author.
    pub fn initiate_aggregation_with_hash(
        &self,
        connection: &Connection,
        call_hash: CallHash,
        author_idx: usize,
    ) -> Result<SignatureAggregation> {
        self.ensure_index(author_idx)?;

        let (xt, connection) =
            self.construct_approve_as_multi(connection, None, author_idx, call_hash);
        let block_hash = self.finalize_xt(&connection, xt, "Initiate multisig aggregation")?;
        info!(target: "aleph-client", "Initiating multisig aggregation for call hash: {:?}", call_hash);

        Ok(SignatureAggregation {
            timepoint: self.get_timestamp(&connection, &call_hash, block_hash)?,
            author: author_idx,
            call_hash,
            call: None,
            approvers: HashSet::from([account_from_keypair(&self.members[author_idx])]),
        })
    }

    /// Compose extrinsic for `multisig::as_multi` call. Assumes that `author_idx` is correct.
    fn construct_as_multi<CallDetails: Encode>(
        &self,
        connection: &Connection,
        timepoint: Option<Timepoint>,
        author_idx: usize,
        call: UncheckedExtrinsicV4<CallDetails>,
        store_call: bool,
    ) -> (AsMultiCall, Connection) {
        let (author, other_signatories) = self.designate_representative_and_represented(author_idx);
        let connection = connection.clone().set_signer(author);
        let xt = compose_extrinsic!(
            connection,
            "Multisig",
            "as_multi",
            self.threshold,
            other_signatories,
            timepoint,
            call.function.encode(),
            store_call,
            MAX_WEIGHT
        );
        (xt, connection)
    }

    /// Effectively starts aggregation process by calling `asMulti`.
    /// Does *not* expect that `connection` is already signed by the author.
    pub fn initiate_aggregation_with_call<CallDetails: Encode + Clone>(
        &self,
        connection: &Connection,
        call: UncheckedExtrinsicV4<CallDetails>,
        store_call: bool,
        author_idx: usize,
    ) -> Result<SignatureAggregation> {
        self.ensure_index(author_idx)?;

        let (xt, connection) =
            self.construct_as_multi(connection, None, author_idx, call.clone(), store_call);
        let block_hash =
            self.finalize_xt(&connection, xt, "Initiate multisig aggregation with call")?;

        let call_hash = compute_call_hash(&call);
        info!(target: "aleph-client", "Initiating multisig aggregation for call hash: {:?}", call_hash);

        Ok(SignatureAggregation {
            timepoint: self.get_timestamp(&connection, &call_hash, block_hash)?,
            author: author_idx,
            call_hash,
            call: Some(call.encode()),
            approvers: HashSet::from([account_from_keypair(&self.members[author_idx])]),
        })
    }

    /// Report approval for `sig_agg` aggregation. Does *not* expect that `connection` is already
    /// signed by the approving member.
    pub fn approve(
        &self,
        connection: &Connection,
        author_idx: usize,
        mut sig_agg: SignatureAggregation,
    ) -> Result<SignatureAggregation> {
        self.ensure_index(author_idx)?;

        let (xt, connection) = self.construct_approve_as_multi(
            connection,
            Some(sig_agg.timepoint),
            author_idx,
            sig_agg.call_hash,
        );
        self.finalize_xt(&connection, xt, "Report approval to multisig aggregation")?;

        info!(target: "aleph-client", "Registered multisig approval for call hash: {:?}", sig_agg.call_hash);
        sig_agg
            .approvers
            .insert(account_from_keypair(&self.members[author_idx]));
        Ok(sig_agg)
    }

    /// Report approval for `sig_agg` aggregation. Does *not* expect that `connection` is already
    /// signed by the approving member.
    pub fn approve_with_call<CallDetails: Encode + Clone>(
        &self,
        connection: &Connection,
        author_idx: usize,
        mut sig_agg: SignatureAggregation,
        call: UncheckedExtrinsicV4<CallDetails>,
        store_call: bool,
    ) -> Result<SignatureAggregation> {
        self.ensure_index(author_idx)?;
        if let Some(ref reported_call) = sig_agg.call {
            ensure!(
                reported_call.eq(&call.encode()),
                MultisigError::CallConflict
            );
        } else {
            ensure!(
                compute_call_hash(&call) == sig_agg.call_hash,
                MultisigError::CallConflict
            );
        }

        let (xt, connection) = self.construct_as_multi(
            connection,
            Some(sig_agg.timepoint),
            author_idx,
            call.clone(),
            store_call,
        );
        self.finalize_xt(
            &connection,
            xt,
            "Report approval to multisig aggregation with call",
        )?;

        info!(target: "aleph-client", "Registered multisig approval for call hash: {:?}", sig_agg.call_hash);
        sig_agg
            .approvers
            .insert(account_from_keypair(&self.members[author_idx]));
        sig_agg.call = Some(call.encode());
        Ok(sig_agg)
    }

    /// Compose extrinsic for `multisig::cancel_as_multi` call. Assumes that `author_idx` is correct.
    fn construct_cancel_as_multi(
        &self,
        connection: &Connection,
        timepoint: Timepoint,
        author_idx: usize,
        call_hash: CallHash,
    ) -> (CancelAsMultiCall, Connection) {
        let (author, other_signatories) = self.designate_representative_and_represented(author_idx);
        let connection = connection.clone().set_signer(author);
        let xt = compose_extrinsic!(
            connection,
            "Multisig",
            "cancel_as_multi",
            self.threshold,
            other_signatories,
            timepoint,
            call_hash
        );
        (xt, connection)
    }

    /// Cancel `sig_agg` aggregation. Does *not* expect that `connection` is already
    /// signed by the canceling member.
    pub fn cancel(
        &self,
        connection: &Connection,
        author_idx: usize,
        sig_agg: SignatureAggregation,
    ) -> Result<()> {
        self.ensure_index(author_idx)?;
        ensure!(sig_agg.author == author_idx, MultisigError::NotAuthor);

        let (xt, connection) = self.construct_cancel_as_multi(
            connection,
            sig_agg.timepoint,
            author_idx,
            sig_agg.call_hash,
        );
        self.finalize_xt(&connection, xt, "Cancel multisig aggregation")?;
        info!(target: "aleph-client", "Cancelled multisig aggregation for call hash: {:?}", sig_agg.call_hash);
        Ok(())
    }
}

/// Dispatch `call` as on behalf of the multisig party of `author` and `other_signatories`
/// with threshold 1. `connection` is *not* assumed to be already signed by `author`.
/// `other_signatories` *must* be sorted (according to the natural ordering on `AccountId`).
pub fn perform_multisig_with_threshold_1<CallDetails: Encode + Clone>(
    connection: &Connection,
    author: KeyPair,
    other_signatories: &[AccountId],
    call: CallDetails,
) -> Result<()> {
    let connection = connection.clone().set_signer(author);
    let xt = compose_extrinsic!(
        connection,
        "Multisig",
        "as_multi_threshold_1",
        other_signatories,
        call
    );
    try_send_xt(
        &connection,
        xt,
        Some("Multisig with threshold 1"),
        Finalized,
    )?
    .expect("For `Finalized` status a block hash should be returned");
    Ok(())
}
