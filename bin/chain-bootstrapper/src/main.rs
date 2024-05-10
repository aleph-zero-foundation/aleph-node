mod chain_spec;

use sc_cli::clap::{self, Parser, Subcommand as ClapSubcommand};

use crate::chain_spec::{BootstrapChainCmd, ConvertChainspecToRawCmd};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, ClapSubcommand)]
pub enum Subcommand {
    /// Generates keystore (libp2p key and session keys), and generates chainspec to stdout
    BootstrapChain(BootstrapChainCmd),

    /// Takes a chainspec and generates a corresponding raw chainspec
    ConvertChainspecToRaw(ConvertChainspecToRawCmd),
}

fn main() -> sc_cli::Result<()> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Some(Subcommand::BootstrapChain(cmd)) => cmd.run(),
        Some(Subcommand::ConvertChainspecToRaw(cmd)) => cmd.run(),

        None => Err("Command was required!".into()),
    }
}
