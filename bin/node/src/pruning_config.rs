use log::warn;
use primitives::DEFAULT_SESSION_PERIOD;
use sc_cli::{Database, DatabasePruningMode, PruningParams};
use static_assertions::const_assert;

use crate::Cli;

/// Anything greater than 1800, which is two normal length sessions, should be enough.
/// We need to be able to read back so many states to retrieve the list of authorities for a session.
const MINIMAL_STATE_PRUNING: u32 = 2048;
const_assert!(MINIMAL_STATE_PRUNING >= 2 * DEFAULT_SESSION_PERIOD);

const DEFAULT_STATE_PRUNING: DatabasePruningMode = DatabasePruningMode::Archive;

const DEFAULT_BLOCKS_PRUNING: DatabasePruningMode = DatabasePruningMode::ArchiveCanonical;

const DEFAULT_DATABASE_FOR_PRUNING: sc_cli::Database = Database::ParityDb;

pub struct PruningConfigValidator {
    pruning_enabled: bool,
    overwritten_pruning: bool,
    invalid_state_pruning_setting: Result<(), u32>,
    invalid_blocks_pruning_setting: Result<(), u32>,
    invalid_database_backend: Result<(), ()>,
}

impl PruningConfigValidator {
    fn pruning_changed(params: &PruningParams) -> bool {
        let state_pruning_changed =
            params.state_pruning.unwrap_or(DEFAULT_STATE_PRUNING) != DEFAULT_STATE_PRUNING;
        let blocks_pruning_changed = params.blocks_pruning != DEFAULT_BLOCKS_PRUNING;

        state_pruning_changed || blocks_pruning_changed
    }

    pub fn process(cli: &mut Cli) -> Self {
        let overwritten_pruning = Self::pruning_changed(&cli.run.import_params.pruning_params);
        let pruning_enabled = cli.aleph.enable_pruning();

        let mut result = PruningConfigValidator {
            pruning_enabled,
            overwritten_pruning,
            invalid_state_pruning_setting: Ok(()),
            invalid_blocks_pruning_setting: Ok(()),
            invalid_database_backend: Ok(()),
        };

        if !pruning_enabled {
            // We need to override state pruning to our default (archive), as substrate has 256 by default.
            // 256 does not work with our code.
            cli.run.import_params.pruning_params.state_pruning = Some(DEFAULT_STATE_PRUNING);
            cli.run.import_params.pruning_params.blocks_pruning = DEFAULT_BLOCKS_PRUNING;
            return result;
        }

        result.process_state_pruning(cli);
        result.process_blocks_pruning(cli);
        result.process_database(cli);

        result
    }

    pub fn report(self) {
        if !self.pruning_enabled {
            if self.overwritten_pruning {
                warn!("Pruning not enabled. Switching to keeping all block bodies and states. Please use `--enable-pruning` flag.");
            }
            return;
        }
        if let Err(max_blocks) = self.invalid_state_pruning_setting {
            warn!(
                "State pruning was enabled but the `state-pruning` \
            parameter is smaller than minimal supported value \
            (provided: {}, min: {}). Switching to {}.",
                max_blocks, MINIMAL_STATE_PRUNING, MINIMAL_STATE_PRUNING,
            );
        }
        if let Err(blocks_pruning) = self.invalid_blocks_pruning_setting {
            warn!(
                "Blocks pruning was enabled but the provided value for the `blocks-pruning` parameter is not valid ({blocks_pruning}). \
            Supported values are `archive` and `archive-canonical`. \
            Switching to `archive-canonical`",
            );
        }
        if self.invalid_database_backend.is_err() {
            warn!(
                "Pruning was enabled but the selected database backend \
            is not supported with pruning. Switching to `paritydb`.",
            );
        }
    }

    fn process_database(&mut self, cli: &mut Cli) {
        match cli
            .run
            .import_params
            .database_params
            .database
            .get_or_insert(DEFAULT_DATABASE_FOR_PRUNING)
        {
            Database::ParityDb => {}
            db @ (Database::RocksDb | Database::Auto | Database::ParityDbDeprecated) => {
                self.invalid_database_backend = Err(());
                *db = DEFAULT_DATABASE_FOR_PRUNING;
            }
        }
    }

    fn process_blocks_pruning(&mut self, cli: &mut Cli) {
        match cli.run.import_params.pruning_params.blocks_pruning {
            DatabasePruningMode::Archive | DatabasePruningMode::ArchiveCanonical => {}
            DatabasePruningMode::Custom(blocks_pruning) => {
                self.invalid_blocks_pruning_setting = Err(blocks_pruning);
                cli.run.import_params.pruning_params.blocks_pruning = DEFAULT_BLOCKS_PRUNING;
            }
        }
    }

    fn process_state_pruning(&mut self, cli: &mut Cli) {
        match cli
            .run
            .import_params
            .pruning_params
            .state_pruning
            .get_or_insert(DatabasePruningMode::Custom(MINIMAL_STATE_PRUNING))
        {
            DatabasePruningMode::Archive | DatabasePruningMode::ArchiveCanonical => {}
            DatabasePruningMode::Custom(max_blocks) => {
                if *max_blocks < MINIMAL_STATE_PRUNING {
                    self.invalid_state_pruning_setting = Err(*max_blocks);
                    *max_blocks = MINIMAL_STATE_PRUNING;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use sc_service::{BlocksPruning, PruningMode};

    use super::PruningParams;
    use crate::pruning_config::{DEFAULT_BLOCKS_PRUNING, DEFAULT_STATE_PRUNING};

    #[test]
    fn pruning_sanity_check() {
        let pruning_params = PruningParams {
            state_pruning: Some(DEFAULT_STATE_PRUNING),
            blocks_pruning: DEFAULT_BLOCKS_PRUNING,
        };

        assert_eq!(
            pruning_params.blocks_pruning().unwrap(),
            BlocksPruning::KeepFinalized
        );

        assert_eq!(
            pruning_params.state_pruning().unwrap().unwrap(),
            PruningMode::ArchiveAll
        );
    }
}
