//! Main purpose of this module is to be able to use two different versions of the abft crate.
//! Older version is referred to as 'Legacy' while newer as 'Current'.
//! We achieve this by hiding types & traits from abft crates behind our owns. In case of traits we
//! implement both current and legacy ones. In case of types we implement trait `From` to be able
//! convert them at the 'glueing' spot to the abft library. Current and legacy versions are marked
//! by numbers. Whenever we upgrade to next version of abft we need to increment and mark each version
//! version accordingly.

mod common;
mod crypto;
mod current;
mod legacy;
mod network;
mod traits;
mod types;

use std::fmt::Debug;

use aleph_bft_crypto::{PartialMultisignature, Signature};
use codec::{Decode, Encode};
pub use crypto::Keychain;
pub use current::{
    create_aleph_config as current_create_aleph_config, run_member as run_current_member,
    VERSION as CURRENT_VERSION,
};
pub use legacy::{
    create_aleph_config as legacy_create_aleph_config, run_member as run_legacy_member,
    VERSION as LEGACY_VERSION,
};
pub use network::{CurrentNetworkData, LegacyNetworkData, NetworkWrapper};
pub use traits::{Hash, SpawnHandle, SpawnHandleT, Wrapper as HashWrapper};
pub use types::{NodeCount, NodeIndex, Recipient};

/// Wrapper for `SignatureSet` to be able to implement both legacy and current `PartialMultisignature` trait.
/// Inner `SignatureSet` is imported from `aleph_bft_crypto` with fixed version for compatibility reasons:
/// this is also used in the justification which already exist in our chain history and we
/// need to be careful with changing this.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Encode, Decode)]
pub struct SignatureSet<Signature>(pub aleph_bft_crypto::SignatureSet<Signature>);

impl<S: Clone> SignatureSet<S> {
    pub fn size(&self) -> NodeCount {
        self.0.size().into()
    }

    pub fn with_size(len: NodeCount) -> Self {
        SignatureSet(legacy_aleph_bft::SignatureSet::with_size(len.into()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeIndex, &S)> {
        self.0.iter().map(|(idx, s)| (idx.into(), s))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (NodeIndex, &mut S)> {
        self.0.iter_mut().map(|(idx, s)| (idx.into(), s))
    }

    pub fn add_signature(self, signature: &S, index: NodeIndex) -> Self
    where
        S: Signature,
    {
        SignatureSet(self.0.add_signature(signature, index.into()))
    }
}

impl<S: 'static> IntoIterator for SignatureSet<S> {
    type Item = (NodeIndex, S);
    type IntoIter = Box<dyn Iterator<Item = (NodeIndex, S)>>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.0.into_iter().map(|(idx, s)| (idx.into(), s)))
    }
}

impl<S: legacy_aleph_bft::Signature> legacy_aleph_bft::PartialMultisignature for SignatureSet<S> {
    type Signature = S;

    fn add_signature(
        self,
        signature: &Self::Signature,
        index: legacy_aleph_bft::NodeIndex,
    ) -> Self {
        SignatureSet::add_signature(self, signature, index.into())
    }
}

impl<S: legacy_aleph_bft::Signature> current_aleph_bft::PartialMultisignature for SignatureSet<S> {
    type Signature = S;

    fn add_signature(
        self,
        signature: &Self::Signature,
        index: current_aleph_bft::NodeIndex,
    ) -> Self {
        SignatureSet::add_signature(self, signature, index.into())
    }
}
