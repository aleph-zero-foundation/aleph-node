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

pub fn change_validators(config: &Config) -> anyhow::Result<()> {
    let Config {
        ref node, seeds, ..
    } = config;

    let accounts = accounts_from_seeds(seeds);
    let sudo = get_sudo(config);

    let connection = create_connection(node).set_signer(sudo);

    let members_before: Vec<AccountId> = connection
        .get_storage_value("Elections", "Members", None)?
        .unwrap();

    info!("[+] members before tx: {:#?}", members_before);

    let new_members: Vec<AccountId> = accounts
        .iter()
        .map(|pair| pair.public().into())
        .chain(iter::once(
            AccountId::from_ss58check("5EHkv1FCd4jeQmVrbYhrETL1EAr8NJxNbukDRT4FaYWbjW8f").unwrap(),
        ))
        .collect();

    info!("[+] New members {:#?}", new_members);

    let call = compose_call!(
        connection.metadata,
        "Elections",
        "change_members",
        new_members.clone()
    );

    let tx = compose_extrinsic!(connection, "Sudo", "sudo_unchecked_weight", call, 0_u64);

    // send and watch extrinsic until finalized
    let tx_hash = connection
        .send_extrinsic(tx.hex_encode(), XtStatus::InBlock)
        .expect("Could not send extrinsc")
        .expect("Could not get tx hash");

    info!("[+] change_members transaction hash: {}", tx_hash);

    #[derive(Debug, Decode, Clone)]
    struct NewMembersEvent {
        members: Vec<AccountId>,
    }
    wait_for_event(
        &connection,
        ("Elections", "ChangeMembers"),
        |e: NewMembersEvent| {
            info!("[+] NewMembersEvent: members{:?}", e.members);

            e.members == new_members
        },
    )?;

    let members_after: Vec<AccountId> = connection
        .get_storage_value("Elections", "Members", None)?
        .unwrap();

    info!("[+] members after tx: {:#?}", members_after);

    assert!(new_members.eq(&members_after));

    Ok(())
}
