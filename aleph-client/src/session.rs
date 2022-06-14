use crate::{
    send_xt, waiting::wait_for_event, AnyConnection, BlockNumber, RootConnection, SignedConnection,
};
use codec::{Decode, Encode};
use log::info;
use sp_core::Pair;
use substrate_api_client::{
    compose_call, compose_extrinsic, AccountId, ExtrinsicParams, FromHexString, XtStatus,
};

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

pub fn change_validators(
    sudo_connection: &RootConnection,
    new_reserved_validators: Option<Vec<AccountId>>,
    new_non_reserved_validators: Option<Vec<AccountId>>,
    validators_per_session: Option<u32>,
    status: XtStatus,
) {
    info!(target: "aleph-client", "New validators: reserved: {:#?}, non_reserved: {:#?}, validators_per_session: {:?}", new_reserved_validators, new_non_reserved_validators, validators_per_session);
    let call = compose_call!(
        sudo_connection.as_connection().metadata,
        "Elections",
        "change_validators",
        new_reserved_validators,
        new_non_reserved_validators,
        validators_per_session
    );
    let xt = compose_extrinsic!(
        sudo_connection.as_connection(),
        "Sudo",
        "sudo_unchecked_weight",
        call,
        0_u64
    );
    send_xt(sudo_connection, xt, Some("change_validators"), status);
}

pub fn change_next_era_reserved_validators(
    sudo_connection: &RootConnection,
    new_validators: Vec<AccountId>,
    status: XtStatus,
) {
    change_validators(sudo_connection, Some(new_validators), None, None, status)
}

pub fn set_keys(connection: &SignedConnection, new_keys: Keys, status: XtStatus) {
    let xt = compose_extrinsic!(
        connection.as_connection(),
        "Session",
        "set_keys",
        new_keys,
        0u8
    );
    send_xt(connection, xt, Some("set_keys"), status);
}

/// Get the number of the current session.
pub fn get_current<C: AnyConnection>(connection: &C) -> u32 {
    connection
        .as_connection()
        .get_storage_value("Session", "CurrentIndex", None)
        .unwrap()
        .unwrap_or(0)
}

pub fn wait_for<C: AnyConnection>(
    connection: &C,
    session_index: u32,
) -> anyhow::Result<BlockNumber> {
    info!(target: "aleph-client", "Waiting for session {}", session_index);

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
