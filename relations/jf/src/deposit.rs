use ark_ff::Zero;
use jf_primitives::circuit::rescue::RescueNativeGadget;
use jf_relation::{Circuit, PlonkCircuit};

use crate::{
    shielder_types::{convert_hash, Note, Nullifier, TokenAmount, TokenId, Trapdoor},
    CircuitField, Marshall, PlonkResult, Relation,
};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct DepositRelation {
    public: DepositPublicInput,
    private: DepositPrivateInput,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct DepositPublicInput {
    pub note: Note,
    pub token_id: TokenId,
    pub token_amount: TokenAmount,
}

impl Marshall for DepositPublicInput {
    fn marshall(&self) -> Vec<CircuitField> {
        vec![
            convert_hash(self.note),
            self.token_id.into(),
            self.token_amount.into(),
        ]
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct DepositPrivateInput {
    pub trapdoor: Trapdoor,
    pub nullifier: Nullifier,
}

impl Relation for DepositRelation {
    type PublicInput = DepositPublicInput;
    type PrivateInput = DepositPrivateInput;

    fn new(public_input: Self::PublicInput, private_input: Self::PrivateInput) -> Self {
        Self {
            public: public_input,
            private: private_input,
        }
    }

    fn generate_subcircuit(&self, circuit: &mut PlonkCircuit<CircuitField>) -> PlonkResult<()> {
        // Register public inputs.
        let note_var = circuit.create_public_variable(convert_hash(self.public.note))?;
        let token_id_var = circuit.create_public_variable(self.public.token_id.into())?;
        let token_amount_var = circuit.create_public_variable(self.public.token_amount.into())?;

        // Register private inputs.
        let trapdoor_var = circuit.create_variable(convert_hash(self.private.trapdoor))?;
        let nullifier_var = circuit.create_variable(convert_hash(self.private.nullifier))?;

        // Ensure that the token amount is valid.
        // todo: extract token amount limiting to at least constant, or even better to a function/type
        circuit.enforce_leq_constant(token_amount_var, CircuitField::from(u128::MAX))?;

        let zero_var = circuit.create_constant_variable(CircuitField::zero())?;

        // Check that the note is valid.
        // todo: move to a common place
        let inputs: [usize; 6] = [
            token_id_var,
            token_amount_var,
            trapdoor_var,
            nullifier_var,
            zero_var,
            zero_var,
        ];
        let computed_note_var = RescueNativeGadget::<CircuitField>::rescue_sponge_no_padding(
            circuit,
            inputs.as_slice(),
            1,
        )?[0];

        circuit.enforce_equal(note_var, computed_note_var)?;

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
        Curve, Marshall, Relation,
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
            .check_circuit_satisfiability(&relation.public.marshall())
            .unwrap();
    }

    #[test]
    fn deposit_constraints_incorrectness_with_wrong_note() {
        let mut relation = relation();
        relation.public.note[0] += 1;
        let circuit = DepositRelation::generate_circuit(&relation).unwrap();
        assert!(circuit
            .check_circuit_satisfiability(&relation.public.marshall())
            .is_err());
    }

    #[test]
    fn deposit_proving_procedure() {
        let rng = &mut jf_utils::test_rng();
        let srs = generate_srs(10_000, rng).unwrap();

        let (pk, vk) = DepositRelation::generate_keys(&srs).unwrap();

        let relation = relation();
        let proof = relation.generate_proof(&pk, rng).unwrap();

        let public_input = relation.public.marshall();

        PlonkKzgSnark::<Curve>::verify::<StandardTranscript>(&vk, &public_input, &proof, None)
            .unwrap();
    }
}
