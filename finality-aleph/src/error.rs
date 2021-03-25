use crate::communication::network::NetworkError;
use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug)]
enum ErrorKind {
    Network(NetworkError),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        use ErrorKind::*;
        match self {
            Network(e) => std::fmt::Display::fmt(&e, f),
        }
    }
}

impl StdError for ErrorKind {}

#[derive(Debug)]
pub struct Error(Box<ErrorKind>);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.0)
    }
}

impl From<NetworkError> for Error {
    fn from(e: NetworkError) -> Error {
        Error(Box::new(ErrorKind::Network(e)))
    }
}
