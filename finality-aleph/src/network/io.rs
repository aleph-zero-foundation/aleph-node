use std::fmt::Debug;

use futures::channel::mpsc;

use crate::network::{
    clique::{Network as CliqueNetwork, PublicKey},
    manager::{DataInSession, VersionedAuthentication},
    AddressingInformation, ConnectionManagerIO, Data, GossipNetwork, SessionManagerIO,
};

type FullIO<D, M, A, CN, GN> = (ConnectionManagerIO<D, M, A, CN, GN>, SessionManagerIO<D>);

pub fn setup<
    D: Data,
    M: Data + Debug,
    A: AddressingInformation + TryFrom<Vec<M>> + Into<Vec<M>>,
    CN: CliqueNetwork<A::PeerId, A, DataInSession<D>>,
    GN: GossipNetwork<VersionedAuthentication<M, A>>,
>(
    validator_network: CN,
    gossip_network: GN,
) -> FullIO<D, M, A, CN, GN>
where
    A::PeerId: PublicKey,
{
    // Prepare and start the network
    let (commands_for_service, commands_from_user) = mpsc::unbounded();
    let (messages_for_service, commands_from_manager) = mpsc::unbounded();

    let connection_io = ConnectionManagerIO::new(
        commands_from_user,
        commands_from_manager,
        validator_network,
        gossip_network,
    );
    let channels_for_session_manager =
        SessionManagerIO::new(commands_for_service, messages_for_service);

    (connection_io, channels_for_session_manager)
}
