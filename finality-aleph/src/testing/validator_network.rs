use std::sync::Once;

use tokio::time::Duration;

use crate::testing::mocks::validator_network::scenario_with_timeout;

static INIT: Once = Once::new();

/// Required to capture logs from the tests e.g. by running
/// `RUST_LOG=info cargo test -- --nocapture testing::validator_network`
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
