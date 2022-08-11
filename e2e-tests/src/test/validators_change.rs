use aleph_client::{
    get_current_block_number, get_next_era_committee_seats, get_next_era_non_reserved_validators,
    get_next_era_reserved_validators, wait_for_event, wait_for_finalized_block, AccountId,
    XtStatus,
};
use codec::Decode;
use log::info;
use primitives::CommitteeSeats;
use sp_core::Pair;

use crate::{accounts::get_validators_keys, config::Config};

pub fn change_validators(config: &Config) -> anyhow::Result<()> {
    let accounts = get_validators_keys(config);
    let connection = config.create_root_connection();

    let reserved_before = get_next_era_reserved_validators(&connection);
    let non_reserved_before = get_next_era_non_reserved_validators(&connection);
    let committee_size_before = get_next_era_committee_seats(&connection);

    info!(
        "[+] state before tx: reserved: {:#?}, non_reserved: {:#?}, committee_size: {:#?}",
        reserved_before, non_reserved_before, committee_size_before
    );

    let new_validators: Vec<AccountId> = accounts.iter().map(|pair| pair.public().into()).collect();
    aleph_client::change_validators(
        &connection,
        Some(new_validators[0..2].to_vec()),
        Some(new_validators[2..].to_vec()),
        Some(CommitteeSeats {
            reserved_seats: 2,
            non_reserved_seats: 2,
        }),
        XtStatus::InBlock,
    );

    #[derive(Debug, Decode, Clone)]
    struct NewValidatorsEvent {
        reserved: Vec<AccountId>,
        non_reserved: Vec<AccountId>,
        committee_size: CommitteeSeats,
    }
    wait_for_event(
        &connection,
        ("Elections", "ChangeValidators"),
        |e: NewValidatorsEvent| {
            info!("[+] NewValidatorsEvent: reserved: {:#?}, non_reserved: {:#?}, committee_size: {:#?}", e.reserved, e.non_reserved, e.non_reserved);

            e.reserved == new_validators[0..2]
                && e.non_reserved == new_validators[2..]
                && e.committee_size
                    == CommitteeSeats {
                        reserved_seats: 2,
                        non_reserved_seats: 2,
                    }
        },
    )?;

    let reserved_after = get_next_era_reserved_validators(&connection);
    let non_reserved_after = get_next_era_non_reserved_validators(&connection);
    let committee_size_after = get_next_era_committee_seats(&connection);

    info!(
        "[+] state before tx: reserved: {:#?}, non_reserved: {:#?}, committee_size: {:#?}",
        reserved_after, non_reserved_after, committee_size_after
    );

    assert_eq!(new_validators[..2], reserved_after);
    assert_eq!(new_validators[2..], non_reserved_after);
    assert_eq!(
        CommitteeSeats {
            reserved_seats: 2,
            non_reserved_seats: 2
        },
        committee_size_after
    );

    let block_number = get_current_block_number(&connection);
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
