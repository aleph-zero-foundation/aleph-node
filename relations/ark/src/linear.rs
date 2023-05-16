use liminal_ark_relation_macro::snark_relation;

/// Linear equation relation: a*x + b = y
///
/// Relation with:
///  - 1 private witness (x)
///  - 3 constants       (a, b, y)
#[snark_relation]
mod relation {
    #[cfg(feature = "circuit")]
    use {
        ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, uint32::UInt32},
        ark_std::vec::Vec,
    };

    #[relation_object_definition]
    #[derive(Clone, Debug)]
    struct LinearEquationRelation {
        /// slope
        #[constant]
        pub a: u32,
        /// private witness
        #[private_input]
        pub x: u32,
        /// an intercept
        #[constant]
        pub b: u32,
        /// constant
        #[constant]
        pub y: u32,
    }

    #[cfg(feature = "circuit")]
    #[circuit_definition]
    fn generate_constraints() {
        // TODO: migrate from real values to values in the finite field (see FpVar)
        // Watch out for overflows!!!
        let x = UInt32::new_witness(ark_relations::ns!(cs, "x"), || self.x())?;
        let b = UInt32::new_constant(ark_relations::ns!(cs, "b"), self.b())?;
        let y = UInt32::new_constant(ark_relations::ns!(cs, "y"), self.y())?;

        let mut left = ark_std::iter::repeat(x)
            .take(*self.a() as usize)
            .collect::<Vec<UInt32<_>>>();

        left.push(b);

        UInt32::addmany(&left)?.enforce_equal(&y)
    }
}

#[cfg(all(test, feature = "circuit"))]
mod tests {
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef};
    use ark_snark::SNARK;

    use super::*;
    use crate::environment::CircuitField;

    const A: u32 = 2;
    const X: u32 = 1;
    const B: u32 = 1;
    const Y: u32 = 3;

    #[test]
    fn linear_constraints_correctness() {
        let circuit = LinearEquationRelationWithFullInput::new(A, B, Y, X);

        let cs: ConstraintSystemRef<CircuitField> = ConstraintSystem::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        let is_satisfied = cs.is_satisfied().unwrap();
        if !is_satisfied {
            println!("{:?}", cs.which_is_unsatisfied());
        }

        assert!(is_satisfied);
    }

    #[test]
    fn linear_proving_procedure() {
        let circuit_wo_input = LinearEquationRelationWithoutInput::new(A, B, Y);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_wo_input, &mut rng).unwrap();

        let circuit_with_public_input = LinearEquationRelationWithPublicInput::new(A, B, Y);
        let input = circuit_with_public_input.serialize_public_input();

        let circuit_with_full_input = LinearEquationRelationWithFullInput::new(A, B, Y, X);

        let proof = Groth16::prove(&pk, circuit_with_full_input, &mut rng).unwrap();
        let valid_proof = Groth16::verify(&vk, &input, &proof).unwrap();
        assert!(valid_proof);
    }
}
