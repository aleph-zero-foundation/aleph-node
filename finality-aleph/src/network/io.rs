use futures::channel::mpsc;

use crate::network::{
    manager::{DataInSession, VersionedAuthentication},
    ConnectionManagerIO, Data, Multiaddress, NetworkServiceIO as NetworkIO, SessionManagerIO,
};

type AuthenticationNetworkIO<D, M> = NetworkIO<VersionedAuthentication<M>, DataInSession<D>, M>;

pub fn setup<D: Data, M: Multiaddress + 'static>() -> (
    ConnectionManagerIO<D, M>,
    AuthenticationNetworkIO<D, M>,
    SessionManagerIO<D>,
) {
    // Prepare and start the network
    let (commands_for_network, commands_from_io) = mpsc::unbounded();
    let (data_for_network, data_from_user) = mpsc::unbounded();
    let (messages_for_network, messages_from_user) = mpsc::unbounded();
    let (commands_for_service, commands_from_user) = mpsc::unbounded();
    let (messages_for_service, commands_from_manager) = mpsc::unbounded();
    let (data_for_user, data_from_network) = mpsc::unbounded();
    let (messages_for_user, messages_from_network) = mpsc::unbounded();

    let connection_io = ConnectionManagerIO::new(
        commands_for_network,
        data_for_network,
        messages_for_network,
        commands_from_user,
        commands_from_manager,
        data_from_network,
        messages_from_network,
    );
    let channels_for_network = NetworkIO::new(
        data_from_user,
        messages_from_user,
        data_for_user,
        messages_for_user,
        commands_from_io,
    );
    let channels_for_session_manager =
        SessionManagerIO::new(commands_for_service, messages_for_service);

    (
        connection_io,
        channels_for_network,
        channels_for_session_manager,
    )
}
