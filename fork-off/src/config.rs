use std::error::Error;

use clap::Parser;

use crate::types::{AccountId, Balance, BlockHash, StoragePath};

#[derive(Debug, Parser)]
#[clap(version = "1.0")]
pub struct Config {
    /// URL address of the node websocket RPC endpoint for the chain you are forking.
    #[clap(long, default_value = "ws://127.0.0.1:9944")]
    pub ws_rpc_endpoint: String,

    /// Path of the initial chainspec (generated with the `bootstrap-chain` command).
    #[clap(long, default_value = "./initial_chainspec.json")]
    pub initial_spec_path: String,

    /// Where to write the snapshot of the current chain state.
    #[clap(long, default_value = "./snapshot.json")]
    pub snapshot_path: String,

    /// Where to write the forked genesis chainspec.
    #[clap(long, default_value = "./chainspec_from_snapshot.json")]
    pub combined_spec_path: String,

    /// Whether to read the state from the ready snapshot file.
    #[clap(long)]
    pub use_snapshot_file: bool,

    /// Maximum concurrent requests to use while downloading storage values.
    #[clap(long, default_value_t = 1000)]
    pub max_requests: u32,

    /// Which modules to keep in forked spec.
    #[clap(
        long,
        multiple_occurrences = true,
        takes_value = true,
        value_delimiter = ',',
        default_value = "Aura,Aleph,Sudo,Staking,Session,Elections"
    )]
    pub storage_keep_state: Vec<StoragePath>,

    #[clap(long)]
    pub accounts_path: Option<String>,

    #[clap(
        long,
        parse(try_from_str = parse_balances),
        value_delimiter = ',',
        multiple_occurrences(true))
    ]
    pub balances: Option<Vec<(AccountId, Balance)>>,

    #[clap(long)]
    pub at_block: Option<BlockHash>,
}

fn parse_balances(s: &str) -> Result<(AccountId, Balance), Box<dyn Error + Send + Sync + 'static>> {
    let sep_pos = s.find('=').ok_or("Invalid ACCOUNT=BALANCE: no `=` found")?;

    let account_raw: String = s[..sep_pos].parse()?;
    let account = AccountId::new(&account_raw);
    let balance = s[sep_pos + 1..].parse()?;
    Ok((account, balance))
}
