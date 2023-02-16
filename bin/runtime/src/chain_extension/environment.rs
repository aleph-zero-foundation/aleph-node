use codec::{Decode, MaxEncodedLen};
use frame_support::weights::Weight;
use pallet_contracts::{
    chain_extension::{BufInBufOutState, Environment as SubstrateEnvironment, Ext, SysConfig},
    ChargedAmount,
};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::DispatchError;
use sp_std::vec::Vec;

use crate::chain_extension::ByteCount;

/// Abstraction around `pallet_contracts::chain_extension::Environment`. Makes testing easier.
///
/// Gathers all the methods that are used by `BabyLiminalChainExtension`. For now, all operations
/// are performed in the `BufInBufOut` mode, so we don't have to take care of other modes.
///
/// Each method is already documented in `pallet_contracts::chain_extension`.
pub(super) trait Environment: Sized {
    /// A type returned by `charge_weight` and passed to `adjust_weight`.
    ///
    /// The original type `ChargedAmount` has only a private constructor and thus we have to
    /// abstract it as well to make testing it possible.
    type ChargedAmount;

    fn in_len(&self) -> ByteCount;
    fn read(&self, max_len: u32) -> Result<Vec<u8>, DispatchError>;
    // It has to be `mut`, because there's a leftover in pallet contracts.
    fn read_as<T: Decode + MaxEncodedLen>(&mut self) -> Result<T, DispatchError>;
    // It has to be `mut`, because there's a leftover in pallet contracts.
    fn write(&mut self, buffer: &[u8]) -> Result<(), DispatchError>;

    fn charge_weight(&mut self, amount: Weight) -> Result<Self::ChargedAmount, DispatchError>;
    fn adjust_weight(&mut self, charged: Self::ChargedAmount, actual_weight: Weight);
}

/// Transparent delegation.
impl<E: Ext> Environment for SubstrateEnvironment<'_, '_, E, BufInBufOutState>
where
    <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
{
    type ChargedAmount = ChargedAmount;

    fn in_len(&self) -> ByteCount {
        self.in_len()
    }

    fn read(&self, max_len: u32) -> Result<Vec<u8>, DispatchError> {
        self.read(max_len)
    }

    fn read_as<T: Decode + MaxEncodedLen>(&mut self) -> Result<T, DispatchError> {
        self.read_as()
    }

    fn write(&mut self, buffer: &[u8]) -> Result<(), DispatchError> {
        self.write(buffer, false, None)
    }

    fn charge_weight(&mut self, amount: Weight) -> Result<Self::ChargedAmount, DispatchError> {
        self.charge_weight(amount)
    }

    fn adjust_weight(&mut self, charged: Self::ChargedAmount, actual_weight: Weight) {
        self.adjust_weight(charged, actual_weight)
    }
}
