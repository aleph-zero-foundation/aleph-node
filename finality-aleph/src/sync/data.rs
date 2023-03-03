use std::{collections::HashSet, marker::PhantomData, mem::size_of};

use aleph_primitives::MAX_BLOCK_SIZE;
use codec::{Decode, Encode, Error as CodecError, Input as CodecInput};
use log::warn;

use crate::{
    network::GossipNetwork,
    sync::{BlockIdFor, Justification, LOG_TARGET},
    Version,
};

/// The representation of the database state to be sent to other nodes.
/// In the first version this only contains the top justification.
#[derive(Clone, Debug, Encode, Decode)]
pub struct State<J: Justification> {
    top_justification: J::Unverified,
}

impl<J: Justification> State<J> {
    pub fn new(top_justification: J::Unverified) -> Self {
        State { top_justification }
    }

    pub fn top_justification(&self) -> J::Unverified {
        self.top_justification.clone()
    }
}

/// Data to be sent over the network.
#[derive(Clone, Debug, Encode, Decode)]
pub enum NetworkData<J: Justification> {
    /// A periodic state broadcast, so that neighbouring nodes can request what they are missing,
    /// send what we are missing, and sometines just use the justifications to update their own
    /// state.
    StateBroadcast(State<J>),
    /// A series of justifications, sent to a node that is clearly behind.
    Justifications(Vec<J::Unverified>, State<J>),
    /// An explicit request for data, potentially a lot of it.
    Request(BlockIdFor<J>, State<J>),
}

/// Version wrapper around the network data.
#[derive(Clone, Debug)]
pub enum VersionedNetworkData<J: Justification> {
    // Most likely from the future.
    Other(Version, Vec<u8>),
    V1(NetworkData<J>),
}

// We need 32 bits, since blocks can be quite sizeable.
type ByteCount = u32;

// We want to be able to safely send at least 10 blocks at once, so this gives uss a bit of wiggle
// room.
const MAX_SYNC_MESSAGE_SIZE: u32 = MAX_BLOCK_SIZE * 11;

fn encode_with_version(version: Version, payload: &[u8]) -> Vec<u8> {
    let size = payload.len().try_into().unwrap_or(ByteCount::MAX);

    if size > MAX_SYNC_MESSAGE_SIZE {
        warn!(
            target: LOG_TARGET,
            "Versioned sync message v{:?} too big during Encode. Size is {:?}. Should be {:?} at max.",
            version,
            payload.len(),
            MAX_SYNC_MESSAGE_SIZE
        );
    }

    let mut result = Vec::with_capacity(version.size_hint() + size.size_hint() + payload.len());

    version.encode_to(&mut result);
    size.encode_to(&mut result);
    result.extend_from_slice(payload);

    result
}

impl<J: Justification> Encode for VersionedNetworkData<J> {
    fn size_hint(&self) -> usize {
        use VersionedNetworkData::*;
        let version_size = size_of::<Version>();
        let byte_count_size = size_of::<ByteCount>();
        version_size
            + byte_count_size
            + match self {
                Other(_, payload) => payload.len(),
                V1(data) => data.size_hint(),
            }
    }

    fn encode(&self) -> Vec<u8> {
        use VersionedNetworkData::*;
        match self {
            Other(version, payload) => encode_with_version(*version, payload),
            V1(data) => encode_with_version(Version(1), &data.encode()),
        }
    }
}

impl<J: Justification> Decode for VersionedNetworkData<J> {
    fn decode<I: CodecInput>(input: &mut I) -> Result<Self, CodecError> {
        use VersionedNetworkData::*;
        let version = Version::decode(input)?;
        let num_bytes = ByteCount::decode(input)?;
        match version {
            Version(1) => Ok(V1(NetworkData::decode(input)?)),
            _ => {
                if num_bytes > MAX_SYNC_MESSAGE_SIZE {
                    Err("Sync message has unknown version and is encoded as more than the maximum size.")?;
                };
                let mut payload = vec![0; num_bytes as usize];
                input.read(payload.as_mut_slice())?;
                Ok(Other(version, payload))
            }
        }
    }
}

/// Wrap around a network to avoid thinking about versioning.
pub struct VersionWrapper<J: Justification, N: GossipNetwork<VersionedNetworkData<J>>> {
    inner: N,
    _phantom: PhantomData<J>,
}

impl<J: Justification, N: GossipNetwork<VersionedNetworkData<J>>> VersionWrapper<J, N> {
    /// Wrap the inner network.
    pub fn new(inner: N) -> Self {
        VersionWrapper {
            inner,
            _phantom: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<J: Justification, N: GossipNetwork<VersionedNetworkData<J>>> GossipNetwork<NetworkData<J>>
    for VersionWrapper<J, N>
{
    type Error = N::Error;
    type PeerId = N::PeerId;

    fn send_to(&mut self, data: NetworkData<J>, peer_id: Self::PeerId) -> Result<(), Self::Error> {
        self.inner.send_to(VersionedNetworkData::V1(data), peer_id)
    }

    fn send_to_random(
        &mut self,
        data: NetworkData<J>,
        peer_ids: HashSet<Self::PeerId>,
    ) -> Result<(), Self::Error> {
        self.inner
            .send_to_random(VersionedNetworkData::V1(data), peer_ids)
    }

    fn broadcast(&mut self, data: NetworkData<J>) -> Result<(), Self::Error> {
        self.inner.broadcast(VersionedNetworkData::V1(data))
    }

    async fn next(&mut self) -> Result<(NetworkData<J>, Self::PeerId), Self::Error> {
        loop {
            match self.inner.next().await? {
                (VersionedNetworkData::Other(version, _), _) => warn!(target: LOG_TARGET, "Received sync data of unsupported version {:?}, this node might be running outdated software.", version),
                (VersionedNetworkData::V1(data), peer_id) => return Ok((data, peer_id)),
            }
        }
    }
}
