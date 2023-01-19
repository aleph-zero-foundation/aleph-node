#![cfg(test)]

use frame_support::{
    construct_runtime, parameter_types, sp_io,
    traits::{OnFinalize, OnInitialize},
    weights::{RuntimeDbWeight, Weight},
};
use primitives::AuthorityId;
use sp_api_hidden_includes_construct_runtime::hidden_include::traits::GenesisBuild;
use sp_core::H256;
use sp_runtime::{
    impl_opaque_keys,
    testing::{Header, TestXt, UintAuthorityId},
    traits::{ConvertInto, IdentityLookup, OpaqueKeys},
};

use super::*;
use crate as pallet_aleph;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type AccountId = u64;

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Aleph: pallet_aleph::{Pallet, Storage, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
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
        frame_system::limits::BlockWeights::simple_max(Weight::from_ref_time(1024));
    pub const TestDbWeight: RuntimeDbWeight = RuntimeDbWeight {
        read: 25,
        write: 100
    };
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
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

impl pallet_balances::Config for Test {
    type Balance = u128;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
}

impl pallet_session::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = Aleph;
    type SessionHandler = <TestSessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = TestSessionKeys;
    type WeightInfo = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    RuntimeCall: From<C>,
{
    type Extrinsic = TestXt<RuntimeCall, ()>;
    type OverarchingCall = RuntimeCall;
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

impl Config for Test {
    type AuthorityId = AuthorityId;
    type RuntimeEvent = RuntimeEvent;
    type SessionInfoProvider = Session;
    type SessionManager = ();
    type NextSessionAuthorityProvider = Session;
}

pub fn to_authority(id: &u64) -> AuthorityId {
    UintAuthorityId(*id).to_public_key()
}

pub fn to_authorities(authorities: &[u64]) -> Vec<AuthorityId> {
    authorities.iter().map(to_authority).collect()
}

pub fn new_session_validators(validators: &[u64]) -> impl Iterator<Item = (&u64, AuthorityId)> {
    validators
        .iter()
        .zip(to_authorities(validators).into_iter())
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

pub(crate) fn run_session(n: u32) {
    for i in Session::current_index()..n {
        Session::on_finalize(System::block_number());
        Aleph::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let parent_hash = if System::block_number() > 1 {
            System::finalize().hash()
        } else {
            System::parent_hash()
        };

        System::initialize(
            &(System::block_number() + 1),
            &parent_hash,
            &Default::default(),
        );
        System::set_block_number((i + 1).into());
        Timestamp::set_timestamp(System::block_number() * 1000);

        System::on_initialize(System::block_number());
        Session::on_initialize(System::block_number());
        Aleph::on_initialize(System::block_number());
    }
}

pub(crate) fn initialize_session() {
    System::initialize(&1, &System::parent_hash(), &Default::default());

    System::on_initialize(System::block_number());
    Session::on_initialize(System::block_number());
    Aleph::on_initialize(System::block_number());
}
