#![cfg(test)]

use frame_election_provider_support::{ElectionProvider, Support};
use frame_support::bounded_vec;
use pallet_session::SessionManager;
#[cfg(feature = "try-runtime")]
use pallets_support::StorageMigration;
use primitives::{BanConfig as BanConfigStruct, CommitteeSeats};

use crate::{
    mock::{
        with_active_era, with_current_era, with_electable_targets, with_elected_validators,
        with_electing_voters, AccountId, Balance, Elections, SessionsPerEra, Test, TestExtBuilder,
    },
    BanConfig, CommitteeSize, CurrentEraValidators, NextEraCommitteeSize,
    NextEraNonReservedValidators, NextEraReservedValidators,
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
            assert_eq!(BanConfig::<Test>::get(), BanConfigStruct::default());
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
                elected,
                &[
                    (1, support(10, vec![(1, 10)])),
                    (2, no_support()),
                    (5, support(10, vec![(5, 10)])),
                    (6, no_support()),
                ]
            );
        });
}

#[test]
fn session_authorities_must_have_been_elected() {
    TestExtBuilder::new(vec![1, 2], vec![5, 6])
        .with_committee_seats(CommitteeSeats {
            reserved_seats: 2,
            non_reserved_seats: 2,
        })
        .build()
        .execute_with(|| {
            let next_era = 41;

            with_active_era(next_era - 1);
            with_elected_validators(next_era, vec![1, 5]);
            with_current_era(next_era);

            let mut authorities = <Elections as SessionManager<AccountId>>::new_session(
                next_era * SessionsPerEra::get(),
            )
            .unwrap_or_default();

            authorities.sort();
            assert_eq!(authorities, &[1, 5]);
        });
}

#[cfg(feature = "try-runtime")]
mod migration_tests {
    use frame_support::migration::put_storage_value;

    use super::*;

    const MODULE: &[u8] = b"Elections";

    #[test]
    fn migration_v0_to_v1_works() {
        TestExtBuilder::new(vec![], vec![])
            .with_storage_version(0)
            .build()
            .execute_with(|| {
                put_storage_value::<Vec<AccountId>>(MODULE, b"Members", &[], vec![1, 2]);
                crate::migrations::v0_to_v1::Migration::<Test, crate::Pallet<Test>>::migrate()
            });
    }

    #[test]
    fn migration_v1_to_v2_works() {
        TestExtBuilder::new(vec![], vec![])
            .with_storage_version(1)
            .build()
            .execute_with(|| {
                put_storage_value::<u32>(MODULE, b"MembersPerSession", &[], 2);
                put_storage_value::<Vec<AccountId>>(MODULE, b"ReservedMembers", &[], vec![1]);
                put_storage_value::<Vec<AccountId>>(MODULE, b"NonReservedMembers", &[], vec![2]);
                put_storage_value::<(Vec<AccountId>, Vec<AccountId>)>(
                    MODULE,
                    b"ErasMembers",
                    &[],
                    (vec![1], vec![2]),
                );
                crate::migrations::v1_to_v2::Migration::<Test, crate::Pallet<Test>>::migrate()
            });
    }

    #[test]
    fn migration_v2_to_v3_works() {
        TestExtBuilder::new(vec![1, 2], vec![3])
            .with_storage_version(2)
            .build()
            .execute_with(|| {
                put_storage_value::<u32>(MODULE, b"CommitteeSize", &[], 2);
                put_storage_value::<u32>(MODULE, b"NextEraCommitteeSize", &[], 3);
                crate::migrations::v2_to_v3::Migration::<Test, crate::Pallet<Test>>::migrate()
            });
    }
}
