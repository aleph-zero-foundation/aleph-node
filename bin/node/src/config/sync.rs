use log::warn;
use sc_cli::arg_enums::SyncMode;

use crate::Cli;

/// Modifies the sync config to ensure only full sync is used.
pub struct SyncConfigValidator {
    overwritten: Option<SyncMode>,
}

impl SyncConfigValidator {
    /// Modifies the settings.
    pub fn process(cli: &mut Cli) -> Self {
        let overwritten = match cli.run.network_params.sync {
            SyncMode::Full => None,
            mode => Some(mode),
        };
        cli.run.network_params.sync = SyncMode::Full;
        SyncConfigValidator { overwritten }
    }

    /// Warns the user if they attempted to use a sync setting other than full.
    pub fn report(self) {
        if let Some(mode) = self.overwritten {
            warn!(
                "Only full sync mode is supported, ignoring request for {:?} mode.",
                mode
            );
        }
    }
}
