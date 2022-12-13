use std::fmt::Debug;

use futures::channel::mpsc;

use crate::{
    network::{
        manager::{DataInSession, VersionedAuthentication},
        AddressingInformation, ConnectionManagerIO, Data, GossipNetwork, SessionManagerIO,
    },
    validator_network::{Network as ValidatorNetwork, PublicKey},
};

type FullIO<D, M, A, VN, GN> = (ConnectionManagerIO<D, M, A, VN, GN>, SessionManagerIO<D>);

pub fn setup<
    D: Data,
    M: Data + Debug,
    A: AddressingInformation + TryFrom<Vec<M>> + Into<Vec<M>>,
    VN: ValidatorNetwork<A::PeerId, A, DataInSession<D>>,
    GN: GossipNetwork<VersionedAuthentication<M, A>>,
>(
    validator_network: VN,
    gossip_network: GN,
) -> FullIO<D, M, A, VN, GN>
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
