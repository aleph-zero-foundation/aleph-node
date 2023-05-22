use frame_support::{
    log,
    sp_runtime::{traits::OpaqueKeys, RuntimeAppPublic},
};
use primitives::AuthorityId;
use sp_std::prelude::*;

use crate::Config;

/// Authorities provider, used only as default value in case of missing this information in our pallet. This can
/// happen for the session after runtime upgraded.
pub trait NextSessionAuthorityProvider<T: Config> {
    fn next_authorities() -> Vec<T::AuthorityId>;
}

impl<T> NextSessionAuthorityProvider<T> for pallet_session::Pallet<T>
where
    T: Config + pallet_session::Config,
{
    fn next_authorities() -> Vec<T::AuthorityId> {
        let next: Option<Vec<_>> = pallet_session::Pallet::<T>::queued_keys()
            .iter()
            .map(|(_, key)| key.get(AuthorityId::ID))
            .collect();

        next.unwrap_or_else(|| {
            log::error!(target: "pallet_aleph", "Missing next session keys");
            vec![]
        })
    }
}
