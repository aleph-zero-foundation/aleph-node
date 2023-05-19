use jf_relation::PlonkCircuit;

use crate::{
    note::{NoteGadget, NoteType, SourcedNote},
    shielder_types::{Note, Nullifier, TokenAmount, TokenId, Trapdoor},
    CircuitField, PlonkResult, PublicInput, Relation,
};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct DepositRelation {
    deposit_note: SourcedNote,
}

impl Default for DepositRelation {
    fn default() -> Self {
        Self::new(Default::default(), Default::default())
    }
}

impl DepositRelation {
    pub fn new(public: DepositPublicInput, private: DepositPrivateInput) -> Self {
        Self {
            deposit_note: SourcedNote {
                note: public.note,
                token_id: public.token_id,
                token_amount: public.token_amount,
                trapdoor: private.trapdoor,
                nullifier: private.nullifier,
                note_type: NoteType::Deposit,
            },
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct DepositPublicInput {
    pub note: Note,
    pub token_id: TokenId,
    pub token_amount: TokenAmount,
}

impl PublicInput for DepositRelation {
    fn public_input(&self) -> Vec<CircuitField> {
        self.deposit_note.public_input()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct DepositPrivateInput {
    pub trapdoor: Trapdoor,
    pub nullifier: Nullifier,
}

impl Relation for DepositRelation {
    fn generate_subcircuit(&self, circuit: &mut PlonkCircuit<CircuitField>) -> PlonkResult<()> {
        let note_var = circuit.create_note_variable(&self.deposit_note)?;
        circuit.enforce_note_preimage(note_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use jf_plonk::{
        proof_system::{PlonkKzgSnark, UniversalSNARK},
        transcript::StandardTranscript,
    };
    use jf_relation::Circuit;

    use crate::{
        deposit::{DepositPrivateInput, DepositPublicInput, DepositRelation},
        generate_srs,
        shielder_types::compute_note,
        Curve, PublicInput, Relation,
    };

    fn relation() -> DepositRelation {
        let token_id = 0;
        let token_amount = 10;
        let trapdoor = [1; 4];
        let nullifier = [2; 4];
        let note = compute_note(token_id, token_amount, trapdoor, nullifier);

        DepositRelation::new(
            DepositPublicInput {
                note,
                token_id,
                token_amount,
            },
            DepositPrivateInput {
                trapdoor,
                nullifier,
            },
        )
    }

    #[test]
    fn deposit_constraints_correctness() {
        let relation = relation();
        let circuit = DepositRelation::generate_circuit(&relation).unwrap();
        circuit
            .check_circuit_satisfiability(&relation.public_input())
            .unwrap();
    }

    #[test]
    fn deposit_constraints_incorrectness_with_wrong_note() {
        let mut relation = relation();
        relation.deposit_note.note[0] += 1;
        let circuit = DepositRelation::generate_circuit(&relation).unwrap();
        assert!(circuit
            .check_circuit_satisfiability(&relation.public_input())
            .is_err());
    }

    #[test]
    fn deposit_proving_procedure() {
        let rng = &mut jf_utils::test_rng();
        let srs = generate_srs(10_000, rng).unwrap();

        let (pk, vk) = DepositRelation::generate_keys(&srs).unwrap();

        let relation = relation();
        let proof = relation.generate_proof(&pk, rng).unwrap();

        let public_input = relation.public_input();

        PlonkKzgSnark::<Curve>::verify::<StandardTranscript>(&vk, &public_input, &proof, None)
            .unwrap();
    }
}
