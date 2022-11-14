use frame_support::{assert_err, assert_ok, sp_runtime, BoundedVec};
use frame_system::{pallet_prelude::OriginFor, Config};
use sp_runtime::traits::Get;

use super::setup::*;
use crate::{Error, ProvingSystem, VerificationKeyIdentifier, VerificationKeys};

type Snarcos = crate::Pallet<TestRuntime>;

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
    <TestRuntime as Config>::Origin::signed(0)
}

fn put_key() {
    VerificationKeys::<TestRuntime>::insert(IDENTIFIER, BoundedVec::try_from(vk()).unwrap());
}

#[test]
fn stores_vk_with_fresh_identifier() {
    new_test_ext().execute_with(|| {
        assert_ok!(Snarcos::store_key(caller(), IDENTIFIER, vk()));

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
            Snarcos::store_key(caller(), IDENTIFIER, vk()),
            Error::<TestRuntime>::IdentifierAlreadyInUse
        );
    });
}

#[test]
fn does_not_store_too_long_key() {
    new_test_ext().execute_with(|| {
        let limit: u32 = <TestRuntime as crate::Config>::MaximumVerificationKeyLength::get();

        assert_err!(
            Snarcos::store_key(caller(), IDENTIFIER, vec![0; (limit + 1) as usize]),
            Error::<TestRuntime>::VerificationKeyTooLong
        );
    });
}

#[test]
fn verifies_proof() {
    new_test_ext().execute_with(|| {
        put_key();

        assert_ok!(Snarcos::verify(
            caller(),
            IDENTIFIER,
            proof(),
            input(),
            SYSTEM
        ));
    });
}

#[test]
fn verify_shouts_when_no_key_was_registered() {
    new_test_ext().execute_with(|| {
        assert_err!(
            Snarcos::verify(caller(), IDENTIFIER, proof(), input(), SYSTEM),
            Error::<TestRuntime>::UnknownVerificationKeyIdentifier
        );
    });
}

#[test]
fn verify_shouts_when_key_is_not_decodable() {
    new_test_ext().execute_with(|| {
        VerificationKeys::<TestRuntime>::insert(
            IDENTIFIER,
            BoundedVec::try_from(vec![0, 1, 2]).unwrap(),
        );

        assert_err!(
            Snarcos::verify(caller(), IDENTIFIER, proof(), input(), SYSTEM),
            Error::<TestRuntime>::DeserializingVerificationKeyFailed
        );
    });
}

#[test]
fn verify_shouts_when_proof_is_not_decodable() {
    new_test_ext().execute_with(|| {
        put_key();

        assert_err!(
            Snarcos::verify(caller(), IDENTIFIER, input(), input(), SYSTEM),
            Error::<TestRuntime>::DeserializingProofFailed
        );
    });
}

#[test]
fn verify_shouts_when_input_is_not_decodable() {
    new_test_ext().execute_with(|| {
        put_key();

        assert_err!(
            Snarcos::verify(caller(), IDENTIFIER, proof(), proof(), SYSTEM),
            Error::<TestRuntime>::DeserializingPublicInputFailed
        );
    });
}

#[test]
fn verify_shouts_when_verification_fails() {
    new_test_ext().execute_with(|| {
        put_key();
        let other_input = include_bytes!("../resources/groth16/linear_equation.public_input.bytes");

        assert_err!(
            Snarcos::verify(caller(), IDENTIFIER, proof(), other_input.to_vec(), SYSTEM),
            Error::<TestRuntime>::VerificationFailed
        );
    });
}

#[test]
fn verify_shouts_when_proof_is_incorrect() {
    new_test_ext().execute_with(|| {
        put_key();
        let other_proof = include_bytes!("../resources/groth16/linear_equation.proof.bytes");

        assert_err!(
            Snarcos::verify(caller(), IDENTIFIER, other_proof.to_vec(), input(), SYSTEM),
            Error::<TestRuntime>::IncorrectProof
        );
    });
}
