use crate::{
    accounts::{accounts_from_seeds, get_sudo},
    config::Config,
};
use aleph_client::{
    change_members, wait_for_event, wait_for_finalized_block, AnyConnection, Header, RootConnection,
};
use codec::Decode;
use log::info;
use sp_core::Pair;
use substrate_api_client::{AccountId, XtStatus};

pub fn change_validators(config: &Config) -> anyhow::Result<()> {
    let Config {
        ref node, seeds, ..
    } = config;

    let mut accounts = accounts_from_seeds(seeds);
    let sudo = get_sudo(config);

    let connection = RootConnection::new(node, sudo);

    let members_before: Vec<AccountId> = connection
        .as_connection()
        .get_storage_value("Elections", "Members", None)?
        .unwrap();

    info!("[+] members before tx: {:#?}", members_before);

    accounts.remove(0);
    let new_members: Vec<AccountId> = accounts.iter().map(|pair| pair.public().into()).collect();
    change_members(&connection, new_members.clone(), XtStatus::InBlock);

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
        .as_connection()
        .get_storage_value("Elections", "Members", None)?
        .unwrap();

    info!("[+] members after tx: {:#?}", members_after);

    assert!(new_members.eq(&members_after));

    let block_number = connection
        .as_connection()
        .get_header::<Header>(None)
        .expect("Could not fetch header")
        .expect("Block exists; qed")
        .number;
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
