use ark_crypto_primitives::SNARK;
use ark_ff::BigInteger256;
use ark_groth16::Groth16;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use liminal_ark_poseidon::hash;

use crate::{
    environment::CircuitField,
    preimage::{preimage_proving, PreimageRelationWithFullInput},
};

#[test]
fn preimage_constraints_correctness() {
    let preimage = CircuitField::from(17u64);
    let image = hash::one_to_one_hash([preimage]);
    let frontend_image: [u64; 4] = image.0 .0;

    let circuit = PreimageRelationWithFullInput::new(frontend_image, preimage.0 .0);

    let cs = ConstraintSystem::new_ref();
    circuit.generate_constraints(cs.clone()).unwrap();

    let is_satisfied = cs.is_satisfied().unwrap();
    assert!(is_satisfied);
}

#[test]
fn unsatisfied_preimage_constraints() {
    let true_preimage = CircuitField::from(17u64);
    let fake_image = hash::one_to_one_hash([CircuitField::from(19u64)]);
    let circuit = PreimageRelationWithFullInput::new(fake_image.0 .0, true_preimage.0 .0);

    let cs = ConstraintSystem::new_ref();
    circuit.generate_constraints(cs.clone()).unwrap();

    let is_satisfied = cs.is_satisfied().unwrap();

    assert!(!is_satisfied);
}

#[test]
fn preimage_proving_and_verifying() {
    let (vk, input, proof) = preimage_proving();

    let is_valid = Groth16::verify(&vk, &input, &proof).unwrap();
    assert!(is_valid);
}

#[test]
fn frontend_to_backend_conversion() {
    let frontend_preimage = 7u64;
    let backend_preimage: CircuitField = CircuitField::from(frontend_preimage);
    let expected_backend_hash: CircuitField = hash::one_to_one_hash([backend_preimage]);

    let bint = BigInteger256::new([
        6921429189085971870u64,
        65421081288123788u64,
        1703765854531614015u64,
        5826733087857826612u64,
    ]);

    let actual_backend_hash = CircuitField::new(bint);

    assert_eq!(expected_backend_hash, actual_backend_hash);
}
