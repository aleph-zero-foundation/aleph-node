mod chain_spec;

use sc_chain_spec::ChainSpec;
use sc_cli::{
    clap::{self, Parser, Subcommand as ClapSubcommand},
    SubstrateCli,
};

use crate::chain_spec::{BootstrapChainCmd, ConvertChainspecToRawCmd};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        panic!("This is not used.")
    }

    fn impl_version() -> String {
        panic!("This is not used.")
    }

    fn description() -> String {
        panic!("This is not used.")
    }

    fn author() -> String {
        panic!("This is not used.")
    }

    fn support_url() -> String {
        panic!("This is not used.")
    }

    fn copyright_start_year() -> i32 {
        panic!("This is not used.")
    }

    fn load_spec(&self, _id: &str) -> Result<Box<dyn ChainSpec>, String> {
        panic!("This is not used.")
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, ClapSubcommand)]
pub enum Subcommand {
    /// Generates keystore (libp2p key and session keys), and generates chainspec to stdout
    BootstrapChain(BootstrapChainCmd),

    /// Takes a chainspec and generates a corresponding raw chainspec
    ConvertChainspecToRaw(ConvertChainspecToRawCmd),

    /// Key management cli utilities
    #[command(subcommand)]
    Key(sc_cli::KeySubcommand),
}

fn main() -> sc_cli::Result<()> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Some(Subcommand::BootstrapChain(cmd)) => cmd.run(),
        Some(Subcommand::ConvertChainspecToRaw(cmd)) => cmd.run(),
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),

        None => Err("Command was required!".into()),
    }
}
