use std::sync::mpsc::channel;

use anyhow::{anyhow, Result as AnyResult};
use codec::Decode;
use log::{error, info};
use substrate_api_client::ApiResult;

use crate::{AnyConnection, BlockNumber, Header};

pub fn wait_for_event<C: AnyConnection, E: Decode + Clone, P: Fn(E) -> bool>(
    connection: &C,
    event: (&str, &str),
    predicate: P,
) -> AnyResult<E> {
    let (module, variant) = event;
    info!(target: "aleph-client", "Creating event subscription {}/{}", module, variant);

    let (events_in, events_out) = channel();
    connection.as_connection().subscribe_events(events_in)?;

    loop {
        let args: ApiResult<E> =
            connection
                .as_connection()
                .wait_for_event(module, variant, None, &events_out);

        match args {
            Ok(event) if predicate(event.clone()) => return Ok(event),
            Ok(_) => (),
            Err(why) => error!(target: "aleph-client", "Error {:?}", why),
        }
    }
}

pub fn wait_for_finalized_block<C: AnyConnection>(
    connection: &C,
    block_number: BlockNumber,
) -> AnyResult<BlockNumber> {
    let (sender, receiver) = channel();
    connection
        .as_connection()
        .subscribe_finalized_heads(sender)?;

    while let Ok(header) = receiver
        .recv()
        .map(|h| serde_json::from_str::<Header>(&h).expect("Should deserialize header"))
    {
        info!(target: "aleph-client", "Received header for a block number {:?}", header.number);

        if header.number.ge(&block_number) {
            return Ok(block_number);
        }
    }

    Err(anyhow!("Waiting for finalization is no longer possible"))
}
