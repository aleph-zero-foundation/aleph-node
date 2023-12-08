use environment::Environment as EnvironmentT;
use executor::BackendExecutor as BackendExecutorT;
use frame_support::{pallet_prelude::DispatchError, sp_runtime::AccountId32};
use frame_system::Config as SystemConfig;
use log::error;
use pallet_baby_liminal::{AlephWeight, Error::*, WeightInfo};
use pallet_contracts::chain_extension::{
    ChainExtension, Environment as SubstrateEnvironment, Ext, InitState,
    Result as ChainExtensionResult, RetVal,
};
use sp_std::marker::PhantomData;

use crate::{backend::executor::MinimalRuntime, extension_ids::VERIFY_EXT_ID, status_codes::*};

mod environment;
mod executor;
#[cfg(test)]
mod tests;

type ByteCount = u32;

/// The actual implementation of the chain extension. This is the code on the runtime side that will
/// be executed when the chain extension is called.
pub struct BabyLiminalChainExtension<Runtime> {
    _config: PhantomData<Runtime>,
}

impl<Runtime> Default for BabyLiminalChainExtension<Runtime> {
    fn default() -> Self {
        Self {
            _config: PhantomData,
        }
    }
}

impl<Runtime: MinimalRuntime> ChainExtension<Runtime> for BabyLiminalChainExtension<Runtime>
where
    <Runtime as SystemConfig>::RuntimeOrigin: From<Option<AccountId32>>,
{
    fn call<E: Ext<T = Runtime>>(
        &mut self,
        env: SubstrateEnvironment<E, InitState>,
    ) -> ChainExtensionResult<RetVal> {
        let func_id = env.func_id() as u32;

        match func_id {
            VERIFY_EXT_ID => Self::verify::<Runtime, _, AlephWeight<Runtime>>(env.buf_in_buf_out()),
            _ => {
                error!("Called an unregistered `func_id`: {func_id}");
                Err(DispatchError::Other("Called an unregistered `func_id`"))
            }
        }
    }
}

impl<Runtime: MinimalRuntime> BabyLiminalChainExtension<Runtime>
where
    <Runtime as SystemConfig>::RuntimeOrigin: From<Option<AccountId32>>,
{
    /// Handle `verify` chain extension call.
    pub fn verify<
        BackendExecutor: BackendExecutorT,
        Environment: EnvironmentT,
        Weighting: WeightInfo,
    >(
        mut env: Environment,
    ) -> ChainExtensionResult<RetVal> {
        // ------- Pre-charge optimistic weight. ---------------------------------------------------
        let pre_charge = env.charge_weight(Weighting::verify())?;

        // ------- Read the arguments. -------------------------------------------------------------
        //
        // TODO: charge additional weight for the args size (spam protection);
        // this requires some benchmarking (maybe possible here, instead of polluting pallet's code)
        // JIRA: https://cardinal-cryptography.atlassian.net/browse/A0-3578
        let args = env.read_as_unbounded(env.in_len())?;

        // ------- Forward the call. ---------------------------------------------------------------
        let result = BackendExecutor::verify(args);

        // ------- Adjust weight if needed. --------------------------------------------------------
        match &result {
            // In the failure case, if pallet provides us with a post-dispatch weight, we can make
            // an adjustment.
            Err((_, Some(actual_weight))) => env.adjust_weight(pre_charge, *actual_weight),
            // Otherwise (positive case, or pallet doesn't provide us with any adjustment hint), we
            // don't need to do anything.
            Ok(_) | Err((_, None)) => {}
        };

        // ------- Translate the status. -----------------------------------------------------------
        let status = match result {
            Ok(()) => VERIFY_SUCCESS,
            Err((DeserializingProofFailed, _)) => VERIFY_DESERIALIZING_PROOF_FAIL,
            Err((DeserializingPublicInputFailed, _)) => VERIFY_DESERIALIZING_INPUT_FAIL,
            Err((UnknownVerificationKeyIdentifier, _)) => VERIFY_UNKNOWN_IDENTIFIER,
            Err((DeserializingVerificationKeyFailed, _)) => VERIFY_DESERIALIZING_KEY_FAIL,
            Err((VerificationFailed, _)) => VERIFY_VERIFICATION_FAIL,
            Err((IncorrectProof, _)) => VERIFY_INCORRECT_PROOF,
            Err(_) => VERIFY_ERROR_UNKNOWN,
        };
        Ok(RetVal::Converging(status))
    }
}
