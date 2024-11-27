use std::{
    borrow::{Borrow, BorrowMut},
    collections::HashSet,
    fmt::{Debug, Display, Error as FmtError, Formatter},
};

use log::{debug, info, trace, warn};
use parity_scale_codec::DecodeAll;
use rand::{seq::IteratorRandom, thread_rng};
pub use sc_network::PeerId;
use sc_network::{
    service::traits::{NotificationEvent as SubstrateEvent, ValidationResult},
    ProtocolName,
};
use tokio::time;

use crate::{
    network::{Data, GossipNetwork, LOG_TARGET},
    STATUS_REPORT_INTERVAL,
};

pub type BoxedNotificationService = Box<dyn sc_network::config::NotificationService>;

/// A thin wrapper around sc_network::config::NotificationService that stores a list
/// of all currently connected peers, and introduces a few convenience methods to
/// allow broadcasting messages and sending data to random peers.
pub struct ProtocolNetwork {
    service: BoxedNotificationService,
    connected_peers: HashSet<PeerId>,
    last_status_report: time::Instant,
}

impl Borrow<BoxedNotificationService> for ProtocolNetwork {
    fn borrow(&self) -> &BoxedNotificationService {
        &self.service
    }
}

impl BorrowMut<BoxedNotificationService> for ProtocolNetwork {
    fn borrow_mut(&mut self) -> &mut BoxedNotificationService {
        &mut self.service
    }
}

impl ProtocolNetwork {
    pub fn new(service: BoxedNotificationService) -> Self {
        Self {
            service,
            connected_peers: HashSet::new(),
            last_status_report: time::Instant::now(),
        }
    }

    pub fn name(&self) -> ProtocolName {
        self.service.protocol().clone()
    }

    fn random_peer<'a>(&'a self, peer_ids: &'a HashSet<PeerId>) -> Option<&'a PeerId> {
        peer_ids
            .intersection(&self.connected_peers)
            .choose(&mut thread_rng())
            .or_else(|| self.connected_peers.iter().choose(&mut thread_rng()))
    }

    fn handle_network_event(&mut self, event: SubstrateEvent) -> Option<(Vec<u8>, PeerId)> {
        use SubstrateEvent::*;
        match event {
            ValidateInboundSubstream {
                peer: _,
                handshake: _,
                result_tx,
            } => {
                let _ = result_tx.send(ValidationResult::Accept);
                None
            }
            NotificationStreamOpened { peer, .. } => {
                self.connected_peers.insert(peer);
                None
            }
            NotificationStreamClosed { peer } => {
                self.connected_peers.remove(&peer);
                None
            }
            NotificationReceived { peer, notification } => Some((notification, peer)),
        }
    }

    fn status_report(&self) {
        let mut status = String::from("Network status report: ");
        status.push_str(&format!(
            "{} connected peers - {:?}; ",
            self.service.protocol(),
            self.connected_peers.len()
        ));
        info!(target: LOG_TARGET, "{}", status);
    }
}

#[derive(Debug)]
pub enum ProtocolNetworkError {
    NetworkStreamTerminated,
}

impl Display for ProtocolNetworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        match self {
            ProtocolNetworkError::NetworkStreamTerminated => {
                write!(f, "Notifications event stream ended.")
            }
        }
    }
}

#[async_trait::async_trait]
impl<D: Data> GossipNetwork<D> for ProtocolNetwork {
    type Error = ProtocolNetworkError;
    type PeerId = PeerId;

    fn send_to(&mut self, data: D, peer_id: PeerId) -> Result<(), Self::Error> {
        trace!(
            target: LOG_TARGET,
            "Sending block sync data to peer {:?}.",
            peer_id,
        );
        self.service.send_sync_notification(&peer_id, data.encode());
        Ok(())
    }

    fn send_to_random(&mut self, data: D, peer_ids: HashSet<PeerId>) -> Result<(), Self::Error> {
        trace!(
            target: LOG_TARGET,
            "Sending data to random peer among {:?}.",
            peer_ids,
        );
        let peer_id = match self.random_peer(&peer_ids) {
            Some(peer_id) => *peer_id,
            None => {
                debug!(
                    target: LOG_TARGET,
                    "Failed to send message to random peer, no peers are available."
                );
                return Ok(());
            }
        };
        self.send_to(data, peer_id)
    }

    fn broadcast(&mut self, data: D) -> Result<(), Self::Error> {
        for peer in self.connected_peers.clone() {
            // in the current version send_to never returns an error
            let _ = self.send_to(data.clone(), peer);
        }
        Ok(())
    }

    async fn next(&mut self) -> Result<(D, PeerId), Self::Error> {
        let mut status_ticker = time::interval_at(
            self.last_status_report
                .checked_add(STATUS_REPORT_INTERVAL)
                .unwrap_or(time::Instant::now()),
            STATUS_REPORT_INTERVAL,
        );
        loop {
            tokio::select! {
                maybe_event = self.service.next_event() => {
                    let event = maybe_event.ok_or(Self::Error::NetworkStreamTerminated)?;
                    let Some((message, peer_id)) = self.handle_network_event(event) else { continue };
                    match D::decode_all(&mut &message[..]) {
                        Ok(message) => return Ok((message, peer_id)),
                        Err(e) => {
                            warn!(
                                target: LOG_TARGET,
                                "Error decoding message: {}", e
                            )
                        },
                    }
                },
                _ = status_ticker.tick() => {
                    self.status_report();
                    self.last_status_report = time::Instant::now();
                },
            }
        }
    }
}
