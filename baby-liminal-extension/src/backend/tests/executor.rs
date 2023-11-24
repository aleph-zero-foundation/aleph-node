#![allow(dead_code)]

use frame_support::pallet_prelude::Weight;
use pallet_baby_liminal::Error as PalletError;

use crate::{
    args::{StoreKeyArgs, VerifyArgs},
    backend::executor::BackendExecutor,
};

/// Describes how the `Executor` should behave when one of its methods is called.
#[derive(Clone, Eq, PartialEq)]
pub enum Responder {
    /// Twist and shout.
    Panicker,
    /// Return `Ok(())`.
    Okayer,
    /// Return `Err(Error)`.
    Errorer(PalletError<()>),
}

/// Auxiliary method to construct type argument.
///
/// Due to "`struct/enum construction is not supported in generic constants`".
pub const fn make_errorer<const ERROR: PalletError<()>>() -> Responder {
    Responder::Errorer(ERROR)
}

/// A testing counterpart for `Runtime`.
///
/// `STORE_KEY_RESPONDER` instructs how to behave then `store_key` is called.
/// `VERIFY_RESPONDER` instructs how to behave then `verify` is called.
pub struct MockedExecutor<const STORE_KEY_RESPONDER: Responder, const VERIFY_RESPONDER: Responder>;

/// Executor that will scream for every associated method.
pub type Panicker = MockedExecutor<{ Responder::Panicker }, { Responder::Panicker }>;

/// Executor that will return `Ok(())` for `store_key` and scream for `verify`.
pub type StoreKeyOkayer = MockedExecutor<{ Responder::Okayer }, { Responder::Panicker }>;
/// Executor that will return `Ok(())` for `verify` and scream for `store_key`.
pub type VerifyOkayer = MockedExecutor<{ Responder::Panicker }, { Responder::Okayer }>;

/// Executor that will return `Err(ERROR)` for `store_key` and scream for `verify`.
pub type StoreKeyErrorer<const ERROR: PalletError<()>> =
    MockedExecutor<{ make_errorer::<ERROR>() }, { Responder::Panicker }>;
/// Executor that will return `Err(ERROR)` for `verify` and scream for `store_key`.
pub type VerifyErrorer<const ERROR: PalletError<()>> =
    MockedExecutor<{ Responder::Panicker }, { make_errorer::<ERROR>() }>;

impl<const STORE_KEY_RESPONDER: Responder, const VERIFY_RESPONDER: Responder> BackendExecutor
    for MockedExecutor<STORE_KEY_RESPONDER, VERIFY_RESPONDER>
{
    type ErrorGenericType = ();

    fn store_key(_: StoreKeyArgs) -> Result<(), PalletError<()>> {
        match STORE_KEY_RESPONDER {
            Responder::Panicker => panic!("Function `store_key` shouldn't have been executed"),
            Responder::Okayer => Ok(()),
            Responder::Errorer(e) => Err(e),
        }
    }

    fn verify(_: VerifyArgs) -> Result<(), (PalletError<()>, Option<Weight>)> {
        match VERIFY_RESPONDER {
            Responder::Panicker => panic!("Function `verify` shouldn't have been executed"),
            Responder::Okayer => Ok(()),
            Responder::Errorer(e) => Err((e, None)),
        }
    }
}
