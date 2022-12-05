#[cfg(feature = "try-runtime")]
use aleph_node::ExecutorDispatch;
use aleph_node::{new_authority, new_full, new_partial, Cli, Subcommand};
#[cfg(feature = "try-runtime")]
use aleph_runtime::Block;
use log::warn;
use sc_cli::{clap::Parser, SubstrateCli};
use sc_network::config::Role;
use sc_service::PartialComponents;

fn main() -> sc_cli::Result<()> {
    let mut cli = Cli::parse();
    if !cli.aleph.experimental_pruning() {
        if cli
            .run
            .import_params
            .pruning_params
            .blocks_pruning
            .is_some()
            || cli.run.import_params.pruning_params.state_pruning != Some("archive".into())
        {
            warn!("Pruning not supported. Switching to keeping all block bodies and states.");
            cli.run.import_params.pruning_params.blocks_pruning = None;
            cli.run.import_params.pruning_params.state_pruning = Some("archive".into());
        }
    } else {
        warn!("Pruning not supported, but flag experimental_pruning was turned on. Usage of this flag can lead to misbehaviour, which can be punished.");
    }

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
                Ok((cmd.run(client, backend, None), task_manager))
            })
        }
        #[cfg(feature = "try-runtime")]
        Some(Subcommand::TryRuntime(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                let task_manager =
                    sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                        .map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;

                Ok((cmd.run::<Block, ExecutorDispatch>(config), task_manager))
            })
        }
        #[cfg(not(feature = "try-runtime"))]
        Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
        You can enable it with `--features try-runtime`."
            .into()),
        None => {
            let runner = cli.create_runner(&cli.run)?;
            let aleph_cli_config = cli.aleph;
            runner.run_node_until_exit(|config| async move {
                match config.role {
                    Role::Authority => {
                        new_authority(config, aleph_cli_config).map_err(sc_cli::Error::Service)
                    }
                    Role::Full => {
                        new_full(config, aleph_cli_config).map_err(sc_cli::Error::Service)
                    }
                }
            })
        }
    }
}
