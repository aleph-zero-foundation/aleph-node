// use super::CircuitField;

use crate::environment::CircuitField;

/// The circuit lifting for `CircuitField`.
pub type FpVar = ark_r1cs_std::fields::fp::FpVar<CircuitField>;
/// The circuit lifting for the byte type.
pub type ByteVar = ark_r1cs_std::uint8::UInt8<CircuitField>;

// Types accepted by the relation constructors.
pub type FrontendNullifier = u64;
pub type FrontendTrapdoor = u64;
pub type FrontendNote = [u64; 4];
pub type FrontendTokenId = u16;
pub type FrontendTokenAmount = u64;
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
pub type BackendLeafIndex = CircuitField;
pub type BackendAccount = CircuitField;

/*
This is a setup for using Pedersen hashing (with field element compressing). It would work well, but
there is a serious problem with keeping/retrieving parameters in the contract. With the window
parameters defined below, serialized parameters take ~133kB. On the other hand, generating them
exhausts block capacity.


#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct PedersenWindow;
// We can hash 8 * 128 = 1024 bits.
impl pedersen::Window for PedersenWindow {
    const WINDOW_SIZE: usize = 8;
    const NUM_WINDOWS: usize = 128;
}

pub type HashFunction = PedersenCRHCompressor<EdwardsProjective, TECompressor, PedersenWindow>;
pub type HashFunctionParameters = <HashFunction as CRHTrait>::Parameters;
pub type Hash = <HashFunction as CRHTrait>::Output;

pub type HashFunctionGadget = PedersenCRHCompressorGadget<
    EdwardsProjective,
    TECompressor,
    PedersenWindow,
    EdwardsVar,
    TECompressorGadget,
>;
pub type HashFunctionParametersVar =
    <HashFunctionGadget as CRHGadget<HashFunction, CircuitField>>::ParametersVar;
pub type HashVar = <HashFunctionGadget as CRHGadget<HashFunction, CircuitField>>::OutputVar;
 */
