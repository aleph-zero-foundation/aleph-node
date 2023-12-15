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
        pub fn call_verify(&self) {
            self.env()
                .extension()
                .verify(Default::default(), vec![0; 41], vec![0; 82])
                .unwrap();
        }
    }

    #[cfg(test)]
    mod tests {
        use ink::env::test::register_chain_extension;

        use super::*;

        struct MockedVerifyExtension;
        impl ink::env::test::ChainExtension for MockedVerifyExtension {
            fn ext_id(&self) -> u16 {
                baby_liminal_extension::extension_ids::EXTENSION_ID
            }

            fn call(&mut self, func_id: u16, _: &[u8], _: &mut Vec<u8>) -> u32 {
                assert_eq!(
                    func_id,
                    baby_liminal_extension::extension_ids::VERIFY_FUNC_ID
                );
                baby_liminal_extension::status_codes::VERIFY_SUCCESS
            }
        }

        #[ink::test]
        fn verify_works() {
            register_chain_extension(MockedVerifyExtension);
            TestContract::new().call_verify();
        }
    }
}
