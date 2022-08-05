#![cfg_attr(not(feature = "std"), no_std)]

mod migration;

pub use migration::{ensure_storage_version, StorageMigration};
