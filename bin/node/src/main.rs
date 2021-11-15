mod chain_spec;
#[macro_use]
mod service;
mod aleph_cli;
mod cli;
mod command;
mod commands;
mod rpc;

fn main() -> sc_cli::Result<()> {
    command::run()
}
