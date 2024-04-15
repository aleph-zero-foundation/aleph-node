use std::io::Write;

use primitives::{AccountId, Version as FinalityVersion, LEGACY_FINALITY_VERSION};
use sc_chain_spec::ChainType;
use sc_cli::{
    clap::{self, Args, Parser},
    Error, KeystoreParams,
};
use sc_service::BasePath;
use sp_application_crypto::Ss58Codec;

use crate::{
    chain_spec::{
        builder::build_chain_spec, CHAINTYPE_DEV, CHAINTYPE_LIVE, CHAINTYPE_LOCAL,
        DEFAULT_CHAIN_ID, DEFAULT_SUDO_ACCOUNT,
    },
    commands::{authority_keys, bootstrap_backup, open_keystore},
    shared_params::SharedParams,
};

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
    #[arg(long, value_delimiter = ',', value_parser = parse_account_id, num_args = 1..)]
    account_ids: Vec<AccountId>,

    /// AccountId of the sudo account
    #[arg(long, value_parser = parse_account_id, default_value(DEFAULT_SUDO_ACCOUNT))]
    sudo_account_id: AccountId,

    /// Accounts that will receive initial endowment in genesis block
    #[arg(long, value_delimiter = ',', value_parser = parse_account_id, num_args = 1..)]
    rich_account_ids: Option<Vec<AccountId>>,

    /// Optional faucet account to be endowed
    #[arg(long, value_parser = parse_account_id)]
    faucet_account_id: Option<AccountId>,

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

    pub fn finality_version(&self) -> FinalityVersion {
        self.finality_version
    }
}

fn parse_chaintype(s: &str) -> Result<ChainType, Error> {
    Ok(match s {
        CHAINTYPE_DEV => ChainType::Development,
        CHAINTYPE_LOCAL => ChainType::Local,
        CHAINTYPE_LIVE => ChainType::Live,
        s => panic!("Wrong chain type {s} Possible values: dev local live"),
    })
}

/// The `bootstrap-chain` command is used to generate private keys for the genesis authorities
/// keys are written to the keystore of the authorities
/// and the chain specification is printed to stdout in the JSON format
#[derive(Debug, Parser)]
pub struct BootstrapChainCmd {
    /// Force raw genesis storage output.
    #[arg(long = "raw")]
    pub raw: bool,

    #[clap(flatten)]
    pub keystore_params: KeystoreParams,

    #[clap(flatten)]
    pub chain_params: ChainParams,

    #[clap(flatten)]
    pub node_params: SharedParams,
}

/// Assumes an input path: some_path/, which is appended to finally become: some_path/account_id
impl BootstrapChainCmd {
    pub fn run(&self) -> Result<(), Error> {
        let base_path = self.node_params.base_path();
        let backup_dir = self.node_params.backup_dir();
        let node_key_file = self.node_params.node_key_file();
        let chain_id = self.chain_params.chain_id();

        let genesis_authorities = self
            .chain_params
            .account_ids()
            .into_iter()
            .map(|account_id| {
                let account_base_path: BasePath =
                    base_path.path().join(account_id.to_string()).into();
                bootstrap_backup(account_base_path.path(), backup_dir);
                let keystore = open_keystore(&self.keystore_params, chain_id, &account_base_path);
                authority_keys(
                    &keystore,
                    account_base_path.path(),
                    node_key_file,
                    account_id,
                )
            })
            .collect();

        let chain_spec = build_chain_spec(self.chain_params.clone(), genesis_authorities)?;

        let json = sc_service::chain_ops::build_spec(&chain_spec, self.raw)?;
        if std::io::stdout().write_all(json.as_bytes()).is_err() {
            let _ = std::io::stderr().write_all(b"Error writing to stdout\n");
        }

        Ok(())
    }
}

/// Generate AccountId based on string command line argument.
fn parse_account_id(s: &str) -> Result<AccountId, Error> {
    Ok(AccountId::from_string(s).expect("Passed string is not a hex encoding of a public key"))
}
