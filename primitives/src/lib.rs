#![allow(clippy::too_many_arguments, clippy::unnecessary_mut_passed)]
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
    use sp_application_crypto::{app_crypto, ed25519};
    app_crypto!(ed25519, crate::KEY_TYPE);
}
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;

sp_application_crypto::with_pair! {
    pub type AuthorityPair = app::Pair;
}
pub type AuthoritySignature = app::Signature;
pub type AuthorityId = app::Public;

#[derive(codec::Encode, codec::Decode, PartialEq, Eq, sp_std::fmt::Debug)]
pub enum ApiError {
    DecodeKey,
}

sp_api::decl_runtime_apis! {
    pub trait AlephSessionApi<Id, BlockNumber>
    where
        Id: Encode + Decode,
        BlockNumber: Encode + Decode,
    {
        fn current_session() -> Session<Id, BlockNumber>;
        fn next_session() -> Result<Session<Id, BlockNumber>, ApiError>;
        fn authorities() -> Vec<AuthorityId>;
        fn session_period() -> u32;
    }
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, sp_std::fmt::Debug)]
pub struct Session<Id, BlockNumber>
where
    Id: Encode + Decode,
    BlockNumber: Encode + Decode,
{
    pub session_id: u32,
    pub stop_h: BlockNumber,
    pub authorities: Vec<Id>,
}
