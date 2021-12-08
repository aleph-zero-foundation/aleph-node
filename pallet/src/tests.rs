#![cfg(test)]

use crate::{migrations, mock::*, pallet};
use frame_support::assert_ok;
use frame_support::traits::{GetStorageVersion, StorageVersion};

#[test]
fn migration_from_v0_to_v1_works() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        frame_support::migration::put_storage_value(
            b"Aleph",
            b"SessionForValidatorsChange",
            &[],
            1u32,
        );

        frame_support::migration::put_storage_value(
            b"Aleph",
            b"Validators",
            &[],
            vec![AccountId::default()],
        );

        let v0 = <pallet::Pallet<Test> as GetStorageVersion>::on_chain_storage_version();

        assert_eq!(
            v0,
            StorageVersion::default(),
            "Storage version before applying migration should be default"
        );

        let _weight = migrations::v0_to_v1::migrate::<Test, Aleph>();

        let v1 = <pallet::Pallet<Test> as GetStorageVersion>::on_chain_storage_version();

        assert_ne!(
            v1,
            StorageVersion::default(),
            "Storage version after applying migration should be incremented"
        );

        assert_eq!(
            Aleph::session_for_validators_change(),
            Some(1u32),
            "Migration should preserve ongoing session change with respect to the session number"
        );

        assert_eq!(
            Aleph::validators(),
            Some(vec![AccountId::default()]),
            "Migration should preserve ongoing session change with respect to the validators set"
        );

        let noop_weight = migrations::v0_to_v1::migrate::<Test, Aleph>();
        assert_eq!(
            noop_weight,
            TestDbWeight::get().reads(1),
            "Migration cannot be run twice"
        );
    })
}

#[test]
fn test_update_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();
        run_session(1);

        Aleph::update_authorities(to_authorities(&[2, 3, 4]).as_slice());

        assert_eq!(Aleph::authorities(), to_authorities(&[2, 3, 4]));
    });
}

#[test]
fn test_initialize_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2]));
    });
}

#[test]
fn test_validators_should_be_none() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        assert_eq!(Aleph::validators(), None);
    });
}

#[test]
fn test_change_validators() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        assert_ok!(Aleph::change_validators(
            Origin::root(),
            vec![AccountId::default()],
            0
        ));

        assert_eq!(Aleph::session_for_validators_change(), Some(0));
        assert_eq!(Aleph::validators(), Some(vec![AccountId::default()]));
    });
}

#[test]
#[should_panic]
fn fails_to_initialize_again_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        Aleph::initialize_authorities(&to_authorities(&[1, 2, 3]));
    });
}

#[test]
fn test_current_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();

        run_session(1);

        Aleph::update_authorities(to_authorities(&[2, 3, 4]).as_slice());

        assert_eq!(Aleph::authorities(), to_authorities(&[2, 3, 4]));

        run_session(2);

        Aleph::update_authorities(to_authorities(&[1, 2, 3]).as_slice());

        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2, 3]),);
    })
}

#[test]
fn test_next_session_authorities() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();

        run_session(1);

        assert_eq!(
            Aleph::next_session_authorities().unwrap(),
            to_authorities(&[1, 2])
        );

        run_session(2);

        assert_eq!(
            Aleph::next_session_authorities().unwrap(),
            to_authorities(&[1, 2])
        );
    })
}
