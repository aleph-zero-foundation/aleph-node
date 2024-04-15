use std::str::FromStr;

use libp2p::PeerId;
use primitives::AccountId;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use sp_application_crypto::Pair;
use sp_core::sr25519;

mod builder;
mod cli;

pub use cli::ChainParams;

pub const CHAINTYPE_DEV: &str = "dev";
pub const CHAINTYPE_LOCAL: &str = "local";
pub const CHAINTYPE_LIVE: &str = "live";

pub const DEFAULT_CHAIN_ID: &str = "a0dnet1";

// Alice is the default sudo holder.
pub const DEFAULT_SUDO_ACCOUNT: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

pub type AlephNodeChainSpec = sc_service::GenericChainSpec<()>;

pub use cli::BootstrapChainCmd;

pub fn mainnet_config() -> Result<AlephNodeChainSpec, String> {
    AlephNodeChainSpec::from_json_bytes(crate::resources::mainnet_chainspec())
}

pub fn testnet_config() -> Result<AlephNodeChainSpec, String> {
    AlephNodeChainSpec::from_json_bytes(crate::resources::testnet_chainspec())
}

/// Generate an account ID from seed.
pub fn account_id_from_string(seed: &str) -> AccountId {
    AccountId::from(
        sr25519::Pair::from_string(seed, None)
            .expect("Can't create pair from seed value")
            .public(),
    )
}

#[derive(Clone)]
pub struct SerializablePeerId {
    inner: PeerId,
}

impl SerializablePeerId {
    pub fn new(inner: PeerId) -> SerializablePeerId {
        SerializablePeerId { inner }
    }
}

impl Serialize for SerializablePeerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s: String = format!("{}", self.inner);
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for SerializablePeerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let inner = PeerId::from_str(&s)
            .map_err(|_| D::Error::custom(format!("Could not deserialize as PeerId: {s}")))?;
        Ok(SerializablePeerId { inner })
    }
}
