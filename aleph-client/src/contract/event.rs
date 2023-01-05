//! Utilities for listening for contract events.
//!
//! To use the module you will need to pass a connection, some contracts and an `UnboundedSender` to the
//! [listen_contract_events] function. You most likely want to `tokio::spawn` the resulting future, so that it runs
//! concurrently.
//!
//! ```no_run
//! # use std::sync::Arc;
//! # use std::sync::mpsc::channel;
//! # use std::time::Duration;
//! # use aleph_client::{AccountId, Connection, SignedConnection};
//! # use aleph_client::contract::ContractInstance;
//! # use aleph_client::contract::event::{listen_contract_events};
//! # use anyhow::Result;
//! use futures::{channel::mpsc::unbounded, StreamExt};
//!
//! # async fn example(conn: Connection, signed_conn: SignedConnection, address1: AccountId, address2: AccountId, path1: &str, path2: &str) -> Result<()> {
//! // The `Arc` makes it possible to pass a reference to the contract to another thread
//! let contract1 = Arc::new(ContractInstance::new(address1, path1)?);
//! let contract2 = Arc::new(ContractInstance::new(address2, path2)?);
//!
//! let conn_copy = conn.clone();
//! let contract1_copy = contract1.clone();
//! let contract2_copy = contract2.clone();
//!
//! let (tx, mut rx) = unbounded();
//! let listen = || async move {
//!     listen_contract_events(&conn, &[contract1_copy.as_ref(), contract2_copy.as_ref()], tx).await?;
//!     <Result<(), anyhow::Error>>::Ok(())
//! };
//! let join = tokio::spawn(listen());
//!
//! contract1.contract_exec0(&signed_conn, "some_method").await?;
//! contract2.contract_exec0(&signed_conn, "some_other_method").await?;
//!
//! println!("Received event {:?}", rx.next().await);
//!
//! rx.close();
//! join.await??;
//!
//! #   Ok(())
//! # }
//! ```

use std::collections::HashMap;

use anyhow::{bail, Result};
use contract_transcode::Value;
use futures::{channel::mpsc::UnboundedSender, StreamExt};

use crate::{contract::ContractInstance, AccountId, Connection};

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

/// Starts an event listening loop.
///
/// Will send contract event and every error encountered while fetching through the provided [UnboundedSender].
/// Only events coming from the address of one of the `contracts` will be decoded.
///
/// The loop will terminate once `sender` is closed. The loop may also terminate in case of errors while fetching blocks
/// or decoding events (pallet events, contract event decoding errors are sent over the channel).
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

        let block = block?;

        for event in block.events().await?.iter() {
            let event = event?;

            if let Some(event) =
                event.as_event::<crate::api::contracts::events::ContractEmitted>()?
            {
                if let Some(contract) = contracts
                    .iter()
                    .find(|contract| contract.address() == &event.contract)
                {
                    let data = zero_prefixed(&event.data);
                    let event = contract
                        .transcoder
                        .decode_contract_event(&mut data.as_slice());

                    sender.unbounded_send(
                        event.and_then(|event| build_event(contract.address().clone(), event)),
                    )?;
                }
            }
        }
    }

    Ok(())
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
