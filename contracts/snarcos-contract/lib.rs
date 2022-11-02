#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract(env = snarcos_extension::DefaultEnvironment)]
#[allow(clippy::let_unit_value)] // clippy complains about the return type of `trigger` message
mod snarcos {
    use snarcos_extension::{StoreKeyError, VerificationKeyIdentifier};
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
        ) -> Result<(), StoreKeyError> {
            self.env().extension().store_key(identifier, key)?;
            Ok(())
        }
    }
}
