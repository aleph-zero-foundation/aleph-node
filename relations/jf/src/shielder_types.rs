use ark_ff::{BigInteger256, PrimeField};
use jf_primitives::crhf::{FixedLengthRescueCRHF, CRHF};

use crate::CircuitField;

pub type Note = [u64; 4];
pub type Nullifier = [u64; 4];
pub type TokenId = u16;
pub type TokenAmount = u128;
pub type Trapdoor = [u64; 4];
pub type Account = [u8; 32];
pub type MerkleRoot = [u64; 4];
pub type MerklePath = Vec<[u64; 4]>;
pub type LeafIndex = u64;

pub fn convert_array(array: [u64; 4]) -> CircuitField {
    CircuitField::new(BigInteger256::new(array))
}

pub fn convert_vec(front: Vec<[u64; 4]>) -> Vec<CircuitField> {
    front.into_iter().map(convert_array).collect()
}

pub fn convert_account(front: [u8; 32]) -> CircuitField {
    CircuitField::from_le_bytes_mod_order(&front)
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
        convert_array(trapdoor),
        convert_array(nullifier),
        0.into(),
        0.into(),
    ];
    // todo: move conversion somewhere else
    FixedLengthRescueCRHF::<CircuitField, 6, 1>::evaluate(input).unwrap()[0]
        .into_bigint()
        .0
}
