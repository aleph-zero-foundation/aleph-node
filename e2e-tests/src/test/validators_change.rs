use crate::{
    accounts::{get_sudo_key, get_validators_keys},
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
    let accounts = get_validators_keys(config);
    let sudo = get_sudo_key(config);

    let connection = RootConnection::new(&config.node, sudo);

    let reserved_before: Vec<AccountId> = connection
        .as_connection()
        .get_storage_value("Elections", "ReservedMembers", None)?
        .unwrap();

    let non_reserved_before: Vec<AccountId> = connection
        .as_connection()
        .get_storage_value("Elections", "NonReservedMembers", None)?
        .unwrap();

    let members_per_session_before: u32 = connection
        .as_connection()
        .get_storage_value("Elections", "MembersPerSession", None)?
        .unwrap();

    info!(
        "[+] state before tx: reserved: {:#?}, non_reserved: {:#?}, members_per_session: {:#?}",
        reserved_before, non_reserved_before, members_per_session_before
    );

    let new_members: Vec<AccountId> = accounts.iter().map(|pair| pair.public().into()).collect();
    change_members(
        &connection,
        Some(new_members[0..2].to_vec()),
        Some(new_members[2..].to_vec()),
        Some(4),
        XtStatus::InBlock,
    );

    #[derive(Debug, Decode, Clone)]
    struct NewMembersEvent {
        reserved: Vec<AccountId>,
        non_reserved: Vec<AccountId>,
        members_per_session: u32,
    }
    wait_for_event(
        &connection,
        ("Elections", "ChangeMembers"),
        |e: NewMembersEvent| {
            info!("[+] NewMembersEvent: reserved: {:#?}, non_reserved: {:#?}, members_per_session: {:#?}", e.reserved, e.non_reserved, e.non_reserved);

            e.reserved == new_members[0..2]
                && e.non_reserved == new_members[2..]
                && e.members_per_session == 4
        },
    )?;

    let reserved_after: Vec<AccountId> = connection
        .as_connection()
        .get_storage_value("Elections", "ReservedMembers", None)?
        .unwrap();

    let non_reserved_after: Vec<AccountId> = connection
        .as_connection()
        .get_storage_value("Elections", "NonReservedMembers", None)?
        .unwrap();

    let members_per_session_after: u32 = connection
        .as_connection()
        .get_storage_value("Elections", "MembersPerSession", None)?
        .unwrap();

    info!(
        "[+] state before tx: reserved: {:#?}, non_reserved: {:#?}, members_per_session: {:#?}",
        reserved_after, non_reserved_after, members_per_session_after
    );

    assert_eq!(new_members[..2], reserved_after);
    assert_eq!(new_members[2..], non_reserved_after);
    assert_eq!(4, members_per_session_after);

    let block_number = connection
        .as_connection()
        .get_header::<Header>(None)
        .expect("Could not fetch header")
        .expect("Block exists; qed")
        .number;
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
