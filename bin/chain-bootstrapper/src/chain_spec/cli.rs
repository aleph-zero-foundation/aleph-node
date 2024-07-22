use primitives::{AccountId, Version as FinalityVersion, LEGACY_FINALITY_VERSION};
use sc_chain_spec::ChainType;
use sc_cli::clap::{self, Args};

use crate::chain_spec::{
    parse_account_id, parse_chaintype, CHAINTYPE_LIVE, DEFAULT_CHAIN_ID, DEFAULT_SUDO_ACCOUNT_ALICE,
};

#[derive(Debug, Args, Clone)]
pub struct ChainSpecParams {
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

    /// all account ids that needs to have session keys generated when bootstraping chain (comma delimited)
    #[arg(long, value_delimiter = ',', value_parser = parse_account_id)]
    account_ids: Vec<AccountId>,

    /// AccountIds of authorities forming the committee at the genesis (comma delimited)
    /// If empty, then `--account-ids` are used as authorities.
    /// If not empty, it should be a subset of `--account-ids`.
    #[arg(long, value_delimiter = ',', value_parser = parse_account_id)]
    authorities_account_ids: Vec<AccountId>,

    /// AccountId of the sudo account
    #[arg(long, value_parser = parse_account_id, default_value(DEFAULT_SUDO_ACCOUNT_ALICE))]
    sudo_account_id: AccountId,

    /// Accounts that will receive initial endowment in genesis block
    #[arg(long, value_delimiter = ',', value_parser = parse_account_id)]
    rich_account_ids: Option<Vec<AccountId>>,

    /// Finality version at chain inception.
    #[arg(long, default_value = LEGACY_FINALITY_VERSION.to_string())]
    finality_version: FinalityVersion,
}

impl ChainSpecParams {
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

    pub fn authorities_account_ids(&self) -> Vec<AccountId> {
        self.authorities_account_ids.clone()
    }

    pub fn sudo_account_id(&self) -> AccountId {
        self.sudo_account_id.clone()
    }

    pub fn rich_account_ids(&self) -> Option<Vec<AccountId>> {
        self.rich_account_ids.clone()
    }

    pub fn finality_version(&self) -> FinalityVersion {
        self.finality_version
    }
}
