use core::{borrow::Borrow, ops::Index};

use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    boolean::Boolean,
};
use ark_relations::r1cs::{Namespace, SynthesisError};
use ark_std::{vec, vec::Vec};
#[cfg(feature = "std")]
use {
    ark_r1cs_std::R1CSVar,
    std::fmt::{Display, Formatter},
};

use crate::environment::CircuitField;

#[derive(Clone, Debug)]
pub struct PathShapeVar {
    shape: Vec<Boolean<CircuitField>>,
}

#[cfg(feature = "std")]
impl Display for PathShapeVar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}",
            self.shape
                .iter()
                .map(|b| b.value().map(|boo| if boo { "left" } else { "right" }))
                .collect::<Vec<_>>()
        )
    }
}

impl PathShapeVar {
    pub(super) fn len(&self) -> usize {
        self.shape.len()
    }
}

impl Index<usize> for PathShapeVar {
    type Output = Boolean<CircuitField>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.shape[index]
    }
}

impl AllocVar<(u8, Result<u64, SynthesisError>), CircuitField> for PathShapeVar {
    fn new_variable<T: Borrow<(u8, Result<u64, SynthesisError>)>>(
        cs: impl Into<Namespace<CircuitField>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let ns = cs.into();
        let cs = ns.cs();

        let mut shape = vec![];

        let (path_length, maybe_leaf_index) = *f()?.borrow();

        for i in 0..path_length {
            shape.push(Boolean::new_variable(
                cs.clone(),
                || {
                    let current_index = maybe_leaf_index? / (1 << i);
                    Ok(current_index & 1 != 1 || current_index == 1)
                },
                mode,
            )?);
        }

        Ok(Self { shape })
    }
}
