use std::env;
use std::time::Instant;

use clap::Parser;

use aleph_e2e_client::config::Config;
use aleph_e2e_client::test;
use log::info;

fn main() -> anyhow::Result<()> {
    init_env();

    let config: Config = Config::parse();

    run(test::finalization, "finalization", &config)?;
    run(test::change_validators, "validators change", &config)?;
    run(test::token_transfer, "token transfer", &config)?;
    run(test::channeling_fee, "channeling fee", &config)?;
    run(test::treasury_access, "treasury access", &config)?;
    run(test::batch_transactions, "batch_transactions", &config)?;
    run(test::staking_test, "staking_test", &config)?;
    run(test::fee_calculation, "fee calculation", &config)?;

    Ok(())
}

fn init_env() {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(env_logger::DEFAULT_FILTER_ENV, "warn");
    }
    env_logger::init();
}

fn run<T>(
    testcase: fn(&Config) -> anyhow::Result<T>,
    name: &str,
    config: &Config,
) -> anyhow::Result<()> {
    info!("Running test: {}", name);
    let start = Instant::now();
    testcase(config).map(|_| {
        let elapsed = Instant::now().duration_since(start);
        println!("Ok! Elapsed time {}ms", elapsed.as_millis());
    })
}
