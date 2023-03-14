use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    eq::EqGadget,
};
use ark_relations::{
    ns,
    r1cs::{ConstraintSystemRef, SynthesisError},
};
use paste::paste;

use crate::{
    environment::FpVar, BackendNote, BackendNullifier, BackendTokenAmount, BackendTokenId,
    BackendTrapdoor, CircuitField,
};

#[derive(Clone, Debug)]
pub struct NoteVar {
    pub token_id: FpVar,
    pub token_amount: FpVar,
    pub trapdoor: FpVar,
    pub nullifier: FpVar,
    pub note: FpVar,
}

#[derive(Clone, Debug)]
pub struct NoteVarBuilder<
    const TOKEN_ID_SET: bool,
    const TOKEN_AMOUNT_SET: bool,
    const TRAPDOOR_SET: bool,
    const NULLIFIER_SET: bool,
    const NOTE_SET: bool,
> {
    token_id: Option<FpVar>,
    token_amount: Option<FpVar>,
    trapdoor: Option<FpVar>,
    nullifier: Option<FpVar>,
    note: Option<FpVar>,
    cs: ConstraintSystemRef<CircuitField>,
}

impl NoteVarBuilder<false, false, false, false, false> {
    pub fn new(cs: ConstraintSystemRef<CircuitField>) -> Self {
        NoteVarBuilder {
            token_id: None,
            token_amount: None,
            trapdoor: None,
            nullifier: None,
            note: None,
            cs,
        }
    }
}

type Result<T> = core::result::Result<T, SynthesisError>;

macro_rules! impl_with_plain_arg {
    ($item: ident, $item_type: ty, $target_type: ty) => {
        paste! {
            pub fn [<with_ $item>] (self, $item: Result<&$item_type>, mode: AllocationMode) -> Result<$target_type> {
                let $item = FpVar::new_variable(ns!(self.cs, stringify!($item)), || $item, mode)?;
                Ok(self. [<with_ $item _var>]($item))
            }
        }
    };
}

macro_rules! impl_with_var_arg {
    ($item: ident, $target_type: ty) => {
        paste! {
            pub fn [<with_ $item _var>] (self, $item: FpVar) -> $target_type {
                let mut note: $target_type = NoteVarBuilder {
                    token_id: self.token_id,
                    token_amount: self.token_amount,
                    trapdoor: self.trapdoor,
                    nullifier: self.nullifier,
                    note: self.note,
                    cs: self.cs,
                };
                note.$item = Some($item);
                note
            }
        }
    };
}

macro_rules! impl_builder {
    ($in_type: ty, $out_type: ty, $item: ident, $item_type: ty) => {
        impl<const _1: bool, const _2: bool, const _3: bool, const _4: bool> $in_type {
            impl_with_plain_arg!($item, $item_type, $out_type);
            impl_with_var_arg!($item, $out_type);
        }
    };
}

impl_builder!(
    NoteVarBuilder<false, _1, _2, _3, _4>,
    NoteVarBuilder<true, _1, _2, _3, _4>,
    token_id, BackendTokenId
);
impl_builder!(
    NoteVarBuilder<_1, false, _2, _3, _4>,
    NoteVarBuilder<_1, true, _2, _3, _4>,
    token_amount, BackendTokenAmount
);
impl_builder!(
    NoteVarBuilder<_1, _2, false, _3, _4>,
    NoteVarBuilder<_1, _2, true, _3, _4>,
    trapdoor, BackendTrapdoor
);
impl_builder!(
    NoteVarBuilder<_1, _2, _3, false, _4>,
    NoteVarBuilder<_1, _2, _3, true, _4>,
    nullifier, BackendNullifier
);
impl_builder!(
    NoteVarBuilder<_1, _2, _3, _4, false>,
    NoteVarBuilder<_1, _2, _3, _4, true>,
    note, BackendNote
);

impl NoteVarBuilder<true, true, true, true, true> {
    /// Verify that `note` is indeed the result of hashing `(token_id, token_amount, trapdoor,
    /// nullifier)`. If so, return `NoteVar` holding all components.
    pub fn build(self) -> Result<NoteVar> {
        let note = NoteVar {
            token_id: self.token_id.unwrap(),
            token_amount: self.token_amount.unwrap(),
            trapdoor: self.trapdoor.unwrap(),
            nullifier: self.nullifier.unwrap(),
            note: self.note.unwrap(),
        };

        let hash = liminal_ark_poseidon::circuit::four_to_one_hash(
            self.cs,
            [
                note.token_id.clone(),
                note.token_amount.clone(),
                note.trapdoor.clone(),
                note.nullifier.clone(),
            ],
        )?;

        hash.enforce_equal(&note.note)?;

        Ok(note)
    }
}
