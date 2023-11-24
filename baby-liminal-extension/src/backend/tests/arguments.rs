use frame_support::sp_runtime::AccountId32;
use scale::Encode;

use crate::{
    args::{StoreKeyArgs, VerifyArgs},
    VerificationKeyIdentifier,
};

const DEPOSITOR: [u8; 32] = [1; 32];
const IDENTIFIER: VerificationKeyIdentifier = [41; 8];
const VK: [u8; 2] = [4, 1];
const PROOF: [u8; 20] = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2, 3, 8, 4];
const INPUT: [u8; 11] = [0, 5, 7, 7, 2, 1, 5, 6, 6, 4, 9];

/// Returns encoded arguments to `store_key` chain extension call.
pub fn store_key_args() -> Vec<u8> {
    StoreKeyArgs {
        depositor: AccountId32::new(DEPOSITOR),
        identifier: IDENTIFIER,
        key: VK.to_vec(),
    }
    .encode()
}

/// Returns encoded arguments to `verify` chain extension call.
pub fn verify_args() -> Vec<u8> {
    VerifyArgs {
        verification_key_identifier: IDENTIFIER,
        proof: PROOF.to_vec(),
        public_input: INPUT.to_vec(),
    }
    .encode()
}
