use std::collections::HashMap;

use aleph_runtime::{
    AccountId, AuraConfig, BalancesConfig, GenesisConfig, Signature, SudoConfig, SystemConfig,
    WASM_BINARY,
};
use sc_service::ChainType;
use sp_application_crypto::key_types;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

pub(crate) const LOCAL_AUTHORITIES: [&str; 8] = [
    "Damian", "Tomasz", "Zbyszko", "Hansu", "Adam", "Matt", "Antoni", "Michal",
];

pub(crate) const KEY_PATH: &str = "/tmp/authorities_keys";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &&str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    let n_members = std::fs::read_to_string("/tmp/n_members")
        .expect("Committee size is not specified")
        .trim()
        .parse::<usize>()
        .expect("Wrong committee size");

    let auth_keys = std::fs::read_to_string(KEY_PATH).expect("keys were not generated");
    let auth_keys: HashMap<u32, Vec<(AuraId,)>> =
        serde_json::from_str(&auth_keys).expect("should contain list of keys");

    Ok(ChainSpec::from_genesis(
        // Name
        "AlephZero Development",
        // ID
        "dev",
        ChainType::Development,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                auth_keys.get(&key_types::AURA.into()).unwrap()[..n_members].to_vec(),
                // Sudo account
                get_account_id_from_seed::<sr25519::Public>(&"Alice"),
                // Pre-funded accounts
                LOCAL_AUTHORITIES
                    .iter()
                    .map(get_account_id_from_seed::<sr25519::Public>)
                    .collect(),
                true,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        None,
        // Extensions
        None,
    ))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(AuraId,)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig {
            // Configure endowed accounts with initial balance of 1 << 60.
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        }),
        pallet_aura: Some(AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        }),
        pallet_sudo: Some(SudoConfig {
            // Assign network admin rights.
            key: root_key,
        }),
    }
}
