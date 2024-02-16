use halo2_proofs::{
    circuit::{Layouter, Value},
    plonk::{create_proof, keygen_pk, keygen_vk, Circuit, ConstraintSystem, Error},
    poly::kzg::{commitment::ParamsKZG, multiopen::ProverGWC},
    standard_plonk::StandardPlonk,
    transcript::{Blake2bWrite, Challenge255, TranscriptWriterBuffer},
};

use crate::snark_verifier::{
    implementation::{Curve, Fr},
    serialize_vk, verify, VerifierError,
};

const CIRCUIT_MAX_K: u32 = 5;

#[derive(Default)]
struct APlusBIsC {
    a: Fr,
    b: Fr,
}

impl Circuit<Fr> for APlusBIsC {
    type Config = <StandardPlonk as Circuit<Fr>>::Config;
    type FloorPlanner = <StandardPlonk as Circuit<Fr>>::FloorPlanner;

    fn without_witnesses(&self) -> Self {
        APlusBIsC::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        StandardPlonk::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "",
            |mut region| {
                region.assign_advice(|| "", config.a, 0, || Value::known(self.a))?;
                region.assign_fixed(|| "", config.q_a, 0, || Value::known(-Fr::one()))?;
                region.assign_advice(|| "", config.b, 0, || Value::known(self.b))?;
                region.assign_fixed(|| "", config.q_b, 0, || Value::known(-Fr::one()))?;
                Ok(())
            },
        )
    }
}

struct EncodedArgs {
    proof: Vec<u8>,
    public_input: Vec<u8>,
    vk: Vec<u8>,
}

fn setup(a: u64, b: u64, c: u64) -> EncodedArgs {
    let circuit = APlusBIsC {
        a: Fr::from(a),
        b: Fr::from(b),
    };
    let instances = vec![Fr::from(c)];

    let params = ParamsKZG::<Curve>::setup(CIRCUIT_MAX_K, ParamsKZG::<Curve>::mock_rng());
    let vk = keygen_vk(&params, &circuit).expect("vk should not fail");
    let pk = keygen_pk(&params, vk.clone(), &circuit).expect("pk should not fail");

    let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
    create_proof::<_, ProverGWC<'_, Curve>, _, _, _, _>(
        &params,
        &pk,
        &[circuit],
        &[&[&instances]],
        rand::rngs::OsRng,
        &mut transcript,
    )
    .expect("prover should not fail");

    let proof = transcript.finalize();
    let public_input = instances
        .iter()
        .flat_map(|i| i.to_bytes())
        .collect::<Vec<_>>();

    EncodedArgs {
        proof,
        public_input,
        vk: serialize_vk(vk, CIRCUIT_MAX_K),
    }
}

#[test]
fn accepts_correct_proof() {
    let EncodedArgs {
        proof,
        public_input,
        vk,
    } = setup(1, 2, 3);

    assert!(verify(&proof, &public_input, &vk).is_ok());
}

#[test]
fn rejects_incorrect_proof() {
    let EncodedArgs {
        proof,
        public_input,
        vk,
    } = setup(2, 2, 3);

    assert_eq!(
        verify(&proof, &public_input, &vk),
        Err(VerifierError::IncorrectProof)
    );
}

#[test]
fn rejects_incorrect_input() {
    let EncodedArgs {
        proof,
        public_input,
        vk,
    } = setup(1, 2, 3);

    let public_input = public_input.iter().map(|i| i + 1).collect::<Vec<_>>();

    assert_eq!(
        verify(&proof, &public_input, &vk),
        Err(VerifierError::IncorrectProof)
    );
}

#[test]
fn rejects_mismatching_input() {
    let EncodedArgs {
        proof,
        public_input,
        vk,
    } = setup(1, 2, 3);

    assert_eq!(verify(&proof, &[], &vk), Err(VerifierError::IncorrectProof));

    let public_input = [public_input.clone(), public_input].concat();

    assert_eq!(
        verify(&proof, &public_input, &vk),
        Err(VerifierError::IncorrectProof)
    );
}

#[test]
fn rejects_invalid_vk() {
    let EncodedArgs {
        proof,
        public_input,
        vk,
    } = setup(1, 2, 3);

    let vk = vk.iter().map(|i| i.saturating_add(1)).collect::<Vec<_>>();

    assert_eq!(
        verify(&proof, &public_input, &vk),
        Err(VerifierError::DeserializingVerificationKeyFailed)
    );
}

#[test]
fn rejects_invalid_public_input() {
    let EncodedArgs {
        proof,
        public_input,
        vk,
    } = setup(1, 2, 3);

    let public_input = public_input[..31].to_vec();

    assert_eq!(
        verify(&proof, &public_input, &vk),
        Err(VerifierError::DeserializingPublicInputFailed)
    );
}

#[test]
fn rejects_invalid_proof() {
    let EncodedArgs {
        proof,
        public_input,
        vk,
    } = setup(1, 2, 3);

    let proof = proof
        .iter()
        .map(|i| i.saturating_add(1))
        .skip(3)
        .collect::<Vec<_>>();

    assert_eq!(
        verify(&proof, &public_input, &vk),
        Err(VerifierError::VerificationFailed)
    );
}
