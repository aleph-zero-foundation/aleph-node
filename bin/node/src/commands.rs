use crate::chain_spec::{self, get_account_id_from_seed, AuthorityKeys, ChainParams};
use aleph_primitives::AuthorityId as AlephId;
use aleph_runtime::AccountId;
use log::info;
use sc_cli::{Error, KeystoreParams};
use sc_keystore::LocalKeystore;
use sc_service::config::{BasePath, KeystoreConfig};
use sp_application_crypto::key_types;
use sp_application_crypto::Ss58Codec;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::sr25519;
use sp_keystore::SyncCryptoStore;
use std::io::Write;
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

fn open_keystore(
    keystore_params: &KeystoreParams,
    chain_params: &ChainParams,
    authority: &str,
) -> impl SyncCryptoStore {
    let chain_id = chain_params.chain_id();
    let base_path: BasePath = chain_params.base_path().path().join(authority).into();

    info!(
        "Writing to keystore for authority {} and chain id {} under path {:?}",
        authority, chain_id, base_path
    );

    let config_dir = base_path.config_dir(chain_id);
    match keystore_params
        .keystore_config(&config_dir)
        .expect("keystore configuration should be available")
    {
        (_, KeystoreConfig::Path { path, password }) => {
            LocalKeystore::open(path, password).expect("Keystore open should succeed")
        }
        _ => unreachable!("keystore_config always returns path and password; qed"),
    }
}

fn authority_keys(keystore: &impl SyncCryptoStore, account_id: &AccountId) -> AuthorityKeys {
    let aura_key = aura_key(keystore);
    let aleph_key = aleph_key(keystore);
    let account_id = account_id.clone();

    AuthorityKeys {
        account_id,
        aura_key,
        aleph_key,
    }
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
        let genesis_authorities = self
            .chain_params
            .account_ids()
            .iter()
            .map(|account_id| {
                let authority = account_id.to_string();
                let keystore = open_keystore(&self.keystore_params, &self.chain_params, &authority);
                authority_keys(&keystore, account_id)
            })
            .collect();

        info!("Building chain spec");

        let chain_spec = match self.chain_params.chain_id() {
            chain_spec::DEVNET_ID => {
                chain_spec::development_config(self.chain_params.clone(), genesis_authorities)
            }
            chain_id => {
                chain_spec::config(self.chain_params.clone(), genesis_authorities, chain_id)
            }
        };

        let spec = chain_spec?;
        let json = sc_service::chain_ops::build_spec(&spec, self.raw)?;
        if std::io::stdout().write_all(json.as_bytes()).is_err() {
            let _ = std::io::stderr().write_all(b"Error writing to stdout\n");
        }

        Ok(())
    }
}

/// The `bootstrap-node` command is used to generate key pairs for a single authority
/// private keys are stored in a specified keystore, and the public keys are written to stdout.
#[derive(Debug, StructOpt)]
pub struct BootstrapNodeCmd {
    /// Pass the AccountId of a new node
    ///
    /// Expects a string with an AccountId
    /// If this argument is not passed a random Id will be generated using account-seed argument as seed
    #[structopt(long)]
    account_id: Option<String>,

    /// human-readable authority name used as a seed to generate the AccountId
    #[structopt(long, required_unless = "account-id")]
    pub account_seed: Option<String>,

    #[structopt(flatten)]
    pub keystore_params: KeystoreParams,

    #[structopt(flatten)]
    pub chain_params: ChainParams,
}

impl BootstrapNodeCmd {
    pub fn run(&self) -> Result<(), Error> {
        let account_id = &self.account_id();
        let authority = account_id.to_string();
        let keystore = open_keystore(&self.keystore_params, &self.chain_params, &authority);

        let authority_keys = authority_keys(&keystore, account_id);
        let keys_json = serde_json::to_string_pretty(&authority_keys)
            .expect("serialization of authority keys should have succeed");
        println!("{}", keys_json);
        Ok(())
    }

    pub fn account_id(&self) -> AccountId {
        match &self.account_id {
            Some(id) => AccountId::from_string(id.as_str())
                .expect("Passed string is not a hex encoding of a public key"),
            None => get_account_id_from_seed::<sr25519::Public>(
                &self
                    .account_seed
                    .clone()
                    .expect("Pass account-id or node-name argument")
                    .as_str(),
            ),
        }
    }
}
