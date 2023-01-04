use log::info;
use synthetic_link::SyntheticNetworkClient;

pub type Milliseconds = u64;

pub async fn set_out_latency(milliseconds: Milliseconds, synthetic_url: String) {
    info!(
        "setting out-latency of node {} to {}ms",
        synthetic_url, milliseconds
    );
    info!("creating an http client for url {}", synthetic_url);
    let mut client = SyntheticNetworkClient::new(synthetic_url);
    let mut config = client
        .load_config()
        .await
        .expect("we should be able to download config of the synthetic-network ");

    config.default_link.egress.latency = milliseconds;

    client
        .commit_config(&config)
        .await
        .expect("unable to commit network configuration");
}
