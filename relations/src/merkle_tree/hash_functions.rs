use ark_crypto_primitives::crh::{
    injective_map::{PedersenCRHCompressor, TECompressor},
    pedersen,
};
use ark_ed_on_bls12_381::EdwardsProjective;

/// Way of calculating hash in parent from child nodes.
pub type TwoToOneHash = PedersenCRHCompressor<EdwardsProjective, TECompressor, TwoToOneWindow>;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct TwoToOneWindow;
// We can hash 4 * 128 = 2 * 256 bits, which should be enough for two nodes.
impl pedersen::Window for TwoToOneWindow {
    const WINDOW_SIZE: usize = 4;
    const NUM_WINDOWS: usize = 128;
}

/// Way of calculating hash in leaf node (from actual data).
pub type LeafHash = PedersenCRHCompressor<EdwardsProjective, TECompressor, LeafWindow>;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct LeafWindow;
// Kinda arbitrary. `WINDOW_SIZE * NUM_WINDOWS` must cover the hashed data len and
// be divisible by 8.
impl pedersen::Window for LeafWindow {
    const WINDOW_SIZE: usize = 4;
    const NUM_WINDOWS: usize = 128;
}
