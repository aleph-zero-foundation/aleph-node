use std::sync::mpsc::channel;

use codec::Decode;
use log::{debug, error, info};
use substrate_api_client::rpc::ws_client::{EventsDecoder, RuntimeEvent};
use substrate_api_client::utils::FromHexString;

use crate::utils::{Connection, Header};

#[derive(Debug, Decode)]
struct NewSessionEvent {
    session_index: u32,
}

/// blocking wait, if ongoing session index is >= new_session_index returns the current
pub fn wait_for_session(connection: Connection, new_session_index: u32) -> anyhow::Result<u32> {
    let module = "Session";
    let variant = "NewSession";
    info!("[+] Creating event subscription {}/{}", module, variant);
    let (events_in, events_out) = channel();
    connection.subscribe_events(events_in)?;

    let event_decoder = EventsDecoder::try_from(connection.metadata)?;

    loop {
        let event_str = events_out.recv().unwrap();
        let events = event_decoder.decode_events(&mut Vec::from_hex(event_str)?.as_slice());

        match events {
            Ok(raw_events) => {
                for (phase, event) in raw_events.into_iter() {
                    info!("[+] Received event: {:?}, {:?}", phase, event);
                    match event {
                        RuntimeEvent::Raw(raw)
                            if raw.module == module && raw.variant == variant =>
                        {
                            let NewSessionEvent { session_index } =
                                NewSessionEvent::decode(&mut &raw.data[..])?;
                            info!("[+] Decoded NewSession event {:?}", &session_index);
                            if session_index.ge(&new_session_index) {
                                return Ok(session_index);
                            }
                        }
                        _ => debug!("Ignoring some other event: {:?}", event),
                    }
                }
            }
            Err(why) => error!("Error {:?}", why),
        }
    }
}

/// blocks the main thread waiting for a block with a number at least `block_number`
pub fn wait_for_finalized_block(connection: Connection, block_number: u32) -> anyhow::Result<u32> {
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
