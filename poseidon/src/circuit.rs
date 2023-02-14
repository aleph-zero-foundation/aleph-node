use ark_bls12_381::Fr;
use ark_r1cs_std::prelude::AllocVar;
use ark_relations::r1cs::{ConstraintSystemRef, SynthesisError};
use ark_sponge::{constraints::CryptographicSpongeVar, poseidon::constraints::PoseidonSpongeVar};
use paste::paste;

use crate::{parameters::*, DOMAIN_SEPARATOR};

type FpVar = ark_r1cs_std::fields::fp::FpVar<Fr>;

macro_rules! n_to_one {
    ($n: literal, $n_as_word: literal) => {
        paste! {
            pub fn [<$n_as_word _to_one_hash>] (cs: ConstraintSystemRef<Fr>, input: [FpVar; $n]) -> Result<FpVar, SynthesisError> {
                let parameters = [<RATE_ $n _PARAMETERS>].clone();

                let mut state: PoseidonSpongeVar<Fr> = PoseidonSpongeVar::new(
                    cs.clone(),
                    &to_ark_sponge_poseidon_parameters(parameters),
                );
                let domain_separator = FpVar::new_constant(cs, *DOMAIN_SEPARATOR)?;
                state.absorb(&[ark_std::vec![domain_separator], input.to_vec()].concat())?;
                let result = state.squeeze_field_elements(1)?;
                Ok(result[0].clone())
            }
        }
    };
}

n_to_one!(1, "one");
n_to_one!(2, "two");
n_to_one!(4, "four");
