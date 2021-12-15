use crate::new_network::{manager::Multiaddr, PeerId};
use ip_network::IpNetwork;
use sc_network::{multiaddr::Protocol, PeerId as ScPeerId};

/// Checks whether the given Multiaddr is globally accessible.
pub fn is_global(address: &Multiaddr) -> bool {
    address.0.iter().all(|protocol| match protocol {
        Protocol::Ip4(ip) => IpNetwork::from(ip).is_global(),
        Protocol::Ip6(ip) => IpNetwork::from(ip).is_global(),
        _ => true,
    })
}

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

#[cfg(test)]
mod tests {
    use super::{get_common_peer_id, is_global, is_p2p};
    use crate::new_network::manager::testing::address;

    #[test]
    fn local_addresses_are_not_global() {
        assert!(!is_global(&address("/ip4/127.0.0.1/udt/sctp/5678").into()));
    }

    #[test]
    fn global_addresses_are_global() {
        assert!(is_global(&address("/ip4/81.6.39.166/udt/sctp/5678").into()));
    }

    #[test]
    fn dns_addresses_are_global() {
        assert!(is_global(
            &address("/dns4/example.com/udt/sctp/5678").into()
        ));
    }

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
}
