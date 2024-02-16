use frame_support::{sp_runtime::DispatchError, weights::Weight};
use pallet_contracts::chain_extension::{
    BufInBufOutState, ChargedAmount, Environment as SubstrateEnvironment, Ext,
};
use parity_scale_codec::Decode;

use super::ByteCount;

/// Abstraction around `pallet_contracts::chain_extension::Environment`. Makes testing easier.
///
/// Gathers all the methods that are used by `BabyLiminalChainExtension`. For now, all operations
/// are performed in the `BufInBufOut` mode, so we don't have to take care of other modes.
#[allow(missing_docs)] // Every method is already documented in `pallet_contracts::chain_extension`.
pub trait Environment {
    /// A type returned by `charge_weight` and passed to `adjust_weight`.
    ///
    /// The original type `ChargedAmount` has only a private constructor and thus we have to
    /// abstract it as well to make testing it possible.
    type ChargedAmount;

    fn in_len(&self) -> ByteCount;
    // It has to be `mut`, because there's a leftover in pallet contracts.
    fn read_as_unbounded<T: Decode>(&mut self, len: u32) -> Result<T, DispatchError>;
    // It has to be `mut`, because there's a leftover in pallet contracts.
    fn write(
        &mut self,
        buffer: &[u8],
        allow_skip: bool,
        weight_per_byte: Option<Weight>,
    ) -> Result<(), DispatchError>;

    fn charge_weight(&mut self, amount: Weight) -> Result<Self::ChargedAmount, DispatchError>;
    fn adjust_weight(&mut self, charged: Self::ChargedAmount, actual_weight: Weight);
}

/// Transparent delegation.
impl<E: Ext> Environment for SubstrateEnvironment<'_, '_, E, BufInBufOutState> {
    type ChargedAmount = ChargedAmount;

    fn in_len(&self) -> ByteCount {
        self.in_len()
    }

    fn read_as_unbounded<T: Decode>(&mut self, len: u32) -> Result<T, DispatchError> {
        self.read_as_unbounded(len)
    }

    fn write(
        &mut self,
        buffer: &[u8],
        allow_skip: bool,
        weight_per_byte: Option<Weight>,
    ) -> Result<(), DispatchError> {
        self.write(buffer, allow_skip, weight_per_byte)
    }

    fn charge_weight(&mut self, amount: Weight) -> Result<Self::ChargedAmount, DispatchError> {
        self.charge_weight(amount)
    }

    fn adjust_weight(&mut self, charged: Self::ChargedAmount, actual_weight: Weight) {
        self.adjust_weight(charged, actual_weight)
    }
}
