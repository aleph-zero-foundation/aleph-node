use std::iter;

use array_bytes::bytes2hex;
use sc_network::{
    config::{NonDefaultSetConfig, NonReservedPeerMode, NotificationHandshake, Role, SetConfig},
    NotificationService,
};
use sc_network_common::sync::message::BlockAnnouncesHandshake;
use sp_core::H256;
use sp_runtime::traits::{Block, Header};

use crate::{BlockHash, BlockNumber};

// NOTE: `set_config` will be ignored by `protocol.rs` as the base
// protocol is still hardcoded into the peerset.
const DUMMY_SET_CONFIG: SetConfig = SetConfig {
    in_peers: 0,
    out_peers: 0,
    reserved_nodes: Vec::new(),
    non_reserved_mode: NonReservedPeerMode::Deny,
};

/// Generate a config for the base protocol and the notification service that should be passed to its service.
pub fn setup<B>(genesis_hash: B::Hash) -> (NonDefaultSetConfig, Box<dyn NotificationService>)
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
{
    // used for backwards compatibility with older nodes, should be safe to remove after update 14
    let legacy_block_announces_protocol =
        format!("/{}/block-announces/1", bytes2hex("", genesis_hash));
    let base_protocol_name = format!("/{}/base-protocol/1", bytes2hex("", genesis_hash));

    NonDefaultSetConfig::new(
        base_protocol_name.into(),
        iter::once(legacy_block_announces_protocol.into()).collect(),
        // This is the maximum message size. We don't need messages at all,
        // but we want to avoid tripping some magic value,
        // which 0 might suddenly become, so 1.
        1,
        Some(NotificationHandshake::new(
            BlockAnnouncesHandshake::<B>::build(
                // All nodes are full nodes.
                (&Role::Full).into(),
                // The best block number, always send a dummy value of 0.
                0,
                // The best block hash, always an obviously dummy value.
                H256([0; 32]),
                genesis_hash,
            ),
        )),
        DUMMY_SET_CONFIG,
    )
}
