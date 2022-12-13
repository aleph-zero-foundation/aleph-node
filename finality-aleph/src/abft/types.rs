//! Types common for current & legacy abft used across finality-aleph

use codec::{Decode, Encode, Error, Input, Output};
use derive_more::{From, Into};

/// The index of a node
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, From, Into)]
pub struct NodeIndex(pub usize);

impl Encode for NodeIndex {
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        (self.0 as u64).encode_to(dest);
    }
}

impl Decode for NodeIndex {
    fn decode<I: Input>(value: &mut I) -> Result<Self, Error> {
        Ok(NodeIndex(u64::decode(value)? as usize))
    }
}

/// Node count. Right now it doubles as node weight in many places in the code, in the future we
/// might need a new type for that.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, From, Into)]
pub struct NodeCount(pub usize);

/// A recipient of a message, either a specific node or everyone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Recipient {
    Everyone,
    Node(NodeIndex),
}

impl From<legacy_aleph_bft::Recipient> for Recipient {
    fn from(recipient: legacy_aleph_bft::Recipient) -> Self {
        match recipient {
            legacy_aleph_bft::Recipient::Everyone => Recipient::Everyone,
            legacy_aleph_bft::Recipient::Node(id) => Recipient::Node(id.into()),
        }
    }
}

impl From<current_aleph_bft::Recipient> for Recipient {
    fn from(recipient: current_aleph_bft::Recipient) -> Self {
        match recipient {
            current_aleph_bft::Recipient::Everyone => Recipient::Everyone,
            current_aleph_bft::Recipient::Node(id) => Recipient::Node(id.into()),
        }
    }
}

impl From<NodeCount> for current_aleph_bft::NodeCount {
    fn from(count: NodeCount) -> Self {
        current_aleph_bft::NodeCount(count.0)
    }
}
impl From<NodeCount> for legacy_aleph_bft::NodeCount {
    fn from(count: NodeCount) -> Self {
        legacy_aleph_bft::NodeCount(count.0)
    }
}

impl From<legacy_aleph_bft::NodeCount> for NodeCount {
    fn from(count: legacy_aleph_bft::NodeCount) -> Self {
        Self(count.0)
    }
}

impl From<current_aleph_bft::NodeCount> for NodeCount {
    fn from(count: current_aleph_bft::NodeCount) -> Self {
        Self(count.0)
    }
}

impl From<NodeIndex> for current_aleph_bft::NodeIndex {
    fn from(idx: NodeIndex) -> Self {
        current_aleph_bft::NodeIndex(idx.0)
    }
}

impl From<NodeIndex> for legacy_aleph_bft::NodeIndex {
    fn from(idx: NodeIndex) -> Self {
        legacy_aleph_bft::NodeIndex(idx.0)
    }
}

impl From<legacy_aleph_bft::NodeIndex> for NodeIndex {
    fn from(idx: legacy_aleph_bft::NodeIndex) -> Self {
        Self(idx.0)
    }
}

impl From<current_aleph_bft::NodeIndex> for NodeIndex {
    fn from(idx: current_aleph_bft::NodeIndex) -> Self {
        Self(idx.0)
    }
}

impl From<Recipient> for current_aleph_bft::Recipient {
    fn from(recipient: Recipient) -> Self {
        match recipient {
            Recipient::Everyone => current_aleph_bft::Recipient::Everyone,
            Recipient::Node(idx) => current_aleph_bft::Recipient::Node(idx.into()),
        }
    }
}

impl From<Recipient> for legacy_aleph_bft::Recipient {
    fn from(recipient: Recipient) -> Self {
        match recipient {
            Recipient::Everyone => legacy_aleph_bft::Recipient::Everyone,
            Recipient::Node(idx) => legacy_aleph_bft::Recipient::Node(idx.into()),
        }
    }
}
