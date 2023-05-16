mod relation;
#[cfg(all(test, feature = "circuit"))]
mod tests;

#[cfg(feature = "circuit")]
use {
    crate::environment::CircuitField,
    ark_bls12_381::Bls12_381,
    ark_crypto_primitives::SNARK,
    ark_ec::bls12::Bls12,
    ark_ff::Fp256,
    ark_groth16::{Groth16, Proof, VerifyingKey},
    ark_std::vec::Vec,
    liminal_ark_poseidon::hash,
};

pub use self::relation::{
    PreimageRelationWithFullInput, PreimageRelationWithPublicInput, PreimageRelationWithoutInput,
};

pub type FrontendHash = [u64; 4];
pub type FrontendPreimage = [u64; 4];

#[cfg(feature = "circuit")]
#[allow(clippy::type_complexity)]
pub fn preimage_proving() -> (
    VerifyingKey<Bls12<ark_bls12_381::Parameters>>,
    Vec<Fp256<ark_ed_on_bls12_381::FqParameters>>,
    Proof<Bls12<ark_bls12_381::Parameters>>,
) {
    let circuit_withouth_input = PreimageRelationWithoutInput::new();

    let preimage = CircuitField::from(7u64);
    let image = hash::one_to_one_hash([preimage]);
    let frontend_image: [u64; 4] = image.0 .0;

    let full_circuit = PreimageRelationWithFullInput::new(frontend_image, preimage.0 .0);

    let mut rng = ark_std::test_rng();
    let (pk, vk) =
        Groth16::<Bls12_381>::circuit_specific_setup(circuit_withouth_input, &mut rng).unwrap();

    let circuit_with_public_input = PreimageRelationWithPublicInput::new(frontend_image);
    let input = circuit_with_public_input.serialize_public_input();

    let proof = Groth16::prove(&pk, full_circuit, &mut rng).unwrap();

    (vk, input, proof)
}
