//! This module provides utilities corresponding to the events emitted by a contract.
//!
//! There are two ways that you can get contract events:
//!  1. By fetching events corresponding to a particular transaction. For this, you will need to
//!     provide a connection, contract instance and transaction coordinate to [get_contract_events]
//!     function. Similarly to [crate::utility::BlocksApi::get_tx_events], it will fetch block
//!     events, filter them and decode all relevant ones.
//!  2. By listening to all contract events. For this, you will need to provide a connection, some
//!     contracts and an `UnboundedSender` to the [listen_contract_events] function. In a loop,
//!     it will inspect every finalized block and look for contract events.

use std::{collections::HashMap, error::Error};

use anyhow::{anyhow, bail, Result};
use contract_transcode::Value;
use futures::{channel::mpsc::UnboundedSender, StreamExt};
use subxt::events::EventDetails;

use crate::{
    api::contracts::events::ContractEmitted, connections::TxInfo, contract::ContractInstance,
    utility::BlocksApi, AccountId, Connection,
};

/// Represents a single event emitted by a contract.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ContractEvent {
    /// The address of the contract that emitted the event.
    pub contract: AccountId,
    /// The name of the event.
    pub name: Option<String>,
    /// Data contained in the event.
    pub data: HashMap<String, Value>,
}

/// Fetch and decode all events that correspond to the call identified by `tx_info` made to
/// `contract`.
///
/// ```no_run
/// # use aleph_client::{AccountId, Connection, SignedConnection};
/// # use aleph_client::contract::ContractInstance;
/// # use aleph_client::contract::event::{get_contract_events, listen_contract_events};
/// # use anyhow::Result;
/// use futures::{channel::mpsc::unbounded, StreamExt};
///
/// # async fn example(conn: Connection, signed_conn: SignedConnection, address: AccountId, path: &str) -> Result<()> {
/// let contract = ContractInstance::new(address, path)?;
///
/// let tx_info = contract.contract_exec0(&signed_conn, "some_method").await?;
///
/// println!("Received events {:?}", get_contract_events(&conn, &contract, tx_info).await);
///
/// #   Ok(())
/// # }
/// ```
pub async fn get_contract_events(
    conn: &Connection,
    contract: &ContractInstance,
    tx_info: TxInfo,
) -> Result<Vec<ContractEvent>> {
    let events = conn.get_tx_events(tx_info).await?;
    translate_events(events.iter(), &[contract])
        .into_iter()
        .collect()
}

/// Starts an event listening loop. Will send contract event and every error encountered while
/// fetching through the provided [UnboundedSender].
///
/// Only events coming from the address of one of the `contracts` will be decoded.
///
/// The loop will terminate once `sender` is closed. The loop may also terminate in case of errors while fetching blocks
/// or decoding events (pallet events, contract event decoding errors are sent over the channel).
///
/// You most likely want to `tokio::spawn` the resulting future, so that it runs concurrently.
///
/// ```no_run
/// # use std::sync::Arc;
/// # use std::sync::mpsc::channel;
/// # use std::time::Duration;
/// # use aleph_client::{AccountId, Connection, SignedConnection};
/// # use aleph_client::contract::ContractInstance;
/// # use aleph_client::contract::event::{listen_contract_events};
/// # use anyhow::Result;
/// use futures::{channel::mpsc::unbounded, StreamExt};
///
/// # async fn example(conn: Connection, signed_conn: SignedConnection, address1: AccountId, address2: AccountId, path1: &str, path2: &str) -> Result<()> {
/// // The `Arc` makes it possible to pass a reference to the contract to another thread
/// let contract1 = Arc::new(ContractInstance::new(address1, path1)?);
/// let contract2 = Arc::new(ContractInstance::new(address2, path2)?);
///
/// let conn_copy = conn.clone();
/// let contract1_copy = contract1.clone();
/// let contract2_copy = contract2.clone();
///
/// let (tx, mut rx) = unbounded();
/// let listen = || async move {
///     listen_contract_events(&conn, &[contract1_copy.as_ref(), contract2_copy.as_ref()], tx).await?;
///     <Result<(), anyhow::Error>>::Ok(())
/// };
/// let join = tokio::spawn(listen());
///
/// contract1.contract_exec0(&signed_conn, "some_method").await?;
/// contract2.contract_exec0(&signed_conn, "some_other_method").await?;
///
/// println!("Received event {:?}", rx.next().await);
///
/// rx.close();
/// join.await??;
///
/// #   Ok(())
/// # }
/// ```
pub async fn listen_contract_events(
    conn: &Connection,
    contracts: &[&ContractInstance],
    sender: UnboundedSender<Result<ContractEvent>>,
) -> Result<()> {
    let mut block_subscription = conn.as_client().blocks().subscribe_finalized().await?;

    while let Some(block) = block_subscription.next().await {
        if sender.is_closed() {
            break;
        }
        let events = block?.events().await?;
        for event in translate_events(events.iter(), contracts) {
            sender.unbounded_send(event)?;
        }
    }

    Ok(())
}

/// Try to convert `events` to `ContractEvent` using matching contract from `contracts`.
fn translate_events<
    Err: Error + Into<anyhow::Error> + Send + Sync + 'static,
    E: Iterator<Item = Result<EventDetails, Err>>,
>(
    events: E,
    contracts: &[&ContractInstance],
) -> Vec<Result<ContractEvent>> {
    events
        .filter_map(|maybe_event| {
            maybe_event
                .map(|e| e.as_event::<ContractEmitted>().ok().flatten())
                .transpose()
        })
        .map(|maybe_event| match maybe_event {
            Ok(e) => translate_event(&e, contracts),
            Err(e) => Err(anyhow::Error::from(e)),
        })
        .collect()
}

/// Try to convert `event` to `ContractEvent` using matching contract from `contracts`.
fn translate_event(
    event: &ContractEmitted,
    contracts: &[&ContractInstance],
) -> Result<ContractEvent> {
    let matching_contract = contracts
        .iter()
        .find(|contract| contract.address() == &event.contract)
        .ok_or_else(|| anyhow!("The event wasn't emitted by any of the provided contracts"))?;

    let data = zero_prefixed(&event.data);
    let data = matching_contract
        .transcoder
        .decode_contract_event(&mut data.as_slice())?;

    build_event(matching_contract.address.clone(), data)
}

/// The contract transcoder assumes there is an extra byte (that it discards) indicating the size of the data. However,
/// data arriving through the subscription as used in this file don't have this extra byte. This function adds it.
fn zero_prefixed(data: &[u8]) -> Vec<u8> {
    let mut result = vec![0];
    result.extend_from_slice(data);
    result
}

fn build_event(address: AccountId, event_data: Value) -> Result<ContractEvent> {
    match event_data {
        Value::Map(map) => Ok(ContractEvent {
            contract: address,
            name: map.ident(),
            data: map
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone()))
                .collect(),
        }),
        _ => bail!("Contract event data is not a map"),
    }
}
