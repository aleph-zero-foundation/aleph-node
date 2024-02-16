use std::marker::PhantomData;

use frame_support::{pallet_prelude::Weight, sp_runtime::DispatchError};
use parity_scale_codec::Decode;

use crate::backend::{environment::Environment, ByteCount};

/// Trait serving as a type-level flag indicating which method we are testing.
pub trait FunctionMode {}
/// We are testing `verify`.
pub enum VerifyMode {}
impl FunctionMode for VerifyMode {}

/// Trait serving as a type-level flag indicating how reading input from a contract should be done.
pub trait ReadingMode {}
/// Reading fails – we won't be able to read a single byte.
pub enum CorruptedMode {}
impl ReadingMode for CorruptedMode {}
/// Reading succeeds – we will read everything.
pub enum StandardMode {}
impl ReadingMode for StandardMode {}

/// We will implement reading for every `ReadingMode`. However, there is no other way than such
/// `_Read` trait to tell Rust compiler that in fact, for every `RM` in `MockedEnvironment<_, RM>`
/// there will be such function.
trait _Read {
    fn _read(&self, max_len: ByteCount) -> Result<Vec<u8>, DispatchError>;
}

/// A testing implementation for `crate::backend::environment::Environment`.
pub struct MockedEnvironment<'charger, FM: FunctionMode, RM: ReadingMode> {
    /// Accumulator for charged weight.
    charger: &'charger mut Weight,

    /// How many bytes are there waiting to be read.
    in_len: ByteCount,

    /// `Some(_)` iff `RM = StandardMode`.
    content: Option<Vec<u8>>,

    _phantom: PhantomData<(FM, RM)>,
}

/// Creating environment with corrupted reading.
impl<'charger, FM: FunctionMode> MockedEnvironment<'charger, FM, CorruptedMode> {
    pub fn new(charger: &'charger mut Weight, in_len: ByteCount) -> Self {
        Self {
            charger,
            in_len,
            content: None,
            _phantom: Default::default(),
        }
    }
}

/// Corrupted reading with possible additional callback invoked.
impl<'charger, FM: FunctionMode> _Read for MockedEnvironment<'charger, FM, CorruptedMode> {
    fn _read(&self, _max_len: ByteCount) -> Result<Vec<u8>, DispatchError> {
        Err(DispatchError::Other("Some error"))
    }
}

/// Creating environment with correct reading of `content`.
impl<'charger, FM: FunctionMode> MockedEnvironment<'charger, FM, StandardMode> {
    pub fn new(charger: &'charger mut Weight, content: Vec<u8>) -> Self {
        Self {
            charger,
            in_len: content.len() as ByteCount,
            content: Some(content),
            _phantom: Default::default(),
        }
    }
}

/// Successful reading.
impl<'charger, FM: FunctionMode> _Read for MockedEnvironment<'charger, FM, StandardMode> {
    fn _read(&self, max_len: ByteCount) -> Result<Vec<u8>, DispatchError> {
        let content = self.content.as_ref().unwrap();
        if max_len > self.in_len {
            Ok(content.clone())
        } else {
            Ok(content[..max_len as usize].to_vec())
        }
    }
}

impl<'charger, FM: FunctionMode, RM: ReadingMode> Environment
    for MockedEnvironment<'charger, FM, RM>
where
    MockedEnvironment<'charger, FM, RM>: _Read,
{
    type ChargedAmount = Weight;

    fn in_len(&self) -> ByteCount {
        self.in_len
    }

    fn read_as_unbounded<T: Decode>(&mut self, len: u32) -> Result<T, DispatchError> {
        self._read(len)
            .map(|bytes| Decode::decode(&mut &bytes[..]).expect("Decoding should succeed"))
    }

    fn write(&mut self, _: &[u8], _: bool, _: Option<Weight>) -> Result<(), DispatchError> {
        todo!()
    }

    fn charge_weight(&mut self, amount: Weight) -> Result<Weight, DispatchError> {
        *self.charger += amount;
        Ok(amount)
    }

    fn adjust_weight(&mut self, previously_charged: Weight, actual_charge: Weight) {
        *self.charger -= previously_charged;
        *self.charger += actual_charge;
    }
}
