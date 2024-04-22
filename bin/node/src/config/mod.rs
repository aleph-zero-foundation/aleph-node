use crate::Cli;

mod pruning;
mod sync;

use pruning::PruningConfigValidator;
use sync::SyncConfigValidator;

/// Validate and modify the configuration to make it conform to our assumptions.
pub struct Validator {
    pruning: PruningConfigValidator,
    sync: SyncConfigValidator,
}

impl Validator {
    /// Modifies the settings.
    pub fn process(cli: &mut Cli) -> Self {
        Validator {
            pruning: PruningConfigValidator::process(cli),
            sync: SyncConfigValidator::process(cli),
        }
    }

    /// Warns the user about the modified settings.
    pub fn report(self) {
        let Validator { pruning, sync } = self;
        pruning.report();
        sync.report();
    }
}
