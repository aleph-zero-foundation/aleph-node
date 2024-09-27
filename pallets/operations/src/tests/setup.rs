use frame_support::{
    construct_runtime,
    pallet_prelude::ConstU32,
    parameter_types,
    traits::{ConstBool, ConstU64, Contains, OneSessionHandler, Randomness},
    weights::{RuntimeDbWeight, Weight},
};
use frame_system::{mocking::MockBlock, pallet_prelude::BlockNumberFor};
use pallet_staking::BalanceOf;
use sp_runtime::{
    testing::{UintAuthorityId, H256},
    traits::{Convert, ConvertInto, IdentityLookup},
    BuildStorage, Perbill,
};
use sp_staking::StakerStatus;
use sp_std::prelude::*;

use crate as pallet_operations;

pub(crate) type AccountId = u64;

construct_runtime!(
    pub struct TestRuntime {
        System: frame_system,
        Balances: pallet_balances,
        Operations: pallet_operations,
        Session: pallet_session,
        Staking: pallet_staking,
        Contracts: pallet_contracts,
        Timestamp: pallet_timestamp,
    }
);

parameter_types! {
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
    pub const TestDbWeight: RuntimeDbWeight = RuntimeDbWeight {
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
    type Block = MockBlock<TestRuntime>;
    type Hash = H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = TestDbWeight;
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
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
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
    type MaxHolds = ConstU32<1>;
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = ();
}

pub struct OtherSessionHandler;

impl OneSessionHandler<AccountId> for OtherSessionHandler {
    type Key = UintAuthorityId;

    fn on_genesis_session<'a, I>(_: I)
    where
        I: Iterator<Item = (&'a AccountId, Self::Key)> + 'a,
        AccountId: 'a,
    {
    }

    fn on_new_session<'a, I>(_: bool, _: I, _: I)
    where
        I: Iterator<Item = (&'a AccountId, Self::Key)> + 'a,
        AccountId: 'a,
    {
    }

    fn on_disabled(_validator_index: u32) {}
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
    type Public = UintAuthorityId;
}

sp_runtime::impl_opaque_keys! {
    pub struct TestSessionKeys {
        pub other: OtherSessionHandler,
    }
}

impl pallet_session::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = ();
    type SessionHandler = (OtherSessionHandler,);
    type Keys = TestSessionKeys;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 3;
}

impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<5>;
    type WeightInfo = ();
}

parameter_types! {
    pub static BondingDuration: u32 = 3;
}

pub struct ZeroEraPayout;

impl pallet_staking::EraPayout<u128> for ZeroEraPayout {
    fn era_payout(_: u128, _: u128, _: u64) -> (u128, u128) {
        (0, 0)
    }
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
    type SessionsPerEra = ();
    type SlashDeferDuration = ();
    type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type BondingDuration = BondingDuration;
    type SessionInterface = ();
    type EraPayout = ZeroEraPayout;
    type NextNewSession = ();
    type MaxExposurePageSize = ConstU32<64>;
    type OffendingValidatorsThreshold = ();
    type ElectionProvider =
        frame_election_provider_support::NoElection<(AccountId, u64, Staking, ConstU32<1>)>;
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
pub const UNITS: u128 = 10_000_000_000;

pub const CENTS: u128 = UNITS / 100; // 100_00
pub const fn deposit(items: u32, bytes: u32) -> u128 {
    items as u128 * CENTS + (bytes as u128) * CENTS
}

parameter_types! {
    pub const DepositPerItem: u128 = deposit(1, 0);
    pub const DepositPerByte: u128 = deposit(0, 1);
    pub const DefaultDepositLimit: u128 = deposit(1024, 1024 * 1024);
    pub Schedule: pallet_contracts::Schedule<TestRuntime> = Default::default();
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
    pub const MaxDelegateDependencies: u32 = 32;
}

pub struct DummyRandomness<T: pallet_contracts::Config>(sp_std::marker::PhantomData<T>);

impl<T: pallet_contracts::Config> Randomness<T::Hash, BlockNumberFor<T>> for DummyRandomness<T> {
    fn random(_subject: &[u8]) -> (T::Hash, BlockNumberFor<T>) {
        (Default::default(), Default::default())
    }
}

#[derive(Clone, Default)]
pub struct Filters;

impl Contains<RuntimeCall> for Filters {
    fn contains(_: &RuntimeCall) -> bool {
        todo!()
    }
}

impl Convert<Weight, BalanceOf<Self>> for TestRuntime {
    fn convert(w: Weight) -> BalanceOf<Self> {
        w.ref_time().into()
    }
}

impl pallet_contracts::Config for TestRuntime {
    type Time = pallet_timestamp::Pallet<Self>;
    type Randomness = DummyRandomness<Self>;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CallFilter = Filters;
    type WeightPrice = Self;
    type WeightInfo = ();
    type ChainExtension = ();
    type Schedule = Schedule;
    type CallStack = [pallet_contracts::Frame<Self>; 5];
    type DepositPerByte = DepositPerByte;
    type DefaultDepositLimit = DefaultDepositLimit;
    type DepositPerItem = DepositPerItem;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type MaxDelegateDependencies = MaxDelegateDependencies;
    type UnsafeUnstableInterface = ConstBool<true>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Migrations = ();
    type Debug = ();
    type Environment = ();
    type Xcm = ();
}

impl pallet_operations::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type AccountInfoProvider = System;
    type BalancesProvider = Balances;
    type NextKeysSessionProvider = Session;
    type BondedStashProvider = Staking;
    type ContractInfoProvider = Contracts;
}

pub fn new_test_ext(accounts_and_balances: &[(u64, bool, u128)]) -> sp_io::TestExternalities {
    let mut t = <frame_system::GenesisConfig<TestRuntime> as BuildStorage>::build_storage(
        &frame_system::GenesisConfig::default(),
    )
    .expect("Storage should be build.");

    assert!(!accounts_and_balances.is_empty());

    let balances: Vec<_> = accounts_and_balances
        .iter()
        .map(|(id, _, balance)| (*id, *balance))
        .collect();

    pallet_balances::GenesisConfig::<TestRuntime> { balances }
        .assimilate_storage(&mut t)
        .unwrap();

    pallet_staking::GenesisConfig::<TestRuntime> {
        validator_count: accounts_and_balances
            .iter()
            .filter(|(_, is_authority, _)| *is_authority)
            .count() as u32,
        minimum_validator_count: 1,
        invulnerables: vec![],
        force_era: Default::default(),
        slash_reward_fraction: Default::default(),
        canceled_payout: 0,
        stakers: accounts_and_balances
            .iter()
            .filter(|(_, is_authority, _)| *is_authority)
            .map(|(id, _, balance)| (*id, *id, *balance / 2, StakerStatus::<AccountId>::Validator))
            .collect(),
        min_nominator_bond: 1,
        min_validator_bond: 1,
        max_validator_count: None,
        max_nominator_count: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_session::GenesisConfig::<TestRuntime> {
        keys: accounts_and_balances
            .iter()
            .filter(|(_, is_authority, _)| *is_authority)
            .map(|(id, _, _)| {
                (
                    *id,
                    *id,
                    TestSessionKeys {
                        other: (*id).into(),
                    },
                )
            })
            .collect(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| frame_system::Pallet::<TestRuntime>::set_block_number(1));
    ext
}
