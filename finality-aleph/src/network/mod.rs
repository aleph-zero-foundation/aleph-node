use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    hash::Hash,
};

use codec::Codec;
use sp_api::NumberFor;
use sp_runtime::traits::Block;

pub mod data;
mod gossip;
mod io;
mod manager;
#[cfg(test)]
pub mod mock;
mod session;
mod substrate;

pub use gossip::{Network as GossipNetwork, Protocol, Service as GossipService};
pub use io::setup as setup_io;
use manager::SessionCommand;
pub use manager::{
    ConnectionIO as ConnectionManagerIO, ConnectionManager, ConnectionManagerConfig,
};
pub use session::{Manager as SessionManager, ManagerError, SessionSender, IO as SessionManagerIO};
pub use substrate::protocol_name;
#[cfg(test)]
pub mod testing {
    use super::manager::LegacyAuthentication;
    pub use super::{
        gossip::mock::{MockEvent, MockRawNetwork},
        manager::{
            Authentication, DataInSession, DiscoveryMessage, LegacyDiscoveryMessage,
            PeerAuthentications, SessionHandler, VersionedAuthentication,
        },
    };
    use crate::testing::mocks::validator_network::MockAddressingInformation;

    pub fn legacy_authentication(
        handler: &SessionHandler<MockAddressingInformation, MockAddressingInformation>,
    ) -> LegacyAuthentication<MockAddressingInformation> {
        match handler
            .authentication()
            .expect("this is a validator handler")
        {
            PeerAuthentications::Both(_, authentication) => authentication,
            _ => panic!("handler doesn't have both authentications"),
        }
    }

    pub fn authentication(
        handler: &SessionHandler<MockAddressingInformation, MockAddressingInformation>,
    ) -> Authentication<MockAddressingInformation> {
        match handler
            .authentication()
            .expect("this is a validator handler")
        {
            PeerAuthentications::Both(authentication, _) => authentication,
            _ => panic!("handler doesn't have both authentications"),
        }
    }
}

/// Represents the id of an arbitrary node.
pub trait PeerId: PartialEq + Eq + Clone + Debug + Display + Hash + Codec + Send {
    /// This function is used for logging. It implements a shorter version of `to_string` for ids implementing display.
    fn to_short_string(&self) -> String {
        let id = format!("{}", self);
        if id.len() <= 12 {
            return id;
        }

        let prefix: String = id.chars().take(4).collect();

        let suffix: String = id.chars().skip(id.len().saturating_sub(8)).collect();

        format!("{}…{}", &prefix, &suffix)
    }
}

/// Represents the address of an arbitrary node.
pub trait AddressingInformation: Debug + Hash + Codec + Clone + Eq + Send + Sync + 'static {
    type PeerId: PeerId;

    /// Returns the peer id associated with this address.
    fn peer_id(&self) -> Self::PeerId;

    /// Verify the information.
    fn verify(&self) -> bool;
}

/// Abstraction for requesting own network addressing information.
pub trait NetworkIdentity {
    type PeerId: PeerId;
    type AddressingInformation: AddressingInformation<PeerId = Self::PeerId>;

    /// The external identity of this node.
    fn identity(&self) -> Self::AddressingInformation;
}

/// Abstraction for requesting justifications for finalized blocks and stale blocks.
pub trait RequestBlocks<B: Block>: Clone + Send + Sync + 'static {
    /// Request the justification for the given block
    fn request_justification(&self, hash: &B::Hash, number: NumberFor<B>);

    /// Request the given block -- this is supposed to be used only for "old forks".
    fn request_stale_block(&self, hash: B::Hash, number: NumberFor<B>);

    /// Clear all pending justification requests. We need this function in case
    /// we requested a justification for a block, which will never get it.
    fn clear_justification_requests(&self);

    /// Are we in the process of downloading the chain?
    ///
    /// Like [`NetworkService::is_major_syncing`][1].
    ///
    /// [1]: sc_network::NetworkService::is_major_syncing
    fn is_major_syncing(&self) -> bool;
}

/// Commands for manipulating the reserved peers set.
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionCommand<A: AddressingInformation> {
    AddReserved(HashSet<A>),
    DelReserved(HashSet<A::PeerId>),
}

/// A basic alias for properties we expect basic data to satisfy.
pub trait Data: Clone + Codec + Send + Sync + 'static {}

impl<D: Clone + Codec + Send + Sync + 'static> Data for D {}

// In practice D: Data and P: PeerId, but we cannot require that in type aliases.
type AddressedData<D, P> = (D, P);
