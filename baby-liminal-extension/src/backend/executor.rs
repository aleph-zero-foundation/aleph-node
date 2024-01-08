use aleph_runtime_interfaces::snark_verifier::{verify, VerifierError};
use pallet_contracts::Config as ContractsConfig;
use pallet_vk_storage::{Config as VkStorageConfig, VerificationKeys};

use crate::args::VerifyArgs;

/// Represents an 'engine' that handles chain extension calls.
pub trait BackendExecutor {
    fn verify(args: VerifyArgs) -> Result<(), VerifierError>;
}

/// Minimal runtime configuration required by the standard chain extension executor.
pub trait MinimalRuntime: VkStorageConfig + ContractsConfig {}
impl<R: VkStorageConfig + ContractsConfig> MinimalRuntime for R {}

/// Default implementation for the chain extension mechanics.
impl<Runtime: MinimalRuntime> BackendExecutor for Runtime {
    fn verify(args: VerifyArgs) -> Result<(), VerifierError> {
        let verifying_key = VerificationKeys::<Runtime>::get(args.verification_key_hash)
            .ok_or(VerifierError::UnknownVerificationKeyIdentifier)?
            .to_vec();

        verify(&args.proof, &args.public_input, &verifying_key)
    }
}
