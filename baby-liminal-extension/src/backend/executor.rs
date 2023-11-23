use frame_support::{pallet_prelude::Weight, sp_runtime::AccountId32};
use frame_system::Config as SystemConfig;
use pallet_baby_liminal::{Config as BabyLiminalConfig, Error as PalletError, Pallet};
use pallet_contracts::Config as ContractsConfig;

use crate::args::{StoreKeyArgs, VerifyArgs};

/// Minimal runtime configuration required by the chain extension executor.
pub trait MinimalRuntime: SystemConfig + BabyLiminalConfig + ContractsConfig {}
impl<R: SystemConfig + BabyLiminalConfig + ContractsConfig> MinimalRuntime for R {}

/// Generalized pallet executor, that can be mocked for testing purposes.
pub trait BackendExecutor {
    /// The pallet's error enum is generic. For most purposes however, it doesn't matter what type
    /// will be passed there. Normally, `Runtime` will be the generic argument, but in the testing
    /// context it will be enough to instantiate it with `()`.
    type ErrorGenericType;

    fn store_key(args: StoreKeyArgs) -> Result<(), PalletError<Self::ErrorGenericType>>;

    fn verify(
        args: VerifyArgs,
    ) -> Result<(), (PalletError<Self::ErrorGenericType>, Option<Weight>)>;
}

/// Default implementation for the chain extension mechanics.
impl<Runtime: MinimalRuntime> BackendExecutor for Runtime
where
    <Runtime as SystemConfig>::RuntimeOrigin: From<Option<AccountId32>>,
{
    type ErrorGenericType = Runtime;

    fn store_key(args: StoreKeyArgs) -> Result<(), PalletError<Self::ErrorGenericType>> {
        Pallet::<Runtime>::bare_store_key(Some(args.depositor).into(), args.identifier, args.key)
    }

    fn verify(
        args: VerifyArgs,
    ) -> Result<(), (PalletError<Self::ErrorGenericType>, Option<Weight>)> {
        Pallet::<Runtime>::bare_verify(
            args.verification_key_identifier,
            args.proof,
            args.public_input,
        )
    }
}
