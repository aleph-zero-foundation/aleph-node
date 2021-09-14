use crate::commands::BootstrapNodeCmd;
use crate::{chain_spec, commands::BootstrapChainCmd};
use sc_cli::{ChainSpec, RunCmd, RuntimeVersion, SubstrateCli};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[structopt(flatten)]
    pub run: RunCmd,
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Substrate Node".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "support.anonymous.an".into()
    }

    fn copyright_start_year() -> i32 {
        2021
    }

    fn load_spec(&self, path: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(Box::new(chain_spec::ChainSpec::from_json_file(
            std::path::PathBuf::from(path),
        )?))
    }

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &aleph_runtime::VERSION
    }
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// Key management cli utilities
    Key(sc_cli::KeySubcommand),

    // NOTE: similarly we could have a BootstrapNode command that takes a node-name parameter
    // and writes aura, aleph and (optionally) libp2p private keys to the base-path of a single node
    // and prints accountId and peerId to the stdout
    /// Populate authorities keystore and generate JSON chainspec (printed to stdout)    
    BootstrapChain(BootstrapChainCmd),

    /// Generate and print to stdout keys for a single node
    BootstrapNode(BootstrapNodeCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),
}
