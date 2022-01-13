mod config;

use clap::Parser;
use codec::{Compact, Decode, Encode};
use common::{create_custom_connection, WsRpcClient};
use config::Config;
use hdrhistogram::Histogram as HdrHistogram;
use log::{debug, info};
use rayon::prelude::*;
use sp_core::{sr25519, Pair};
use sp_runtime::{generic, traits::BlakeTwo256, MultiAddress, OpaqueExtrinsic};
use std::convert::TryInto;
use std::{
    io::{Read, Write},
    iter::{once, repeat},
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use substrate_api_client::{
    compose_call, compose_extrinsic_offline, AccountId, Api, GenericAddress, UncheckedExtrinsicV4,
    XtStatus,
};

type TransferTransaction =
    UncheckedExtrinsicV4<([u8; 2], MultiAddress<AccountId, ()>, codec::Compact<u128>)>;
type BlockNumber = u32;
type Header = generic::Header<BlockNumber, BlakeTwo256>;
type Block = generic::Block<Header, OpaqueExtrinsic>;
type Nonce = u32;

fn main() -> Result<(), anyhow::Error> {
    let time_stats = Instant::now();

    env_logger::init();
    let config: Config = Config::parse();
    info!("Starting benchmark with config {:#?}", &config);

    // we want to fail fast in case seed or phrase are incorrect
    if !config.skip_initialization && config.phrase.is_none() && config.seed.is_none() {
        panic!("Needs --phrase or --seed");
    }
    let rate_limiting = match (config.transactions_in_interval, config.interval_secs) {
        (Some(tii), Some(is)) => Some((tii, is)),
        (None, None) => None,
        _ => panic!("--transactions-in-interval needs to be specified with --interval-secs"),
    };
    let tx_status = match config.wait_for_ready {
        true => XtStatus::Ready,
        false => XtStatus::SubmitOnly,
    };
    info!(
        "initializing thread-pool: {}ms",
        time_stats.elapsed().as_millis()
    );
    let mut thread_pool_builder = rayon::ThreadPoolBuilder::new();
    if let Some(threads) = config.threads {
        let threads = threads.try_into().expect("`threads` within usize range");
        thread_pool_builder = thread_pool_builder.num_threads(threads);
    }
    let thread_pool = thread_pool_builder
        .build()
        .expect("thread-pool should be created");

    info!(
        "thread-pool initialized: {}ms",
        time_stats.elapsed().as_millis()
    );
    let threads = thread_pool.current_num_threads();
    info!("thread-pool reported {} threads", threads);
    // each thread should have its own connection
    info!(
        "creating connection-pool: {}ms",
        time_stats.elapsed().as_millis()
    );
    let pool = create_connection_pool(&config.nodes, threads);
    info!(
        "connection-pool created: {}ms",
        time_stats.elapsed().as_millis()
    );
    let connection = pool.get(0).unwrap().get(0).unwrap().clone();

    info!(
        "preparing transactions: {}ms",
        time_stats.elapsed().as_millis()
    );
    let txs = thread_pool.install(|| prepare_txs(&config, &connection));
    info!(
        "transactions prepared: {}ms",
        time_stats.elapsed().as_millis()
    );

    let histogram = Arc::new(Mutex::new(
        HdrHistogram::<u64>::new_with_bounds(1, u64::MAX, 3).unwrap(),
    ));

    let (transactions_in_interval, interval_duration) = rate_limiting.map_or(
        (txs.len(), Duration::from_secs(0)),
        |(transactions_in_interval, secs)| {
            (
                transactions_in_interval
                    .try_into()
                    .expect("`transactions_in_interval` should be within usize range"),
                Duration::from_secs(secs),
            )
        },
    );

    let pool_getter = |thread_id: ThreadId, tx_ix: usize| {
        pool.get(thread_id % pool.len())
            .and_then(|threads_pool| threads_pool.get(tx_ix % threads_pool.len()))
            .unwrap()
    };

    info!("flooding: {}ms", time_stats.elapsed().as_millis());
    let tick = Instant::now();

    flood(
        pool_getter,
        txs,
        tx_status,
        &histogram,
        transactions_in_interval,
        interval_duration,
        &thread_pool,
    );

    let tock = tick.elapsed().as_millis();
    let histogram = histogram.lock().unwrap();

    println!(
        "Summary:\n\
    TransferTransactions sent: {}\n\
    Total time:        {} ms\n\
    Slowest tx:        {} ms\n\
    Fastest tx:        {} ms\n\
    Average:           {:.1} ms\n\
    Throughput:        {:.1} tx/s",
        histogram.len(),
        tock,
        histogram.max(),
        histogram.min(),
        histogram.mean(),
        1000.0 * histogram.len() as f64 / tock as f64
    );

    Ok(())
}

type ThreadId = usize;

fn flood<'a>(
    pool: impl Fn(ThreadId, usize) -> &'a Api<sr25519::Pair, WsRpcClient> + Sync,
    txs: Vec<TransferTransaction>,
    status: XtStatus,
    histogram: &Arc<Mutex<HdrHistogram<u64>>>,
    transactions_in_interval: usize,
    interval_duration: Duration,
    thread_pool: &rayon::ThreadPool,
) {
    let mut current_start_ix = 0;
    thread_pool.install(|| {
        txs
            .chunks(transactions_in_interval)
            .enumerate()
            .for_each(|(interval_idx, interval)| {
                let start = Instant::now();
                info!("Starting {} interval", interval_idx);

                let interval_len = interval.len();

                interval.
                    into_par_iter()
                    .enumerate()
                    .map(|( tx_ix, tx )| (tx_ix + current_start_ix, tx))
                    .for_each(|(tx_ix, tx)| {

                    const DEFAULT_THREAD_ID: usize = 0;
                    let thread_id = thread_pool.current_thread_index().unwrap_or(DEFAULT_THREAD_ID);
                    send_tx(
                        pool(thread_id, tx_ix),
                        &tx,
                        status,
                        Arc::clone(histogram),
                    );
                });

                let exec_time = start.elapsed();

                if let Some(remaining_time) = interval_duration.checked_sub(exec_time) {
                    debug!("Sleeping for {}ms", remaining_time.as_millis());
                    thread::sleep(remaining_time);
                } else {
                    debug!(
                        "Execution for interval {} was slower than desired the target {}ms, was {}ms",
                        interval_idx,
                        interval_duration.as_millis(),
                        exec_time.as_millis()
                    );
                }
                current_start_ix += interval_len;
            });
    });
}

fn estimate_tx_fee(connection: &Api<sr25519::Pair, WsRpcClient>, tx: &TransferTransaction) -> u128 {
    let block = connection.get_block::<Block>(None).unwrap().unwrap();
    let block_hash = block.header.hash();
    let fee = connection
        .get_fee_details(&tx.hex_encode(), Some(block_hash))
        .unwrap()
        .unwrap();

    let inclusion_fee = fee.inclusion_fee.unwrap();

    fee.tip + inclusion_fee.base_fee + inclusion_fee.len_fee + inclusion_fee.adjusted_weight_fee
}

fn load_transactions(file_path: impl AsRef<Path>) -> Vec<TransferTransaction> {
    let zipfile = std::fs::File::open(file_path).expect("Missing file with txs");
    let mut archive = zip::ZipArchive::new(zipfile).expect("Zipfile is not properly created");
    assert!(archive.len() == 1, "There should be one file with txs");

    let mut file = archive.by_index(0).unwrap();
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes).expect("buffer overflow");

    Vec::<TransferTransaction>::decode(&mut &bytes[..]).expect("Error while decoding txs")
}

fn prepare_txs(
    config: &Config,
    connection: &Api<sr25519::Pair, WsRpcClient>,
) -> Vec<TransferTransaction> {
    match config.load_txs {
        true => {
            let path = match &config.tx_store_path {
                Some(path) => path,
                None => panic!("tx_store_path is not set"),
            };
            load_transactions(path)
        }
        false => generate_txs(config, connection),
    }
}

fn generate_txs(
    config: &Config,
    connection: &Api<sr25519::Pair, WsRpcClient>,
) -> Vec<TransferTransaction> {
    let tx_store_path = match (config.store_txs, &config.tx_store_path) {
        (true, None) => panic!("tx_store_path is not set"),
        (true, path) => path,
        (false, _) => &None,
    };
    let first_account_in_range = config.first_account_in_range;
    let total_users = config.transactions;
    let transfer_amount = 1u128;
    let initialize_accounts_flag = !config.skip_initialization;

    let accounts = (first_account_in_range..first_account_in_range + total_users)
        .into_par_iter()
        .map(derive_user_account)
        .collect();

    if initialize_accounts_flag {
        initialize_accounts(config, connection, transfer_amount, &accounts);
    }

    let nonces: Vec<_> = match config.download_nonces {
        false => repeat(0).take(accounts.len()).collect(),
        true => accounts
            .par_iter()
            .map(|account| get_nonce(&connection, &AccountId::from(account.public())))
            .collect(),
    };
    let receiver = accounts
        .first()
        .expect("we should have some accounts available for this test, but the list is empty")
        .clone();
    let txs: Vec<_> = sign_transactions(
        &connection,
        receiver,
        accounts.into_par_iter().zip(nonces),
        transfer_amount,
    )
    .collect();

    if let Some(tx_store_path) = tx_store_path {
        zip_and_store_txs(&txs, tx_store_path);
    }

    txs
}

fn initialize_accounts(
    config: &Config,
    connection: &Api<sr25519::Pair, WsRpcClient>,
    transfer_amount: u128,
    accounts: &Vec<sr25519::Pair>,
) {
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
    let source_account_id = AccountId::from(account.public());
    let source_account_nonce = get_nonce(&connection, &source_account_id);
    let total_amount =
        estimate_amount(&connection, &account, source_account_nonce, transfer_amount);

    assert!(
        get_funds(&connection, &source_account_id)
            .ge(&(total_amount * u128::from(config.transactions))),
        "Account is too poor"
    );

    initialize_accounts_on_chain(
        &connection,
        &account,
        source_account_nonce,
        accounts,
        total_amount,
    );
    debug!("all accounts have received funds");
}

fn sign_tx(
    connection: &Api<sr25519::Pair, WsRpcClient>,
    signer: &sr25519::Pair,
    nonce: Nonce,
    to: &AccountId,
    amount: u128,
) -> TransferTransaction {
    let call = compose_call!(
        connection.metadata,
        "Balances",
        "transfer",
        GenericAddress::Id(to.clone()),
        Compact(amount)
    );

    compose_extrinsic_offline!(
        signer,
        call,
        nonce,
        Era::Immortal,
        connection.genesis_hash,
        connection.genesis_hash,
        connection.runtime_version.spec_version,
        connection.runtime_version.transaction_version
    )
}

/// prepares payload for flooding
fn sign_transactions<'a>(
    connection: &'a Api<sr25519::Pair, WsRpcClient>,
    account: sr25519::Pair,
    users_and_nonces: impl IntoParallelIterator<Item = (sr25519::Pair, u32)> + 'a,
    transfer_amount: u128,
) -> impl ParallelIterator<Item = TransferTransaction> + 'a {
    let to = AccountId::from(account.public());
    // NOTE : assumes one tx per derived user account
    // but we could create less accounts and send them round robin fashion
    // (will need to seed them with more funds as well, tx_per_account times more to be exact)
    users_and_nonces
        .into_par_iter()
        .map(move |(from, nonce)| sign_tx(connection, &from, nonce, &to, transfer_amount))
}

fn estimate_amount(
    connection: &Api<sr25519::Pair, WsRpcClient>,
    account: &sr25519::Pair,
    account_nonce: u32,
    transfer_amount: u128,
) -> u128 {
    let existential_deposit = connection.get_existential_deposit().unwrap();
    // start with a heuristic tx fee
    let total_amount = existential_deposit + (transfer_amount + 375_000_000);

    let tx = sign_tx(
        connection,
        account,
        account_nonce,
        &AccountId::from(account.public()),
        total_amount,
    );

    // estimate fees
    let tx_fee = estimate_tx_fee(connection, &tx);
    info!("Estimated transfer tx fee {}", tx_fee);
    // adjust with estimated tx fee
    existential_deposit + (transfer_amount + tx_fee)
}

fn initialize_accounts_on_chain(
    connection: &Api<sr25519::Pair, WsRpcClient>,
    source_account: &sr25519::Pair,
    mut source_account_nonce: u32,
    accounts: &[sr25519::Pair],
    total_amount: u128,
) {
    // NOTE: use a new connection for the last transaction.
    // This is a workaround for issues with accounts initialization using our hacky
    // single-threaded WsRpcClient. It seems like `subscriptions`
    // for all `wait-until-Ready` transactions should be cancelled,
    // otherwise they are producing a lot of noise, which can be incorrectly
    // interpreted by the `substrate-api-client` library. Creating a separate
    // connection for the last `wait-until-Finalized` transaction should `partially`
    // fix this issue.
    let conn_copy = connection.clone();
    let connections = repeat(connection)
        .take(accounts.len() - 1)
        .chain(once(&conn_copy));
    // ensure all txs are finalized by waiting for the last one sent
    let status = repeat(XtStatus::Ready)
        .take(accounts.len() - 1)
        .chain(once(XtStatus::Finalized));
    for ((derived, status), connection) in accounts.iter().zip(status).zip(connections) {
        source_account_nonce = initialize_account(
            connection,
            source_account,
            source_account_nonce,
            derived,
            total_amount,
            status,
        );
    }
}

fn initialize_account(
    connection: &Api<sr25519::Pair, WsRpcClient>,
    account: &sr25519::Pair,
    account_nonce: Nonce,
    derived: &sr25519::Pair,
    total_amount: u128,
    status: XtStatus,
) -> Nonce {
    let tx = sign_tx(
        connection,
        account,
        account_nonce,
        &AccountId::from(derived.public()),
        total_amount,
    );

    info!("sending funds to account {}", &derived.public());

    let hash = connection
        .send_extrinsic(tx.hex_encode(), status)
        .expect("Could not send transaction");

    info!(
        "account {} will receive funds, tx hash {:?}",
        &derived.public(),
        hash
    );

    account_nonce + 1
}

fn derive_user_account(seed: u64) -> sr25519::Pair {
    debug!("deriving an account from seed={}", seed);
    let seed = seed.to_string();
    sr25519::Pair::from_string(&("//".to_string() + &seed), None).unwrap()
}

fn send_tx<Call>(
    connection: &Api<sr25519::Pair, WsRpcClient>,
    tx: &UncheckedExtrinsicV4<Call>,
    status: XtStatus,
    histogram: Arc<Mutex<HdrHistogram<u64>>>,
) where
    Call: Encode,
{
    let start_time = Instant::now();

    connection
        .send_extrinsic(tx.hex_encode(), status)
        .expect("Could not send transaction");

    let elapsed_time = start_time.elapsed().as_millis();

    let mut hist = histogram.lock().unwrap();
    *hist += elapsed_time as u64;
}

fn create_connection_pool(
    nodes: &[String],
    threads: usize,
) -> Vec<Vec<Api<sr25519::Pair, WsRpcClient>>> {
    repeat(nodes)
        .cycle()
        .take(threads)
        .map(|urls| {
            urls.iter()
                .map(|url| {
                    create_custom_connection(url).expect("it should return initialized connection")
                })
                .collect()
        })
        .collect()
}

fn get_nonce(connection: &Api<sr25519::Pair, WsRpcClient>, account: &AccountId) -> u32 {
    connection
        .get_account_info(account)
        .map(|acc_opt| acc_opt.map_or_else(|| 0, |acc| acc.nonce))
        .expect("retrieved nonce's value")
}

fn get_funds(connection: &Api<sr25519::Pair, WsRpcClient>, account: &AccountId) -> u128 {
    match connection.get_account_data(account).unwrap() {
        Some(data) => data.free,
        None => 0,
    }
}

fn zip_and_store_txs(txs: &[TransferTransaction], path: impl AsRef<Path>) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("tx_store", options)
        .expect("Failed to initialize accounts");
    zip.write_all(&txs.encode())
        .expect("Failed to store encoded bytes");
    zip.finish().expect("Failed to zip the encoded txs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore] // requires access to a chain
    #[test]
    fn write_read_txs() {
        env_logger::init();

        let url = "127.0.0.1:9944".to_string();
        let mut config = Config {
            nodes: vec![url.clone()],
            transactions: 313,
            phrase: None,
            seed: None,
            skip_initialization: true,
            first_account_in_range: 0,
            load_txs: false,
            tx_store_path: Some("/tmp/tx_store".to_string()),
            threads: None,
            download_nonces: false,
            wait_for_ready: false,
            store_txs: true,
            interval_secs: None,
            transactions_in_interval: None,
        };
        let conn = create_custom_connection(&url).unwrap();

        let txs_gen = prepare_txs(&config, &conn);

        config.load_txs = true;

        let txs_read = prepare_txs(&config, &conn);

        assert!(txs_gen == txs_read)
    }
}
