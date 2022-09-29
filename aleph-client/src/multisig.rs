use std::{
    collections::HashSet,
    fmt::{Debug, Formatter},
};

use anyhow::{ensure, Result};
use codec::{Decode, Encode};
use log::{error, info};
use primitives::Balance;
use sp_core::{blake2_256, crypto::AccountId32, Pair};
use sp_runtime::traits::TrailingZeroInput;
use substrate_api_client::{compose_extrinsic, ExtrinsicParams, XtStatus::Finalized};
use thiserror::Error;

use crate::{
    account_from_keypair, try_send_xt, AccountId, AnyConnection, BlockNumber, Extrinsic,
    SignedConnection, H256,
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
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Error)]
pub enum MultisigError {
    #[error("👪❌ Threshold should be between 2 and {0}.")]
    IncorrectThreshold(usize),
    #[error("👪❌ There should be at least 2 unique members.")]
    TooFewMembers,
    #[error("👪❌ There is no such member in the party.")]
    NoSuchMember,
    #[error("👪❌ There is no entry for this multisig aggregation in the pallet storage.")]
    NoAggregationFound,
    #[error("👪❌ Trying to report approval for a different call that already registered.")]
    CallConflict,
    #[error("👪❌ Only the author can cancel aggregation.")]
    NotAuthor,
    #[error("👪❌ The connection is signed by an account that doesn't match to any member.")]
    NonMemberSignature,
}

type CallHash = [u8; 32];
type Call = Vec<u8>;
type Timepoint = pallet_multisig::Timepoint<BlockNumber>;

type ApproveAsMultiCall = Extrinsic<(
    [u8; 2],           // call index
    u16,               // threshold
    Vec<AccountId32>,  // other signatories
    Option<Timepoint>, // timepoint, `None` for initiating
    CallHash,          // call hash
    u64,               // max weight
)>;

type AsMultiCall = Extrinsic<(
    [u8; 2],           // call index
    u16,               // threshold
    Vec<AccountId32>,  // other signatories
    Option<Timepoint>, // timepoint, `None` for initiating
    Call,              // call
    bool,              // whether to store call
    u64,               // max weight
)>;

type CancelAsMultiCall = Extrinsic<(
    [u8; 2],          // call index
    u16,              // threshold
    Vec<AccountId32>, // other signatories
    Timepoint,        // timepoint, `None` for initiating
    CallHash,         // call hash
)>;

pub fn compute_call_hash<CallDetails: Encode>(call: &Extrinsic<CallDetails>) -> CallHash {
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
#[derive(Eq, PartialEq, Debug)]
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
#[derive(Clone)]
pub struct MultisigParty {
    /// Derived multiparty account (public key).
    account: AccountId,
    /// *Sorted* collection of members.
    members: Vec<AccountId>,
    /// Minimum required approvals.
    threshold: u16,
}

impl Debug for MultisigParty {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultisigParty")
            .field("account", &self.account)
            .field("threshold", &self.threshold)
            .field("member count", &self.members.len())
            .finish()
    }
}

impl MultisigParty {
    /// Creates new party. `members` does *not* have to be already sorted. Also:
    /// - `members` must be of length between 2 and `pallet_multisig::MaxSignatories`;
    ///    since checking the upperbound is expensive, it is the caller's responsibility
    ///    to ensure it is not exceeded
    /// - `members` may contain duplicates, but they are ignored and not counted to the cardinality
    /// - `threshold` must be between 2 and `members.len()`
    pub fn new(members: &[AccountId], threshold: u16) -> Result<Self> {
        let mut members = members.to_vec();
        members.sort();
        members.dedup();

        ensure!(2 <= members.len(), MultisigError::TooFewMembers);
        ensure!(
            2 <= threshold && threshold <= members.len() as u16,
            MultisigError::IncorrectThreshold(members.len())
        );

        let account = Self::multi_account_id(&members, threshold);
        Ok(Self {
            account,
            members,
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
    /// *Note:* if this function changes in some newer Substrate version, this code should be adjusted.
    pub fn multi_account_id(who: &[AccountId], threshold: u16) -> AccountId {
        let entropy = (b"modlpy/utilisuba", who, threshold).using_encoded(blake2_256);
        Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
            .expect("infinite length input; no invalid inputs for type; qed")
    }

    /// Provide the address corresponding to the party (and the threshold).
    pub fn get_account(&self) -> AccountId {
        self.account.clone()
    }

    /// This is a convenience method, as usually you may want to perform an action
    /// as a particular member, without sorting their public keys on the callee side.
    pub fn get_member_index(&self, member: AccountId) -> Result<usize> {
        self.members
            .binary_search(&member)
            .map_err(|_| MultisigError::NoSuchMember.into())
    }

    /// For all extrinsics we have to sign them with the caller (representative) and pass
    /// accounts of the other party members (represented).
    ///
    /// Assumes that `representative_idx` is a valid index for `self.members`.
    fn designate_represented(&self, representative_idx: usize) -> Vec<AccountId> {
        let mut members = self.members.clone();
        members.remove(representative_idx);
        members
    }

    /// Compose extrinsic for `multisig::approve_as_multi` call.
    fn construct_approve_as_multi(
        &self,
        connection: &SignedConnection,
        other_signatories: Vec<AccountId>,
        timepoint: Option<Timepoint>,
        call_hash: CallHash,
    ) -> ApproveAsMultiCall {
        compose_extrinsic!(
            connection.as_connection(),
            "Multisig",
            "approve_as_multi",
            self.threshold,
            other_signatories,
            timepoint,
            call_hash,
            MAX_WEIGHT
        )
    }

    /// Tries sending `xt` with `connection` and waits for its finalization. Returns the hash
    /// of the containing block.
    fn finalize_xt<C: AnyConnection, T: Encode>(
        &self,
        connection: &C,
        xt: Extrinsic<T>,
        description: &'static str,
    ) -> Result<H256> {
        Ok(try_send_xt(connection, xt, Some(description), Finalized)?
            .expect("For `Finalized` status a block hash should be returned"))
    }

    /// Reads the pallet storage and takes the timestamp regarding procedure for the `self` party
    /// initiated at `block_hash`.
    fn get_timestamp<C: AnyConnection>(
        &self,
        connection: &C,
        call_hash: &CallHash,
        block_hash: H256,
    ) -> Result<Timepoint> {
        let multisig: Multisig = connection
            .as_connection()
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

    /// Checks whether `connection` is signed by some member and if so, returns their index.
    fn map_signer_to_member_index(&self, connection: &SignedConnection) -> Result<usize> {
        self.members
            .binary_search(&account_from_keypair(&connection.signer))
            .map_err(|_| MultisigError::NonMemberSignature.into())
    }

    /// Effectively starts the aggregation process by calling `approveAsMulti`.
    ///
    /// `connection` should be signed by some member.
    pub fn initiate_aggregation_with_hash(
        &self,
        connection: &SignedConnection,
        call_hash: CallHash,
    ) -> Result<SignatureAggregation> {
        let author_idx = self.map_signer_to_member_index(connection)?;

        let other_signatories = self.designate_represented(author_idx);
        let xt = self.construct_approve_as_multi(connection, other_signatories, None, call_hash);

        let block_hash = self.finalize_xt(connection, xt, "Initiate multisig aggregation")?;
        info!(target: "aleph-client", "Initiating multisig aggregation for call hash: {:?}", call_hash);

        Ok(SignatureAggregation {
            timepoint: self.get_timestamp(connection, &call_hash, block_hash)?,
            author: author_idx,
            call_hash,
            call: None,
            approvers: HashSet::from([self.members[author_idx].clone()]),
        })
    }

    /// Compose extrinsic for `multisig::as_multi` call.
    fn construct_as_multi<CallDetails: Encode>(
        &self,
        connection: &SignedConnection,
        other_signatories: Vec<AccountId>,
        timepoint: Option<Timepoint>,
        call: Extrinsic<CallDetails>,
        store_call: bool,
    ) -> AsMultiCall {
        compose_extrinsic!(
            connection.as_connection(),
            "Multisig",
            "as_multi",
            self.threshold,
            other_signatories,
            timepoint,
            call.function.encode(),
            store_call,
            MAX_WEIGHT
        )
    }

    /// Effectively starts aggregation process by calling `asMulti`.
    ///
    /// `connection` should be signed by some member.
    pub fn initiate_aggregation_with_call<CallDetails: Encode + Clone>(
        &self,
        connection: &SignedConnection,
        call: Extrinsic<CallDetails>,
        store_call: bool,
    ) -> Result<SignatureAggregation> {
        let author_idx = self.map_signer_to_member_index(connection)?;

        let xt = self.construct_as_multi(
            connection,
            self.designate_represented(author_idx),
            None,
            call.clone(),
            store_call,
        );

        let block_hash =
            self.finalize_xt(connection, xt, "Initiate multisig aggregation with call")?;

        let call_hash = compute_call_hash(&call);
        info!(target: "aleph-client", "Initiating multisig aggregation for call hash: {:?}", call_hash);

        Ok(SignatureAggregation {
            timepoint: self.get_timestamp(connection, &call_hash, block_hash)?,
            author: author_idx,
            call_hash,
            call: Some(call.encode()),
            approvers: HashSet::from([self.members[author_idx].clone()]),
        })
    }

    /// Report approval for `sig_agg` aggregation.
    ///
    /// `connection` should be signed by some member.
    pub fn approve(
        &self,
        connection: &SignedConnection,
        mut sig_agg: SignatureAggregation,
    ) -> Result<SignatureAggregation> {
        let member_idx = self.map_signer_to_member_index(connection)?;

        let xt = self.construct_approve_as_multi(
            connection,
            self.designate_represented(member_idx),
            Some(sig_agg.timepoint),
            sig_agg.call_hash,
        );

        self.finalize_xt(connection, xt, "Report approval to multisig aggregation")?;

        info!(target: "aleph-client", "Registered multisig approval for call hash: {:?}", sig_agg.call_hash);
        sig_agg.approvers.insert(self.members[member_idx].clone());
        Ok(sig_agg)
    }

    /// Report approval for `sig_agg` aggregation.
    ///     
    /// `connection` should be signed by some member.
    pub fn approve_with_call<CallDetails: Encode + Clone>(
        &self,
        connection: &SignedConnection,
        mut sig_agg: SignatureAggregation,
        call: Extrinsic<CallDetails>,
        store_call: bool,
    ) -> Result<SignatureAggregation> {
        let member_idx = self.map_signer_to_member_index(connection)?;
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

        let xt = self.construct_as_multi(
            connection,
            self.designate_represented(member_idx),
            Some(sig_agg.timepoint),
            call.clone(),
            store_call,
        );

        self.finalize_xt(
            connection,
            xt,
            "Report approval to multisig aggregation with call",
        )?;

        info!(target: "aleph-client", "Registered multisig approval for call hash: {:?}", sig_agg.call_hash);
        sig_agg.approvers.insert(self.members[member_idx].clone());
        sig_agg.call = Some(call.encode());
        Ok(sig_agg)
    }

    /// Compose extrinsic for `multisig::cancel_as_multi` call.
    fn construct_cancel_as_multi(
        &self,
        connection: &SignedConnection,
        other_signatories: Vec<AccountId>,
        timepoint: Timepoint,
        call_hash: CallHash,
    ) -> CancelAsMultiCall {
        compose_extrinsic!(
            connection.as_connection(),
            "Multisig",
            "cancel_as_multi",
            self.threshold,
            other_signatories,
            timepoint,
            call_hash
        )
    }

    /// Cancel `sig_agg` aggregation.
    ///
    /// `connection` should be signed by the aggregation author.
    pub fn cancel(
        &self,
        connection: &SignedConnection,
        sig_agg: SignatureAggregation,
    ) -> Result<()> {
        let author_idx = self.map_signer_to_member_index(connection)?;
        ensure!(sig_agg.author == author_idx, MultisigError::NotAuthor);

        let xt = self.construct_cancel_as_multi(
            connection,
            self.designate_represented(author_idx),
            sig_agg.timepoint,
            sig_agg.call_hash,
        );
        self.finalize_xt(connection, xt, "Cancel multisig aggregation")?;
        info!(target: "aleph-client", "Cancelled multisig aggregation for call hash: {:?}", sig_agg.call_hash);
        Ok(())
    }
}

/// Dispatch `call` on behalf of the multisig party of `connection.get_signer()` and
/// `other_signatories` with threshold 1.
///
/// `other_signatories` *must* be sorted (according to the natural ordering on `AccountId`).
pub fn perform_multisig_with_threshold_1<CallDetails: Encode + Clone>(
    connection: &SignedConnection,
    other_signatories: &[AccountId],
    call: CallDetails,
) -> Result<()> {
    let xt = compose_extrinsic!(
        connection.as_connection(),
        "Multisig",
        "as_multi_threshold_1",
        other_signatories,
        call
    );
    try_send_xt(connection, xt, Some("Multisig with threshold 1"), Finalized)?
        .expect("For `Finalized` status a block hash should be returned");
    Ok(())
}
