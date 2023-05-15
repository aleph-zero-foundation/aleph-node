use ark_bls12_381::{Bls12_381, Fr};
use ark_serialize::CanonicalDeserialize;
use jf_plonk::{
    errors::PlonkError,
    proof_system::{
        structs::{Proof, VerifyingKey},
        PlonkKzgSnark, UniversalSNARK,
    },
    transcript::StandardTranscript,
};
use sp_runtime_interface::pass_by::PassByEnum;

pub type Curve = Bls12_381;
pub type CircuitField = Fr;

#[derive(Copy, Clone, Eq, PartialEq, Debug, codec::Encode, codec::Decode, PassByEnum)]
pub enum VerificationError {
    WrongProof,
    DeserializationError,
    OtherError,
}

#[sp_runtime_interface::runtime_interface]
pub trait Jellyfier {
    fn verify_proof(
        vk: Vec<u8>,
        public_input: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(), VerificationError> {
        let vk: VerifyingKey<Curve> = CanonicalDeserialize::deserialize_compressed(&*vk)
            .map_err(|_| VerificationError::DeserializationError)?;
        let public_input: Vec<CircuitField> =
            CanonicalDeserialize::deserialize_compressed(&*public_input)
                .map_err(|_| VerificationError::DeserializationError)?;
        let proof: Proof<Curve> = CanonicalDeserialize::deserialize_compressed(&*proof)
            .map_err(|_| VerificationError::DeserializationError)?;

        PlonkKzgSnark::verify::<StandardTranscript>(&vk, &public_input, &proof, None).map_err(|e| {
            match e {
                PlonkError::WrongProof => VerificationError::WrongProof,
                _ => VerificationError::OtherError,
            }
        })
    }
}

#[cfg(test)]
mod test {
    use ark_serialize::{CanonicalSerialize, Compress};
    use jf_plonk::{
        proof_system::{
            structs::{Proof, VerifyingKey},
            PlonkKzgSnark, UniversalSNARK,
        },
        transcript::StandardTranscript,
    };
    use jf_relation::{Arithmetization, Circuit, PlonkCircuit};

    use crate::{CircuitField, Curve, VerificationError};

    const A: u32 = 1; // public input
    const B: u32 = 2; // private input
    const C: u32 = 3; // constant

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

    fn setup() -> (VerifyingKey<Curve>, Proof<Curve>) {
        let circuit = generate_circuit();

        let rng = &mut jf_utils::test_rng();
        let srs =
            PlonkKzgSnark::<Curve>::universal_setup_for_testing(circuit.srs_size().unwrap(), rng)
                .unwrap();

        let (pk, vk) = PlonkKzgSnark::<Curve>::preprocess(&srs, &circuit).unwrap();
        let proof =
            PlonkKzgSnark::<Curve>::prove::<_, _, StandardTranscript>(rng, &circuit, &pk, None)
                .unwrap();

        (vk, proof)
    }

    fn serialize<T: CanonicalSerialize>(t: &T) -> Vec<u8> {
        let mut bytes = vec![0; t.serialized_size(Compress::Yes)];
        t.serialize_compressed(&mut bytes[..]).unwrap();
        bytes.to_vec()
    }

    fn do_verification(public_input: CircuitField) -> Result<(), VerificationError> {
        let (vk, proof) = setup();
        let vk = serialize(&vk);
        let proof = serialize(&proof);
        let public_input = serialize(&vec![public_input]);

        // We cannot use `Jellyfier` trait directly, as it is transformed by substrate macro into
        // a module (with private trait `Jellyfier` and public function `verify_proof`).
        crate::jellyfier::verify_proof(vk, public_input, proof)
    }

    #[test]
    fn verify_proof() {
        assert!(do_verification(A.into()).is_ok());
        assert!(matches!(
            do_verification((A + 1).into()),
            Err(VerificationError::WrongProof)
        ));
    }
}
