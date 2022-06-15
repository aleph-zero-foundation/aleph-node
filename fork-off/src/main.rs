use clap::Parser;
use env_logger::Env;
use log::info;
use serde_json::Value;

use crate::{
    account_setting::{account_info_from_free, apply_account_setting, AccountSetting},
    chainspec_combining::combine_states,
    config::Config,
    fetching::StateFetcher,
    fsio::{
        file_content, read_json_from_file, read_snapshot_from_file, save_snapshot_to_file,
        write_to_file,
    },
    types::Storage,
};

mod account_setting;
mod chainspec_combining;
mod config;
mod fetching;
mod fsio;
mod jsonrpc_client;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let config = Config::parse();
    info!(target: "fork-off", "{:?}", config);

    let Config {
        ws_rpc_endpoint,
        initial_spec_path,
        snapshot_path,
        combined_spec_path,
        use_snapshot_file,
        storage_keep_state,
        max_requests,
        accounts_path,
        balances,
        at_block,
    } = config;

    let mut initial_spec: Value = read_json_from_file(initial_spec_path);
    assert_ne!(
        initial_spec["genesis"]["raw"],
        Value::Null,
        "The initial provided chainspec must be raw! Make sure you use --raw when generating it."
    );

    if !use_snapshot_file {
        let fetcher = StateFetcher::new(ws_rpc_endpoint).await;
        let state = fetcher.get_full_state(at_block, max_requests).await;
        save_snapshot_to_file(state, snapshot_path.clone());
    }
    let state = read_snapshot_from_file(snapshot_path);

    let initial_state: Storage =
        serde_json::from_value(initial_spec["genesis"]["raw"]["top"].take())
            .expect("Deserialization of state from given chainspec file failed");

    let state = combine_states(state, initial_state, storage_keep_state);

    let account_setting: AccountSetting = match accounts_path {
        Some(accounts_path) => serde_json::from_str(&file_content(accounts_path))
            .expect("Deserialization of balance configuration file failed"),
        None => match balances {
            Some(balances) => {
                let info = balances
                    .into_iter()
                    .map(|(account, free)| (account, account_info_from_free(free)));
                AccountSetting::from_iter(info)
            }
            None => AccountSetting::new(),
        },
    };
    let state = apply_account_setting(state, account_setting);

    let json_state = serde_json::to_value(state).expect("Failed to convert a storage map to json");
    initial_spec["genesis"]["raw"]["top"] = json_state;
    let new_spec = serde_json::to_vec_pretty(&initial_spec)?;

    info!(target: "fork-off", "Writing new chainspec to {}", &combined_spec_path);
    write_to_file(combined_spec_path, &new_spec);

    info!(target: "fork-off", "Done!");
    Ok(())
}
