use frame_support::{
    construct_runtime,
    pallet_prelude::ConstU32,
    parameter_types, sp_runtime,
    sp_runtime::{
        testing::{Header, H256},
        traits::IdentityLookup,
    },
    traits::Everything,
};
use frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use pallet_balances::AccountData;
use sp_core::ConstU64;
use sp_io::TestExternalities;
use sp_runtime::traits::BlakeTwo256;

use crate as pallet_baby_liminal;

construct_runtime!(
    pub enum TestRuntime where
        Block = MockBlock<TestRuntime>,
        NodeBlock = MockBlock<TestRuntime>,
        UncheckedExtrinsic = MockUncheckedExtrinsic<TestRuntime>,
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        BabyLiminal: pallet_baby_liminal::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

impl frame_system::Config for TestRuntime {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u128;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ();
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for TestRuntime {
    type MaxLocks = ();
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}

impl pallet_baby_liminal::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type MaximumVerificationKeyLength = ConstU32<10_000>;
    type MaximumDataLength = ConstU32<10_000>;
    type VerificationKeyDepositPerByte = ConstU64<10>;
}

pub fn new_test_ext() -> TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<TestRuntime>()
        .unwrap();

    pallet_balances::GenesisConfig::<TestRuntime> {
        balances: vec![
            (1, 1000000),
            (2, 1000000),
            (201078993247613318810609354531638512790, 1000000), // seed 41 for benches
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    TestExternalities::new(t)
}
