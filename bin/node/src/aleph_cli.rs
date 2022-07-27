use std::path::PathBuf;

use aleph_primitives::DEFAULT_UNIT_CREATION_DELAY;
use clap::{ArgGroup, Parser};
use finality_aleph::UnitCreationDelay;

#[derive(Debug, Parser, Clone)]
#[clap(group(ArgGroup::new("backup")))]
pub struct AlephCli {
    #[clap(long)]
    unit_creation_delay: Option<u64>,

    /// Turn off backups, at the cost of limiting crash recoverability.
    ///
    /// If backups are turned off and the node crashes, it most likely will not be able to continue
    /// the session during which it crashed. It will join AlephBFT consensus in the next session.
    #[clap(long, group = "backup")]
    no_backup: bool,
    /// The path to save backups to.
    ///
    /// Backups created by the node are saved under this path. When restarted after a crash,
    /// the backups will be used to recover the node's state, helping prevent auto-forks. The layout
    /// of the directory is unspecified. This flag must be specified unless backups are turned off
    /// with `--no-backup`, but note that that limits crash recoverability.
    #[clap(long, value_name = "PATH", group = "backup")]
    backup_path: Option<PathBuf>,
}

impl AlephCli {
    pub fn unit_creation_delay(&self) -> UnitCreationDelay {
        UnitCreationDelay(
            self.unit_creation_delay
                .unwrap_or(DEFAULT_UNIT_CREATION_DELAY),
        )
    }

    pub fn backup_path(&self) -> Option<PathBuf> {
        self.backup_path.clone()
    }

    pub fn no_backup(&self) -> bool {
        self.no_backup
    }
}
