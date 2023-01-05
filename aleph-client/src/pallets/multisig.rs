use std::{collections::HashSet, marker::PhantomData};

use anyhow::{anyhow, ensure};
use codec::{Decode, Encode};
use primitives::{Balance, BlockNumber};
use sp_core::blake2_256;
use sp_runtime::traits::TrailingZeroInput;

use crate::{
    account_from_keypair, aleph_runtime::RuntimeCall, api, api::runtime_types,
    sp_weights::weight_v2::Weight, AccountId, BlockHash, ConnectionApi, SignedConnectionApi,
    TxStatus,
};

/// An alias for a call hash.
pub type CallHash = [u8; 32];
/// An alias for a call.
pub type Call = RuntimeCall;
/// An alias for a threshold.
pub type MultisigThreshold = u16;
/// An alias for a timepoint.
pub type Timepoint = runtime_types::pallet_multisig::Timepoint<BlockNumber>;
/// An alias for a multisig structure in the pallet storage.
pub type Multisig = runtime_types::pallet_multisig::Multisig<BlockNumber, Balance, AccountId>;

/// `MAX_WEIGHT` is the extrinsic parameter specifying upperbound for executing approved call.
/// Unless the approval is final, it has no effect. However, if due to your approval the
/// threshold is reached, you will be charged for execution process. By setting `max_weight`
/// low enough, you can avoid paying and left it for another member.
///
/// However, passing such parameter everytime is cumbersome and introduces the need of either
/// estimating call weight or setting very high universal bound at every caller side.
/// Thus, we keep a fairly high limit, which should cover almost any call (0.05 token).
pub const DEFAULT_MAX_WEIGHT: Weight = Weight::new(500_000_000, 0);

/// Pallet multisig api.
#[async_trait::async_trait]
pub trait MultisigUserApi {
    /// API for [`as_multi_threshold_1`](https://paritytech.github.io/substrate/master/pallet_multisig/pallet/struct.Pallet.html#method.as_multi_threshold_1) call.
    async fn as_multi_threshold_1(
        &self,
        other_signatories: Vec<AccountId>,
        call: Call,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    /// API for [`as_multi`](https://paritytech.github.io/substrate/master/pallet_multisig/pallet/struct.Pallet.html#method.as_multi) call.
    async fn as_multi(
        &self,
        threshold: MultisigThreshold,
        other_signatories: Vec<AccountId>,
        timepoint: Option<Timepoint>,
        max_weight: Weight,
        call: Call,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    /// API for [`approve_as_multi`](https://paritytech.github.io/substrate/master/pallet_multisig/pallet/struct.Pallet.html#method.approve_as_multi) call.
    async fn approve_as_multi(
        &self,
        threshold: MultisigThreshold,
        other_signatories: Vec<AccountId>,
        timepoint: Option<Timepoint>,
        max_weight: Weight,
        call_hash: CallHash,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    /// API for [`cancel_as_multi`](https://paritytech.github.io/substrate/master/pallet_multisig/pallet/struct.Pallet.html#method.cancel_as_multi) call.
    async fn cancel_as_multi(
        &self,
        threshold: MultisigThreshold,
        other_signatories: Vec<AccountId>,
        timepoint: Timepoint,
        call_hash: CallHash,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> MultisigUserApi for S {
    async fn as_multi_threshold_1(
        &self,
        other_signatories: Vec<AccountId>,
        call: Call,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx()
            .multisig()
            .as_multi_threshold_1(other_signatories, call);

        self.send_tx(tx, status).await
    }

    async fn as_multi(
        &self,
        threshold: MultisigThreshold,
        other_signatories: Vec<AccountId>,
        timepoint: Option<Timepoint>,
        max_weight: Weight,
        call: Call,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx().multisig().as_multi(
            threshold,
            other_signatories,
            timepoint,
            call,
            max_weight,
        );

        self.send_tx(tx, status).await
    }

    async fn approve_as_multi(
        &self,
        threshold: MultisigThreshold,
        other_signatories: Vec<AccountId>,
        timepoint: Option<Timepoint>,
        max_weight: Weight,
        call_hash: CallHash,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx().multisig().approve_as_multi(
            threshold,
            other_signatories,
            timepoint,
            call_hash,
            max_weight,
        );

        self.send_tx(tx, status).await
    }

    async fn cancel_as_multi(
        &self,
        threshold: MultisigThreshold,
        other_signatories: Vec<AccountId>,
        timepoint: Timepoint,
        call_hash: CallHash,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx().multisig().cancel_as_multi(
            threshold,
            other_signatories,
            timepoint,
            call_hash,
        );

        self.send_tx(tx, status).await
    }
}

/// A group of accounts together with a threshold.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct MultisigParty {
    signatories: Vec<AccountId>,
    threshold: MultisigThreshold,
}

impl MultisigParty {
    /// Create new party from `signatories` and `threshold`.
    ///
    /// `signatories` can contain duplicates and doesn't have to be sorted. However, there must be
    /// at least 2 unique accounts. There is also a virtual upper bound - `MaxSignatories` constant.
    /// It isn't checked here (since it requires client), however, using too big party will fail
    /// when performing any chain interaction.
    ///
    /// `threshold` must be between 2 and number of unique accounts in `signatories`. For threshold
    /// 1, use special method `MultisigUserApi::as_multi_threshold_1`.
    pub fn new(signatories: &[AccountId], threshold: MultisigThreshold) -> anyhow::Result<Self> {
        let mut sorted_signatories = signatories.to_vec();
        sorted_signatories.sort();
        sorted_signatories.dedup();

        ensure!(
            sorted_signatories.len() > 1,
            "There must be at least 2 different signatories"
        );
        ensure!(
            sorted_signatories.len() >= threshold as usize,
            "Threshold must not be greater than the number of unique signatories"
        );
        ensure!(
            threshold >= 2,
            "Threshold must be at least 2 - for threshold 1, use `as_multi_threshold_1`"
        );

        Ok(Self {
            signatories: sorted_signatories,
            threshold,
        })
    }

    /// The multisig account derived from signatories and threshold.
    ///
    /// This method is copied from the pallet, because:
    ///  -  we don't want to add a new dependency
    ///  -  we cannot instantiate pallet object here anyway (the corresponding functionality exists
    ///     as pallet's method rather than standalone function)
    pub fn account(&self) -> AccountId {
        let entropy =
            (b"modlpy/utilisuba", &self.signatories, &self.threshold).using_encoded(blake2_256);
        Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
            .expect("infinite length input; no invalid inputs for type; qed")
    }
}

/// Pallet multisig functionality that is not directly related to any pallet call.
#[async_trait::async_trait]
pub trait MultisigApiExt {
    /// Get the coordinate that corresponds to the ongoing signature aggregation for `party_account`
    /// and `call_hash`.
    async fn get_timepoint(
        &self,
        party_account: &AccountId,
        call_hash: &CallHash,
        block_hash: Option<BlockHash>,
    ) -> Timepoint;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> MultisigApiExt for C {
    async fn get_timepoint(
        &self,
        party_account: &AccountId,
        call_hash: &CallHash,
        block_hash: Option<BlockHash>,
    ) -> Timepoint {
        let multisigs = api::storage()
            .multisig()
            .multisigs(party_account, call_hash);
        let Multisig { when, .. } = self.get_storage_entry(&multisigs, block_hash).await;
        when
    }
}

/// We will mark context object as either ongoing procedure or a closed one. However, we put this
/// distinction to the type level, so instead of enum, we use a trait.
pub trait ContextState {}

/// Context of the signature aggregation that is still in progress.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Ongoing {}
impl ContextState for Ongoing {}

/// Context of the signature aggregation that was either successfully performed or canceled.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Closed {}
impl ContextState for Closed {}

/// A context in which ongoing signature aggregation is performed.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Context<S: ContextState> {
    /// The entity for which aggregation is being made.
    party: MultisigParty,
    /// Derived multisig account (the source of the target call).
    author: AccountId,

    /// Pallet's coordinate for this aggregation.
    timepoint: Timepoint,
    /// Weight limit when dispatching the call.
    max_weight: Weight,

    /// The target dispatchable, if already provided.
    call: Option<Call>,
    /// The hash of the target dispatchable.
    call_hash: CallHash,

    /// The set of accounts, that already approved the call (via this context object), including the
    /// author.
    ///
    /// `approvers.len() < party.threshold` always holds.
    approvers: HashSet<AccountId>,

    _phantom: PhantomData<S>,
}

/// After approval action, our context can be in two modes - either for further use (`Ongoing`), or
/// read only (`Closed`).
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ContextAfterUse {
    /// Signature aggregation is in progress.
    Ongoing(Context<Ongoing>),
    /// Signature aggregation was either successfully performed or was canceled.
    Closed(Context<Closed>),
}

impl Context<Ongoing> {
    fn new(
        party: MultisigParty,
        author: AccountId,
        timepoint: Timepoint,
        max_weight: Weight,
        call: Option<Call>,
        call_hash: CallHash,
    ) -> Self {
        Self {
            party,
            author: author.clone(),
            timepoint,
            max_weight,
            call,
            call_hash,
            approvers: HashSet::from([author]),
            _phantom: PhantomData,
        }
    }

    /// In case `Context` object has been passed somewhere, where this limit should be adjusted, we
    /// allow for that.
    ///
    /// Actually, this isn't used until threshold is met, so such changing is perfectly safe.
    pub fn change_max_weight(&mut self, max_weight: Weight) {
        self.max_weight = max_weight;
    }

    /// Set `call` only if `self.call_hash` is matching.
    fn set_call(&mut self, call: Call) -> anyhow::Result<()> {
        ensure!(
            self.call_hash == compute_call_hash(&call),
            "Call doesn't match to the registered hash"
        );
        self.call = Some(call);
        Ok(())
    }

    /// Register another approval. Depending on the threshold meeting and `call` content, we treat
    /// signature aggregation process as either still ongoing or closed.
    fn add_approval(mut self, approver: AccountId) -> ContextAfterUse {
        self.approvers.insert(approver);
        if self.call.is_some() && self.approvers.len() >= (self.party.threshold as usize) {
            ContextAfterUse::Closed(self.close())
        } else {
            ContextAfterUse::Ongoing(self)
        }
    }

    /// Casting to the closed variant. Private, so that the user don't accidentally call `into()`
    /// and close ongoing context.
    fn close(self) -> Context<Closed> {
        Context::<Closed> {
            party: self.party,
            author: self.author,
            timepoint: self.timepoint,
            max_weight: self.max_weight,
            call: self.call,
            call_hash: self.call_hash,
            approvers: self.approvers,
            _phantom: Default::default(),
        }
    }
}

impl Context<Closed> {
    /// Read party.
    pub fn party(&self) -> &MultisigParty {
        &self.party
    }
    /// Read author.
    pub fn author(&self) -> &AccountId {
        &self.author
    }
    /// Read timepoint.
    pub fn timepoint(&self) -> &Timepoint {
        &self.timepoint
    }
    /// Read max weight.
    pub fn max_weight(&self) -> &Weight {
        &self.max_weight
    }
    /// Read call.
    pub fn call(&self) -> &Option<Call> {
        &self.call
    }
    /// Read call hash.
    pub fn call_hash(&self) -> CallHash {
        self.call_hash
    }
    /// Read approvers set.
    pub fn approvers(&self) -> &HashSet<AccountId> {
        &self.approvers
    }
}

/// Pallet multisig API, but suited for cases when the whole scenario is performed in a single place
/// - we keep data in a context object which helps in concise programming.
#[async_trait::async_trait]
pub trait MultisigContextualApi {
    /// Start signature aggregation for `party` and `call_hash`. Get `Context` object as a result
    /// (together with standard block hash).
    ///
    /// This is the recommended way of initialization.
    async fn initiate(
        &self,
        party: &MultisigParty,
        max_weight: &Weight,
        call_hash: CallHash,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, Context<Ongoing>)>;
    /// Start signature aggregation for `party` and `call`. Get `Context` object as a result
    /// (together with standard block hash).
    ///
    /// Note: it is usually a better idea to pass `call` only with the final approval (so that it
    /// isn't stored on-chain).
    async fn initiate_with_call(
        &self,
        party: &MultisigParty,
        max_weight: &Weight,
        call: Call,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, Context<Ongoing>)>;
    /// Express contextual approval for the call hash.
    ///
    /// This is the recommended way for every intermediate approval.
    async fn approve(
        &self,
        context: Context<Ongoing>,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, ContextAfterUse)>;
    /// Express contextual approval for the `call`.
    ///
    /// This is the recommended way only for the final approval.
    async fn approve_with_call(
        &self,
        context: Context<Ongoing>,
        call: Option<Call>,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, ContextAfterUse)>;
    /// Cancel signature aggregation.
    async fn cancel(
        &self,
        context: Context<Ongoing>,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, Context<Closed>)>;
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> MultisigContextualApi for S {
    async fn initiate(
        &self,
        party: &MultisigParty,
        max_weight: &Weight,
        call_hash: CallHash,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, Context<Ongoing>)> {
        let other_signatories = ensure_signer_in_party(self, party)?;

        let block_hash = self
            .approve_as_multi(
                party.threshold,
                other_signatories,
                None,
                max_weight.clone(),
                call_hash,
                status,
            )
            .await?;

        // Even though `subxt` allows us to get timepoint when waiting for the submission
        // confirmation (see e.g. `ExtrinsicEvents` object that is returned from
        // `wait_for_finalized_success`), we chose to perform one additional storage read.
        // Firstly, because of brevity here (we would have to duplicate some lines from
        // `connections` module. Secondly, if `Timepoint` struct change, this method (reading raw
        // extrinsic position) might become incorrect.
        let timepoint = self
            .get_timepoint(&party.account(), &call_hash, Some(block_hash))
            .await;

        Ok((
            block_hash,
            Context::new(
                party.clone(),
                self.account_id().clone(),
                timepoint,
                max_weight.clone(),
                None,
                call_hash,
            ),
        ))
    }

    async fn initiate_with_call(
        &self,
        party: &MultisigParty,
        max_weight: &Weight,
        call: Call,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, Context<Ongoing>)> {
        let other_signatories = ensure_signer_in_party(self, party)?;

        let block_hash = self
            .as_multi(
                party.threshold,
                other_signatories,
                None,
                max_weight.clone(),
                call.clone(),
                status,
            )
            .await?;

        let call_hash = compute_call_hash(&call);
        let timepoint = self
            .get_timepoint(&party.account(), &call_hash, Some(block_hash))
            .await;

        Ok((
            block_hash,
            Context::new(
                party.clone(),
                self.account_id().clone(),
                timepoint,
                max_weight.clone(),
                Some(call.clone()),
                call_hash,
            ),
        ))
    }

    async fn approve(
        &self,
        context: Context<Ongoing>,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, ContextAfterUse)> {
        let other_signatories = ensure_signer_in_party(self, &context.party)?;

        self.approve_as_multi(
            context.party.threshold,
            other_signatories,
            Some(context.timepoint.clone()),
            context.max_weight.clone(),
            context.call_hash,
            status,
        )
        .await
        .map(|block_hash| (block_hash, context.add_approval(self.account_id().clone())))
    }

    async fn approve_with_call(
        &self,
        mut context: Context<Ongoing>,
        call: Option<Call>,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, ContextAfterUse)> {
        let other_signatories = ensure_signer_in_party(self, &context.party)?;

        let call = match (call.as_ref(), context.call.as_ref()) {
            (None, None) => Err(anyhow!(
                "Call wasn't provided earlier - you must pass it now"
            )),
            (None, Some(call)) => Ok(call),
            (Some(call), None) => {
                context.set_call(call.clone())?;
                Ok(call)
            }
            (Some(saved_call), Some(new_call)) => {
                ensure!(
                    saved_call == new_call,
                    "The call is different that the one used previously"
                );
                Ok(new_call)
            }
        }?;

        self.as_multi(
            context.party.threshold,
            other_signatories,
            Some(context.timepoint.clone()),
            context.max_weight.clone(),
            call.clone(),
            status,
        )
        .await
        .map(|block_hash| (block_hash, context.add_approval(self.account_id().clone())))
    }

    async fn cancel(
        &self,
        context: Context<Ongoing>,
        status: TxStatus,
    ) -> anyhow::Result<(BlockHash, Context<Closed>)> {
        let other_signatories = ensure_signer_in_party(self, &context.party)?;

        ensure!(
            *self.account_id() == context.author,
            "Only the author can cancel multisig aggregation"
        );

        let block_hash = self
            .cancel_as_multi(
                context.party.threshold,
                other_signatories,
                context.timepoint.clone(),
                context.call_hash,
                status,
            )
            .await?;

        Ok((block_hash, context.close()))
    }
}

/// Compute hash of `call`.
pub fn compute_call_hash(call: &Call) -> CallHash {
    call.using_encoded(blake2_256)
}

/// Ensure that the signer of `conn` is present in `party.signatories`. If so, return all other
/// signatories.
fn ensure_signer_in_party<S: SignedConnectionApi>(
    conn: &S,
    party: &MultisigParty,
) -> anyhow::Result<Vec<AccountId>> {
    let signer_account = account_from_keypair(conn.signer().signer());
    if let Ok(index) = party.signatories.binary_search(&signer_account) {
        let mut other_signatories = party.signatories.clone();
        other_signatories.remove(index);
        Ok(other_signatories)
    } else {
        Err(anyhow!("Connection should be signed by a party member"))
    }
}
