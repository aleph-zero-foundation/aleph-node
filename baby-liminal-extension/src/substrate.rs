use obce::substrate::{
    frame_support::weights::Weight,
    frame_system::Config as SysConfig,
    pallet_contracts::{chain_extension::RetVal, Config as ContractConfig},
    sp_core::crypto::UncheckedFrom,
    sp_runtime::traits::StaticLookup,
    sp_std::{mem::size_of, vec::Vec},
    ChainExtensionEnvironment, ExtensionContext,
};
use pallet_baby_liminal::{
    Config as BabyLiminalConfig, Error, VerificationKeyIdentifier, WeightInfo,
};
use primitives::host_functions::poseidon;

use crate::{
    executor::Executor, BabyLiminalError, BabyLiminalExtension, ProvingSystem, SingleHashInput,
    BABY_LIMINAL_STORE_KEY_TOO_LONG_KEY,
};

pub type ByteCount = u32;

/// Provides a weight of `store_key` dispatchable.
pub fn weight_of_store_key<T: BabyLiminalConfig>(key_length: ByteCount) -> Weight {
    <<T as BabyLiminalConfig>::WeightInfo as WeightInfo>::store_key(key_length)
}

/// Provides a weight of `verify` dispatchable depending on the `ProvingSystem`.
///
/// In case no system is passed, we return maximal amongst all the systems.
pub fn weight_of_verify<T: BabyLiminalConfig>(system: Option<ProvingSystem>) -> Weight {
    match system {
        Some(ProvingSystem::Groth16) => {
            <<T as BabyLiminalConfig>::WeightInfo as WeightInfo>::verify_groth16()
        }
        Some(ProvingSystem::Gm17) => {
            <<T as BabyLiminalConfig>::WeightInfo as WeightInfo>::verify_gm17()
        }
        Some(ProvingSystem::Marlin) => {
            <<T as BabyLiminalConfig>::WeightInfo as WeightInfo>::verify_marlin()
        }
        None => weight_of_verify::<T>(Some(ProvingSystem::Groth16))
            .max(weight_of_verify::<T>(Some(ProvingSystem::Gm17)))
            .max(weight_of_verify::<T>(Some(ProvingSystem::Marlin))),
    }
}

#[derive(Default)]
pub struct Extension;

#[obce::implementation]
impl<'a, E, T, Env> BabyLiminalExtension for ExtensionContext<'a, E, T, Env, Extension>
where
    T: SysConfig + ContractConfig + BabyLiminalConfig,
    <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
    <T as SysConfig>::AccountId: UncheckedFrom<<T as SysConfig>::Hash> + AsRef<[u8]>,
    Env: ChainExtensionEnvironment<E, T> + Executor<T>,
{
    #[obce(
        weight(
            expr = r#"{
                let approx_key_length = env
                    .in_len()
                    .saturating_sub(size_of::<VerificationKeyIdentifier>() as ByteCount);

                if approx_key_length > 10_000 {
                    return Ok(RetVal::Converging(BABY_LIMINAL_STORE_KEY_TOO_LONG_KEY));
                }

                weight_of_store_key::<T>(approx_key_length)
            }"#,
            pre_charge
        ),
        ret_val
    )]
    fn store_key(
        &mut self,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
    ) -> Result<(), BabyLiminalError> {
        let pre_charged = self.pre_charged().unwrap();

        // Now we know the exact key length.
        self.env.adjust_weight(
            pre_charged,
            weight_of_store_key::<T>(key.len() as ByteCount),
        );

        match Env::store_key(identifier, key) {
            Ok(_) => Ok(()),
            // In case `DispatchResultWithPostInfo` was returned (or some simpler equivalent for
            // `bare_store_key`), we could have adjusted weight. However, for the storing key action
            // it doesn't make much sense.
            Err(Error::VerificationKeyTooLong) => Err(BabyLiminalError::VerificationKeyTooLong),
            Err(Error::IdentifierAlreadyInUse) => Err(BabyLiminalError::IdentifierAlreadyInUse),
            _ => Err(BabyLiminalError::StoreKeyErrorUnknown),
        }
    }

    #[obce(weight(expr = "weight_of_verify::<T>(None)", pre_charge), ret_val)]
    fn verify(
        &mut self,
        identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        input: Vec<u8>,
        system: ProvingSystem,
    ) -> Result<(), BabyLiminalError> {
        let pre_charged = self.pre_charged().unwrap();

        let result = Env::verify(identifier, proof, input, system);

        match &result {
            // Positive case: we can adjust weight based on the system used.
            Ok(_) => self
                .env
                .adjust_weight(pre_charged, weight_of_verify::<T>(Some(system))),
            // Negative case: Now we inspect how we should adjust weighting. In case pallet provides
            // us with post-dispatch weight, we will use it. Otherwise, we weight the call in the
            // same way as in the positive case.
            Err((_, Some(actual_weight))) => self.env.adjust_weight(pre_charged, *actual_weight),
            Err((_, None)) => self
                .env
                .adjust_weight(pre_charged, weight_of_verify::<T>(Some(system))),
        };

        match result {
            Ok(_) => Ok(()),
            Err((Error::DeserializingProofFailed, _)) => {
                Err(BabyLiminalError::DeserializingProofFailed)
            }
            Err((Error::DeserializingPublicInputFailed, _)) => {
                Err(BabyLiminalError::DeserializingPublicInputFailed)
            }
            Err((Error::UnknownVerificationKeyIdentifier, _)) => {
                Err(BabyLiminalError::UnknownVerificationKeyIdentifier)
            }
            Err((Error::DeserializingVerificationKeyFailed, _)) => {
                Err(BabyLiminalError::DeserializingVerificationKeyFailed)
            }
            Err((Error::VerificationFailed(_), _)) => Err(BabyLiminalError::VerificationFailed),
            Err((Error::IncorrectProof, _)) => Err(BabyLiminalError::IncorrectProof),
            Err((_, _)) => Err(BabyLiminalError::VerifyErrorUnknown),
        }
    }

    #[obce(weight(
        expr = "<<T as BabyLiminalConfig>::WeightInfo as WeightInfo>::poseidon_one_to_one_host()",
        pre_charge
    ))]
    fn poseidon_one_to_one(&self, input: [SingleHashInput; 1]) -> SingleHashInput {
        poseidon::one_to_one_hash(input[0])
    }

    #[obce(weight(
        expr = "<<T as BabyLiminalConfig>::WeightInfo as WeightInfo>::poseidon_two_to_one_host()",
        pre_charge
    ))]
    fn poseidon_two_to_one(&self, input: [SingleHashInput; 2]) -> SingleHashInput {
        poseidon::two_to_one_hash(input[0], input[1])
    }

    #[obce(weight(
        expr = "<<T as BabyLiminalConfig>::WeightInfo as WeightInfo>::poseidon_four_to_one_host()",
        pre_charge
    ))]
    fn poseidon_four_to_one(&self, input: [SingleHashInput; 4]) -> SingleHashInput {
        poseidon::four_to_one_hash(input[0], input[1], input[2], input[3])
    }
}
