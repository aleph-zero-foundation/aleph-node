use std::fmt::Debug;

use futures::channel::mpsc;

use crate::{
    network::{
        manager::{DataInSession, VersionedAuthentication},
        AddressingInformation, ConnectionManagerIO, Data, NetworkServiceIO as NetworkIO,
        SessionManagerIO,
    },
    validator_network::{Network as ValidatorNetwork, PublicKey},
};

type AuthenticationNetworkIO<M, A> = NetworkIO<VersionedAuthentication<M, A>>;

type FullIO<D, M, A, VN> = (
    ConnectionManagerIO<D, M, A, VN>,
    AuthenticationNetworkIO<M, A>,
    SessionManagerIO<D>,
);

pub fn setup<
    D: Data,
    M: Data + Debug,
    A: AddressingInformation + TryFrom<Vec<M>> + Into<Vec<M>>,
    VN: ValidatorNetwork<A::PeerId, A, DataInSession<D>>,
>(
    validator_network: VN,
) -> FullIO<D, M, A, VN>
where
    A::PeerId: PublicKey,
{
    // Prepare and start the network
    let (messages_for_network, messages_from_user) = mpsc::unbounded();
    let (commands_for_service, commands_from_user) = mpsc::unbounded();
    let (messages_for_service, commands_from_manager) = mpsc::unbounded();
    let (messages_for_user, messages_from_network) = mpsc::unbounded();

    let connection_io = ConnectionManagerIO::new(
        messages_for_network,
        commands_from_user,
        commands_from_manager,
        messages_from_network,
        validator_network,
    );
    let channels_for_network = NetworkIO::new(messages_from_user, messages_for_user);
    let channels_for_session_manager =
        SessionManagerIO::new(commands_for_service, messages_for_service);

    (
        connection_io,
        channels_for_network,
        channels_for_session_manager,
    )
}
