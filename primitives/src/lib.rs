#![cfg_attr(not(feature = "std"), no_std)]
use sp_core::crypto::KeyTypeId;
use sp_std::vec::Vec;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"alp0");

mod app {
    use sp_application_crypto::{app_crypto, ed25519};
    app_crypto!(ed25519, crate::KEY_TYPE);
}

pub type AuthorityId = app::Public;

sp_api::decl_runtime_apis! {
    pub trait AlephApi {
        fn authorities() -> Vec<AuthorityId>;
    }
}
