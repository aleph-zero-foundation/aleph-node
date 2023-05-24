use ark_serialize::{CanonicalSerialize, Compress};
use jf_plonk::{
    proof_system::{PlonkKzgSnark, UniversalSNARK},
    transcript::StandardTranscript,
};
use jf_relation::{Arithmetization, Circuit, PlonkCircuit};

use crate::{CircuitField, Curve};

const A: u32 = 1; // public input
const B: u32 = 2; // private input
const C: u32 = 3; // constant

pub struct Artifacts {
    pub vk: Vec<u8>,
    pub proof: Vec<u8>,
    pub input: Vec<u8>,
}

fn serialize<T: CanonicalSerialize>(t: &T) -> Vec<u8> {
    let mut bytes = vec![0; t.serialized_size(Compress::Yes)];
    t.serialize_compressed(&mut bytes[..]).unwrap();
    bytes.to_vec()
}

pub fn get_artifacts() -> Artifacts {
    _get_artifacts(generate_circuit())
}

fn _get_artifacts(circuit: PlonkCircuit<CircuitField>) -> Artifacts {
    let rng = &mut jf_utils::test_rng();
    let srs = PlonkKzgSnark::<Curve>::universal_setup_for_testing(circuit.srs_size().unwrap(), rng)
        .unwrap();

    let (pk, vk) = PlonkKzgSnark::<Curve>::preprocess(&srs, &circuit).unwrap();
    let proof = PlonkKzgSnark::<Curve>::prove::<_, _, StandardTranscript>(rng, &circuit, &pk, None)
        .unwrap();

    Artifacts {
        vk: serialize(&vk),
        proof: serialize(&proof),
        input: serialize(&vec![CircuitField::from(A)]),
    }
}

pub fn get_invalid_input() -> Vec<u8> {
    serialize(&vec![CircuitField::from(A), CircuitField::from(A)])
}

pub fn get_incorrect_proof() -> Vec<u8> {
    let mut circuit = PlonkCircuit::<CircuitField>::new_turbo_plonk();
    circuit.add_gate(0, 0, 0).unwrap(); // there must be at least one gate
    circuit.finalize_for_arithmetization().unwrap();
    _get_artifacts(circuit).proof
}

fn generate_circuit() -> PlonkCircuit<CircuitField> {
    let mut circuit = PlonkCircuit::<CircuitField>::new_turbo_plonk();

    let a_var = circuit.create_public_variable(A.into()).unwrap();
    let b_var = circuit.create_variable(B.into()).unwrap();
    let c_var = circuit.create_constant_variable(C.into()).unwrap();

    circuit.add_gate(a_var, b_var, c_var).unwrap();

    assert!(circuit.check_circuit_satisfiability(&[A.into()]).is_ok());

    circuit.finalize_for_arithmetization().unwrap();

    circuit
}
