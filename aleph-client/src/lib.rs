#![warn(missing_docs)]
//! API for [aleph-node](https://github.com/Cardinal-Cryptography/aleph-node) chain.
//!
//! This crate provides a Rust application interface for submitting transactions to `aleph-node` chain.
//! Most of the [pallets](https://docs.substrate.io/reference/frame-pallets/) are common to any
//! [Substrate](https://github.com/paritytech/substrate) chain, but there are some unique to `aleph-node`,
//! e.g. [`pallets::elections::ElectionsApi`].

#![feature(auto_traits)]
#![feature(negative_impls)]

use std::str::FromStr;

use anyhow::anyhow;
pub use contract_transcode;
pub use subxt::ext::{
    codec, sp_core,
    sp_core::{
        crypto::{PublicError, Ss58Codec},
        Pair,
    },
    sp_runtime,
};
use subxt::{
    config::polkadot::PolkadotExtrinsicParams,
    ext::{
        sp_core::{ed25519, sr25519, H256},
        sp_runtime::{MultiAddress, MultiSignature},
    },
    Config, OnlineClient, PolkadotConfig,
};

use crate::api::runtime_types::aleph_runtime::RuntimeCall as Call;

#[cfg(not(feature = "liminal"))]
#[allow(clippy::all)]
#[doc(hidden)]
mod aleph_zero;

#[cfg(feature = "liminal")]
#[path = "./aleph_zero_liminal.rs"]
#[allow(clippy::all)]
#[doc(hidden)]
mod aleph_zero;

mod connections;
pub mod contract;
/// API for pallets.
pub mod pallets;
mod runtime_types;
/// Block / session / era API.
pub mod utility;
/// Waiting for some events API.
pub mod waiting;

pub use ::primitives::*;
pub use aleph_zero::api;
pub use runtime_types::*;

/// An alias for a pallet aleph keys.
pub type AlephKeyPair = ed25519::Pair;
/// An alias for a type of a key pair that signs chain transactions.
pub type RawKeyPair = sr25519::Pair;
/// An alias for an account id type.
pub type AccountId = subxt::ext::sp_core::crypto::AccountId32;
/// An alias for a hash type.
pub type CodeHash = H256;
/// An alias for a block hash type.
pub type BlockHash = H256;
/// An alias for a transaction hash type.
pub type TxHash = H256;
/// An alias for an RPC client type.
pub type SubxtClient = OnlineClient<AlephConfig>;

pub use connections::{
    AsConnection, AsSigned, Connection, ConnectionApi, RootConnection, SignedConnection,
    SignedConnectionApi, SudoCall, TxInfo,
};

/// An alias for a configuration of live chain, e.g. block index type, hash type.
pub enum AlephConfig {}

impl Config for AlephConfig {
    type Hash = <PolkadotConfig as Config>::Hash;
    type AccountId = AccountId;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = <PolkadotConfig as Config>::Hasher;
    type Header = <PolkadotConfig as Config>::Header;
    type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
}
type ParamsBuilder = subxt::config::polkadot::PolkadotExtrinsicParamsBuilder<AlephConfig>;
type PairSigner = subxt::tx::PairSigner<AlephConfig, RawKeyPair>;

/// Used for signing extrinsic payload
pub struct KeyPair {
    inner: PairSigner,
}

impl Clone for KeyPair {
    fn clone(&self) -> Self {
        KeyPair::new(self.inner.signer().clone())
    }
}

impl FromStr for KeyPair {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        let pair = sr25519::Pair::from_string(s, None)
            .map_err(|e| anyhow!("Can't create pair from seed value: {:?}", e))?;
        Ok(KeyPair::new(pair))
    }
}

impl KeyPair {
    /// Constructs a new KeyPair from RawKeyPair
    pub fn new(keypair: RawKeyPair) -> Self {
        KeyPair {
            inner: PairSigner::new(keypair),
        }
    }

    /// Returns a reference to the inner RawKeyPair
    pub fn signer(&self) -> &RawKeyPair {
        self.inner.signer()
    }

    /// Returns corresponding AccountId
    pub fn account_id(&self) -> &AccountId {
        self.inner.account_id()
    }
}

/// When submitting a transaction, wait for given status before proceeding.
#[derive(Copy, Clone)]
pub enum TxStatus {
    /// A tx must be included in some block.
    InBlock,
    /// A tx must be included in some finalized block.
    Finalized,
    /// A tx must be successfully submitted.
    Submitted,
}

/// Converts given seed phrase to a sr25519 [`KeyPair`] object.
/// * `seed` - a 12 or 24 word seed phrase
pub fn keypair_from_string(seed: &str) -> KeyPair {
    let pair = sr25519::Pair::from_string(seed, None).expect("Can't create pair from seed value");
    KeyPair::new(pair)
}

/// Converts given seed phrase to a sr25519 [`RawKeyPair`] object.
/// * `seed` - a 12 or 24 word seed phrase
pub fn raw_keypair_from_string(seed: &str) -> RawKeyPair {
    sr25519::Pair::from_string(seed, None).expect("Can't create pair from seed value")
}

/// Converts given seed phrase to a ed25519 [`AlephKeyPair`] object.
/// * `seed` - a 12 or 24 word seed phrase
pub fn aleph_keypair_from_string(seed: &str) -> AlephKeyPair {
    ed25519::Pair::from_string(seed, None).expect("Can't create pair from seed value")
}

/// Converts a key pair object to `AccountId`.
/// * `keypair` - a key-pair object, e.g. [`ed25519::Pair`] or [`sr25519::Pair`]
pub fn account_from_keypair<P>(keypair: &P) -> AccountId
where
    P: Pair,
    AccountId: From<<P as Pair>::Public>,
{
    AccountId::from(keypair.public())
}
