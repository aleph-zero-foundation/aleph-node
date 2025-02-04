use frame_support::{
    construct_runtime,
    pallet_prelude::ConstU32,
    parameter_types,
    traits::{EstimateNextSessionRotation, Hooks},
    weights::{RuntimeDbWeight, Weight},
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_staking::{ExposureOf, Forcing};
use primitives::{
    AuthorityId, CommitteeSeats, SessionIndex, SessionInfoProvider,
    TotalIssuanceProvider as TotalIssuanceProviderT, DEFAULT_MAX_WINNERS, DEFAULT_SESSIONS_PER_ERA,
    DEFAULT_SESSION_PERIOD,
};
use sp_core::{ConstU64, H256};
use sp_runtime::{
    impl_opaque_keys,
    testing::{TestXt, UintAuthorityId},
    traits::{ConvertInto, IdentityLookup},
    BuildStorage, Perbill,
};
use sp_staking::{EraIndex, Exposure, StakerStatus};

use super::*;
use crate as pallet_committee_management;

type Block = frame_system::mocking::MockBlock<TestRuntime>;
pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = BlockNumberFor<TestRuntime>;

construct_runtime!(
    pub enum TestRuntime
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Staking: pallet_staking,
        Session: pallet_session,
        History: pallet_session::historical,
        Aleph: pallet_aleph,
        CommitteeManagement: pallet_committee_management,
        Elections: pallet_elections,
    }
);

impl_opaque_keys! {
    pub struct TestSessionKeys {
        pub aleph: Aleph,
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
    pub const TestRuntimeDbWeight: RuntimeDbWeight = RuntimeDbWeight {
        read: 25,
        write: 100
    };
}

impl frame_system::Config for TestRuntime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeTask = RuntimeTask;
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = TestRuntimeDbWeight;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}

impl pallet_balances::Config for TestRuntime {
    type Balance = u128;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = RuntimeFreezeReason;
}

pub struct ZeroEraPayout;
impl pallet_staking::EraPayout<u128> for ZeroEraPayout {
    fn era_payout(_: u128, _: u128, _: u64) -> (u128, u128) {
        (0, 0)
    }
}

parameter_types! {
    pub const SessionsPerEra: SessionIndex = DEFAULT_SESSIONS_PER_ERA;
    pub static BondingDuration: u32 = 3;
}

impl pallet_staking::Config for TestRuntime {
    type Currency = Balances;
    type CurrencyBalance = u128;
    type UnixTime = pallet_timestamp::Pallet<Self>;
    type CurrencyToVote = ();
    type RewardRemainder = ();
    type RuntimeEvent = RuntimeEvent;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = SessionsPerEra;
    type SlashDeferDuration = ();
    type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type BondingDuration = BondingDuration;
    type SessionInterface = Self;
    type EraPayout = ZeroEraPayout;
    type NextNewSession = Session;
    type MaxExposurePageSize = ConstU32<64>;
    type OffendingValidatorsThreshold = ();
    type ElectionProvider = Elections;
    type GenesisElectionProvider = Self::ElectionProvider;
    type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<TestRuntime>;
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
    type MaxUnlockingChunks = ConstU32<32>;
    type MaxControllersInDeprecationBatch = ConstU32<64>;
    type HistoryDepth = ConstU32<84>;
    type EventListeners = ();
    type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
    type WeightInfo = ();
}

impl pallet_session::historical::Config for TestRuntime {
    type FullIdentification = Exposure<AccountId, Balance>;
    type FullIdentificationOf = ExposureOf<TestRuntime>;
}

pub struct SessionInfoImpl;
impl SessionInfoProvider<BlockNumberFor<TestRuntime>> for SessionInfoImpl {
    fn current_session() -> SessionIndex {
        pallet_session::CurrentIndex::<TestRuntime>::get()
    }
    fn next_session_block_number(
        current_block: BlockNumberFor<TestRuntime>,
    ) -> Option<BlockNumberFor<TestRuntime>> {
        <TestRuntime as pallet_session::Config>::NextSessionRotation::estimate_next_session_rotation(
            current_block,
        )
        .0
    }
}

parameter_types! {
    pub const SessionPeriod: u32 = DEFAULT_SESSION_PERIOD;
    pub const Offset: u64 = 0;
}

impl pallet_session::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<SessionPeriod, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<SessionPeriod, Offset>;
    type SessionManager = Aleph;
    type SessionHandler = (Aleph,);
    type Keys = TestSessionKeys;
    type WeightInfo = ();
}

parameter_types! {
    pub const ScoreSubmissionPeriod: u32 = 15;
}

impl pallet_aleph::Config for TestRuntime {
    type AuthorityId = AuthorityId;
    type RuntimeEvent = RuntimeEvent;
    type SessionInfoProvider = SessionInfoImpl;
    type SessionManager = SessionAndEraManager<
        Staking,
        Elections,
        pallet_session::historical::NoteHistoricalRoot<TestRuntime, Staking>,
        TestRuntime,
    >;
    type NextSessionAuthorityProvider = Session;
    type TotalIssuanceProvider = TotalIssuanceProvider;
    type ScoreSubmissionPeriod = ScoreSubmissionPeriod;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for TestRuntime
where
    RuntimeCall: From<C>,
{
    type Extrinsic = TestXt<RuntimeCall, ()>;
    type OverarchingCall = RuntimeCall;
}

impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<3>;
    type WeightInfo = ();
}

pub struct TotalIssuanceProvider;
impl TotalIssuanceProviderT for TotalIssuanceProvider {
    fn get() -> Balance {
        pallet_balances::Pallet::<TestRuntime>::total_issuance()
    }
}

parameter_types! {
    pub static MaxWinners: u32 = DEFAULT_MAX_WINNERS;
}

impl pallet_elections::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type DataProvider = Staking;
    type ValidatorProvider = Staking;
    type MaxWinners = MaxWinners;
    type BannedValidators = CommitteeManagement;
}

impl Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type BanHandler = Elections;
    type EraInfoProvider = Staking;
    type ValidatorProvider = Elections;
    type ValidatorRewardsHandler = Staking;
    type ValidatorExtractor = Staking;
    type FinalityCommitteeManager = Aleph;
    type SessionPeriod = SessionPeriod;
    type AbftScoresProvider = Aleph;
}

pub fn active_era() -> EraIndex {
    pallet_staking::ActiveEra::<TestRuntime>::get()
        .unwrap()
        .index
}

pub const INIT_TIMESTAMP: u64 = 100_000;
pub const BLOCK_TIME: u64 = 1000;

pub fn run_to_block(n: BlockNumber) {
    Staking::on_finalize(System::block_number());
    for b in System::block_number() + 1..=n {
        System::set_block_number(b);
        Session::on_initialize(b);
        Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
        if b != n {
            Staking::on_finalize(System::block_number());
        }
    }
}

pub fn start_session(session_index: SessionIndex) {
    let end = session_index * SessionPeriod::get();
    run_to_block(end as u64);
    assert_eq!(
        Session::current_index(),
        session_index,
        "current session index = {}, expected = {}",
        Session::current_index(),
        session_index,
    );
}

pub(crate) fn advance_era() {
    let active_era = active_era();
    let first_session_in_next_era = SessionsPerEra::get() * (active_era + 1);
    start_session(first_session_in_next_era);
}

pub(crate) fn committee_management_events() -> Vec<crate::Event<TestRuntime>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let RuntimeEvent::CommitteeManagement(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect()
}

pub struct TestBuilderConfig {
    pub reserved_validators: Vec<AccountId>,
    pub non_reserved_validators: Vec<AccountId>,
    pub non_reserved_seats: u32,
    pub non_reserved_finality_seats: u32,
}

pub struct TestExtBuilder {
    reserved_validators: Vec<AccountId>,
    non_reserved_validators: Vec<AccountId>,
    committee_seats: CommitteeSeats,
}

impl TestExtBuilder {
    pub fn new(config: TestBuilderConfig) -> Self {
        let TestBuilderConfig {
            reserved_validators,
            non_reserved_validators,
            non_reserved_seats,
            non_reserved_finality_seats,
        } = config;
        Self {
            committee_seats: CommitteeSeats {
                reserved_seats: reserved_validators.len() as u32,
                non_reserved_seats,
                non_reserved_finality_seats,
            },
            reserved_validators,
            non_reserved_validators,
        }
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = <frame_system::GenesisConfig<TestRuntime> as BuildStorage>::build_storage(
            &frame_system::GenesisConfig::default(),
        )
        .expect("Storage should be build.");

        let validators: Vec<_> = self
            .non_reserved_validators
            .iter()
            .chain(self.reserved_validators.iter())
            .collect();

        let balances: Vec<_> = validators.iter().map(|i| (**i, 10_000_000)).collect();

        pallet_balances::GenesisConfig::<TestRuntime> { balances }
            .assimilate_storage(&mut t)
            .unwrap();

        pallet_staking::GenesisConfig::<TestRuntime> {
            validator_count: self.committee_seats.size(),
            minimum_validator_count: 1,
            invulnerables: vec![],
            force_era: Forcing::NotForcing,
            slash_reward_fraction: Perbill::from_percent(0),
            canceled_payout: 0,
            stakers: validators
                .iter()
                .map(|&&v| (v, v, 5_000_000, StakerStatus::<AccountId>::Validator))
                .collect(),
            min_nominator_bond: 1,
            min_validator_bond: 1,
            max_validator_count: None,
            max_nominator_count: None,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let session_keys: Vec<_> = validators
            .iter()
            .map(|&&v| UintAuthorityId(v).to_public_key::<AuthorityId>())
            .enumerate()
            .map(|(i, k)| (i as u64, i as u64, TestSessionKeys { aleph: k }))
            .collect();

        pallet_session::GenesisConfig::<TestRuntime> { keys: session_keys }
            .assimilate_storage(&mut t)
            .unwrap();

        pallet_elections::GenesisConfig::<TestRuntime> {
            non_reserved_validators: self.non_reserved_validators,
            reserved_validators: self.reserved_validators,
            committee_seats: self.committee_seats,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::from(t);
        ext.execute_with(|| {
            System::set_block_number(1);
            Session::on_initialize(1);
            <Staking as Hooks<u64>>::on_initialize(1);
            Timestamp::set_timestamp(INIT_TIMESTAMP);
        });

        ext
    }
}
