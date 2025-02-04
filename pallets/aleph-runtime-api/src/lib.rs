//! Runtime API definition for pallet aleph.
#![cfg_attr(not(feature = "std"), no_std)]

use primitives::{
    crypto::SignatureSet, AccountId, ApiError, AuthorityId, AuthoritySignature, Balance, Perbill,
    Score, SessionAuthorityData, SessionCommittee, SessionIndex, SessionValidatorError, Version,
};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait AlephSessionApi {
        fn next_session_authorities() -> Result<Vec<AuthorityId>, ApiError>;
        fn authorities() -> Vec<AuthorityId>;
        fn next_session_authority_data() -> Result<SessionAuthorityData, ApiError>;
        fn authority_data() -> SessionAuthorityData;
        fn session_period() -> u32;
        fn millisecs_per_block() -> u64;
        fn finality_version() -> Version;
        fn next_session_finality_version() -> Version;
        fn score_submission_period() -> u32;
        /// Predict finality committee and block producers for the given session. `session` must be
        /// within the current era (current, in the staking context).
        ///
        /// If the active era `E` starts in the session `a`, and ends in session `b` then from
        /// session `a` to session `b-1` this function can answer question who will be in the
        /// committee in the era `E`. In the last session of the era `E` (`b`) this can be used to
        /// determine all of the sessions in the era `E+1`.
        fn predict_session_committee(
            session: SessionIndex
        ) -> Result<SessionCommittee<AccountId>, SessionValidatorError>;
        fn next_session_aura_authorities() -> Vec<(AccountId, AuraId)>;
        /// Returns owner (`AccountId`) corresponding to an AuthorityId (in some contexts referenced
        /// also as `aleph_key` - consensus engine's part of session keys) in the current session
        /// of AlephBFT (finalisation committee).
        fn key_owner(key: AuthorityId) -> Option<AccountId>;
        /// Returns inflation from now to now + 1 year. Capped at 100%
        fn yearly_inflation() -> Perbill;
        /// Returns payout. First tuple item is a validators payout, 2nd is the rest.
        fn current_era_payout() -> (Balance, Balance);
        /// Submits score for a nonce in a session of performance of finality committee members.
        fn submit_abft_score(score: Score, signature: SignatureSet<AuthoritySignature>) -> Option<()>;
    }
}
