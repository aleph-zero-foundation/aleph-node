//! Gossip validator which validates incoming messages with basic packet checks.
use crate::{
    communication::peer::{
        rep::{PeerGoodBehavior, PeerMisbehavior, Reputation},
        Peers,
    },
    temp::{NodeIndex, Unit, UnitCoord},
    AuthorityId, AuthoritySignature,
};
use codec::{Decode, Encode};
use log::debug;
use parking_lot::RwLock;
use prometheus_endpoint::{CounterVec, Opts, PrometheusError, Registry, U64};
use sc_network::{ObservedRole, PeerId, ReputationChange};
use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
use sc_telemetry::{telemetry, CONSENSUS_DEBUG};
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::traits::Block;
use sp_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

#[derive(Debug, PartialEq, Eq, Hash)]
/// As `PeerId` does not implement `Hash`, we need to turn it into bytes.
struct PeerIdBytes(Vec<u8>);

impl From<PeerId> for PeerIdBytes {
    fn from(peer_id: PeerId) -> Self {
        PeerIdBytes(peer_id.into_bytes())
    }
}

impl AsRef<[u8]> for PeerIdBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// A wrapped unit which contains both an authority public key and signature.
#[derive(Debug, Encode, Decode)]
pub(crate) struct SignedUnit<B: Block> {
    unit: Unit<B>,
    signature: AuthoritySignature,
    // NOTE: This will likely be changed to a usize to get the authority out of
    // a map in the future to reduce data sizes of packets.
    id: AuthorityId,
}

impl<B: Block> SignedUnit<B> {
    /// Encodes the unit with a buffer vector.
    pub(crate) fn encode_unit_with_buffer(&self, buf: &mut Vec<u8>) {
        buf.clear();
        self.unit.encode_to(buf);
    }

    /// Verifies the unit's signature with a buffer.
    pub(crate) fn verify_unit_signature_with_buffer(&self, buf: &mut Vec<u8>) -> bool {
        self.encode_unit_with_buffer(buf);

        let valid = self.id.verify(&buf, &self.signature);
        if !valid {
            debug!(target: "afa", "Bad signature message from {:?}", self.unit.creator);
        }

        valid
    }

    /// Verifies the unit's signature.
    pub(crate) fn verify_unit_signature(&self) -> bool {
        self.verify_unit_signature_with_buffer(&mut Vec::new())
    }
}

/// Actions for incoming messages.
#[derive(Debug)]
enum MessageAction<H> {
    /// Keeps the incoming message to be re-propagated.
    Keep(H, Reputation),
    /// Flags the message to be processed then discarded. It does not get
    /// re-propagated.
    ProcessAndDiscard(H, Reputation),
    /// Discards the incoming message usually because of some fault or
    /// violation.
    Discard(Reputation),
}

/// Multicast sends a message to all peers.
#[derive(Debug, Encode, Decode)]
pub(crate) struct Multicast<B: Block> {
    signed_unit: SignedUnit<B>,
}

/// A fetch request which asks for units from coordinates.
#[derive(Debug, Encode, Decode)]
pub(crate) struct FetchRequest {
    coords: Vec<UnitCoord>,
    peer_id: NodeIndex,
}

/// A fetch response which returns units from requested coordinates.
#[derive(Debug, Encode, Decode)]
pub(crate) struct FetchResponse<B: Block> {
    signed_units: Vec<SignedUnit<B>>,
    peer_id: NodeIndex,
}

// TODO
#[derive(Debug, Encode, Decode)]
struct Alert {}

/// The kind of message that is being sent.
#[derive(Debug, Encode, Decode)]
enum GossipMessage<B: Block> {
    /// A multicast message kind.
    Multicast(Multicast<B>),
    /// A fetch request message kind.
    FetchRequest(FetchRequest),
    /// A fetch response message kind.
    FetchResponse(FetchResponse<B>),
    /// An alert message kind.
    Alert(Alert),
}

/// Reports a peer with a reputation change.
pub(crate) struct PeerReport {
    who: PeerId,
    change: ReputationChange,
}

/// A prometheus result.
type PrometheusResult<T> = Result<T, PrometheusError>;

/// Metrics used for prometheus.
struct Metrics {
    messages_validated: CounterVec<U64>,
}

impl Metrics {
    /// Registers a prometheus end point.
    pub(crate) fn register(registry: &prometheus_endpoint::Registry) -> PrometheusResult<Self> {
        Ok(Self {
            messages_validated: prometheus_endpoint::register(
                CounterVec::new(
                    Opts::new(
                        "finality_aleph_communication_gossip_validator_messages",
                        "Number of messages validated by the finality aleph gossip validator.",
                    ),
                    &["message", "action"],
                )?,
                registry,
            )?,
        })
    }
}

/// A gossip validator which is used to validate incoming messages.
///
/// When we receive a message it is first checked here to see if it passes
/// basic validation rules that are not part of consensus but related to the
/// message itself.
pub(super) struct GossipValidator<B: Block> {
    peers: RwLock<Peers>,
    authority_set: RwLock<HashSet<AuthorityId>>,
    report_sender: TracingUnboundedSender<PeerReport>,
    metrics: Option<Metrics>,
    pending_requests: RwLock<HashSet<(PeerIdBytes, Vec<UnitCoord>)>>,
    phantom: PhantomData<B>,
}

impl<B: Block> GossipValidator<B> {
    /// Constructs a new gossip validator and unbounded `PeerReport` receiver
    /// channel with an optional prometheus registry.
    pub(crate) fn new(
        prometheus_registry: Option<&Registry>,
    ) -> (GossipValidator<B>, TracingUnboundedReceiver<PeerReport>) {
        let metrics: Option<Metrics> = prometheus_registry.and_then(|reg| {
            Metrics::register(reg)
                .map_err(|e| debug!(target: "afa", "Failed to register metrics: {:?}", e))
                .ok()
        });

        let (tx, rx) = tracing_unbounded("mpsc_aleph_gossip_validator");
        let val = GossipValidator {
            peers: RwLock::new(Peers::default()),
            authority_set: RwLock::new(HashSet::new()),
            report_sender: tx,
            metrics,
            pending_requests: RwLock::new(HashSet::new()),
            phantom: PhantomData::default(),
        };

        (val, rx)
    }

    /// Reports a peer with a reputation change.
    pub(crate) fn report_peer(&self, who: PeerId, change: ReputationChange) {
        let _ = self
            .report_sender
            .unbounded_send(PeerReport { who, change });
    }

    /// Notes pending fetch requests so that the gossip validator is aware of
    /// incoming fetch responses to watch out for.
    pub(crate) fn note_pending_fetch_request(&mut self, peer: PeerId, mut request: FetchRequest) {
        let mut pending_request = self.pending_requests.write();
        request.coords.sort();
        pending_request.insert((PeerIdBytes::from(peer), request.coords));
    }

    /// Sets the current authorities which are used to ensure that the incoming
    /// messages are indeed signed by these authorities.
    pub(crate) fn set_authorities<I>(&mut self, authorities: I)
    where
        I: IntoIterator<Item = AuthorityId>,
    {
        let mut old_authorities = self.authority_set.write();
        old_authorities.clear();
        old_authorities.extend(authorities.into_iter());
    }

    /// Removes a single authority in case they had been forked out.
    pub(crate) fn remove_authority(&mut self, authority: &AuthorityId) {
        let mut authorities = self.authority_set.write();
        authorities.remove(authority);
    }

    /// Validates a signed unit message.
    ///
    /// This first checks if the message came from a known authority from the
    /// authority set and if the signature is valid.
    fn validate_signed_unit(
        &self,
        signed_unit: &SignedUnit<B>,
    ) -> Result<(), MessageAction<B::Hash>> {
        let id = &signed_unit.id;
        if !self.authority_set.read().contains(id) {
            debug!(target: "afa", "Message from unknown authority: {}", id);
            return Err(MessageAction::Discard(PeerMisbehavior::UnknownVoter.into()));
        }

        if !signed_unit.verify_unit_signature() {
            debug!(target: "afa", "Bad message signature: {}", id);
            return Err(MessageAction::Discard(PeerMisbehavior::BadSignature.into()));
        }

        Ok(())
    }

    /// Validates a multicast message.
    ///
    /// It checks if the message is signed by a known authority in the current
    /// set as well as if the signature is valid.
    fn validate_multicast(&self, message: &Multicast<B>) -> MessageAction<B::Hash> {
        match self.validate_signed_unit(&message.signed_unit) {
            Ok(_) => {
                let topic: <B as Block>::Hash = super::multicast_topic::<B>(
                    message.signed_unit.unit.round,
                    message.signed_unit.unit.epoch_id,
                );
                MessageAction::Keep(topic, PeerGoodBehavior::Multicast.into())
            }
            Err(e) => e,
        }
    }

    /// Validates a fetch response.
    ///
    /// These messages must come from a peer that has the role of an authority,
    /// not necessarily part of the authority set. If the message is not what we
    /// requested it too is flagged. The signed unit is then checked to ensure
    /// that the authority is known and if the signature is valid.
    fn validate_fetch_response(
        &self,
        sender: &PeerId,
        message: &FetchResponse<B>,
    ) -> MessageAction<B::Hash> {
        if !self.peers.read().contains_authority(sender) {
            return MessageAction::Discard(PeerMisbehavior::NotAuthority.into());
        }

        let mut pending_requests = self.pending_requests.write();
        if pending_requests.len() != 0 {
            let mut coords = Vec::with_capacity(message.signed_units.len());
            for signed_unit in &message.signed_units {
                let unit = &signed_unit.unit;
                let coord: UnitCoord = unit.into();
                coords.push(coord);
            }
            coords.sort();
            let sender: PeerIdBytes = sender.clone().into();
            if !pending_requests.remove(&(sender, coords)) {
                return MessageAction::Discard(PeerMisbehavior::OutOfScopeResponse.into());
            }

            for signed_unit in &message.signed_units {
                if let Err(e) = self.validate_signed_unit(signed_unit) {
                    return e;
                }
            }

            let topic: <B as Block>::Hash = super::index_topic::<B>(message.peer_id);
            MessageAction::ProcessAndDiscard(topic, PeerGoodBehavior::FetchResponse.into())
        } else {
            MessageAction::Discard(Reputation::from(PeerMisbehavior::OutOfScopeResponse))
        }
    }

    // TODO: Rate limiting should be applied here. We would not want to let an unlimited amount of
    // requests. Though, it should be checked if this is already done on the other layers. Not to
    // my knowledge though.
    /// Validates a fetch request.
    ///
    /// These must come from a known peer with the role of a validator.
    fn validate_fetch_request(
        &self,
        sender: &PeerId,
        message: &FetchRequest,
    ) -> MessageAction<B::Hash> {
        if self.peers.read().contains_authority(sender) {
            let topic: <B as Block>::Hash = super::index_topic::<B>(message.peer_id);
            MessageAction::ProcessAndDiscard(topic, PeerGoodBehavior::FetchRequest.into())
        } else {
            MessageAction::Discard(PeerMisbehavior::NotAuthority.into())
        }
    }
}

impl<B: Block> Validator<B> for GossipValidator<B> {
    fn new_peer(&self, _context: &mut dyn ValidatorContext<B>, who: &PeerId, role: ObservedRole) {
        self.peers.write().insert(who.clone(), role);
    }

    fn peer_disconnected(&self, _context: &mut dyn ValidatorContext<B>, who: &PeerId) {
        self.peers.write().remove(who);
    }

    fn validate(
        &self,
        context: &mut dyn ValidatorContext<B>,
        sender: &PeerId,
        mut data: &[u8],
    ) -> ValidationResult<B::Hash> {
        let message_name: Option<&str>;
        let action = match GossipMessage::<B>::decode(&mut data) {
            Ok(GossipMessage::Multicast(ref message)) => {
                message_name = Some("multicast");
                self.validate_multicast(message)
            }
            Ok(GossipMessage::FetchRequest(ref message)) => {
                message_name = Some("fetch_request");
                self.validate_fetch_request(sender, message)
            }
            Ok(GossipMessage::FetchResponse(ref message)) => {
                message_name = Some("fetch_response");
                self.validate_fetch_response(sender, message)
            }
            Ok(GossipMessage::Alert(ref _message)) => {
                message_name = Some("fetch_response");
                todo!()
            }
            Err(e) => {
                message_name = None;
                debug!(target: "afa", "Error decoding message: {}", e.what());
                telemetry!(CONSENSUS_DEBUG; "afa.err_decoding_msg"; "" => "");

                let len = std::cmp::min(i32::max_value() as usize, data.len()) as i32;
                MessageAction::Discard(PeerMisbehavior::UndecodablePacket(len).into())
            }
        };

        if let (Some(metrics), Some(message_name)) = (&self.metrics, message_name) {
            let action_name = match action {
                MessageAction::Keep(_, _) => "keep",
                MessageAction::ProcessAndDiscard(_, _) => "process_and_discard",
                MessageAction::Discard(_) => "discard",
            };
            metrics
                .messages_validated
                .with_label_values(&[message_name, action_name])
                .inc();
        }

        match action {
            MessageAction::Keep(topic, rep_change) => {
                self.report_peer(sender.clone(), rep_change.change());
                context.broadcast_message(topic, data.to_vec(), false);
                ValidationResult::ProcessAndKeep(topic)
            }
            MessageAction::ProcessAndDiscard(topic, rep_change) => {
                self.report_peer(sender.clone(), rep_change.change());
                ValidationResult::ProcessAndDiscard(topic)
            }
            MessageAction::Discard(rep_change) => {
                self.report_peer(sender.clone(), rep_change.change());
                ValidationResult::Discard
            }
        }
    }

    fn message_expired(&self) -> Box<dyn FnMut(B::Hash, &[u8]) -> bool> {
        // We do not do anything special if a message expires.
        Box::new(move |_topic, _data| false)
    }

    fn message_allowed(&self) -> Box<dyn FnMut(&PeerId, MessageIntent, &B::Hash, &[u8]) -> bool> {
        // There should be epoch tracking somewhere. If the data is for a
        // previous epoch, deny.
        Box::new(move |_who, _intent, _topic, _data| true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        temp::{ControlHash, CreatorId, EpochId, NodeMap, Round, Unit},
        AuthorityPair, AuthoritySignature,
    };
    use sp_core::{Pair, H256};
    use sp_runtime::traits::Extrinsic as ExtrinsicT;

    #[derive(Debug, PartialEq, Clone, Eq, Encode, Decode, serde::Serialize)]
    pub struct Extrinsic {}

    parity_util_mem::malloc_size_of_is_0!(Extrinsic);

    impl ExtrinsicT for Extrinsic {
        type Call = Extrinsic;
        type SignaturePayload = ();
    }

    pub type BlockNumber = u64;

    pub type Hashing = sp_runtime::traits::BlakeTwo256;

    pub type Header = sp_runtime::generic::Header<BlockNumber, Hashing>;

    pub type Hash = H256;

    pub type Block = sp_runtime::generic::Block<Header, Extrinsic>;

    impl GossipValidator<Block> {
        fn new_dummy() -> Self {
            GossipValidator::<Block>::new(None).0
        }

        fn with_dummy_authorities(self, authorities: Vec<AuthorityId>) -> Self {
            self.authority_set.write().extend(authorities);
            self
        }

        fn with_dummy_peers(self, new_peers: Vec<(PeerId, ObservedRole)>) -> Self {
            let mut peers = self.peers.write();
            for (peer, role) in new_peers.into_iter() {
                peers.insert(peer, role);
            }
            drop(peers);
            self
        }
    }

    impl ControlHash<Hash> {
        fn new_dummy() -> Self {
            ControlHash {
                parents: NodeMap(vec![false]),
                hash: Hash::from([1u8; 32]),
            }
        }
    }

    impl Unit<Block> {
        fn new_dummy() -> Self {
            Unit {
                creator: CreatorId(0),
                round: Round(0),
                epoch_id: EpochId(0),
                hash: Hash::from([1u8; 32]),
                control_hash: ControlHash::new_dummy(),
                best_block: Hash::from([1u8; 32]),
            }
        }
    }

    impl SignedUnit<Block> {
        fn new_dummy() -> Self {
            SignedUnit {
                unit: Unit::new_dummy(),
                signature: AuthoritySignature::default(),
                id: AuthorityId::default(),
            }
        }
    }

    #[test]
    fn good_multicast() {
        let keypair = AuthorityPair::from_seed_slice(&[1u8; 32]).unwrap();
        let unit = Unit::new_dummy();
        let signature = keypair.sign(&unit.encode());
        let message = Multicast {
            signed_unit: SignedUnit {
                unit,
                signature,
                id: keypair.public(),
            },
        };
        let peer = PeerId::random();

        let val = GossipValidator::new_dummy()
            .with_dummy_authorities(vec![keypair.public()])
            .with_dummy_peers(vec![(peer, ObservedRole::Authority)]);

        let res = val.validate_multicast(&message);
        assert!(matches!(res, MessageAction::Keep(..)));
    }

    #[test]
    fn bad_signature_multicast() {
        let keypair = AuthorityPair::from_seed_slice(&[1u8; 32]).unwrap();
        let message: Multicast<Block> = Multicast {
            signed_unit: SignedUnit {
                id: keypair.public(),
                ..SignedUnit::new_dummy()
            },
        };

        let peer = PeerId::random();
        let val = GossipValidator::new_dummy()
            .with_dummy_authorities(vec![keypair.public()])
            .with_dummy_peers(vec![(peer, ObservedRole::Authority)]);

        let res = val.validate_multicast(&message);
        let _action: MessageAction<Hash> =
            MessageAction::Discard(PeerMisbehavior::BadSignature.into());
        assert!(matches!(res, _action));
    }

    #[test]
    fn unknown_authority_multicast() {
        let keypair = AuthorityPair::from_seed_slice(&[1u8; 32]).unwrap();
        let unit = Unit::new_dummy();
        let signature = keypair.sign(&unit.encode());
        let message: Multicast<Block> = Multicast {
            signed_unit: SignedUnit {
                unit,
                signature,
                ..SignedUnit::new_dummy()
            },
        };
        let peer = PeerId::random();
        let val = GossipValidator::new_dummy()
            .with_dummy_authorities(vec![AuthorityId::default()])
            .with_dummy_peers(vec![(peer, ObservedRole::Authority)]);

        let res = val.validate_multicast(&message);
        let _action: MessageAction<Hash> =
            MessageAction::Discard(PeerMisbehavior::UnknownVoter.into());
        assert!(matches!(res, _action));
    }

    #[test]
    fn good_fetch_response() {
        let keypair = AuthorityPair::from_seed_slice(&[1u8; 32]).unwrap();

        let mut coords: Vec<UnitCoord> = Vec::with_capacity(10);
        for x in 0..10 {
            let unit_coord = UnitCoord {
                creator: CreatorId(x),
                round: Round(x + 1),
            };
            coords.push(unit_coord);
        }

        let mut signed_units = Vec::with_capacity(10);
        for x in 0..10 {
            let unit: Unit<Block> = Unit {
                creator: CreatorId(x),
                round: Round(x + 1),
                ..Unit::new_dummy()
            };
            let signature = keypair.sign(&unit.encode());

            let signed_unit = SignedUnit {
                unit,
                signature,
                id: keypair.public(),
            };

            signed_units.push(signed_unit);
        }

        let fetch_response = FetchResponse {
            signed_units,
            peer_id: NodeIndex(0),
        };

        let fetch_request = FetchRequest {
            coords,
            peer_id: NodeIndex(0),
        };

        let peer = PeerId::random();
        let mut val = GossipValidator::new_dummy()
            .with_dummy_authorities(vec![keypair.public()])
            .with_dummy_peers(vec![(peer.clone(), ObservedRole::Authority)]);

        val.note_pending_fetch_request(peer.clone(), fetch_request);

        let res = val.validate_fetch_response(&peer, &fetch_response);
        assert!(matches!(res, MessageAction::ProcessAndDiscard(..)))
    }

    #[test]
    fn not_authority_fetch_response() {
        let keypair = AuthorityPair::from_seed_slice(&[1u8; 32]).unwrap();
        let authority_id = keypair.public();
        let mut signed_units = Vec::with_capacity(10);
        for x in 0..10 {
            let unit: Unit<Block> = Unit {
                creator: CreatorId(x),
                round: Round(x + 1),
                ..Unit::new_dummy()
            };
            let signature = keypair.sign(&unit.encode());

            let signed_unit = SignedUnit {
                unit,
                signature,
                id: authority_id.clone(),
            };

            signed_units.push(signed_unit);
        }
        let fetch_response = FetchResponse {
            signed_units,
            peer_id: NodeIndex(0),
        };

        let mut coords: Vec<UnitCoord> = Vec::with_capacity(10);
        for x in 0..10 {
            let unit_coord = UnitCoord {
                creator: CreatorId(x),
                round: Round(x + 1),
            };
            coords.push(unit_coord);
        }
        let fetch_request = FetchRequest {
            coords,
            peer_id: NodeIndex(0),
        };

        let peer = PeerId::random();
        let mut val = GossipValidator::new_dummy()
            .with_dummy_authorities(vec![authority_id])
            .with_dummy_peers(vec![(peer.clone(), ObservedRole::Full)]);

        val.note_pending_fetch_request(peer.clone(), fetch_request);

        let res = val.validate_fetch_response(&peer, &fetch_response);
        let _action: MessageAction<Hash> =
            MessageAction::Discard(PeerMisbehavior::NotAuthority.into());
        assert!(matches!(res, _action));
    }

    #[test]
    fn bad_signature_fetch_response() {
        let mut coords: Vec<UnitCoord> = Vec::with_capacity(10);
        for x in 0..10 {
            let unit_coord = UnitCoord {
                creator: CreatorId(x),
                round: Round(x + 1),
            };
            coords.push(unit_coord);
        }

        let mut signed_units = Vec::with_capacity(10);
        for x in 0..10 {
            let unit: Unit<Block> = Unit {
                creator: CreatorId(x),
                round: Round(x + 1),
                ..Unit::new_dummy()
            };

            let signed_unit = SignedUnit {
                unit,
                ..SignedUnit::new_dummy()
            };

            signed_units.push(signed_unit);
        }

        let fetch_request = FetchRequest {
            coords,
            peer_id: NodeIndex(0),
        };

        let fetch_response = FetchResponse {
            signed_units,
            peer_id: Default::default(),
        };

        let peer = PeerId::random();
        let mut val = GossipValidator::new_dummy()
            .with_dummy_authorities(vec![AuthorityId::default()])
            .with_dummy_peers(vec![(peer.clone(), ObservedRole::Authority)]);

        val.note_pending_fetch_request(peer.clone(), fetch_request);

        let res = val.validate_fetch_response(&peer, &fetch_response);
        let _action: MessageAction<Hash> =
            MessageAction::Discard(PeerMisbehavior::BadSignature.into());
        assert!(matches!(res, _action));
    }

    #[test]
    fn unknown_authority_fetch_response() {
        let keypair = AuthorityPair::from_seed_slice(&[1u8; 32]).unwrap();

        let mut coords: Vec<UnitCoord> = Vec::with_capacity(10);
        for x in 0..10 {
            let unit_coord = UnitCoord {
                creator: CreatorId(x),
                round: Round(x + 1),
            };
            coords.push(unit_coord);
        }

        let mut signed_units = Vec::with_capacity(10);
        for x in 0..10 {
            let unit: Unit<Block> = Unit {
                creator: CreatorId(x),
                round: Round(x + 2),
                ..Unit::new_dummy()
            };
            let signature = keypair.sign(&unit.encode());

            let signed_unit = SignedUnit {
                unit,
                signature,
                id: keypair.public(),
            };

            signed_units.push(signed_unit);
        }

        let fetch_request = FetchRequest {
            coords,
            peer_id: NodeIndex(0),
        };

        let fetch_response = FetchResponse {
            signed_units,
            peer_id: Default::default(),
        };

        let peer = PeerId::random();
        let mut val = GossipValidator::new_dummy()
            .with_dummy_peers(vec![(peer.clone(), ObservedRole::Authority)]);

        val.note_pending_fetch_request(peer.clone(), fetch_request);

        let res = val.validate_fetch_response(&peer, &fetch_response);
        let _action: MessageAction<Hash> =
            MessageAction::Discard(PeerMisbehavior::UnknownVoter.into());
        assert!(matches!(res, _action));
    }

    // TODO: Once the fetch request has a bit more logic in it, there needs to
    // be a test for it.
    #[test]
    fn good_fetch_request() {
        let fetch_request = FetchRequest {
            coords: Vec::new(),
            peer_id: NodeIndex(0),
        };

        let peer = PeerId::random();
        let val = GossipValidator::new_dummy()
            .with_dummy_peers(vec![(peer.clone(), ObservedRole::Authority)]);

        let res = val.validate_fetch_request(&peer, &fetch_request);
        assert!(matches!(res, MessageAction::ProcessAndDiscard(..)))
    }

    #[test]
    fn not_authority_request() {
        let fetch_request = FetchRequest {
            coords: Vec::new(),
            peer_id: NodeIndex(0),
        };

        let peer = PeerId::random();
        let val =
            GossipValidator::new_dummy().with_dummy_peers(vec![(peer.clone(), ObservedRole::Full)]);

        let res = val.validate_fetch_request(&peer, &fetch_request);
        let _action: MessageAction<Hash> =
            MessageAction::Discard(PeerMisbehavior::NotAuthority.into());
        assert!(matches!(res, _action))
    }
}
