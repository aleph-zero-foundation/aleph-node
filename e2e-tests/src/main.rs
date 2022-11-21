use std::{env, time::Instant};

use aleph_e2e_client::{run_all_testcases, run_testcase, Config};
use clap::Parser;
use log::info;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_env();

    let config: Config = Config::parse();
    let test_cases = config.test_cases.clone();
    // Possibility to handle specified vs. default test cases
    // is helpful to parallelize e2e tests.
    match test_cases {
        Some(cases) => {
            info!("Running specified test cases.");
            run_specified_test_cases(cases, &config).await?;
        }
        None => {
            info!("Running all handled test cases.");
            run_all_testcases(&config).await?;
        }
    };
    Ok(())
}

fn init_env() {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(env_logger::DEFAULT_FILTER_ENV, "warn");
    }
    env_logger::init();
}

/// Runs specified test cases in sequence.
/// Checks whether each provided test case is valid.
async fn run_specified_test_cases(test_names: Vec<String>, config: &Config) -> anyhow::Result<()> {
    for test_name in test_names {
        run(&test_name, config).await?;
    }
    Ok(())
}

/// Runs single test case. Allows for a generic return type.
async fn run(name: &str, config: &Config) -> anyhow::Result<()> {
    info!("Running test: {}", name);
    let start = Instant::now();
    run_testcase(name, config).await.map(|_| {
        let elapsed = Instant::now().duration_since(start);
        println!("Ok! Elapsed time {}ms", elapsed.as_millis());
    })
}
