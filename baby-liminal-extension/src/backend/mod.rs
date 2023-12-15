use environment::Environment as EnvironmentT;
use executor::{BackendExecutor as BackendExecutorT, ExecutorError::*};
use frame_support::{pallet_prelude::DispatchError, sp_runtime::AccountId32};
use frame_system::Config as SystemConfig;
use log::error;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment as SubstrateEnvironment, Ext, InitState,
    Result as ChainExtensionResult, RetVal,
};
use sp_std::marker::PhantomData;

use crate::{
    backend::{
        executor::MinimalRuntime,
        weights::{AlephWeight, WeightInfo},
    },
    extension_ids::{EXTENSION_ID as BABY_LIMINAL_EXTENSION_ID, VERIFY_FUNC_ID},
    status_codes::*,
};

mod environment;
mod executor;
#[cfg(test)]
mod tests;
mod weights;

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
        let (ext_id, func_id) = (env.ext_id(), env.func_id());
        match (ext_id, func_id) {
            (BABY_LIMINAL_EXTENSION_ID, VERIFY_FUNC_ID) => {
                Self::verify::<Runtime, _, AlephWeight>(env.buf_in_buf_out())
            }
            _ => {
                error!("There is no function `{func_id}` registered for an extension `{ext_id}`");
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
        let _pre_charge = env.charge_weight(Weighting::verify())?;

        // ------- Read the arguments. -------------------------------------------------------------
        env.charge_weight(Weighting::verify_read_args(env.in_len()))?;
        let args = env.read_as_unbounded(env.in_len())?;

        // ------- Forward the call. ---------------------------------------------------------------
        let result = BackendExecutor::verify(args);

        // ------- Translate the status. -----------------------------------------------------------
        let status = match result {
            Ok(()) => VERIFY_SUCCESS,
            Err(DeserializingProofFailed) => VERIFY_DESERIALIZING_PROOF_FAIL,
            Err(DeserializingPublicInputFailed) => VERIFY_DESERIALIZING_INPUT_FAIL,
            Err(UnknownVerificationKeyIdentifier) => VERIFY_UNKNOWN_IDENTIFIER,
            Err(DeserializingVerificationKeyFailed) => VERIFY_DESERIALIZING_KEY_FAIL,
            Err(VerificationFailed) => VERIFY_VERIFICATION_FAIL,
            Err(IncorrectProof) => VERIFY_INCORRECT_PROOF,
        };
        Ok(RetVal::Converging(status))
    }
}
