use clap::{arg, Parser};
use log::info;
use synthetic_link::{SyntheticNetwork, SyntheticNetworkClient};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "http://Node0:80/qos")]
    url: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Args::parse();
    let synth_net_url = args.url;

    info!("reading SyntheticNetwork configuration from stdin");
    let deserializer = serde_json::Deserializer::from_reader(std::io::stdin());
    let synth_net_config: SyntheticNetwork = deserializer
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no configuration on stdin"))
        .unwrap_or_else(|e| panic!("unable to parse SyntheticNetwork config: {}", e));
    info!("parsed SyntheticNetwork configuration");

    info!("commiting configuration");
    let mut synth_net_client = SyntheticNetworkClient::new(synth_net_url);
    synth_net_client
        .commit_config(&synth_net_config)
        .await
        .unwrap_or_else(|e| panic!("failed to commit SyntheticNetwork configuration: {}", e));
    info!("successfully committed new configuration");
}
