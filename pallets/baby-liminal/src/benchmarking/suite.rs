#![allow(clippy::let_unit_value)]

use frame_benchmarking::{account, benchmarks, vec, Vec};
use frame_support::{traits::Get, BoundedVec};
use frame_system::RawOrigin;
use primitives::host_functions::poseidon;

use crate::{
    benchmarking::import::Artifacts, get_artifacts, BalanceOf, Call, Config, Pallet,
    ProvingSystem::*, VerificationKeyDeposits, VerificationKeyIdentifier, VerificationKeyOwners,
    VerificationKeys,
};

const SEED: u32 = 41;
const IDENTIFIER: VerificationKeyIdentifier = [0; 8];

fn caller<T: Config>() -> RawOrigin<<T as frame_system::Config>::AccountId> {
    RawOrigin::Signed(account("caller", 0, SEED))
}

fn insert_key<T: Config>(key: Vec<u8>) {
    let owner: T::AccountId = account("caller", 0, SEED);
    let deposit = BalanceOf::<T>::from(0u32);
    VerificationKeys::<T>::insert(IDENTIFIER, BoundedVec::try_from(key).unwrap());
    VerificationKeyOwners::<T>::insert(IDENTIFIER, &owner);
    VerificationKeyDeposits::<T>::insert((&owner, IDENTIFIER), deposit);
}

fn gen_poseidon_host_input(x: u32) -> (u64, u64, u64, u64) {
    (x as u64, 0, 0, 0)
}

benchmarks! {

    store_key {
        let l in 1 .. T::MaximumVerificationKeyLength::get();
        let key = vec![0u8; l as usize];
    } : _(caller::<T>(), IDENTIFIER, key)

    overwrite_equal_key {
        let l in 1 .. T::MaximumVerificationKeyLength::get();
        let key = vec![0u8; l as usize];
        let _ = insert_key::<T>(key.clone ());
    } : overwrite_key(caller::<T>(), IDENTIFIER, key)

    overwrite_key {
        let l in 1 .. T::MaximumVerificationKeyLength::get() - 1;
        let _ = insert_key::<T>(vec![0u8; l as usize]);
        let longer_key = vec![0u8; (l + 1) as usize];
    } : overwrite_key(caller::<T>(), IDENTIFIER, longer_key)

    delete_key {
        let l in 1 .. T::MaximumVerificationKeyLength::get();
        let key = vec![0u8; l as usize];
        let _ = insert_key::<T>(key);
    } : _(caller::<T>(), IDENTIFIER)

    // Groth16 benchmarks

    verify_groth16_xor {
        let Artifacts { key, proof, input } = get_artifacts!(Groth16, Xor);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Groth16)

    verify_groth16_linear_equation {
        let Artifacts { key, proof, input } = get_artifacts!(Groth16, LinearEquation);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Groth16)

    verify_groth16_merkle_tree_8 {
        let Artifacts { key, proof, input } = get_artifacts!(Groth16, MerkleTree8);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Groth16)

    verify_groth16_merkle_tree_64 {
        let Artifacts { key, proof, input } = get_artifacts!(Groth16, MerkleTree64);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Groth16)

    verify_groth16_merkle_tree_1024 {
        let Artifacts { key, proof, input } = get_artifacts!(Groth16, MerkleTree1024);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Groth16)

    // GM17 benchmarks

    verify_gm17_xor {
        let Artifacts { key, proof, input } = get_artifacts!(Gm17, Xor);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Gm17)

    verify_gm17_linear_equation {
        let Artifacts { key, proof, input } = get_artifacts!(Gm17, LinearEquation);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Gm17)

    verify_gm17_merkle_tree_8 {
        let Artifacts { key, proof, input } = get_artifacts!(Gm17, MerkleTree8);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Gm17)

    verify_gm17_merkle_tree_64 {
        let Artifacts { key, proof, input } = get_artifacts!(Gm17, MerkleTree64);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Gm17)

    verify_gm17_merkle_tree_1024 {
        let Artifacts { key, proof, input } = get_artifacts!(Gm17, MerkleTree1024);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Gm17)

    // Marlin benchmarks

    verify_marlin_xor {
        let Artifacts { key, proof, input } = get_artifacts!(Marlin, Xor);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Marlin)

    verify_marlin_linear_equation {
        let Artifacts { key, proof, input } = get_artifacts!(Marlin, LinearEquation);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Marlin)

    verify_marlin_merkle_tree_8 {
        let Artifacts { key, proof, input } = get_artifacts!(Marlin, MerkleTree8);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Marlin)

    verify_marlin_merkle_tree_64 {
        let Artifacts { key, proof, input } = get_artifacts!(Marlin, MerkleTree64);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Marlin)

    verify_marlin_merkle_tree_1024 {
        let Artifacts { key, proof, input } = get_artifacts!(Marlin, MerkleTree1024);
        let _ = insert_key::<T>(key);
    } : verify(caller::<T>(), IDENTIFIER, proof, input, Marlin)

    // Partial `verify` execution

    verify_data_too_long {
        // Excess. Unfortunately, anything like
        // `let e in (T::MaximumDataLength::get() + 1) .. (T::MaximumDataLength::get() * 1_000)`
        // doesn't compile.
        let e in 1 .. T::MaximumDataLength::get() * 1_000;
        let proof = vec![255u8; (T::MaximumDataLength::get() + e) as usize];
        let Artifacts { key, proof: _proof, input } = get_artifacts!(Groth16, MerkleTree1024);
    } : {
        assert!(
            Pallet::<T>::verify(caller::<T>().into(), IDENTIFIER, proof, input, Groth16).is_err()
        )
    }

    // It shouldn't matter whether deserializing of proof fails, but for input it succeeds, or the
    // other way round. The only thing that is important is that we don't read storage nor run
    // verification procedure.
    verify_data_deserializing_fails {
        let l in 1 .. T::MaximumDataLength::get();
        let proof = vec![255u8; l as usize];
        // System shouldn't have any serious impact on deserializing - the data is just some
        // elements from the field.
        let Artifacts { key, proof: _proof, input } = get_artifacts!(Groth16, MerkleTree1024);
    } : {
        assert!(
            Pallet::<T>::verify(caller::<T>().into(), IDENTIFIER, proof, input, Groth16).is_err()
        )
    }

    verify_key_deserializing_fails {
        let l in 1 .. T::MaximumVerificationKeyLength::get();
        let _ = insert_key::<T>(vec![255u8; l as usize]);

        // System shouldn't have any serious impact on deserializing - the data is just some
        // elements from the field.
        let Artifacts { key, proof, input } = get_artifacts!(Groth16, MerkleTree1024);
    } : {
        assert!(
            Pallet::<T>::verify(caller::<T>().into(), IDENTIFIER, proof, input, Groth16).is_err()
        )
    }

    // Cryptography

    poseidon_one_to_one_wasm {
        let x in 0 .. u32::MAX;
    } : {
        liminal_ark_poseidon::hash::one_to_one_hash([(x as u64).into()]);
    }

    poseidon_two_to_one_wasm {
        let x in 0 .. u32::MAX;
        let y in 0 .. u32::MAX;
    } : {
        liminal_ark_poseidon::hash::two_to_one_hash([(x as u64).into(), (y as u64).into()]);
    }

    poseidon_four_to_one_wasm {
        let x in 0 .. u32::MAX;
        let y in 0 .. u32::MAX;
        let w in 0 .. u32::MAX;
        let z in 0 .. u32::MAX;
    } : {
        liminal_ark_poseidon::hash::four_to_one_hash([(x as u64).into(), (y as u64).into(), (w as u64).into(), (z as u64).into()]);
    }

    poseidon_one_to_one_host{
        let x in 0 .. u32::MAX;
    } : {
        poseidon::one_to_one_hash(gen_poseidon_host_input(x));
    }

    poseidon_two_to_one_host{
        let x in 0 .. u32::MAX;
        let y in 0 .. u32::MAX;
    } : {
        poseidon::two_to_one_hash(gen_poseidon_host_input(x), gen_poseidon_host_input(y));
    }

    poseidon_four_to_one_host{
        let x in 0 .. u32::MAX;
        let y in 0 .. u32::MAX;
        let w in 0 .. u32::MAX;
        let z in 0 .. u32::MAX;
    } : {
        poseidon::four_to_one_hash(gen_poseidon_host_input(x), gen_poseidon_host_input(y), gen_poseidon_host_input(w), gen_poseidon_host_input(z));
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::TestRuntime);
}
