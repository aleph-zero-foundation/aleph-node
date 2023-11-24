#![allow(dead_code)]

use std::marker::PhantomData;

use frame_support::{pallet_prelude::Weight, sp_runtime::DispatchError};
use scale::Decode;

use crate::backend::{environment::Environment, ByteCount};

/// Trait serving as a type-level flag indicating which method we are testing.
pub trait FunctionMode {}
/// We are testing `store_key`.
pub enum StoreKeyMode {}
impl FunctionMode for StoreKeyMode {}
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
pub struct MockedEnvironment<FM: FunctionMode, RM: ReadingMode> {
    /// An optional callback to be invoked just before (failing to) read. `Some(_)` only if
    /// `RM = CorruptedMode`.
    on_read: Option<Box<dyn Fn()>>,

    /// How many bytes are there waiting to be read.
    in_len: ByteCount,

    /// `Some(_)` iff `RM = StandardMode`.
    content: Option<Vec<u8>>,

    _phantom: PhantomData<(FM, RM)>,
}

/// Creating environment with corrupted reading.
impl<FM: FunctionMode> MockedEnvironment<FM, CorruptedMode> {
    pub fn new(in_len: ByteCount, on_read: Option<Box<dyn Fn()>>) -> Self {
        Self {
            on_read,
            in_len,
            content: None,
            _phantom: Default::default(),
        }
    }
}

/// Corrupted reading with possible additional callback invoked.
impl<FM: FunctionMode> _Read for MockedEnvironment<FM, CorruptedMode> {
    fn _read(&self, _max_len: ByteCount) -> Result<Vec<u8>, DispatchError> {
        self.on_read.as_ref().map(|action| action());
        Err(DispatchError::Other("Some error"))
    }
}

/// Creating environment with correct reading of `content`.
impl<FM: FunctionMode> MockedEnvironment<FM, StandardMode> {
    pub fn new(content: Vec<u8>) -> Self {
        Self {
            on_read: None,
            in_len: content.len() as ByteCount,
            content: Some(content),
            _phantom: Default::default(),
        }
    }
}

/// Successful reading.
impl<FM: FunctionMode> _Read for MockedEnvironment<FM, StandardMode> {
    fn _read(&self, max_len: ByteCount) -> Result<Vec<u8>, DispatchError> {
        let content = self.content.as_ref().unwrap();
        if max_len > self.in_len {
            Ok(content.clone())
        } else {
            Ok(content[..max_len as usize].to_vec())
        }
    }
}

impl<FM: FunctionMode, RM: ReadingMode> Environment for MockedEnvironment<FM, RM>
where
    MockedEnvironment<FM, RM>: _Read,
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
        Ok(amount)
    }

    fn adjust_weight(&mut self, _: Weight, _: Weight) {}
}
