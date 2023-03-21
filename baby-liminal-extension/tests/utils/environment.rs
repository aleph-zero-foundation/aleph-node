use std::{
    iter::Sum,
    marker::PhantomData,
    mem::size_of,
    ops::Neg,
    sync::mpsc::{channel, Receiver, Sender},
};

use baby_liminal_extension::{
    executor::Executor, substrate::ByteCount, ProvingSystem, VerificationKeyIdentifier,
};
use obce::substrate::{
    frame_support::weights::Weight, sp_runtime::AccountId32, ChainExtensionEnvironment,
    CriticalError,
};
use pallet_baby_liminal::Error;

use crate::utils::{STORE_KEY_ID, VERIFY_ID};

/// In order to compute final fee (after all adjustments) sometimes we will have to subtract
/// weights.
#[derive(Debug, PartialEq, Eq)]
pub struct RevertibleWeight(i64);

impl RevertibleWeight {
    fn neg(weight: Weight) -> Self {
        RevertibleWeight((weight.ref_time() as i64).neg())
    }

    pub const ZERO: RevertibleWeight = RevertibleWeight(0);
}

impl From<Weight> for RevertibleWeight {
    fn from(w: Weight) -> Self {
        RevertibleWeight(w.ref_time() as i64)
    }
}

impl Sum for RevertibleWeight {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        RevertibleWeight(iter.map(|rw| rw.0).sum())
    }
}

/// Trait serving as a type-level flag indicating how reading input from a contract should be done.
pub trait ReadingMode {}
/// Reading fails - we won't be able to read a single byte.
pub enum CorruptedMode {}
impl ReadingMode for CorruptedMode {}
/// Reading succeeds - we will read everything.
pub enum StandardMode {}
impl ReadingMode for StandardMode {}

/// Describes how the environment should behave when one of pallet methods are called.
#[derive(Clone, Eq, PartialEq)]
pub enum Responder {
    /// Twist and shout.
    Panicker,
    /// Return `Ok(())`.
    Okayer,
    /// Return `Err(Error)`.
    Errorer(Error<()>, Option<Weight>),
}

/// We will implement reading for every `ReadingMode`. However, there is no other way than such
/// `_Read` trait to tell Rust compiler that in fact, for every `RM` in `MockedEnvironment<_, RM>`
/// there will be such function.
trait _Read {
    fn _read<U: scale::Decode>(&self, len: ByteCount) -> Result<U, CriticalError>;
}

/// Testing counterpart for `pallet_baby_liminal::chain_extension::Environment`.
pub struct MockedEnvironment<
    const METHOD: u16,
    RM: ReadingMode,
    const STORE_KEY_RESPONDER: Responder,
    const VERIFY_RESPONDER: Responder,
> {
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

    _phantom: PhantomData<RM>,
}

/// Creating environment with corrupted reading.
impl<
        const METHOD: u16,
        const STORE_KEY_RESPONDER: Responder,
        const VERIFY_RESPONDER: Responder,
    > MockedEnvironment<METHOD, CorruptedMode, STORE_KEY_RESPONDER, VERIFY_RESPONDER>
{
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
impl<
        const METHOD: u16,
        const STORE_KEY_RESPONDER: Responder,
        const VERIFY_RESPONDER: Responder,
    > _Read for MockedEnvironment<METHOD, CorruptedMode, STORE_KEY_RESPONDER, VERIFY_RESPONDER>
{
    fn _read<U: scale::Decode>(&self, _: ByteCount) -> Result<U, CriticalError> {
        if let Some(action) = self.on_read.as_ref() {
            action()
        }
        Err(CriticalError::Other("Some error"))
    }
}

/// Creating environment with correct reading of `content`.
impl<
        const METHOD: u16,
        const STORE_KEY_RESPONDER: Responder,
        const VERIFY_RESPONDER: Responder,
    > MockedEnvironment<METHOD, StandardMode, STORE_KEY_RESPONDER, VERIFY_RESPONDER>
{
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

/// Successful reading
impl<
        const METHOD: u16,
        const STORE_KEY_RESPONDER: Responder,
        const VERIFY_RESPONDER: Responder,
    > _Read for MockedEnvironment<METHOD, StandardMode, STORE_KEY_RESPONDER, VERIFY_RESPONDER>
{
    fn _read<U: scale::Decode>(&self, len: ByteCount) -> Result<U, CriticalError> {
        let content = self.content.as_ref().unwrap();
        if len > self.in_len {
            Ok(U::decode(&mut &content[..]).unwrap())
        } else {
            Ok(U::decode(&mut &content[..len as usize]).unwrap())
        }
    }
}

/// In case we are testing `pallet_baby_liminal::store_key`, we might want to approximate how long is the
/// verifying key.
///
/// The returned value will be an upperbound - it will be the sum of the whole key encoding
/// (including its length).
impl<RM: ReadingMode, const STORE_KEY_RESPONDER: Responder, const VERIFY_RESPONDER: Responder>
    MockedEnvironment<STORE_KEY_ID, RM, STORE_KEY_RESPONDER, VERIFY_RESPONDER>
{
    pub fn approx_key_len(&self) -> ByteCount {
        self.in_len
            .checked_sub(size_of::<VerificationKeyIdentifier>() as ByteCount)
            .unwrap()
    }
}

impl<
        const METHOD: u16,
        RM: ReadingMode,
        const STORE_KEY_RESPONDER: Responder,
        const VERIFY_RESPONDER: Responder,
        E,
        T,
    > ChainExtensionEnvironment<E, T>
    for MockedEnvironment<METHOD, RM, STORE_KEY_RESPONDER, VERIFY_RESPONDER>
where
    MockedEnvironment<METHOD, RM, STORE_KEY_RESPONDER, VERIFY_RESPONDER>: _Read,
{
    type ChargedAmount = Weight;

    fn func_id(&self) -> u16 {
        METHOD
    }

    fn ext_id(&self) -> u16 {
        <dyn baby_liminal_extension::BabyLiminalExtension as obce::codegen::ExtensionDescription>::ID
    }

    fn in_len(&self) -> ByteCount {
        self.in_len
    }

    fn read(&self, _: u32) -> Result<Vec<u8>, CriticalError> {
        todo!()
    }

    fn read_into(&self, _: &mut &mut [u8]) -> Result<(), CriticalError> {
        todo!()
    }

    fn read_as<U: scale::Decode + scale::MaxEncodedLen>(&mut self) -> Result<U, CriticalError> {
        todo!()
    }

    fn read_as_unbounded<U: scale::Decode>(&mut self, len: ByteCount) -> Result<U, CriticalError> {
        self._read(len)
    }

    fn write(&mut self, _: &[u8], _: bool, _: Option<Weight>) -> Result<(), CriticalError> {
        Ok(())
    }

    fn ext(&mut self) -> &mut E {
        todo!()
    }

    fn charge_weight(&mut self, amount: Weight) -> Result<Weight, CriticalError> {
        self.charging_channel.send(amount.into()).unwrap();
        Ok(amount)
    }

    fn adjust_weight(&mut self, charged: Weight, actual_weight: Weight) {
        self.charging_channel
            .send(RevertibleWeight::neg(charged - actual_weight))
            .unwrap();
    }
}

/// Auxiliary method to construct type argument.
///
/// Due to "`struct/enum construction is not supported in generic constants`".
pub const fn make_errorer<const ERROR: Error<()>, const WEIGHT: Option<u64>>() -> Responder {
    Responder::Errorer(
        ERROR,
        match WEIGHT {
            None => None,
            Some(w) => Some(Weight::from_ref_time(w)),
        },
    )
}

/// Executor that will return `Ok(())` for `store_key` and scream for `verify`.
pub type StoreKeyOkayer =
    MockedEnvironment<STORE_KEY_ID, StandardMode, { Responder::Okayer }, { Responder::Panicker }>;
/// Executor that will return `Ok(())` for `verify` and scream for `store_key`.
pub type VerifyOkayer =
    MockedEnvironment<VERIFY_ID, StandardMode, { Responder::Panicker }, { Responder::Okayer }>;

pub const NO_WEIGHT: Option<u64> = None;

/// Executor that will return `Err(ERROR)` for `store_key` and scream for `verify`.
pub type StoreKeyErrorer<const ERROR: Error<()>> = MockedEnvironment<
    STORE_KEY_ID,
    StandardMode,
    { make_errorer::<ERROR, NO_WEIGHT>() },
    { Responder::Panicker },
>;
/// Executor that will return `Err(ERROR)` for `verify` and scream for `store_key`.
pub type VerifyErrorer<const ERROR: Error<()>, const WEIGHT: Option<u64>> = MockedEnvironment<
    VERIFY_ID,
    StandardMode,
    { Responder::Panicker },
    { make_errorer::<ERROR, WEIGHT>() },
>;

impl<
        T,
        RM: ReadingMode,
        const METHOD: u16,
        const STORE_KEY_RESPONDER: Responder,
        const VERIFY_RESPONDER: Responder,
    > Executor<T> for MockedEnvironment<METHOD, RM, STORE_KEY_RESPONDER, VERIFY_RESPONDER>
{
    type ErrorGenericType = ();

    fn store_key(
        _: AccountId32,
        _: VerificationKeyIdentifier,
        _: Vec<u8>,
    ) -> Result<(), Error<Self::ErrorGenericType>> {
        match STORE_KEY_RESPONDER {
            Responder::Panicker => panic!("Function `store_key` shouldn't have been executed"),
            Responder::Okayer => Ok(()),
            Responder::Errorer(e, _) => Err(e),
        }
    }

    fn verify(
        _: VerificationKeyIdentifier,
        _: Vec<u8>,
        _: Vec<u8>,
        _: ProvingSystem,
    ) -> Result<(), (Error<Self::ErrorGenericType>, Option<Weight>)> {
        match VERIFY_RESPONDER {
            Responder::Panicker => panic!("Function `verify` shouldn't have been executed"),
            Responder::Okayer => Ok(()),
            Responder::Errorer(e, w) => Err((e, w)),
        }
    }
}
