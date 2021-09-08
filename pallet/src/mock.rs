#![cfg(test)]

use super::*;
use crate as pallet_aleph;

use sp_core::H256;

use frame_support::{
    construct_runtime, parameter_types, sp_io,
    traits::{OnFinalize, OnInitialize},
};
use primitives::AuthorityId;
use sp_runtime::{
    impl_opaque_keys,
    testing::{Header, TestXt, UintAuthorityId},
    traits::{ConvertInto, IdentityLookup, OpaqueKeys},
    Perbill,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;

// Based on grandpa mock

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        Aleph: pallet_aleph::{Pallet, Call, Config<T>, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
    }
);

impl_opaque_keys! {
    pub struct TestSessionKeys {
        pub aleph: super::Pallet<Test>,
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}

parameter_types! {
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = u128;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
}

impl pallet_session::Config for Test {
    type Event = Event;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = pallet_aleph::AlephSessionManager<Self>;
    type SessionHandler = <TestSessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = TestSessionKeys;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    Call: From<C>,
{
    type Extrinsic = TestXt<Call, ()>;
    type OverarchingCall = Call;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 3;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {}

impl Config for Test {
    type AuthorityId = AuthorityId;
    type Event = Event;
}

pub fn to_authorities(authorities: &[u64]) -> Vec<AuthorityId> {
    authorities
        .iter()
        .map(|id| UintAuthorityId(*id).to_public_key::<AuthorityId>())
        .collect()
}

pub fn new_test_ext(authorities: &[(u64, u64)]) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let balances: Vec<_> = (0..authorities.len())
        .map(|i| (i as u64, 10_000_000))
        .collect();

    pallet_balances::GenesisConfig::<Test> { balances }
        .assimilate_storage(&mut t)
        .unwrap();

    let session_keys: Vec<_> = authorities
        .iter()
        .map(|(id, weight)| (UintAuthorityId(*id).to_public_key::<AuthorityId>(), weight))
        .enumerate()
        .map(|(i, (k, _))| (i as u64, i as u64, TestSessionKeys { aleph: k }))
        .collect();

    pallet_session::GenesisConfig::<Test> { keys: session_keys }
        .assimilate_storage(&mut t)
        .unwrap();

    t.into()
}

pub(crate) fn run_session(n: u64) {
    while System::block_number() < n {
        Session::on_finalize(System::block_number());
        Aleph::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        System::initialize(
            &(System::block_number() + 1),
            &System::parent_hash(),
            &Default::default(),
            Default::default(),
        );

        System::on_initialize(System::block_number());
        Session::on_initialize(System::block_number());
        Aleph::on_initialize(System::block_number());
    }
}

pub(crate) fn initialize_session() {
    System::initialize(
        &1,
        &System::parent_hash(),
        &Default::default(),
        Default::default(),
    );

    System::on_initialize(System::block_number());
    Session::on_initialize(System::block_number());
    Aleph::on_initialize(System::block_number());
}
