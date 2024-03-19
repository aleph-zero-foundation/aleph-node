use frame_support::{
    construct_runtime,
    pallet_prelude::ConstU32,
    parameter_types,
    traits::{ConstU64, OneSessionHandler},
    weights::{RuntimeDbWeight, Weight},
};
use frame_system::mocking::MockBlock;
use sp_runtime::{
    testing::{UintAuthorityId, H256},
    traits::{ConvertInto, IdentityLookup},
    BuildStorage,
};

use crate as pallet_operations;
pub(crate) type AccountId = u64;

construct_runtime!(
    pub struct TestRuntime {
        System: frame_system,
        Balances: pallet_balances,
        Operations: pallet_operations,
        Session: pallet_session,
        Staking: pallet_staking,
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
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
}

pub struct OtherSessionHandler;
impl OneSessionHandler<AccountId> for OtherSessionHandler {
    type Key = UintAuthorityId;

    fn on_genesis_session<'a, I: 'a>(_: I)
    where
        I: Iterator<Item = (&'a AccountId, Self::Key)>,
        AccountId: 'a,
    {
    }

    fn on_new_session<'a, I: 'a>(_: bool, _: I, _: I)
    where
        I: Iterator<Item = (&'a AccountId, Self::Key)>,
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

pub struct UniformEraPayout;

impl pallet_staking::EraPayout<u128> for UniformEraPayout {
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
    type EraPayout = UniformEraPayout;
    type NextNewSession = ();
    type MaxNominatorRewardedPerValidator = ConstU32<64>;
    type OffendingValidatorsThreshold = ();
    type ElectionProvider =
        frame_election_provider_support::NoElection<(AccountId, u64, Staking, ())>;
    type GenesisElectionProvider = Self::ElectionProvider;
    type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<TestRuntime>;
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
    type MaxUnlockingChunks = ConstU32<32>;
    type HistoryDepth = ConstU32<84>;
    type EventListeners = ();
    type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
    type WeightInfo = ();
}

impl pallet_operations::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type AccountInfoProvider = System;
    type BalancesProvider = Balances;
    type NextKeysSessionProvider = Session;
    type BondedStashProvider = Staking;
}

pub fn new_test_ext(accounts_and_balances: &[(u64, bool, u128)]) -> sp_io::TestExternalities {
    let mut t = <frame_system::GenesisConfig<TestRuntime> as BuildStorage>::build_storage(
        &frame_system::GenesisConfig::default(),
    )
    .expect("Storage should be build.");

    let balances: Vec<_> = accounts_and_balances
        .iter()
        .map(|(id, _, balance)| (*id, *balance))
        .collect();

    pallet_balances::GenesisConfig::<TestRuntime> { balances }
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
