use std::{thread::sleep, time::Duration};

use anyhow::anyhow;
use codec::Decode;
use log::info;
use subxt::{
    ext::sp_core::Bytes,
    metadata::DecodeWithMetadata,
    rpc::RpcParams,
    storage::{address::Yes, StaticStorageAddress, StorageAddress},
    tx::{BaseExtrinsicParamsBuilder, PlainTip, TxPayload},
    SubstrateConfig,
};

use crate::{
    api, frame_support::weights::weight_v2::Weight, BlockHash, Call, Client, KeyPair, TxStatus,
};

#[derive(Clone)]
pub struct Connection {
    pub client: Client,
}

pub struct SignedConnection {
    pub connection: Connection,
    pub signer: KeyPair,
}

pub struct RootConnection {
    pub connection: Connection,
    pub root: KeyPair,
}

#[async_trait::async_trait]
pub trait SudoCall {
    async fn sudo_unchecked(&self, call: Call, status: TxStatus) -> anyhow::Result<BlockHash>;
    async fn sudo(&self, call: Call, status: TxStatus) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl SudoCall for RootConnection {
    async fn sudo_unchecked(&self, call: Call, status: TxStatus) -> anyhow::Result<BlockHash> {
        info!(target: "aleph-client", "sending call as sudo_unchecked {:?}", call);
        let sudo = api::tx()
            .sudo()
            .sudo_unchecked_weight(call, Weight { ref_time: 0 });

        self.as_signed().send_tx(sudo, status).await
    }

    async fn sudo(&self, call: Call, status: TxStatus) -> anyhow::Result<BlockHash> {
        info!(target: "aleph-client", "sending call as sudo {:?}", call);
        let sudo = api::tx().sudo().sudo(call);

        self.as_signed().send_tx(sudo, status).await
    }
}

impl Connection {
    const DEFAULT_RETRIES: u32 = 10;
    const RETRY_WAIT_SECS: u64 = 1;

    pub async fn new(address: String) -> Self {
        Self::new_with_retries(address, Self::DEFAULT_RETRIES).await
    }

    pub async fn new_with_retries(address: String, mut retries: u32) -> Self {
        loop {
            let client = Client::from_url(&address).await;
            match (retries, client) {
                (_, Ok(client)) => return Self { client },
                (0, Err(e)) => panic!("{:?}", e),
                _ => {
                    sleep(Duration::from_secs(Self::RETRY_WAIT_SECS));
                    retries -= 1;
                }
            }
        }
    }

    pub async fn get_storage_entry<T: DecodeWithMetadata, Defaultable, Iterable>(
        &self,
        addrs: &StaticStorageAddress<T, Yes, Defaultable, Iterable>,
        at: Option<BlockHash>,
    ) -> T::Target {
        self.get_storage_entry_maybe(addrs, at)
            .await
            .expect("There should be a value")
    }

    pub async fn get_storage_entry_maybe<T: DecodeWithMetadata, Defaultable, Iterable>(
        &self,
        addrs: &StaticStorageAddress<T, Yes, Defaultable, Iterable>,
        at: Option<BlockHash>,
    ) -> Option<T::Target> {
        info!(target: "aleph-client", "accessing storage at {}::{} at block {:?}", addrs.pallet_name(), addrs.entry_name(), at);
        self.client
            .storage()
            .fetch(addrs, at)
            .await
            .expect("Should access storage")
    }

    pub async fn rpc_call<R: Decode>(
        &self,
        func_name: String,
        params: RpcParams,
    ) -> anyhow::Result<R> {
        info!(target: "aleph-client", "submitting rpc call `{}`, with params {:?}", func_name, params);
        let bytes: Bytes = self.client.rpc().request(&func_name, params).await?;

        Ok(R::decode(&mut bytes.as_ref())?)
    }
}

impl SignedConnection {
    pub async fn new(address: String, signer: KeyPair) -> Self {
        Self::from_connection(Connection::new(address).await, signer)
    }

    pub fn from_connection(connection: Connection, signer: KeyPair) -> Self {
        Self { connection, signer }
    }

    pub async fn send_tx<Call: TxPayload>(
        &self,
        tx: Call,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        self.send_tx_with_params(tx, Default::default(), status)
            .await
    }

    pub async fn send_tx_with_params<Call: TxPayload>(
        &self,
        tx: Call,
        params: BaseExtrinsicParamsBuilder<SubstrateConfig, PlainTip>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        if let Some(details) = tx.validation_details() {
            info!(target:"aleph-client", "Sending extrinsic {}.{} with params: {:?}", details.pallet_name, details.call_name, params);
        }
        let progress = self
            .connection
            .client
            .tx()
            .sign_and_submit_then_watch(&tx, &self.signer, params)
            .await?;

        // In case of Submitted hash does not mean anything
        let hash = match status {
            TxStatus::InBlock => progress.wait_for_in_block().await?.block_hash(),
            TxStatus::Finalized => progress.wait_for_finalized_success().await?.block_hash(),
            TxStatus::Submitted => return Ok(BlockHash::from_low_u64_be(0)),
        };
        info!(target: "aleph-client", "tx included in block {:?}", hash);

        Ok(hash)
    }
}

impl RootConnection {
    pub async fn new(address: String, root: KeyPair) -> anyhow::Result<Self> {
        RootConnection::try_from_connection(Connection::new(address).await, root).await
    }

    pub async fn try_from_connection(
        connection: Connection,
        signer: KeyPair,
    ) -> anyhow::Result<Self> {
        let root_address = api::storage().sudo().key();

        let root = match connection.client.storage().fetch(&root_address, None).await {
            Ok(Some(account)) => account,
            _ => return Err(anyhow!("Could not read sudo key from chain")),
        };

        if root != *signer.account_id() {
            return Err(anyhow!(
                "Provided account is not a sudo on chain. sudo key - {}, provided: {}",
                root,
                signer.account_id()
            ));
        }

        Ok(Self {
            connection,
            root: signer,
        })
    }

    pub fn as_signed(&self) -> SignedConnection {
        SignedConnection {
            connection: self.connection.clone(),
            signer: KeyPair::new(self.root.signer().clone()),
        }
    }
}
