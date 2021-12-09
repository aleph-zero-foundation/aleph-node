// Everything here is dead code, but I don't want to create one enormous PR.
#![allow(dead_code)]
use codec::{Decode, Encode};
use futures::stream::Stream;
use sc_network::{Event, Multiaddr, NotificationSender, PeerId as ScPeerId};
use sp_api::NumberFor;
use sp_runtime::traits::Block;
use std::{borrow::Cow, collections::HashSet, pin::Pin};

mod connection_manager;
mod service;
mod substrate;

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
pub(crate) const ALEPH_PROTOCOL_NAME: &str = "/cardinals/aleph/2";

/// Name of the network protocol used by Aleph Zero validators. Similar to
/// ALEPH_PROTOCOL_NAME, but only used by validators that authenticated to each other.
pub(crate) const ALEPH_VALIDATOR_PROTOCOL_NAME: &str = "/cardinals/aleph_validator/1";

/// The Generic protocol is used for validator discovery.
/// The Validator protocol is used for validator-specific messages, i.e. ones needed for
/// finalization.
#[derive(Debug, PartialEq)]
pub enum Protocol {
    Generic,
    Validator,
}

impl Protocol {
    fn name(&self) -> Cow<'static, str> {
        use Protocol::*;
        match self {
            Generic => Cow::Borrowed(ALEPH_PROTOCOL_NAME),
            Validator => Cow::Borrowed(ALEPH_VALIDATOR_PROTOCOL_NAME),
        }
    }
}

type NetworkEventStream = Pin<Box<dyn Stream<Item = Event> + Send>>;

/// Abstraction over a network.
pub trait Network: Clone + Send + Sync + 'static {
    /// Returns a stream of events representing what happens on the network.
    fn event_stream(&self) -> NetworkEventStream;

    /// A sender for sending messages to the given peer, see the substrate network docs for how to
    /// use it. Returns None if not connected to the peer.
    fn message_sender(
        &self,
        peer_id: PeerId,
        protocol: Cow<'static, str>,
    ) -> Option<NotificationSender>;

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
#[derive(Debug, PartialEq)]
pub enum DataCommand {
    Broadcast,
    SendTo(PeerId, Protocol),
}

enum ConnectionCommand {
    AddReserved(HashSet<Multiaddr>),
    DelReserved(HashSet<PeerId>),
}
