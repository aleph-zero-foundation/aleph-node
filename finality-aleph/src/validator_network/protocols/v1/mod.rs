use codec::{Decode, Encode};
use futures::{channel::mpsc, StreamExt};
use log::{debug, info, trace};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::{timeout, Duration},
};

use crate::validator_network::{
    io::{receive_data, send_data},
    protocols::{
        handshake::{v0_handshake_incoming, v0_handshake_outgoing},
        ConnectionType, ProtocolError, ResultForService,
    },
    Data, PublicKey, SecretKey, Splittable,
};

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_MISSED_HEARTBEATS: u32 = 4;

#[derive(Debug, Clone, Encode, Decode)]
enum Message<D: Data> {
    Data(D),
    Heartbeat,
}

async fn sending<PK: PublicKey, D: Data, S: AsyncWrite + Unpin + Send>(
    mut sender: S,
    mut data_from_user: mpsc::UnboundedReceiver<D>,
) -> Result<(), ProtocolError<PK>> {
    use Message::*;
    loop {
        let to_send = match timeout(HEARTBEAT_TIMEOUT, data_from_user.next()).await {
            Ok(maybe_data) => match maybe_data {
                Some(data) => Data(data),
                // We have been closed by the parent service, all good.
                None => return Ok(()),
            },
            _ => Heartbeat,
        };
        sender = send_data(sender, to_send).await?;
    }
}

async fn receiving<PK: PublicKey, D: Data, S: AsyncRead + Unpin + Send>(
    mut stream: S,
    data_for_user: mpsc::UnboundedSender<D>,
) -> Result<(), ProtocolError<PK>> {
    use Message::*;
    loop {
        let (old_stream, message) = timeout(
            MAX_MISSED_HEARTBEATS * HEARTBEAT_TIMEOUT,
            receive_data(stream),
        )
        .await
        .map_err(|_| ProtocolError::CardiacArrest)??;
        stream = old_stream;
        match message {
            Data(data) => data_for_user
                .unbounded_send(data)
                .map_err(|_| ProtocolError::NoUserConnection)?,
            Heartbeat => (),
        }
    }
}

async fn manage_connection<
    PK: PublicKey,
    D: Data,
    S: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
>(
    sender: S,
    receiver: R,
    data_from_user: mpsc::UnboundedReceiver<D>,
    data_for_user: mpsc::UnboundedSender<D>,
) -> Result<(), ProtocolError<PK>> {
    let sending = sending(sender, data_from_user);
    let receiving = receiving(receiver, data_for_user);
    tokio::select! {
        result = receiving => result,
        result = sending => result,
    }
}

/// Performs the outgoing handshake, and then manages a connection sending and receiving data.
/// Exits on parent request, or in case of broken or dead network connection.
pub async fn outgoing<SK: SecretKey, D: Data, S: Splittable>(
    stream: S,
    secret_key: SK,
    public_key: SK::PublicKey,
    result_for_parent: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
    data_for_user: mpsc::UnboundedSender<D>,
) -> Result<(), ProtocolError<SK::PublicKey>> {
    trace!(target: "validator-network", "Extending hand to {}.", public_key);
    let (sender, receiver) = v0_handshake_outgoing(stream, secret_key, public_key.clone()).await?;
    info!(target: "validator-network", "Outgoing handshake with {} finished successfully.", public_key);
    let (data_for_network, data_from_user) = mpsc::unbounded();
    result_for_parent
        .unbounded_send((
            public_key.clone(),
            Some(data_for_network),
            ConnectionType::New,
        ))
        .map_err(|_| ProtocolError::NoParentConnection)?;
    debug!(target: "validator-network", "Starting worker for communicating with {}.", public_key);
    manage_connection(sender, receiver, data_from_user, data_for_user).await
}

/// Performs the incoming handshake, and then manages a connection sending and receiving data.
/// Exits on parent request (when the data source is dropped), or in case of broken or dead
/// network connection.
pub async fn incoming<SK: SecretKey, D: Data, S: Splittable>(
    stream: S,
    secret_key: SK,
    result_for_parent: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
    data_for_user: mpsc::UnboundedSender<D>,
) -> Result<(), ProtocolError<SK::PublicKey>> {
    trace!(target: "validator-network", "Waiting for extended hand...");
    let (sender, receiver, public_key) = v0_handshake_incoming(stream, secret_key).await?;
    info!(target: "validator-network", "Incoming handshake with {} finished successfully.", public_key);
    let (data_for_network, data_from_user) = mpsc::unbounded();
    result_for_parent
        .unbounded_send((
            public_key.clone(),
            Some(data_for_network),
            ConnectionType::New,
        ))
        .map_err(|_| ProtocolError::NoParentConnection)?;
    debug!(target: "validator-network", "Starting worker for communicating with {}.", public_key);
    manage_connection(sender, receiver, data_from_user, data_for_user).await
}

#[cfg(test)]
mod tests {
    use aleph_primitives::AuthorityId;
    use futures::{
        channel::{mpsc, mpsc::UnboundedReceiver},
        pin_mut, FutureExt, StreamExt,
    };

    use super::{incoming, outgoing, ProtocolError};
    use crate::{
        crypto::AuthorityPen,
        validator_network::{
            mock::{key, MockSplittable},
            protocols::{ConnectionType, ResultForService},
            Data,
        },
    };

    async fn prepare<D: Data>() -> (
        AuthorityId,
        AuthorityPen,
        AuthorityId,
        AuthorityPen,
        impl futures::Future<Output = Result<(), ProtocolError<AuthorityId>>>,
        impl futures::Future<Output = Result<(), ProtocolError<AuthorityId>>>,
        UnboundedReceiver<D>,
        UnboundedReceiver<D>,
        UnboundedReceiver<ResultForService<AuthorityId, D>>,
        UnboundedReceiver<ResultForService<AuthorityId, D>>,
    ) {
        let (stream_incoming, stream_outgoing) = MockSplittable::new(4096);
        let (id_incoming, pen_incoming) = key().await;
        let (id_outgoing, pen_outgoing) = key().await;
        assert_ne!(id_incoming, id_outgoing);
        let (incoming_result_for_service, result_from_incoming) = mpsc::unbounded();
        let (outgoing_result_for_service, result_from_outgoing) = mpsc::unbounded();
        let (incoming_data_for_user, data_from_incoming) = mpsc::unbounded::<D>();
        let (outgoing_data_for_user, data_from_outgoing) = mpsc::unbounded::<D>();
        let incoming_handle = incoming(
            stream_incoming,
            pen_incoming.clone(),
            incoming_result_for_service,
            incoming_data_for_user,
        );
        let outgoing_handle = outgoing(
            stream_outgoing,
            pen_outgoing.clone(),
            id_incoming.clone(),
            outgoing_result_for_service,
            outgoing_data_for_user,
        );
        (
            id_incoming,
            pen_incoming,
            id_outgoing,
            pen_outgoing,
            incoming_handle,
            outgoing_handle,
            data_from_incoming,
            data_from_outgoing,
            result_from_incoming,
            result_from_outgoing,
        )
    }

    #[tokio::test]
    async fn send_data() {
        let (
            _id_incoming,
            _pen_incoming,
            _id_outgoing,
            _pen_outgoing,
            incoming_handle,
            outgoing_handle,
            mut data_from_incoming,
            mut data_from_outgoing,
            mut result_from_incoming,
            mut result_from_outgoing,
        ) = prepare::<Vec<i32>>().await;
        let incoming_handle = incoming_handle.fuse();
        let outgoing_handle = outgoing_handle.fuse();
        pin_mut!(incoming_handle);
        pin_mut!(outgoing_handle);
        let _data_for_outgoing = tokio::select! {
            _ = &mut incoming_handle => panic!("incoming process unexpectedly finished"),
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
            result = result_from_outgoing.next() => {
                let (_, maybe_data_for_outgoing, connection_type) = result.expect("the chennel shouldn't be dropped");
                assert_eq!(connection_type, ConnectionType::New);
                let data_for_outgoing = maybe_data_for_outgoing.expect("successfully connected");
                data_for_outgoing
                    .unbounded_send(vec![4, 3, 43])
                    .expect("should send");
                data_for_outgoing
                    .unbounded_send(vec![2, 1, 3, 7])
                    .expect("should send");
                data_for_outgoing
            },
        };
        let _data_for_incoming = tokio::select! {
            _ = &mut incoming_handle => panic!("incoming process unexpectedly finished"),
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
            result = result_from_incoming.next() => {
                let (_, maybe_data_for_incoming, connection_type) = result.expect("the chennel shouldn't be dropped");
                assert_eq!(connection_type, ConnectionType::New);
                let data_for_incoming = maybe_data_for_incoming.expect("successfully connected");
                data_for_incoming
                    .unbounded_send(vec![5, 4, 44])
                    .expect("should send");
                data_for_incoming
                    .unbounded_send(vec![3, 2, 4, 8])
                    .expect("should send");
                data_for_incoming
            },
        };
        tokio::select! {
            _ = &mut incoming_handle => panic!("incoming process unexpectedly finished"),
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
            v = data_from_incoming.next() => {
                assert_eq!(v, Some(vec![4, 3, 43]));
            },
        };
        tokio::select! {
            _ = &mut incoming_handle => panic!("incoming process unexpectedly finished"),
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
            v = data_from_incoming.next() => {
                assert_eq!(v, Some(vec![2, 1, 3, 7]));
            },
        };
        tokio::select! {
            _ = &mut incoming_handle => panic!("incoming process unexpectedly finished"),
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
            v = data_from_outgoing.next() => {
                assert_eq!(v, Some(vec![5, 4, 44]));
            },
        };
        tokio::select! {
            _ = &mut incoming_handle => panic!("incoming process unexpectedly finished"),
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
            v = data_from_outgoing.next() => {
                assert_eq!(v, Some(vec![3, 2, 4, 8]));
            },
        };
    }

    #[tokio::test]
    async fn closed_by_parent_service() {
        let (
            _id_incoming,
            _pen_incoming,
            id_outgoing,
            _pen_outgoing,
            incoming_handle,
            outgoing_handle,
            _data_from_incoming,
            _data_from_outgoing,
            mut result_from_incoming,
            _result_from_outgoing,
        ) = prepare::<Vec<i32>>().await;
        let incoming_handle = incoming_handle.fuse();
        let outgoing_handle = outgoing_handle.fuse();
        pin_mut!(incoming_handle);
        pin_mut!(outgoing_handle);
        tokio::select! {
            _ = &mut incoming_handle => panic!("incoming process unexpectedly finished"),
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
            received = result_from_incoming.next() => {
                // we drop the data sending channel, thus finishing incoming_handle
                let (received_id, _, connection_type) = received.expect("the chennel shouldn't be dropped");
                assert_eq!(connection_type, ConnectionType::New);
                assert_eq!(received_id, id_outgoing);
            },
        };
        incoming_handle
            .await
            .expect("closed manually, should finish with no error");
    }

    #[tokio::test]
    async fn parent_service_dead() {
        let (
            _id_incoming,
            _pen_incoming,
            _id_outgoing,
            _pen_outgoing,
            incoming_handle,
            outgoing_handle,
            _data_from_incoming,
            _data_from_outgoing,
            result_from_incoming,
            _result_from_outgoing,
        ) = prepare::<Vec<i32>>().await;
        std::mem::drop(result_from_incoming);
        let incoming_handle = incoming_handle.fuse();
        let outgoing_handle = outgoing_handle.fuse();
        pin_mut!(incoming_handle);
        pin_mut!(outgoing_handle);
        tokio::select! {
            e = &mut incoming_handle => match e {
                Err(ProtocolError::NoParentConnection) => (),
                Err(e) => panic!("unexpected error: {}", e),
                Ok(_) => panic!("successfully finished when parent dead"),
            },
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
        };
    }

    #[tokio::test]
    async fn parent_user_dead() {
        let (
            _id_incoming,
            _pen_incoming,
            _id_outgoing,
            _pen_outgoing,
            incoming_handle,
            outgoing_handle,
            data_from_incoming,
            _data_from_outgoing,
            _result_from_incoming,
            mut result_from_outgoing,
        ) = prepare::<Vec<i32>>().await;
        std::mem::drop(data_from_incoming);
        let incoming_handle = incoming_handle.fuse();
        let outgoing_handle = outgoing_handle.fuse();
        pin_mut!(incoming_handle);
        pin_mut!(outgoing_handle);
        let _data_for_outgoing = tokio::select! {
            _ = &mut incoming_handle => panic!("incoming process unexpectedly finished"),
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
            result = result_from_outgoing.next() => {
                let (_, maybe_data_for_outgoing, connection_type) = result.expect("the chennel shouldn't be dropped");
                assert_eq!(connection_type, ConnectionType::New);
                let data_for_outgoing = maybe_data_for_outgoing.expect("successfully connected");
                data_for_outgoing
                    .unbounded_send(vec![2, 1, 3, 7])
                    .expect("should send");
                data_for_outgoing
            },
        };
        tokio::select! {
            e = &mut incoming_handle => match e {
                Err(ProtocolError::NoUserConnection) => (),
                Err(e) => panic!("unexpected error: {}", e),
                Ok(_) => panic!("successfully finished when user dead"),
            },
            _ = &mut outgoing_handle => panic!("outgoing process unexpectedly finished"),
        };
    }

    #[tokio::test]
    async fn sender_dead_before_handshake() {
        let (
            _id_incoming,
            _pen_incoming,
            _id_outgoing,
            _pen_outgoing,
            incoming_handle,
            outgoing_handle,
            _data_from_incoming,
            _data_from_outgoing,
            _result_from_incoming,
            _result_from_outgoing,
        ) = prepare::<Vec<i32>>().await;
        std::mem::drop(outgoing_handle);
        match incoming_handle.await {
            Err(ProtocolError::HandshakeError(_)) => (),
            Err(e) => panic!("unexpected error: {}", e),
            Ok(_) => panic!("successfully finished when connection dead"),
        };
    }

    #[tokio::test]
    async fn sender_dead_after_handshake() {
        let (
            _id_incoming,
            _pen_incoming,
            _id_outgoing,
            _pen_outgoing,
            incoming_handle,
            outgoing_handle,
            _data_from_incoming,
            _data_from_outgoing,
            mut result_from_incoming,
            _result_from_outgoing,
        ) = prepare::<Vec<i32>>().await;
        let incoming_handle = incoming_handle.fuse();
        pin_mut!(incoming_handle);
        let (_, _exit, connection_type) = tokio::select! {
            _ = &mut incoming_handle => panic!("incoming process unexpectedly finished"),
            _ = outgoing_handle => panic!("outgoing process unexpectedly finished"),
            out = result_from_incoming.next() => out.expect("should receive"),
        };
        assert_eq!(connection_type, ConnectionType::New);
        // outgoing_handle got consumed by tokio::select!, the sender is dead
        match incoming_handle.await {
            Err(ProtocolError::ReceiveError(_)) => (),
            Err(e) => panic!("unexpected error: {}", e),
            Ok(_) => panic!("successfully finished when connection dead"),
        };
    }

    #[tokio::test]
    async fn receiver_dead_before_handshake() {
        let (
            _id_incoming,
            _pen_incoming,
            _id_outgoing,
            _pen_outgoing,
            incoming_handle,
            outgoing_handle,
            _data_from_incoming,
            _data_from_outgoing,
            _result_from_incoming,
            _result_from_outgoing,
        ) = prepare::<Vec<i32>>().await;
        std::mem::drop(incoming_handle);
        match outgoing_handle.await {
            Err(ProtocolError::HandshakeError(_)) => (),
            Err(e) => panic!("unexpected error: {}", e),
            Ok(_) => panic!("successfully finished when connection dead"),
        };
    }
}
