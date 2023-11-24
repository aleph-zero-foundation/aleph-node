mod arguments;
mod environment;
mod executor;

use aleph_runtime::Runtime as AlephRuntime;
use pallet_baby_liminal::Error::*;
use pallet_contracts::chain_extension::{ChainExtension, RetVal};

use crate::{
    backend::{
        executor::BackendExecutor,
        tests::{
            arguments::{store_key_args, verify_args},
            environment::{MockedEnvironment, StandardMode, StoreKeyMode, VerifyMode},
            executor::{StoreKeyErrorer, StoreKeyOkayer, VerifyErrorer, VerifyOkayer},
        },
    },
    status_codes::*,
    BabyLiminalChainExtension,
};

fn simulate_store_key<Exc: BackendExecutor>(expected_ret_val: u32) {
    let env = MockedEnvironment::<StoreKeyMode, StandardMode>::new(store_key_args());
    let result = BabyLiminalChainExtension::<AlephRuntime>::store_key::<Exc, _>(env);
    assert!(matches!(result, Ok(RetVal::Converging(ret_val)) if ret_val == expected_ret_val));
}

fn simulate_verify<Exc: BackendExecutor>(expected_ret_val: u32) {
    let env = MockedEnvironment::<VerifyMode, StandardMode>::new(verify_args());
    let result = BabyLiminalChainExtension::<AlephRuntime>::verify::<Exc, _>(env);
    assert!(matches!(result, Ok(RetVal::Converging(ret_val)) if ret_val == expected_ret_val));
}

#[test]
fn extension_is_enabled() {
    assert!(BabyLiminalChainExtension::<AlephRuntime>::enabled())
}

#[test]
#[allow(non_snake_case)]
fn store_key__positive_scenario() {
    simulate_store_key::<StoreKeyOkayer>(STORE_KEY_SUCCESS)
}

#[test]
#[allow(non_snake_case)]
fn store_key__pallet_says_too_long_vk() {
    simulate_store_key::<StoreKeyErrorer<{ VerificationKeyTooLong }>>(STORE_KEY_TOO_LONG_KEY)
}

#[test]
#[allow(non_snake_case)]
fn store_key__pallet_says_identifier_in_use() {
    simulate_store_key::<StoreKeyErrorer<{ IdentifierAlreadyInUse }>>(STORE_KEY_IDENTIFIER_IN_USE)
}

#[test]
#[allow(non_snake_case)]
fn verify__positive_scenario() {
    simulate_verify::<VerifyOkayer>(VERIFY_SUCCESS)
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_proof_deserialization_failed() {
    simulate_verify::<VerifyErrorer<{ DeserializingProofFailed }>>(VERIFY_DESERIALIZING_PROOF_FAIL)
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_input_deserialization_failed() {
    simulate_verify::<VerifyErrorer<{ DeserializingPublicInputFailed }>>(
        VERIFY_DESERIALIZING_INPUT_FAIL,
    )
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_no_such_vk() {
    simulate_verify::<VerifyErrorer<{ UnknownVerificationKeyIdentifier }>>(
        VERIFY_UNKNOWN_IDENTIFIER,
    )
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_vk_deserialization_failed() {
    simulate_verify::<VerifyErrorer<{ DeserializingVerificationKeyFailed }>>(
        VERIFY_DESERIALIZING_KEY_FAIL,
    )
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_verification_failed() {
    simulate_verify::<VerifyErrorer<{ VerificationFailed }>>(VERIFY_VERIFICATION_FAIL)
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_incorrect_proof() {
    simulate_verify::<VerifyErrorer<{ IncorrectProof }>>(VERIFY_INCORRECT_PROOF)
}
