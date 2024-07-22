use std::{io::Write, path::PathBuf};

use primitives::DEFAULT_BACKUP_FOLDER;
use sc_cli::{
    clap::{self, Parser},
    Error, KeystoreParams,
};
use sc_service::config::BasePath;

use crate::chain_spec::{
    builder::build_chain_spec_json,
    cli::ChainSpecParams,
    keystore::{
        create_abft_backup_dir, create_account_session_keys, create_p2p_key, open_keystore,
    },
    AlephNodeChainSpec,
};

/// This command generates session keys and libp2p key for all input accounts.
/// Those keys are written to the keystore in a given base path. It also generates a chainspec.
#[derive(Debug, Parser)]
pub struct BootstrapChainCmd {
    /// Force raw genesis storage output.
    #[arg(long = "raw")]
    pub raw: bool,

    /// Base path `base-path` is a directory that contains keystore and db
    /// `base-path` has a account id directory in it, and that directory stores keystore and db
    #[arg(long, short = 'd', value_name = "PATH")]
    base_path: PathBuf,

    /// Specify filename to write node private p2p keys to
    /// Resulting keys will be stored at: base_path/account_id/node_key_file for each node
    #[arg(long, default_value = "p2p_secret")]
    node_key_file: String,

    /// Directory under which AlephBFT backup is stored
    #[arg(long, default_value = DEFAULT_BACKUP_FOLDER)]
    backup_dir: String,

    #[clap(flatten)]
    pub chain_spec_params: ChainSpecParams,

    #[clap(flatten)]
    pub keystore_params: KeystoreParams,
}

impl BootstrapChainCmd {
    pub fn run(&self) -> Result<(), Error> {
        let base_path: BasePath = self.base_path.clone().into();
        let backup_dir = &self.backup_dir;
        let node_key_file = &self.node_key_file;
        let chain_id = self.chain_spec_params.chain_id();
        let account_ids = self.chain_spec_params.account_ids();
        let mut authorities = self.chain_spec_params.authorities_account_ids();
        if authorities.is_empty() {
            if account_ids.is_empty() {
                return Err(
                    ("Both --account-ids and --authorities-account-ids are empty. \
                              Please specify at least one account.")
                        .into(),
                );
            }
            authorities = account_ids.clone();
        } else if !authorities
            .iter()
            .all(|authority| account_ids.contains(authority))
        {
            return Err("--authorities-account-ids must be a subset of --accounts-ids.".into());
        }

        let account_session_keys = self
            .chain_spec_params
            .clone()
            .account_ids()
            .into_iter()
            .map(|account_id| {
                let account_base_path: BasePath =
                    base_path.path().join(account_id.to_string()).into();
                create_abft_backup_dir(account_base_path.path(), backup_dir);
                let keystore = open_keystore(&self.keystore_params, chain_id, &account_base_path);
                let session_keys = create_account_session_keys(&keystore, account_id);
                let node_key_path = account_base_path.path().join(node_key_file);
                create_p2p_key(node_key_path.as_path());
                session_keys
            });
        let only_authorities = account_session_keys
            .filter(|account_session_key| authorities.contains(&account_session_key.account_id))
            .collect();

        let chain_spec =
            build_chain_spec_json(self.raw, &self.chain_spec_params, only_authorities)?;
        if std::io::stdout().write_all(chain_spec.as_bytes()).is_err() {
            let _ = std::io::stderr().write_all(b"Error writing to stdout\n");
        }

        Ok(())
    }
}

/// Command used to go from chainspec to the raw chainspec format
#[derive(Debug, Parser)]
pub struct ConvertChainspecToRawCmd {
    /// Specify path to JSON chainspec
    #[arg(long)]
    pub chain: PathBuf,
}

impl ConvertChainspecToRawCmd {
    pub fn run(&self) -> Result<(), Error> {
        let spec = AlephNodeChainSpec::from_json_file(self.chain.to_owned())
            .expect("Cannot read chainspec");

        let raw_chainspec = sc_service::chain_ops::build_spec(&spec, true)?;
        if std::io::stdout()
            .write_all(raw_chainspec.as_bytes())
            .is_err()
        {
            let _ = std::io::stderr().write_all(b"Error writing to stdout\n");
        }

        Ok(())
    }
}
