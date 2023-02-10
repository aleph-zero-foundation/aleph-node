use pallet_baby_liminal::{Error as BabyLiminalError, ProvingSystem, VerificationKeyIdentifier};

use crate::{chain_extension::executor::Executor, Weight};

type Error = BabyLiminalError<()>;

/// Describes how the `Executor` should behave when one of its methods is called.
#[derive(Clone, Eq, PartialEq)]
pub(super) enum Responder {
    /// Twist and shout.
    Panicker,
    /// Return `Ok(())`.
    Okayer,
    /// Return `Err(Error)`.
    Errorer(Error, Option<Weight>),
}

/// Testing counterpart for `Runtime`.
///
/// `STORE_KEY_RESPONDER` instructs how to behave then `store_key` is called.
/// `VERIFY_RESPONDER` instructs how to behave then `verify` is called.
pub(super) struct MockedExecutor<
    const STORE_KEY_RESPONDER: Responder,
    const VERIFY_RESPONDER: Responder,
>;

/// Auxiliary method to construct type argument.
///
/// Due to "`struct/enum construction is not supported in generic constants`".
pub(super) const fn make_errorer<const ERROR: Error, const WEIGHT: Option<u64>>() -> Responder {
    Responder::Errorer(
        ERROR,
        match WEIGHT {
            None => None,
            Some(w) => Some(Weight::from_ref_time(w)),
        },
    )
}

/// Executor that will scream for every associated method.
pub(super) type Panicker = MockedExecutor<{ Responder::Panicker }, { Responder::Panicker }>;

/// Executor that will return `Ok(())` for `store_key` and scream for `verify`.
pub(super) type StoreKeyOkayer = MockedExecutor<{ Responder::Okayer }, { Responder::Panicker }>;
/// Executor that will return `Ok(())` for `verify` and scream for `store_key`.
pub(super) type VerifyOkayer = MockedExecutor<{ Responder::Panicker }, { Responder::Okayer }>;

pub(super) const NO_WEIGHT: Option<u64> = None;

/// Executor that will return `Err(ERROR)` for `store_key` and scream for `verify`.
pub(super) type StoreKeyErrorer<const ERROR: Error> =
    MockedExecutor<{ make_errorer::<ERROR, NO_WEIGHT>() }, { Responder::Panicker }>;
/// Executor that will return `Err(ERROR)` for `verify` and scream for `store_key`.
pub(super) type VerifyErrorer<const ERROR: Error, const WEIGHT: Option<u64>> =
    MockedExecutor<{ Responder::Panicker }, { make_errorer::<ERROR, WEIGHT>() }>;

impl<const STORE_KEY_RESPONDER: Responder, const VERIFY_RESPONDER: Responder> Executor
    for MockedExecutor<STORE_KEY_RESPONDER, VERIFY_RESPONDER>
{
    type ErrorGenericType = ();

    fn store_key(_identifier: VerificationKeyIdentifier, _key: Vec<u8>) -> Result<(), Error> {
        match STORE_KEY_RESPONDER {
            Responder::Panicker => panic!("Function `store_key` shouldn't have been executed"),
            Responder::Okayer => Ok(()),
            Responder::Errorer(e, _) => Err(e),
        }
    }

    fn verify(
        _verification_key_identifier: VerificationKeyIdentifier,
        _proof: Vec<u8>,
        _public_input: Vec<u8>,
        _system: ProvingSystem,
    ) -> Result<(), (Error, Option<Weight>)> {
        match VERIFY_RESPONDER {
            Responder::Panicker => panic!("Function `verify` shouldn't have been executed"),
            Responder::Okayer => Ok(()),
            Responder::Errorer(e, w) => Err((e, w)),
        }
    }
}
