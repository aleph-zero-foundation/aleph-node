use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};

pub mod v0_to_v1;
pub mod v1_to_v2;
pub mod v2_to_v3;

/// In order to run both pre- and post- checks around every migration, we entangle methods of
/// `OnRuntimeUpgrade` into the desired flow and expose it with `migrate` method.
///
/// This way, `try-runtime` no longer triggers checks. We do it by hand.
pub trait StorageMigration: OnRuntimeUpgrade {
    #[allow(clippy::let_and_return)]
    fn migrate() -> Weight {
        #[cfg(feature = "try-runtime")]
        Self::pre_upgrade().expect("Pre upgrade should succeed");

        let weight = Self::on_runtime_upgrade();

        #[cfg(feature = "try-runtime")]
        Self::post_upgrade().expect("Post upgrade should succeed");

        weight
    }
}
