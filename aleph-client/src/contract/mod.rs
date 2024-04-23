//! Contains types and functions simplifying common contract-related operations.
//!
//! For example, you could write this wrapper around (some of) the functionality of PSP22
//! contracts using the building blocks provided by this module:
//!
//! ```no_run
//! # use anyhow::{Result, Context};
//! # use aleph_client::{AccountId, Balance};
//! # use aleph_client::{Connection, SignedConnection, TxInfo};
//! # use aleph_client::contract::ContractInstance;
//! #
//! #[derive(Debug)]
//! struct PSP22TokenInstance {
//!     contract: ContractInstance,
//! }
//!
//! impl PSP22TokenInstance {
//!     fn new(address: AccountId, metadata_path: &Option<String>) -> Result<Self> {
//!         let metadata_path = metadata_path
//!             .as_ref()
//!             .context("PSP22Token metadata not set.")?;
//!         Ok(Self {
//!             contract: ContractInstance::new(address, metadata_path)?,
//!         })
//!     }
//!
//!     async fn transfer(&self, conn: &SignedConnection, to: AccountId, amount: Balance) -> Result<TxInfo> {
//!         self.contract.exec(
//!             conn,
//!             "PSP22::transfer",
//!             vec![to.to_string().as_str(), amount.to_string().as_str(), "0x00"].as_slice(),
//!             Default::default()
//!         ).await
//!     }
//!
//!     async fn balance_of(&self, conn: &Connection, account: AccountId) -> Result<Balance> {
//!         self.contract.read(
//!             conn,
//!             "PSP22::balance_of",
//!             &vec![account.to_string().as_str()],
//!             Default::default()
//!         ).await?
//!     }
//! }
//! ```

mod convertible_value;
pub mod event;

use std::fmt::{Debug, Formatter};

use anyhow::{anyhow, Context, Result};
use contract_transcode::ContractMessageTranscoder;
pub use convertible_value::ConvertibleValue;
use log::info;
use pallet_contracts::ContractExecResult;
use serde::__private::Clone;

use crate::{
    connections::TxInfo,
    contract_transcode::Value,
    pallets::contract::{ContractCallArgs, ContractRpc, ContractsUserApi, EventRecord},
    sp_weights::weight_v2::Weight,
    AccountId, Balance, BlockHash, ConnectionApi, SignedConnectionApi, TxStatus,
};

/// Represents a contract instantiated on the chain.
pub struct ContractInstance {
    address: AccountId,
    transcoder: ContractMessageTranscoder,
}

/// Builder for read only contract call
#[derive(Debug, Clone, Default)]
pub struct ReadonlyCallParams {
    at: Option<BlockHash>,
    sender: Option<AccountId>,
}

impl ReadonlyCallParams {
    /// Creates a new instance of `ReadonlyCallParams`.
    pub fn new() -> Self {
        Default::default()
    }
    /// Sets the block hash to execute the call at. If not set, by default the latest block is used.
    pub fn at(mut self, at: BlockHash) -> Self {
        self.at = Some(at);
        self
    }

    /// Overriders `sender` of the contract call as if it was executed by them. If not set,
    /// by default the contract address is used.
    pub fn sender(mut self, sender: AccountId) -> Self {
        self.sender = Some(sender);
        self
    }
}

/// Builder for a contract call that will be submitted to chain
#[derive(Debug, Clone, Default)]
pub struct ExecCallParams {
    value: Balance,
    max_gas: Option<Weight>,
}

impl ExecCallParams {
    /// Creates a new instance of `ExecCallParams`.
    pub fn new() -> Self {
        Default::default()
    }
    /// Sets the `value` balance to send with the call.
    pub fn value(mut self, value: Balance) -> Self {
        self.value = value;
        self
    }

    /// Sets the `gas_limit` in the call.
    pub fn gas_limit(mut self, max_gas: Weight) -> Self {
        self.max_gas = Some(max_gas);
        self
    }
}

impl ContractInstance {
    /// Creates a new contract instance under `address` with metadata read from `metadata_path`.
    pub fn new(address: AccountId, metadata_path: &str) -> Result<Self> {
        Ok(Self {
            address,
            transcoder: ContractMessageTranscoder::load(metadata_path)?,
        })
    }

    /// The address of this contract instance.
    pub fn address(&self) -> &AccountId {
        &self.address
    }

    /// Reads the value of a read-only, 0-argument call via RPC.
    pub async fn read0<T: TryFrom<ConvertibleValue, Error = anyhow::Error>, C: ConnectionApi>(
        &self,
        conn: &C,
        message: &str,
        params: ReadonlyCallParams,
    ) -> Result<T> {
        self.read::<String, T, C>(conn, message, &[], params).await
    }

    /// Reads the value of a read-only call via RPC.
    pub async fn read<
        S: AsRef<str> + Debug,
        T: TryFrom<ConvertibleValue, Error = anyhow::Error>,
        C: ConnectionApi,
    >(
        &self,
        conn: &C,
        message: &str,
        args: &[S],
        params: ReadonlyCallParams,
    ) -> Result<T> {
        let sender = params.sender.unwrap_or(self.address.clone());

        let result = self
            .dry_run_any(conn, message, args, sender, 0, None, params.at)
            .await?
            .result
            .map_err(|e| anyhow!("Contract exec failed {:?}", e))?;

        let decoded = self.decode(message, result.data)?;
        ConvertibleValue(decoded).try_into()?
    }

    /// Executes a 0-argument contract call sending with a given params.
    pub async fn exec0<C: SignedConnectionApi>(
        &self,
        conn: &C,
        message: &str,
        params: ExecCallParams,
    ) -> Result<TxInfo> {
        self.exec::<C, String>(conn, message, &[], params).await
    }

    /// Executes a contract with a given params.
    pub async fn exec<C: SignedConnectionApi, S: AsRef<str> + Debug>(
        &self,
        conn: &C,
        message: &str,
        args: &[S],
        params: ExecCallParams,
    ) -> Result<TxInfo> {
        let dry_run_result = self
            .exec_dry_run(
                conn,
                conn.account_id().clone(),
                message,
                args,
                params.clone(),
            )
            .await?;

        let data = self.encode(message, args)?;
        conn.call(
            self.address.clone(),
            params.value,
            params.max_gas.unwrap_or(Weight::new(
                dry_run_result.gas_required.ref_time(),
                dry_run_result.gas_required.proof_size(),
            )),
            None,
            data,
            TxStatus::Finalized,
        )
        .await
    }

    /// Dry-runs contract call with the given params. Useful to measure gas or to check if
    /// the call will likely fail or not.
    pub async fn exec_dry_run<C: ConnectionApi, S: AsRef<str> + Debug>(
        &self,
        conn: &C,
        sender: AccountId,
        message: &str,
        args: &[S],
        params: ExecCallParams,
    ) -> Result<ContractExecResult<Balance, EventRecord>> {
        self.dry_run_any(
            conn,
            message,
            args,
            sender,
            params.value,
            params.max_gas,
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn dry_run_any<S: AsRef<str> + Debug, C: ConnectionApi>(
        &self,
        conn: &C,
        message: &str,
        args: &[S],
        sender: AccountId,
        value: Balance,
        gas_limit: Option<Weight>,
        at: Option<BlockHash>,
    ) -> Result<ContractExecResult<Balance, EventRecord>> {
        let payload = self.encode(message, args)?;
        let args = ContractCallArgs {
            origin: sender,
            dest: self.address.clone(),
            value,
            gas_limit,
            input_data: payload,
            storage_deposit_limit: None,
        };

        let contract_read_result = conn
            .call_and_get(args, at)
            .await
            .context("RPC request error - there may be more info in node logs.")?;

        if !contract_read_result.debug_message.is_empty() {
            info!(
                target: "aleph_client::contract",
                "Dry-run debug messages: {:?}",
                core::str::from_utf8(&contract_read_result.debug_message)
                    .unwrap_or("<Invalid UTF8>")
                    .split('\n')
                    .filter(|m| !m.is_empty())
                    .collect::<Vec<_>>()
            );
        }

        // For dry run, failed transactions don't return `Err` but `Ok(_)`
        // and we have to inspect flags manually.
        if let Ok(res) = &contract_read_result.result {
            if res.did_revert() {
                return Err(anyhow!(
                    "Dry-run call reverted, decoded result: {:?}",
                    self.decode(message, res.data.clone())
                ));
            }
        }

        Ok(contract_read_result)
    }

    fn encode<S: AsRef<str> + Debug>(&self, message: &str, args: &[S]) -> Result<Vec<u8>> {
        self.transcoder.encode(message, args)
    }

    fn decode(&self, message: &str, data: Vec<u8>) -> Result<Value> {
        self.transcoder.decode_return(message, &mut data.as_slice())
    }
}

impl Debug for ContractInstance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContractInstance")
            .field("address", &self.address)
            .finish()
    }
}
