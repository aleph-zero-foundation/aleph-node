use crate::chain_spec::{self, AuthorityKeys, ChainParams};
use aleph_primitives::AuthorityId as AlephId;
use log::info;
use sc_cli::{Error, KeystoreParams};
use sc_keystore::LocalKeystore;
use sc_service::config::{BasePath, KeystoreConfig};
use sp_application_crypto::key_types;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_keystore::SyncCryptoStore;
use std::io::Write;
use std::ops::Deref;
use std::sync::Arc;
use structopt::StructOpt;

/// returns Aura key, if absent a new key is generated
fn aura_key(keystore: &impl SyncCryptoStore) -> AuraId {
    SyncCryptoStore::sr25519_public_keys(&*keystore, key_types::AURA)
        .pop()
        .unwrap_or_else(|| {
            SyncCryptoStore::sr25519_generate_new(&*keystore, key_types::AURA, None)
                .expect("Could not create Aura key")
        })
        .into()
}

/// returns Aleph key, if absent a new key is generated
fn aleph_key(keystore: &impl SyncCryptoStore) -> AlephId {
    SyncCryptoStore::ed25519_public_keys(&*keystore, aleph_primitives::KEY_TYPE)
        .pop()
        .unwrap_or_else(|| {
            SyncCryptoStore::ed25519_generate_new(&*keystore, aleph_primitives::KEY_TYPE, None)
                .expect("Could not create Aleph key")
        })
        .into()
}

/// The `bootstrap-chain` command is used to generate private keys for the genesis authorities
/// keys are written to the keystore of the authorities
/// and the chain specification is printed to stdout in the JSON format
#[derive(Debug, StructOpt)]
pub struct BootstrapChainCmd {
    /// Force raw genesis storage output.
    #[structopt(long = "raw")]
    pub raw: bool,

    #[structopt(flatten)]
    pub keystore_params: KeystoreParams,

    #[structopt(flatten)]
    pub chain_params: ChainParams,
}

impl BootstrapChainCmd {
    pub fn run(&self) -> Result<(), Error> {
        let chain_id = self.chain_params.chain_id();

        let genesis_authorities = self
            .chain_params
            .account_ids()
            .iter()
            .map(|account_id| {
                let authority = account_id.to_string();

                let authority_keystore = self
                    .open_keystore(&authority, chain_id)
                    .unwrap_or_else(|_| panic!("Cannot open keystore for {}", authority));

                let aura_key = aura_key(Deref::deref(&authority_keystore));
                let aleph_key = aleph_key(Deref::deref(&authority_keystore));
                let account_id = account_id.to_owned();

                AuthorityKeys {
                    account_id,
                    aura_key,
                    aleph_key,
                }
            })
            .collect();

        info!("Building chain spec");

        let chain_spec = match self.chain_params.chain_id() {
            chain_spec::DEVNET_ID => {
                chain_spec::development_config(self.chain_params.clone(), genesis_authorities)
            }
            _ => chain_spec::config(self.chain_params.clone(), genesis_authorities, chain_id),
        };

        let spec = chain_spec?;
        let json = sc_service::chain_ops::build_spec(&spec, self.raw)?;
        if std::io::stdout().write_all(json.as_bytes()).is_err() {
            let _ = std::io::stderr().write_all(b"Error writing to stdout\n");
        }

        Ok(())
    }

    fn open_keystore(&self, authority: &str, chain_id: &str) -> Result<Arc<LocalKeystore>, Error> {
        let base_path: BasePath = self.chain_params.base_path().path().join(authority).into();

        info!(
            "Writing to keystore for authority {} and chain id {} under path {:?}",
            authority, chain_id, base_path
        );

        let config_dir = base_path.config_dir(chain_id);
        match self.keystore_params.keystore_config(&config_dir)? {
            (_, KeystoreConfig::Path { path, password }) => {
                Ok(Arc::new(LocalKeystore::open(path, password)?))
            }
            _ => unreachable!("keystore_config always returns path and password; qed"),
        }
    }
}
