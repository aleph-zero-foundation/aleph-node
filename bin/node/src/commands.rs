use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use libp2p::identity::{ed25519 as libp2p_ed25519, PublicKey};
use primitives::{AccountId, AuraId, AuthorityId as AlephId};
use sc_cli::{
    clap::{self, Parser},
    Error, KeystoreParams,
};
use sc_keystore::LocalKeystore;
use sc_service::config::{BasePath, KeystoreConfig};
use serde::{Deserialize, Serialize};
use sp_application_crypto::{key_types, Ss58Codec};
use sp_keystore::Keystore;

use crate::{
    chain_spec::{account_id_from_string, AlephNodeChainSpec, ChainParams, SerializablePeerId},
    shared_params::SharedParams,
};

/// returns Aura key, if absent a new key is generated
fn aura_key(keystore: &impl Keystore) -> AuraId {
    Keystore::sr25519_public_keys(keystore, key_types::AURA)
        .pop()
        .unwrap_or_else(|| {
            Keystore::sr25519_generate_new(keystore, key_types::AURA, None)
                .expect("Could not create Aura key")
        })
        .into()
}

/// returns Aleph key, if absent a new key is generated
fn aleph_key(keystore: &impl Keystore) -> AlephId {
    Keystore::ed25519_public_keys(keystore, primitives::KEY_TYPE)
        .pop()
        .unwrap_or_else(|| {
            Keystore::ed25519_generate_new(keystore, primitives::KEY_TYPE, None)
                .expect("Could not create Aleph key")
        })
        .into()
}

/// Returns peer id, if not p2p key found under base_path/node-key-file a new private key gets generated
fn p2p_key(node_key_path: &Path) -> SerializablePeerId {
    if node_key_path.exists() {
        let mut file_content =
            hex::decode(fs::read(node_key_path).unwrap()).expect("Failed to decode secret as hex");
        let secret = libp2p_ed25519::SecretKey::try_from_bytes(&mut file_content)
            .expect("Bad node key file");
        let keypair = libp2p_ed25519::Keypair::from(secret);
        SerializablePeerId::new(PublicKey::from(keypair.public()).to_peer_id())
    } else {
        let keypair = libp2p_ed25519::Keypair::generate();
        let secret = keypair.secret();
        let secret_hex = hex::encode(secret.as_ref());
        fs::write(node_key_path, secret_hex).expect("Could not write p2p secret");
        SerializablePeerId::new(PublicKey::from(keypair.public()).to_peer_id())
    }
}

fn backup_path(base_path: &Path, backup_dir: &str) -> PathBuf {
    base_path.join(backup_dir)
}

pub fn open_keystore(
    keystore_params: &KeystoreParams,
    chain_id: &str,
    base_path: &BasePath,
) -> impl Keystore {
    let config_dir = base_path.config_dir(chain_id);
    match keystore_params
        .keystore_config(&config_dir)
        .expect("keystore configuration should be available")
    {
        KeystoreConfig::Path { path, password } => {
            LocalKeystore::open(path, password).expect("Keystore open should succeed")
        }
        _ => unreachable!("keystore_config always returns path and password; qed"),
    }
}

pub fn bootstrap_backup(base_path_with_account_id: &Path, backup_dir: &str) {
    let backup_path = backup_path(base_path_with_account_id, backup_dir);

    if backup_path.exists() {
        if !backup_path.is_dir() {
            panic!("Could not create backup directory at {backup_path:?}. Path is already a file.");
        }
    } else {
        fs::create_dir_all(backup_path).expect("Could not create backup directory.");
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AuthorityKeys {
    pub account_id: AccountId,
    pub aura_key: AuraId,
    pub aleph_key: AlephId,
    pub peer_id: SerializablePeerId,
}

pub fn authority_keys(
    keystore: &impl Keystore,
    base_path: &Path,
    node_key_file: &str,
    account_id: AccountId,
) -> AuthorityKeys {
    let aura_key = aura_key(keystore);
    let aleph_key = aleph_key(keystore);
    let node_key_path = base_path.join(node_key_file);
    let peer_id = p2p_key(node_key_path.as_path());

    AuthorityKeys {
        account_id,
        aura_key,
        aleph_key,
        peer_id,
    }
}

/// The `bootstrap-node` command is used to generate key pairs and AlephBFT backup folder for a single authority
/// private keys are stored in a specified keystore, and the public keys are written to stdout.
#[derive(Debug, Parser)]
pub struct BootstrapNodeCmd {
    /// Pass the account id of a new node
    ///
    /// Expects a string with an account id (hex encoding of an sr2559 public key)
    /// If this argument is not passed a random account id will be generated using account-seed argument as a seed
    #[arg(long)]
    account_id: Option<String>,

    /// Pass seed used to generate the account private key (sr2559) and the corresponding AccountId
    #[arg(long, required_unless_present = "account_id")]
    pub account_seed: Option<String>,

    #[clap(flatten)]
    pub keystore_params: KeystoreParams,

    #[clap(flatten)]
    pub chain_params: ChainParams,

    #[clap(flatten)]
    pub shared_params: SharedParams,
}

/// Assumes an input path: some_path/account_id/, which is not appended with an account id
impl BootstrapNodeCmd {
    pub fn run(&self) -> Result<(), Error> {
        let base_path = self.shared_params.base_path();
        let backup_dir = self.shared_params.backup_dir();
        let node_key_file = self.shared_params.node_key_file();
        let chain_id = self.chain_params.chain_id();

        bootstrap_backup(base_path.path(), backup_dir);

        let keystore = open_keystore(&self.keystore_params, chain_id, &base_path);

        // Does not rely on the account id in the path
        let account_id = self.account_id();
        let authority_keys = authority_keys(&keystore, base_path.path(), node_key_file, account_id);
        let keys_json = serde_json::to_string_pretty(&authority_keys)
            .expect("serialization of authority keys should have succeeded");
        println!("{keys_json}");
        Ok(())
    }

    fn account_id(&self) -> AccountId {
        match &self.account_id {
            Some(id) => AccountId::from_string(id.as_str())
                .expect("Passed string is not a hex encoding of a public key"),
            None => account_id_from_string(
                self.account_seed
                    .clone()
                    .expect("Pass account-seed argument")
                    .as_str(),
            ),
        }
    }
}

/// Command used to go from chainspec to the raw chainspec format
#[derive(Debug, Parser)]
pub struct ConvertChainspecToRawCmd {
    /// Specify path to JSON chainspec
    #[arg(long)]
    pub chain: PathBuf,
}

impl ConvertChainspecToRawCmd {
    pub fn run(&self) -> Result<(), Error> {
        let spec = AlephNodeChainSpec::from_json_file(self.chain.to_owned())
            .expect("Cannot read chainspec");

        let raw_chainspec = sc_service::chain_ops::build_spec(&spec, true)?;
        if std::io::stdout()
            .write_all(raw_chainspec.as_bytes())
            .is_err()
        {
            let _ = std::io::stderr().write_all(b"Error writing to stdout\n");
        }

        Ok(())
    }
}
