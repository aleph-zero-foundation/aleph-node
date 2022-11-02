#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract(env = snarcos_extension::DefaultEnvironment)]
#[allow(clippy::let_unit_value)] // clippy complains about the return type of the messages
mod snarcos {
    use snarcos_extension::{ProvingSystem, SnarcosError, VerificationKeyIdentifier};
    use sp_std::vec::Vec;

    #[ink(storage)]
    #[derive(Default)]
    pub struct SnarcosExtension;

    impl SnarcosExtension {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message)]
        pub fn store_key(
            &mut self,
            identifier: VerificationKeyIdentifier,
            key: Vec<u8>,
        ) -> Result<(), SnarcosError> {
            self.env().extension().store_key(identifier, key)?;
            Ok(())
        }

        #[ink(message)]
        pub fn verify(
            &mut self,
            identifier: VerificationKeyIdentifier,
            proof: Vec<u8>,
            input: Vec<u8>,
            system: ProvingSystem,
        ) -> Result<(), SnarcosError> {
            self.env()
                .extension()
                .verify(identifier, proof, input, system)?;
            Ok(())
        }
    }
}
