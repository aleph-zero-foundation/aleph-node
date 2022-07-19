#![cfg(test)]

use frame_election_provider_support::{ElectionProvider, Support};
use frame_support::bounded_vec;
use pallet_session::SessionManager;

use crate::{mock::*, CommitteeSeats, CommitteeSize};

fn no_support() -> Support<AccountId> {
    Default::default()
}

fn support(total: Balance, voters: Vec<(AccountId, Balance)>) -> Support<AccountId> {
    Support { total, voters }
}

#[test]
fn validators_are_elected_only_when_staking() {
    new_test_ext(vec![1, 2, 3, 4], vec![5, 6, 7, 8]).execute_with(|| {
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

        let elected = <Elections as ElectionProvider>::elect().expect("`elect()` should succeed");

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
    new_test_ext(vec![1, 2], vec![5, 6]).execute_with(|| {
        let next_era = 41;

        CommitteeSize::<Test>::put(CommitteeSeats {
            reserved_seats: 2,
            non_reserved_seats: 2,
        });

        with_active_era(next_era - 1);
        with_elected_validators(next_era, vec![1, 5]);

        let mut authorities =
            <Elections as SessionManager<AccountId>>::new_session(next_era * SessionsPerEra::get())
                .unwrap_or_default();

        authorities.sort();
        assert_eq!(authorities, &[1, 5]);
    });
}
