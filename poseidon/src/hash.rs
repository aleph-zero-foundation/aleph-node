use ark_bls12_381::Fr;
use paste::paste;

use crate::{domain_separator, parameters::*};

macro_rules! n_to_one {
    ($n: literal, $n_as_word: literal) => {
        paste! {
            #[doc = "Compute "]
            #[doc = stringify!($n)]
            #[doc = ":1 Poseidon hash of `input`."]
            pub fn [<$n_as_word _to_one_hash>] (input: [Fr; $n]) -> Fr {
                let parameters = [<rate_ $n>]::<Fr>();
                let mut state = liminal_ark_pnbr_poseidon_permutation::Instance::new(&parameters);
                state.n_to_1_fixed_hash([ark_ff::vec![domain_separator()], input.to_vec()].concat())
            }
        }
    };
}

n_to_one!(1, "one");
n_to_one!(2, "two");
n_to_one!(4, "four");
