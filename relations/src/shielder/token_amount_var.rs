use core::{borrow::Borrow, cmp::Ordering, ops::Add};

use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    boolean::Boolean,
    eq::EqGadget,
    R1CSVar,
};
use ark_relations::r1cs::{Namespace, SynthesisError};

use crate::environment::{CircuitField, FpVar};

#[derive(Clone, Debug)]
pub struct TokenAmountVar {
    value: FpVar,
}

impl TokenAmountVar {
    fn new(value: FpVar) -> Result<Self, SynthesisError> {
        // We allow token amounts to use full power of `u128`, but nothing more. Validation mustn't
        // be via `enforce_cmp_unchecked` since we have no guarantees about `value`.
        let limit = FpVar::new_constant(value.cs(), CircuitField::from(u128::MAX))?;
        value.enforce_cmp(&limit, Ordering::Less, true)?;

        Ok(Self { value })
    }
}

impl AllocVar<CircuitField, CircuitField> for TokenAmountVar {
    fn new_variable<T: Borrow<CircuitField>>(
        cs: impl Into<Namespace<CircuitField>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        TokenAmountVar::new(FpVar::new_variable(cs, f, mode)?)
    }
}

impl EqGadget<CircuitField> for TokenAmountVar {
    fn is_eq(&self, other: &Self) -> Result<Boolean<CircuitField>, SynthesisError> {
        self.value.is_eq(&other.value)
    }
}

impl Add for TokenAmountVar {
    type Output = Result<Self, SynthesisError>;

    fn add(self, rhs: Self) -> Self::Output {
        let sum = self.value + rhs.value;
        TokenAmountVar::new(sum)
    }
}

impl From<TokenAmountVar> for FpVar {
    fn from(value: TokenAmountVar) -> Self {
        value.value
    }
}
