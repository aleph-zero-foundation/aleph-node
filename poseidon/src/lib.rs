#![cfg_attr(not(feature = "std"), no_std)]

use ark_bls12_381::Fr;
use once_cell::sync::Lazy;
mod parameters;

type CircuitField = ark_bls12_381::Fr;
type FpVar = ark_r1cs_std::fields::fp::FpVar<CircuitField>;

// Poseidon paper suggests using domain separation for concretely encoding the use case in the capacity element (which is fine as it is 256 bits large and has a lot of bits to fill)
pub static DOMAIN_SEPARATOR: Lazy<Fr> = Lazy::new(|| Fr::from(2137));

pub mod hash {
    use ark_bls12_381::Fr;
    use ark_std::vec;
    use poseidon_permutation::Instance;

    use super::DOMAIN_SEPARATOR;
    use crate::parameters::RATE_1_PARAMETERS;
    /// hashes one field value, outputs a fixed length field value
    pub fn one_to_one_hash(value: Fr) -> Fr {
        let parameters = RATE_1_PARAMETERS.clone();
        let mut state = Instance::new(&parameters);
        state.n_to_1_fixed_hash(vec![*DOMAIN_SEPARATOR, value])
    }
}

pub mod circuit {
    use ark_bls12_381::Fr;
    use ark_r1cs_std::prelude::AllocVar;
    use ark_relations::r1cs::{ConstraintSystemRef, SynthesisError};
    use ark_sponge::{
        constraints::CryptographicSpongeVar, poseidon::constraints::PoseidonSpongeVar,
    };
    use ark_std::vec;

    use super::{FpVar, DOMAIN_SEPARATOR};
    use crate::parameters::{to_ark_sponge_poseidon_parameters, RATE_1_PARAMETERS};

    /// hashes one field value inside the circuit    
    pub fn one_to_one_hash(
        cs: ConstraintSystemRef<Fr>,
        value: FpVar,
    ) -> Result<FpVar, SynthesisError> {
        let mut state: PoseidonSpongeVar<Fr> = PoseidonSpongeVar::new(
            cs.clone(),
            &to_ark_sponge_poseidon_parameters(RATE_1_PARAMETERS.clone()),
        );
        let domain_separator = FpVar::new_constant(cs, *DOMAIN_SEPARATOR)?;
        state.absorb(&vec![domain_separator, value])?;
        let result = state.squeeze_field_elements(1)?;
        Ok(result[0].clone())
    }
}
