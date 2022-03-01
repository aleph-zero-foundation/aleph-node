use crate::waiting::wait_for_event;
use crate::{send_xt, AccountId, BlockNumber, Connection, KeyPair};
use codec::{Decode, Encode};
use common::create_connection;
use log::info;
use sp_core::Pair;
use substrate_api_client::{compose_call, compose_extrinsic, XtStatus};

// Using custom struct and rely on default Encode trait from Parity's codec
// it works since byte arrays are encoded in a straight forward way, it as-is
#[derive(Debug, Encode, Clone)]
pub struct TestSessionKeys {
    pub aura: [u8; 32],
    pub aleph: [u8; 32],
}

// Manually implementing decoding
impl From<Vec<u8>> for TestSessionKeys {
    fn from(bytes: Vec<u8>) -> Self {
        assert_eq!(bytes.len(), 64);
        Self {
            aura: bytes[0..32].try_into().unwrap(),
            aleph: bytes[32..64].try_into().unwrap(),
        }
    }
}

pub fn send_change_members(sudo_connection: &Connection, new_members: Vec<AccountId>) {
    info!("New members {:#?}", new_members);
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
        &sudo_connection,
        xt.hex_encode(),
        "sudo_unchecked_weight",
        XtStatus::InBlock,
    );
}

pub fn session_set_keys(
    address: &str,
    signer: &KeyPair,
    new_keys: TestSessionKeys,
    tx_status: XtStatus,
) {
    let connection = create_connection(address).set_signer(signer.clone());
    let xt = compose_extrinsic!(connection, "Session", "set_keys", new_keys, 0u8);
    send_xt(&connection, xt.hex_encode(), "set_keys", tx_status);
}

pub fn get_current_session(connection: &Connection) -> u32 {
    connection
        .get_storage_value("Session", "CurrentIndex", None)
        .unwrap()
        .unwrap()
}

pub fn wait_for_session(
    connection: &Connection,
    session_index: u32,
) -> anyhow::Result<BlockNumber> {
    info!("Waiting for the session {}", session_index);

    #[derive(Debug, Decode, Clone)]
    struct NewSessionEvent {
        session_index: u32,
    }
    wait_for_event(
        connection,
        ("Session", "NewSession"),
        |e: NewSessionEvent| {
            info!("[+] new session {}", e.session_index);

            e.session_index == session_index
        },
    )?;
    Ok(session_index)
}
