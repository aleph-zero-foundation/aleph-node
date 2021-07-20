use crate::{
    justification::AlephJustification,
    network::{Recipient, RmcNetwork},
    AuthorityKeystore, Signature,
};
use aleph_bft::{
    rmc::{DoublingDelayScheduler, Message, ReliableMulticast},
    MultiKeychain, NodeIndex, Signable, SignatureSet,
};
use aleph_primitives::ALEPH_ENGINE_ID;
use codec::{Codec, Decode, Encode};
use futures::{channel::mpsc, Stream, StreamExt};
use log::{debug, error, warn};
use sc_client_api::Backend;
use sp_api::{BlockId, NumberFor};
use sp_runtime::{
    traits::{Block, Header},
    Justification,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
    sync::Arc,
};
use tokio::time::Duration;

#[deny(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum Error {
    Internal(sp_blockchain::Error),
    MissingBlock,
}

pub(crate) fn finalize_block_as_authority<BE, B, C>(
    client: Arc<C>,
    h: B::Hash,
    auth_keystore: &AuthorityKeystore,
) -> core::result::Result<(), Error>
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let block_number = match client.number(h) {
        Ok(Some(number)) => number,
        _ => {
            error!(target: "afa", "a block with hash {} should already be in chain", h);
            return Err(Error::MissingBlock);
        }
    };
    finalize_block(
        client,
        h,
        block_number,
        Some((
            ALEPH_ENGINE_ID,
            AlephJustification::new::<B>(auth_keystore, h).encode(),
        )),
    )
}

pub(crate) fn finalize_block<BE, B, C>(
    client: Arc<C>,
    hash: B::Hash,
    block_number: NumberFor<B>,
    justification: Option<Justification>,
) -> core::result::Result<(), Error>
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let status = client.info();
    if status.finalized_number >= block_number {
        warn!(target: "afa", "trying to finalize a block with hash {} and number {}
               that is not greater than already finalized {}", hash, block_number, status.finalized_number);
        return Ok(());
    }

    debug!(target: "afa", "Finalizing block with hash {:?}. Previous best: #{:?}.", hash, status.finalized_number);

    let update_res = client.lock_import_and_run(|import_op| {
        // NOTE: all other finalization logic should come here, inside the lock
        client.apply_finality(import_op, BlockId::Hash(hash), justification, true)
    });

    let status = client.info();
    debug!(target: "afa", "Attempted to finalize block with hash {:?}. Current best: #{:?}.", hash, status.finalized_number);
    update_res.map_err(Error::Internal)
}

/// Given hashes `last_finalized` and `new_hash` of two block, returns
/// the sequence of headers of the blocks on the path from `last_finalized` to `new_hash`
/// excluding the header corresponding to `last_finalized`, or an empty sequence if
/// `new_hash` is not a descendant of `last_finalized`.
fn chain_extension_step<BE, B, C>(
    last_finalized: B::Hash,
    new_hash: B::Hash,
    client: &C,
) -> VecDeque<B::Header>
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    // this early return is for optimization reasons only.
    if new_hash == last_finalized {
        return VecDeque::new();
    }

    let last_finalized_number = match client.number(last_finalized) {
        Ok(Some(number)) => number,
        _ => {
            error!(target: "afa", "No block number for {}", last_finalized);
            return VecDeque::new();
        }
    };

    let mut extension = VecDeque::new();
    // iterate ancestors of `new_hash` and push their headers to the front of `extension`
    // until reaching a block with number <= last_finalized_number.
    let mut hash = new_hash;
    loop {
        let header = match client.header(BlockId::Hash(hash)) {
            Ok(Some(header)) => header,
            _ => {
                error!(target: "afa", "no header for hash {}", hash);
                return VecDeque::new();
            }
        };

        if header.number() <= &last_finalized_number {
            if hash != last_finalized {
                // `new_hash` is not an ancestor of `last_finalized`
                return VecDeque::new();
            }
            break;
        }
        hash = *header.parent_hash();
        extension.push_front(header);
    }
    extension
}

/// Given the hash of the last finalized block, transforms a nonempty stream of (arbitrary) block
/// hashes into a new chains by doing the following:
///  (1) greedily filters elements to form a chain consisting of distinct proper descendants of the last finalized block
///  (2) inserts missing elements so that each element is followed by its child
///  (3) stops at an element with number `max_h`
pub(crate) fn chain_extension<BE, B, C, St>(
    hashes: St,
    client: Arc<C>,
    max_h: NumberFor<B>,
) -> impl Stream<Item = B::Hash>
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
    St: Stream<Item = B::Hash> + Unpin,
{
    let mut last_finalized = client.info().finalized_hash;
    hashes
        .flat_map(move |new_hash| {
            let extension = chain_extension_step(last_finalized, new_hash, client.as_ref());
            if let Some(header) = extension.back() {
                last_finalized = header.hash();
            }
            futures::stream::iter(extension)
        })
        .take_while(move |header| std::future::ready(header.number() <= &max_h))
        .map(|header| header.hash())
}

#[derive(PartialEq, Eq, Hash, Clone, Debug, Encode, Decode)]
pub(crate) struct SignableHash<H: Codec> {
    hash: H,
}

impl<H: AsRef<[u8]> + Hash + Clone + Codec> Signable for SignableHash<H> {
    type Hash = H;
    fn hash(&self) -> Self::Hash {
        self.hash.clone()
    }
}

type RmcMessage<B> = Message<SignableHash<<B as Block>::Hash>, Signature, SignatureSet<Signature>>;
/// A wrapper around an RMC returning the signed hashes in the order of the [`ReliableMulticast::start_rmc`] calls.
pub(crate) struct BlockSignatureAggregator<'a, B: Block, MK: MultiKeychain> {
    messages_for_rmc: mpsc::UnboundedSender<RmcMessage<B>>,
    messages_from_rmc: mpsc::UnboundedReceiver<(NodeIndex, RmcMessage<B>)>,
    signatures: HashMap<B::Hash, MK::PartialMultisignature>,
    hash_queue: VecDeque<B::Hash>,
    network: RmcNetwork<B>,
    rmc: ReliableMulticast<'a, SignableHash<B::Hash>, MK>,
    finished: bool,
    started_hashes: HashSet<B::Hash>,
}

impl<
        'a,
        B: Block,
        MK: MultiKeychain<Signature = Signature, PartialMultisignature = SignatureSet<Signature>>,
    > BlockSignatureAggregator<'a, B, MK>
{
    pub(crate) fn new(network: RmcNetwork<B>, keychain: &'a MK) -> Self {
        let (messages_for_rmc, messages_from_network) = mpsc::unbounded();
        let (messages_for_network, messages_from_rmc) = mpsc::unbounded();
        let scheduler = DoublingDelayScheduler::new(Duration::from_millis(10));
        let rmc = ReliableMulticast::new(
            messages_from_network,
            messages_for_network,
            keychain,
            keychain.node_count(),
            scheduler,
        );
        BlockSignatureAggregator {
            messages_for_rmc,
            messages_from_rmc,
            signatures: HashMap::new(),
            hash_queue: VecDeque::new(),
            network,
            rmc,
            finished: false,
            started_hashes: HashSet::new(),
        }
    }

    pub(crate) async fn start_aggregation(&mut self, hash: B::Hash) {
        if !self.started_hashes.insert(hash) {
            return;
        }
        self.hash_queue.push_back(hash);
        self.rmc.start_rmc(SignableHash { hash }).await
    }

    pub(crate) async fn finish(&mut self) {
        self.finished = true;
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.finished
    }

    pub(crate) async fn next_multisigned_hash(
        &mut self,
    ) -> Option<(B::Hash, MK::PartialMultisignature)> {
        loop {
            match self.hash_queue.front() {
                Some(hash) => {
                    if let Some(multisignature) = self.signatures.remove(hash) {
                        let hash = self
                            .hash_queue
                            .pop_front()
                            .expect("VecDeque::front() returned Some(_), qed.");
                        return Some((hash, multisignature));
                    }
                }
                None => {
                    if self.finished {
                        return None;
                    }
                }
            }
            loop {
                tokio::select! {
                    multisigned_hash = self.rmc.next_multisigned_hash() => {
                            let unchecked = multisigned_hash.into_unchecked();
                            self.signatures
                                .insert(unchecked.signable.hash, unchecked.signature);
                            break;
                    }
                    message_from_rmc = self.messages_from_rmc.next() => {
                        if let Some((i, message_from_rmc)) = message_from_rmc {
                            self.network.send(message_from_rmc, Recipient::Target(i)).expect("sending message from rmc failed")
                        } else {
                            warn!(target: "afa", "the channel of messages from rmc closed");
                        }
                    }
                    message_from_network = self.network.next() => {
                        if let Some(message_from_network) = message_from_network {
                            self.messages_for_rmc.unbounded_send(message_from_network).expect("sending message to rmc failed");
                        } else {
                            warn!(target: "afa", "the network channel closed");}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_block_builder::BlockBuilderProvider;
    use sp_consensus::BlockOrigin;
    use substrate_test_runtime::Extrinsic;
    use substrate_test_runtime_client::{
        ClientBlockImportExt, ClientExt, DefaultTestClientBuilderExt, TestClient,
        TestClientBuilder, TestClientBuilderExt,
    };
    use tokio::stream::StreamExt;

    fn create_chain(client: &mut Arc<TestClient>, n: u64) -> Vec<sp_core::H256> {
        let mut blocks = vec![client.genesis_hash()];

        for _ in 1..=n {
            let block = client
                .new_block(Default::default())
                .unwrap()
                .build()
                .unwrap()
                .block;

            blocks.push(block.header.hash());
            futures::executor::block_on(client.import(BlockOrigin::Own, block)).unwrap();
        }

        blocks
    }

    #[test]
    fn chain_extenstion_step_for_descendant() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let n = 5;
        let blocks = create_chain(&mut client, n as u64);
        for i in 0..n {
            for j in i..n {
                let extension = chain_extension_step(blocks[i], blocks[j], client.as_ref());
                assert!(extension
                    .iter()
                    .map(|header| header.hash())
                    .eq(blocks[i + 1..j + 1].iter().cloned()));
            }
        }
    }

    #[test]
    fn chain_extenstion_step_for_non_descendant() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let n = 5;
        let blocks = create_chain(&mut client, n as u64);

        for i in 0..=n {
            for j in 0..i {
                let extension = chain_extension_step(blocks[i], blocks[j], client.as_ref());
                assert!(extension.is_empty());
            }
        }

        let extra_children: Vec<_> = blocks
            .iter()
            .map(|hash| {
                let mut builder = client
                    .new_block_at(&BlockId::Hash(*hash), Default::default(), false)
                    .unwrap();
                // Add a dummy extrinsic to make the block distinct from the one on chain
                builder
                    .push(Extrinsic::AuthoritiesChange(Vec::new()))
                    .unwrap();
                let block = builder.build().unwrap().block;
                let hash = block.header.hash();
                futures::executor::block_on(client.import(BlockOrigin::Own, block)).unwrap();
                hash
            })
            .collect();

        for i in 0..=n {
            for j in 0..=n {
                if i != j {
                    let extension =
                        chain_extension_step(extra_children[i], extra_children[j], client.as_ref());
                    assert!(extension.is_empty());
                }
            }
        }
    }

    #[tokio::test]
    async fn chain_extension_scenario() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let n = 5;
        let blocks = create_chain(&mut client, n as u64);

        let extra_children: Vec<_> = blocks
            .iter()
            .map(|hash| {
                let mut builder = client
                    .new_block_at(&BlockId::Hash(*hash), Default::default(), false)
                    .unwrap();
                // Add a dummy extrinsic to make the block distinct from the one on chain
                builder
                    .push(Extrinsic::AuthoritiesChange(Vec::new()))
                    .unwrap();
                let block = builder.build().unwrap().block;
                let hash = block.header.hash();
                futures::executor::block_on(client.import(BlockOrigin::Own, block)).unwrap();
                hash
            })
            .collect();

        let hashes = vec![
            blocks[0], //ignored
            blocks[1],
            blocks[1],         // ignored
            extra_children[0], // ignored
            blocks[3],
            blocks[3],         // ignored
            extra_children[2], //ignored
            extra_children[2], //ignored
            blocks[0],         // ignored
            blocks[5],
            extra_children[4], // ignored
        ];
        let extension: Vec<_> = chain_extension(futures::stream::iter(hashes), client, 4)
            .collect()
            .await;
        assert!(extension.iter().eq(blocks[1..=4].iter()));
    }
}
