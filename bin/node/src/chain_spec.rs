use std::collections::HashMap;

use aleph_primitives::AuthorityId as AlephId;
use aleph_runtime::{
    AccountId, AlephConfig, AuraConfig, BalancesConfig, GenesisConfig, SessionConfig, SessionKeys,
    Signature, SudoConfig, SystemConfig, WASM_BINARY,
};
use hex_literal::hex;
use sc_service::ChainType;
use sp_application_crypto::key_types;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{ed25519, sr25519, Pair, Public};
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

fn read_keys(n_members: usize) -> (Vec<AuraId>, Vec<AlephId>) {
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
        .map(|bytes| AlephId::from(ed25519::Public::from_raw(bytes)))
        .collect();

    (aura_keys, aleph_keys)
}

pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    let n_members = std::fs::read_to_string("/tmp/n_members")
        .expect("Committee size is not specified")
        .trim()
        .parse::<usize>()
        .expect("Wrong committee size");

    let (aura_keys, aleph_keys) = read_keys(n_members);

    let mut rich_accounts: Vec<_> = [
        "Alice",
        "Alice//stash",
        "Bob",
        "Bob//stash",
        "Charlie",
        "Dave",
        "Eve",
    ]
    .iter()
    .map(get_account_id_from_seed::<sr25519::Public>)
    .collect();
    // Also give money to the faucet account.
    rich_accounts
        .push(hex!["eaefd9d9b42915bda608154f17bb03e407cbf244318a0499912c2fb1cd879b74"].into());
    let sudo_account = rich_accounts[0].clone();
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
                sudo_account.clone(),
                // Pre-funded accounts
                LOCAL_AUTHORITIES
                    .iter()
                    .take(n_members)
                    .map(get_account_id_from_seed::<sr25519::Public>)
                    .collect(),
                rich_accounts.clone(),
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
        Some(
            [(
                "tokenSymbol".to_string(),
                serde_json::Value::String("DZERO".into()),
            )]
            .iter()
            .cloned()
            .collect(),
        ),
        // Extensions
        None,
    ))
}

pub fn testnet1_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    let n_members = 6;
    let (aura_keys, aleph_keys) = read_keys(n_members);

    // Give money to the faucet account.
    let faucet =
        vec![hex!["eaefd9d9b42915bda608154f17bb03e407cbf244318a0499912c2fb1cd879b74"].into()];
    let sudo_public: sr25519::Public = aura_keys[0].clone().into();
    let sudo_account: AccountId = AccountPublic::from(sudo_public).into_account();
    Ok(ChainSpec::from_genesis(
        // Name
        "Aleph Zero",
        // ID
        "a0tnet1",
        ChainType::Live,
        move || {
            testnet_genesis(
                wasm_binary,
                aura_keys.clone(),
                aleph_keys.clone(),
                sudo_account.clone(),
                // Pre-funded accounts
                LOCAL_AUTHORITIES
                    .iter()
                    .take(n_members)
                    .map(get_account_id_from_seed::<sr25519::Public>)
                    .collect(),
                faucet.clone(),
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
        Some(
            [(
                "tokenSymbol".to_string(),
                serde_json::Value::String("TZERO".into()),
            )]
            .iter()
            .cloned()
            .collect(),
        ),
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
    rich_accounts: Vec<AccountId>,
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
                .chain(rich_accounts.into_iter())
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
    }
}
