use frame_support::pallet_prelude::Weight;

use crate::backend::{weights::WeightInfo, ByteCount};

/// Test weights.
pub enum TestWeight {}

impl WeightInfo for TestWeight {
    fn verify() -> Weight {
        Weight::from(1)
    }

    fn verify_read_args(_: ByteCount) -> Weight {
        Weight::from(10)
    }
}
