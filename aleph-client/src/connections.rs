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
    api, sp_weights::weight_v2::Weight, AccountId, BlockHash, Call, KeyPair, SubxtClient, TxStatus,
};

#[derive(Clone)]
pub struct Connection {
    client: SubxtClient,
}

pub struct SignedConnection {
    connection: Connection,
    signer: KeyPair,
}

#[derive(Clone)]
pub struct RootConnection {
    connection: SignedConnection,
}

pub(crate) trait AsConnection {
    fn as_connection(&self) -> &Connection;
}

pub(crate) trait AsSigned {
    fn as_signed(&self) -> &SignedConnection;
}

#[async_trait::async_trait]
pub trait ConnectionApi: Sync {
    async fn get_storage_entry<T: DecodeWithMetadata + Sync, Defaultable: Sync, Iterable: Sync>(
        &self,
        addrs: &StaticStorageAddress<T, Yes, Defaultable, Iterable>,
        at: Option<BlockHash>,
    ) -> T::Target;

    async fn get_storage_entry_maybe<
        T: DecodeWithMetadata + Sync,
        Defaultable: Sync,
        Iterable: Sync,
    >(
        &self,
        addrs: &StaticStorageAddress<T, Yes, Defaultable, Iterable>,
        at: Option<BlockHash>,
    ) -> Option<T::Target>;

    async fn rpc_call<R: Decode>(&self, func_name: String, params: RpcParams) -> anyhow::Result<R>;
}

#[async_trait::async_trait]
pub trait SignedConnectionApi: ConnectionApi {
    async fn send_tx<Call: TxPayload + Send + Sync>(
        &self,
        tx: Call,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;

    async fn send_tx_with_params<Call: TxPayload + Send + Sync>(
        &self,
        tx: Call,
        params: BaseExtrinsicParamsBuilder<SubstrateConfig, PlainTip>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;

    fn account_id(&self) -> &AccountId;
    fn signer(&self) -> &KeyPair;
    async fn try_as_root(&self) -> anyhow::Result<RootConnection>;
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
        let sudo = api::tx().sudo().sudo_unchecked_weight(
            call,
            Weight {
                ref_time: 0,
                proof_size: 0,
            },
        );

        self.as_signed().send_tx(sudo, status).await
    }

    async fn sudo(&self, call: Call, status: TxStatus) -> anyhow::Result<BlockHash> {
        info!(target: "aleph-client", "sending call as sudo {:?}", call);
        let sudo = api::tx().sudo().sudo(call);

        self.as_signed().send_tx(sudo, status).await
    }
}

impl Clone for SignedConnection {
    fn clone(&self) -> Self {
        SignedConnection {
            connection: self.connection.clone(),
            signer: KeyPair::new(self.signer.signer().clone()),
        }
    }
}

impl AsConnection for Connection {
    fn as_connection(&self) -> &Connection {
        self
    }
}

impl<S: AsSigned> AsConnection for S {
    fn as_connection(&self) -> &Connection {
        &self.as_signed().connection
    }
}

impl AsSigned for SignedConnection {
    fn as_signed(&self) -> &SignedConnection {
        self
    }
}

impl AsSigned for RootConnection {
    fn as_signed(&self) -> &SignedConnection {
        &self.connection
    }
}

#[async_trait::async_trait]
impl<C: AsConnection + Sync> ConnectionApi for C {
    async fn get_storage_entry<T: DecodeWithMetadata + Sync, Defaultable: Sync, Iterable: Sync>(
        &self,
        addrs: &StaticStorageAddress<T, Yes, Defaultable, Iterable>,
        at: Option<BlockHash>,
    ) -> T::Target {
        self.get_storage_entry_maybe(addrs, at)
            .await
            .expect("There should be a value")
    }

    async fn get_storage_entry_maybe<
        T: DecodeWithMetadata + Sync,
        Defaultable: Sync,
        Iterable: Sync,
    >(
        &self,
        addrs: &StaticStorageAddress<T, Yes, Defaultable, Iterable>,
        at: Option<BlockHash>,
    ) -> Option<T::Target> {
        info!(target: "aleph-client", "accessing storage at {}::{} at block {:?}", addrs.pallet_name(), addrs.entry_name(), at);
        self.as_connection()
            .as_client()
            .storage()
            .fetch(addrs, at)
            .await
            .expect("Should access storage")
    }

    async fn rpc_call<R: Decode>(&self, func_name: String, params: RpcParams) -> anyhow::Result<R> {
        info!(target: "aleph-client", "submitting rpc call `{}`, with params {:?}", func_name, params);
        let bytes: Bytes = self
            .as_connection()
            .as_client()
            .rpc()
            .request(&func_name, params)
            .await?;

        Ok(R::decode(&mut bytes.as_ref())?)
    }
}

#[async_trait::async_trait]
impl<S: AsSigned + Sync> SignedConnectionApi for S {
    async fn send_tx<Call: TxPayload + Send + Sync>(
        &self,
        tx: Call,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        self.send_tx_with_params(tx, Default::default(), status)
            .await
    }

    async fn send_tx_with_params<Call: TxPayload + Send + Sync>(
        &self,
        tx: Call,
        params: BaseExtrinsicParamsBuilder<SubstrateConfig, PlainTip>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        if let Some(details) = tx.validation_details() {
            info!(target:"aleph-client", "Sending extrinsic {}.{} with params: {:?}", details.pallet_name, details.call_name, params);
        }

        let progress = self
            .as_connection()
            .as_client()
            .tx()
            .sign_and_submit_then_watch(&tx, self.as_signed().signer(), params)
            .await
            .map_err(|e| anyhow!("Failed to submit transaction: {:?}", e))?;

        // In case of Submitted hash does not mean anything
        let hash = match status {
            TxStatus::InBlock => progress.wait_for_in_block().await?.block_hash(),
            TxStatus::Finalized => progress.wait_for_finalized_success().await?.block_hash(),
            TxStatus::Submitted => return Ok(BlockHash::from_low_u64_be(0)),
        };
        info!(target: "aleph-client", "tx included in block {:?}", hash);

        Ok(hash)
    }

    fn account_id(&self) -> &AccountId {
        self.as_signed().signer().account_id()
    }

    fn signer(&self) -> &KeyPair {
        &self.as_signed().signer
    }

    async fn try_as_root(&self) -> anyhow::Result<RootConnection> {
        let temp = self.as_signed().clone();
        RootConnection::try_from_connection(temp.connection, temp.signer).await
    }
}

impl Connection {
    const DEFAULT_RETRIES: u32 = 10;
    const RETRY_WAIT_SECS: u64 = 1;

    pub async fn new(address: &str) -> Connection {
        Self::new_with_retries(address, Self::DEFAULT_RETRIES).await
    }

    async fn new_with_retries(address: &str, mut retries: u32) -> Connection {
        loop {
            let client = SubxtClient::from_url(&address).await;
            match (retries, client) {
                (_, Ok(client)) => return Connection { client },
                (0, Err(e)) => panic!("{:?}", e),
                _ => {
                    sleep(Duration::from_secs(Self::RETRY_WAIT_SECS));
                    retries -= 1;
                }
            }
        }
    }

    pub(crate) fn as_client(&self) -> &SubxtClient {
        &self.client
    }
}

impl SignedConnection {
    pub async fn new(address: &str, signer: KeyPair) -> Self {
        Self::from_connection(Connection::new(address).await, signer)
    }

    pub fn from_connection(connection: Connection, signer: KeyPair) -> Self {
        Self { connection, signer }
    }
}

impl RootConnection {
    pub async fn new(address: &str, root: KeyPair) -> anyhow::Result<Self> {
        RootConnection::try_from_connection(Connection::new(address).await, root).await
    }

    pub async fn try_from_connection(
        connection: Connection,
        signer: KeyPair,
    ) -> anyhow::Result<Self> {
        let root_address = api::storage().sudo().key();

        let root = match connection
            .as_client()
            .storage()
            .fetch(&root_address, None)
            .await
        {
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
            connection: SignedConnection { connection, signer },
        })
    }
}
