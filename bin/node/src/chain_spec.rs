use std::{collections::HashSet, str::FromStr, string::ToString};

use aleph_primitives::{
    staking::{MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND},
    AuthorityId as AlephId, SessionValidators, Version as FinalityVersion, ADDRESSES_ENCODING,
    LEGACY_FINALITY_VERSION, TOKEN, TOKEN_DECIMALS,
};
use aleph_runtime::{
    AccountId, AlephConfig, AuraConfig, BalancesConfig, CommitteeManagementConfig, ElectionsConfig,
    GenesisConfig, Perbill, SessionConfig, SessionKeys, StakingConfig, SudoConfig, SystemConfig,
    VestingConfig, WASM_BINARY,
};
use libp2p::PeerId;
use pallet_staking::{Forcing, StakerStatus};
use sc_cli::{
    clap::{self, Args},
    Error as CliError,
};
use sc_service::ChainType;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Number, Value};
use sp_application_crypto::Ss58Codec;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair};

pub const CHAINTYPE_DEV: &str = "dev";
pub const CHAINTYPE_LOCAL: &str = "local";
pub const CHAINTYPE_LIVE: &str = "live";

pub const DEFAULT_CHAIN_ID: &str = "a0dnet1";

// Alice is the default sudo holder.
pub const DEFAULT_SUDO_ACCOUNT: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

pub const DEFAULT_BACKUP_FOLDER: &str = "backup-stash";

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

/// Generate an account ID from seed.
pub fn account_id_from_string(seed: &str) -> AccountId {
    AccountId::from(
        sr25519::Pair::from_string(seed, None)
            .expect("Can't create pair from seed value")
            .public(),
    )
}

/// Generate AccountId based on string command line argument.
fn parse_account_id(s: &str) -> Result<AccountId, CliError> {
    Ok(AccountId::from_string(s).expect("Passed string is not a hex encoding of a public key"))
}

fn parse_chaintype(s: &str) -> Result<ChainType, CliError> {
    Ok(match s {
        CHAINTYPE_DEV => ChainType::Development,
        CHAINTYPE_LOCAL => ChainType::Local,
        CHAINTYPE_LIVE => ChainType::Live,
        s => panic!("Wrong chain type {} Possible values: dev local live", s),
    })
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

#[derive(Debug, Args, Clone)]
pub struct ChainParams {
    /// Chain ID is a short identifier of the chain
    #[arg(long, value_name = "ID", default_value = DEFAULT_CHAIN_ID)]
    chain_id: String,

    /// The type of the chain. Possible values: "dev", "local", "live" (default)
    #[arg(long, value_name = "TYPE", value_parser = parse_chaintype, default_value = CHAINTYPE_LIVE)]
    chain_type: ChainType,

    /// Chain name. Default is "Aleph Zero Development"
    #[arg(long, default_value = "Aleph Zero Development")]
    chain_name: String,

    /// Token symbol. Default is DZERO
    #[arg(long, default_value = "DZERO")]
    token_symbol: String,

    /// AccountIds of authorities forming the committee at the genesis (comma delimited)
    #[arg(long, value_delimiter = ',', value_parser = parse_account_id, num_args=1..)]
    account_ids: Vec<AccountId>,

    /// AccountId of the sudo account
    #[arg(long, value_parser = parse_account_id, default_value(DEFAULT_SUDO_ACCOUNT))]
    sudo_account_id: AccountId,

    /// AccountIds of the optional rich accounts
    #[arg(long, value_delimiter = ',', value_parser = parse_account_id, num_args=1..)]
    rich_account_ids: Option<Vec<AccountId>>,

    /// AccountId of the optional faucet account
    #[arg(long, value_parser = parse_account_id)]
    faucet_account_id: Option<AccountId>,

    /// Minimum number of stakers before chain enters emergency state.
    #[arg(long, default_value = "4")]
    min_validator_count: u32,

    /// Finality version at chain inception.
    #[arg(long, default_value = LEGACY_FINALITY_VERSION.to_string())]
    finality_version: FinalityVersion,
}

impl ChainParams {
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    pub fn chain_type(&self) -> ChainType {
        self.chain_type.clone()
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

    pub fn rich_account_ids(&self) -> Option<Vec<AccountId>> {
        self.rich_account_ids.clone()
    }

    pub fn faucet_account_id(&self) -> Option<AccountId> {
        self.faucet_account_id.clone()
    }

    pub fn min_validator_count(&self) -> u32 {
        self.min_validator_count
    }

    pub fn finality_version(&self) -> FinalityVersion {
        self.finality_version
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

/// Generate chain spec for local runs.
/// Controller accounts are generated for the specified authorities.
pub fn config(
    chain_params: ChainParams,
    authorities: Vec<AuthorityKeys>,
) -> Result<ChainSpec, String> {
    let controller_accounts: Vec<AccountId> = to_account_ids(&authorities)
        .into_iter()
        .enumerate()
        .map(|(index, _account)| {
            account_id_from_string(format!("//{}//Controller", index).as_str())
        })
        .collect();
    generate_chain_spec_config(chain_params, authorities, controller_accounts)
}

fn generate_chain_spec_config(
    chain_params: ChainParams,
    authorities: Vec<AuthorityKeys>,
    controller_accounts: Vec<AccountId>,
) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
    let token_symbol = String::from(chain_params.token_symbol());
    let chain_name = String::from(chain_params.chain_name());
    let chain_id = String::from(chain_params.chain_id());
    let chain_type = chain_params.chain_type();
    let sudo_account = chain_params.sudo_account_id();
    let rich_accounts = chain_params.rich_account_ids();
    let faucet_account = chain_params.faucet_account_id();
    let min_validator_count = chain_params.min_validator_count();
    let finality_version = chain_params.finality_version();

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
                rich_accounts.clone(), // Pre-funded accounts
                faucet_account.clone(), // Pre-funded faucet account
                controller_accounts.clone(), // Controller accounts for staking.
                min_validator_count,
                finality_version,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Fork ID
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

// total issuance of 300M (for devnet/tests/local runs only)
const TOTAL_ISSUANCE: u128 = 300_000_000u128 * 10u128.pow(TOKEN_DECIMALS);

/// Calculate initial endowments such that total issuance is kept approximately constant.
fn calculate_initial_endowment(accounts: &[AccountId]) -> u128 {
    TOTAL_ISSUANCE / (accounts.len() as u128)
}

/// Provides configuration for staking by defining balances, members, keys and stakers.
struct AccountsConfig {
    balances: Vec<(AccountId, u128)>,
    members: Vec<AccountId>,
    keys: Vec<(AccountId, AccountId, SessionKeys)>,
    stakers: Vec<(AccountId, AccountId, u128, StakerStatus<AccountId>)>,
}

/// Provides accounts for GenesisConfig setup based on distinct staking accounts.
/// Assumes validator == stash, but controller is a distinct account
fn configure_chain_spec_fields(
    unique_accounts_balances: Vec<(AccountId, u128)>,
    authorities: Vec<AuthorityKeys>,
    controllers: Vec<AccountId>,
) -> AccountsConfig {
    let balances = unique_accounts_balances
        .into_iter()
        .chain(
            controllers
                .clone()
                .into_iter()
                .map(|account| (account, TOKEN)),
        )
        .collect();

    let keys = authorities
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
        .collect();

    let stakers = authorities
        .iter()
        .zip(controllers)
        .enumerate()
        .map(|(validator_idx, (validator, controller))| {
            (
                validator.account_id.clone(),
                controller,
                (validator_idx + 1) as u128 * MIN_VALIDATOR_BOND,
                StakerStatus::Validator,
            )
        })
        .collect();

    let members = to_account_ids(&authorities).collect();

    AccountsConfig {
        balances,
        members,
        keys,
        stakers,
    }
}

/// Configure initial storage state for FRAME modules.
#[allow(clippy::too_many_arguments)]
fn generate_genesis_config(
    wasm_binary: &[u8],
    authorities: Vec<AuthorityKeys>,
    sudo_account: AccountId,
    rich_accounts: Option<Vec<AccountId>>,
    faucet_account: Option<AccountId>,
    controller_accounts: Vec<AccountId>,
    min_validator_count: u32,
    finality_version: FinalityVersion,
) -> GenesisConfig {
    let special_accounts = {
        let mut all = rich_accounts.unwrap_or_default();
        all.push(sudo_account.clone());
        if let Some(faucet_account) = faucet_account {
            all.push(faucet_account);
        }
        all
    };

    // NOTE: some combinations of bootstrap chain arguments can potentially
    // lead to duplicated rich accounts, e.g. if a sudo account is also an authority
    // which is why we remove the duplicates if any here
    let unique_accounts = deduplicate(
        to_account_ids(&authorities)
            .chain(special_accounts)
            .collect(),
    );

    let endowment = calculate_initial_endowment(&unique_accounts);

    let unique_accounts_balances = unique_accounts
        .into_iter()
        .map(|account| (account, endowment))
        .collect::<Vec<_>>();

    let validator_count = authorities.len() as u32;

    let accounts_config =
        configure_chain_spec_fields(unique_accounts_balances, authorities, controller_accounts);

    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
        },
        balances: BalancesConfig {
            // Configure endowed accounts with an initial, significant balance
            balances: accounts_config.balances,
        },
        aura: AuraConfig {
            authorities: vec![],
        },
        sudo: SudoConfig {
            // Assign network admin rights.
            key: Some(sudo_account),
        },
        elections: ElectionsConfig {
            reserved_validators: accounts_config.members.clone(),
            non_reserved_validators: vec![],
            committee_seats: Default::default(),
        },
        session: SessionConfig {
            keys: accounts_config.keys,
        },
        staking: StakingConfig {
            force_era: Forcing::NotForcing,
            validator_count,
            // to satisfy some e2e tests as this cannot be changed during runtime
            minimum_validator_count: min_validator_count,
            slash_reward_fraction: Perbill::from_percent(10),
            stakers: accounts_config.stakers,
            min_validator_bond: MIN_VALIDATOR_BOND,
            min_nominator_bond: MIN_NOMINATOR_BOND,
            ..Default::default()
        },
        aleph: AlephConfig {
            finality_version,
            ..Default::default()
        },
        treasury: Default::default(),
        vesting: VestingConfig { vesting: vec![] },
        nomination_pools: Default::default(),
        transaction_payment: Default::default(),
        committee_management: CommitteeManagementConfig {
            committee_ban_config: Default::default(),
            session_validators: SessionValidators {
                committee: accounts_config.members,
                non_committee: vec![],
            },
        },
    }
}

pub fn mainnet_config() -> Result<ChainSpec, String> {
    ChainSpec::from_json_bytes(crate::resources::mainnet_chainspec())
}

pub fn testnet_config() -> Result<ChainSpec, String> {
    ChainSpec::from_json_bytes(crate::resources::testnet_chainspec())
}
