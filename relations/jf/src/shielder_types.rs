use ark_ff::{BigInteger256, PrimeField};
use jf_primitives::crhf::{FixedLengthRescueCRHF, CRHF};

use crate::CircuitField;

pub type Note = [u64; 4];
pub type Nullifier = [u64; 4];
pub type TokenId = u16;
pub type TokenAmount = u128;
pub type Trapdoor = [u64; 4];

pub fn convert_hash(array: [u64; 4]) -> CircuitField {
    CircuitField::new(BigInteger256::new(array))
}

pub fn compute_note(
    token_id: TokenId,
    token_amount: TokenAmount,
    trapdoor: Trapdoor,
    nullifier: Nullifier,
) -> Note {
    let input: [CircuitField; 6] = [
        token_id.into(),
        token_amount.into(),
        convert_hash(trapdoor),
        convert_hash(nullifier),
        0.into(),
        0.into(),
    ];
    // todo: move conversion somewhere else
    FixedLengthRescueCRHF::<CircuitField, 6, 1>::evaluate(input).unwrap()[0]
        .into_bigint()
        .0
}
