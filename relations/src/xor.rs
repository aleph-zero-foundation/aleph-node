use std::marker::PhantomData;

use ark_ff::PrimeField;
use ark_r1cs_std::prelude::{AllocVar, EqGadget, UInt8};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, SynthesisError::AssignmentMissing,
};

use crate::{
    byte_to_bits,
    relation::{
        state::{FullInput, NoInput, OnlyPublicInput, State, WithPublicInput},
        GetPublicInput,
    },
    CircuitField,
};

/// XOR relation: a ⊕ b = c
///
/// Relation with:
///  - 1 public input    (a | `public_xoree`)
///  - 1 private witness (b | `private_xoree`)
///  - 1 constant        (c | `result`)
/// such that: a ^ b = c.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct XorRelation<S: State> {
    // ToDo: Especially for Groth16, it is better to provide public input as a field element.
    // Otherwise, we have to provide it to circuit bit by bit.
    pub public_xoree: Option<u8>,
    pub private_xoree: Option<u8>,

    pub result: u8,

    _phantom: PhantomData<S>,
}

impl XorRelation<NoInput> {
    pub fn without_input(result: u8) -> Self {
        Self {
            public_xoree: None,
            private_xoree: None,
            result,
            _phantom: PhantomData,
        }
    }
}

impl XorRelation<OnlyPublicInput> {
    pub fn with_public_input(public_xoree: u8, result: u8) -> Self {
        Self {
            public_xoree: Some(public_xoree),
            private_xoree: None,
            result,
            _phantom: PhantomData,
        }
    }
}

impl XorRelation<FullInput> {
    pub fn with_full_input(public_xoree: u8, private_xoree: u8, result: u8) -> Self {
        Self {
            public_xoree: Some(public_xoree),
            private_xoree: Some(private_xoree),
            result,
            _phantom: PhantomData,
        }
    }
}

impl<Field: PrimeField, S: State> ConstraintSynthesizer<Field> for XorRelation<S> {
    fn generate_constraints(self, cs: ConstraintSystemRef<Field>) -> Result<(), SynthesisError> {
        // TODO: migrate from u8 values to values in the finite field (see FpVar)
        let public_xoree = UInt8::new_input(ark_relations::ns!(cs, "public_xoree"), || {
            self.public_xoree.ok_or(AssignmentMissing)
        })?;
        let private_xoree = UInt8::new_witness(ark_relations::ns!(cs, "private_xoree"), || {
            self.private_xoree.ok_or(AssignmentMissing)
        })?;
        let result = UInt8::new_constant(ark_relations::ns!(cs, "result"), self.result)?;

        let xor = UInt8::xor(&public_xoree, &private_xoree)?;
        xor.enforce_equal(&result)
    }
}

impl<S: WithPublicInput> GetPublicInput<CircuitField> for XorRelation<S> {
    fn public_input(&self) -> Vec<CircuitField> {
        byte_to_bits(self.public_xoree.unwrap()).to_vec()
    }
}
