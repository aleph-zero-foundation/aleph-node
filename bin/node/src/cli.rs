use sc_cli::{Error, KeystoreParams, RunCmd, SharedParams};
use sc_service::config::{BasePath, KeystoreConfig};
use std::{collections::HashMap, convert::TryFrom, sync::Arc};

use sc_keystore::LocalKeystore;
use sp_core::crypto::KeyTypeId;
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[structopt(flatten)]
    pub run: RunCmd,
}

#[derive(Debug, StructOpt)]
pub struct CLiDevKeys {
    /// List of genesis authorities
    #[structopt(long)]
    pub authorities: Vec<String>,

    /// Key types, examples: "aura", or "alp0"
    #[structopt(long)]
    pub key_types: Vec<String>,

    #[structopt(flatten)]
    pub keystore_params: KeystoreParams,

    #[structopt(flatten)]
    pub shared_params: SharedParams,
}

impl CLiDevKeys {
    pub fn run(&self) -> Result<(), Error> {
        let key_types: Vec<_> = self
            .key_types
            .iter()
            .map(|kt| KeyTypeId::try_from(kt.as_str()).expect("wrong key type"))
            .collect();
        let mut auth_keys: HashMap<_, _> = key_types
            .iter()
            .zip(vec![vec![]].into_iter().cycle())
            .collect();
        for authority in &crate::chain_spec::LOCAL_AUTHORITIES {
            let keystore = self.open_keystore(authority)?;
            for &key_type in &key_types {
                // TODO match key_type to crypto_type
                let keys = SyncCryptoStore::sr25519_public_keys(&*keystore, key_type);
                let key = if keys.is_empty() {
                    SyncCryptoStore::sr25519_generate_new(&*keystore, key_type, None)
                        .map_err(|_| Error::KeyStoreOperation)?
                } else {
                    keys[0]
                };
                auth_keys.get_mut(&key_type).unwrap().push((key,));
            }
        }

        let keys_path = crate::chain_spec::KEY_PATH;
        let auth_keys: HashMap<_, _> = auth_keys.iter().map(|(k, v)| (u32::from(**k), v)).collect();
        let auth_keys = serde_json::to_string(&auth_keys).map_err(|e| Error::Io(e.into()))?;
        std::fs::write(keys_path, &auth_keys).map_err(|e| Error::Io(e))?;

        Ok(())
    }

    fn open_keystore(&self, authority: &str) -> Result<SyncCryptoStorePtr, Error> {
        let base_path: BasePath = self
            .shared_params
            .base_path()
            .unwrap()
            .path()
            .join(authority)
            .into();
        let chain_id = self.shared_params.chain_id(self.shared_params.is_dev());
        let config_dir = base_path.config_dir(&chain_id);

        match self.keystore_params.keystore_config(&config_dir)? {
            (_, KeystoreConfig::Path { path, password }) => {
                Ok(Arc::new(LocalKeystore::open(path, password)?))
            }
            _ => unreachable!("keystore_config always returns path and password; qed"),
        }
    }
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// Key management cli utilities
    Key(sc_cli::KeySubcommand),
    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),

    /// Generate keys for local tests
    DevKeys(CLiDevKeys),
}
