use std::sync::mpsc::channel;

use codec::Decode;
use log::{error, info};
use substrate_api_client::ApiResult;

use crate::Connection;

pub fn wait_for_event<E: Decode + Clone, P: Fn(E) -> bool>(
    connection: &Connection,
    event: (&str, &str),
    predicate: P,
) -> anyhow::Result<E> {
    let (module, variant) = event;
    info!(target: "aleph-client", "Creating event subscription {}/{}", module, variant);

    let (events_in, events_out) = channel();
    connection.subscribe_events(events_in)?;

    loop {
        let args: ApiResult<E> = connection.wait_for_event(module, variant, None, &events_out);
        match args {
            Ok(event) if predicate(event.clone()) => return Ok(event),
            Ok(_) => (),
            Err(why) => error!(target: "aleph-client", "Error {:?}", why),
        }
    }
}
