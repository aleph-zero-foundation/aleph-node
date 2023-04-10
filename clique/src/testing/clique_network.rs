use std::{
    collections::{BTreeMap, HashSet},
    sync::Once,
};

use aleph_bft_mock::Spawner;
use aleph_bft_types::SpawnHandle;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::info;
use rand::{thread_rng, Rng};
use tokio::time::{error::Elapsed, interval, timeout, Duration};

use crate::{
    mock::{
        random_keys, Addresses, MockData, MockDialer, MockListener, MockPublicKey, MockSecretKey,
        UnreliableConnectionMaker,
    },
    service::SpawnHandleT,
    Network, SecretKey, Service,
};

impl SpawnHandleT for Spawner {
    fn spawn(&self, name: &'static str, task: impl futures::Future<Output = ()> + Send + 'static) {
        SpawnHandle::spawn(self, name, task)
    }

    fn spawn_essential(
        &self,
        name: &'static str,
        task: impl futures::Future<Output = ()> + Send + 'static,
    ) -> std::pin::Pin<Box<dyn futures::Future<Output = Result<(), ()>> + Send>> {
        SpawnHandle::spawn_essential(self, name, task)
    }
}

pub const LOG_TARGET: &str = "network-clique-test";

const TWICE_MAX_DATA_SIZE: usize = 32 * 1024 * 1024;

#[allow(clippy::too_many_arguments)]
fn spawn_peer(
    secret_key: MockSecretKey,
    addr: Addresses,
    n_msg: usize,
    large_message_interval: Option<usize>,
    corrupted_message_interval: Option<usize>,
    dialer: MockDialer,
    listener: MockListener,
    report: mpsc::UnboundedSender<(MockPublicKey, usize)>,
    spawn_handle: Spawner,
) {
    let our_id = secret_key.public_key();
    let (service, mut interface) = Service::new(dialer, listener, secret_key, spawn_handle);
    // run the service
    tokio::spawn(async {
        let (_exit, rx) = oneshot::channel();
        service.run(rx).await;
    });
    // start connecting with the peers
    let mut peer_ids = Vec::with_capacity(addr.len());
    for (id, addrs) in addr.into_iter() {
        interface.add_connection(id.clone(), addrs);
        peer_ids.push(id);
    }
    // peer main loop
    // we send random messages to random peers
    // a message is a number in range 0..n_msg
    // we also keep a list of messages received at least once
    // on receiving a message we report the total number of distinct messages received so far
    // the goal is to receive every message at least once
    tokio::spawn(async move {
        let mut received: HashSet<usize> = HashSet::with_capacity(n_msg);
        let mut send_ticker = tokio::time::interval(Duration::from_millis(5));
        let mut counter: usize = 0;
        loop {
            tokio::select! {
                _ = send_ticker.tick() => {
                    counter += 1;
                    // generate random message
                    let filler_size = match large_message_interval {
                        Some(lmi) if counter % lmi == 0 => TWICE_MAX_DATA_SIZE,
                        _ => 0,
                    };
                    let data = match corrupted_message_interval {
                        Some(cmi) if counter % cmi == 0 => MockData::new_undecodable(thread_rng().gen_range(0..n_msg) as u32, filler_size),
                        _ => MockData::new(thread_rng().gen_range(0..n_msg) as u32, filler_size),
                    };
                    // choose a peer
                    let peer: MockPublicKey = peer_ids[thread_rng().gen_range(0..peer_ids.len())].clone();
                    // send
                    interface.send(data, peer);
                },
                data = interface.next() => {
                    // receive the message
                    let data: MockData = data.expect("next should not be closed");
                    // mark the message as received, we do not care about sender's identity
                    received.insert(data.data() as usize);
                    // report the number of received messages
                    report.unbounded_send((our_id.clone(), received.len())).expect("should send");
                },
            };
        }
    });
}

/// Takes O(n log n) rounds to finish, where n = n_peers * n_msg.
async fn scenario(
    n_peers: usize,
    n_msg: usize,
    broken_connection_interval: Option<usize>,
    large_message_interval: Option<usize>,
    corrupted_message_interval: Option<usize>,
    status_report_interval: Duration,
) {
    // create peer identities
    info!(target: LOG_TARGET, "generating keys...");
    let keys = random_keys(n_peers);
    info!(target: LOG_TARGET, "done");
    // prepare and run the manager
    let (mut connection_manager, mut callers, addr) =
        UnreliableConnectionMaker::new(keys.keys().cloned().collect());
    tokio::spawn(async move {
        connection_manager.run(broken_connection_interval).await;
    });
    // channel for receiving status updates from spawned peers
    let (tx_report, mut rx_report) = mpsc::unbounded::<(MockPublicKey, usize)>();
    let mut reports: BTreeMap<MockPublicKey, usize> =
        keys.keys().cloned().map(|id| (id, 0)).collect();
    // spawn peers
    for (id, secret_key) in keys.into_iter() {
        let mut addr = addr.clone();
        // do not connect with itself
        addr.remove(&secret_key.public_key());
        let (dialer, listener) = callers.remove(&id).expect("should contain all ids");
        spawn_peer(
            secret_key,
            addr,
            n_msg,
            large_message_interval,
            corrupted_message_interval,
            dialer,
            listener,
            tx_report.clone(),
            Spawner,
        );
    }
    let mut status_ticker = interval(status_report_interval);
    loop {
        tokio::select! {
            maybe_report = rx_report.next() => match maybe_report {
                Some((peer_id, peer_n_msg)) => {
                    reports.insert(peer_id, peer_n_msg);
                    if reports.values().all(|&x| x == n_msg) {
                        info!(target: LOG_TARGET, "Peers received {:?} messages out of {}, finishing.", reports.values(), n_msg);
                        return;
                    }
                },
                None => panic!("should receive"),
            },
            _ = status_ticker.tick() => {
                info!(target: LOG_TARGET, "Peers received {:?} messages out of {}.", reports.values(), n_msg);
            }
        };
    }
}

/// Takes O(n log n) rounds to finish, where n = n_peers * n_msg.
async fn scenario_with_timeout(
    n_peers: usize,
    n_msg: usize,
    broken_connection_interval: Option<usize>,
    large_message_interval: Option<usize>,
    corrupted_message_interval: Option<usize>,
    status_report_interval: Duration,
    scenario_timeout: Duration,
) -> Result<(), Elapsed> {
    timeout(
        scenario_timeout,
        scenario(
            n_peers,
            n_msg,
            broken_connection_interval,
            large_message_interval,
            corrupted_message_interval,
            status_report_interval,
        ),
    )
    .await
}

static INIT: Once = Once::new();

/// Required to capture logs from the tests e.g. by running
/// `RUST_LOG=info cargo test -- --nocapture testing::clique_network`
fn setup() {
    // env_logger::init can be called at most once
    INIT.call_once(|| {
        env_logger::init();
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn normal_conditions() {
    setup();
    let n_peers: usize = 10;
    let n_msg: usize = 30;
    let broken_connection_interval: Option<usize> = None;
    let large_message_interval: Option<usize> = None;
    let corrupted_message_interval: Option<usize> = None;
    let status_report_interval: Duration = Duration::from_secs(1);
    let timeout: Duration = Duration::from_secs(300);
    scenario_with_timeout(
        n_peers,
        n_msg,
        broken_connection_interval,
        large_message_interval,
        corrupted_message_interval,
        status_report_interval,
        timeout,
    )
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread")]
async fn connections_break() {
    setup();
    let n_peers: usize = 10;
    let n_msg: usize = 30;
    let broken_connection_interval: Option<usize> = Some(10);
    let large_message_interval: Option<usize> = None;
    let corrupted_message_interval: Option<usize> = None;
    let status_report_interval: Duration = Duration::from_secs(1);
    let timeout: Duration = Duration::from_secs(300);
    scenario_with_timeout(
        n_peers,
        n_msg,
        broken_connection_interval,
        large_message_interval,
        corrupted_message_interval,
        status_report_interval,
        timeout,
    )
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread")]
async fn large_messages_being_sent() {
    setup();
    let n_peers: usize = 10;
    let n_msg: usize = 30;
    let broken_connection_interval: Option<usize> = None;
    let large_message_interval: Option<usize> = Some(10);
    let corrupted_message_interval: Option<usize> = None;
    let status_report_interval: Duration = Duration::from_secs(1);
    let timeout: Duration = Duration::from_secs(300);
    scenario_with_timeout(
        n_peers,
        n_msg,
        broken_connection_interval,
        large_message_interval,
        corrupted_message_interval,
        status_report_interval,
        timeout,
    )
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread")]
async fn corrupted_messages_being_sent() {
    setup();
    let n_peers: usize = 10;
    let n_msg: usize = 30;
    let broken_connection_interval: Option<usize> = None;
    let large_message_interval: Option<usize> = None;
    let corrupted_message_interval: Option<usize> = Some(10);
    let status_report_interval: Duration = Duration::from_secs(1);
    let timeout: Duration = Duration::from_secs(300);
    scenario_with_timeout(
        n_peers,
        n_msg,
        broken_connection_interval,
        large_message_interval,
        corrupted_message_interval,
        status_report_interval,
        timeout,
    )
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread")]
async fn everything_fails_all_the_time() {
    setup();
    let n_peers: usize = 3;
    let n_msg: usize = 10;
    let broken_connection_interval: Option<usize> = Some(5);
    let large_message_interval: Option<usize> = Some(7);
    let corrupted_message_interval: Option<usize> = Some(8);
    let status_report_interval: Duration = Duration::from_secs(1);
    let timeout: Duration = Duration::from_secs(600);
    scenario_with_timeout(
        n_peers,
        n_msg,
        broken_connection_interval,
        large_message_interval,
        corrupted_message_interval,
        status_report_interval,
        timeout,
    )
    .await
    .expect("timeout");
}
