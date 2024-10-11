#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub use frame_support::{
    construct_runtime,
    genesis_builder_helper::{build_config, create_default_config},
    parameter_types,
    traits::{
        Currency, EstimateNextNewSession, Imbalance, KeyOwnerProofSystem, LockIdentifier, Nothing,
        OnUnbalanced, Randomness, ValidatorSet,
    },
    weights::{
        constants::{
            BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
        },
        IdentityFee, Weight,
    },
    StorageValue,
};
use frame_support::{
    sp_runtime::Perquintill,
    traits::{
        tokens::{PayFromAccount, UnityAssetBalanceConversion},
        ConstBool, ConstU32, Contains, EqualPrivilegeOnly, EstimateNextSessionRotation, InsideBoth,
        InstanceFilter, SortedMembers, WithdrawReasons,
    },
    weights::{constants::WEIGHT_REF_TIME_PER_MILLIS, WeightToFee},
    PalletId,
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess, EnsureSignedBy};
#[cfg(feature = "try-runtime")]
use frame_try_runtime::UpgradeCheckSelect;
pub use pallet_balances::Call as BalancesCall;
use pallet_committee_management::SessionAndEraManager;
use pallet_identity::legacy::IdentityInfo;
use pallet_session::QueuedKeys;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use pallet_tx_pause::RuntimeCallNameOf;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use primitives::{
    staking::MAX_NOMINATORS_REWARDED_PER_VALIDATOR, wrap_methods, Address,
    AlephNodeSessionKeys as SessionKeys, ApiError as AlephApiError, AuraId, AuthorityId as AlephId,
    BlockNumber as AlephBlockNumber, Header as AlephHeader, SessionAuthorityData, SessionCommittee,
    SessionIndex, SessionInfoProvider, SessionValidatorError,
    TotalIssuanceProvider as TotalIssuanceProviderT, Version as FinalityVersion,
    ADDRESSES_ENCODING, DEFAULT_BAN_REASON_LENGTH, DEFAULT_MAX_WINNERS, DEFAULT_SESSIONS_PER_ERA,
    DEFAULT_SESSION_PERIOD, MAX_BLOCK_SIZE, MILLISECS_PER_BLOCK, TOKEN,
};
pub use primitives::{AccountId, AccountIndex, Balance, Hash, Nonce, Signature};
use sp_api::impl_runtime_apis;
use sp_application_crypto::key_types::AURA;
use sp_consensus_aura::SlotDuration;
use sp_core::{crypto::KeyTypeId, ConstU128, OpaqueMetadata};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
    create_runtime_str, generic,
    traits::{
        AccountIdLookup, BlakeTwo256, Block as BlockT, Bounded, Convert, ConvertInto,
        IdentityLookup, One, OpaqueKeys, Verify,
    },
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, FixedU128, RuntimeDebug, SaturatedConversion,
};
pub use sp_runtime::{FixedPointNumber, Perbill, Permill, Saturating};
use sp_staking::{currency_to_vote::U128CurrencyToVote, EraIndex};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("aleph-node"),
    impl_name: create_runtime_str!("aleph-node"),
    authoring_version: 1,
    spec_version: 14_000_000,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 18,
    state_version: 0,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

pub const DAYS: u32 = 24 * 60 * 60 * 1000 / (MILLISECS_PER_BLOCK as u32);

pub const BLOCKS_PER_HOUR: u32 = 60 * 60 * 1000 / (MILLISECS_PER_BLOCK as u32);

pub const MILLI_AZERO: Balance = TOKEN / 1000;
pub const MICRO_AZERO: Balance = MILLI_AZERO / 1000;
pub const NANO_AZERO: Balance = MICRO_AZERO / 1000;
pub const PICO_AZERO: Balance = NANO_AZERO / 1000;

// 99% block weight is dedicated to normal extrinsics leaving 1% reserved space for the operational
// extrinsics.
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(99);
// The whole process for a single block should take 1s, of which 400ms is for creation,
// 200ms for propagation and 400ms for validation. Hence the block weight should be within 400ms.
pub const MAX_BLOCK_WEIGHT: Weight =
    Weight::from_parts(WEIGHT_REF_TIME_PER_MILLIS.saturating_mul(400), 0);

// The storage deposit is roughly 1 TOKEN per 1kB -- this is the legacy value, used for pallet Identity and Multisig.
pub const LEGACY_DEPOSIT_PER_BYTE: Balance = MILLI_AZERO;

// The storage per one byte of contract storage: 4*10^{-5} AZERO per byte.
pub const CONTRACT_DEPOSIT_PER_BYTE: Balance = 4 * (TOKEN / 100_000);

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;
    pub const BlockHashCount: AlephBlockNumber = 2400;
    pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
        ::with_sensible_defaults(MAX_BLOCK_WEIGHT.set_proof_size(u64::MAX), NORMAL_DISPATCH_RATIO);
    pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
        ::max_with_normal_ratio(MAX_BLOCK_SIZE, NORMAL_DISPATCH_RATIO);
    pub const SS58Prefix: u8 = ADDRESSES_ENCODING;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = InsideBoth<SafeMode, TxPause>;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = BlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = BlockLength;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    /// The aggregated Task type.
    type RuntimeTask = RuntimeTask;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = AccountIdLookup<AccountId, ()>;
    /// The type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The block type.
    type Block = Block;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    // https://github.com/paritytech/polkadot/blob/9ce5f7ef5abb1a4291454e8c9911b304d80679f9/runtime/polkadot/src/lib.rs#L784
    pub const MaxAuthorities: u32 = 100_000;
}

impl pallet_aura::Config for Runtime {
    type MaxAuthorities = MaxAuthorities;
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type AllowMultipleBlocksPerSlot = ConstBool<false>;
}

parameter_types! {
    pub const UncleGenerations: AlephBlockNumber = 0;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type EventHandler = (CommitteeManagement,);
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 500 * PICO_AZERO;
    pub const MaxLocks: u32 = 50;
    pub const MaxHolds: u32 = 50;
    pub const MaxFreezes: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxHolds = MaxHolds;
    type MaxFreezes = MaxFreezes;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct EverythingToTheTreasury;

impl OnUnbalanced<NegativeImbalance> for EverythingToTheTreasury {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(fees) = fees_then_tips.next() {
            Treasury::on_unbalanced(fees);
            if let Some(tips) = fees_then_tips.next() {
                Treasury::on_unbalanced(tips);
            }
        }
    }
}

parameter_types! {
    // This value increases the priority of `Operational` transactions by adding
    // a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
    // follows polkadot : https://github.com/paritytech/polkadot/blob/9ce5f7ef5abb1a4291454e8c9911b304d80679f9/runtime/polkadot/src/lib.rs#L369
    pub const OperationalFeeMultiplier: u8 = 5;
    // We expect that on average 50% of the normal capacity will be occupied with normal txs.
    pub const TargetSaturationLevel: Perquintill = Perquintill::from_percent(50);
    // During 20 blocks the fee may not change more than by 100%. This, together with the
    // `TargetSaturationLevel` value, results in variability ~0.067. For the corresponding
    // formulas please refer to Substrate code at `frame/transaction-payment/src/lib.rs`.
    pub FeeVariability: Multiplier = Multiplier::saturating_from_rational(67, 1000);
    // Fee should never be lower than the computational cost.
    pub MinimumMultiplier: Multiplier = Multiplier::one();
    pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

pub struct DivideFeeBy<const N: Balance>;

impl<const N: Balance> WeightToFee for DivideFeeBy<N> {
    type Balance = Balance;

    fn weight_to_fee(weight: &Weight) -> Self::Balance {
        Balance::saturated_from(weight.ref_time()).saturating_div(N)
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = CurrencyAdapter<Balances, EverythingToTheTreasury>;
    type LengthToFee = DivideFeeBy<10>;
    type WeightToFee = DivideFeeBy<10>;
    type FeeMultiplierUpdate = TargetedFeeAdjustment<
        Self,
        TargetSaturationLevel,
        FeeVariability,
        MinimumMultiplier,
        MaximumMultiplier,
    >;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = ();
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

pub struct SessionInfoImpl;
impl SessionInfoProvider<AlephBlockNumber> for SessionInfoImpl {
    fn current_session() -> SessionIndex {
        pallet_session::CurrentIndex::<Runtime>::get()
    }
    fn next_session_block_number(current_block: AlephBlockNumber) -> Option<AlephBlockNumber> {
        <Runtime as pallet_session::Config>::NextSessionRotation::estimate_next_session_rotation(
            current_block,
        )
        .0
    }
}

pub struct TotalIssuanceProvider;
impl TotalIssuanceProviderT for TotalIssuanceProvider {
    fn get() -> Balance {
        pallet_balances::Pallet::<Runtime>::total_issuance()
    }
}

impl pallet_aleph::Config for Runtime {
    type AuthorityId = AlephId;
    type RuntimeEvent = RuntimeEvent;
    type SessionInfoProvider = SessionInfoImpl;
    type SessionManager = SessionAndEraManager<
        Staking,
        Elections,
        pallet_session::historical::NoteHistoricalRoot<Runtime, Staking>,
        Runtime,
    >;
    type NextSessionAuthorityProvider = Session;
    type TotalIssuanceProvider = TotalIssuanceProvider;
}

parameter_types! {
    pub const SessionPeriod: u32 = DEFAULT_SESSION_PERIOD;
    pub const MaximumBanReasonLength: u32 = DEFAULT_BAN_REASON_LENGTH;
    pub const MaxWinners: u32 = DEFAULT_MAX_WINNERS;
}

impl pallet_elections::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type DataProvider = Staking;
    type ValidatorProvider = Staking;
    type MaxWinners = MaxWinners;
    type BannedValidators = CommitteeManagement;
}

impl pallet_operations::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AccountInfoProvider = System;
    type BalancesProvider = Balances;
    type NextKeysSessionProvider = Session;
    type BondedStashProvider = Staking;
    type ContractInfoProvider = Contracts;
}

impl pallet_committee_management::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BanHandler = Elections;
    type EraInfoProvider = Staking;
    type ValidatorProvider = Elections;
    type ValidatorRewardsHandler = Staking;
    type ValidatorExtractor = Staking;
    type FinalityCommitteeManager = Aleph;
    type SessionPeriod = SessionPeriod;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

parameter_types! {
    pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type ShouldEndSession = pallet_session::PeriodicSessions<SessionPeriod, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<SessionPeriod, Offset>;
    type SessionManager = Aleph;
    type SessionHandler = (Aura, Aleph);
    type Keys = SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

parameter_types! {
    pub const PostUnbondPoolsWindow: u32 = 4;
    pub const NominationPoolsPalletId: PalletId = PalletId(*b"py/nopls");
    pub const MaxPointsToBalance: u8 = 10;
}

pub struct BalanceToU256;

impl Convert<Balance, sp_core::U256> for BalanceToU256 {
    fn convert(balance: Balance) -> sp_core::U256 {
        sp_core::U256::from(balance)
    }
}

pub struct U256ToBalance;

impl Convert<sp_core::U256, Balance> for U256ToBalance {
    fn convert(n: sp_core::U256) -> Balance {
        n.try_into().unwrap_or(Balance::max_value())
    }
}

impl pallet_nomination_pools::Config for Runtime {
    type WeightInfo = ();
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type RewardCounter = FixedU128;
    type BalanceToU256 = BalanceToU256;
    type U256ToBalance = U256ToBalance;
    type Staking = pallet_staking::Pallet<Self>;
    type PostUnbondingPoolsWindow = PostUnbondPoolsWindow;
    type MaxMetadataLen = ConstU32<256>;
    type MaxUnbonding = ConstU32<8>;
    type PalletId = NominationPoolsPalletId;
    type MaxPointsToBalance = MaxPointsToBalance;
    type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
    pub const BondingDuration: EraIndex = 14;
    pub const SlashDeferDuration: EraIndex = 13;
    // this is coupled with weights for payout_stakers() call
    // see custom implementation of WeightInfo below
    pub const MaxExposurePageSize: u32 = MAX_NOMINATORS_REWARDED_PER_VALIDATOR;
    pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(33);
    pub const SessionsPerEra: EraIndex = DEFAULT_SESSIONS_PER_ERA;
    pub HistoryDepth: u32 = 84;
}

pub struct ExponentialEraPayout;

impl ExponentialEraPayout {
    fn era_payout(total_issuance: Balance, era_duration_millis: u64) -> (Balance, Balance) {
        const VALIDATOR_REWARD: Perbill = Perbill::from_percent(90);

        let azero_cap = pallet_aleph::AzeroCap::<Runtime>::get();
        let horizon = pallet_aleph::ExponentialInflationHorizon::<Runtime>::get();

        let total_payout: Balance =
            exp_helper(Perbill::from_rational(era_duration_millis, horizon))
                * (azero_cap.saturating_sub(total_issuance));
        let validators_payout = VALIDATOR_REWARD * total_payout;
        let rest = total_payout - validators_payout;

        (validators_payout, rest)
    }
}

/// Calculates 1 - exp(-x) for small positive x
fn exp_helper(x: Perbill) -> Perbill {
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x2 * x2;
    let x5 = x4 * x;
    (x - x2 / 2 + x3 / 6 - x4 / 24 + x5 / 120).min(x)
}

impl pallet_staking::EraPayout<Balance> for ExponentialEraPayout {
    fn era_payout(
        _: Balance,
        total_issuance: Balance,
        era_duration_millis: u64,
    ) -> (Balance, Balance) {
        ExponentialEraPayout::era_payout(total_issuance, era_duration_millis)
    }
}

type SubstrateStakingWeights = pallet_staking::weights::SubstrateWeight<Runtime>;

pub struct PayoutStakersDecreasedWeightInfo;

impl pallet_staking::WeightInfo for PayoutStakersDecreasedWeightInfo {
    // To make possible to change nominators per validator we need to decrease weight for payout_stakers
    fn payout_stakers_alive_staked(n: u32) -> Weight {
        SubstrateStakingWeights::payout_stakers_alive_staked(n) / 2
    }
    wrap_methods!(
        (bond(), SubstrateStakingWeights, Weight),
        (bond_extra(), SubstrateStakingWeights, Weight),
        (unbond(), SubstrateStakingWeights, Weight),
        (
            withdraw_unbonded_update(s: u32),
            SubstrateStakingWeights,
            Weight
        ),
        (
            withdraw_unbonded_kill(s: u32),
            SubstrateStakingWeights,
            Weight
        ),
        (validate(), SubstrateStakingWeights, Weight),
        (kick(k: u32), SubstrateStakingWeights, Weight),
        (nominate(n: u32), SubstrateStakingWeights, Weight),
        (chill(), SubstrateStakingWeights, Weight),
        (set_payee(), SubstrateStakingWeights, Weight),
        (update_payee(), SubstrateStakingWeights, Weight),
        (set_controller(), SubstrateStakingWeights, Weight),
        (set_validator_count(), SubstrateStakingWeights, Weight),
        (force_no_eras(), SubstrateStakingWeights, Weight),
        (force_new_era(), SubstrateStakingWeights, Weight),
        (force_new_era_always(), SubstrateStakingWeights, Weight),
        (set_invulnerables(v: u32), SubstrateStakingWeights, Weight),
        (deprecate_controller_batch(i: u32), SubstrateStakingWeights, Weight),
        (force_unstake(s: u32), SubstrateStakingWeights, Weight),
        (
            cancel_deferred_slash(s: u32),
            SubstrateStakingWeights,
            Weight
        ),
        (rebond(l: u32), SubstrateStakingWeights, Weight),
        (reap_stash(s: u32), SubstrateStakingWeights, Weight),
        (new_era(v: u32, n: u32), SubstrateStakingWeights, Weight),
        (
            get_npos_voters(v: u32, n: u32),
            SubstrateStakingWeights,
            Weight
        ),
        (get_npos_targets(v: u32), SubstrateStakingWeights, Weight),
        (chill_other(), SubstrateStakingWeights, Weight),
        (
            set_staking_configs_all_set(),
            SubstrateStakingWeights,
            Weight
        ),
        (
            set_staking_configs_all_remove(),
            SubstrateStakingWeights,
            Weight
        ),
        (
            force_apply_min_commission(),
            SubstrateStakingWeights,
            Weight
        ),
        (set_min_commission(), SubstrateStakingWeights, Weight)
    );
}

pub struct StakingBenchmarkingConfig;

impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
    type MaxValidators = ConstU32<1000>;
    type MaxNominators = ConstU32<1000>;
}

const MAX_NOMINATORS: u32 = 1;

impl pallet_staking::Config for Runtime {
    // Do not change this!!! It guarantees that we have DPoS instead of NPoS.
    type Currency = Balances;
    type UnixTime = Timestamp;
    type CurrencyToVote = U128CurrencyToVote;
    type ElectionProvider = Elections;
    type GenesisElectionProvider = Elections;
    type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_NOMINATORS>;
    type RewardRemainder = Treasury;
    type RuntimeEvent = RuntimeEvent;
    type Slash = Treasury;
    type Reward = ();
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    type SessionInterface = Self;
    type EraPayout = ExponentialEraPayout;
    type NextNewSession = Session;
    type MaxExposurePageSize = MaxExposurePageSize;
    type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
    type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Runtime>;
    type MaxUnlockingChunks = ConstU32<16>;
    type MaxControllersInDeprecationBatch = ConstU32<4084>;
    type BenchmarkingConfig = StakingBenchmarkingConfig;
    type WeightInfo = PayoutStakersDecreasedWeightInfo;
    type CurrencyBalance = Balance;
    type HistoryDepth = HistoryDepth;
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type AdminOrigin = EnsureRoot<AccountId>;
    type EventListeners = NominationPools;
}

parameter_types! {
    pub const MinimumPeriod: u64 = MILLISECS_PER_BLOCK / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    RuntimeCall: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = RuntimeCall;
}

parameter_types! {
    pub const MinVestedTransfer: Balance = MICRO_AZERO;
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons = WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    type BlockNumberProvider = System;
    // Maximum number of vesting schedules an account may have at a given moment
    // follows polkadot https://github.com/paritytech/polkadot/blob/9ce5f7ef5abb1a4291454e8c9911b304d80679f9/runtime/polkadot/src/lib.rs#L980
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
    // One storage item; key size is 32+32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = 120 * LEGACY_DEPOSIT_PER_BYTE;
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = 32 * LEGACY_DEPOSIT_PER_BYTE;
    pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

#[cfg(not(feature = "enable_treasury_proposals"))]
// This value effectively disables treasury.
pub const TREASURY_PROPOSAL_BOND: Balance = 100_000_000_000 * TOKEN;

#[cfg(feature = "enable_treasury_proposals")]
pub const TREASURY_PROPOSAL_BOND: Balance = 100 * TOKEN;

parameter_types! {
    // We do not burn any money within treasury.
    pub const Burn: Permill = Permill::from_percent(0);
    // The fraction of the proposal that the proposer should deposit.
    // We agreed on non-progressive deposit.
    pub const ProposalBond: Permill = Permill::from_percent(0);
    // The minimal deposit for proposal.
    pub const ProposalBondMinimum: Balance = TREASURY_PROPOSAL_BOND;
    // The upper bound of the deposit for the proposal.
    pub const ProposalBondMaximum: Balance = TREASURY_PROPOSAL_BOND;
    // Maximum number of approvals that can wait in the spending queue.
    pub const MaxApprovals: u32 = 20;
    // Every 4 hours we fund accepted proposals.
    pub const SpendPeriod: AlephBlockNumber = 4 * BLOCKS_PER_HOUR;
    pub const TreasuryPalletId: PalletId = PalletId(*b"a0/trsry");
    pub TreasuryAccount: AccountId = Treasury::account_id();
}

pub struct TreasuryGovernance;

impl SortedMembers<AccountId> for TreasuryGovernance {
    fn sorted_members() -> Vec<AccountId> {
        pallet_sudo::Pallet::<Runtime>::key().into_iter().collect()
    }
}

impl pallet_treasury::Config for Runtime {
    type ApproveOrigin = EnsureSignedBy<TreasuryGovernance, AccountId>;
    type Burn = Burn;
    type BurnDestination = ();
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type MaxApprovals = MaxApprovals;
    type OnSlash = ();
    type PalletId = TreasuryPalletId;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type ProposalBondMaximum = ProposalBondMaximum;
    type RejectOrigin = EnsureSignedBy<TreasuryGovernance, AccountId>;
    type SpendFunds = ();
    type SpendOrigin = frame_support::traits::NeverEnsureOrigin<u128>;
    type SpendPeriod = SpendPeriod;
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type AssetKind = ();
    type Beneficiary = Self::AccountId;
    type BeneficiaryLookup = IdentityLookup<Self::AccountId>;
    type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
    type BalanceConverter = UnityAssetBalanceConversion;
    type PayoutPeriod = ConstU32<0>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
    type PalletsOrigin = OriginCaller;
}

parameter_types! {
    // Refundable deposit per storage item
    pub const DepositPerItem: Balance = 32 * CONTRACT_DEPOSIT_PER_BYTE;
    // Refundable deposit per byte of storage
    pub const DepositPerByte: Balance = CONTRACT_DEPOSIT_PER_BYTE;
    // How much weight of each block can be spent on the lazy deletion queue of terminated contracts
    pub DeletionWeightLimit: Weight = Perbill::from_percent(10) * BlockWeights::get().max_block; // 40ms
    // Maximum size of the lazy deletion queue of terminated contracts.
    pub const DeletionQueueDepth: u32 = 128;
    pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
    pub CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
}

// The filter for the runtime calls that are allowed to be executed by contracts.
// Currently we allow only staking and nomination pools calls.
pub enum ContractsCallRuntimeFilter {}

impl Contains<RuntimeCall> for ContractsCallRuntimeFilter {
    fn contains(call: &RuntimeCall) -> bool {
        matches!(
            call,
            RuntimeCall::Staking(_) | RuntimeCall::NominationPools(_)
        )
    }
}

impl pallet_contracts::Config for Runtime {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;

    type CallFilter = ContractsCallRuntimeFilter;
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type Schedule = Schedule;
    type CallStack = [pallet_contracts::Frame<Self>; 16];
    type DepositPerByte = DepositPerByte;
    type DefaultDepositLimit = ConstU128<{ u128::MAX }>;
    type DepositPerItem = DepositPerItem;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 256 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type UnsafeUnstableInterface = ConstBool<false>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Migrations = ();
    type MaxDelegateDependencies = ConstU32<32>;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type Debug = ();
    type Environment = ();
    type Xcm = ();
}

parameter_types! {
    // bytes count taken from:
    // https://github.com/paritytech/polkadot/blob/016dc7297101710db0483ab6ef199e244dff711d/runtime/kusama/src/lib.rs#L995
    pub const BasicDeposit: Balance = 258 * LEGACY_DEPOSIT_PER_BYTE;
    pub const ByteDeposit: Balance = 66 * LEGACY_DEPOSIT_PER_BYTE;
    pub const SubAccountDeposit: Balance = 53 * LEGACY_DEPOSIT_PER_BYTE;
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type ByteDeposit = ByteDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = Treasury;
    type ForceOrigin = EnsureRoot<AccountId>;
    type RegistrarOrigin = EnsureRoot<AccountId>;
    type OffchainSignature = Signature;
    type SigningPublicKey = <Signature as Verify>::Signer;
    type UsernameAuthorityOrigin = EnsureRoot<AccountId>;
    type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
    type MaxSuffixLength = ConstU32<7>;
    type MaxUsernameLength = ConstU32<32>;
    type WeightInfo = pallet_identity::weights::SubstrateWeight<Self>;
    type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
}
parameter_types! {
    // Key size = 32, value size = 8
    pub const ProxyDepositBase: Balance = 40 * LEGACY_DEPOSIT_PER_BYTE;
    // One storage item (32) plus `ProxyType` (1) encode len.
    pub const ProxyDepositFactor: Balance = 33 * LEGACY_DEPOSIT_PER_BYTE;
    // Key size = 32, value size 8
    pub const AnnouncementDepositBase: Balance =  40 * LEGACY_DEPOSIT_PER_BYTE;
    // AccountId, Hash and BlockNumber sum up to 68
    pub const AnnouncementDepositFactor: Balance =  68 * LEGACY_DEPOSIT_PER_BYTE;
}
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Encode,
    Decode,
    RuntimeDebug,
    MaxEncodedLen,
    scale_info::TypeInfo,
)]
pub enum ProxyType {
    Any = 0,
    NonTransfer = 1,
    Staking = 2,
    Nomination = 3,
}
impl Default for ProxyType {
    fn default() -> Self {
        Self::Any
    }
}
impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::NonTransfer => matches!(
                c,
                RuntimeCall::Staking(..)
                    | RuntimeCall::Session(..)
                    | RuntimeCall::Treasury(..)
                    | RuntimeCall::Vesting(pallet_vesting::Call::vest { .. })
                    | RuntimeCall::Vesting(pallet_vesting::Call::vest_other { .. })
                    | RuntimeCall::Vesting(pallet_vesting::Call::merge_schedules { .. })
                    | RuntimeCall::Utility(..)
                    | RuntimeCall::Multisig(..)
                    | RuntimeCall::NominationPools(..)
                    | RuntimeCall::Identity(..)
            ),
            ProxyType::Staking => {
                matches!(
                    c,
                    RuntimeCall::Staking(..)
                        | RuntimeCall::Session(..)
                        | RuntimeCall::Utility(..)
                        | RuntimeCall::NominationPools(..)
                )
            }
            ProxyType::Nomination => {
                matches!(
                    c,
                    RuntimeCall::Staking(pallet_staking::Call::nominate { .. })
                )
            }
        }
    }
    fn is_superset(&self, o: &Self) -> bool {
        // ProxyType::Nomination ⊆ ProxyType::Staking ⊆ ProxyType::NonTransfer ⊆ ProxyType::Any
        match self {
            ProxyType::Any => true,
            ProxyType::NonTransfer => match o {
                ProxyType::Any => false,
                ProxyType::NonTransfer | ProxyType::Staking | ProxyType::Nomination => true,
            },
            ProxyType::Staking => match o {
                ProxyType::Any | ProxyType::NonTransfer => false,
                ProxyType::Staking | ProxyType::Nomination => true,
            },
            ProxyType::Nomination => match o {
                ProxyType::Any | ProxyType::NonTransfer | ProxyType::Staking => false,
                ProxyType::Nomination => true,
            },
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = ConstU32<32>;
    type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
    type MaxPending = ConstU32<32>;
    type CallHasher = BlakeTwo256;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
    pub const DisallowPermissionlessEnterDuration: AlephBlockNumber = 0;
    pub const DisallowPermissionlessExtendDuration: AlephBlockNumber = 0;

    // Safe mode on enter will last 1 session
    pub const RootEnterDuration: AlephBlockNumber = DEFAULT_SESSION_PERIOD;
    // Safe mode on extend will 1 session
    pub const RootExtendDuration: AlephBlockNumber = DEFAULT_SESSION_PERIOD;

    pub const DisallowPermissionlessEntering: Option<Balance> = None;
    pub const DisallowPermissionlessExtending: Option<Balance> = None;
    pub const DisallowPermissionlessRelease: Option<AlephBlockNumber> = None;
}

/// Calls that can bypass the safe-mode pallet.
pub struct SafeModeWhitelistedCalls;
impl Contains<RuntimeCall> for SafeModeWhitelistedCalls {
    fn contains(call: &RuntimeCall) -> bool {
        matches!(
            call,
            RuntimeCall::Sudo(_)
                | RuntimeCall::System(_)
                | RuntimeCall::SafeMode(_)
                | RuntimeCall::Timestamp(_)
        )
    }
}

impl pallet_safe_mode::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type WhitelistedCalls = SafeModeWhitelistedCalls;
    type EnterDuration = DisallowPermissionlessEnterDuration;
    type ExtendDuration = DisallowPermissionlessExtendDuration;
    type EnterDepositAmount = DisallowPermissionlessEntering;
    type ExtendDepositAmount = DisallowPermissionlessExtending;
    type ForceEnterOrigin = EnsureRootWithSuccess<AccountId, RootEnterDuration>;
    type ForceExtendOrigin = EnsureRootWithSuccess<AccountId, RootExtendDuration>;
    type ForceExitOrigin = EnsureRoot<AccountId>;
    type ForceDepositOrigin = EnsureRoot<AccountId>;
    type Notify = ();
    type ReleaseDelay = DisallowPermissionlessRelease;
    type WeightInfo = pallet_safe_mode::weights::SubstrateWeight<Runtime>;
}

/// Calls that can bypass the tx-pause pallet.
/// We always allow system calls and timestamp since it is required for block production
pub struct TxPauseWhitelistedCalls;
impl Contains<RuntimeCallNameOf<Runtime>> for TxPauseWhitelistedCalls {
    fn contains(full_name: &RuntimeCallNameOf<Runtime>) -> bool {
        matches!(full_name.0.as_slice(), b"Sudo" | b"System" | b"Timestamp")
    }
}

impl pallet_tx_pause::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PauseOrigin = EnsureRoot<AccountId>;
    type UnpauseOrigin = EnsureRoot<AccountId>;
    type WhitelistedCalls = TxPauseWhitelistedCalls;
    type MaxNameLen = ConstU32<256>;
    type WeightInfo = pallet_tx_pause::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
    pub struct Runtime {
        System: frame_system = 0,
        RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip = 1,
        Scheduler: pallet_scheduler = 2,
        Aura: pallet_aura = 3,
        Timestamp: pallet_timestamp = 4,
        Balances: pallet_balances = 5,
        TransactionPayment: pallet_transaction_payment = 6,
        Authorship: pallet_authorship = 7,
        Staking: pallet_staking = 8,
        History: pallet_session::historical = 9,
        Session: pallet_session = 10,
        Aleph: pallet_aleph = 11,
        Elections: pallet_elections = 12,
        Treasury: pallet_treasury = 13,
        Vesting: pallet_vesting = 14,
        Utility: pallet_utility = 15,
        Multisig: pallet_multisig = 16,
        Sudo: pallet_sudo = 17,
        Contracts: pallet_contracts = 18,
        NominationPools: pallet_nomination_pools = 19,
        Identity: pallet_identity = 20,
        CommitteeManagement: pallet_committee_management = 21,
        Proxy: pallet_proxy = 22,
        SafeMode: pallet_safe_mode = 23,
        TxPause: pallet_tx_pause = 24,
        Operations: pallet_operations = 255,
    }
);

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<AlephHeader, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    frame_benchmarking::define_benchmarks!();
}

type EventRecord = frame_system::EventRecord<RuntimeEvent, Hash>;

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> sp_std::vec::Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> SlotDuration {
            SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities().to_vec()
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_aleph_runtime_api::AlephSessionApi<Block> for Runtime {
        fn millisecs_per_block() -> u64 {
            MILLISECS_PER_BLOCK
        }

        fn session_period() -> u32 {
            SessionPeriod::get()
        }

        fn authorities() -> Vec<AlephId> {
            Aleph::authorities()
        }

        fn next_session_authorities() -> Result<Vec<AlephId>, AlephApiError> {
            let next_authorities = Aleph::next_authorities();
            if next_authorities.is_empty() {
                return Err(AlephApiError::DecodeKey)
            }

            Ok(next_authorities)
        }

        fn authority_data() -> SessionAuthorityData {
            SessionAuthorityData::new(Aleph::authorities(), Aleph::emergency_finalizer())
        }

        fn next_session_authority_data() -> Result<SessionAuthorityData, AlephApiError> {
            Ok(SessionAuthorityData::new(
                Self::next_session_authorities()?,
                Aleph::queued_emergency_finalizer(),
            ))
        }

        fn finality_version() -> FinalityVersion {
            Aleph::finality_version()
        }

        fn next_session_finality_version() -> FinalityVersion {
            Aleph::next_session_finality_version()
        }

        fn predict_session_committee(
            session: SessionIndex,
        ) -> Result<SessionCommittee<AccountId>, SessionValidatorError> {
            CommitteeManagement::predict_session_committee_for_session(session)
        }

        fn next_session_aura_authorities() -> Vec<(AccountId, AuraId)> {
            let queued_keys = QueuedKeys::<Runtime>::get();

            queued_keys.into_iter().filter_map(|(account_id, keys)| keys.get(AURA).map(|key| (account_id, key))).collect()
        }

        fn key_owner(key: AlephId) -> Option<AccountId> {
            Session::key_owner(primitives::KEY_TYPE, key.as_ref())
        }

        fn yearly_inflation() -> Perbill {
            // Milliseconds per year for the Julian year (365.25 days).
            const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
            let total_issuance = pallet_balances::Pallet::<Runtime>::total_issuance();

            let (validator_payout, rest)
                = ExponentialEraPayout::era_payout(total_issuance, MILLISECONDS_PER_YEAR);

            Perbill::from_rational(validator_payout + rest, total_issuance)
        }

        fn current_era_payout() -> (Balance, Balance) {
            const MILLISECONDS_PER_ERA: u64 = MILLISECS_PER_BLOCK * (DEFAULT_SESSION_PERIOD * DEFAULT_SESSIONS_PER_ERA) as u64;
            let total_issuance = pallet_balances::Pallet::<Runtime>::total_issuance();

            ExponentialEraPayout::era_payout(total_issuance, MILLISECONDS_PER_ERA)
        }
    }

    impl pallet_nomination_pools_runtime_api::NominationPoolsApi<Block, AccountId, Balance> for Runtime {
        fn pending_rewards(member: AccountId) -> Balance {
            NominationPools::api_pending_rewards(member).unwrap_or_default()
        }

        fn points_to_balance(pool_id: pallet_nomination_pools::PoolId, points: Balance) -> Balance {
            NominationPools::api_points_to_balance(pool_id, points)
        }

        fn balance_to_points(pool_id: pallet_nomination_pools::PoolId, new_funds: Balance) -> Balance {
            NominationPools::api_balance_to_points(pool_id, new_funds)
        }
    }

    impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
        fn nominations_quota(_balance: Balance) -> u32 {
            MAX_NOMINATORS
        }

        fn eras_stakers_page_count(era: sp_staking::EraIndex, account: AccountId) -> sp_staking::Page {
            Staking::api_eras_stakers_page_count(era, account)
        }
    }

    impl pallet_contracts::ContractsApi<Block, AccountId, Balance, AlephBlockNumber, Hash, EventRecord>
        for Runtime
    {
        fn call(
            origin: AccountId,
            dest: AccountId,
            value: Balance,
            gas_limit: Option<Weight>,
            storage_deposit_limit: Option<Balance>,
            input_data: Vec<u8>,
        ) -> pallet_contracts::ContractExecResult<Balance, EventRecord> {
            let gas_limit = gas_limit.unwrap_or(BlockWeights::get().max_block);
            Contracts::bare_call(
                origin,
                dest,
                value,
                gas_limit,
                storage_deposit_limit,
                input_data,
                pallet_contracts::DebugInfo::UnsafeDebug,
                pallet_contracts::CollectEvents::UnsafeCollect,
                pallet_contracts::Determinism::Enforced,
            )
        }

        fn instantiate(
            origin: AccountId,
            value: Balance,
            gas_limit: Option<Weight>,
            storage_deposit_limit: Option<Balance>,
            code: pallet_contracts::Code<Hash>,
            data: Vec<u8>,
            salt: Vec<u8>,
        ) -> pallet_contracts::ContractInstantiateResult<AccountId, Balance, EventRecord>
        {
            let gas_limit = gas_limit.unwrap_or(BlockWeights::get().max_block);
            Contracts::bare_instantiate(
                origin,
                value,
                gas_limit,
                storage_deposit_limit,
                code,
                data,
                salt,
                pallet_contracts::DebugInfo::UnsafeDebug,
                pallet_contracts::CollectEvents::UnsafeCollect,
            )
        }

        fn upload_code(
            origin: AccountId,
            code: Vec<u8>,
            storage_deposit_limit: Option<Balance>,
            determinism: pallet_contracts::Determinism,
        ) -> pallet_contracts::CodeUploadResult<Hash, Balance>
        {
            Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
        }

        fn get_storage(
            address: AccountId,
            key: Vec<u8>,
        ) -> pallet_contracts::GetStorageResult {
            Contracts::get_storage(address, key)
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: UpgradeCheckSelect) -> (Weight, Weight) {
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, BlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            checks: bool,
            select: frame_try_runtime::TryStateSelect,
        ) -> Weight {
            Executive::try_execute_block(block, state_root_check, checks, select).unwrap()
        }
     }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();

            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch};
            use frame_support::traits::WhitelistedStorageKeys;

            let whitelist: Vec<_> = AllPalletsWithSystem::whitelisted_storage_keys();

            let params = (&config, &whitelist);
            let mut batches = Vec::<BenchmarkBatch>::new();
            add_benchmarks!(params, batches);

            Ok(batches)
        }
     }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn create_default_config() -> Vec<u8> {
            create_default_config::<RuntimeGenesisConfig>()
        }

        fn build_config(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_config::<RuntimeGenesisConfig>(config)
        }
    }
}

#[cfg(test)]
mod tests {
    use frame_support::traits::Get;
    use primitives::HEAP_PAGES;
    use smallvec::Array;

    use super::*;

    #[test]
    fn test_proxy_is_superset() {
        let proxies = [
            ProxyType::Any,
            ProxyType::NonTransfer,
            ProxyType::Staking,
            ProxyType::Nomination,
        ];
        for (i, proxy) in proxies.iter().enumerate() {
            for (j, other) in proxies.iter().enumerate() {
                assert_eq!(proxy.is_superset(other), i <= j);
            }
        }
    }

    #[test]
    // This test is to make sure that we don't break call-runtime.
    fn test_staking_pallet_index() {
        // arbitrary call that is easy to construct
        let c = RuntimeCall::Staking(pallet_staking::Call::bond_extra { max_additional: 0 });
        // first byte is pallet index
        assert_eq!(c.encode()[0], 8);
    }

    #[test]
    // This test is to make sure that we don't break call-runtime.
    fn test_nomination_pools_pallet_index() {
        // arbitrary call that is easy to construct
        let c = RuntimeCall::NominationPools(pallet_nomination_pools::Call::chill { pool_id: 0 });
        // first byte is pallet index
        assert_eq!(c.encode()[0], 19);
    }

    fn match_staking_call(c: pallet_staking::Call<Runtime>) {
        match c {
            pallet_staking::Call::bond { value: _, payee: _ } => {}
            pallet_staking::Call::bond_extra { max_additional: _ } => {}
            pallet_staking::Call::unbond { value: _ } => {}
            pallet_staking::Call::withdraw_unbonded {
                num_slashing_spans: _,
            } => {}
            pallet_staking::Call::validate { prefs: _ } => {}
            pallet_staking::Call::nominate { targets: _ } => {}
            pallet_staking::Call::chill {} => {}
            pallet_staking::Call::set_payee { payee: _ } => {}
            pallet_staking::Call::set_controller {} => {}
            pallet_staking::Call::set_validator_count { new: _ } => {}
            pallet_staking::Call::increase_validator_count { additional: _ } => {}
            pallet_staking::Call::scale_validator_count { factor: _ } => {}
            pallet_staking::Call::force_no_eras {} => {}
            pallet_staking::Call::force_new_era {} => {}
            pallet_staking::Call::set_invulnerables { invulnerables: _ } => {}
            pallet_staking::Call::force_unstake {
                stash: _,
                num_slashing_spans: _,
            } => {}
            pallet_staking::Call::force_new_era_always {} => {}
            pallet_staking::Call::cancel_deferred_slash {
                era: _,
                slash_indices: _,
            } => {}
            pallet_staking::Call::payout_stakers {
                validator_stash: _,
                era: _,
            } => {}
            pallet_staking::Call::rebond { value: _ } => {}
            pallet_staking::Call::reap_stash {
                stash: _,
                num_slashing_spans: _,
            } => {}
            pallet_staking::Call::kick { who: _ } => {}
            pallet_staking::Call::set_staking_configs {
                min_nominator_bond: _,
                min_validator_bond: _,
                max_nominator_count: _,
                max_validator_count: _,
                chill_threshold: _,
                min_commission: _,
            } => {}
            pallet_staking::Call::chill_other { stash: _ } => {}
            pallet_staking::Call::force_apply_min_commission { validator_stash: _ } => {}
            pallet_staking::Call::set_min_commission { new: _ } => {}
            pallet_staking::Call::payout_stakers_by_page {
                validator_stash: _,
                era: _,
                page: _,
            } => {}
            pallet_staking::Call::update_payee { controller: _ } => {}
            pallet_staking::Call::deprecate_controller_batch { controllers: _ } => {}
            pallet_staking::Call::__Ignore(..) => {}
        }
    }

    fn match_nomination_pools_call(c: pallet_nomination_pools::Call<Runtime>) {
        match c {
            pallet_nomination_pools::Call::join {
                amount: _,
                pool_id: _,
            } => {}
            pallet_nomination_pools::Call::bond_extra { extra: _ } => {}
            pallet_nomination_pools::Call::claim_payout {} => {}
            pallet_nomination_pools::Call::unbond {
                member_account: _,
                unbonding_points: _,
            } => {}
            pallet_nomination_pools::Call::pool_withdraw_unbonded {
                pool_id: _,
                num_slashing_spans: _,
            } => {}
            pallet_nomination_pools::Call::withdraw_unbonded {
                member_account: _,
                num_slashing_spans: _,
            } => {}
            pallet_nomination_pools::Call::create {
                amount: _,
                root: _,
                nominator: _,
                bouncer: _,
            } => {}
            pallet_nomination_pools::Call::create_with_pool_id {
                amount: _,
                root: _,
                nominator: _,
                bouncer: _,
                pool_id: _,
            } => {}
            pallet_nomination_pools::Call::nominate {
                pool_id: _,
                validators: _,
            } => {}
            pallet_nomination_pools::Call::set_state {
                pool_id: _,
                state: _,
            } => {}
            pallet_nomination_pools::Call::set_metadata {
                pool_id: _,
                metadata: _,
            } => {}
            pallet_nomination_pools::Call::set_configs {
                min_join_bond: _,
                min_create_bond: _,
                max_pools: _,
                max_members: _,
                max_members_per_pool: _,
                global_max_commission: _,
            } => {}
            pallet_nomination_pools::Call::update_roles {
                pool_id: _,
                new_root: _,
                new_nominator: _,
                new_bouncer: _,
            } => {}
            pallet_nomination_pools::Call::chill { pool_id: _ } => {}
            pallet_nomination_pools::Call::bond_extra_other {
                member: _,
                extra: _,
            } => {}
            pallet_nomination_pools::Call::set_claim_permission { permission: _ } => {}
            pallet_nomination_pools::Call::claim_payout_other { other: _ } => {}
            pallet_nomination_pools::Call::set_commission {
                pool_id: _,
                new_commission: _,
            } => {}
            pallet_nomination_pools::Call::set_commission_max {
                pool_id: _,
                max_commission: _,
            } => {}
            pallet_nomination_pools::Call::set_commission_change_rate {
                pool_id: _,
                change_rate: _,
            } => {}
            pallet_nomination_pools::Call::claim_commission { pool_id: _ } => {}
            pallet_nomination_pools::Call::adjust_pool_deposit { pool_id: _ } => {}
            pallet_nomination_pools::Call::set_commission_claim_permission {
                pool_id: _,
                permission: _,
            } => {}
            pallet_nomination_pools::Call::__Ignore(..) => {}
        }
    }

    #[test]
    fn test_call_runtime_api_stability() {
        // If this thing does not compile it means there are breaking changes in staking or nomination pools pallet. This affects call-runtime.
        // Please do not fix blindly -- action required, escalate.
        let _ = {
            |c: RuntimeCall| match c {
                RuntimeCall::Staking(call) => match_staking_call(call),
                RuntimeCall::NominationPools(call) => match_nomination_pools_call(call),
                _ => {}
            }
        };
    }

    #[test]
    fn state_version_must_be_zero() {
        assert_eq!(0, VERSION.state_version);
    }

    #[test]
    fn check_contracts_memory_parameters() {
        // Memory limit of one instance of a runtime
        const MAX_RUNTIME_MEM: u32 = HEAP_PAGES as u32 * 64 * 1024;
        // Max stack size defined by wasmi - 1MB
        const MAX_STACK_SIZE: u32 = 1024 * 1024;
        // Max heap size is 16 mempages of 64KB each - 1MB
        let max_heap_size = <Runtime as pallet_contracts::Config>::Schedule::get()
            .limits
            .max_memory_size();
        // Max call depth is CallStack::size() + 1
        let max_call_depth = <Runtime as pallet_contracts::Config>::CallStack::size() as u32 + 1;
        // Max code len
        let max_code_len: u32 = <Runtime as pallet_contracts::Config>::MaxCodeLen::get();

        // The factor comes from allocator, contracts representation, and wasmi
        let lhs = max_call_depth * (36 * max_code_len + max_heap_size + MAX_STACK_SIZE);
        // We allocate only 75% of all runtime memory to contracts execution. Important: it's not
        // enforeced in wasmtime
        let rhs = MAX_RUNTIME_MEM * 3 / 4;

        assert!(lhs < rhs);
    }

    const MILLISECS_PER_DAY: u64 = 24 * 60 * 60 * 1000;

    struct EraPayoutInputs {
        azero_cap: Balance,
        horizon: u64,
        total_issuance: Balance,
        era_duration_millis: u64,
    }

    struct EraPayoutOutputs {
        validators_payout: Balance,
        rest: Balance,
    }

    fn assert_era_payout(inputs: EraPayoutInputs, outputs: EraPayoutOutputs) {
        use sp_io::TestExternalities;
        TestExternalities::default().execute_with(|| {
            pallet_aleph::AzeroCap::<Runtime>::put(inputs.azero_cap);
            pallet_aleph::ExponentialInflationHorizon::<Runtime>::put(inputs.horizon);
            let (validators_payout, rest) =
                <Runtime as pallet_staking::Config>::EraPayout::era_payout(
                    inputs.total_issuance,
                    inputs.era_duration_millis,
                );
            assert_eq!(validators_payout, outputs.validators_payout);
            assert_eq!(rest, outputs.rest);
        });
    }

    fn era_payout_multiple_eras(
        inputs: EraPayoutInputs,
        n_eras: usize,
    ) -> (Vec<EraPayoutOutputs>, Balance) {
        use sp_io::TestExternalities;
        let mut outputs = vec![];
        let mut total_issuance = inputs.total_issuance;
        for _ in 0..n_eras {
            TestExternalities::default().execute_with(|| {
                pallet_aleph::AzeroCap::<Runtime>::put(inputs.azero_cap);
                pallet_aleph::ExponentialInflationHorizon::<Runtime>::put(inputs.horizon);
                let (validators_payout, rest) =
                    <Runtime as pallet_staking::Config>::EraPayout::era_payout(
                        total_issuance,
                        inputs.era_duration_millis,
                    );
                outputs.push(EraPayoutOutputs {
                    validators_payout,
                    rest,
                });
                total_issuance += validators_payout + rest;
            });
        }
        (outputs, total_issuance)
    }

    #[test]
    fn era_payout_standard_case() {
        assert_era_payout(
            EraPayoutInputs {
                azero_cap: 100_000_000 * TOKEN,
                horizon: 365 * MILLISECS_PER_DAY,
                total_issuance: 50_000_000 * TOKEN,
                era_duration_millis: MILLISECS_PER_DAY,
            },
            EraPayoutOutputs {
                validators_payout: 123_118 * TOKEN + 920_000_000_000,
                rest: 13_679 * TOKEN + 880_000_000_000,
            },
        );
    }

    #[test]
    /// Simulate long run by calling `era_payout` multiple times,
    /// and keeping track of `total_issuance` between calls.
    /// After 3 * horizon milliseconds the gap should be reduced by ~95%.
    fn era_payout_long_run() {
        let (_, total_issuance) = era_payout_multiple_eras(
            EraPayoutInputs {
                azero_cap: 150_000_000 * TOKEN,
                horizon: 365 * MILLISECS_PER_DAY,
                total_issuance: 50_000_000 * TOKEN,
                era_duration_millis: MILLISECS_PER_DAY,
            },
            3 * 365,
        );
        assert_eq!(total_issuance, 145_021_290 * TOKEN + 959_387_724_274);
    }

    #[test]
    /// Era longer than horizon.
    /// Perbill will saturate, (era_duration_millis / horizon) == 1,
    /// even if horizon == 0.
    /// The actual values do not matter, only the ratio is used.
    /// We expect the gap to be reduced by ~63%.
    fn era_payout_horizon_too_short() {
        assert_era_payout(
            EraPayoutInputs {
                azero_cap: 100_000_000 * TOKEN,
                horizon: 0,
                total_issuance: 50_000_000 * TOKEN,
                era_duration_millis: MILLISECS_PER_DAY,
            },
            EraPayoutOutputs {
                validators_payout: 28_499_999 * TOKEN + 985_000_000_000,
                rest: 3_166_666 * TOKEN + 665_000_000_000,
            },
        );
    }

    #[test]
    /// AZERO cap equal to total issuance, we expect no payout.
    fn era_payout_cap_reached() {
        assert_era_payout(
            EraPayoutInputs {
                azero_cap: 100_000_000 * TOKEN,
                horizon: 365 * MILLISECS_PER_DAY,
                total_issuance: 100_000_000 * TOKEN,
                era_duration_millis: MILLISECS_PER_DAY,
            },
            EraPayoutOutputs {
                validators_payout: 0,
                rest: 0,
            },
        );
    }

    #[test]
    /// Total issuance larger than AZERO cap, we expect no payout.
    fn era_payout_cap_exceeded() {
        assert_era_payout(
            EraPayoutInputs {
                azero_cap: 50_000_000 * TOKEN,
                horizon: 365 * MILLISECS_PER_DAY,
                total_issuance: 100_000_000 * TOKEN,
                era_duration_millis: MILLISECS_PER_DAY,
            },
            EraPayoutOutputs {
                validators_payout: 0,
                rest: 0,
            },
        );
    }

    #[test]
    /// Zero-length era, we expect no payout (as it depends on era lenght).
    fn era_payout_zero_lenght_era() {
        assert_era_payout(
            EraPayoutInputs {
                azero_cap: 100_000_000 * TOKEN,
                horizon: 365 * MILLISECS_PER_DAY,
                total_issuance: 50_000_000 * TOKEN,
                era_duration_millis: 0,
            },
            EraPayoutOutputs {
                validators_payout: 0,
                rest: 0,
            },
        );
    }
}
