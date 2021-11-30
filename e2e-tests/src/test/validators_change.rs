use std::iter;

use codec::Decode;
use common::create_connection;
use log::info;
use sp_core::crypto::Ss58Codec;
use sp_core::Pair;
use substrate_api_client::{compose_call, compose_extrinsic, AccountId, XtStatus};

use crate::accounts::{accounts_from_seeds, get_sudo};
use crate::config::Config;
use crate::waiting::wait_for_event;
use crate::Connection;

pub fn change_validators(config: Config) -> anyhow::Result<()> {
    let Config { node, seeds, .. } = config.clone();

    let accounts = accounts_from_seeds(seeds);
    let sudo = get_sudo(config);

    let connection = create_connection(node).set_signer(sudo);

    let validators_before: Vec<AccountId> = connection
        .get_storage_value("Session", "Validators", None)?
        .unwrap();

    info!("[+] Validators before tx: {:#?}", validators_before);

    let new_validators: Vec<AccountId> = accounts
        .into_iter()
        .map(|pair| pair.public().into())
        .chain(iter::once(
            AccountId::from_ss58check("5EHkv1FCd4jeQmVrbYhrETL1EAr8NJxNbukDRT4FaYWbjW8f").unwrap(),
        ))
        .collect();

    info!("[+] New validators {:#?}", new_validators);

    // wait beyond session 1
    let current_session_index = wait_for_session(&connection, 1)?;
    let session_for_change = current_session_index + 2;
    info!("[+] Current session index {:?}", current_session_index);

    let call = compose_call!(
        connection.metadata,
        "Aleph",
        "change_validators",
        new_validators.clone(),
        session_for_change
    );

    let tx = compose_extrinsic!(connection, "Sudo", "sudo_unchecked_weight", call, 0_u64);

    // send and watch extrinsic until finalized
    let tx_hash = connection
        .send_extrinsic(tx.hex_encode(), XtStatus::Finalized)
        .expect("Could not send extrinsc")
        .expect("Could not get tx hash");

    info!("[+] change_validators transaction hash: {}", tx_hash);

    // wait for the change to be applied
    wait_for_session(&connection, session_for_change)?;

    let validators_after: Vec<AccountId> = connection
        .get_storage_value("Session", "Validators", None)?
        .unwrap();

    info!("[+] Validators after tx: {:#?}", validators_after);

    assert!(new_validators.eq(&validators_after));

    Ok(())
}

#[derive(Debug, Decode, Copy, Clone)]
struct NewSessionEvent {
    session_index: u32,
}

/// blocking wait, if ongoing session index is >= new_session_index returns the current
fn wait_for_session(connection: &Connection, new_session_index: u32) -> anyhow::Result<u32> {
    wait_for_event(
        connection,
        ("Session", "NewSession"),
        |e: NewSessionEvent| {
            let session_index = e.session_index;
            info!("[+] NewSession event: session index {:?}", session_index);
            session_index.ge(&new_session_index)
        },
    )
    .map(|e| e.session_index)
}
