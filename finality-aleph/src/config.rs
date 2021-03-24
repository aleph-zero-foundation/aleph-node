use std::time::Duration;

/// Configuration for the Aleph protocol service.
pub struct Config {
    /// The duration of a message to be sent across the network.
    pub gossip_duration: Duration,
    /// If the node is running as an authority.
    pub is_authority: bool,
    /// The name of this particular node.
    pub name: Option<String>,
    /// The keystore which stores the keys.
    pub keystore: Option<sp_keystore::SyncCryptoStorePtr>,
}

impl Config {
    pub(crate) fn _name(&self) -> &str {
        self.name.as_deref().unwrap_or("<unknown>")
    }
}
