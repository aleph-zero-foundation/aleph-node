use sc_cli::{Error, KeystoreParams, RunCmd, SharedParams, SubstrateCli};
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
    pub fn run<C: SubstrateCli>(&self, cli: &C) -> Result<(), Error> {
        let key_types: Vec<_> = self
            .key_types
            .iter()
            .map(|kt| KeyTypeId::try_from(kt.as_str()).expect("wrong key type"))
            .collect();
        // A hashmap from a key type to a 32-byte representation of a sr25519 or ed25519 key.
        let mut auth_keys: HashMap<_, _> = key_types
            .iter()
            .zip(vec![vec![]].into_iter().cycle())
            .collect();
        for authority in &crate::chain_spec::LOCAL_AUTHORITIES {
            let keystore = self.open_keystore(authority, cli)?;
            for &key_type in &key_types {
                use sp_core::crypto::key_types;
                match key_type {
                    key_types::AURA => {
                        let keys = SyncCryptoStore::sr25519_public_keys(&*keystore, key_type);
                        let key = keys.into_iter().next().map_or_else(
                            || {
                                SyncCryptoStore::sr25519_generate_new(&*keystore, key_type, None)
                                    .map_err(|_| Error::KeyStoreOperation)
                            },
                            Ok,
                        )?;
                        auth_keys
                            .get_mut(&key_type)
                            .unwrap()
                            .push(*key.as_array_ref());
                    }
                    aleph_primitives::KEY_TYPE => {
                        let keys = SyncCryptoStore::ed25519_public_keys(&*keystore, key_type);
                        let key = keys.into_iter().next().map_or_else(
                            || {
                                SyncCryptoStore::ed25519_generate_new(&*keystore, key_type, None)
                                    .map_err(|_| Error::KeyStoreOperation)
                            },
                            Ok,
                        )?;
                        auth_keys
                            .get_mut(&key_type)
                            .unwrap()
                            .push(*key.as_array_ref());
                    }
                    _ => return Err(Error::Input("Unsupported key type".into())),
                }
            }
        }

        let keys_path = crate::chain_spec::KEY_PATH;
        let auth_keys: HashMap<_, _> = auth_keys.iter().map(|(k, v)| (u32::from(**k), v)).collect();
        let auth_keys = serde_json::to_string(&auth_keys).map_err(|e| Error::Io(e.into()))?;
        std::fs::write(keys_path, &auth_keys).map_err(Error::Io)?;

        Ok(())
    }

    fn open_keystore<C: SubstrateCli>(
        &self,
        authority: &str,
        cli: &C,
    ) -> Result<SyncCryptoStorePtr, Error> {
        let base_path: BasePath = self
            .shared_params
            .base_path()
            .unwrap()
            .path()
            .join(authority)
            .into();
        let chain_id = self.shared_params.chain_id(self.shared_params.is_dev());
        let chain_spec = cli.load_spec(&chain_id)?;
        let config_dir = base_path.config_dir(chain_spec.id());

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
