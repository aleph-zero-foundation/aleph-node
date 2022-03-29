use std::{env, time::Instant};

use clap::Parser;
use log::info;

use aleph_e2e_client::{
    test_batch_transactions, test_change_validators, test_channeling_fee, test_fee_calculation,
    test_finalization, test_staking_era_payouts, test_staking_new_validator, test_token_transfer,
    test_treasury_access, Config,
};

fn main() -> anyhow::Result<()> {
    init_env();

    let config: Config = Config::parse();

    run(test_finalization, "finalization", &config)?;
    run(test_token_transfer, "token transfer", &config)?;
    run(test_channeling_fee, "channeling fee", &config)?;
    run(test_treasury_access, "treasury access", &config)?;
    run(test_batch_transactions, "batch_transactions", &config)?;
    run(test_staking_era_payouts, "staking_era_payouts", &config)?;
    run(test_staking_new_validator, "staking_new_validator", &config)?;
    run(test_change_validators, "validators change", &config)?;
    run(test_fee_calculation, "fee calculation", &config)?;

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
