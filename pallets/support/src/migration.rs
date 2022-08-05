#[cfg(feature = "try-runtime")]
use frame_support::{
    codec::{Decode, Encode},
    sp_io,
    storage::storage_prefix,
};
use frame_support::{
    pallet_prelude::{PalletInfoAccess, StorageVersion, Weight},
    traits::OnRuntimeUpgrade,
};

/// In order to run both pre- and post- checks around every migration, we entangle methods of
/// `OnRuntimeUpgrade` into the desired flow and expose it with `migrate` method.
///
/// This way, `try-runtime` no longer triggers checks. We do it by hand.
pub trait StorageMigration: OnRuntimeUpgrade {
    #[cfg(feature = "try-runtime")]
    const MIGRATION_STORAGE_PREFIX: &'static [u8];

    #[allow(clippy::let_and_return)]
    fn migrate() -> Weight {
        #[cfg(feature = "try-runtime")]
        Self::pre_upgrade().expect("Pre upgrade should succeed");

        let weight = Self::on_runtime_upgrade();

        #[cfg(feature = "try-runtime")]
        Self::post_upgrade().expect("Post upgrade should succeed");

        weight
    }

    /// Wrapper for `OnRuntimeUpgradeHelpersExt::set_temp_storage`.
    ///
    /// Together with the associated const `MIGRATION_STORAGE_PREFIX` they form a shortcut for:
    /// ```rust
    /// # use frame_support::traits::OnRuntimeUpgradeHelpersExt;
    /// # use crate::pallet_elections::Config;
    /// # use frame_support::storage::storage_prefix;
    /// # use frame_support::pallet_prelude::PalletInfoAccess;
    /// # use frame_support::sp_std;
    ///
    /// #[cfg(feature = "try-runtime")]
    /// const MIGRATION_STORAGE_PREFIX: &[u8] = b"...";
    ///
    /// # struct Migration<T, P>(sp_std::marker::PhantomData<(T, P)>);
    ///
    /// #[cfg(feature = "try-runtime")]
    /// impl<T: Config, P: PalletInfoAccess> OnRuntimeUpgradeHelpersExt for Migration<T, P> {
    ///     fn storage_key(ident: &str) -> [u8; 32] {
    ///         storage_prefix(MIGRATION_STORAGE_PREFIX, ident.as_bytes())
    ///     }
    /// }
    /// ```
    /// which would be required for every implementor of `StorageMigration`.
    #[cfg(feature = "try-runtime")]
    fn store_temp<T: Encode>(storage_key: &str, data: T) {
        let full_key = storage_prefix(Self::MIGRATION_STORAGE_PREFIX, storage_key.as_bytes());
        sp_io::storage::set(&full_key, &data.encode());
    }

    /// Wrapper for `OnRuntimeUpgradeHelpersExt::get_temp_storage`.
    ///
    /// Analogous to `Self::store_temp`.
    #[cfg(feature = "try-runtime")]
    fn read_temp<T: Decode>(storage_key: &str) -> T {
        let full_key = storage_prefix(Self::MIGRATION_STORAGE_PREFIX, storage_key.as_bytes());
        sp_io::storage::get(&full_key)
            .and_then(|bytes| Decode::decode(&mut &*bytes).ok())
            .unwrap_or_else(|| panic!("No `{storage_key}` in the temp storage"))
    }
}

pub fn ensure_storage_version<P: PalletInfoAccess>(version: u16) -> Result<(), &'static str> {
    if StorageVersion::get::<P>() == StorageVersion::new(version) {
        Ok(())
    } else {
        Err("Bad storage version")
    }
}
