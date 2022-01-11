use sc_cli::SubstrateCli;
use sc_service::PartialComponents;

use aleph_node::{new_full, new_partial, Cli, Subcommand};

fn main() -> sc_cli::Result<()> {
    let cli = Cli::from_args();
    match &cli.subcommand {
        Some(Subcommand::BootstrapChain(cmd)) => cmd.run(),
        Some(Subcommand::BootstrapNode(cmd)) => cmd.run(),
        Some(Subcommand::ConvertChainspecToRaw(cmd)) => cmd.run(),
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, backend), task_manager))
            })
        }
        None => {
            let runner = cli.create_runner(&cli.run)?;
            let aleph_cli_config = cli.aleph;
            runner.run_node_until_exit(|config| async move {
                new_full(config, aleph_cli_config).map_err(sc_cli::Error::Service)
            })
        }
    }
}
