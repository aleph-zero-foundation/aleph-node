use ark_bls12_381::Fr;
use ark_r1cs_std::prelude::AllocVar;
use ark_relations::r1cs::{ConstraintSystemRef, SynthesisError};
use liminal_ark_pnbr_poseidon_parameters::{Alpha, PoseidonParameters};
use liminal_ark_pnbr_sponge::{
    constraints::CryptographicSpongeVar,
    poseidon::{constraints::PoseidonSpongeVar, PoseidonParameters as ArkSpongePoseidonParameters},
};
use paste::paste;

use crate::{domain_separator, parameters::*};

type FpVar = ark_r1cs_std::fields::fp::FpVar<Fr>;

macro_rules! n_to_one {
    ($n: literal, $n_as_word: literal) => {
        paste! {
            #[doc = "Compute "]
            #[doc = stringify!($n)]
            #[doc = ":1 Poseidon hash of `input`."]
            pub fn [<$n_as_word _to_one_hash>] (cs: ConstraintSystemRef<Fr>, input: [FpVar; $n]) -> Result<FpVar, SynthesisError> {
                let parameters = [<rate_ $n>]::<Fr>();

                let mut state: PoseidonSpongeVar<Fr> = PoseidonSpongeVar::new(
                    cs.clone(),
                    &to_ark_sponge_poseidon_parameters(parameters),
                );
                let domain_separator = FpVar::new_constant(cs, domain_separator())?;
                state.absorb(&[ark_ff::vec![domain_separator], input.to_vec()].concat())?;
                let result = state.squeeze_field_elements(1)?;
                Ok(result[0].clone())
            }
        }
    };
}

n_to_one!(1, "one");
n_to_one!(2, "two");
n_to_one!(4, "four");

fn to_ark_sponge_poseidon_parameters(
    params: PoseidonParameters<Fr>,
) -> ArkSpongePoseidonParameters<Fr> {
    let alpha = match params.alpha {
        Alpha::Exponent(exp) => exp as u64,
        Alpha::Inverse => panic!("ark-sponge does not allow inverse alpha"),
    };
    let capacity = 1;
    let rate = params.t - capacity;
    let full_rounds = params.rounds.full();
    let partial_rounds = params.rounds.partial();

    ArkSpongePoseidonParameters {
        full_rounds,
        partial_rounds,
        alpha,
        ark: params.arc.into(),
        mds: params.mds.into(),
        rate,
        capacity,
    }
}
