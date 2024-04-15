use std::path::PathBuf;

use sc_cli::clap::{self, Args};
use sc_service::BasePath;

pub const DEFAULT_BACKUP_FOLDER: &str = "backup-stash";

#[derive(Debug, Args)]
pub struct SharedParams {
    /// For `bootstrap-node` and `purge-chain` it works with this directory as base.
    /// For `bootstrap-chain` the base path is appended with an account id for each node.
    #[arg(long, short = 'd', value_name = "PATH")]
    base_path: PathBuf,

    /// Specify filename to write node private p2p keys to
    /// Resulting keys will be stored at: base_path/account_id/node_key_file for each node
    #[arg(long, default_value = "p2p_secret")]
    node_key_file: String,

    /// Directory under which AlephBFT backup is stored
    #[arg(long, default_value = DEFAULT_BACKUP_FOLDER)]
    backup_dir: String,
}

impl SharedParams {
    pub fn base_path(&self) -> BasePath {
        BasePath::new(&self.base_path)
    }

    pub fn node_key_file(&self) -> &str {
        &self.node_key_file
    }

    pub fn backup_dir(&self) -> &str {
        &self.backup_dir
    }
}
