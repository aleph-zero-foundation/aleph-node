// Everything here is dead code, but I don't want to create one enormous PR.
#![allow(dead_code)]
use aleph_bft::Recipient;
use async_trait::async_trait;
use codec::{Codec, Decode, Encode};
use futures::stream::Stream;
use sc_network::{Event, Multiaddr, PeerId as ScPeerId};
use sp_api::NumberFor;
use sp_runtime::traits::Block;
use std::{borrow::Cow, collections::HashSet, pin::Pin};

mod aleph;
mod component;
mod manager;
#[cfg(test)]
mod mock;
mod rmc;
mod service;
mod session;
mod split;
mod substrate;

use component::{
    Network as ComponentNetwork, Receiver as ReceiverComponent, Sender as SenderComponent,
};
use manager::SessionCommand;

pub use aleph::NetworkData as AlephNetworkData;
pub use rmc::NetworkData as RmcNetworkData;

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub struct PeerId(pub(crate) ScPeerId);

impl From<PeerId> for ScPeerId {
    fn from(wrapper: PeerId) -> Self {
        wrapper.0
    }
}

impl From<ScPeerId> for PeerId {
    fn from(id: ScPeerId) -> Self {
        PeerId(id)
    }
}

impl Encode for PeerId {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.0.to_bytes().using_encoded(f)
    }
}

impl Decode for PeerId {
    fn decode<I: codec::Input>(value: &mut I) -> Result<Self, codec::Error> {
        let bytes = Vec::<u8>::decode(value)?;
        ScPeerId::from_bytes(&bytes)
            .map_err(|_| "PeerId not encoded with to_bytes".into())
            .map(|pid| pid.into())
    }
}

/// Name of the network protocol used by Aleph Zero. This is how messages
/// are subscribed to ensure that we are gossiping and communicating with our
/// own network.
const ALEPH_PROTOCOL_NAME: &str = "/cardinals/aleph/2";

/// Name of the network protocol used by Aleph Zero validators. Similar to
/// ALEPH_PROTOCOL_NAME, but only used by validators that authenticated to each other.
const ALEPH_VALIDATOR_PROTOCOL_NAME: &str = "/cardinals/aleph_validator/1";

/// The Generic protocol is used for validator discovery.
/// The Validator protocol is used for validator-specific messages, i.e. ones needed for
/// finalization.
#[derive(Debug, PartialEq, Clone)]
pub enum Protocol {
    Generic,
    Validator,
}

impl Protocol {
    pub fn name(&self) -> Cow<'static, str> {
        use Protocol::*;
        match self {
            Generic => Cow::Borrowed(ALEPH_PROTOCOL_NAME),
            Validator => Cow::Borrowed(ALEPH_VALIDATOR_PROTOCOL_NAME),
        }
    }
}

type NetworkEventStream = Pin<Box<dyn Stream<Item = Event> + Send>>;

/// Abstraction over a network.
#[async_trait]
pub trait Network: Clone + Send + Sync + 'static {
    type SendError: std::error::Error;
    /// Returns a stream of events representing what happens on the network.
    fn event_stream(&self) -> NetworkEventStream;

    /// A method for sending data to the given peer using a given protocol. Returns Error if not connected to the peer.
    async fn send<'a>(
        &'a self,
        data: impl Into<Vec<u8>> + Send + Sync + 'static,
        peer_id: PeerId,
        protocol: Cow<'static, str>,
    ) -> Result<(), Self::SendError>;

    /// Add peers to one of the reserved sets.
    fn add_reserved(&self, addresses: HashSet<Multiaddr>, protocol: Cow<'static, str>);

    /// Remove peers from one of the reserved sets.
    fn remove_reserved(&self, peers: HashSet<PeerId>, protocol: Cow<'static, str>);
}

/// Abstraction for requesting own network addresses and PeerId.
pub trait NetworkIdentity {
    /// The external identity of this node, consisting of addresses and the PeerId.
    fn identity(&self) -> (Vec<Multiaddr>, PeerId);
}

/// Abstraction for requesting justifications for finalized blocks and stale blocks.
pub trait RequestBlocks<B: Block>: Clone + Send + Sync + 'static {
    /// Request the justification for the given block
    fn request_justification(&self, hash: &B::Hash, number: NumberFor<B>);

    /// Request the given block -- this is supposed to be used only for "old forks".
    fn request_stale_block(&self, hash: B::Hash, number: NumberFor<B>);
}

/// What do do with a specific piece of data.
/// Note that broadcast does not specify the protocol, as we only broadcast Generic messages in this sense.
#[derive(Debug, PartialEq, Clone)]
pub enum DataCommand {
    Broadcast,
    SendTo(PeerId, Protocol),
}

/// Commands for manipulating the reserved peers set.
#[derive(Debug, PartialEq)]
pub enum ConnectionCommand {
    AddReserved(HashSet<Multiaddr>),
    DelReserved(HashSet<PeerId>),
}

/// Returned when something went wrong when sending data using a DataNetwork.
pub enum SendError {
    SendFailed,
}

/// What the data sent using the network has to satisfy.
pub trait Data: Clone + Codec + Send + Sync {}

impl<D: Clone + Codec + Send + Sync> Data for D {}

/// A generic interface for sending and receiving data.
#[async_trait::async_trait]
pub trait DataNetwork<D: Data>: Send + Sync {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError>;
    async fn next(&mut self) -> Option<D>;
}

// This should be removed after compatibility with the old network is no longer needed.
mod compatibility;
pub use compatibility::*;
