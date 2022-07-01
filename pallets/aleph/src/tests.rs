#![cfg(test)]

use std::collections::HashMap;

use frame_support::{
    storage::migration::{get_storage_value, put_storage_value},
    storage_alias,
    traits::{GetStorageVersion, OneSessionHandler, StorageVersion},
};

use crate::{migrations, mock::*, pallet};

#[storage_alias]
type SessionForValidatorsChange = StorageValue<Aleph, u32>;

#[storage_alias]
type Validators<T> = StorageValue<Aleph, Vec<<T as frame_system::Config>::AccountId>>;

#[test]
fn migration_from_v0_to_v1_works() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        put_storage_value(b"Aleph", b"SessionForValidatorsChange", &[], Some(7u32));

        let before = get_storage_value::<Option<u32>>(b"Aleph", b"SessionForValidatorsChange", &[]);

        assert_eq!(
            before,
            Some(Some(7)),
            "Storage before migration has type Option<u32>"
        );

        put_storage_value(
            b"Aleph",
            b"Validators",
            &[],
            Some(vec![AccountId::default()]),
        );

        let v0 = <pallet::Pallet<Test> as GetStorageVersion>::on_chain_storage_version();

        assert_eq!(
            v0,
            StorageVersion::default(),
            "Storage version before applying migration should be default",
        );

        let _weight = migrations::v0_to_v1::migrate::<Test, Aleph>();

        let v1 = <pallet::Pallet<Test> as GetStorageVersion>::on_chain_storage_version();

        assert_ne!(
            v1,
            StorageVersion::default(),
            "Storage version after applying migration should be incremented"
        );

        assert_eq!(
            SessionForValidatorsChange::get(),
            Some(7u32),
            "Migration should preserve ongoing session change with respect to the session number"
        );

        assert_eq!(
            Validators::<Test>::get(),
            Some(vec![AccountId::default()]),
            "Migration should preserve ongoing session change with respect to the validators set"
        );
    })
}

#[test]
fn migration_from_v1_to_v2_works() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        let map = [
            "SessionForValidatorsChange",
            "Validators",
            "MillisecsPerBlock",
            "SessionPeriod",
        ]
        .iter()
        .zip(0..4)
        .collect::<HashMap<_, _>>();

        map.iter().for_each(|(item, value)| {
            put_storage_value(b"Aleph", item.as_bytes(), &[], value);
        });

        let _weight = migrations::v1_to_v2::migrate::<Test, Aleph>();

        let v2 = <pallet::Pallet<Test> as GetStorageVersion>::on_chain_storage_version();

        assert_eq!(
            v2,
            StorageVersion::new(2),
            "Storage version after applying migration should be incremented"
        );

        for item in map.keys() {
            assert!(
                get_storage_value::<i32>(b"Aleph", item.as_bytes(), &[]).is_none(),
                "Storage item {} should be killed",
                item
            );
        }
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

        assert_eq!(Aleph::authorities(), to_authorities(&[1, 2, 3]));
    })
}

#[test]
fn test_session_rotation() {
    new_test_ext(&[(1u64, 1u64), (2u64, 2u64)]).execute_with(|| {
        initialize_session();
        run_session(1);

        let new_validators = new_session_validators(&[3u64, 4u64]);
        let queued_validators = new_session_validators(&[]);
        Aleph::on_new_session(true, new_validators, queued_validators);
        assert_eq!(Aleph::authorities(), to_authorities(&[3, 4]));
    })
}
