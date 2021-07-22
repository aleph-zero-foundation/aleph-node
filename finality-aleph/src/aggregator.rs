use crate::{
    network::{Recipient, RmcNetwork},
    Signature,
};
use aleph_bft::{
    rmc::{DoublingDelayScheduler, Message, ReliableMulticast},
    MultiKeychain, Signable, SignatureSet,
};
use codec::{Codec, Decode, Encode};
use futures::{channel::mpsc, StreamExt};
use log::{debug, trace, warn};
use sp_runtime::traits::Block;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
};
use tokio::time::Duration;

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
    messages_from_rmc: mpsc::UnboundedReceiver<RmcMessage<B>>,
    signatures: HashMap<B::Hash, MK::PartialMultisignature>,
    hash_queue: VecDeque<B::Hash>,
    network: RmcNetwork<B>,
    rmc: ReliableMulticast<'a, SignableHash<B::Hash>, MK>,
    last_hash_placed: bool,
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
        let scheduler = DoublingDelayScheduler::new(Duration::from_millis(500));
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
            last_hash_placed: false,
            started_hashes: HashSet::new(),
        }
    }

    pub(crate) async fn start_aggregation(&mut self, hash: B::Hash) {
        debug!(target: "afa", "Started aggregation for block hash {:?}", hash);
        if !self.started_hashes.insert(hash) {
            return;
        }
        self.hash_queue.push_back(hash);
        self.rmc.start_rmc(SignableHash { hash }).await;
    }

    pub(crate) fn notify_last_hash(&mut self) {
        self.last_hash_placed = true;
    }

    pub(crate) async fn next_multisigned_hash(
        &mut self,
    ) -> Option<(B::Hash, MK::PartialMultisignature)> {
        loop {
            trace!(target: "afa", "Entering next_multisigned_hash loop.");
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
                    if self.last_hash_placed {
                        debug!(target: "afa", "Terminating next_multisigned_hash because the last hash has been signed.");
                        return None;
                    }
                }
            }
            loop {
                tokio::select! {
                    multisigned_hash = self.rmc.next_multisigned_hash() => {
                        let hash = multisigned_hash.as_signable().hash;
                        let unchecked = multisigned_hash.into_unchecked().signature();
                            debug!(target: "afa", "New multisigned_hash {:?}.", unchecked);
                            self.signatures.insert(hash, unchecked);
                            break;
                    }
                    message_from_rmc = self.messages_from_rmc.next() => {
                        trace!(target: "afa", "Our rmc message {:?}.", message_from_rmc);
                        if let Some(message_from_rmc) = message_from_rmc {
                            self.network.send(message_from_rmc, Recipient::All).expect("sending message from rmc failed")
                        } else {
                            warn!(target: "afa", "the channel of messages from rmc closed");
                        }
                    }
                    message_from_network = self.network.next() => {
                        if let Some(message_from_network) = message_from_network {
                            trace!(target: "afa", "Received message for rmc: {:?}", message_from_network);
                            self.messages_for_rmc.unbounded_send(message_from_network).expect("sending message to rmc failed");
                        } else {
                            warn!(target: "afa", "the network channel closed");
                            // In case the network is down we can terminate (?).
                            return None;
                        }
                    }
                }
            }
        }
    }
}
