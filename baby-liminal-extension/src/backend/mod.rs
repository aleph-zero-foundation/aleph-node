use environment::Environment as EnvironmentT;
use executor::BackendExecutor as BackendExecutorT;
use frame_support::{pallet_prelude::DispatchError, sp_runtime::AccountId32};
use frame_system::Config as SystemConfig;
use log::error;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment as SubstrateEnvironment, Ext, InitState,
    Result as ChainExtensionResult, RetVal,
};
use sp_std::marker::PhantomData;

use crate::{
    backend::executor::MinimalRuntime,
    extension_ids::{STORE_KEY_EXT_ID, VERIFY_EXT_ID},
    status_codes::{STORE_KEY_SUCCESS, VERIFY_SUCCESS},
};

mod environment;
mod executor;

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
            STORE_KEY_EXT_ID => Self::store_key::<Runtime, _>(env.buf_in_buf_out()),
            VERIFY_EXT_ID => Self::verify::<Runtime, _>(env.buf_in_buf_out()),
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
    /// Handle `store_key` chain extension call.
    pub fn store_key<BackendExecutor: BackendExecutorT, Environment: EnvironmentT>(
        mut env: Environment,
    ) -> ChainExtensionResult<RetVal> {
        // todo: charge weight, validate args, handle errors
        let args = env.read_as_unbounded(env.in_len())?;
        BackendExecutor::store_key(args)
            .map_err(|_| ())
            .expect("`store_key` failed; this should be handled more gently");
        Ok(RetVal::Converging(STORE_KEY_SUCCESS))
    }

    /// Handle `verify` chain extension call.
    pub fn verify<BackendExecutor: BackendExecutorT, Environment: EnvironmentT>(
        mut env: Environment,
    ) -> ChainExtensionResult<RetVal> {
        // todo: charge weight, validate args, handle errors
        let args = env.read_as_unbounded(env.in_len())?;
        BackendExecutor::verify(args)
            .map_err(|_| ())
            .expect("`verify` failed; this should be handled more gently");
        Ok(RetVal::Converging(VERIFY_SUCCESS))
    }
}
