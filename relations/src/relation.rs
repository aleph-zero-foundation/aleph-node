use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;

pub trait GetPublicInput<CircuitField: PrimeField + CanonicalSerialize> {
    fn public_input(&self) -> Vec<CircuitField> {
        vec![]
    }
}
