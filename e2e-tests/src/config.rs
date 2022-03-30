use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
pub struct Config {
    /// WS endpoint address of the node to connect to
    #[clap(long, default_value = "127.0.0.1:9943")]
    pub node: String,

    /// seed values to create accounts
    #[clap(long)]
    pub seeds: Option<Vec<String>>,

    /// seed value of sudo account
    #[clap(long)]
    pub sudo: Option<String>,
}
