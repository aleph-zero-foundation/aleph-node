use ark_ff::PrimeField;
use ark_r1cs_std::{
    prelude::{AllocVar, EqGadget},
    uint32::UInt32,
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use crate::{relation::GetPublicInput, CircuitField};

/// Linear equation relation: a*x + b = y
///
/// Relation with:
///  - 1 private witness (x)
///  - 3 constants        (a, b, y)
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct LinearEquationRelation {
    /// constant (a slope)
    pub a: u32,
    /// private witness
    pub x: u32,
    /// constant(an intercept)
    pub b: u32,
    /// constant
    pub y: u32,
}

impl LinearEquationRelation {
    pub fn new(a: u32, x: u32, b: u32, y: u32) -> Self {
        Self { a, x, b, y }
    }
}

impl<Field: PrimeField> ConstraintSynthesizer<Field> for LinearEquationRelation {
    fn generate_constraints(self, cs: ConstraintSystemRef<Field>) -> Result<(), SynthesisError> {
        // TODO: migrate from real values to values in the finite field (see FpVar)
        // Watch out for overflows!!!
        let x = UInt32::new_witness(ark_relations::ns!(cs, "x"), || Ok(&self.x))?;
        let b = UInt32::new_constant(ark_relations::ns!(cs, "b"), self.b)?;
        let y = UInt32::new_constant(ark_relations::ns!(cs, "y"), self.y)?;

        let mut left = std::iter::repeat(x)
            .take(self.a as usize)
            .collect::<Vec<UInt32<Field>>>();

        left.push(b);

        UInt32::addmany(&left)?.enforce_equal(&y)
    }
}

impl GetPublicInput<CircuitField> for LinearEquationRelation {
    fn public_input(&self) -> Vec<CircuitField> {
        todo!()
    }
}
