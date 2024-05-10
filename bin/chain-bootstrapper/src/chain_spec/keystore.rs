use std::{
    fs,
    path::{Path, PathBuf},
};

use libp2p::identity::ed25519 as libp2p_ed25519;
use primitives::{AccountId, AuraId, AuthorityId};
use sc_cli::KeystoreParams;
use sc_keystore::{Keystore, LocalKeystore};
use sc_service::{config::KeystoreConfig, BasePath};
use serde::{Deserialize, Serialize};
use sp_core::crypto::key_types;

#[derive(Clone, Deserialize, Serialize)]
pub struct AccountSessionKeys {
    pub account_id: AccountId,
    pub aura_key: AuraId,
    pub aleph_key: AuthorityId,
}

/// returns Aura key, if absent a new key is generated
fn create_aura_key(keystore: &impl Keystore) -> AuraId {
    Keystore::sr25519_public_keys(keystore, key_types::AURA)
        .pop()
        .unwrap_or_else(|| {
            Keystore::sr25519_generate_new(keystore, key_types::AURA, None)
                .expect("Could not create Aura key")
        })
        .into()
}

/// returns Aleph key, if absent a new key is generated
fn create_aleph_key(keystore: &impl Keystore) -> AuthorityId {
    Keystore::ed25519_public_keys(keystore, primitives::KEY_TYPE)
        .pop()
        .unwrap_or_else(|| {
            Keystore::ed25519_generate_new(keystore, primitives::KEY_TYPE, None)
                .expect("Could not create Aleph key")
        })
        .into()
}

fn abft_backup_path(base_path: &Path, backup_dir: &str) -> PathBuf {
    base_path.join(backup_dir)
}

/// Creates a key for p2p network and writes it into keystore
pub fn create_p2p_key(node_key_path: &Path) {
    let keypair = libp2p_ed25519::Keypair::generate();
    let secret = keypair.secret();
    let secret_hex = hex::encode(secret.as_ref());
    fs::write(node_key_path, secret_hex).expect("Could not write p2p secret");
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

pub fn create_account_session_keys(
    keystore: &impl Keystore,
    account_id: AccountId,
) -> AccountSessionKeys {
    let aura_key = create_aura_key(keystore);
    let aleph_key = create_aleph_key(keystore);

    AccountSessionKeys {
        account_id,
        aura_key,
        aleph_key,
    }
}

pub fn create_abft_backup_dir(base_path_with_account_id: &Path, backup_dir: &str) {
    let backup_path = abft_backup_path(base_path_with_account_id, backup_dir);

    if backup_path.exists() {
        if !backup_path.is_dir() {
            panic!("Could not create backup directory at {backup_path:?}. Path is already a file.");
        }
    } else {
        fs::create_dir_all(backup_path).expect("Could not create backup directory.");
    }
}
