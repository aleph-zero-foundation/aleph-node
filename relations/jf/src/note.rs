use ark_ff::Zero;
use jf_primitives::circuit::rescue::RescueNativeGadget;
use jf_relation::{Circuit, PlonkCircuit, Variable};

use crate::{
    shielder_types::{convert_array, Note, Nullifier, TokenAmount, TokenId, Trapdoor},
    CircuitField, PlonkResult, PublicInput,
};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum NoteType {
    Deposit,
    Spend,
    Redeposit,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SourcedNote {
    pub note: Note,
    pub token_id: TokenId,
    pub token_amount: TokenAmount,
    pub trapdoor: Trapdoor,
    pub nullifier: Nullifier,
    pub note_type: NoteType,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct SourcedNoteVar {
    pub note_var: Variable,
    pub token_id_var: Variable,
    pub token_amount_var: Variable,
    pub trapdoor_var: Variable,
    pub nullifier_var: Variable,
}

pub trait NoteGadget {
    fn create_note_variable(&mut self, note: &SourcedNote) -> PlonkResult<SourcedNoteVar>;
    fn enforce_note_preimage(&mut self, note_var: SourcedNoteVar) -> PlonkResult<()>;
}

impl NoteGadget for PlonkCircuit<CircuitField> {
    fn create_note_variable(&mut self, note: &SourcedNote) -> PlonkResult<SourcedNoteVar> {
        let note_var = self.create_variable(convert_array(note.note))?;
        let token_id_var = self.create_variable(note.token_id.into())?;
        let token_amount_var = self.create_variable(note.token_amount.into())?;
        let nullifier_var = self.create_variable(convert_array(note.nullifier))?;
        let trapdoor_var = self.create_variable(convert_array(note.trapdoor))?;

        match note.note_type {
            NoteType::Deposit => {
                self.set_variable_public(note_var)?;
                self.set_variable_public(token_id_var)?;
                self.set_variable_public(token_amount_var)?;
            }
            NoteType::Spend => {
                self.set_variable_public(nullifier_var)?;
            }
            NoteType::Redeposit => {
                self.set_variable_public(note_var)?;
                self.set_variable_public(token_id_var)?;
            }
        }

        // Ensure that the token amount is valid.
        // todo: extract token amount limiting to at least constant, or even better to a function/type
        self.enforce_leq_constant(token_amount_var, CircuitField::from(u128::MAX))?;

        Ok(SourcedNoteVar {
            note_var,
            token_id_var,
            token_amount_var,
            nullifier_var,
            trapdoor_var,
        })
    }

    fn enforce_note_preimage(&mut self, note_var: SourcedNoteVar) -> PlonkResult<()> {
        let SourcedNoteVar {
            note_var,
            token_id_var,
            token_amount_var,
            nullifier_var,
            trapdoor_var,
        } = note_var;

        let zero_var = self.create_constant_variable(CircuitField::zero())?;

        // Check that the note is valid.
        let inputs: [usize; 6] = [
            token_id_var,
            token_amount_var,
            trapdoor_var,
            nullifier_var,
            zero_var,
            zero_var,
        ];

        let computed_note_var = RescueNativeGadget::<CircuitField>::rescue_sponge_no_padding(
            self,
            inputs.as_slice(),
            1,
        )?[0];

        self.enforce_equal(note_var, computed_note_var)?;

        Ok(())
    }
}

impl PublicInput for SourcedNote {
    fn public_input(&self) -> Vec<CircuitField> {
        match self.note_type {
            NoteType::Deposit => {
                vec![
                    convert_array(self.note),
                    self.token_id.into(),
                    self.token_amount.into(),
                ]
            }
            NoteType::Spend => {
                vec![convert_array(self.nullifier)]
            }
            NoteType::Redeposit => {
                vec![convert_array(self.note), self.token_id.into()]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use jf_relation::{Circuit, PlonkCircuit};

    use crate::{
        note::{NoteGadget, NoteType, SourcedNote},
        shielder_types::compute_note,
        CircuitField, PublicInput,
    };

    fn gen_note(note_type: NoteType) -> SourcedNote {
        let token_id = 0;
        let token_amount = 10;
        let trapdoor = [1; 4];
        let nullifier = [2; 4];
        let note = compute_note(token_id, token_amount, trapdoor, nullifier);

        SourcedNote {
            note,
            nullifier,
            token_id,
            token_amount,
            trapdoor,
            note_type,
        }
    }

    fn test_note(note_type: NoteType) {
        let mut circuit = PlonkCircuit::<CircuitField>::new_turbo_plonk();

        let note = gen_note(note_type);

        let note_var = circuit.create_note_variable(&note).unwrap();
        circuit.enforce_note_preimage(note_var).unwrap();

        let public_input = note.public_input();
        circuit.check_circuit_satisfiability(&public_input).unwrap();
    }

    #[test]
    fn spend_note() {
        test_note(NoteType::Spend)
    }

    #[test]
    fn deposit_note() {
        test_note(NoteType::Deposit)
    }

    #[test]
    fn redeposit_note() {
        test_note(NoteType::Redeposit)
    }
}
