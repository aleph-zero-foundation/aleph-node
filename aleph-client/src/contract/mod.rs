//! Contains types and functions simplifying common contract-related operations.
//!
//! For example, you could write this wrapper around (some of) the functionality of openbrush PSP22
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
//!         self.contract.contract_exec(
//!             conn,
//!             "PSP22::transfer",
//!             vec![to.to_string().as_str(), amount.to_string().as_str(), "0x00"].as_slice(),
//!         ).await
//!     }
//!
//!     async fn balance_of(&self, conn: &Connection, account: AccountId) -> Result<Balance> {
//!         self.contract.contract_read(
//!             conn,
//!             "PSP22::balance_of",
//!             &vec![account.to_string().as_str()],
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
use log::{error, info};
use pallet_contracts_primitives::ContractExecResult;

use crate::{
    connections::TxInfo,
    contract_transcode::Value,
    pallets::contract::{ContractCallArgs, ContractRpc, ContractsUserApi},
    sp_weights::weight_v2::Weight,
    AccountId, Balance, ConnectionApi, SignedConnectionApi, TxStatus,
};

/// Default gas limit, which allows up to 25% of block time (62.5% of the actual block capacity).
pub const DEFAULT_MAX_GAS: u64 = 250_000_000_000u64;
/// Default proof size limit, which allows up to 25% of block time (62.5% of the actual block
/// capacity).
pub const DEFAULT_MAX_PROOF_SIZE: u64 = 250_000_000_000u64;

/// Represents a contract instantiated on the chain.
pub struct ContractInstance {
    address: AccountId,
    transcoder: ContractMessageTranscoder,
    max_gas_override: Option<u64>,
    max_proof_size_override: Option<u64>,
}

impl ContractInstance {
    /// Creates a new contract instance under `address` with metadata read from `metadata_path`.
    pub fn new(address: AccountId, metadata_path: &str) -> Result<Self> {
        Ok(Self {
            address,
            transcoder: ContractMessageTranscoder::load(metadata_path)?,
            max_gas_override: None,
            max_proof_size_override: None,
        })
    }

    /// From now on, the contract instance will use `limit_override` as the gas limit for all
    /// contract calls. If `limit_override` is `None`, then [DEFAULT_MAX_GAS] will be used.
    pub fn override_gas_limit(&mut self, limit_override: Option<u64>) {
        self.max_gas_override = limit_override;
    }

    /// From now on, the contract instance will use `limit_override` as the proof size limit for all
    /// contract calls. If `limit_override` is `None`, then [DEFAULT_MAX_PROOF_SIZE] will be used.
    pub fn override_proof_size_limit(&mut self, limit_override: Option<u64>) {
        self.max_proof_size_override = limit_override;
    }

    /// The address of this contract instance.
    pub fn address(&self) -> &AccountId {
        &self.address
    }

    /// Reads the value of a read-only, 0-argument call via RPC.
    pub async fn contract_read0<
        T: TryFrom<ConvertibleValue, Error = anyhow::Error>,
        C: ConnectionApi,
    >(
        &self,
        conn: &C,
        message: &str,
    ) -> Result<T> {
        self.contract_read::<String, T, C>(conn, message, &[]).await
    }

    /// Reads the value of a read-only call via RPC.
    pub async fn contract_read<
        S: AsRef<str> + Debug,
        T: TryFrom<ConvertibleValue, Error = anyhow::Error>,
        C: ConnectionApi,
    >(
        &self,
        conn: &C,
        message: &str,
        args: &[S],
    ) -> Result<T> {
        self.contract_read_as(conn, message, args, self.address.clone())
            .await
    }

    /// Reads the value of a contract call via RPC as if it was executed by `sender`.
    pub async fn contract_read_as<
        S: AsRef<str> + Debug,
        T: TryFrom<ConvertibleValue, Error = anyhow::Error>,
        C: ConnectionApi,
    >(
        &self,
        conn: &C,
        message: &str,
        args: &[S],
        sender: AccountId,
    ) -> Result<T> {
        let result = self
            .dry_run(conn, message, args, sender)
            .await?
            .result
            .map_err(|e| anyhow!("Contract exec failed {:?}", e))?;

        let decoded = self.decode(message, result.data)?;
        ConvertibleValue(decoded).try_into()?
    }

    /// Executes a 0-argument contract call.
    pub async fn contract_exec0<C: SignedConnectionApi>(
        &self,
        conn: &C,
        message: &str,
    ) -> Result<TxInfo> {
        self.contract_exec::<C, String>(conn, message, &[]).await
    }

    /// Executes a contract call.
    pub async fn contract_exec<C: SignedConnectionApi, S: AsRef<str> + Debug>(
        &self,
        conn: &C,
        message: &str,
        args: &[S],
    ) -> Result<TxInfo> {
        self.contract_exec_value::<C, S>(conn, message, args, 0)
            .await
    }

    /// Executes a 0-argument contract call sending the given amount of value with it.
    pub async fn contract_exec_value0<C: SignedConnectionApi>(
        &self,
        conn: &C,
        message: &str,
        value: Balance,
    ) -> Result<TxInfo> {
        self.contract_exec_value::<C, String>(conn, message, &[], value)
            .await
    }

    /// Executes a contract call sending the given amount of value with it.
    pub async fn contract_exec_value<C: SignedConnectionApi, S: AsRef<str> + Debug>(
        &self,
        conn: &C,
        message: &str,
        args: &[S],
        value: Balance,
    ) -> Result<TxInfo> {
        let dry_run_result = self
            .dry_run(conn, message, args, conn.account_id().clone())
            .await?;

        let data = self.encode(message, args)?;
        conn.call(
            self.address.clone(),
            value,
            Weight {
                ref_time: dry_run_result.gas_required.ref_time(),
                proof_size: dry_run_result.gas_required.proof_size(),
            },
            None,
            data,
            TxStatus::Finalized,
        )
        .await
    }

    fn encode<S: AsRef<str> + Debug>(&self, message: &str, args: &[S]) -> Result<Vec<u8>> {
        self.transcoder.encode(message, args)
    }

    fn decode(&self, message: &str, data: Vec<u8>) -> Result<Value> {
        self.transcoder.decode_return(message, &mut data.as_slice())
    }

    async fn dry_run<S: AsRef<str> + Debug, C: ConnectionApi>(
        &self,
        conn: &C,
        message: &str,
        args: &[S],
        sender: AccountId,
    ) -> Result<ContractExecResult<Balance>> {
        let payload = self.encode(message, args)?;
        let args = ContractCallArgs {
            origin: sender,
            dest: self.address.clone(),
            value: 0,
            gas_limit: None,
            input_data: payload,
            storage_deposit_limit: None,
        };

        let contract_read_result = conn
            .call_and_get(args)
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

        if let Ok(res) = &contract_read_result.result {
            if res.did_revert() {
                // For dry run, failed transactions don't return `Err` but `Ok(_)`
                // and we have to inspect flags manually.
                error!("Dry-run call reverted");
            }
        }

        Ok(contract_read_result)
    }
}

impl Debug for ContractInstance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContractInstance")
            .field("address", &self.address)
            .finish()
    }
}
