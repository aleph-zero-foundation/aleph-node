#![allow(clippy::let_unit_value)]

use frame_benchmarking::{account, benchmarks};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use sp_std::vec;

use crate::{Call, Config, Pallet};

const SEED: u32 = 41;

fn caller<T: Config>() -> RawOrigin<<T as frame_system::Config>::AccountId> {
    RawOrigin::Signed(account("caller", 0, SEED))
}

benchmarks! {
    store_key {
        let l in 1 .. T::MaximumKeyLength::get();
        let key = vec![l as u8; l as usize];
    } : _(caller::<T>(), key)

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::TestRuntime);
}
