use frame_support::log::error;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::DispatchError;

use crate::Runtime;

pub const SNARCOS_CHAIN_EXT: u32 = 41;

pub struct AlephChainExtension;
impl ChainExtension<Runtime> for AlephChainExtension {
    fn call<E: Ext>(func_id: u32, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        match func_id {
            SNARCOS_CHAIN_EXT => {
                use pallet_snarcos::{Error, Pallet as Snarcos};

                // The argument-passing-mode doesn't matter for now. All the data to runtime call
                // are mocked now.
                let mut env = env.buf_in_buf_out();
                // After benchmarking is merged and `pallet_snarcos::WeightInfo` is available,
                // use real weight here.
                env.charge_weight(41)?;

                match Snarcos::<Runtime>::bare_store_key([0u8; 4], [0u8; 8].to_vec()) {
                    // In case `DispatchResultWithPostInfo` was returned (or some simpler
                    // equivalent for `bare_store_key`), we could adjust weight. However, for the
                    // storing key action it doesn't make sense.
                    Ok(_) => {
                        // Return status code for success.
                        Ok(RetVal::Converging(0))
                    }
                    Err(Error::<Runtime>::VerificationKeyTooLong) => Ok(RetVal::Converging(1)),
                    Err(Error::<Runtime>::IdentifierAlreadyInUse) => Ok(RetVal::Converging(2)),
                    // Unknown error.
                    _ => Ok(RetVal::Converging(3)),
                }
            }
            _ => {
                error!("Called an unregistered `func_id`: {}", func_id);
                Err(DispatchError::Other("Unimplemented func_id"))
            }
        }
    }
}
