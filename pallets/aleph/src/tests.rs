#![cfg(test)]

use frame_support::{storage_alias, traits::OneSessionHandler};
use primitives::VersionChange;

use crate::{mock::*, NextFinalityCommittee};

#[storage_alias]
type SessionForValidatorsChange = StorageValue<Aleph, u32>;

#[storage_alias]
type Validators<T> = StorageValue<Aleph, Vec<<T as frame_system::Config>::AccountId>>;

#[test]
fn test_update_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();
        run_session(1);

        NextFinalityCommittee::<Test>::put(vec![2, 3, 4]);
        let authorities = [2, 3, 4].iter().zip(to_authorities(&[2, 3, 4])).collect();

        Aleph::update_authorities(authorities);

        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2]));
        assert_eq!(Aleph::next_authorities(), to_authorities(&[2, 3, 4]));
    });
}

#[test]
fn test_initialize_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2]));
        assert_eq!(Aleph::next_authorities(), to_authorities(&[1, 2]));
    });
}

#[test]
fn fails_to_initialize_again_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        let authorities = to_authorities(&[1, 2, 3]);
        Aleph::initialize_authorities(&authorities, &authorities);

        // should not update storage
        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2]));
    });
}

#[test]
fn test_current_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();

        run_session(1);

        NextFinalityCommittee::<Test>::put(vec![2, 3, 4]);
        let authorities = [2, 3, 4].iter().zip(to_authorities(&[2, 3, 4])).collect();
        Aleph::update_authorities(authorities);

        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2]));
        assert_eq!(Aleph::next_authorities(), to_authorities(&[2, 3, 4]));

        run_session(2);

        NextFinalityCommittee::<Test>::put(vec![1, 2, 3]);
        let authorities = [1, 2, 3].iter().zip(to_authorities(&[1, 2, 3])).collect();
        Aleph::update_authorities(authorities);

        assert_eq!(Aleph::authorities(), to_authorities(&[2, 3, 4]));
        assert_eq!(Aleph::next_authorities(), to_authorities(&[1, 2, 3]));
    })
}

#[test]
fn test_session_rotation() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();
        run_session(1);

        NextFinalityCommittee::<Test>::put(vec![5, 6]);
        let new_validators = new_session_validators(&[1, 2]);
        let queued_validators = new_session_validators(&[5, 6]);
        Aleph::on_new_session(true, new_validators, queued_validators);
        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2]));
        assert_eq!(Aleph::next_authorities(), to_authorities(&[5, 6]));
    })
}

#[test]
fn test_emergency_signer() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();

        run_session(1);

        Aleph::set_next_emergency_finalizer(to_authority(&21));

        assert_eq!(Aleph::emergency_finalizer(), None);
        assert_eq!(Aleph::queued_emergency_finalizer(), None);

        run_session(2);

        Aleph::set_next_emergency_finalizer(to_authority(&37));

        assert_eq!(Aleph::emergency_finalizer(), None);
        assert_eq!(Aleph::queued_emergency_finalizer(), Some(to_authority(&21)));

        run_session(3);

        assert_eq!(Aleph::emergency_finalizer(), Some(to_authority(&21)));
        assert_eq!(Aleph::queued_emergency_finalizer(), Some(to_authority(&37)));
    })
}

#[test]
fn test_finality_version_scheduling() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();

        run_session(1);

        let version_to_schedule = VersionChange {
            version_incoming: 1,
            session: 4,
        };

        let scheduling_result =
            Aleph::do_schedule_finality_version_change(version_to_schedule.clone());
        assert_eq!(scheduling_result, Ok(()));

        let scheduled_version_change = Aleph::finality_version_change();
        assert_eq!(scheduled_version_change, Some(version_to_schedule.clone()));

        run_session(4);

        let current_version = Aleph::finality_version();
        assert_eq!(current_version, version_to_schedule.version_incoming);

        let scheduled_version_change = Aleph::finality_version_change();
        assert_eq!(scheduled_version_change, None);

        let version_to_schedule = VersionChange {
            version_incoming: 1,
            session: 5,
        };

        let scheduling_result = Aleph::do_schedule_finality_version_change(version_to_schedule);
        assert!(scheduling_result.is_err());
    })
}
