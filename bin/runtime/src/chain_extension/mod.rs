use codec::Decode;
use frame_support::{log::error, pallet_prelude::Weight};
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use pallet_snarcos::{
    Config, Error, Pallet as Snarcos, ProvingSystem, VerificationKeyIdentifier, WeightInfo,
};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::DispatchError;
use sp_std::{mem::size_of, vec::Vec};
use Error::*;

use crate::{MaximumVerificationKeyLength, Runtime};

pub const SNARCOS_STORE_KEY_FUNC_ID: u32 = 41;
pub const SNARCOS_VERIFY_FUNC_ID: u32 = 42;

// Return codes for `SNARCOS_STORE_KEY_FUNC_ID`.
pub const SNARCOS_STORE_KEY_OK: u32 = 10_000;
pub const SNARCOS_STORE_KEY_TOO_LONG_KEY: u32 = 10_001;
pub const SNARCOS_STORE_KEY_IN_USE: u32 = 10_002;
pub const SNARCOS_STORE_KEY_ERROR_UNKNOWN: u32 = 10_003;

// Return codes for `SNARCOS_VERIFY_FUNC_ID`.
pub const SNARCOS_VERIFY_OK: u32 = 11_000;
pub const SNARCOS_VERIFY_DESERIALIZING_PROOF_FAIL: u32 = 11_001;
pub const SNARCOS_VERIFY_DESERIALIZING_INPUT_FAIL: u32 = 11_002;
pub const SNARCOS_VERIFY_UNKNOWN_IDENTIFIER: u32 = 11_003;
pub const SNARCOS_VERIFY_DESERIALIZING_KEY_FAIL: u32 = 11_004;
pub const SNARCOS_VERIFY_VERIFICATION_FAIL: u32 = 11_005;
pub const SNARCOS_VERIFY_INCORRECT_PROOF: u32 = 11_006;
pub const SNARCOS_VERIFY_ERROR_UNKNOWN: u32 = 11_007;

pub struct SnarcosChainExtension;

impl ChainExtension<Runtime> for SnarcosChainExtension {
    fn call<E: Ext>(func_id: u32, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        match func_id {
            SNARCOS_STORE_KEY_FUNC_ID => Self::snarcos_store_key(env),
            SNARCOS_VERIFY_FUNC_ID => Self::snarcos_verify(env),
            _ => {
                error!("Called an unregistered `func_id`: {}", func_id);
                Err(DispatchError::Other("Unimplemented func_id"))
            }
        }
    }
}

pub type ByteCount = u32;

/// Struct to be decoded from a byte slice passed from the contract.
///
/// Notice, that contract can pass these arguments one by one, not necessarily as such struct. Only
/// the order of values is important.
///
/// It cannot be `MaxEncodedLen` due to `Vec<_>` and thus `Environment::read_as` cannot be used.
#[derive(Decode)]
struct StoreKeyArgs {
    pub identifier: VerificationKeyIdentifier,
    pub key: Vec<u8>,
}

/// Struct to be decoded from a byte slice passed from the contract.
///
/// Notice, that contract can pass these arguments one by one, not necessarily as such struct. Only
/// the order of values is important.
///
/// It cannot be `MaxEncodedLen` due to `Vec<_>` and thus `Environment::read_as` cannot be used.
#[derive(Decode)]
struct VerifyArgs {
    pub identifier: VerificationKeyIdentifier,
    pub proof: Vec<u8>,
    pub input: Vec<u8>,
    pub system: ProvingSystem,
}

impl SnarcosChainExtension {
    fn snarcos_store_key<E: Ext>(env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        // We need to read input as plain bytes (encoded args).
        let mut env = env.buf_in_buf_out();

        // Check if it makes sense to read and decode data.
        let key_length = env
            .in_len()
            .saturating_sub(size_of::<VerificationKeyIdentifier>() as ByteCount);
        if key_length > MaximumVerificationKeyLength::get() {
            return Ok(RetVal::Converging(SNARCOS_STORE_KEY_TOO_LONG_KEY));
        }

        // We charge now - even if decoding fails and we shouldn't touch storage, we have to incur
        // fee for reading memory.
        env.charge_weight(<<Runtime as Config>::WeightInfo as WeightInfo>::store_key(
            key_length,
        ))?;

        // Parsing will have to be done here. This is due to the fact that methods
        // `Environment<_,_,_,S: BufIn>::read*` don't move starting pointer and thus we can make
        // only a single read. Since `key` is just an ('unbounded') `Vec<u8>` we can only use
        // `env.read()` method and decode arguments by hand here.
        //
        // It is safe to read `env.in_len()` bytes since we already checked that it's not too much.
        let bytes = env.read(env.in_len())?;

        let args = StoreKeyArgs::decode(&mut &*bytes)
            .map_err(|_| DispatchError::Other("Failed to decode arguments"))?;

        let return_status = match Snarcos::<Runtime>::bare_store_key(args.identifier, args.key) {
            Ok(_) => SNARCOS_STORE_KEY_OK,
            // In case `DispatchResultWithPostInfo` was returned (or some simpler equivalent for
            // `bare_store_key`), we could adjust weight. However, for the storing key action it
            // doesn't make sense.
            Err(VerificationKeyTooLong) => SNARCOS_STORE_KEY_TOO_LONG_KEY,
            Err(IdentifierAlreadyInUse) => SNARCOS_STORE_KEY_IN_USE,
            _ => SNARCOS_STORE_KEY_ERROR_UNKNOWN,
        };
        Ok(RetVal::Converging(return_status))
    }

    fn weight_of_verify(system: Option<ProvingSystem>) -> Weight {
        match system {
            Some(ProvingSystem::Groth16) => {
                <<Runtime as Config>::WeightInfo as WeightInfo>::verify_groth16()
            }
            Some(ProvingSystem::Gm17) => {
                <<Runtime as Config>::WeightInfo as WeightInfo>::verify_gm17()
            }
            Some(ProvingSystem::Marlin) => {
                <<Runtime as Config>::WeightInfo as WeightInfo>::verify_marlin()
            }
            None => Self::weight_of_verify(Some(ProvingSystem::Groth16))
                .max(Self::weight_of_verify(Some(ProvingSystem::Gm17)))
                .max(Self::weight_of_verify(Some(ProvingSystem::Marlin))),
        }
    }

    fn snarcos_verify<E: Ext>(env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        // We need to read input as plain bytes (encoded args).
        let mut env = env.buf_in_buf_out();

        // We charge optimistically, i.e. assuming that decoding succeeds and the verification
        // key is present. However, since we don't know the system yet, we have to charge maximal
        // possible fee. We will adjust it as soon as possible.
        //
        // Currently, we cannot do more in terms of charging due to insufficiently flexible
        // weighting (`pallet_snarcos::WeightInfo` API). Once we have functions like
        // `pallet_snarcos::WeightInfo::verify_decoding_failure`, we can both charge less here
        // (with further `env.adjust_weight`) and in the pallet itself (returning
        // `DispatchErrorWithPostInfo` reducing actual fee and the block weight).
        let pre_charge = env.charge_weight(Self::weight_of_verify(None))?;

        // Parsing is done here for similar reasons as in `Self::snarcos_store_key`.
        let bytes = env.read(env.in_len())?;

        let args: VerifyArgs = VerifyArgs::decode(&mut &*bytes)
            .map_err(|_| DispatchError::Other("Failed to decode arguments"))?;

        env.adjust_weight(pre_charge, Self::weight_of_verify(Some(args.system)));

        let result =
            Snarcos::<Runtime>::bare_verify(args.identifier, args.proof, args.input, args.system);

        let return_status = match result {
            Ok(_) => SNARCOS_VERIFY_OK,
            // In case `DispatchResultWithPostInfo` was returned (or some simpler equivalent for
            // `bare_store_key`), we could adjust weight. However, we don't support it yet.
            Err(DeserializingProofFailed) => SNARCOS_VERIFY_DESERIALIZING_PROOF_FAIL,
            Err(DeserializingPublicInputFailed) => SNARCOS_VERIFY_DESERIALIZING_INPUT_FAIL,
            Err(UnknownVerificationKeyIdentifier) => SNARCOS_VERIFY_UNKNOWN_IDENTIFIER,
            Err(DeserializingVerificationKeyFailed) => SNARCOS_VERIFY_DESERIALIZING_KEY_FAIL,
            Err(VerificationFailed) => SNARCOS_VERIFY_VERIFICATION_FAIL,
            Err(IncorrectProof) => SNARCOS_VERIFY_INCORRECT_PROOF,
            _ => SNARCOS_VERIFY_ERROR_UNKNOWN,
        };
        Ok(RetVal::Converging(return_status))
    }
}
