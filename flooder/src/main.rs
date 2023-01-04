use std::time::Duration;

use aleph_client::{
    account_from_keypair, pallets::balances::BalanceUserApi, raw_keypair_from_string, AccountId,
    KeyPair, SignedConnection, SignedConnectionApi, TxStatus,
};
use clap::Parser;
use config::Config;
use futures::future::join_all;
use log::info;
use subxt::ext::sp_core::{sr25519, Pair};
use tokio::{time, time::sleep};

mod config;

async fn flood(
    connections: Vec<SignedConnection>,
    dest: AccountId,
    transfer_amount: u128,
    tx_count: u64,
    rate_limiting: Option<(u64, u64)>,
    status: TxStatus,
) -> anyhow::Result<()> {
    let handles: Vec<_> = connections
        .into_iter()
        .map(|conn| {
            let dest = dest.clone();
            tokio::spawn(async move {
                let (time, (tx_count, round_count)) = match rate_limiting {
                    Some((tx_in_interval, interval_secs)) => (
                        interval_secs,
                        (tx_in_interval, (tx_count + tx_in_interval) / tx_in_interval),
                    ),
                    _ => (0, (tx_count, 1)),
                };

                for i in 0..round_count {
                    info!("starting round #{}", i);
                    let start = time::Instant::now();

                    info!("sending #{} transactions", tx_count);
                    for _ in 0..tx_count {
                        conn.transfer(dest.clone(), transfer_amount, status)
                            .await
                            .unwrap();
                    }

                    let dur = time::Instant::now().saturating_duration_since(start);

                    let left_duration = Duration::from_secs(time).saturating_sub(dur);

                    info!("sleeping for {}ms", left_duration.as_millis());
                    sleep(left_duration).await;
                }
            })
        })
        .collect();

    join_all(handles).await;

    Ok(())
}

async fn initialize_n_accounts<F: Fn(u32) -> String>(
    connection: SignedConnection,
    n: u32,
    node: F,
    account_balance: u128,
    skip: bool,
) -> Vec<SignedConnection> {
    let mut connections = vec![];
    for i in 0..n {
        let seed = i.to_string();
        let signer = KeyPair::new(raw_keypair_from_string(&("//".to_string() + &seed)));
        connections.push(SignedConnection::new(&node(i), signer).await);
    }

    if skip {
        return connections;
    }
    for conn in connections.iter() {
        connection
            .transfer(
                conn.account_id().clone(),
                account_balance,
                TxStatus::Submitted,
            )
            .await
            .unwrap();
    }

    connection
        .transfer(connection.account_id().clone(), 1, TxStatus::Finalized)
        .await
        .unwrap();

    connections
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let config: Config = Config::parse();
    info!("Starting benchmark with config {:#?}", &config);

    // we want to fail fast in case seed or phrase are incorrect
    if !config.skip_initialization && config.phrase.is_none() && config.seed.is_none() {
        panic!("Needs --phrase or --seed");
    }

    let tx_count = config.transactions;
    let accounts = (tx_count + 1) / 100;
    let tx_per_account = 100;
    let rate_limiting = match (config.transactions_in_interval, config.interval_secs) {
        (Some(tii), Some(is)) => Some((tii, is)),
        (None, None) => None,
        _ => panic!("--transactions-in-interval needs to be specified with --interval-secs"),
    };
    let tx_status = match config.wait_for_ready {
        true => TxStatus::InBlock,
        false => TxStatus::Submitted,
    };

    let account = match &config.phrase {
        Some(phrase) => {
            sr25519::Pair::from_phrase(&config::read_phrase(phrase.clone()), None)
                .unwrap()
                .0
        }
        None => sr25519::Pair::from_string(
            config.seed.as_ref().expect("We checked it is not None."),
            None,
        )
        .unwrap(),
    };
    let main_connection =
        SignedConnection::new(&config.nodes[0], KeyPair::new(account.clone())).await;

    let nodes = config.nodes.clone();

    let connections = initialize_n_accounts(
        main_connection,
        accounts as u32,
        |i| nodes[i as usize % nodes.len()].clone(),
        tx_per_account + tx_per_account * 10_000,
        config.skip_initialization,
    )
    .await;

    flood(
        connections,
        account_from_keypair(&account),
        1,
        tx_per_account as u64,
        rate_limiting,
        tx_status,
    )
    .await?;

    Ok(())
}
