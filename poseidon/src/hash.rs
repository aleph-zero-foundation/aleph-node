use ark_bls12_381::Fr;
use paste::paste;

use crate::{parameters::*, DOMAIN_SEPARATOR};

macro_rules! n_to_one {
    ($n: literal, $n_as_word: literal) => {
        paste! {
            pub fn [<$n_as_word _to_one_hash>] (input: [Fr; $n]) -> Fr {
                let parameters = [<RATE_ $n _PARAMETERS>].clone();
                let mut state = poseidon_permutation::Instance::new(&parameters);
                state.n_to_1_fixed_hash([ark_std::vec![*DOMAIN_SEPARATOR], input.to_vec()].concat())
            }
        }
    };
}

n_to_one!(1, "one");
n_to_one!(2, "two");
n_to_one!(4, "four");
