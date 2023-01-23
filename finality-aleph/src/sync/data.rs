use std::mem::size_of;

use aleph_primitives::MAX_BLOCK_SIZE;
use codec::{Decode, Encode, Error as CodecError, Input as CodecInput};
use log::warn;

use crate::{sync::Justification, Version};

/// The representation of the database state to be sent to other nodes.
/// In the first version this only contains the top justification.
#[derive(Clone, Debug, Encode, Decode)]
pub struct State<J: Justification> {
    top_justification: J::Unverified,
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
