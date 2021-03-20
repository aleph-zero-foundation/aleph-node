use crate::{
    communication::{
        epoch_topic,
        gossip::{FetchRequest, GossipMessage, GossipValidator, Multicast, PeerReport},
        request_topic,
    },
    config::Config,
    hash::Hash,
    AuthorityCryptoStore, UnitCoord,
};
use codec::{Decode, Encode};
use futures::{channel::mpsc, prelude::*};
use log::{debug, trace};
use parking_lot::Mutex;
use prometheus_endpoint::Registry;
use rush::{EpochId, NotificationIn, NotificationOut};
use sc_network::PeerId;
use sc_network_gossip::{GossipEngine, Network as GossipNetwork};
use sp_runtime::{traits::Block, ConsensusEngineId};
use sp_utils::mpsc::TracingUnboundedReceiver;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/// Name of the notifications protocol used by Aleph Zero. This is how messages
/// are subscribed to to ensure that we are gossiping and communicating with our
/// own network.
pub(crate) const ALEPH_PROTOCOL_NAME: &str = "/cardinals/aleph/1";

pub(crate) const ALEPH_ENGINE_ID: ConsensusEngineId = *b"ALPH";

pub(crate) trait Network<B: Block>: GossipNetwork<B> + Clone + Send + 'static {}

pub struct NotificationOutSender<B: Block, H: Hash> {
    network: Arc<Mutex<GossipEngine<B>>>,
    sender: mpsc::Sender<NotificationIn<B::Hash, H>>,
    epoch_id: EpochId,
    auth_cryptostore: AuthorityCryptoStore,
}

unsafe impl<B: Block, H: Hash> Send for NotificationOutSender<B, H> {}

impl<B: Block, H: Hash> Sink<NotificationOut<B::Hash, H>> for NotificationOutSender<B, H> {
    // TODO! error
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: NotificationOut<B::Hash, H>,
    ) -> Result<(), Self::Error> {
        return match item {
            NotificationOut::CreatedUnit(u) => {
                let signed_unit = match super::gossip::sign_unit::<B, H>(&self.auth_cryptostore, u)
                {
                    Some(s) => s,
                    None => return Err(()),
                };
                let message = GossipMessage::Multicast(Multicast {
                    signed_unit: signed_unit.clone(),
                });

                let topic: <B as Block>::Hash = super::epoch_topic::<B>(self.epoch_id);
                self.network
                    .lock()
                    .gossip_message(topic, message.encode(), false);

                let notification = NotificationIn::NewUnits(vec![signed_unit.unit]);
                self.sender.start_send(notification).map_err(|_e| ())
            }
            NotificationOut::MissingUnits(coords, aux) => {
                let n_coords = {
                    let mut n_coords: Vec<UnitCoord> = Vec::with_capacity(coords.len());
                    for coord in coords {
                        n_coords.push(coord.into());
                    }
                    n_coords
                };
                let message: GossipMessage<B, H> = GossipMessage::FetchRequest(FetchRequest {
                    coords: n_coords,
                    peer_id: aux.child_creator(),
                });

                debug!(target: "afa", "Sending out message to our peers for epoch {}", self.epoch_id.0);
                let topic: <B as Block>::Hash = super::request_topic::<B>();
                self.network
                    .lock()
                    .gossip_message(topic, message.encode(), false);

                Ok(())
            }
        };
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Sink::poll_close(Pin::new(&mut self.sender), cx).map(|elem| elem.map_err(|_e| ()))
    }
}

pub(crate) struct NetworkBridge<B: Block, H, N: Network<B>> {
    network_service: N,
    gossip_engine: Arc<Mutex<GossipEngine<B>>>,
    gossip_validator: Arc<GossipValidator<B, H>>,
    peer_report_handle: Arc<Mutex<TracingUnboundedReceiver<PeerReport>>>,
}

impl<B: Block, H: Hash, N: Network<B>> NetworkBridge<B, H, N> {
    pub(crate) fn new(network_service: N, _config: Config, registry: Option<&Registry>) -> Self {
        let (gossip_validator, peer_report_handle) = {
            let (validator, peer_report_handle) = GossipValidator::<B, H>::new(registry);
            let validator = Arc::new(validator);
            let peer_report_handle = Arc::new(Mutex::new(peer_report_handle));
            (validator, peer_report_handle)
        };
        let gossip_engine = Arc::new(Mutex::new(GossipEngine::new(
            network_service.clone(),
            ALEPH_ENGINE_ID,
            ALEPH_PROTOCOL_NAME,
            gossip_validator.clone(),
        )));

        NetworkBridge {
            network_service,
            gossip_engine,
            gossip_validator,
            peer_report_handle,
        }
    }

    pub(crate) fn note_pending_fetch_request(&mut self, peer: PeerId, fetch_request: FetchRequest) {
        self.gossip_validator
            .note_pending_fetch_request(peer, fetch_request)
    }

    // TODO: keystore should be optional later.
    pub(crate) fn communication(
        &self,
        epoch_id: EpochId,
        auth_cryptostore: AuthorityCryptoStore,
    ) -> (
        impl Stream<Item = NotificationIn<B::Hash, H>> + Unpin,
        impl Sink<NotificationOut<B::Hash, H>> + Unpin,
    ) {
        let topic = epoch_topic::<B>(epoch_id);
        let gossip_engine = self.gossip_engine.clone();

        let incoming_units = gossip_engine
            .lock()
            .messages_for(topic)
            .filter_map(move |notification| {
                let decoded = GossipMessage::<B, H>::decode(&mut &notification.message[..]);
                if let Ok(message) = decoded {
                    let notification = match message {
                        GossipMessage::Multicast(m) => {
                            let s_unit = m.signed_unit;
                            Some(NotificationIn::NewUnits(vec![s_unit.unit]))
                        }
                        GossipMessage::FetchResponse(m) => {
                            let mut units = Vec::with_capacity(m.signed_units.len());
                            for s_unit in m.signed_units {
                                units.push(s_unit.unit);
                            }
                            Some(NotificationIn::NewUnits(units))
                        }
                        _ => None,
                    };
                    futures::future::ready(notification)
                } else {
                    // NOTE: This should be unreachable due to the validator.
                    trace!(target: "afa", "Skipping malformed incoming message: {:?}", notification);
                    futures::future::ready(None)
                }
            });

        let request_topic = request_topic::<B>();
        let incoming_requests = gossip_engine
            .lock()
            .messages_for(request_topic)
            .filter_map(move |notification| {
                let decoded = GossipMessage::<B, H>::decode(&mut &notification.message[..]);
                if let Ok(message) = decoded {
                    let notification = match message {
                        GossipMessage::FetchRequest(_m) => {
                            todo!()
                        }
                        _ => None,
                    };
                    futures::future::ready(notification)
                } else {
                    // NOTE: This should be unreachable due to the validator.
                    trace!(target: "afa", "Skipping malformed incoming message: {:?}", notification);
                    futures::future::ready(None)
                }
            });

        let (tx, rx) = mpsc::channel(0);
        let outgoing = NotificationOutSender::<B, H> {
            network: self.gossip_engine.clone(),
            sender: tx,
            epoch_id,
            auth_cryptostore,
        };

        // NOTE: From how I understand this code and documentation, this should
        // be ok. If you whatever reason we are getting no incoming, this might
        // be the culprit.
        let external_incoming = stream::select(incoming_units, incoming_requests);
        let incoming = stream::select(external_incoming, rx);

        (incoming, outgoing)
    }
}
