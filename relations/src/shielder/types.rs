use ark_std::vec::Vec;

use crate::environment::CircuitField;

#[cfg(feature = "circuit")]
/// The circuit lifting for the byte type.
pub type ByteVar = ark_r1cs_std::uint8::UInt8<CircuitField>;

// Types accepted by the relation constructors.
pub type FrontendNullifier = [u64; 4];
pub type FrontendTrapdoor = [u64; 4];
pub type FrontendNote = [u64; 4];
pub type FrontendTokenId = u16;
pub type FrontendTokenAmount = u128;
pub type FrontendMerkleRoot = [u64; 4];
pub type FrontendMerklePath = Vec<[u64; 4]>;
pub type FrontendLeafIndex = u64;
pub type FrontendAccount = [u8; 32];
pub type FrontendMerklePathNode = [u64; 4];

// Types used internally by the relations (but still outside circuit environment).
pub type BackendNullifier = CircuitField;
pub type BackendTrapdoor = CircuitField;
pub type BackendNote = CircuitField;
pub type BackendTokenId = CircuitField;
pub type BackendTokenAmount = CircuitField;
pub type BackendMerkleRoot = CircuitField;
pub type BackendMerklePath = Vec<CircuitField>;
pub type BackendLeafIndex = u64;
pub type BackendAccount = CircuitField;
