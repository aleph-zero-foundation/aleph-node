use aleph_client::{create_connection, wait_for_finalized_block};

use crate::config::Config;

pub fn finalization(config: &Config) -> anyhow::Result<u32> {
    let connection = create_connection(&config.node);
    wait_for_finalized_block(&connection, 1)
}
