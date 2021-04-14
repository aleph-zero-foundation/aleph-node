//! Gossip validator which validates incoming messages with basic packet checks.
use crate::{
    communication::{
        dummy_topic,
        peer::{
            rep::{PeerGoodBehavior, PeerMisbehavior, Reputation},
            Peers,
        },
    },
    hash::Hash,
    AuthorityId, AuthorityKeystore, AuthoritySignature, UnitCoord,
};
use codec::{Decode, Encode};
use log::debug;
use parking_lot::RwLock;
use prometheus_endpoint::{CounterVec, Opts, PrometheusError, Registry, U64};
use rush::{PreUnit};
use sc_network::{ObservedRole, PeerId, ReputationChange};
use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
use sc_telemetry::{telemetry, CONSENSUS_DEBUG};
use sp_application_crypto::RuntimeAppPublic;

use sp_runtime::traits::Block;
use sp_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use std::{collections::HashSet, marker::PhantomData};

/// A wrapped unit which contains both an authority public key and signature.

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct FullUnit<B: Block, H: Hash> {
    pub(crate) inner: PreUnit<H>,
    pub(crate) block_hash: B::Hash,
}

#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct SignedUnit<B: Block, H: Hash> {
    pub(crate) unit: FullUnit<B, H>,
    signature: AuthoritySignature,
    // TODO: This *must* be changed ASAP to NodeIndex to reduce data size of packets.
    id: AuthorityId,
}

impl<B: Block, H: Hash> SignedUnit<B, H> {
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
            debug!(target: "afa", "Bad signature message from {:?}", self.unit.inner.creator());
        }

        valid
    }

    /// Verifies the unit's signature.
    pub(crate) fn verify_unit_signature(&self) -> bool {
        self.verify_unit_signature_with_buffer(&mut Vec::new())
    }

    pub(crate) fn hash(&self, hashing: impl Fn(&[u8]) -> H) -> H {
        hashing(&self.unit.encode())
    }
}

pub(crate) fn sign_unit<B: Block, H: Hash>(
    auth_crypto_store: &AuthorityKeystore,
    unit: FullUnit<B, H>,
) -> SignedUnit<B, H> {
    let encoded = unit.encode();
    let signature = auth_crypto_store.sign(&encoded[..]);

    SignedUnit {
        unit,
        signature,
        id: auth_crypto_store.authority_id.clone(),
    }
}

/// Actions for incoming messages.
#[derive(Debug)]
enum MessageAction<H> {
    /// Flags the message to be processed then discarded. It does not get
    /// re-propagated.
    ProcessAndDiscard(H, Reputation),
    /// Discards the incoming message usually because of some fault or
    /// violation.
    Discard(Reputation),
}

/// Multicast sends a message to all peers.
#[derive(Debug, Encode, Decode)]
pub(crate) struct Multicast<B: Block, H: Hash> {
    pub(crate) signed_unit: SignedUnit<B, H>,
}

/// A fetch request which asks for units from coordinates.
#[derive(Debug, Encode, Decode)]
pub(crate) struct FetchRequest {
    pub(crate) coord: UnitCoord,
}

/// A fetch response which returns units from requested coordinates.
#[derive(Debug, Encode, Decode)]
pub(crate) struct FetchResponse<B: Block, H: Hash> {
    pub(crate) signed_unit: SignedUnit<B, H>,
}

/// The kind of message that is being sent.
#[derive(Debug, Encode, Decode)]
pub(crate) enum GossipMessage<B: Block, H: Hash> {
    /// A multicast message kind.
    Multicast(Multicast<B, H>),
    /// A fetch request message kind.
    FetchRequest(FetchRequest),
    /// A fetch response message kind.
    FetchResponse(FetchResponse<B, H>),
}

/// Reports a peer with a reputation change.
pub(crate) struct PeerReport {
    pub(crate) who: PeerId,
    pub(crate) change: ReputationChange,
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
pub struct GossipValidator<B: Block, H: rush::HashT> {
    peers: RwLock<Peers>,
    authority_set: RwLock<HashSet<AuthorityId>>,
    report_sender: TracingUnboundedSender<PeerReport>,
    metrics: Option<Metrics>,
    pending_requests: RwLock<HashSet<(PeerId, UnitCoord)>>,
    block_phantom: PhantomData<B>,
    hash_phantom: PhantomData<H>,
}

impl<B: Block, H: rush::HashT + Hash> GossipValidator<B, H> {
    /// Constructs a new gossip validator and unbounded `PeerReport` receiver
    /// channel with an optional prometheus registry.
    pub(crate) fn new(
        prometheus_registry: Option<&Registry>,
    ) -> (GossipValidator<B, H>, TracingUnboundedReceiver<PeerReport>) {
        let metrics: Option<Metrics> = prometheus_registry.and_then(|reg| {
            Metrics::register(reg)
                .map_err(|e| debug!(target: "afa", "Failed to register metrics: {:?}", e))
                .ok()
        });

        let (tx_report, rx_report) = tracing_unbounded("mpsc_aleph_gossip_validator");
        let val = GossipValidator {
            peers: RwLock::new(Peers::default()),
            authority_set: RwLock::new(HashSet::new()),
            report_sender: tx_report,
            metrics,
            pending_requests: RwLock::new(HashSet::new()),
            block_phantom: PhantomData::default(),
            hash_phantom: PhantomData::default(),
        };

        (val, rx_report)
    }

    /// Reports a peer with a reputation change.
    pub(crate) fn report_peer(&self, who: PeerId, change: ReputationChange) {
        let _ = self
            .report_sender
            .unbounded_send(PeerReport { who, change });
    }

    /// Notes pending fetch requests so that the gossip validator is aware of
    /// incoming fetch responses to watch out for.
    pub(crate) fn note_pending_fetch_request(&self, peer: PeerId, coord: UnitCoord) {
        let mut pending_request = self.pending_requests.write();
        pending_request.insert((peer, coord));
    }

    /// Sets the current authorities which are used to ensure that the incoming
    /// messages are indeed signed by these authorities.
    pub(crate) fn set_authorities<I>(&self, authorities: I)
    where
        I: IntoIterator<Item = AuthorityId>,
    {
        let mut old_authorities = self.authority_set.write();
        old_authorities.clear();
        old_authorities.extend(authorities.into_iter());
    }

    /// Removes a single authority in case they had been forked out.
    pub(crate) fn _remove_authority(&self, authority: &AuthorityId) {
        let mut authorities = self.authority_set.write();
        authorities.remove(authority);
    }

    /// Validates a signed unit message.
    ///
    /// This first checks if the message came from a known authority from the
    /// authority set and if the signature is valid.
    fn validate_signed_unit(
        &self,
        signed_unit: &SignedUnit<B, H>,
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
    fn validate_multicast(&self, message: &Multicast<B, H>) -> MessageAction<B::Hash> {
        match self.validate_signed_unit(&message.signed_unit) {
            Ok(_) => MessageAction::ProcessAndDiscard(
                dummy_topic::<B>(),
                PeerGoodBehavior::Multicast.into(),
            ),
            Err(e) => {
                debug!(target: "afa", "Validation error: {:?}", e);
                e
            }
        }
    }

    fn validate_fetch_response(
        &self,
        sender: &PeerId,
        message: &FetchResponse<B, H>,
    ) -> MessageAction<B::Hash> {
        let unit = &message.signed_unit.unit.inner;
        let coord: UnitCoord = (unit.round(), unit.creator()).into();
        debug!(target: "afa", "Validating fetch response: {:?} from {:?}", message, sender);
        if !self.pending_requests.write().remove(&(*sender, coord)) {
            // This means that this is a response to a request we did not send.
            // Might also happen in rare cases that there were multiple requests and multiple responses from the same peer.
            // This check is not strictly necessary to have, but in the future we might need it for rate limiting.
            return MessageAction::Discard(PeerMisbehavior::OutOfScopeResponse.into());
        }

        if let Err(e) = self.validate_signed_unit(&message.signed_unit) {
            return e;
        }
        debug!(target: "afa", "Fetch response validated succesfully.");
        MessageAction::ProcessAndDiscard(dummy_topic::<B>(), PeerGoodBehavior::FetchResponse.into())
    }

    // TODO: Rate limiting should be applied here. We would not want to let an unlimited amount of
    // requests. Though, it should be checked if this is already done on the other layers. Not to
    // my knowledge though.
    /// Validates a fetch request.
    fn validate_fetch_request(
        &self,
        _sender: &PeerId,
        _message: &FetchRequest,
    ) -> MessageAction<B::Hash> {
        debug!(target: "afa", "Validating fetch request: {:?} from {:?}", _message, _sender);
        let topic: <B as Block>::Hash = dummy_topic::<B>();
        MessageAction::ProcessAndDiscard(topic, PeerGoodBehavior::FetchRequest.into())
    }

    pub(crate) fn get_random_peer(&self) -> Option<PeerId> {
        self.peers.read().sample_random()
    }
}

impl<B: Block, H: Hash> Validator<B> for GossipValidator<B, H> {
    fn new_peer(&self, _context: &mut dyn ValidatorContext<B>, who: &PeerId, role: ObservedRole) {
        self.peers.write().insert(*who, role);
    }

    fn peer_disconnected(&self, _context: &mut dyn ValidatorContext<B>, who: &PeerId) {
        self.peers.write().remove(who);
    }

    fn validate(
        &self,
        _context: &mut dyn ValidatorContext<B>,
        sender: &PeerId,
        mut data: &[u8],
    ) -> ValidationResult<B::Hash> {
        let message_name: Option<&str>;
        let action = match GossipMessage::<B, H>::decode(&mut data) {
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
            Err(e) => {
                message_name = None;
                debug!(target: "afa", "Error decoding message: {}", e);
                telemetry!(CONSENSUS_DEBUG; "afa.err_decoding_msg"; "" => "");

                let len = std::cmp::min(i32::max_value() as usize, data.len()) as i32;
                MessageAction::Discard(PeerMisbehavior::UndecodablePacket(len).into())
            }
        };

        if let (Some(metrics), Some(message_name)) = (&self.metrics, message_name) {
            let action_name = match action {
                MessageAction::ProcessAndDiscard(_, _) => "process_and_discard",
                MessageAction::Discard(_) => "discard",
            };
            metrics
                .messages_validated
                .with_label_values(&[message_name, action_name])
                .inc();
        }

        match action {
            MessageAction::ProcessAndDiscard(topic, rep_change) => {
                self.report_peer(*sender, rep_change.change());
                ValidationResult::ProcessAndDiscard(topic)
            }
            MessageAction::Discard(rep_change) => {
                self.report_peer(*sender, rep_change.change());
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
    use crate::{AuthorityPair, AuthoritySignature};
    use rush::{nodes::NodeIndex, ControlHash};
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


    impl GossipValidator<Block, Hash> {
        fn new_dummy() -> Self {
            GossipValidator::<Block, Hash>::new(None).0
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

    impl FullUnit<Block, Hash> {
        fn new_dummy() -> Self {
            FullUnit {
                inner: PreUnit::default(),
                block_hash: Hash::default(),
            }
        }
    }

    impl SignedUnit<Block, Hash> {
        fn new_dummy() -> Self {
            SignedUnit {
                unit: FullUnit::new_dummy(),
                signature: AuthoritySignature::default(),
                id: AuthorityId::default(),
            }
        }
    }

    #[test]
    fn good_multicast() {
        let keypair = AuthorityPair::from_seed_slice(&[1u8; 32]).unwrap();
        let unit = FullUnit::new_dummy();
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
        assert!(matches!(res, MessageAction::ProcessAndDiscard(..)));
    }

    #[test]
    fn bad_signature_multicast() {
        let keypair = AuthorityPair::from_seed_slice(&[1u8; 32]).unwrap();
        let message: Multicast<Block, Hash> = Multicast {
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
        let unit = FullUnit::new_dummy();
        let signature = keypair.sign(&unit.encode());
        let message: Multicast<Block, Hash> = Multicast {
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

        for x in 0..10 {
            let coord: UnitCoord = UnitCoord {
                creator: NodeIndex(x),
                round: (x + 1) as u64,
            };
            let unit = PreUnit::new(
                NodeIndex(x),
                (x + 1) as usize,
                ControlHash::default(),
            );

            let unit = FullUnit {inner: unit, block_hash: Hash::default()};

            let signature = keypair.sign(&unit.encode());

            let signed_unit = SignedUnit {
                unit,
                signature,
                id: keypair.public(),
            };

            let fetch_response = FetchResponse { signed_unit };

            let peer = PeerId::random();
            let val = GossipValidator::new_dummy()
                .with_dummy_authorities(vec![keypair.public()])
                .with_dummy_peers(vec![(peer, ObservedRole::Authority)]);

            val.note_pending_fetch_request(peer, coord);

            let res = val.validate_fetch_response(&peer, &fetch_response);
            debug!("res {:?}", res);
            assert!(matches!(res, MessageAction::ProcessAndDiscard(..)))
        }
    }

    #[test]
    fn bad_signature_fetch_response() {
        for x in 0..10 {
            let coord = UnitCoord {
                creator: NodeIndex(x),
                round: (x + 1) as u64,
            };

            let unit = PreUnit::new(
                NodeIndex(x),
                (x + 1) as usize,
                ControlHash::default(),
            );

            let unit = FullUnit {inner: unit, block_hash: Hash::default()};

            let signed_unit = SignedUnit {
                unit,
                ..SignedUnit::new_dummy()
            };

            let fetch_response = FetchResponse { signed_unit };

            let peer = PeerId::random();
            let val = GossipValidator::new_dummy()
                .with_dummy_authorities(vec![AuthorityId::default()])
                .with_dummy_peers(vec![(peer, ObservedRole::Authority)]);

            val.note_pending_fetch_request(peer, coord);

            let res = val.validate_fetch_response(&peer, &fetch_response);
            let _action: MessageAction<Hash> =
                MessageAction::Discard(PeerMisbehavior::BadSignature.into());
            assert!(matches!(res, _action));
        }
    }

    // TODO: Once the fetch request has a bit more logic in it, there needs to
    // be a test for it.
    #[test]
    fn good_fetch_request() {

        let coord = UnitCoord {
            creator: NodeIndex(0),
            round: 0_u64,
        };
        let fetch_request = FetchRequest { coord };

        let peer = PeerId::random();
        let val =
            GossipValidator::new_dummy().with_dummy_peers(vec![(peer, ObservedRole::Authority)]);

        let res = val.validate_fetch_request(&peer, &fetch_request);
        assert!(matches!(res, MessageAction::ProcessAndDiscard(..)))
    }
}
