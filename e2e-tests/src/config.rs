use clap::Parser;

use aleph_client::{from as parse_to_protocol, Protocol};

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
pub struct Config {
    /// WS endpoint address of the node to connect to
    #[clap(long, default_value = "127.0.0.1:9943")]
    pub node: String,

    /// Protocol to be used for connecting to node (`ws` or `wss`)
    #[clap(name = "use_ssl", parse(from_flag = parse_to_protocol))]
    pub protocol: Protocol,

    /// seed values to create accounts
    #[clap(long)]
    pub seeds: Option<Vec<String>>,

    /// seed value of sudo account
    #[clap(long)]
    pub sudo: Option<String>,
}
