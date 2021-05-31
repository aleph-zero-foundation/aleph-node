use std::collections::HashMap;

use aleph_primitives::AuthorityId as AlephId;
use aleph_runtime::{
    AccountId, AlephConfig, AuraConfig, BalancesConfig, GenesisConfig, SessionConfig, SessionKeys,
    Signature, StakingConfig, SudoConfig, SystemConfig, WASM_BINARY,
};
use pallet_staking::StakerStatus;
use sc_service::ChainType;
use sp_application_crypto::key_types;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

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
    let auth_keys: HashMap<u32, Vec<[u8; 32]>> =
        serde_json::from_str(&auth_keys).expect("should contain list of keys");

    let aura_keys: Vec<_> = auth_keys
        .get(&key_types::AURA.into())
        .unwrap()
        .iter()
        .take(n_members)
        .copied()
        .map(|bytes| AuraId::from(sr25519::Public::from_raw(bytes)))
        .collect();

    let aleph_keys: Vec<_> = auth_keys
        .get(&aleph_primitives::KEY_TYPE.into())
        .unwrap()
        .iter()
        .take(n_members)
        .copied()
        .map(|bytes| AlephId::from(sr25519::Public::from_raw(bytes)))
        .collect();

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
                aura_keys.clone(),
                aleph_keys.clone(),
                // Sudo account
                get_account_id_from_seed::<sr25519::Public>(&"Alice"),
                // Pre-funded accounts
                LOCAL_AUTHORITIES
                    .iter()
                    .take(n_members)
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
    aura_authorities: Vec<AuraId>,
    aleph_authorities: Vec<AlephId>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        },
        pallet_balances: BalancesConfig {
            // Configure endowed accounts with initial balance of 1 << 60.
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        },
        pallet_aura: AuraConfig {
            authorities: vec![],
        },
        pallet_sudo: SudoConfig {
            // Assign network admin rights.
            key: root_key,
        },
        pallet_aleph: AlephConfig {
            authorities: aleph_authorities.to_vec(),
        },
        pallet_session: SessionConfig {
            keys: endowed_accounts
                .iter()
                .zip(aura_authorities.iter())
                .zip(aleph_authorities.iter())
                .map(|((account_id, aura_id), aleph_id)| {
                    (
                        account_id.clone(),
                        account_id.clone(),
                        SessionKeys {
                            aura: aura_id.clone(),
                            aleph: aleph_id.clone(),
                        },
                    )
                })
                .collect(),
        },
        pallet_staking: StakingConfig {
            validator_count: endowed_accounts.len() as u32 * 2,
            minimum_validator_count: endowed_accounts.len() as u32,
            stakers: endowed_accounts
                .iter()
                .map(|x| (x.clone(), x.clone(), 1 << 50, StakerStatus::Validator))
                .collect(),
            invulnerables: endowed_accounts.clone(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        },
    }
}
