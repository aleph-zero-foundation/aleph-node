use frame_benchmarking::{account, benchmarks, vec};
use frame_support::{traits::Get, BoundedVec};
use frame_system::RawOrigin;

use crate::{
    benchmarking::{linear_input, linear_proof, linear_vk, xor_input, xor_proof, xor_vk},
    *,
};

const SEED: u32 = 41;

benchmarks! {

    store_key {
        let caller = account("caller", 0, SEED);
        let identifier = [0u8; 4];
        let l in 1 .. T::MaximumVerificationKeyLength::get();
        let key = vec![0u8; l as usize];
    } : _(RawOrigin::Signed(caller), identifier, key.clone())

    verify_xor {
        let caller = account("caller", 0, SEED);

        let key = xor_vk().to_vec();
        let proof = xor_proof().to_vec();
        let input = xor_input().to_vec();

        let identifier = [0u8; 4];
        let _ = VerificationKeys::<T>::insert(
            identifier.clone(),
            BoundedVec::try_from(key).unwrap()
        );

    } : verify(RawOrigin::Signed(caller), identifier, proof, input, ProvingSystem::Groth16)

    verify_linear_equation {
        let caller = account("caller", 0, SEED);

        let key = linear_vk().to_vec();
        let proof = linear_proof().to_vec();
        let input = linear_input().to_vec();

        let identifier = [0u8; 4];
        let _ = VerificationKeys::<T>::insert(
            identifier.clone(),
            BoundedVec::try_from(key).unwrap()
        );

    } : verify(RawOrigin::Signed(caller), identifier, proof, input, ProvingSystem::Groth16)

}
