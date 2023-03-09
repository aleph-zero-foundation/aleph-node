use std::{
    fmt::{Display, Error as FmtError, Formatter},
    io::Error as IoError,
};

use codec::DecodeAll;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::Data;

// We allow sending up to 16MiB, that should be enough forever.
pub const MAX_DATA_SIZE: u32 = 16 * 1024 * 1024;

/// A general error when sending or receving data.
#[derive(Debug)]
pub enum Error {
    ConnectionClosed(IoError),
    DataTooLong(u32),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            ConnectionClosed(e) => write!(f, "connection unexpectedly closed: {}", e),
            DataTooLong(length) => write!(
                f,
                "encoded data too long - {} bytes, the limit is {}",
                length, MAX_DATA_SIZE
            ),
        }
    }
}

/// An error when sending data.
#[derive(Debug)]
pub struct SendError(Error);

impl Display for SendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}", self.0)
    }
}

impl From<Error> for SendError {
    fn from(e: Error) -> Self {
        SendError(e)
    }
}

/// An error when receiving data.
#[derive(Debug)]
pub enum ReceiveError {
    Error(Error),
    DataCorrupted,
}

impl Display for ReceiveError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use ReceiveError::*;
        match self {
            Error(e) => write!(f, "{}", e),
            DataCorrupted => write!(f, "received corrupted data"),
        }
    }
}

impl From<Error> for ReceiveError {
    fn from(e: Error) -> Self {
        ReceiveError::Error(e)
    }
}

/// Sends some data using the stream.
pub async fn send_data<S: AsyncWriteExt + Unpin, D: Data>(
    mut stream: S,
    data: D,
) -> Result<S, SendError> {
    let encoded = data.encode();
    let len = u32::try_from(encoded.len()).map_err(|_| Error::DataTooLong(u32::MAX))?;
    if len > MAX_DATA_SIZE {
        return Err(Error::DataTooLong(len).into());
    }
    let encoded_len = len.to_le_bytes().to_vec();
    stream
        .write_all(&encoded_len)
        .await
        .map_err(Error::ConnectionClosed)?;
    stream
        .write_all(&encoded)
        .await
        .map_err(Error::ConnectionClosed)?;
    Ok(stream)
}

/// Attempts to receive some data using the stream.
pub async fn receive_data<S: AsyncReadExt + Unpin, D: Data>(
    mut stream: S,
) -> Result<(S, D), ReceiveError> {
    let mut buf = [0; 4];
    stream
        .read_exact(&mut buf[..])
        .await
        .map_err(Error::ConnectionClosed)?;
    let len = u32::from_le_bytes(buf);
    if len > MAX_DATA_SIZE {
        return Err(Error::DataTooLong(len).into());
    }
    let mut buf: Vec<u8> = vec![0; len as usize];
    stream
        .read_exact(&mut buf[..])
        .await
        .map_err(Error::ConnectionClosed)?;
    let data = D::decode_all(&mut &buf[..]).map_err(|_| ReceiveError::DataCorrupted)?;
    Ok((stream, data))
}

#[cfg(test)]
mod tests {
    use tokio::io::{duplex, AsyncWriteExt};

    use super::{receive_data, send_data, Error, ReceiveError, SendError, MAX_DATA_SIZE};

    #[tokio::test]
    async fn sends_and_receives_correct_data() {
        let (sender, receiver) = duplex(4096);
        let data: Vec<i32> = vec![4, 3, 43];
        let _sender = send_data(sender, data.clone())
            .await
            .expect("data should send");
        let (_receiver, received_data) = receive_data(receiver).await.expect("should receive data");
        let received_data: Vec<i32> = received_data;
        assert_eq!(data, received_data);
    }

    #[tokio::test]
    async fn fails_to_receive_from_dropped_connection() {
        let (_, receiver) = duplex(4096);
        match receive_data::<_, i32>(receiver).await {
            Err(e) => match e {
                ReceiveError::Error(Error::ConnectionClosed(_)) => (),
                e => panic!("unexpected error: {}", e),
            },
            _ => panic!("received data from a dropped stream!"),
        }
    }

    #[tokio::test]
    async fn fails_to_send_to_dropped_connection() {
        let (sender, _) = duplex(4096);
        let data: Vec<i32> = vec![4, 3, 43];
        match send_data(sender, data.clone()).await {
            Err(e) => match e {
                SendError(Error::ConnectionClosed(_)) => (),
                e => panic!("unexpected error: {}", e),
            },
            _ => panic!("send data to a dropped stream!"),
        }
    }

    #[tokio::test]
    async fn fails_to_send_too_big_message() {
        let (sender, _) = duplex(4096);
        let data: Vec<u8> = vec![
            43;
            (MAX_DATA_SIZE + 1)
                .try_into()
                .expect("why are you running tests on a 16 bit machine? o.0")
        ];
        match send_data(sender, data.clone()).await {
            Err(e) => match e {
                SendError(Error::DataTooLong(_)) => (),
                e => panic!("unexpected error: {}", e),
            },
            _ => panic!("send data to a dropped stream!"),
        }
    }

    #[tokio::test]
    async fn fails_to_receive_too_much_data() {
        let (mut sender, receiver) = duplex(4096);
        let too_long = MAX_DATA_SIZE + 43;
        let payload = too_long.to_le_bytes().to_vec();
        sender
            .write_all(&payload)
            .await
            .expect("sending should work");
        match receive_data::<_, i32>(receiver).await {
            Err(e) => match e {
                ReceiveError::Error(Error::DataTooLong(long)) => assert_eq!(long, too_long),
                e => panic!("unexpected error: {}", e),
            },
            _ => panic!("received too long data!"),
        }
    }

    #[tokio::test]
    async fn fails_to_decode_empty_data() {
        let (mut sender, receiver) = duplex(4096);
        let payload = 0u32.to_le_bytes().to_vec();
        sender
            .write_all(&payload)
            .await
            .expect("sending should work");
        match receive_data::<_, i32>(receiver).await {
            Err(e) => match e {
                ReceiveError::DataCorrupted => (),
                e => panic!("unexpected error: {}", e),
            },
            _ => panic!("decoded no data into something?!"),
        }
    }
}
