#![cfg(test)]

use frame_election_provider_support::{ElectionProvider, Support};
use primitives::CommitteeSeats;
use sp_core::bounded_vec;

use crate::{
    mock::{
        with_electable_targets, with_electing_voters, AccountId, Balance, Elections, Test,
        TestExtBuilder,
    },
    CommitteeSize, CurrentEraValidators, NextEraCommitteeSize, NextEraNonReservedValidators,
    NextEraReservedValidators,
};

fn no_support() -> Support<AccountId> {
    Default::default()
}

fn support(total: Balance, voters: Vec<(AccountId, Balance)>) -> Support<AccountId> {
    Support { total, voters }
}

#[test]
fn storage_is_initialized_already_in_genesis() {
    const RESERVED: [AccountId; 3] = [1, 2, 3];
    const NON_RESERVED: [AccountId; 2] = [4, 5];
    const COMMITTEE_SEATS: CommitteeSeats = CommitteeSeats {
        reserved_seats: 3,
        non_reserved_seats: 2,
        non_reserved_finality_seats: 2,
    };

    TestExtBuilder::new(RESERVED.to_vec(), NON_RESERVED.to_vec())
        .with_committee_seats(COMMITTEE_SEATS)
        .build()
        .execute_with(|| {
            assert_eq!(CommitteeSize::<Test>::get(), COMMITTEE_SEATS);
            assert_eq!(NextEraCommitteeSize::<Test>::get(), COMMITTEE_SEATS);
            assert_eq!(NextEraReservedValidators::<Test>::get(), RESERVED);
            assert_eq!(NextEraNonReservedValidators::<Test>::get(), NON_RESERVED);
            assert_eq!(CurrentEraValidators::<Test>::get().reserved, RESERVED);
            assert_eq!(
                CurrentEraValidators::<Test>::get().non_reserved,
                NON_RESERVED
            );
            // We do not expect SessionValidatorBlockCount and ValidatorEraTotalReward to be
            // populated from genesis, so does the ban related storages:
            // UnderperformedValidatorSessionCount and Banned
        });
}

#[test]
fn validators_are_elected_only_when_staking() {
    TestExtBuilder::new(vec![1, 2, 3, 4], vec![5, 6, 7, 8])
        .build()
        .execute_with(|| {
            // We check all 4 possibilities for both reserved and non reserved validators:
            // { staking validator, not staking validator } x { any support, no support }.
            //
            // Only those considered as staking should be elected.

            with_electable_targets(vec![1, 2, 5, 6]);
            with_electing_voters(vec![
                (1, 10, bounded_vec![1]),
                (3, 10, bounded_vec![3]),
                (5, 10, bounded_vec![5]),
                (7, 10, bounded_vec![7]),
            ]);

            let elected =
                <Elections as ElectionProvider>::elect().expect("`elect()` should succeed");

            assert_eq!(
                elected.into_inner(),
                &[
                    (1, support(10, vec![(1, 10)])),
                    (2, no_support()),
                    (5, support(10, vec![(5, 10)])),
                    (6, no_support()),
                ]
            );
        });
}
