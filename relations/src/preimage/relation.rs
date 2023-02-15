use liminal_ark_relation_macro::snark_relation;

/// This relation showcases how to use Poseidon in r1cs circuits
#[snark_relation]
mod dummy_module {

    use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget};
    use ark_relations::ns;
    use liminal_ark_poseidon::circuit;

    use crate::{
        environment::FpVar,
        preimage::{FrontendHash, FrontendPreimage},
        shielder::convert_hash,
        CircuitField,
    };

    /// Preimage relation : H(preimage)=hash
    /// where:
    /// - hash : public input
    /// - preimage : private witness
    #[relation_object_definition]
    struct PreimageRelation {
        #[private_input(frontend_type = "FrontendPreimage", parse_with = "convert_hash")]
        pub preimage: CircuitField,
        #[public_input(frontend_type = "FrontendHash", parse_with = "convert_hash")]
        pub hash: CircuitField,
    }

    #[circuit_definition]
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<CircuitField>,
    ) -> Result<(), SynthesisError> {
        let preimage = FpVar::new_witness(ns!(cs, "preimage"), || self.preimage())?;
        let hash = FpVar::new_input(ns!(cs, "hash"), || self.hash())?;
        let hash_result = circuit::one_to_one_hash(cs, [preimage])?;

        hash.enforce_equal(&hash_result)?;

        Ok(())
    }
}
