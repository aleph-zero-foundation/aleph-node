use primitives::BlockNumber;

use crate::{
    api, sp_weights::weight_v2::Weight, AccountId, BlockHash, SignedConnectionApi, TxStatus,
};

/// An alias for a call hash.
pub type CallHash = [u8; 32];
/// An alias for a call.
pub type Call = Vec<u8>;
/// An alias for a timepoint.
pub type Timepoint = api::runtime_types::pallet_multisig::Timepoint<BlockNumber>;

/// Pallet multisig api.
#[async_trait::async_trait]
pub trait MultisigUserApi {
    /// API for [`approve_as_multi`](https://paritytech.github.io/substrate/master/pallet_multisig/pallet/struct.Pallet.html#method.approve_as_multi) call.
    async fn approve_as_multi(
        &self,
        threshold: u16,
        other_signatories: Vec<AccountId>,
        timepoint: Option<Timepoint>,
        max_weight: Weight,
        call_hash: CallHash,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;

    /// API for [`cancel_as_multi`](https://paritytech.github.io/substrate/master/pallet_multisig/pallet/struct.Pallet.html#method.cancel_as_multi) call.
    async fn cancel_as_multi(
        &self,
        threshold: u16,
        other_signatories: Vec<AccountId>,
        timepoint: Timepoint,
        call_hash: CallHash,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> MultisigUserApi for S {
    async fn approve_as_multi(
        &self,
        threshold: u16,
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
        threshold: u16,
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
