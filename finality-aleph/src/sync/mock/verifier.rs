use std::fmt::{Display, Error as FmtError, Formatter};

use crate::sync::{mock::MockJustification, Verifier};

#[derive(Debug)]
pub struct MockVerifier;

#[derive(Debug)]
pub enum Error {
    IncorrectJustification,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{:?}", self)
    }
}

impl Verifier<MockJustification> for MockVerifier {
    type Error = Error;

    fn verify(
        &mut self,
        justification: MockJustification,
    ) -> Result<MockJustification, Self::Error> {
        if justification.is_correct {
            Ok(justification)
        } else {
            Err(Error::IncorrectJustification)
        }
    }
}
