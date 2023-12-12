use frame_support::pallet_prelude::Weight;

use crate::backend::ByteCount;

/// Weight information for the chain extension methods (analogous to the dispatchables).
pub trait WeightInfo {
    /// Weight for `verify` call.
    ///
    /// # Arguments
    ///
    /// To be added and measured (we are waiting for the proving backend choice).
    fn verify() -> Weight;

    /// Weight of reading arguments for the `verify` call.
    ///
    /// # Arguments
    ///
    /// * `input_length` - length of the input buffer.
    fn verify_read_args(input_length: ByteCount) -> Weight;
}

/// Real weights for the Aleph Zero chain.
pub enum AlephWeight {}
impl WeightInfo for AlephWeight {
    fn verify() -> Weight {
        Weight::zero()
    }

    fn verify_read_args(_: ByteCount) -> Weight {
        Weight::zero()
    }
}

/// Test weights.
#[cfg(test)]
pub enum TestWeight {}
#[cfg(test)]
impl WeightInfo for TestWeight {
    fn verify() -> Weight {
        Weight::from(1)
    }

    fn verify_read_args(_: ByteCount) -> Weight {
        Weight::from(10)
    }
}
