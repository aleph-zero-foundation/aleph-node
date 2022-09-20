use std::{borrow::Cow, collections::HashSet, fmt, iter, pin::Pin, sync::Arc};

use async_trait::async_trait;
use codec::{Decode, Encode};
use futures::stream::{Stream, StreamExt};
use log::error;
use sc_network::{
    multiaddr::Protocol as MultiaddressProtocol, Event as SubstrateEvent, ExHashT, Multiaddr,
    NetworkService, NetworkStateInfo, NotificationSender, PeerId as SubstratePeerId,
};
use sp_api::NumberFor;
use sp_runtime::traits::Block;

use crate::network::{
    Event, EventStream, Multiaddress as MultiaddressT, Network, NetworkIdentity, NetworkSender,
    PeerId as PeerIdT, Protocol, RequestBlocks,
};

impl<B: Block, H: ExHashT> RequestBlocks<B> for Arc<NetworkService<B, H>> {
    fn request_justification(&self, hash: &B::Hash, number: NumberFor<B>) {
        NetworkService::request_justification(self, hash, number)
    }

    fn request_stale_block(&self, hash: B::Hash, number: NumberFor<B>) {
        // The below comment is adapted from substrate:
        // Notifies the sync service to try and sync the given block from the given peers. If the given vector
        // of peers is empty (as in our case) then the underlying implementation should make a best effort to fetch
        // the block from any peers it is connected to.
        NetworkService::set_sync_fork_request(self, Vec::new(), hash, number)
    }

    /// Clear all pending justification requests.
    fn clear_justification_requests(&self) {
        NetworkService::clear_justification_requests(self)
    }

    fn is_major_syncing(&self) -> bool {
        NetworkService::is_major_syncing(self)
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub struct PeerId(SubstratePeerId);

impl From<PeerId> for SubstratePeerId {
    fn from(wrapper: PeerId) -> Self {
        wrapper.0
    }
}

impl From<SubstratePeerId> for PeerId {
    fn from(id: SubstratePeerId) -> Self {
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
        SubstratePeerId::from_bytes(&bytes)
            .map_err(|_| "PeerId not encoded with to_bytes".into())
            .map(|pid| pid.into())
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let peer_id: String = self.0.to_string();

        let prefix: String = peer_id.chars().take(4).collect();

        let suffix: String = peer_id
            .chars()
            .skip(peer_id.len().saturating_sub(8))
            .collect();

        write!(f, "{}…{}", &prefix, &suffix)
    }
}

impl PeerIdT for PeerId {}

fn peer_id(protocol: &MultiaddressProtocol<'_>) -> Option<PeerId> {
    match protocol {
        MultiaddressProtocol::P2p(hashed_peer_id) => {
            SubstratePeerId::from_multihash(*hashed_peer_id)
                .ok()
                .map(PeerId)
        }
        _ => None,
    }
}

/// A wrapper for the Substrate multiaddress to allow encoding & decoding.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Multiaddress(Multiaddr);

impl From<Multiaddr> for Multiaddress {
    fn from(addr: Multiaddr) -> Self {
        Multiaddress(addr)
    }
}

impl From<Multiaddress> for Multiaddr {
    fn from(addr: Multiaddress) -> Self {
        addr.0
    }
}

impl Encode for Multiaddress {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.0.to_vec().using_encoded(f)
    }
}

impl Decode for Multiaddress {
    fn decode<I: codec::Input>(value: &mut I) -> Result<Self, codec::Error> {
        let bytes = Vec::<u8>::decode(value)?;
        Multiaddr::try_from(bytes)
            .map_err(|_| "Multiaddr not encoded as bytes".into())
            .map(|multiaddr| multiaddr.into())
    }
}

enum CommonPeerId {
    Unknown,
    Unique(PeerId),
    NotUnique,
}

impl From<CommonPeerId> for Option<PeerId> {
    fn from(cpi: CommonPeerId) -> Self {
        use CommonPeerId::*;
        match cpi {
            Unique(peer_id) => Some(peer_id),
            Unknown | NotUnique => None,
        }
    }
}

impl CommonPeerId {
    fn aggregate(self, peer_id: PeerId) -> Self {
        use CommonPeerId::*;
        match self {
            Unknown => Unique(peer_id),
            Unique(current_peer_id) => match peer_id == current_peer_id {
                true => Unique(current_peer_id),
                false => NotUnique,
            },
            NotUnique => NotUnique,
        }
    }
}

impl MultiaddressT for Multiaddress {
    type PeerId = PeerId;

    fn get_peer_id(&self) -> Option<Self::PeerId> {
        self.0
            .iter()
            .fold(
                CommonPeerId::Unknown,
                |common_peer_id, protocol| match peer_id(&protocol) {
                    Some(peer_id) => common_peer_id.aggregate(peer_id),
                    None => common_peer_id,
                },
            )
            .into()
    }

    fn add_matching_peer_id(mut self, peer_id: Self::PeerId) -> Option<Self> {
        match self.get_peer_id() {
            Some(peer) => match peer == peer_id {
                true => Some(self),
                false => None,
            },
            None => {
                self.0.push(MultiaddressProtocol::P2p(peer_id.0.into()));
                Some(self)
            }
        }
    }
}

/// Name of the network protocol used by Aleph Zero. This is how messages
/// are subscribed to ensure that we are gossiping and communicating with our
/// own network.
const ALEPH_PROTOCOL_NAME: &str = "/cardinals/aleph/2";

/// Name of the network protocol used by Aleph Zero validators. Similar to
/// ALEPH_PROTOCOL_NAME, but only used by validators that authenticated to each other.
const ALEPH_VALIDATOR_PROTOCOL_NAME: &str = "/cardinals/aleph_validator/1";

/// Returns the canonical name of the protocol.
pub fn protocol_name(protocol: &Protocol) -> Cow<'static, str> {
    use Protocol::*;
    match protocol {
        Generic => Cow::Borrowed(ALEPH_PROTOCOL_NAME),
        Validator => Cow::Borrowed(ALEPH_VALIDATOR_PROTOCOL_NAME),
    }
}

/// Attempts to convert the protocol name to a protocol.
fn to_protocol(protocol_name: &str) -> Result<Protocol, ()> {
    match protocol_name {
        ALEPH_PROTOCOL_NAME => Ok(Protocol::Generic),
        ALEPH_VALIDATOR_PROTOCOL_NAME => Ok(Protocol::Validator),
        _ => Err(()),
    }
}

#[derive(Debug)]
pub enum SenderError {
    CannotCreateSender(PeerId, Protocol),
    LostConnectionToPeer(PeerId),
    LostConnectionToPeerReady(PeerId),
}

impl fmt::Display for SenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SenderError::CannotCreateSender(peer_id, protocol) => {
                write!(
                    f,
                    "Can not create sender to peer {:?} with protocol {:?}",
                    peer_id, protocol
                )
            }
            SenderError::LostConnectionToPeer(peer_id) => {
                write!(
                    f,
                    "Lost connection to peer {:?} while preparing sender",
                    peer_id
                )
            }
            SenderError::LostConnectionToPeerReady(peer_id) => {
                write!(
                    f,
                    "Lost connection to peer {:?} after sender was ready",
                    peer_id
                )
            }
        }
    }
}

impl std::error::Error for SenderError {}

pub struct SubstrateNetworkSender {
    notification_sender: NotificationSender,
    peer_id: PeerId,
}

#[async_trait]
impl NetworkSender for SubstrateNetworkSender {
    type SenderError = SenderError;

    async fn send<'a>(
        &'a self,
        data: impl Into<Vec<u8>> + Send + Sync + 'static,
    ) -> Result<(), SenderError> {
        self.notification_sender
            .ready()
            .await
            .map_err(|_| SenderError::LostConnectionToPeer(self.peer_id))?
            .send(data)
            .map_err(|_| SenderError::LostConnectionToPeerReady(self.peer_id))
    }
}

type NetworkEventStream = Pin<Box<dyn Stream<Item = SubstrateEvent> + Send>>;

#[async_trait]
impl EventStream<Multiaddress> for NetworkEventStream {
    async fn next_event(&mut self) -> Option<Event<Multiaddress>> {
        use Event::*;
        use SubstrateEvent::*;
        loop {
            match self.next().await {
                Some(event) => match event {
                    SyncConnected { remote } => {
                        return Some(Connected(Multiaddress(
                            iter::once(MultiaddressProtocol::P2p(remote.into())).collect(),
                        )))
                    }
                    SyncDisconnected { remote } => return Some(Disconnected(remote.into())),
                    NotificationStreamOpened {
                        remote, protocol, ..
                    } => match to_protocol(protocol.as_ref()) {
                        Ok(protocol) => return Some(StreamOpened(remote.into(), protocol)),
                        Err(_) => continue,
                    },
                    NotificationStreamClosed { remote, protocol } => {
                        match to_protocol(protocol.as_ref()) {
                            Ok(protocol) => return Some(StreamClosed(remote.into(), protocol)),
                            Err(_) => continue,
                        }
                    }
                    NotificationsReceived { messages, .. } => {
                        return Some(Messages(
                            messages
                                .into_iter()
                                .filter_map(|(protocol, data)| {
                                    match to_protocol(protocol.as_ref()) {
                                        Ok(_) => Some(data),
                                        // This might end with us returning an empty vec, but it's probably not
                                        // worth it to handle this situation here.
                                        Err(_) => None,
                                    }
                                })
                                .collect(),
                        ));
                    }
                    Dht(_) => continue,
                },
                None => return None,
            }
        }
    }
}

impl<B: Block, H: ExHashT> Network for Arc<NetworkService<B, H>> {
    type SenderError = SenderError;
    type NetworkSender = SubstrateNetworkSender;
    type PeerId = PeerId;
    type Multiaddress = Multiaddress;
    type EventStream = NetworkEventStream;

    fn event_stream(&self) -> Self::EventStream {
        Box::pin(self.as_ref().event_stream("aleph-network"))
    }

    fn sender(
        &self,
        peer_id: Self::PeerId,
        protocol: Protocol,
    ) -> Result<Self::NetworkSender, Self::SenderError> {
        Ok(SubstrateNetworkSender {
            // Currently method `notification_sender` does not distinguish whether we are not connected to the peer
            // or there is no such protocol so we need to have this worthless `SenderError::CannotCreateSender` error here
            notification_sender: self
                .notification_sender(peer_id.into(), protocol_name(&protocol))
                .map_err(|_| SenderError::CannotCreateSender(peer_id, protocol))?,
            peer_id,
        })
    }

    fn add_reserved(&self, addresses: HashSet<Self::Multiaddress>, protocol: Protocol) {
        if let Err(e) = self.add_peers_to_reserved_set(
            protocol_name(&protocol),
            addresses
                .into_iter()
                .map(|address| address.into())
                .collect(),
        ) {
            error!(target: "aleph-network", "add_reserved failed: {}", e);
        }
    }

    fn remove_reserved(&self, peers: HashSet<Self::PeerId>, protocol: Protocol) {
        let addresses = peers.into_iter().map(|peer_id| peer_id.0).collect();
        self.remove_peers_from_reserved_set(protocol_name(&protocol), addresses);
    }
}

impl<B: Block, H: ExHashT> NetworkIdentity for Arc<NetworkService<B, H>> {
    type PeerId = PeerId;
    type Multiaddress = Multiaddress;

    fn identity(&self) -> (Vec<Self::Multiaddress>, Self::PeerId) {
        (
            self.external_addresses()
                .into_iter()
                .map(|address| address.into())
                .collect(),
            (*self.local_peer_id()).into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use codec::{Decode, Encode};

    use super::Multiaddress;
    use crate::network::Multiaddress as _;

    fn address(text: &str) -> Multiaddress {
        Multiaddress(text.parse().unwrap())
    }

    #[test]
    fn non_p2p_addresses_are_not_p2p() {
        assert!(address("/dns4/example.com/udt/sctp/5678")
            .get_peer_id()
            .is_none());
    }

    #[test]
    fn p2p_addresses_are_p2p() {
        assert!(address(
            "/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L"
        )
        .get_peer_id()
        .is_some());
    }

    #[test]
    fn non_p2p_address_matches_peer_id() {
        let address = address(
            "/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L",
        );
        let peer_id = address.get_peer_id().unwrap();
        let mut peerless_address = address.clone().0;
        peerless_address.pop();
        let peerless_address = Multiaddress(peerless_address);
        assert!(peerless_address.get_peer_id().is_none());
        assert_eq!(
            peerless_address.add_matching_peer_id(peer_id),
            Some(address),
        );
    }

    #[test]
    fn p2p_address_matches_own_peer_id() {
        let address = address(
            "/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L",
        );
        let peer_id = address.get_peer_id().unwrap();
        let expected_address = address.clone();
        assert_eq!(
            address.add_matching_peer_id(peer_id),
            Some(expected_address),
        );
    }

    #[test]
    fn p2p_address_does_not_match_other_peer_id() {
        let nonmatching_address = address(
            "/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L",
        );
        let peer_id = address("/dns4/peer.example.com/tcp/30333/p2p/12D3KooWFVXnvJdPuGnGYMPn5qLQAQYwmRBgo6SmEQsKZSrDoo2k").get_peer_id().unwrap();
        assert!(nonmatching_address.add_matching_peer_id(peer_id).is_none());
    }

    #[test]
    fn multiaddr_encode_decode() {
        let multiaddr: Multiaddress = address(
            "/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L",
        );
        assert_eq!(
            Multiaddress::decode(&mut &multiaddr.encode()[..]).unwrap(),
            multiaddr,
        );
    }
}
