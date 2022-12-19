use core::iter::Sum;
use std::{ops::Neg, sync::mpsc::Receiver};

use environment::{CorruptedMode, MockedEnvironment, StandardMode, StoreKeyMode, VerifyMode};

use super::*;
use crate::chain_extension::tests::executor::{
    Panicker, StoreKeyErrorer, StoreKeyOkayer, VerifyErrorer, VerifyOkayer,
};

mod environment;
mod executor;

/// In order to compute final fee (after all adjustments) sometimes we will have to subtract
/// weights.
#[derive(Debug, PartialEq, Eq)]
pub struct RevertibleWeight(i64);

impl RevertibleWeight {
    fn neg(weight: Weight) -> Self {
        RevertibleWeight((weight.ref_time() as i64).neg())
    }

    const ZERO: RevertibleWeight = RevertibleWeight(0);
}

impl From<Weight> for RevertibleWeight {
    fn from(w: Weight) -> Self {
        RevertibleWeight(w.ref_time() as i64)
    }
}

impl Sum for RevertibleWeight {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        RevertibleWeight(iter.map(|rw| rw.0).sum())
    }
}

const IDENTIFIER: VerificationKeyIdentifier = [1, 7, 2, 9];
const VK: [u8; 2] = [4, 1];
const PROOF: [u8; 20] = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2, 3, 8, 4];
const INPUT: [u8; 11] = [0, 5, 7, 7, 2, 1, 5, 6, 6, 4, 9];
const SYSTEM: ProvingSystem = ProvingSystem::Groth16;

/// Returns encoded arguments to `store_key`.
fn store_key_args() -> Vec<u8> {
    StoreKeyArgs {
        identifier: IDENTIFIER,
        key: VK.to_vec(),
    }
    .encode()
}

/// Returns encoded arguments to `verify`.
fn verify_args() -> Vec<u8> {
    VerifyArgs {
        identifier: IDENTIFIER,
        proof: PROOF.to_vec(),
        input: INPUT.to_vec(),
        system: SYSTEM,
    }
    .encode()
}

/// Fetches all charges and computes the final fee.
fn charged(charging_listener: Receiver<RevertibleWeight>) -> RevertibleWeight {
    charging_listener.into_iter().sum()
}

#[test]
fn extension_is_enabled() {
    assert!(SnarcosChainExtension::enabled())
}

#[test]
#[allow(non_snake_case)]
fn store_key__charges_before_reading() {
    let (env, charging_listener) = MockedEnvironment::<StoreKeyMode, CorruptedMode>::new(41, None);
    let key_length = env.approx_key_len();

    let result = SnarcosChainExtension::snarcos_store_key::<_, Panicker>(env);

    assert!(matches!(result, Err(_)));
    assert_eq!(
        charged(charging_listener),
        weight_of_store_key(key_length).into()
    );
}

#[test]
#[allow(non_snake_case)]
fn store_key__too_much_to_read() {
    let (env, charging_listener) = MockedEnvironment::<StoreKeyMode, CorruptedMode>::new(
        ByteCount::MAX,
        Some(Box::new(|| panic!("Shouldn't read anything at all"))),
    );

    let result = SnarcosChainExtension::snarcos_store_key::<_, Panicker>(env);

    assert!(matches!(
        result,
        Ok(RetVal::Converging(SNARCOS_STORE_KEY_TOO_LONG_KEY))
    ));
    assert_eq!(charged(charging_listener), RevertibleWeight::ZERO);
}

fn simulate_store_key<Exc: Executor>(expected_ret_val: u32) {
    let (env, charging_listener) =
        MockedEnvironment::<StoreKeyMode, StandardMode>::new(store_key_args());

    let result = SnarcosChainExtension::snarcos_store_key::<_, Exc>(env);

    assert!(matches!(result, Ok(RetVal::Converging(ret_val)) if ret_val == expected_ret_val));
    assert_eq!(
        charged(charging_listener),
        weight_of_store_key(VK.len() as ByteCount).into()
    );
}

#[test]
#[allow(non_snake_case)]
fn store_key__pallet_says_too_long_vk() {
    simulate_store_key::<StoreKeyErrorer<{ VerificationKeyTooLong }>>(
        SNARCOS_STORE_KEY_TOO_LONG_KEY,
    )
}

#[test]
#[allow(non_snake_case)]
fn store_key__pallet_says_identifier_in_use() {
    simulate_store_key::<StoreKeyErrorer<{ IdentifierAlreadyInUse }>>(
        SNARCOS_STORE_KEY_IDENTIFIER_IN_USE,
    )
}

#[test]
#[allow(non_snake_case)]
fn store_key__positive_scenario() {
    simulate_store_key::<StoreKeyOkayer>(SNARCOS_STORE_KEY_OK)
}

#[test]
#[allow(non_snake_case)]
fn verify__charges_before_reading() {
    let (env, charging_listener) = MockedEnvironment::<VerifyMode, CorruptedMode>::new(41, None);

    let result = SnarcosChainExtension::snarcos_verify::<_, Panicker>(env);

    assert!(matches!(result, Err(_)));
    assert_eq!(charged(charging_listener), weight_of_verify(None).into());
}

const ADJUSTED_WEIGHT: u64 = 1729;

// Unfortunately, due to the `unconstrained generic constant` error, `ACTUAL_WEIGHT` will have to be
// passed twice for failure tests (once to `VerifyErrorer` and second time as a separate value).
fn simulate_verify<Exc: Executor, const ACTUAL_WEIGHT: Option<u64>, const EXPECTED_RET_VAL: u32>() {
    let (env, charging_listener) =
        MockedEnvironment::<VerifyMode, StandardMode>::new(verify_args());

    let result = SnarcosChainExtension::snarcos_verify::<_, Exc>(env);

    assert!(matches!(result, Ok(RetVal::Converging(ret_val)) if ret_val == EXPECTED_RET_VAL));

    let expected_charge =
        ACTUAL_WEIGHT.unwrap_or_else(|| weight_of_verify(Some(SYSTEM)).ref_time());
    assert_eq!(
        charged(charging_listener),
        Weight::from_ref_time(expected_charge).into()
    );
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_proof_deserialization_failed() {
    simulate_verify::<
        VerifyErrorer<{ DeserializingProofFailed }, { Some(ADJUSTED_WEIGHT) }>,
        { Some(ADJUSTED_WEIGHT) },
        SNARCOS_VERIFY_DESERIALIZING_PROOF_FAIL,
    >()
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_input_deserialization_failed() {
    simulate_verify::<
        VerifyErrorer<{ DeserializingPublicInputFailed }, { Some(ADJUSTED_WEIGHT) }>,
        { Some(ADJUSTED_WEIGHT) },
        SNARCOS_VERIFY_DESERIALIZING_INPUT_FAIL,
    >()
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_no_such_vk() {
    simulate_verify::<
        VerifyErrorer<{ UnknownVerificationKeyIdentifier }, { Some(ADJUSTED_WEIGHT) }>,
        { Some(ADJUSTED_WEIGHT) },
        SNARCOS_VERIFY_UNKNOWN_IDENTIFIER,
    >()
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_vk_deserialization_failed() {
    simulate_verify::<
        VerifyErrorer<{ DeserializingVerificationKeyFailed }, { Some(ADJUSTED_WEIGHT) }>,
        { Some(ADJUSTED_WEIGHT) },
        SNARCOS_VERIFY_DESERIALIZING_KEY_FAIL,
    >()
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_verification_failed() {
    simulate_verify::<
        VerifyErrorer<{ VerificationFailed }, { None }>,
        { None },
        SNARCOS_VERIFY_VERIFICATION_FAIL,
    >()
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_incorrect_proof() {
    simulate_verify::<
        VerifyErrorer<{ IncorrectProof }, { None }>,
        { None },
        SNARCOS_VERIFY_INCORRECT_PROOF,
    >()
}

#[test]
#[allow(non_snake_case)]
fn verify__positive_scenario() {
    simulate_verify::<VerifyOkayer, { None }, SNARCOS_VERIFY_OK>()
}
