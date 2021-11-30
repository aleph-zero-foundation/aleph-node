use std::sync::mpsc::channel;

use codec::Decode;
use log::{error, info};
use substrate_api_client::ApiResult;

use crate::{Connection, Header};

pub fn wait_for_event<E: Decode + Clone, P: Fn(E) -> bool>(
    connection: &Connection,
    event: (&str, &str),
    predicate: P,
) -> anyhow::Result<E> {
    let (module, variant) = event;
    info!("[+] Creating event subscription {}/{}", module, variant);

    let (events_in, events_out) = channel();
    connection.subscribe_events(events_in)?;

    loop {
        let args: ApiResult<E> = connection.wait_for_event(module, variant, None, &events_out);
        match args {
            Ok(event) if predicate(event.clone()) => return Ok(event),
            Ok(_) => (),
            Err(why) => error!("Error {:?}", why),
        }
    }
}

/// blocks the main thread waiting for a block with a number at least `block_number`
pub fn wait_for_finalized_block(connection: &Connection, block_number: u32) -> anyhow::Result<u32> {
    let (sender, receiver) = channel();
    connection.subscribe_finalized_heads(sender)?;

    while let Ok(header) = receiver
        .recv()
        .map(|h| serde_json::from_str::<Header>(&h).unwrap())
    {
        info!("[+] Received header for a block number {:?}", header.number);

        if header.number.ge(&block_number) {
            return Ok(block_number);
        }
    }

    Err(anyhow::anyhow!("Giving up"))
}
