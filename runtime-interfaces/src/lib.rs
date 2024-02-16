//! A collection of runtime interfaces (Substrate's concept for outsourcing computation to the host) for Aleph Zero
//! chain.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

pub mod snark_verifier;
