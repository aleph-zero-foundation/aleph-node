use ark_bls12_381::Bls12_381;
use ark_crypto_primitives::SNARK;
use ark_groth16::Groth16;
//
// cargo bench
//
use criterion::{criterion_group, criterion_main, Criterion};
use liminal_ark_poseidon::hash;
use liminal_ark_relations::environment::CircuitField;
pub use liminal_ark_relations::preimage::{
    PreimageRelationWithFullInput, PreimageRelationWithPublicInput, PreimageRelationWithoutInput,
};

fn preimage(c: &mut Criterion) {
    let circuit_withouth_input = PreimageRelationWithoutInput::new();

    let preimage = CircuitField::from(7u64);
    let image = hash::one_to_one_hash([preimage]);
    let frontend_image: [u64; 4] = image.0 .0;

    let mut rng = ark_std::test_rng();
    let (pk, _) =
        Groth16::<Bls12_381>::circuit_specific_setup(circuit_withouth_input, &mut rng).unwrap();

    c.bench_function("preimage", |f| {
        f.iter(|| {
            let full_circuit = PreimageRelationWithFullInput::new(frontend_image, preimage.0 .0);
            let _ = Groth16::prove(&pk, full_circuit, &mut rng).unwrap();
        })
    });
}

criterion_group!(benches, preimage);
criterion_main!(benches);
