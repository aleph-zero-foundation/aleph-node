use futures::channel::mpsc;

use crate::network::{
    manager::NetworkData, ConnectionManagerIO, Data, Multiaddress, NetworkServiceIO as NetworkIo,
    SessionManagerIO,
};

type NetworkServiceIO<D, M> = NetworkIo<NetworkData<D, M>, M>;

pub fn setup<D: Data, M: Multiaddress + 'static>() -> (
    ConnectionManagerIO<D, M>,
    NetworkServiceIO<D, M>,
    SessionManagerIO<D>,
) {
    // Prepare and start the network
    let (commands_for_network, commands_from_io) = mpsc::unbounded();
    let (messages_for_network, messages_from_user) = mpsc::unbounded();
    let (commands_for_service, commands_from_user) = mpsc::unbounded();
    let (messages_for_service, commands_from_manager) = mpsc::unbounded();
    let (messages_for_user, messages_from_network) = mpsc::unbounded();

    let connection_io = ConnectionManagerIO::new(
        commands_for_network,
        messages_for_network,
        commands_from_user,
        commands_from_manager,
        messages_from_network,
    );
    let channels_for_network =
        NetworkServiceIO::new(messages_from_user, messages_for_user, commands_from_io);
    let channels_for_session_manager =
        SessionManagerIO::new(commands_for_service, messages_for_service);

    (
        connection_io,
        channels_for_network,
        channels_for_session_manager,
    )
}
