use common::create_connection;

use crate::config::Config;
use crate::waiting::wait_for_finalized_block;

pub fn finalization(config: Config) -> anyhow::Result<u32> {
    let connection = create_connection(config.node);
    wait_for_finalized_block(&connection, 1)
}
