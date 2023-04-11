#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

mod haltable;

pub use haltable::{Haltable, HaltableData, HaltableError, HaltableResult, Internal};
