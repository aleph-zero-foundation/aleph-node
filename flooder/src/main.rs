use core::cmp::min;

use aleph_client::{
    pallets::{
        author::AuthorRpc, balances::BalanceUserApi, system::SystemApi, timestamp::TimestampApi,
    },
    raw_keypair_from_string,
    utility::BlocksApi,
    AccountId, Balance, KeyPair, Nonce, SignedConnection, SignedConnectionApi,
    SignedConnectionApiExt, TxStatus, TOKEN,
};
use clap::Parser;
use config::Config;
use futures::future::join_all;
use log::{debug, info};
use subxt::{
    config::{extrinsic_params::BaseExtrinsicParamsBuilder, substrate::Era},
    ext::sp_core::{sr25519, Pair},
    tx::TxPayload,
    utils::{MultiAddress, Static},
};
use tokio::time::{interval, Duration, Instant};

mod config;

fn transfer_keep_alive(dest: AccountId, amount: Balance) -> impl TxPayload + Send + Sync {
    aleph_client::api::tx()
        .balances()
        .transfer_keep_alive(MultiAddress::Id(Static(dest)), amount)
}

fn transfer_all(dest: AccountId, keep_alive: bool) -> impl TxPayload + Send + Sync {
    aleph_client::api::tx()
        .balances()
        .transfer_all(MultiAddress::Id(Static(dest)), keep_alive)
}

struct Schedule {
    pub intervals: u64,
    pub interval_duration: Duration,
    pub transactions_in_interval: u64,
}

async fn flood(
    connections: Vec<SignedConnection>,
    dest: AccountId,
    transfer_amount: Balance,
    schedule: Schedule,
    status: TxStatus,
    pool_limit: u64,
) -> anyhow::Result<Vec<(SignedConnection, Nonce)>> {
    let start = Instant::now();
    let total_duration = schedule.interval_duration * (schedule.intervals as u32);

    let start_finalized_hash = connections[0].get_finalized_block_hash().await?;
    let start_finalized_number = connections[0]
        .get_block_number(start_finalized_hash)
        .await?
        .unwrap() as u64;

    // Set mortality roughly to flooding length
    let params = BaseExtrinsicParamsBuilder::default().era(
        Era::mortal(total_duration.as_secs() + 30, start_finalized_number),
        start_finalized_hash,
    );

    let n_connections = connections.len();
    let mut start_nonces = vec![0; n_connections];
    for (conn_id, conn) in connections.iter().enumerate() {
        start_nonces[conn_id] = conn.account_nonce(conn.account_id()).await?;
    }

    let split_per_connections = move |total, conn_id| {
        let mut part = total / n_connections as u64;
        if conn_id < (total as usize) % n_connections {
            part += 1;
        }
        part
    };

    let handles = connections
        .into_iter()
        .enumerate()
        .map(|(conn_id, conn)| {
            let dest = dest.clone();
            let mut nonce = start_nonces[conn_id];
            tokio::spawn(async move {
                let mut interval = interval(schedule.interval_duration);
                let mut overdue_transactions = 0;
                for i in 0..schedule.intervals {
                    interval.tick().await;
                    overdue_transactions += split_per_connections(schedule.transactions_in_interval, conn_id);

                    let pending_in_pool = conn.pending_extrinsics_len().await?;
                    let transactions_to_pool_limit = pool_limit.saturating_sub(pending_in_pool);
                    let my_limit_part = split_per_connections(transactions_to_pool_limit, conn_id);

                    let my_transactions = min(
                        overdue_transactions,
                        my_limit_part,
                    );
                    overdue_transactions -= my_transactions;
                    debug!(
                        "Interval {}, sending {my_transactions} transaction from connection {conn_id}. \
                         In the pool, there are pending {pending_in_pool} transactions. \
                         Overdue transactions: {overdue_transactions}.",
                        i + 1
                    );

                    for _ in 0..my_transactions {
                        conn.sign_with_params(
                            transfer_keep_alive(dest.clone(), transfer_amount),
                            params,
                            nonce,
                        )?
                        .submit(status)
                        .await?;
                        nonce += 1;
                        if Instant::now().saturating_duration_since(start) > total_duration {
                            return anyhow::Ok((nonce, conn));
                        }
                    }
                }
                anyhow::Ok((nonce, conn))
            })
        });

    let mut total_submitted = 0;
    let mut last_error = None;
    let mut res = vec![];
    for (conn_id, result) in join_all(handles).await.into_iter().enumerate() {
        match result? {
            Ok((nonce, conn)) => {
                total_submitted += nonce - start_nonces[conn_id];
                res.push((conn, nonce));
            }
            Err(e) => {
                info!("Sender subtask finished with an error: {e:?}");
                last_error = Some(e);
            }
        }
    }

    let target_transactions = schedule.intervals * schedule.transactions_in_interval;
    info!(
        "Submitted {total_submitted} txns out of {target_transactions} that should be sent ({:.2}%)",
        total_submitted as f64 / target_transactions as f64 * 100.0
    );

    match last_error {
        Some(e) => Err(e),
        None => Ok(res),
    }
}

async fn return_balances(
    connections_and_nonces: &[(SignedConnection, Nonce)],
    dest: AccountId,
) -> anyhow::Result<()> {
    for (conn, nonce) in connections_and_nonces {
        conn.sign_with_params(
            transfer_all(dest.clone(), false),
            Default::default(),
            *nonce,
        )?
        .submit(TxStatus::Submitted)
        .await?;
    }
    debug!("Returned balance back to main account");
    Ok(())
}

async fn initialize_n_accounts<F: Fn(u32) -> String>(
    main_connection: &SignedConnection,
    first_account_in_range: u64,
    n: u32,
    node: F,
    amount: Balance,
    skip: bool,
) -> anyhow::Result<Vec<SignedConnection>> {
    const ACCOUNTS_SEED_PREFIX: &str = "//";
    info!(
        "Initializing accounts, estimated total fee per account: {}",
        amount as f32 / TOKEN as f32
    );
    let mut connections = vec![];
    for i in 0..n {
        let seed = (i as u64 + first_account_in_range).to_string();
        let signer = KeyPair::new(raw_keypair_from_string(
            format!("{ACCOUNTS_SEED_PREFIX}{seed}").as_ref(),
        ));
        connections.push(SignedConnection::new(&node(i), signer).await);
    }

    if skip {
        return Ok(connections);
    }

    let nonce = main_connection
        .account_nonce(main_connection.account_id())
        .await?;
    for (i, conn) in connections.iter().enumerate() {
        let status = if i + 1 == n as usize {
            TxStatus::Finalized
        } else {
            TxStatus::Submitted
        };

        main_connection
            .sign_with_params(
                transfer_keep_alive(conn.account_id().clone(), amount),
                Default::default(),
                nonce + i as Nonce,
            )?
            .submit(status)
            .await?;
    }

    Ok(connections)
}

/// Only a rough estimation, for the worst case where blocks are 75% full
/// (it is a maximum for non-operational transactions).
/// See https://github.com/Cardinal-Cryptography/aleph-node/blob/b6ac239809667b5c6a113c4e3c9ef9216c5b97eb/bin/runtime/src/lib.rs#L267
async fn estimate_avg_fee_per_transaction_in_block(
    main_connection: &SignedConnection,
    schedule: &Schedule,
) -> anyhow::Result<u128> {
    let estimated_blocks = (schedule.intervals * schedule.interval_duration.as_secs()) as u128;
    let fee_estimation_tx = main_connection
        .transfer_keep_alive(main_connection.account_id().clone(), 1, TxStatus::Finalized)
        .await?;
    let starting_fee = main_connection.get_tx_fee(fee_estimation_tx).await?;

    let mut total_fee = 0;
    let mut fee = starting_fee;
    for _ in 0..estimated_blocks {
        total_fee += fee;
        fee = (fee as f64 * 1.0345) as Balance;
        if total_fee > Balance::MAX / 4 {
            return Err(anyhow::anyhow!("Fee estimation overflowed."));
        }
    }
    Ok((total_fee + estimated_blocks - 1) / estimated_blocks)
}

struct FloodStats {
    transactions_per_second: f64,
    transactions_per_block: f64,
    transactions_per_block_stddev: f64,
    block_time: f64,
    block_time_stddev: f64,
}

async fn compute_stats(
    connection: &SignedConnection,
    start_block: u32,
    end_block: u32,
) -> anyhow::Result<FloodStats> {
    let mut xt_counts = vec![];
    let mut block_times = vec![];

    let timestamp = |number| async move {
        anyhow::Ok(
            connection
                .get_timestamp(connection.get_block_hash(number).await?)
                .await
                .unwrap(),
        )
    };

    for number in start_block..=end_block {
        let hash = connection.get_block_hash(number).await?.unwrap();
        let block = connection.connection.as_client().blocks().at(hash).await?;
        xt_counts.push(block.body().await?.extrinsics().len().try_into()?);
        block_times.push(timestamp(number).await? - timestamp(number - 1).await?);
    }

    let total_time_ms = timestamp(end_block).await? - timestamp(start_block - 1).await?;
    let total_xt: u64 = xt_counts.iter().sum();

    Ok(FloodStats {
        transactions_per_second: total_xt as f64 * 1000.0 / total_time_ms as f64,
        transactions_per_block: total_xt as f64 / xt_counts.len() as f64,
        transactions_per_block_stddev: stddev(&xt_counts[..]),
        block_time: total_time_ms as f64 / xt_counts.len() as f64,
        block_time_stddev: stddev(&block_times[..]),
    })
}

fn stddev(values: &[u64]) -> f64 {
    let mean = values.iter().map(|&x| x as f64).sum::<f64>() / values.len() as f64;
    let mean_of_squares =
        values.iter().map(|&x| x as f64 * x as f64).sum::<f64>() / values.len() as f64;
    (mean_of_squares - mean * mean).sqrt()
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::builder().format_timestamp_millis().init();
    let config: Config = Config::parse();
    info!("Starting benchmark with config {:#?}", &config);

    // We want to fail fast in case seed or phrase are incorrect
    if !config.skip_initialization && config.phrase.is_none() && config.seed.is_none() {
        panic!("Needs --phrase or --seed");
    }

    let schedule = Schedule {
        intervals: config.intervals,
        interval_duration: Duration::from_secs(config.interval_duration),
        transactions_in_interval: config.transactions_in_interval,
    };

    let accounts: u32 = (schedule.transactions_in_interval as f64).sqrt() as u32;

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

    let mut avg_fee_per_transaction =
        estimate_avg_fee_per_transaction_in_block(&main_connection, &schedule).await?;
    avg_fee_per_transaction = avg_fee_per_transaction * 5 / 4; // Leave some margin

    let total_fee_per_account = (avg_fee_per_transaction / accounts as u128)
        .saturating_mul(schedule.transactions_in_interval as u128)
        .saturating_mul(schedule.intervals as u128);

    let nodes = config.nodes.clone();
    let connections = initialize_n_accounts(
        &main_connection,
        config.first_account_in_range,
        accounts,
        |i| nodes[i as usize % nodes.len()].clone(),
        total_fee_per_account,
        config.skip_initialization,
    )
    .await?;

    let best_block_pre_flood = main_connection.get_best_block().await.unwrap().unwrap();

    let connections_and_nonces = flood(
        connections,
        main_connection.account_id().clone(),
        1,
        schedule,
        tx_status,
        config.pool_limit,
    )
    .await?;

    if !config.skip_initialization {
        return_balances(
            &connections_and_nonces,
            main_connection.account_id().clone(),
        )
        .await?;
    }

    let end_block = main_connection.get_best_block().await.unwrap().unwrap();
    let start_block = best_block_pre_flood + (end_block - best_block_pre_flood) / 10;
    let stats = compute_stats(&main_connection, start_block, end_block).await?;
    info!("Stats measured for blocks {start_block} to {end_block} inclusive");
    info!(
        "Stats:\nActual transactions per second: {:.2}\nTransactions per block: {:.2} (stddev = {:.2})\nBlock time: {:.2}ms (stddev = {:.2})",
        stats.transactions_per_second,
        stats.transactions_per_block,
        stats.transactions_per_block_stddev,
        stats.block_time,
        stats.block_time_stddev,
    );

    Ok(())
}
