#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract(env = baby_liminal_extension::Environment)]
mod test_contract {
    use ink::prelude::vec;

    #[ink(storage)]
    pub struct TestContract {}

    impl TestContract {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message)]
        pub fn call_ce(&self) {
            self.env()
                .extension()
                .store_key(self.env().caller(), [0; 8], vec![0; 32])
                .unwrap();
        }
    }

    #[cfg(test)]
    mod tests {
        use ink::env::test::register_chain_extension;

        use super::*;

        struct MockedStoreKeyExtension;
        impl ink::env::test::ChainExtension for MockedStoreKeyExtension {
            fn func_id(&self) -> u32 {
                baby_liminal_extension::extension_ids::STORE_KEY_EXT_ID
            }

            fn call(&mut self, _: &[u8], _: &mut Vec<u8>) -> u32 {
                baby_liminal_extension::status_codes::STORE_KEY_SUCCESS
            }
        }

        #[ink::test]
        fn it_works() {
            register_chain_extension(MockedStoreKeyExtension);

            let test_contract = TestContract::new();
            test_contract.call_ce();
        }
    }
}
