use frame_support::dispatch::Weight;
use pallet_baby_liminal::{Error, Pallet as BabyLiminal, ProvingSystem, VerificationKeyIdentifier};
use sp_std::vec::Vec;

use crate::Runtime;

/// Abstraction around `Runtime`. Makes testing easier.
///
/// Gathers all the methods that are used by `BabyLiminalChainExtension`.
///
/// Each method is already documented in `pallet_baby_liminal`.
pub(super) trait Executor: Sized {
    /// The error returned from dispatchables is generic. For most purposes however, it doesn't
    /// matter what type will be passed there. Normally, `Runtime` will be the generic argument,
    /// but in testing it will be sufficient to instantiate it with `()`.
    type ErrorGenericType;

    fn store_key(
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
    ) -> Result<(), Error<Self::ErrorGenericType>>;

    fn verify(
        verification_key_identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        public_input: Vec<u8>,
        system: ProvingSystem,
    ) -> Result<(), (Error<Self::ErrorGenericType>, Option<Weight>)>;
}

/// Transparent delegation.
impl Executor for Runtime {
    type ErrorGenericType = Runtime;

    fn store_key(
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
    ) -> Result<(), Error<Runtime>> {
        BabyLiminal::<Runtime>::bare_store_key(identifier, key)
    }

    fn verify(
        verification_key_identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        public_input: Vec<u8>,
        system: ProvingSystem,
    ) -> Result<(), (Error<Self::ErrorGenericType>, Option<Weight>)> {
        BabyLiminal::<Runtime>::bare_verify(
            verification_key_identifier,
            proof,
            public_input,
            system,
        )
    }
}
