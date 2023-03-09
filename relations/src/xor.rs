use liminal_ark_relation_macro::snark_relation;

/// XOR relation: a ⊕ b = c
///
/// Relation with:
///  - 1 public input    (a | `public_xoree`)
///  - 1 private witness (b | `private_xoree`)
///  - 1 constant        (c | `result`)
/// such that: a ^ b = c.
#[snark_relation]
mod relation {
    use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, uint8::UInt8};

    use crate::byte_to_bits;

    #[relation_object_definition]
    struct XorRelation {
        // ToDo: Especially for Groth16, it is better to provide public input as a field element.
        // Otherwise, we have to provide it to circuit bit by bit.
        #[public_input(serialize_with = "byte_to_bits")]
        public_xoree: u8,
        #[private_input]
        private_xoree: u8,
        #[constant]
        result: u8,
    }

    #[circuit_definition]
    fn generate_constraints() {
        let public_xoree = UInt8::new_input(ark_relations::ns!(cs, "public_xoree"), || {
            self.public_xoree()
        })?;
        let private_xoree = UInt8::new_witness(ark_relations::ns!(cs, "private_xoree"), || {
            self.private_xoree()
        })?;
        let result = UInt8::new_constant(ark_relations::ns!(cs, "result"), self.result())?;

        let xor = UInt8::xor(&public_xoree, &private_xoree)?;
        xor.enforce_equal(&result)
    }
}

#[cfg(test)]
mod tests {
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef};
    use ark_snark::SNARK;

    use super::*;
    use crate::CircuitField;

    const A: u8 = 2;
    const B: u8 = 3;
    const C: u8 = 1;

    #[test]
    fn xor_constraints_correctness() {
        let circuit = XorRelationWithFullInput::new(A, B, C);

        let cs: ConstraintSystemRef<CircuitField> = ConstraintSystem::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        let is_satisfied = cs.is_satisfied().unwrap();
        if !is_satisfied {
            println!("{:?}", cs.which_is_unsatisfied());
        }

        assert!(is_satisfied);
    }

    #[test]
    fn xor_proving_procedure() {
        let circuit_wo_input = XorRelationWithoutInput::new(C);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_wo_input, &mut rng).unwrap();

        let circuit_with_public_input = XorRelationWithPublicInput::new(C, A);
        let input = circuit_with_public_input.serialize_public_input();

        let circuit_with_full_input = XorRelationWithFullInput::new(C, A, B);

        let proof = Groth16::prove(&pk, circuit_with_full_input, &mut rng).unwrap();
        let valid_proof = Groth16::verify(&vk, &input, &proof).unwrap();
        assert!(valid_proof);
    }
}
