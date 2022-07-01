#![cfg(test)]

use frame_election_provider_support::{data_provider, ElectionDataProvider, VoteWeight};
use frame_support::{
    construct_runtime, parameter_types, sp_io, traits::GenesisBuild, weights::RuntimeDbWeight,
    BoundedVec,
};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::IdentityLookup,
};
use sp_staking::{EraIndex, SessionIndex};
use sp_std::collections::btree_set::BTreeSet;

use super::*;
use crate as pallet_elections;
use crate::traits::{EraInfoProvider, SessionInfoProvider, ValidatorRewardsHandler};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Elections: pallet_elections::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

pub(crate) type AccountId = u64;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
    pub const TestDbWeight: RuntimeDbWeight = RuntimeDbWeight {
        read: 25,
        write: 100
    };
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

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    Call: From<C>,
{
    type Extrinsic = TestXt<Call, ()>;
    type OverarchingCall = Call;
}

parameter_types! {
    pub const SessionPeriod: u32 = 5;
}

pub struct MockProvider;

impl SessionInfoProvider<Test> for MockProvider {
    fn current_committee() -> BTreeSet<<Test as frame_system::Config>::AccountId> {
        todo!()
    }
}

impl ValidatorRewardsHandler<Test> for MockProvider {
    fn validator_totals(_era: EraIndex) -> Vec<(<Test as frame_system::Config>::AccountId, u128)> {
        todo!()
    }

    fn add_rewards(
        _rewards: impl IntoIterator<Item = (<Test as frame_system::Config>::AccountId, u32)>,
    ) {
        todo!()
    }
}

impl EraInfoProvider for MockProvider {
    fn active_era() -> Option<EraIndex> {
        todo!()
    }

    fn era_start_session_index(_era: EraIndex) -> Option<SessionIndex> {
        todo!()
    }

    fn sessions_per_era() -> SessionIndex {
        todo!()
    }
}

impl Config for Test {
    type EraInfoProvider = MockProvider;
    type Event = Event;
    type DataProvider = StakingMock;
    type SessionPeriod = SessionPeriod;
    type SessionManager = ();
    type SessionInfoProvider = MockProvider;
    type ValidatorRewardsHandler = MockProvider;
}

type AccountIdBoundedVec = BoundedVec<AccountId, ()>;

pub struct StakingMock;
impl ElectionDataProvider for StakingMock {
    type AccountId = AccountId;
    type BlockNumber = u64;
    type MaxVotesPerVoter = ();

    fn electable_targets(_maybe_max_len: Option<usize>) -> data_provider::Result<Vec<AccountId>> {
        Ok(Vec::new())
    }

    fn electing_voters(
        _maybe_max_len: Option<usize>,
    ) -> data_provider::Result<Vec<(AccountId, VoteWeight, AccountIdBoundedVec)>> {
        Ok(Vec::new())
    }

    fn desired_targets() -> data_provider::Result<u32> {
        Ok(0)
    }

    fn next_election_prediction(_now: u64) -> u64 {
        0
    }
}

pub fn new_test_ext(
    reserved_validators: Vec<AccountId>,
    non_reserved_validators: Vec<AccountId>,
) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let validators: Vec<_> = non_reserved_validators
        .iter()
        .chain(reserved_validators.iter())
        .collect();

    let balances: Vec<_> = (0..validators.len())
        .map(|i| (i as u64, 10_000_000))
        .collect();

    pallet_balances::GenesisConfig::<Test> { balances }
        .assimilate_storage(&mut t)
        .unwrap();

    let committee_size = validators.len() as u32;
    crate::GenesisConfig::<Test> {
        non_reserved_validators,
        committee_size,
        reserved_validators,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
