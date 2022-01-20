use crate::network::{manager::Multiaddr, PeerId};
use sc_network::{multiaddr::Protocol, PeerId as ScPeerId};

/// Checks whether the given Multiaddr contains a libp2p component.
pub fn is_p2p(address: &Multiaddr) -> bool {
    address
        .0
        .iter()
        .any(|protocol| peer_id(&protocol).is_some())
}

enum UniquePeerId {
    Unique(PeerId),
    NotUnique,
    Unknown,
}

impl UniquePeerId {
    fn into_option(self) -> Option<PeerId> {
        use UniquePeerId::*;
        match self {
            Unique(peer_id) => Some(peer_id),
            _ => None,
        }
    }

    fn accumulate(self, maybe_peer_id: Option<PeerId>) -> UniquePeerId {
        use UniquePeerId::*;
        match self {
            Unique(old_peer_id) => match maybe_peer_id {
                Some(peer_id) => {
                    if peer_id == old_peer_id {
                        self
                    } else {
                        NotUnique
                    }
                }
                None => self,
            },
            NotUnique => NotUnique,
            Unknown => match maybe_peer_id {
                Some(peer_id) => Unique(peer_id),
                None => Unknown,
            },
        }
    }

    fn accumulate_strict(self, maybe_peer_id: Option<PeerId>) -> UniquePeerId {
        use UniquePeerId::*;
        match maybe_peer_id {
            Some(peer_id) => match self {
                Unique(old_peer_id) => {
                    if peer_id == old_peer_id {
                        self
                    } else {
                        NotUnique
                    }
                }
                NotUnique => NotUnique,
                Unknown => Unique(peer_id),
            },
            None => NotUnique,
        }
    }
}

fn peer_id(protocol: &Protocol<'_>) -> Option<PeerId> {
    match protocol {
        Protocol::P2p(hashed_peer_id) => ScPeerId::from_multihash(*hashed_peer_id).ok().map(PeerId),
        _ => None,
    }
}

/// Returns the peer id associated with this multiaddress if it exists and is unique.
pub fn get_peer_id(address: &Multiaddr) -> Option<PeerId> {
    address
        .0
        .iter()
        .fold(UniquePeerId::Unknown, |result, protocol| {
            result.accumulate(peer_id(&protocol))
        })
        .into_option()
}

/// Returns the peer id contained in the set of multiaddresses if it's unique and present in every
/// address, None otherwise.
pub fn get_common_peer_id(addresses: &[Multiaddr]) -> Option<PeerId> {
    addresses
        .iter()
        .fold(UniquePeerId::Unknown, |result, address| {
            result.accumulate_strict(get_peer_id(address))
        })
        .into_option()
}

/// Returns the address extended by the peer id, unless it already contained another peer id.
pub fn add_matching_peer_id(mut address: Multiaddr, peer_id: PeerId) -> Option<Multiaddr> {
    match get_peer_id(&address) {
        Some(peer) => match peer == peer_id {
            true => Some(address),
            false => None,
        },
        None => {
            address.0.push(Protocol::P2p(peer_id.0.into()));
            Some(address)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{add_matching_peer_id, get_common_peer_id, get_peer_id, is_p2p};
    use crate::network::manager::testing::address;

    #[test]
    fn non_p2p_addresses_are_not_p2p() {
        assert!(!is_p2p(&address("/dns4/example.com/udt/sctp/5678").into()));
    }

    #[test]
    fn p2p_addresses_are_p2p() {
        assert!(is_p2p(&address("/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L").into()));
    }

    #[test]
    fn no_addresses_have_no_peer_id() {
        assert!(get_common_peer_id(&[]).is_none());
    }

    #[test]
    fn non_p2p_addresses_have_no_peer_id() {
        assert!(get_common_peer_id(&[
            address("/dns4/example.com/udt/sctp/5678").into(),
            address("/ip4/81.6.39.166/udt/sctp/5678").into(),
        ])
        .is_none());
    }

    #[test]
    fn p2p_addresses_with_common_peer_id_have_unique_peer_id() {
        assert!(get_common_peer_id(&[
                address("/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L").into(),
                address("/dns4/peer.example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L").into(),
        ]).is_some());
    }

    #[test]
    fn mixed_addresses_have_no_unique_peer_id() {
        assert!(get_common_peer_id(&[
                address("/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L").into(),
                address("/dns4/peer.example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L").into(),
                address("/dns4/example.com/udt/sctp/5678").into(),
                address("/ip4/81.6.39.166/udt/sctp/5678").into(),
        ]).is_none());
    }

    #[test]
    fn p2p_addresses_with_differing_peer_ids_have_no_unique_peer_id() {
        assert!(get_common_peer_id(&[
                address("/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L").into(),
                address("/dns4/peer.example.com/tcp/30333/p2p/12D3KooWFVXnvJdPuGnGYMPn5qLQAQYwmRBgo6SmEQsKZSrDoo2k").into(),
        ]).is_none());
    }

    #[test]
    fn non_p2p_address_matches_peer_id() {
        let address = address(
            "/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L",
        )
        .into();
        let peer_id = get_peer_id(&address).unwrap();
        let mut peerless_address = address.clone().0;
        peerless_address.pop();
        let peerless_address = peerless_address.into();
        assert!(get_peer_id(&peerless_address).is_none());
        assert_eq!(
            add_matching_peer_id(peerless_address, peer_id),
            Some(address)
        );
    }

    #[test]
    fn p2p_address_matches_own_peer_id() {
        let address = address(
            "/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L",
        )
        .into();
        let peer_id = get_peer_id(&address).unwrap();
        assert_eq!(
            &add_matching_peer_id(address.clone(), peer_id),
            &Some(address)
        );
    }

    #[test]
    fn p2p_address_does_not_match_other_peer_id() {
        let nonmatching_address = address(
            "/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L",
        )
        .into();
        let peer_id = get_peer_id(&address("/dns4/peer.example.com/tcp/30333/p2p/12D3KooWFVXnvJdPuGnGYMPn5qLQAQYwmRBgo6SmEQsKZSrDoo2k").into()).unwrap();
        assert!(add_matching_peer_id(nonmatching_address, peer_id).is_none());
    }
}
