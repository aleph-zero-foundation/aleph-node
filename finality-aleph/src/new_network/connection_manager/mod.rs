use crate::{crypto::Signature, NodeIndex, SessionId};
use codec::{Decode, Encode};
use sc_network::Multiaddr as ScMultiaddr;
use std::convert::TryFrom;

mod addresses;
mod session;

use addresses::{get_common_peer_id, is_p2p};

/// A wrapper for the Substrate multiaddress to allow encoding & decoding.
#[derive(Clone, Debug)]
pub struct Multiaddr(pub(crate) ScMultiaddr);

impl From<ScMultiaddr> for Multiaddr {
    fn from(addr: ScMultiaddr) -> Self {
        Multiaddr(addr)
    }
}

impl Encode for Multiaddr {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.0.to_vec().using_encoded(f)
    }
}

impl Decode for Multiaddr {
    fn decode<I: codec::Input>(value: &mut I) -> Result<Self, codec::Error> {
        let bytes = Vec::<u8>::decode(value)?;
        ScMultiaddr::try_from(bytes)
            .map_err(|_| "Multiaddr not encoded as bytes".into())
            .map(|multiaddr| multiaddr.into())
    }
}

/// Data validators use to authenticate themselves for a single session
/// and disseminate their addresses.
#[derive(Clone, Debug, Encode, Decode)]
pub struct AuthData {
    addresses: Vec<Multiaddr>,
    node_id: NodeIndex,
    session_id: SessionId,
}

/// A full authentication, consisting of a signed AuthData.
pub type Authentication = (AuthData, Signature);

#[cfg(test)]
mod test {
    use super::{Multiaddr, ScMultiaddr};
    use codec::{Decode, Encode};

    #[test]
    fn multiaddr_encode_decode() {
        let sc_multiaddr: ScMultiaddr =
            "/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L"
                .parse()
                .unwrap();
        let multiaddr: Multiaddr = sc_multiaddr.into();
        assert_eq!(
            Multiaddr::decode(&mut &multiaddr.encode()[..])
                .unwrap()
                .encode(),
            multiaddr.encode()
        );
    }
}
