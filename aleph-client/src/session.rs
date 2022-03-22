use crate::{send_xt, waiting::wait_for_event, BlockNumber, Connection};
use codec::{Decode, Encode};
use log::info;
use sp_core::Pair;
use substrate_api_client::{compose_call, compose_extrinsic, AccountId, FromHexString, XtStatus};

// Using custom struct and rely on default Encode trait from Parity's codec
// it works since byte arrays are encoded in a straight forward way, it as-is
#[derive(Debug, Encode, Clone)]
pub struct Keys {
    pub aura: [u8; 32],
    pub aleph: [u8; 32],
}

// Manually implementing decoding
impl From<Vec<u8>> for Keys {
    fn from(bytes: Vec<u8>) -> Self {
        assert_eq!(bytes.len(), 64);
        Self {
            aura: bytes[0..32].try_into().unwrap(),
            aleph: bytes[32..64].try_into().unwrap(),
        }
    }
}

impl TryFrom<String> for Keys {
    type Error = ();

    fn try_from(keys: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = match FromHexString::from_hex(keys) {
            Ok(bytes) => bytes,
            Err(_) => return Err(()),
        };
        Ok(Keys::from(bytes))
    }
}

pub fn change_members(sudo_connection: &Connection, new_members: Vec<AccountId>, status: XtStatus) {
    info!(target: "aleph-client", "New members {:#?}", new_members);
    let call = compose_call!(
        sudo_connection.metadata,
        "Elections",
        "change_members",
        new_members
    );
    let xt = compose_extrinsic!(
        sudo_connection,
        "Sudo",
        "sudo_unchecked_weight",
        call,
        0_u64
    );
    send_xt(
        sudo_connection,
        xt.hex_encode(),
        "sudo_unchecked_weight",
        status,
    );
}

pub fn set_keys(connection: &Connection, new_keys: Keys, status: XtStatus) {
    let xt = compose_extrinsic!(connection, "Session", "set_keys", new_keys, 0u8);
    send_xt(connection, xt.hex_encode(), "set_keys", status);
}

pub fn get_current(connection: &Connection) -> u32 {
    connection
        .get_storage_value("Session", "CurrentIndex", None)
        .unwrap()
        .unwrap()
}

pub fn wait_for(connection: &Connection, session_index: u32) -> anyhow::Result<BlockNumber> {
    info!(target: "aleph-client", "Waiting for the session {}", session_index);

    #[derive(Debug, Decode, Clone)]
    struct NewSessionEvent {
        session_index: u32,
    }
    wait_for_event(
        connection,
        ("Session", "NewSession"),
        |e: NewSessionEvent| {
            info!(target: "aleph-client", "New session {}", e.session_index);

            e.session_index == session_index
        },
    )?;
    Ok(session_index)
}
