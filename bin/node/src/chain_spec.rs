use aleph_primitives::{
    AuthorityId as AlephId, ADDRESSES_ENCODING, DEFAULT_MILLISECS_PER_BLOCK,
    DEFAULT_SESSION_PERIOD, DEFAULT_UNIT_CREATION_DELAY, TOKEN_DECIMALS,
};
use aleph_primitives::{MillisecsPerBlock, SessionPeriod, UnitCreationDelay};
use aleph_runtime::{
    AccountId, AlephConfig, AuraConfig, BalancesConfig, GenesisConfig, SessionConfig, SessionKeys,
    Signature, SudoConfig, SystemConfig, VestingConfig, WASM_BINARY,
};
use hex_literal::hex;
use libp2p::PeerId;
use sc_service::config::BasePath;
use sc_service::ChainType;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Number, Value};
use sp_application_crypto::Ss58Codec;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::collections::HashSet;
use std::{path::PathBuf, str::FromStr};
use structopt::StructOpt;

pub const DEVNET_ID: &str = "dev";

pub const WELL_KNOWNS_ACCOUNTS: [&str; 7] = [
    "Alice",
    "Alice//stash",
    "Bob",
    "Bob//stash",
    "Charlie",
    "Dave",
    "Eve",
];

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
pub fn get_account_id_from_seed<TPublic: Public>(seed: &&str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AuthorityKeys {
    pub account_id: AccountId,
    pub aura_key: AuraId,
    pub aleph_key: AlephId,
    pub peer_id: SerializablePeerId,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ChainParams {
    /// Pass the chain id.
    ///
    /// It can be a predefined one (dev) or an arbitrary chain id passed to the genesis block
    /// `dev` chain id means that a set of known accounts will be used to form a comittee
    #[structopt(long, value_name = "CHAIN_SPEC", default_value = "a0dnet1")]
    pub chain_id: String,

    /// Specify custom base path.
    #[structopt(long, short = "d", value_name = "PATH", parse(from_os_str))]
    pub base_path: PathBuf,

    /// Specify filename to write node private p2p keys to
    /// Resulting keys will be stored at: base_path/account_id/node_key_file for each node
    #[structopt(long, default_value = "p2p_secret")]
    pub node_key_file: String,

    /// Specify filename to write the nodes private key to
    /// Used in conjunction with `--n-members` argument
    /// Resulting keys will be stored at: base_path/account_id/account_key_file for each node
    #[structopt(long, default_value = "account_secret")]
    pub account_key_file: String,

    #[structopt(long)]
    pub session_period: Option<u32>,

    #[structopt(long)]
    pub millisecs_per_block: Option<u64>,

    #[structopt(long)]
    pub unit_creation_delay: Option<u64>,

    #[structopt(long)]
    pub chain_name: Option<String>,

    #[structopt(long)]
    pub token_symbol: Option<String>,

    /// Pass the AccountIds of authorities forming the committe at the genesis
    ///
    /// Expects a delimited collection of AccountIds
    #[structopt(long, require_delimiter = true)]
    account_ids: Option<Vec<String>>,

    /// Pass the AccountId of the sudo account
    ///
    /// If the chain-id is "dev" it will default to the first generated account (Alice)
    /// and use a default pre-defined id in any other case
    #[structopt(long)]
    sudo_account_id: Option<String>,
}

impl ChainParams {
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    pub fn base_path(&self) -> BasePath {
        self.base_path.clone().into()
    }

    pub fn millisecs_per_block(&self) -> MillisecsPerBlock {
        MillisecsPerBlock(
            self.millisecs_per_block
                .unwrap_or(DEFAULT_MILLISECS_PER_BLOCK),
        )
    }

    pub fn unit_creation_delay(&self) -> UnitCreationDelay {
        UnitCreationDelay(
            self.unit_creation_delay
                .unwrap_or(DEFAULT_UNIT_CREATION_DELAY),
        )
    }

    pub fn session_period(&self) -> SessionPeriod {
        SessionPeriod(self.session_period.unwrap_or(DEFAULT_SESSION_PERIOD))
    }

    pub fn token_symbol(&self) -> &str {
        match &self.token_symbol {
            Some(symbol) => symbol,
            None => "DZERO",
        }
    }

    pub fn chain_name(&self) -> &str {
        match &self.chain_name {
            Some(name) => name,
            None => "Aleph Zero Development",
        }
    }

    pub fn account_ids(&self) -> Vec<AccountId> {
        match &self.account_ids {
            Some(ids) => ids
                .iter()
                .map(|id| {
                    AccountId::from_string(id.as_str())
                        .expect("Passed string is not a hex encoding of a public key")
                })
                .collect(),
            None => {
                // NOTE : chain id "dev" means that a set of known accounts is generated from KNOWN_ACCOUNTS seed values
                // this follows the default Substrate behaviour
                match self.chain_id() {
                    DEVNET_ID => WELL_KNOWNS_ACCOUNTS
                        .iter()
                        .map(get_account_id_from_seed::<sr25519::Public>)
                        .collect(),
                    _ => panic!("Pass account-ids or use chain-id dev"),
                }
            }
        }
    }

    pub fn sudo_account_id(&self) -> AccountId {
        match &self.sudo_account_id {
            // account is passed explicitely as a CLI argument
            Some(id) => AccountId::from_string(id)
                .expect("Passed string is not a hex encoding of a public key"),
            // provide some sensible defaults
            None => match self.chain_id() {
                // defaults to the first account if chain is "dev", this is the same as substarte default behaviour
                DEVNET_ID => get_account_id_from_seed::<sr25519::Public>(&WELL_KNOWNS_ACCOUNTS[0]),
                // hardcoded account for any other chain
                _ => hex![
                    // 5F4SvwaUEQubiqkPF8YnRfcN77cLsT2DfG4vFeQmSXNjR7hD
                    "848274306fea52dc528eabc8e14e6ae78ea275bc4247a5d6e2882ac8e948fe68"
                ]
                .into(),
            },
        }
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

pub fn config(
    chain_params: ChainParams,
    authorities: Vec<AuthorityKeys>,
    chain_id: &str,
) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
    let token_symbol = String::from(chain_params.token_symbol());
    let chain_name = String::from(chain_params.chain_name());
    let sudo_account = chain_params.sudo_account_id();

    // NOTE: this could be passed as a CLI argument similar to the sudo-account id
    let faucet_account: AccountId =
        hex!["eaefd9d9b42915bda608154f17bb03e407cbf244318a0499912c2fb1cd879b74"].into();

    let chain_type = match chain_id {
        DEVNET_ID => ChainType::Development,
        _ => ChainType::Live,
    };

    Ok(ChainSpec::from_genesis(
        // Name
        &chain_name,
        // ID
        chain_id,
        chain_type,
        move || {
            genesis(
                wasm_binary,
                authorities.clone(), // Initial PoA authorities, will receive funds
                sudo_account.clone(), // Root account, will also be pre funded
                faucet_account.clone(), // Pre-funded faucet account
                chain_params.clone(),
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
fn genesis(
    wasm_binary: &[u8],
    authorities: Vec<AuthorityKeys>,
    root_key: AccountId,
    faucet_account: AccountId,
    chain_params: ChainParams,
) -> GenesisConfig {
    let session_period = chain_params.session_period();
    let millisecs_per_block = chain_params.millisecs_per_block();
    let unit_creation_delay = chain_params.unit_creation_delay();

    // NOTE: some combinations of bootstrap chain arguments can potentially
    // lead to duplicated rich accounts, e.g. if a root account is also an authority
    // which is why we remove the duplicates if any here
    let unique_accounts: Vec<AccountId> = deduplicate(
        authorities
            .iter()
            .map(|auth| &auth.account_id)
            .cloned()
            .chain(vec![root_key.clone(), faucet_account])
            .collect(),
    );

    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        },
        balances: BalancesConfig {
            // Configure endowed accounts with initial balance of 1 << 60.
            balances: unique_accounts.into_iter().map(|k| (k, 1 << 60)).collect(),
        },
        aura: AuraConfig {
            authorities: vec![],
        },
        sudo: SudoConfig {
            // Assign network admin rights.
            key: root_key,
        },
        aleph: AlephConfig {
            authorities: vec![],
            session_period,
            millisecs_per_block,
            validators: authorities
                .iter()
                .map(|auth| auth.account_id.clone())
                .collect(),
            unit_creation_delay,
        },
        session: SessionConfig {
            keys: authorities
                .into_iter()
                .map(|auth| {
                    (
                        auth.account_id.clone(),
                        auth.account_id.clone(),
                        SessionKeys {
                            aura: auth.aura_key.clone(),
                            aleph: auth.aleph_key,
                        },
                    )
                })
                .collect(),
        },
        vesting: VestingConfig { vesting: vec![] },
    }
}
