#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use sp_core::crypto::KeyTypeId;
use sp_runtime::ConsensusEngineId;
use sp_std::vec::Vec;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"alp0");

// Same as GRANDPA_ENGINE_ID because as of right now substrate sends only
// grandpa justifications over the network.
// TODO: change this once https://github.com/paritytech/substrate/issues/8172 will be resolved.
pub const ALEPH_ENGINE_ID: ConsensusEngineId = *b"FRNK";

mod app {
    use sp_application_crypto::{app_crypto, sr25519};
    app_crypto!(sr25519, crate::KEY_TYPE);
}

sp_application_crypto::with_pair! {
    pub type AuthorityPair = app::Pair;
}
pub type AuthoritySignature = app::Signature;
pub type AuthorityId = app::Public;

sp_api::decl_runtime_apis! {
    pub trait AlephApi {
        fn authorities() -> Vec<AuthorityId>;
    }
}

#[derive(Decode, Encode, PartialEq, Eq, Clone)]
pub enum AuthoritiesLog<Id, Number>
where
    Id: Encode + Decode,
    Number: Encode + Decode,
{
    WillChange {
        session_id: u64,
        when: Number,
        next_authorities: Vec<Id>,
    },
}
