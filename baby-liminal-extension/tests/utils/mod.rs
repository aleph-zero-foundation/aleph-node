use std::sync::mpsc::Receiver;

use aleph_runtime::Runtime;
use baby_liminal_extension::{
    executor::Executor,
    substrate::{weight_of_store_key, Extension},
    BabyLiminalExtension, VerificationKeyIdentifier,
};
use obce::substrate::{
    frame_support::weights::Weight, pallet_contracts::chain_extension::RetVal,
    sp_runtime::AccountId32, CallableChainExtension, ChainExtensionEnvironment,
};
use scale::{Decode, Encode};

mod environment;

pub use environment::{
    CorruptedMode, MockedEnvironment, Responder, RevertibleWeight, StandardMode, StoreKeyErrorer,
    StoreKeyOkayer, VerifyErrorer, VerifyOkayer,
};
use pallet_baby_liminal::{Config as BabyLiminalConfig, WeightInfo};

pub const STORE_KEY_ID: u16 = obce::id!(BabyLiminalExtension::store_key);
pub const VERIFY_ID: u16 = obce::id!(BabyLiminalExtension::verify);

const IDENTIFIER: VerificationKeyIdentifier = [1, 7, 2, 9, 1, 7, 2, 9];
const VK: [u8; 2] = [4, 1];
const PROOF: [u8; 20] = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2, 3, 8, 4];
const INPUT: [u8; 11] = [0, 5, 7, 7, 2, 1, 5, 6, 6, 4, 9];

/// Struct to be decoded from a byte slice passed from the contract.
///
/// Notice, that contract can pass these arguments one by one, not necessarily as such struct. Only
/// the order of values is important.
///
/// It cannot be `MaxEncodedLen` due to `Vec<_>` and thus `Environment::read_as` cannot be used.
#[derive(Decode, Encode)]
struct StoreKeyArgs {
    pub origin: AccountId32,
    pub identifier: VerificationKeyIdentifier,
    pub key: Vec<u8>,
}

/// Struct to be decoded from a byte slice passed from the contract.
///
/// Notice, that contract can pass these arguments one by one, not necessarily as such struct. Only
/// the order of values is important.
///
/// It cannot be `MaxEncodedLen` due to `Vec<_>` and thus `Environment::read_as` cannot be used.
#[derive(Decode, Encode)]
struct VerifyArgs {
    pub identifier: VerificationKeyIdentifier,
    pub proof: Vec<u8>,
    pub input: Vec<u8>,
}

/// Returns encoded arguments to `store_key`.
pub fn store_key_args() -> Vec<u8> {
    StoreKeyArgs {
        origin: AccountId32::from([0; 32]),
        identifier: IDENTIFIER,
        key: VK.to_vec(),
    }
    .encode()
}

/// Returns encoded arguments to `verify`.
pub fn verify_args() -> Vec<u8> {
    VerifyArgs {
        identifier: IDENTIFIER,
        proof: PROOF.to_vec(),
        input: INPUT.to_vec(),
    }
    .encode()
}

/// Fetches all charges and computes the final fee.
pub fn charged(charging_listener: Receiver<RevertibleWeight>) -> RevertibleWeight {
    charging_listener.into_iter().sum()
}

pub fn simulate_store_key<Env>(
    (env, charging_listener): (Env, Receiver<RevertibleWeight>),
    expected_ret_val: u32,
) where
    Env: ChainExtensionEnvironment<(), Runtime> + Executor<Runtime>,
{
    let result = <Extension as CallableChainExtension<(), Runtime, _>>::call(&mut Extension, env);

    assert!(matches!(result, Ok(RetVal::Converging(ret_val)) if ret_val == expected_ret_val));
    assert_eq!(
        charged(charging_listener),
        weight_of_store_key::<Runtime>(VK.len() as u32).into()
    );
}

pub const ADJUSTED_WEIGHT: u64 = 1729;

// Unfortunately, due to the `unconstrained generic constant` error, `ACTUAL_WEIGHT` will have to be
// passed twice for failure tests (once to `VerifyErrorer` and second time as a separate value).
pub fn simulate_verify<Env, const ACTUAL_WEIGHT: Option<u64>, const EXPECTED_RET_VAL: u32>(
    (env, charging_listener): (Env, Receiver<RevertibleWeight>),
) where
    Env: ChainExtensionEnvironment<(), Runtime> + Executor<Runtime>,
{
    let result = <Extension as CallableChainExtension<(), Runtime, _>>::call(&mut Extension, env);

    assert!(matches!(result, Ok(RetVal::Converging(ret_val)) if ret_val == EXPECTED_RET_VAL));

    let expected_charge = ACTUAL_WEIGHT.unwrap_or_else(|| {
        <<Runtime as BabyLiminalConfig>::WeightInfo as WeightInfo>::verify().ref_time()
    });
    assert_eq!(
        charged(charging_listener),
        Weight::from_ref_time(expected_charge).into()
    );
}
