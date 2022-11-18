#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

use ink_env::Hash;
use ink_prelude::vec::Vec;
use ink_storage::Mapping;
use snarcos_extension::{ProvingSystem, VerificationKeyIdentifier};

mod contract;
mod error;

type Scalar = u64;
type Nullifier = Scalar;

/// Type of the value in the Merkle tree leaf.
type Note = Hash;
/// Type of the value in the Merkle tree root.
type MerkleRoot = Hash;

/// Short identifier of a registered token contract.
type TokenId = u16;
/// `arkworks` does not support serializing `u128` and thus we have to operate on `u64` amounts.
type TokenAmount = u64;

type Set<T> = Mapping<T, ()>;

/// Verification key identifier for the `deposit` relation (to be registered in `pallet_snarcos`).
const DEPOSIT_VK_IDENTIFIER: VerificationKeyIdentifier = [b'd', b'p', b's', b't'];
/// Verification key identifier for the `withdraw` relation (to be registered in `pallet_snarcos`).
const WITHDRAW_VK_IDENTIFIER: VerificationKeyIdentifier = [b'w', b't', b'h', b'd'];
/// The only supported proving system for now.
const SYSTEM: ProvingSystem = ProvingSystem::Groth16;

/// PSP22 standard selector for transferring on behalf.
const PSP22_TRANSFER_FROM_SELECTOR: [u8; 4] = [0x54, 0xb3, 0xc7, 0x6e];
/// PSP22 standard selector for transferring own tokens.
const PSP22_TRANSFER_SELECTOR: [u8; 4] = [0xdb, 0x20, 0xf9, 0xf5];

/// Temporary implementation of two-to-one hashing function.
fn kinder_blender(left: &Hash, right: &Hash) -> Hash {
    left.as_ref()
        .iter()
        .cloned()
        .zip(right.as_ref().iter().cloned())
        .map(|(l, r)| l ^ r)
        .collect::<Vec<_>>()
        .as_slice()
        .try_into()
        .unwrap()
}
