use aleph_primitives::{
    staking::{MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND},
    AuthorityId as AlephId, ADDRESSES_ENCODING, TOKEN_DECIMALS,
};
use aleph_runtime::{
    AccountId, AuraConfig, BalancesConfig, ElectionsConfig, GenesisConfig, Perbill, SessionConfig,
    SessionKeys, Signature, StakingConfig, SudoConfig, SystemConfig, VestingConfig, WASM_BINARY,
};
use libp2p::PeerId;
use pallet_staking::{Forcing, StakerStatus};
use sc_service::{config::BasePath, ChainType};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Number, Value};
use sp_application_crypto::Ss58Codec;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::{collections::HashSet, path::PathBuf, str::FromStr};
use structopt::StructOpt;

pub const CHAINTYPE_DEV: &str = "dev";
pub const CHAINTYPE_LOCAL: &str = "local";
pub const CHAINTYPE_LIVE: &str = "live";

pub const DEFAULT_CHAIN_ID: &str = "a0dnet1";

// Alice is the default sudo holder.
pub const DEFAULT_SUDO_ACCOUNT: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

#[derive(Clone)]
pub struct SerializablePeerId {
    inner: PeerId,
}

impl SerializablePeerId {
    pub fn new(inner: PeerId) -> SerializablePeerId {
        SerializablePeerId { inner }
    }
}

impl Serialize for SerializablePeerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s: String = format!("{}", self.inner);
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for SerializablePeerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let inner = PeerId::from_str(&s)
            .map_err(|_| D::Error::custom(format!("Could not deserialize as PeerId: {}", s)))?;
        Ok(SerializablePeerId { inner })
    }
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate a crypto pair from seed.
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate AccountId based on string command line argument.
fn parse_account_id(s: &str) -> AccountId {
    AccountId::from_string(s).expect("Passed string is not a hex encoding of a public key")
}

fn parse_chaintype(s: &str) -> ChainType {
    match s {
        CHAINTYPE_DEV => ChainType::Development,
        CHAINTYPE_LOCAL => ChainType::Local,
        CHAINTYPE_LIVE => ChainType::Live,
        s => panic!("Wrong chain type {} Possible values: dev local live", s),
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AuthorityKeys {
    pub account_id: AccountId,
    pub aura_key: AuraId,
    pub aleph_key: AlephId,
    pub peer_id: SerializablePeerId,
}

fn to_account_ids(authorities: &[AuthorityKeys]) -> impl Iterator<Item = AccountId> + '_ {
    authorities.iter().map(|auth| auth.account_id.clone())
}

#[derive(Debug, StructOpt, Clone)]
pub struct ChainParams {
    /// Chain ID is a short identifier of the chain
    #[structopt(long, value_name = "ID", default_value = DEFAULT_CHAIN_ID)]
    chain_id: String,

    /// The type of the chain. Possible values: "dev", "local", "live" (default)
    #[structopt(long, value_name = "TYPE", parse(from_str = parse_chaintype), default_value = CHAINTYPE_LIVE)]
    chain_type: ChainType,

    /// Specify custom base path
    #[structopt(long, short = "d", value_name = "PATH", parse(from_os_str))]
    base_path: PathBuf,

    /// Specify filename to write node private p2p keys to
    /// Resulting keys will be stored at: base_path/account_id/node_key_file for each node
    #[structopt(long, default_value = "p2p_secret")]
    node_key_file: String,

    /// Chain name. Default is "Aleph Zero Development"
    #[structopt(long, default_value = "Aleph Zero Development")]
    chain_name: String,

    /// Token symbol. Default is DZERO
    #[structopt(long, default_value = "DZERO")]
    token_symbol: String,

    /// AccountIds of authorities forming the committee at the genesis (comma delimited)
    #[structopt(long, require_delimiter = true, parse(from_str = parse_account_id))]
    account_ids: Vec<AccountId>,

    /// AccountId of the sudo account
    #[structopt(long, parse(from_str = parse_account_id), default_value(DEFAULT_SUDO_ACCOUNT))]
    sudo_account_id: AccountId,

    /// AccountId of the optional faucet account
    #[structopt(long, parse(from_str = parse_account_id))]
    faucet_account_id: Option<AccountId>,
}

impl ChainParams {
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    pub fn chain_type(&self) -> ChainType {
        self.chain_type.clone()
    }

    pub fn base_path(&self) -> BasePath {
        self.base_path.clone().into()
    }

    pub fn node_key_file(&self) -> &str {
        &self.node_key_file
    }

    pub fn chain_name(&self) -> &str {
        &self.chain_name
    }

    pub fn token_symbol(&self) -> &str {
        &self.token_symbol
    }

    pub fn account_ids(&self) -> Vec<AccountId> {
        self.account_ids.clone()
    }

    pub fn sudo_account_id(&self) -> AccountId {
        self.sudo_account_id.clone()
    }

    pub fn faucet_account_id(&self) -> Option<AccountId> {
        self.faucet_account_id.clone()
    }
}

fn system_properties(token_symbol: String) -> serde_json::map::Map<String, Value> {
    [
        ("tokenSymbol".to_string(), Value::String(token_symbol)),
        (
            "tokenDecimals".to_string(),
            Value::Number(Number::from(TOKEN_DECIMALS)),
        ),
        (
            "ss58Format".to_string(),
            Value::Number(Number::from(ADDRESSES_ENCODING)),
        ),
    ]
    .iter()
    .cloned()
    .collect()
}

pub fn devnet_config(
    chain_params: ChainParams,
    authorities: Vec<AuthorityKeys>,
) -> Result<ChainSpec, String> {
    // TODO fix it properly so there's a default configuration for distinct stash and controller
    // accounts, and linked controller accounts with validators
    // for now it's better to leave it as empty not to imply (not advised) initial staking configuration
    // in which stash == controller == validator
    generate_chain_spec_config(chain_params, authorities, vec![])
}

pub fn config(
    chain_params: ChainParams,
    authorities: Vec<AuthorityKeys>,
) -> Result<ChainSpec, String> {
    generate_chain_spec_config(chain_params, authorities, vec![])
}

fn generate_chain_spec_config(
    chain_params: ChainParams,
    authorities: Vec<AuthorityKeys>,
    stakers: Vec<AccountId>,
) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
    let token_symbol = String::from(chain_params.token_symbol());
    let chain_name = String::from(chain_params.chain_name());
    let chain_id = String::from(chain_params.chain_id());
    let chain_type = chain_params.chain_type();
    let sudo_account = chain_params.sudo_account_id();
    let faucet_account = chain_params.faucet_account_id();

    Ok(ChainSpec::from_genesis(
        // Name
        &chain_name,
        // ID
        &chain_id,
        chain_type,
        move || {
            generate_genesis_config(
                wasm_binary,
                authorities.clone(), // Initial PoA authorities, will receive funds
                sudo_account.clone(), // Sudo account, will also be pre funded
                faucet_account.clone(), // Pre-funded faucet account
                stakers.clone(),
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        Some(system_properties(token_symbol)),
        // Extensions
        None,
    ))
}

/// Given a Vec<AccountIds> returns a unique collection
fn deduplicate(accounts: Vec<AccountId>) -> Vec<AccountId> {
    let set: HashSet<_> = accounts.into_iter().collect();
    set.into_iter().collect()
}

/// Configure initial storage state for FRAME modules
fn generate_genesis_config(
    wasm_binary: &[u8],
    authorities: Vec<AuthorityKeys>,
    sudo_account: AccountId,
    faucet_account: Option<AccountId>,
    stakers: Vec<AccountId>,
) -> GenesisConfig {
    let special_accounts = match faucet_account {
        Some(faucet_id) => vec![sudo_account.clone(), faucet_id],
        None => vec![sudo_account.clone()],
    };

    // NOTE: some combinations of bootstrap chain arguments can potentially
    // lead to duplicated rich accounts, e.g. if a sudo account is also an authority
    // which is why we remove the duplicates if any here
    let unique_accounts: Vec<AccountId> = deduplicate(
        to_account_ids(&authorities)
            .chain(special_accounts)
            .chain(stakers.iter().cloned())
            .collect(),
    );

    // to have total issuance 300M (for devnet/tests/local runs only)
    const ENDOWMENT: u128 = 60_000_000u128 * 10u128.pow(TOKEN_DECIMALS);

    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
        },
        balances: BalancesConfig {
            // Configure endowed accounts with an initial, significant balance
            balances: unique_accounts
                .into_iter()
                .map(|account| (account, ENDOWMENT))
                .collect(),
        },
        aura: AuraConfig {
            authorities: vec![],
        },
        sudo: SudoConfig {
            // Assign network admin rights.
            key: sudo_account,
        },
        elections: ElectionsConfig {
            members: to_account_ids(&authorities).collect(),
        },
        session: SessionConfig {
            keys: authorities
                .iter()
                .map(|auth| {
                    (
                        auth.account_id.clone(),
                        auth.account_id.clone(),
                        SessionKeys {
                            aura: auth.aura_key.clone(),
                            aleph: auth.aleph_key.clone(),
                        },
                    )
                })
                .collect(),
        },
        staking: StakingConfig {
            force_era: Forcing::NotForcing,
            validator_count: authorities.len() as u32,
            // to satisfy some e2e tests as this cannot be changed during runtime
            minimum_validator_count: 4,
            invulnerables: to_account_ids(&authorities).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            stakers: stakers
                .into_iter()
                .map(|account_id| {
                    (
                        account_id.clone(),
                        account_id,
                        MIN_VALIDATOR_BOND,
                        StakerStatus::Validator,
                    )
                })
                .collect(),
            min_validator_bond: MIN_VALIDATOR_BOND,
            min_nominator_bond: MIN_NOMINATOR_BOND,
            ..Default::default()
        },
        treasury: Default::default(),
        vesting: VestingConfig { vesting: vec![] },
    }
}

pub fn mainnet_config() -> Result<ChainSpec, String> {
    ChainSpec::from_json_bytes(crate::resources::mainnet_chainspec())
}

pub fn testnet_config() -> Result<ChainSpec, String> {
    ChainSpec::from_json_bytes(crate::resources::testnet_chainspec())
}
