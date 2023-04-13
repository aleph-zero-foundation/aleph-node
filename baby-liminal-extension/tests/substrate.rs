#![cfg_attr(test, allow(incomplete_features))]
#![cfg_attr(test, feature(adt_const_params))]
#![cfg_attr(test, feature(generic_const_exprs))]

mod utils;

use aleph_runtime::Runtime;
use baby_liminal_extension::{
    substrate::{weight_of_store_key, Extension},
    BABY_LIMINAL_STORE_KEY_IDENTIFIER_IN_USE, BABY_LIMINAL_STORE_KEY_TOO_LONG_KEY,
    BABY_LIMINAL_VERIFY_DESERIALIZING_INPUT_FAIL, BABY_LIMINAL_VERIFY_DESERIALIZING_KEY_FAIL,
    BABY_LIMINAL_VERIFY_DESERIALIZING_PROOF_FAIL, BABY_LIMINAL_VERIFY_INCORRECT_PROOF,
    BABY_LIMINAL_VERIFY_UNKNOWN_IDENTIFIER, BABY_LIMINAL_VERIFY_VERIFICATION_FAIL,
};
use obce::substrate::{pallet_contracts::chain_extension::RetVal, CallableChainExtension};
use pallet_baby_liminal::{Config as BabyLiminalConfig, Error, VerificationError, WeightInfo};
use utils::{
    charged, simulate_store_key, simulate_verify, store_key_args, verify_args, CorruptedMode,
    MockedEnvironment, Responder, RevertibleWeight, StoreKeyErrorer, StoreKeyOkayer, VerifyErrorer,
    VerifyOkayer, ADJUSTED_WEIGHT, STORE_KEY_ID, VERIFY_ID,
};

#[test]
#[allow(non_snake_case)]
fn store_key__charges_before_reading() {
    let (env, charging_listener) = MockedEnvironment::<
        STORE_KEY_ID,
        CorruptedMode,
        { Responder::Panicker },
        { Responder::Panicker },
    >::new(41, None);

    let key_length = env.approx_key_len();

    let result = <Extension as CallableChainExtension<(), Runtime, _>>::call(&mut Extension, env);

    assert!(matches!(result, Err(_)));
    assert_eq!(
        charged(charging_listener),
        weight_of_store_key::<Runtime>(key_length).into()
    );
}

#[test]
#[allow(non_snake_case)]
fn store_key__too_much_to_read() {
    let (env, charging_listener) = MockedEnvironment::<
        STORE_KEY_ID,
        CorruptedMode,
        { Responder::Panicker },
        { Responder::Panicker },
    >::new(
        u32::MAX,
        Some(Box::new(|| panic!("Shouldn't read anything at all"))),
    );

    let result = <Extension as CallableChainExtension<(), Runtime, _>>::call(&mut Extension, env);

    assert!(matches!(
        result,
        Ok(RetVal::Converging(BABY_LIMINAL_STORE_KEY_TOO_LONG_KEY))
    ));
    assert_eq!(charged(charging_listener), RevertibleWeight::ZERO);
}

#[test]
#[allow(non_snake_case)]
fn store_key__pallet_says_too_long_vk() {
    simulate_store_key(
        StoreKeyErrorer::<{ Error::VerificationKeyTooLong }>::new(store_key_args()),
        BABY_LIMINAL_STORE_KEY_TOO_LONG_KEY,
    )
}

#[test]
#[allow(non_snake_case)]
fn store_key__pallet_says_identifier_in_use() {
    simulate_store_key(
        StoreKeyErrorer::<{ Error::IdentifierAlreadyInUse }>::new(store_key_args()),
        BABY_LIMINAL_STORE_KEY_IDENTIFIER_IN_USE,
    )
}

#[test]
#[allow(non_snake_case)]
fn store_key__positive_scenario() {
    simulate_store_key(StoreKeyOkayer::new(store_key_args()), 0)
}

#[test]
#[allow(non_snake_case)]
fn verify__charges_before_reading() {
    let (env, charging_listener) = MockedEnvironment::<
        VERIFY_ID,
        CorruptedMode,
        { Responder::Panicker },
        { Responder::Panicker },
    >::new(41, None);

    let result = <Extension as CallableChainExtension<(), Runtime, _>>::call(&mut Extension, env);

    assert!(matches!(result, Err(_)));
    assert_eq!(
        charged(charging_listener),
        <<Runtime as BabyLiminalConfig>::WeightInfo as WeightInfo>::verify().into()
    );
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_proof_deserialization_failed() {
    simulate_verify::<_, { Some(ADJUSTED_WEIGHT) }, { BABY_LIMINAL_VERIFY_DESERIALIZING_PROOF_FAIL }>(
        VerifyErrorer::<{ Error::DeserializingProofFailed }, { Some(ADJUSTED_WEIGHT) }>::new(
            verify_args(),
        ),
    )
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_input_deserialization_failed() {
    simulate_verify::<_, { Some(ADJUSTED_WEIGHT) }, { BABY_LIMINAL_VERIFY_DESERIALIZING_INPUT_FAIL }>(
        VerifyErrorer::<{ Error::DeserializingPublicInputFailed }, { Some(ADJUSTED_WEIGHT) }>::new(
            verify_args(),
        ),
    )
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_no_such_vk() {
    simulate_verify::<_, { Some(ADJUSTED_WEIGHT) }, { BABY_LIMINAL_VERIFY_UNKNOWN_IDENTIFIER }>(
        VerifyErrorer::<{ Error::UnknownVerificationKeyIdentifier }, { Some(ADJUSTED_WEIGHT) }>::new(
            verify_args(),
        ),
    )
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_vk_deserialization_failed() {
    simulate_verify::<_, { Some(ADJUSTED_WEIGHT) }, { BABY_LIMINAL_VERIFY_DESERIALIZING_KEY_FAIL }>(
        VerifyErrorer::<{ Error::DeserializingVerificationKeyFailed }, { Some(ADJUSTED_WEIGHT) }>::new(
            verify_args(),
        ),
    )
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_verification_failed() {
    simulate_verify::<_, { None }, { BABY_LIMINAL_VERIFY_VERIFICATION_FAIL }>(VerifyErrorer::<
        { Error::VerificationFailed(VerificationError::MalformedVerifyingKey) },
        { None },
    >::new(
        verify_args()
    ))
}

#[test]
#[allow(non_snake_case)]
fn verify__pallet_says_incorrect_proof() {
    simulate_verify::<_, { None }, { BABY_LIMINAL_VERIFY_INCORRECT_PROOF }>(VerifyErrorer::<
        { Error::IncorrectProof },
        { None },
    >::new(
        verify_args()
    ))
}

#[test]
#[allow(non_snake_case)]
fn verify__positive_scenario() {
    simulate_verify::<_, { None }, 0>(VerifyOkayer::new(verify_args()))
}
