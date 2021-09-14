use aleph_primitives::{
    AuthorityId as AlephId, DEFAULT_MILLISECS_PER_BLOCK, DEFAULT_SESSION_PERIOD,
};
use aleph_runtime::{
    AccountId, AlephConfig, AuraConfig, BalancesConfig, GenesisConfig, SessionConfig, SessionKeys,
    Signature, SudoConfig, SystemConfig, VestingConfig, WASM_BINARY,
};
use hex_literal::hex;
use sc_service::config::BasePath;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_application_crypto::Ss58Codec;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::path::PathBuf;
use structopt::StructOpt;

const FAUCET_HASH: [u8; 32] =
    hex!("eaefd9d9b42915bda608154f17bb03e407cbf244318a0499912c2fb1cd879b74");

pub const DEVNET_ID: &str = "dev";

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

#[derive(Clone, Deserialize, Serialize)]
pub struct AuthorityKeys {
    pub account_id: AccountId,
    pub aura_key: AuraId,
    pub aleph_key: AlephId,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ChainParams {
    /// Pass the chain id.
    ///
    /// It can be a predefined one (dev) or an arbitrary chain id passed to the genesis block
    #[structopt(long, value_name = "CHAIN_SPEC", default_value = "a0dnet1")]
    pub chain_id: String,

    /// Specify custom base path.
    #[structopt(long, short = "d", value_name = "PATH", parse(from_os_str))]
    pub base_path: PathBuf,

    #[structopt(long)]
    pub session_period: Option<u32>,

    #[structopt(long)]
    pub millisecs_per_block: Option<u64>,

    #[structopt(long)]
    pub chain_name: Option<String>,

    #[structopt(long)]
    pub token_symbol: Option<String>,

    /// Pass the AccountIds of authorities forming the committe at the genesis
    ///
    /// Expects a delimited collection of AccountIds
    /// If this argument is not found n_members is used instead to generate a collection of size `n_members`
    /// filled with randomly generated Ids
    #[structopt(long, require_delimiter = true)]
    account_ids: Option<Vec<String>>,

    #[structopt(long)]
    n_members: Option<u32>,
}

impl ChainParams {
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    pub fn base_path(&self) -> BasePath {
        self.base_path.clone().into()
    }

    pub fn millisecs_per_block(&self) -> u64 {
        self.millisecs_per_block
            .unwrap_or(DEFAULT_MILLISECS_PER_BLOCK)
    }

    pub fn session_period(&self) -> u32 {
        self.session_period.unwrap_or(DEFAULT_SESSION_PERIOD)
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
                let n_members = self
                    .n_members
                    .expect("Pass account-ids or n-members argument");

                (0..n_members)
                    .into_iter()
                    .map(|id| {
                        let seed = id.to_string();
                        get_account_id_from_seed::<sr25519::Public>(&seed.as_str())
                    })
                    .collect()
            }
        }
    }
}

pub fn development_config(
    chain_params: ChainParams,
    authorities: Vec<AuthorityKeys>,
) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    let rich_accounts: Vec<_> = [
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
    // Also give money to the faucet account.
    .chain(std::iter::once(FAUCET_HASH.into()))
    .collect();

    let sudo_account = rich_accounts[0].clone();
    let token_symbol = String::from(chain_params.token_symbol());
    let chain_name = String::from(chain_params.chain_name());

    Ok(ChainSpec::from_genesis(
        // Name
        &chain_name,
        // ID
        DEVNET_ID,
        ChainType::Development,
        move || {
            genesis(
                wasm_binary,
                // Initial PoA authorities
                authorities.clone(),
                // Pre-funded accounts
                sudo_account.clone(),
                rich_accounts.clone(),
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
        Some(
            [(
                "tokenSymbol".to_string(),
                serde_json::Value::String(token_symbol),
            )]
            .iter()
            .cloned()
            .collect(),
        ),
        // Extensions
        None,
    ))
}

pub fn config(
    chain_params: ChainParams,
    authorities: Vec<AuthorityKeys>,
    chain_id: &str,
) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    let sudo_account: AccountId = hex![
        // 5F4SvwaUEQubiqkPF8YnRfcN77cLsT2DfG4vFeQmSXNjR7hD
        "848274306fea52dc528eabc8e14e6ae78ea275bc4247a5d6e2882ac8e948fe68"
    ]
    .into();

    let token_symbol = String::from(chain_params.token_symbol());
    let chain_name = String::from(chain_params.chain_name());

    // Give money to the faucet account
    let faucet: AccountId = FAUCET_HASH.into();
    let rich_accounts = vec![faucet, sudo_account.clone()];

    Ok(ChainSpec::from_genesis(
        // Name
        &chain_name,
        // ID
        chain_id,
        ChainType::Live,
        move || {
            genesis(
                wasm_binary,
                // Initial PoA authorities
                authorities.clone(),
                sudo_account.clone(),
                // Pre-funded accounts
                rich_accounts.clone(),
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
        Some(
            [(
                "tokenSymbol".to_string(),
                serde_json::Value::String(token_symbol),
            )]
            .iter()
            .cloned()
            .collect(),
        ),
        // Extensions
        None,
    ))
}

/// Configure initial storage state for FRAME modules
fn genesis(
    wasm_binary: &[u8],
    authorities: Vec<AuthorityKeys>,
    root_key: AccountId,
    rich_accounts: Vec<AccountId>,
    chain_params: ChainParams,
) -> GenesisConfig {
    let session_period = chain_params.session_period();
    let millisecs_per_block = chain_params.millisecs_per_block();

    log::debug!("chain parameters {:?}", &chain_params);

    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        },
        balances: BalancesConfig {
            // Configure endowed accounts with initial balance of 1 << 60.
            balances: authorities
                .iter()
                .map(|auth| &auth.account_id)
                .cloned()
                .chain(rich_accounts.into_iter())
                .map(|k| (k, 1 << 60))
                .collect(),
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
