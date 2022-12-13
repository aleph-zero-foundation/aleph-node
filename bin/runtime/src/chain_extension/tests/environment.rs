use std::{
    marker::PhantomData,
    sync::mpsc::{channel, Sender},
};

use super::*;

/// Trait serving as a type-level flag indicating which method we are testing.
pub(super) trait FunctionMode {}
/// We are testing `pallet_snarcos::store_key`.
pub(super) enum StoreKeyMode {}
impl FunctionMode for StoreKeyMode {}
/// We are testing `pallet_snarcos::verify`.
pub(super) enum VerifyMode {}
impl FunctionMode for VerifyMode {}

/// Trait serving as a type-level flag indicating how reading input from a contract should be done.
pub(super) trait ReadingMode {}
/// Reading fails - we won't be able to read a single byte.
pub(super) enum CorruptedMode {}
impl ReadingMode for CorruptedMode {}
/// Reading succeeds - we will read everything.
pub(super) enum StandardMode {}
impl ReadingMode for StandardMode {}

/// We will implement reading for every `ReadingMode`. However, there is no other way than such
/// `_Read` trait to tell Rust compiler that in fact, for every `RM` in `MockedEnvironment<_, RM>`
/// there will be such function.
trait _Read {
    fn _read(&self, max_len: ByteCount) -> Result<Vec<u8>, DispatchError>;
}

/// Testing counterpart for `pallet_snarcos::chain_extension::Environment`.
pub(super) struct MockedEnvironment<FM: FunctionMode, RM: ReadingMode> {
    /// Channel to report all charges.
    ///
    /// We have to save charges outside this object, because it is consumed by the main call.
    charging_channel: Sender<RevertibleWeight>,

    /// `Some(_)` only if `RM = CorruptedMode`.
    ///
    /// An optional callback to be invoked just before (failing to) read.
    on_read: Option<Box<dyn Fn()>>,
    /// `Some(_)` iff `RM = StandardMode`.
    content: Option<Vec<u8>>,

    /// How many bytes are there waiting to be read.
    in_len: ByteCount,

    _phantom: PhantomData<(FM, RM)>,
}

/// Creating environment with corrupted reading.
impl<FM: FunctionMode> MockedEnvironment<FM, CorruptedMode> {
    pub fn new(
        in_len: ByteCount,
        on_read: Option<Box<dyn Fn()>>,
    ) -> (Self, Receiver<RevertibleWeight>) {
        let (sender, receiver) = channel();
        (
            Self {
                charging_channel: sender,
                on_read,
                content: None,
                in_len,
                _phantom: Default::default(),
            },
            receiver,
        )
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
    pub fn new(content: Vec<u8>) -> (Self, Receiver<RevertibleWeight>) {
        let (sender, receiver) = channel();
        (
            Self {
                charging_channel: sender,
                on_read: None,
                in_len: content.len() as ByteCount,
                content: Some(content),
                _phantom: Default::default(),
            },
            receiver,
        )
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

/// In case we are testing `pallet_snarcos::store_key`, we might want to approximate how long is the
/// verifying key.
///
/// The returned value will be an upperbound - it will be the sum of the whole key encoding
/// (including its length).
impl<RM: ReadingMode> MockedEnvironment<StoreKeyMode, RM> {
    pub fn approx_key_len(&self) -> ByteCount {
        self.in_len
            .checked_sub(size_of::<VerificationKeyIdentifier>() as ByteCount)
            .unwrap()
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

    fn read(&self, max_len: u32) -> Result<Vec<u8>, DispatchError> {
        self._read(max_len)
    }

    fn charge_weight(&mut self, amount: Weight) -> Result<Weight, DispatchError> {
        self.charging_channel.send(amount.into()).unwrap();
        Ok(amount)
    }

    fn adjust_weight(&mut self, charged: Weight, actual_weight: Weight) {
        self.charging_channel
            .send(RevertibleWeight::neg(charged - actual_weight))
            .unwrap();
    }
}
