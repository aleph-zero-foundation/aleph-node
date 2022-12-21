#![cfg(test)]

use frame_election_provider_support::{data_provider, ElectionDataProvider, VoteWeight};
use frame_support::{
    construct_runtime, parameter_types, sp_io,
    traits::{ConstU32, GenesisBuild},
    weights::{RuntimeDbWeight, Weight},
    BasicExternalities, BoundedVec,
};
use primitives::{BanConfig, CommitteeSeats};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::IdentityLookup,
};
use sp_staking::{EraIndex, SessionIndex};
use sp_std::{cell::RefCell, collections::btree_set::BTreeSet};

use super::*;
use crate as pallet_elections;
use crate::traits::{
    EraInfoProvider, SessionInfoProvider, ValidatorExtractor, ValidatorRewardsHandler,
};

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
pub(crate) type Balance = u128;

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
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = TestDbWeight;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
    RuntimeCall: From<C>,
{
    type Extrinsic = TestXt<RuntimeCall, ()>;
    type OverarchingCall = RuntimeCall;
}

parameter_types! {
    pub const SessionPeriod: u32 = 5;
    pub const SessionsPerEra: u32 = 5;
}

pub struct MockProvider;

impl SessionInfoProvider<Test> for MockProvider {
    fn current_committee() -> BTreeSet<<Test as frame_system::Config>::AccountId> {
        todo!()
    }
}

impl ValidatorRewardsHandler<Test> for MockProvider {
    fn validator_totals(
        _era: EraIndex,
    ) -> Vec<(<Test as frame_system::Config>::AccountId, Balance)> {
        Default::default()
    }

    fn add_rewards(
        _rewards: impl IntoIterator<Item = (<Test as frame_system::Config>::AccountId, u32)>,
    ) {
        todo!()
    }
}

thread_local! {
    static ACTIVE_ERA: RefCell<EraIndex> = RefCell::new(Default::default());
    static CURRENT_ERA: RefCell<EraIndex> = RefCell::new(Default::default());
    static ELECTED_VALIDATORS: RefCell<BTreeMap<EraIndex, Vec<AccountId>>> = RefCell::new(Default::default());
}

pub fn with_active_era(era: EraIndex) {
    ACTIVE_ERA.with(|ae| *ae.borrow_mut() = era);
}

pub fn with_current_era(era: EraIndex) {
    CURRENT_ERA.with(|ce| *ce.borrow_mut() = era);
}

pub fn with_elected_validators(era: EraIndex, validators: Vec<AccountId>) {
    ELECTED_VALIDATORS.with(|ev| *ev.borrow_mut() = BTreeMap::from_iter([(era, validators)]));
}

impl EraInfoProvider for MockProvider {
    type AccountId = AccountId;

    fn active_era() -> Option<EraIndex> {
        Some(ACTIVE_ERA.with(|ae| *ae.borrow()))
    }

    fn current_era() -> Option<EraIndex> {
        Some(ACTIVE_ERA.with(|ae| *ae.borrow()))
    }

    fn era_start_session_index(era: EraIndex) -> Option<SessionIndex> {
        Some(era * SessionsPerEra::get())
    }

    fn sessions_per_era() -> SessionIndex {
        todo!()
    }

    fn elected_validators(era: EraIndex) -> Vec<Self::AccountId> {
        ELECTED_VALIDATORS.with(|ev| ev.borrow().get(&era).unwrap().clone())
    }
}

impl ValidatorExtractor for MockProvider {
    type AccountId = AccountId;

    fn remove_validator(_who: &AccountId) {}
}

impl Config for Test {
    type EraInfoProvider = MockProvider;
    type RuntimeEvent = RuntimeEvent;
    type DataProvider = StakingMock;
    type SessionPeriod = SessionPeriod;
    type SessionManager = ();
    type SessionInfoProvider = MockProvider;
    type ValidatorRewardsHandler = MockProvider;
    type ValidatorExtractor = MockProvider;
    type MaximumBanReasonLength = ConstU32<300>;
}

type MaxVotesPerVoter = ConstU32<1>;
type AccountIdBoundedVec = BoundedVec<AccountId, MaxVotesPerVoter>;
type Vote = (AccountId, VoteWeight, AccountIdBoundedVec);

thread_local! {
    static ELECTABLE_TARGETS: RefCell<Vec<AccountId>> = RefCell::new(Default::default());
    static ELECTING_VOTERS: RefCell<Vec<Vote>> = RefCell::new(Default::default());
}

pub fn with_electable_targets(targets: Vec<AccountId>) {
    ELECTABLE_TARGETS.with(|et| *et.borrow_mut() = targets);
}

pub fn with_electing_voters(voters: Vec<Vote>) {
    ELECTING_VOTERS.with(|ev| *ev.borrow_mut() = voters);
}

pub struct StakingMock;
impl ElectionDataProvider for StakingMock {
    type AccountId = AccountId;
    type BlockNumber = u64;
    type MaxVotesPerVoter = MaxVotesPerVoter;

    fn electable_targets(_maybe_max_len: Option<usize>) -> data_provider::Result<Vec<AccountId>> {
        ELECTABLE_TARGETS.with(|et| Ok(et.borrow().clone()))
    }

    fn electing_voters(_maybe_max_len: Option<usize>) -> data_provider::Result<Vec<Vote>> {
        ELECTING_VOTERS.with(|ev| Ok(ev.borrow().clone()))
    }

    fn desired_targets() -> data_provider::Result<u32> {
        Ok(0)
    }

    fn next_election_prediction(_now: u64) -> u64 {
        0
    }
}

pub struct TestExtBuilder {
    reserved_validators: Vec<AccountId>,
    non_reserved_validators: Vec<AccountId>,
    committee_seats: CommitteeSeats,
    committee_ban_config: BanConfig,
    storage_version: StorageVersion,
}

impl TestExtBuilder {
    pub fn new(
        reserved_validators: Vec<AccountId>,
        non_reserved_validators: Vec<AccountId>,
    ) -> Self {
        Self {
            committee_seats: CommitteeSeats {
                reserved_seats: reserved_validators.len() as u32,
                non_reserved_seats: non_reserved_validators.len() as u32,
            },
            reserved_validators,
            non_reserved_validators,
            committee_ban_config: BanConfig::default(),
            storage_version: STORAGE_VERSION,
        }
    }

    pub fn with_committee_seats(mut self, committee_seats: CommitteeSeats) -> Self {
        self.committee_seats = committee_seats;
        self
    }

    #[cfg(feature = "try-runtime")]
    pub fn with_storage_version(mut self, version: u16) -> Self {
        self.storage_version = StorageVersion::new(version);
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let validators: Vec<_> = self
            .non_reserved_validators
            .iter()
            .chain(self.reserved_validators.iter())
            .collect();

        let balances: Vec<_> = validators.iter().map(|i| (**i, 10_000_000)).collect();

        pallet_balances::GenesisConfig::<Test> { balances }
            .assimilate_storage(&mut t)
            .unwrap();

        crate::GenesisConfig::<Test> {
            non_reserved_validators: self.non_reserved_validators,
            reserved_validators: self.reserved_validators,
            committee_seats: self.committee_seats,
            committee_ban_config: self.committee_ban_config,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        BasicExternalities::execute_with_storage(&mut t, || {
            self.storage_version.put::<Pallet<Test>>()
        });

        t.into()
    }
}
