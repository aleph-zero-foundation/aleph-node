use crate::chain_spec::{
    self, get_account_id_from_seed, AuthorityKeys, ChainParams, SerializablePeerId,
};
use aleph_primitives::AuthorityId as AlephId;
use aleph_runtime::AccountId;
use libp2p::identity::{ed25519 as libp2p_ed25519, PublicKey};
use sc_cli::{Error, KeystoreParams};
use sc_keystore::LocalKeystore;
use sc_service::config::{BasePath, KeystoreConfig};
use sp_application_crypto::key_types;
use sp_application_crypto::Ss58Codec;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::sr25519;
use sp_keystore::SyncCryptoStore;
use std::fs;
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

/// Returns peer id, if not p2p key found under base_path/account-id/node-key-file a new provate key gets generated
fn p2p_key(chain_params: &ChainParams, account_id: &AccountId) -> SerializablePeerId {
    let authority = account_id.to_string();
    let file = chain_params
        .base_path()
        .path()
        .join(authority)
        .join(&chain_params.node_key_file);

    if file.exists() {
        let mut file_content =
            hex::decode(fs::read(&file).unwrap()).expect("failed to decode secret as hex");
        let secret =
            libp2p_ed25519::SecretKey::from_bytes(&mut file_content).expect("Bad node key file");
        let keypair = libp2p_ed25519::Keypair::from(secret);
        SerializablePeerId::new(PublicKey::Ed25519(keypair.public()).into_peer_id())
    } else {
        let keypair = libp2p_ed25519::Keypair::generate();
        let secret = keypair.secret();
        let secret_hex = hex::encode(secret.as_ref());
        fs::write(file, secret_hex).expect("Could not write p2p secret");
        SerializablePeerId::new(PublicKey::Ed25519(keypair.public()).into_peer_id())
    }
}

fn open_keystore(
    keystore_params: &KeystoreParams,
    chain_params: &ChainParams,
    account_id: &AccountId,
) -> impl SyncCryptoStore {
    let chain_id = chain_params.chain_id();
    let base_path: BasePath = chain_params
        .base_path()
        .path()
        .join(account_id.to_string())
        .into();

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

fn authority_keys(
    keystore: &impl SyncCryptoStore,
    chain_params: &ChainParams,
    account_id: &AccountId,
) -> AuthorityKeys {
    let aura_key = aura_key(keystore);
    let aleph_key = aleph_key(keystore);
    let peer_id = p2p_key(chain_params, account_id);

    let account_id = account_id.clone();
    AuthorityKeys {
        account_id,
        aura_key,
        aleph_key,
        peer_id,
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
                let keystore = open_keystore(&self.keystore_params, &self.chain_params, account_id);
                authority_keys(&keystore, &self.chain_params, account_id)
            })
            .collect();

        let chain_spec = chain_spec::config(
            self.chain_params.clone(),
            genesis_authorities,
            self.chain_params.chain_id(),
        )?;

        let json = sc_service::chain_ops::build_spec(&chain_spec, self.raw)?;
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
    /// Expects a string with an AccountId (hex encoding of an sr2559 public key)
    /// If this argument is not passed a random AccountId will be generated using account-seed argument as a seed
    #[structopt(long)]
    account_id: Option<String>,

    /// Pass seed used to generate the account pivate key (sr2559) and the corresponding AccountId
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
        let keystore = open_keystore(&self.keystore_params, &self.chain_params, account_id);

        let authority_keys = authority_keys(&keystore, &self.chain_params, account_id);
        let keys_json = serde_json::to_string_pretty(&authority_keys)
            .expect("serialization of authority keys should have succeeded");
        println!("{}", keys_json);
        Ok(())
    }

    fn account_id(&self) -> AccountId {
        match &self.account_id {
            Some(id) => AccountId::from_string(id.as_str())
                .expect("Passed string is not a hex encoding of a public key"),
            None => get_account_id_from_seed::<sr25519::Public>(
                &self
                    .account_seed
                    .clone()
                    .expect("Pass account-seed argument")
                    .as_str(),
            ),
        }
    }
}
