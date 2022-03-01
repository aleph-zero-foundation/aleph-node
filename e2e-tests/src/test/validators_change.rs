use codec::Decode;
use common::create_connection;
use log::info;
use sp_core::Pair;
use substrate_api_client::AccountId;

use crate::accounts::{accounts_from_seeds, get_sudo};
use crate::config::Config;
use crate::session::send_change_members;
use crate::waiting::{wait_for_event, wait_for_finalized_block};
use crate::Header;

pub fn change_validators(config: &Config) -> anyhow::Result<()> {
    let Config {
        ref node, seeds, ..
    } = config;

    let mut accounts = accounts_from_seeds(seeds);
    let sudo = get_sudo(config);

    let connection = create_connection(node).set_signer(sudo);

    let members_before: Vec<AccountId> = connection
        .get_storage_value("Elections", "Members", None)?
        .unwrap();

    info!("[+] members before tx: {:#?}", members_before);

    accounts.remove(0);
    let new_members: Vec<AccountId> = accounts.iter().map(|pair| pair.public().into()).collect();
    send_change_members(&connection, new_members.clone());

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

    let block_number = connection
        .get_header::<Header>(None)
        .expect("Could not fetch header")
        .expect("Block exists; qed")
        .number;
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
