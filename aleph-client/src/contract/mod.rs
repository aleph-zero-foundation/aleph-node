//! Contains types and functions simplifying common contract-related operations.
//!
//! For example, you could write this wrapper around (some of) the functionality of openbrush PSP22
//! contracts using the building blocks provided by this module:
//!
//! ```no_run
//! # use anyhow::{Result, Context};
//! # use sp_core::crypto::AccountId32;
//! # use aleph_client::{AccountId, Connection, SignedConnection};
//! # use aleph_client::contract::ContractInstance;
//! # use aleph_client::contract::util::to_u128;
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
//!     fn transfer(&self, conn: &SignedConnection, to: AccountId, amount: u128) -> Result<()> {
//!         self.contract.contract_exec(
//!             conn,
//!             "PSP22::transfer",
//!             vec![to.to_string().as_str(), amount.to_string().as_str(), "0x00"].as_slice(),
//!         )
//!     }
//!
//!     fn balance_of(&self, conn: &Connection, account: AccountId) -> Result<u128> {
//!         to_u128(self.contract.contract_read(
//!             conn,
//!             "PSP22::balance_of",
//!             &vec![account.to_string().as_str()],
//!         )?)
//!     }
//! }
//! ```

pub mod util;

use std::{
    fmt::{Debug, Formatter},
    fs::File,
};

use anyhow::{anyhow, Context, Result};
use codec::{Compact, Decode};
use contract_metadata::ContractMetadata;
use contract_transcode::ContractMessageTranscoder;
use ink_metadata::{InkProject, MetadataVersioned};
use serde_json::{from_reader, from_value};

use crate::{
    frame_support::weights::weight_v2::Weight,
    pallets::contract::{ContractCallArgs, ContractRpc, ContractsUserApi},
    AccountId, Connection, SignedConnection, TxStatus,
};

/// Represents a contract instantiated on the chain.
pub struct ContractInstance {
    address: AccountId,
    ink_project: InkProject,
}

impl ContractInstance {
    const MAX_READ_GAS: u64 = 500000000000u64;
    const MAX_GAS: u64 = 10000000000u64;
    const PAYABLE_VALUE: u64 = 0u64;
    const STORAGE_FEE_LIMIT: Option<Compact<u128>> = None;

    /// Creates a new contract instance under `address` with metadata read from `metadata_path`.
    pub fn new(address: AccountId, metadata_path: &str) -> Result<Self> {
        Ok(Self {
            address,
            ink_project: load_metadata(metadata_path)?,
        })
    }

    /// The address of this contract instance.
    pub fn address(&self) -> &AccountId {
        &self.address
    }

    /// The metadata of this contract instance.
    pub fn ink_project(&self) -> &InkProject {
        &self.ink_project
    }

    /// Reads the value of a read-only, 0-argument call via RPC.
    pub async fn contract_read0<T: Decode>(&self, conn: &Connection, message: &str) -> Result<T> {
        self.contract_read(conn, message, &[]).await
    }

    /// Reads the value of a read-only call via RPC.
    pub async fn contract_read<T: Decode>(
        &self,
        conn: &Connection,
        message: &str,
        args: &[&str],
    ) -> Result<T> {
        let payload = self.encode(message, args)?;
        let args = ContractCallArgs {
            origin: self.address.clone(),
            dest: self.address.clone(),
            value: 0,
            gas_limit: Weight {
                ref_time: Self::MAX_READ_GAS,
            },
            input_data: payload,
            storage_deposit_limit: None,
        };
        conn.call_and_get(args)
            .await
            .context("RPC request error - there may be more info in node logs.")
    }

    /// Executes a 0-argument contract call.
    pub async fn contract_exec0(&self, conn: &SignedConnection, message: &str) -> Result<()> {
        self.contract_exec(conn, message, &[]).await
    }

    /// Executes a contract call.
    pub async fn contract_exec(
        &self,
        conn: &SignedConnection,
        message: &str,
        args: &[&str],
    ) -> Result<()> {
        let data = self.encode(message, args)?;
        conn.call(
            self.address.clone(),
            Self::PAYABLE_VALUE as u128,
            Weight {
                ref_time: Self::MAX_GAS,
            },
            Self::STORAGE_FEE_LIMIT,
            data,
            TxStatus::InBlock,
        )
        .await
        .context("Failed to exec contract message")?;

        Ok(())
    }

    fn encode(&self, message: &str, args: &[&str]) -> Result<Vec<u8>> {
        ContractMessageTranscoder::new(&self.ink_project).encode(message, args)
    }
}

impl Debug for ContractInstance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContractInstance")
            .field("address", &self.address)
            .field("ink_project", &self.ink_project)
            .finish()
    }
}

/// Helper for loading contract metadata from a file.
///
/// The contract-metadata lib contains a similar function starting with version 0.2. It seems that
/// version conflicts with some of our other dependencies, however, if we upgrade in the future we
/// can drop this function in favour of their implementation.
fn load_metadata(path: &str) -> Result<InkProject> {
    let file = File::open(path)?;
    let metadata: ContractMetadata = from_reader(file)?;
    let ink_metadata = from_value(serde_json::Value::Object(metadata.abi))?;

    if let MetadataVersioned::V3(ink_project) = ink_metadata {
        Ok(ink_project)
    } else {
        Err(anyhow!("Unsupported ink metadata version. Expected V3"))
    }
}
