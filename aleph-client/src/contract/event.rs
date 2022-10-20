//! Utilities for listening for contract events.
//!
//! To use the module you will need to first create a subscription (a glorified `Receiver<String>`),
//! then run the listen loop. You might want to run the loop in a separate thread.
//!
//! ```no_run
//! # use std::sync::Arc;
//! # use std::sync::mpsc::channel;
//! # use std::thread;
//! # use std::time::Duration;
//! # use aleph_client::{Connection, SignedConnection};
//! # use aleph_client::contract::ContractInstance;
//! # use aleph_client::contract::event::{listen_contract_events, subscribe_events};
//! # use anyhow::Result;
//! # use sp_core::crypto::AccountId32;
//! # fn example(conn: SignedConnection, address1: AccountId32, address2: AccountId32, path1: &str, path2: &str) -> Result<()> {
//!     let subscription = subscribe_events(&conn)?;
//!
//!     // The `Arc` makes it possible to pass a reference to the contract to another thread
//!     let contract1 = Arc::new(ContractInstance::new(address1, path1)?);
//!     let contract2 = Arc::new(ContractInstance::new(address2, path2)?);
//!     let (cancel_tx, cancel_rx) = channel();
//!
//!     let contract1_copy = contract1.clone();
//!     let contract2_copy = contract2.clone();
//!
//!     thread::spawn(move || {
//!         listen_contract_events(
//!             subscription,
//!             &[contract1_copy.as_ref(), &contract2_copy.as_ref()],
//!             Some(cancel_rx),
//!             |event_or_error| { println!("{:?}", event_or_error) }
//!         );
//!     });
//!
//!     thread::sleep(Duration::from_secs(20));
//!     cancel_tx.send(()).unwrap();
//!
//!     contract1.contract_exec0(&conn, "some_method")?;
//!     contract2.contract_exec0(&conn, "some_other_method")?;
//!
//! #   Ok(())
//! # }
//! ```

use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver},
};

use ac_node_api::events::{EventsDecoder, Raw, RawEvent};
use anyhow::{bail, Context, Result};
use contract_transcode::{ContractMessageTranscoder, Transcoder, TranscoderBuilder, Value};
use ink_metadata::InkProject;
use sp_core::crypto::{AccountId32, Ss58Codec};
use substrate_api_client::Metadata;

use crate::{contract::ContractInstance, AnyConnection};

/// Represents a single event emitted by a contract.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ContractEvent {
    /// The address of the contract that emitted the event.
    pub contract: AccountId32,
    /// The name of the event.
    pub ident: Option<String>,
    /// Data contained in the event.
    pub data: HashMap<String, Value>,
}

/// An opaque wrapper around a `Receiver<String>` that can be used to listen for contract events.
pub struct EventSubscription {
    receiver: Receiver<String>,
    metadata: Metadata,
}

/// Creates a subscription to all events that can be used to [listen_contract_events]
pub fn subscribe_events<C: AnyConnection>(conn: &C) -> Result<EventSubscription> {
    let conn = conn.as_connection();
    let (sender, receiver) = channel();

    conn.subscribe_events(sender)?;

    Ok(EventSubscription {
        metadata: conn.metadata,
        receiver,
    })
}

/// Starts an event listening loop.
///
/// Will execute the handler for every contract event and every error encountered while fetching
/// from `subscription`. Only events coming from the address of one of the `contracts` will be
/// decoded.
///
/// The loop will terminate once `subscription` is closed or once any message is received on
/// `cancel` (if provided).
pub fn listen_contract_events<F: Fn(Result<ContractEvent>)>(
    subscription: EventSubscription,
    contracts: &[&ContractInstance],
    cancel: Option<Receiver<()>>,
    handler: F,
) {
    let events_decoder = EventsDecoder::new(subscription.metadata.clone());
    let events_transcoder = TranscoderBuilder::new(&subscription.metadata.runtime_metadata().types)
        .with_default_custom_type_transcoders()
        .done();
    let contracts = contracts
        .iter()
        .map(|contract| (contract.address().clone(), contract.ink_project()))
        .collect::<HashMap<_, _>>();

    for batch in subscription.receiver.iter() {
        match decode_contract_event_batch(
            &subscription.metadata,
            &events_decoder,
            &events_transcoder,
            &contracts,
            batch,
        ) {
            Ok(events) => {
                for event in events {
                    handler(event);
                }
            }
            Err(err) => handler(Err(err)),
        }

        if cancel
            .as_ref()
            .map(|cancel| cancel.try_recv().is_ok())
            .unwrap_or(false)
        {
            break;
        }
    }
}

/// Consumes a raw `batch` of chain events, and returns only those that are coming from `contracts`.
///
/// This function, somewhat confusingly, returns a `Result<Vec<Result<_>>>` - this is to represent
/// the fact that an error might occur both while decoding the whole batch and for each event. This
/// is unwrapped in [listen_contract_events] and doesn't leak outside this module.
fn decode_contract_event_batch(
    metadata: &Metadata,
    events_decoder: &EventsDecoder,
    events_transcoder: &Transcoder,
    contracts: &HashMap<AccountId32, &InkProject>,
    batch: String,
) -> Result<Vec<Result<ContractEvent>>> {
    let mut results = vec![];

    let batch = batch.replacen("0x", "", 1);
    let bytes = hex::decode(batch)?;
    let events = events_decoder.decode_events(&mut bytes.as_slice())?;

    for (_phase, raw_event) in events {
        match raw_event {
            Raw::Error(err) => results.push(Err(err.into())),
            Raw::Event(event) => {
                if event.pallet == "Contracts" && event.variant == "ContractEmitted" {
                    results.push(decode_contract_event(
                        metadata,
                        contracts,
                        events_transcoder,
                        event,
                    ))
                }
            }
        }
    }

    Ok(results)
}

fn decode_contract_event(
    metadata: &Metadata,
    contracts: &HashMap<AccountId32, &InkProject>,
    events_transcoder: &Transcoder,
    event: RawEvent,
) -> Result<ContractEvent> {
    let event_metadata = metadata.event(event.pallet_index, event.variant_index)?;

    let parse_pointer = &mut event.data.0.as_slice();
    let mut raw_data = None;
    let mut contract_address = None;

    for field in event_metadata.variant().fields() {
        if field.name() == Some(&"data".to_string()) {
            raw_data = Some(<&[u8]>::clone(parse_pointer));
        } else {
            let field_value = events_transcoder.decode(field.ty().id(), parse_pointer);

            if field.name() == Some(&"contract".to_string()) {
                contract_address = field_value.ok();
            }
        }
    }

    if let Some(Value::Literal(address)) = contract_address {
        let address = AccountId32::from_string(&address)?;
        let contract_metadata = contracts
            .get(&address)
            .context("Event from unknown contract")?;

        let mut raw_data = raw_data.context("Event data field not found")?;
        let event_data = ContractMessageTranscoder::new(contract_metadata)
            .decode_contract_event(&mut raw_data)
            .context("Failed to decode contract event")?;

        build_event(address, event_data)
    } else {
        bail!("Contract event did not contain contract address");
    }
}

fn build_event(address: AccountId32, event_data: Value) -> Result<ContractEvent> {
    match event_data {
        Value::Map(map) => Ok(ContractEvent {
            contract: address,
            ident: map.ident(),
            data: map
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone()))
                .collect(),
        }),
        _ => bail!("Contract event data is not a map"),
    }
}
