use std::collections::HashMap;

use aleph_client::{
    change_validators, get_current_block_number, get_current_session, get_validators_for_session,
    wait_for_finalized_block, wait_for_full_era_completion, wait_for_session, XtStatus,
};
use primitives::CommitteeSeats;

use crate::{
    accounts::account_ids_from_keys, elections::get_members_subset_for_session,
    validators::get_test_validators, Config,
};

const TEST_LENGTH: u32 = 5;

pub fn validators_rotate(config: &Config) -> anyhow::Result<()> {
    let connection = config.get_first_signed_connection();
    let root_connection = config.create_root_connection();

    let era_validators = get_test_validators(config);
    let reserved_validators = account_ids_from_keys(&era_validators.reserved);

    let non_reserved_validators = account_ids_from_keys(&era_validators.non_reserved);

    let seats = CommitteeSeats {
        reserved_seats: 2,
        non_reserved_seats: 2,
    };

    change_validators(
        &root_connection,
        Some(reserved_validators.clone()),
        Some(non_reserved_validators.clone()),
        Some(seats),
        XtStatus::InBlock,
    );
    wait_for_full_era_completion(&connection)?;

    let current_session = get_current_session(&connection);
    wait_for_session(&connection, current_session + TEST_LENGTH)?;

    let mut non_reserved_count = HashMap::new();

    for session in current_session..current_session + TEST_LENGTH {
        let elected = get_validators_for_session(&connection, session);

        let non_reserved = get_members_subset_for_session(
            seats.non_reserved_seats,
            &non_reserved_validators,
            session,
        );

        for nr in non_reserved.clone() {
            *non_reserved_count.entry(nr).or_insert(0) += 1;
        }

        let reserved_included = reserved_validators
            .clone()
            .iter()
            .all(|reserved| elected.contains(reserved));

        let non_reserved_include = non_reserved
            .iter()
            .all(|non_reserved| elected.contains(non_reserved));

        let only_expected_validators = elected
            .iter()
            .all(|elected| reserved_validators.contains(elected) || non_reserved.contains(elected));

        assert!(
            reserved_included,
            "Reserved nodes should always be present, session #{}",
            session
        );
        assert!(
            non_reserved_include,
            "Missing non reserved node, session #{}",
            session
        );
        assert!(
            only_expected_validators,
            "Only expected validators should be present, session #{}",
            session
        );
    }

    let max_elected = non_reserved_count.values().max().unwrap();
    let min_elected = non_reserved_count.values().min().unwrap();
    assert!(max_elected - min_elected <= 1);

    let block_number = get_current_block_number(&connection);
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
