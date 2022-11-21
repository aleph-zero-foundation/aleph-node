use primitives::{Balance, BlockNumber};

use crate::{api, api::runtime_types, AccountId, BlockHash, SignedConnection, TxStatus};

pub type CallHash = [u8; 32];
pub type Call = Vec<u8>;
pub type Timepoint = api::runtime_types::pallet_multisig::Timepoint<BlockNumber>;
pub type Multisig = runtime_types::pallet_multisig::Multisig<BlockNumber, Balance, AccountId>;

#[async_trait::async_trait]
pub trait MultisigUserApi {
    async fn approve_as_multi(
        &self,
        threshold: u16,
        other_signatories: Vec<AccountId>,
        timepoint: Option<Timepoint>,
        max_weight: u64,
        call_hash: CallHash,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
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
impl MultisigUserApi for SignedConnection {
    async fn approve_as_multi(
        &self,
        threshold: u16,
        other_signatories: Vec<AccountId>,
        timepoint: Option<Timepoint>,
        max_weight: u64,
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
