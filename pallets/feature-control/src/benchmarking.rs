use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
    use frame_system::RawOrigin;

    use crate::{ActiveFeatures, Call, Config, Feature, Pallet};

    #[benchmark]
    fn enable() {
        #[extrinsic_call]
        _(RawOrigin::Root, Feature::OnChainVerifier);

        assert!(ActiveFeatures::<T>::contains_key(Feature::OnChainVerifier));
    }

    #[benchmark]
    fn disable() {
        Pallet::<T>::enable(RawOrigin::Root.into(), Feature::OnChainVerifier).unwrap();

        #[extrinsic_call]
        _(RawOrigin::Root, Feature::OnChainVerifier);

        assert!(!Pallet::<T>::is_feature_enabled(Feature::OnChainVerifier));
    }

    frame_benchmarking::impl_benchmark_test_suite!(
        Pallet,
        crate::tests::new_test_ext(),
        crate::tests::TestRuntime
    );
}
