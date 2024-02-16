use frame_support::{assert_err, assert_ok, pallet_prelude::Get};
use frame_system::{pallet_prelude::OriginFor, Config};
use sp_core::Hasher;

use super::setup::*;
use crate::{Error, KeyHash, KeyHasher, VerificationKeys};

type VkStorage = crate::Pallet<TestRuntime>;

fn vk() -> Vec<u8> {
    vec![41; 1000]
}

fn vk_hash() -> KeyHash {
    KeyHasher::hash(&vk())
}

fn caller() -> OriginFor<TestRuntime> {
    <TestRuntime as Config>::RuntimeOrigin::signed(1)
}

#[test]
fn stores_new_vk() {
    new_test_ext().execute_with(|| {
        assert_ok!(VkStorage::store_key(caller(), vk()));

        let stored_key = VerificationKeys::<TestRuntime>::get(vk_hash());
        assert!(stored_key.is_some());
        assert_eq!(stored_key.unwrap().to_vec(), vk());
    });
}

#[test]
fn overwrite_is_idempotent() {
    new_test_ext().execute_with(|| {
        assert_ok!(VkStorage::store_key(caller(), vk()));
        assert_ok!(VkStorage::store_key(caller(), vk()));
        assert_ok!(VkStorage::store_key(caller(), vk()));

        let stored_key = VerificationKeys::<TestRuntime>::get(vk_hash());
        assert!(stored_key.is_some());
        assert_eq!(stored_key.unwrap().to_vec(), vk());
    });
}

#[test]
fn does_not_store_too_long_key() {
    new_test_ext().execute_with(|| {
        let limit: u32 = <TestRuntime as crate::Config>::MaximumKeyLength::get();

        assert_err!(
            VkStorage::store_key(caller(), vec![0; (limit + 1) as usize]),
            Error::<TestRuntime>::VerificationKeyTooLong
        );
    });
}
