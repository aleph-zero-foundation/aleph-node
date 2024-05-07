//TODO(A0-3750): This code should be used.
#![allow(dead_code)]
mod config;
mod handler;
mod service;

pub use config::setup;
pub use service::Service;

const LOG_TARGET: &str = "aleph-base-protocol";
