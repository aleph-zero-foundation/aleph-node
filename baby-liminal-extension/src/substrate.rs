use obce::substrate::{
    frame_support::weights::Weight,
    frame_system::Config as SysConfig,
    pallet_contracts::{chain_extension::RetVal, Config as ContractConfig},
    sp_core::crypto::UncheckedFrom,
    sp_runtime::{traits::StaticLookup, AccountId32},
    sp_std::{mem::size_of, vec::Vec},
    ChainExtensionEnvironment, ExtensionContext,
};
use pallet_baby_liminal::{
    Config as BabyLiminalConfig, Error, VerificationKeyIdentifier, WeightInfo,
};

use crate::{
    executor::Executor, BabyLiminalError, BabyLiminalExtension, BABY_LIMINAL_STORE_KEY_TOO_LONG_KEY,
};

pub type ByteCount = u32;

/// Provides a weight of `store_key` dispatchable.
pub fn weight_of_store_key<T: BabyLiminalConfig>(key_length: ByteCount) -> Weight {
    <<T as BabyLiminalConfig>::WeightInfo as WeightInfo>::store_key(key_length)
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
    <T as SysConfig>::RuntimeOrigin: From<Option<AccountId32>>,
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
        origin: AccountId32,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
    ) -> Result<(), BabyLiminalError> {
        let pre_charged = self.pre_charged().unwrap();

        // Now we know the exact key length.
        self.env.adjust_weight(
            pre_charged,
            weight_of_store_key::<T>(key.len() as ByteCount),
        );

        match Env::store_key(origin, identifier, key) {
            Ok(_) => Ok(()),
            // In case `DispatchResultWithPostInfo` was returned (or some simpler equivalent for
            // `bare_store_key`), we could have adjusted weight. However, for the storing key action
            // it doesn't make much sense.
            Err(Error::VerificationKeyTooLong) => Err(BabyLiminalError::VerificationKeyTooLong),
            Err(Error::IdentifierAlreadyInUse) => Err(BabyLiminalError::IdentifierAlreadyInUse),
            _ => Err(BabyLiminalError::StoreKeyErrorUnknown),
        }
    }

    #[obce(weight(expr = "Weight::default()", pre_charge), ret_val)]
    fn verify(
        &mut self,
        identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        input: Vec<u8>,
    ) -> Result<(), BabyLiminalError> {
        let pre_charged = self.pre_charged().unwrap();

        let result = Env::verify(identifier, proof, input);

        // In case the dispatchable failed and pallet provides us with post-dispatch weight, we can
        // adjust charging. Otherwise (positive case or no post-dispatch info) we cannot refund
        // anything.
        if let Err((_, Some(actual_weight))) = &result {
            self.env.adjust_weight(pre_charged, *actual_weight);
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
            Err((Error::VerificationFailed, _)) => Err(BabyLiminalError::VerificationFailed),
            Err((Error::IncorrectProof, _)) => Err(BabyLiminalError::IncorrectProof),
            Err((_, _)) => Err(BabyLiminalError::VerifyErrorUnknown),
        }
    }
}
