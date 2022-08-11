#![allow(clippy::too_many_arguments, clippy::unnecessary_mut_passed)]
#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
    generic::Header as GenericHeader,
    traits::{BlakeTwo256, Header as HeaderT},
    ConsensusEngineId,
};
pub use sp_staking::{EraIndex, SessionIndex};
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

sp_application_crypto::with_pair! {
    pub type AuthorityPair = app::Pair;
}
pub type AuthoritySignature = app::Signature;
pub type AuthorityId = app::Public;

pub type Balance = u128;
pub type Header = GenericHeader<BlockNumber, BlakeTwo256>;
pub type BlockHash = <Header as HeaderT>::Hash;
pub type BlockNumber = u32;

pub const MILLISECS_PER_BLOCK: u64 = 1000;

// Quick sessions for testing purposes
#[cfg(feature = "short_session")]
pub const DEFAULT_SESSION_PERIOD: u32 = 30;
#[cfg(feature = "short_session")]
pub const DEFAULT_SESSIONS_PER_ERA: SessionIndex = 5;

// Default values outside testing
#[cfg(not(feature = "short_session"))]
pub const DEFAULT_SESSION_PERIOD: u32 = 900;
#[cfg(not(feature = "short_session"))]
pub const DEFAULT_SESSIONS_PER_ERA: SessionIndex = 96;

pub const TOKEN_DECIMALS: u32 = 12;
pub const TOKEN: u128 = 10u128.pow(TOKEN_DECIMALS);

pub const ADDRESSES_ENCODING: u8 = 42;
pub const DEFAULT_UNIT_CREATION_DELAY: u64 = 300;

pub const DEFAULT_COMMITTEE_SIZE: u32 = 4;

#[derive(Decode, Encode, TypeInfo, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct CommitteeSeats {
    pub reserved_seats: u32,
    pub non_reserved_seats: u32,
}

impl CommitteeSeats {
    pub fn size(&self) -> u32 {
        self.reserved_seats.saturating_add(self.non_reserved_seats)
    }
}

impl Default for CommitteeSeats {
    fn default() -> Self {
        CommitteeSeats {
            reserved_seats: DEFAULT_COMMITTEE_SIZE,
            non_reserved_seats: 0,
        }
    }
}

#[derive(Eq, PartialEq, Decode, Encode, TypeInfo)]
pub struct EraValidators<AccountId> {
    pub reserved: Vec<AccountId>,
    pub non_reserved: Vec<AccountId>,
}

impl<AccountId> Default for EraValidators<AccountId> {
    fn default() -> Self {
        Self {
            reserved: Vec::new(),
            non_reserved: Vec::new(),
        }
    }
}

#[derive(Encode, Decode, PartialEq, Eq, Debug)]
pub enum ApiError {
    DecodeKey,
}

/// All the data needed to verify block finalization justifications.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub struct SessionAuthorityData {
    authorities: Vec<AuthorityId>,
    emergency_finalizer: Option<AuthorityId>,
}

impl SessionAuthorityData {
    pub fn new(authorities: Vec<AuthorityId>, emergency_finalizer: Option<AuthorityId>) -> Self {
        SessionAuthorityData {
            authorities,
            emergency_finalizer,
        }
    }

    pub fn authorities(&self) -> &Vec<AuthorityId> {
        &self.authorities
    }

    pub fn emergency_finalizer(&self) -> &Option<AuthorityId> {
        &self.emergency_finalizer
    }
}

sp_api::decl_runtime_apis! {
    pub trait AlephSessionApi
    {
        fn next_session_authorities() -> Result<Vec<AuthorityId>, ApiError>;
        fn authorities() -> Vec<AuthorityId>;
        fn next_session_authority_data() -> Result<SessionAuthorityData, ApiError>;
        fn authority_data() -> SessionAuthorityData;
        fn session_period() -> u32;
        fn millisecs_per_block() -> u64;
    }
}

pub mod staking {
    use sp_runtime::Perbill;

    use super::Balance;
    use crate::TOKEN;

    pub const MIN_VALIDATOR_BOND: u128 = 25_000 * TOKEN;
    pub const MIN_NOMINATOR_BOND: u128 = 100 * TOKEN;
    pub const MAX_NOMINATORS_REWARDED_PER_VALIDATOR: u32 = 1024;
    pub const YEARLY_INFLATION: Balance = 30_000_000 * TOKEN;
    pub const VALIDATOR_REWARD: Perbill = Perbill::from_percent(90);

    pub fn era_payout(miliseconds_per_era: u64) -> (Balance, Balance) {
        // Milliseconds per year for the Julian year (365.25 days).
        const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

        let portion = Perbill::from_rational(miliseconds_per_era, MILLISECONDS_PER_YEAR);
        let total_payout = portion * YEARLY_INFLATION;
        let validators_payout = VALIDATOR_REWARD * total_payout;
        let rest = total_payout - validators_payout;

        (validators_payout, rest)
    }

    /// Macro for making a default implementation of non-self methods from given class.
    ///
    /// As an input it expects list of tuples of form
    ///
    /// `(method_name(arg1: type1, arg2: type2, ...), class_name, return_type)`
    ///
    /// where
    ///* `method_name`is a wrapee method,
    ///* `arg1: type1, arg2: type,...`is a list of arguments and will be passed as is, can be empty
    ///* `class_name`is a class that has non-self `method-name`,ie symbol `class_name::method_name` exists,
    ///* `return_type` is type returned from `method_name`
    /// Example
    /// ```rust
    ///  wrap_methods!(
    ///         (bond(), SubstrateStakingWeights, Weight),
    ///         (bond_extra(), SubstrateStakingWeights, Weight)
    /// );
    /// ```
    #[macro_export]
    macro_rules! wrap_methods {
        ($(($wrapped_method:ident( $($arg_name:ident: $argument_type:ty), *), $wrapped_class:ty, $return_type:ty)), *) => {
            $(
                fn $wrapped_method($($arg_name: $argument_type), *) -> $return_type {
                    <$wrapped_class>::$wrapped_method($($arg_name), *)
                }
            )*
        };
    }
}
