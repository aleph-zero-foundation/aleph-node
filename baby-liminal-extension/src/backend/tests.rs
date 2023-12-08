mod arguments;
mod environment;
mod executor;

use aleph_runtime::Runtime as AlephRuntime;
use frame_support::pallet_prelude::Weight;
use pallet_baby_liminal::{Error::*, WeightInfo};
use pallet_contracts::chain_extension::{ChainExtension, RetVal};

use crate::{
    backend::{
        executor::BackendExecutor,
        tests::{
            arguments::verify_args,
            environment::{CorruptedMode, MockedEnvironment, StandardMode, VerifyMode},
            executor::{Panicker, VerifyErrorer, VerifyOkayer},
        },
    },
    status_codes::*,
    BabyLiminalChainExtension,
};

fn simulate_verify<Exc: BackendExecutor>(expected_ret_val: u32) {
    let mut charged = Weight::zero();
    let env = MockedEnvironment::<VerifyMode, StandardMode>::new(&mut charged, verify_args());

    let result = BabyLiminalChainExtension::<AlephRuntime>::verify::<Exc, _, ()>(env);

    assert!(matches!(result, Ok(RetVal::Converging(ret_val)) if ret_val == expected_ret_val));
    assert_eq!(charged, <()>::verify());
}

#[test]
fn extension_is_enabled() {
    assert!(BabyLiminalChainExtension::<AlephRuntime>::enabled())
}

#[test]
#[allow(non_snake_case)]
fn verify__charges_before_reading_arguments() {
    let mut charged = Weight::zero();
    // `CorruptedMode` ensures that the CE call will fail at argument reading/decoding phase.
    let env = MockedEnvironment::<VerifyMode, CorruptedMode>::new(&mut charged, 41);

    // `Panicker` ensures that the call will not be forwarded to the pallet.
    let result = BabyLiminalChainExtension::<AlephRuntime>::verify::<Panicker, _, ()>(env);

    assert!(matches!(result, Err(_)));
    assert_eq!(charged, <()>::verify());
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
