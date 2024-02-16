use std::{fs, path::PathBuf};

use clap::Parser;

#[derive(Debug, Parser)]
#[clap(version = "1.0")]
pub struct Config {
    /// URL address(es) of the nodes (in the same network) to send transactions to
    #[clap(long, default_value = "ws://127.0.0.1:9944")]
    pub nodes: Vec<String>,

    /// How many transactions to send in the interval
    #[clap(long)]
    pub transactions_in_interval: u64,

    /// How long the interval is, in secs
    #[clap(long, default_value = "1")]
    pub interval_duration: u64,

    /// For how many intervals should the flood last
    #[clap(long, default_value = "180")]
    pub intervals: u64,

    /// Secret phrase : a path to a file or passed on stdin
    #[clap(long)]
    pub phrase: Option<String>,

    /// Secret seed of the account keypair passed on stdin
    #[clap(long, conflicts_with_all = &["phrase"])]
    pub seed: Option<String>,

    /// Allows to skip accounts
    #[clap(long)]
    pub skip_initialization: bool,

    /// Beginning of the integer range used to derive accounts
    #[clap(long, default_value = "0")]
    pub first_account_in_range: u64,

    /// Changes the awaited status of every transaction from `SubmitOnly` to `Ready`
    #[clap(long)]
    pub wait_for_ready: bool,

    /// Flooder will pause sending transactions to the node, if there are more than
    /// `pool_limit` transactions in the tx pool of the node. Should
    /// be smaller than `--pool-limit` parameter of nodes.
    #[clap(long, default_value = "6144")]
    pub pool_limit: u64,
}

pub fn read_phrase(phrase: String) -> String {
    let file = PathBuf::from(&phrase);
    if file.is_file() {
        fs::read_to_string(phrase).unwrap().trim_end().to_owned()
    } else {
        phrase
    }
}
