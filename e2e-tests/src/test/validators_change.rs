use aleph_client::{
    wait_for_event, wait_for_finalized_block, AnyConnection, Header, RootConnection,
};
use codec::Decode;
use log::info;
use sp_core::Pair;
use substrate_api_client::{AccountId, XtStatus};

use crate::{
    accounts::{get_sudo_key, get_validators_keys},
    config::Config,
};

pub fn change_validators(config: &Config) -> anyhow::Result<()> {
    let accounts = get_validators_keys(config);
    let sudo = get_sudo_key(config);

    let connection = RootConnection::new(&config.node, sudo);

    let reserved_before: Vec<AccountId> =
        connection.read_storage_value("Elections", "NextEraReservedValidators");

    let non_reserved_before: Vec<AccountId> =
        connection.read_storage_value("Elections", "NextEraNonReservedValidators");

    let committee_size_before: u32 = connection.read_storage_value("Elections", "CommitteeSize");

    info!(
        "[+] state before tx: reserved: {:#?}, non_reserved: {:#?}, committee_size: {:#?}",
        reserved_before, non_reserved_before, committee_size_before
    );

    let new_validators: Vec<AccountId> = accounts.iter().map(|pair| pair.public().into()).collect();
    aleph_client::change_validators(
        &connection,
        Some(new_validators[0..2].to_vec()),
        Some(new_validators[2..].to_vec()),
        Some(4),
        XtStatus::InBlock,
    );

    #[derive(Debug, Decode, Clone)]
    struct NewValidatorsEvent {
        reserved: Vec<AccountId>,
        non_reserved: Vec<AccountId>,
        committee_size: u32,
    }
    wait_for_event(
        &connection,
        ("Elections", "ChangeValidators"),
        |e: NewValidatorsEvent| {
            info!("[+] NewValidatorsEvent: reserved: {:#?}, non_reserved: {:#?}, committee_size: {:#?}", e.reserved, e.non_reserved, e.non_reserved);

            e.reserved == new_validators[0..2]
                && e.non_reserved == new_validators[2..]
                && e.committee_size == 4
        },
    )?;

    let reserved_after: Vec<AccountId> =
        connection.read_storage_value("Elections", "NextEraReservedValidators");

    let non_reserved_after: Vec<AccountId> =
        connection.read_storage_value("Elections", "NextEraNonReservedValidators");

    let committee_size_after: u32 = connection.read_storage_value("Elections", "CommitteeSize");

    info!(
        "[+] state before tx: reserved: {:#?}, non_reserved: {:#?}, committee_size: {:#?}",
        reserved_after, non_reserved_after, committee_size_after
    );

    assert_eq!(new_validators[..2], reserved_after);
    assert_eq!(new_validators[2..], non_reserved_after);
    assert_eq!(4, committee_size_after);

    let block_number = connection
        .as_connection()
        .get_header::<Header>(None)
        .expect("Could not fetch header")
        .expect("Block exists; qed")
        .number;
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
