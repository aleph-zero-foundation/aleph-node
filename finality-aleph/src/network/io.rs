use futures::channel::mpsc;

use crate::{
    network::{
        manager::{DataInSession, VersionedAuthentication},
        ConnectionManagerIO, Data, Multiaddress, NetworkServiceIO as NetworkIO, SessionManagerIO,
    },
    validator_network::{Network as ValidatorNetwork, PublicKey},
};

type AuthenticationNetworkIO<M> = NetworkIO<VersionedAuthentication<M>>;

pub fn setup<
    D: Data,
    M: Multiaddress + 'static,
    VN: ValidatorNetwork<M::PeerId, M, DataInSession<D>>,
>(
    validator_network: VN,
) -> (
    ConnectionManagerIO<D, M, VN>,
    AuthenticationNetworkIO<M>,
    SessionManagerIO<D>,
)
where
    M::PeerId: PublicKey,
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
