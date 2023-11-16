#![cfg(test)]

use frame_support::{
    construct_runtime,
    pallet_prelude::ConstU32,
    parameter_types,
    traits::{EstimateNextSessionRotation, OnFinalize, OnInitialize},
    weights::{RuntimeDbWeight, Weight},
};
use frame_system::pallet_prelude::BlockNumberFor;
use primitives::{AuthorityId, SessionInfoProvider};
use sp_core::H256;
use sp_runtime::{
    impl_opaque_keys,
    testing::{TestXt, UintAuthorityId},
    traits::{ConvertInto, IdentityLookup, OpaqueKeys},
    BuildStorage,
};

use super::*;
use crate as pallet_aleph;

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type AccountId = u64;

construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Aleph: pallet_aleph,
        Session: pallet_session,
        Timestamp: pallet_timestamp,
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
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
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
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
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
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
}

pub struct SessionInfoImpl;
impl SessionInfoProvider<BlockNumberFor<Test>> for SessionInfoImpl {
    fn current_session() -> SessionIndex {
        pallet_session::CurrentIndex::<Test>::get()
    }
    fn next_session_block_number(
        current_block: BlockNumberFor<Test>,
    ) -> Option<BlockNumberFor<Test>> {
        <Test as pallet_session::Config>::NextSessionRotation::estimate_next_session_rotation(
            current_block,
        )
        .0
    }
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
    type SessionInfoProvider = SessionInfoImpl;
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
    let mut t = <frame_system::GenesisConfig<Test> as BuildStorage>::build_storage(
        &frame_system::GenesisConfig::default(),
    )
    .expect("Storage should be build.");

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
