use frame_support::{assert_err, assert_ok, error::BadOrigin, sp_runtime, BoundedVec};
use frame_system::{pallet_prelude::OriginFor, Config};
use sp_runtime::traits::Get;

use super::setup::*;
use crate::{Error, ProvingSystem, VerificationError, VerificationKeyIdentifier, VerificationKeys};

type BabyLiminal = crate::Pallet<TestRuntime>;

const IDENTIFIER: VerificationKeyIdentifier = [0; 4];
const SYSTEM: ProvingSystem = ProvingSystem::Groth16;

fn vk() -> Vec<u8> {
    include_bytes!("../resources/groth16/xor.vk.bytes").to_vec()
}

fn proof() -> Vec<u8> {
    include_bytes!("../resources/groth16/xor.proof.bytes").to_vec()
}

fn input() -> Vec<u8> {
    include_bytes!("../resources/groth16/xor.public_input.bytes").to_vec()
}

fn caller() -> OriginFor<TestRuntime> {
    <TestRuntime as Config>::RuntimeOrigin::signed(0)
}

fn root() -> OriginFor<TestRuntime> {
    <TestRuntime as Config>::RuntimeOrigin::root()
}

fn put_key() {
    VerificationKeys::<TestRuntime>::insert(IDENTIFIER, BoundedVec::try_from(vk()).unwrap());
}

#[test]
fn stores_vk_with_fresh_identifier() {
    new_test_ext().execute_with(|| {
        assert_ok!(BabyLiminal::store_key(caller(), IDENTIFIER, vk()));

        let stored_key = VerificationKeys::<TestRuntime>::get(IDENTIFIER);
        assert!(stored_key.is_some());
        assert_eq!(stored_key.unwrap().to_vec(), vk());
    });
}

#[test]
fn does_not_overwrite_registered_key() {
    new_test_ext().execute_with(|| {
        put_key();

        assert_err!(
            BabyLiminal::store_key(caller(), IDENTIFIER, vk()),
            Error::<TestRuntime>::IdentifierAlreadyInUse
        );
    });
}

#[test]
fn caller_cannot_delete_key() {
    new_test_ext().execute_with(|| {
        put_key();
        assert_err!(BabyLiminal::delete_key(caller(), IDENTIFIER), BadOrigin);
    });
}

#[test]
fn sudo_can_delete_key() {
    new_test_ext().execute_with(|| {
        put_key();
        assert_ok!(BabyLiminal::delete_key(root(), IDENTIFIER));
    });
}

#[test]
fn caller_cannot_overwrite_key() {
    new_test_ext().execute_with(|| {
        put_key();
        assert_err!(
            BabyLiminal::overwrite_key(caller(), IDENTIFIER, vk()),
            BadOrigin
        );
    });
}

#[test]
fn sudo_can_overwrite_key() {
    new_test_ext().execute_with(|| {
        put_key();
        assert_ok!(BabyLiminal::overwrite_key(root(), IDENTIFIER, vk()));
    });
}

#[test]
fn does_not_store_too_long_key() {
    new_test_ext().execute_with(|| {
        let limit: u32 = <TestRuntime as crate::Config>::MaximumVerificationKeyLength::get();

        assert_err!(
            BabyLiminal::store_key(caller(), IDENTIFIER, vec![0; (limit + 1) as usize]),
            Error::<TestRuntime>::VerificationKeyTooLong
        );
    });
}

#[test]
fn verifies_proof() {
    new_test_ext().execute_with(|| {
        put_key();

        assert_ok!(BabyLiminal::verify(
            caller(),
            IDENTIFIER,
            proof(),
            input(),
            SYSTEM
        ));
    });
}

#[test]
fn verify_shouts_when_data_is_too_long() {
    new_test_ext().execute_with(|| {
        let limit: u32 = <TestRuntime as crate::Config>::MaximumDataLength::get();

        let result = BabyLiminal::verify(
            caller(),
            IDENTIFIER,
            vec![0; (limit + 1) as usize],
            input(),
            SYSTEM,
        );
        assert_err!(
            result.map_err(|e| e.error),
            Error::<TestRuntime>::DataTooLong
        );
        assert!(result.unwrap_err().post_info.actual_weight.is_some());

        let result = BabyLiminal::verify(
            caller(),
            IDENTIFIER,
            proof(),
            vec![0; (limit + 1) as usize],
            SYSTEM,
        );
        assert_err!(
            result.map_err(|e| e.error),
            Error::<TestRuntime>::DataTooLong
        );
        assert!(result.unwrap_err().post_info.actual_weight.is_some());
    });
}

#[test]
fn verify_shouts_when_no_key_was_registered() {
    new_test_ext().execute_with(|| {
        let result = BabyLiminal::verify(caller(), IDENTIFIER, proof(), input(), SYSTEM);

        assert_err!(
            result.map_err(|e| e.error),
            Error::<TestRuntime>::UnknownVerificationKeyIdentifier
        );
        assert!(result.unwrap_err().post_info.actual_weight.is_some());
    });
}

#[test]
fn verify_shouts_when_key_is_not_deserializable() {
    new_test_ext().execute_with(|| {
        VerificationKeys::<TestRuntime>::insert(
            IDENTIFIER,
            BoundedVec::try_from(vec![0, 1, 2]).unwrap(),
        );

        let result = BabyLiminal::verify(caller(), IDENTIFIER, proof(), input(), SYSTEM);

        assert_err!(
            result.map_err(|e| e.error),
            Error::<TestRuntime>::DeserializingVerificationKeyFailed
        );
        assert!(result.unwrap_err().post_info.actual_weight.is_some());
    });
}

#[test]
fn verify_shouts_when_proof_is_not_deserializable() {
    new_test_ext().execute_with(|| {
        put_key();

        let result = BabyLiminal::verify(caller(), IDENTIFIER, input(), input(), SYSTEM);

        assert_err!(
            result.map_err(|e| e.error),
            Error::<TestRuntime>::DeserializingProofFailed
        );
        assert!(result.unwrap_err().post_info.actual_weight.is_some());
    });
}

#[test]
fn verify_shouts_when_input_is_not_deserializable() {
    new_test_ext().execute_with(|| {
        put_key();

        let result = BabyLiminal::verify(caller(), IDENTIFIER, proof(), proof(), SYSTEM);

        assert_err!(
            result.map_err(|e| e.error),
            Error::<TestRuntime>::DeserializingPublicInputFailed
        );
        assert!(result.unwrap_err().post_info.actual_weight.is_some());
    });
}

#[test]
fn verify_shouts_when_verification_fails() {
    new_test_ext().execute_with(|| {
        put_key();
        let other_input = include_bytes!("../resources/groth16/linear_equation.public_input.bytes");

        let result =
            BabyLiminal::verify(caller(), IDENTIFIER, proof(), other_input.to_vec(), SYSTEM);

        assert_err!(
            result,
            Error::<TestRuntime>::VerificationFailed(VerificationError::MalformedVerifyingKey)
        );
        assert!(result.unwrap_err().post_info.actual_weight.is_none());
    });
}

#[test]
fn verify_shouts_when_proof_is_incorrect() {
    new_test_ext().execute_with(|| {
        put_key();
        let other_proof = include_bytes!("../resources/groth16/linear_equation.proof.bytes");

        let result =
            BabyLiminal::verify(caller(), IDENTIFIER, other_proof.to_vec(), input(), SYSTEM);

        assert_err!(result, Error::<TestRuntime>::IncorrectProof);
        assert!(result.unwrap_err().post_info.actual_weight.is_none());
    });
}
