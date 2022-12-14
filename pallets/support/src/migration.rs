use frame_support::{
    pallet_prelude::{PalletInfoAccess, StorageVersion, Weight},
    traits::OnRuntimeUpgrade,
};

/// In order to run both pre- and post- checks around every migration, we entangle methods of
/// `OnRuntimeUpgrade` into the desired flow and expose it with `migrate` method.
///
/// This way, `try-runtime` no longer triggers checks. We do it by hand.
pub trait StorageMigration: OnRuntimeUpgrade {
    #[allow(clippy::let_and_return)]
    fn migrate() -> Weight {
        #[cfg(feature = "try-runtime")]
        let state = Self::pre_upgrade().expect("Pre upgrade should succeed");

        let weight = Self::on_runtime_upgrade();

        #[cfg(feature = "try-runtime")]
        Self::post_upgrade(state).expect("Post upgrade should succeed");

        weight
    }
}

impl<T: OnRuntimeUpgrade> StorageMigration for T {}

/// Ensure that the current pallet storage version matches `version`.
pub fn ensure_storage_version<P: PalletInfoAccess>(version: u16) -> Result<(), &'static str> {
    if StorageVersion::get::<P>() == StorageVersion::new(version) {
        Ok(())
    } else {
        Err("Bad storage version")
    }
}
