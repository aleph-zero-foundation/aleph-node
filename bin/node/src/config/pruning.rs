use log::warn;
use primitives::DEFAULT_SESSION_PERIOD;
use sc_cli::{Database, DatabasePruningMode, PruningParams};
use static_assertions::const_assert;

use crate::Cli;

/// Anything greater than 900, which is one normal length session, should be enough.
/// We need to be able to read back state from previous session to retrieve the list of authorities for a session.
const MINIMAL_STATE_PRUNING: u32 = 901;
const_assert!(MINIMAL_STATE_PRUNING > DEFAULT_SESSION_PERIOD);

const DEFAULT_STATE_PRUNING: DatabasePruningMode = DatabasePruningMode::Archive;

const DEFAULT_BLOCKS_PRUNING: DatabasePruningMode = DatabasePruningMode::ArchiveCanonical;

/// Max value for the state-pruning after which RocksDB backend complains about its memory consumption.
/// Setting to some greater value can cause out-of-memory errors.
const ROCKSDB_PRUNING_THRESHOLD: u32 = 1000;
const_assert!(MINIMAL_STATE_PRUNING <= ROCKSDB_PRUNING_THRESHOLD);

pub struct PruningConfigValidator {
    pruning_enabled: bool,
    overwritten_pruning: bool,
    invalid_state_pruning_setting: Result<(), u32>,
    invalid_state_pruning_setting_for_rocksdb: Result<(), (u32, Database)>,
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
            invalid_state_pruning_setting_for_rocksdb: Ok(()),
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
        if let Err((max_blocks, database)) = self.invalid_state_pruning_setting_for_rocksdb {
            let mut message = format!("State pruning was enabled but the `state-pruning` \
                                   parameter is greater than maximal value supported by \
                                   the RocksDB database engine (provided: {}, max supported: {}). Switching to {}.",
                                  max_blocks, ROCKSDB_PRUNING_THRESHOLD, ROCKSDB_PRUNING_THRESHOLD).to_string();
            if database == Database::Auto {
                message +=
                    " Database engine was set to `Auto` - please use explicit value for the `--database` cli parameter \
                       if you want to use the `ParityDB` database engine (it supports greater values for the `state-pruning` \
                       parameter), e.g. `--database ParityDB`.";
            }

            warn!("{}", message);
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

    fn process_state_pruning(&mut self, cli: &mut Cli) {
        let database_engine = cli
            .run
            .import_params
            .database_params
            .database
            .unwrap_or(Database::Auto);
        let might_be_rocksdb = matches!(database_engine, Database::RocksDb | Database::Auto);

        match cli
            .run
            .import_params
            .pruning_params
            .state_pruning
            .get_or_insert(DatabasePruningMode::Custom(MINIMAL_STATE_PRUNING))
        {
            DatabasePruningMode::Archive | DatabasePruningMode::ArchiveCanonical => {}
            DatabasePruningMode::Custom(max_blocks) => {
                if might_be_rocksdb && *max_blocks > ROCKSDB_PRUNING_THRESHOLD {
                    self.invalid_state_pruning_setting_for_rocksdb =
                        Err((*max_blocks, database_engine));
                    *max_blocks = ROCKSDB_PRUNING_THRESHOLD;
                }
                if *max_blocks < MINIMAL_STATE_PRUNING {
                    self.invalid_state_pruning_setting = Err(*max_blocks);
                    *max_blocks = MINIMAL_STATE_PRUNING;
                }
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
}

#[cfg(test)]
mod tests {
    use sc_service::{BlocksPruning, PruningMode};

    use super::PruningParams;
    use crate::config::pruning::{DEFAULT_BLOCKS_PRUNING, DEFAULT_STATE_PRUNING};

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
