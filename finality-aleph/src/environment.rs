use crate::{
    communication::network::{Network, NetworkBridge, NetworkError, NotificationOutSender},
    NodeId,
};
use futures::channel::mpsc;
use log::{debug, error};
use rush::{Hashing, NotificationIn};
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_core::{blake2_256, H256};
use sp_runtime::{
    generic::BlockId,
    traits::{Block, Header},
};
use std::{marker::PhantomData, sync::Arc};

pub(crate) struct Environment<B: Block, N: Network<B>, C, BE, SC> {
    pub(crate) client: Arc<C>,
    pub(crate) network: NetworkBridge<B, H256, N>,
    pub(crate) select_chain: SC,
    pub(crate) _phantom: std::marker::PhantomData<(B, BE)>,
}

impl<B, N, C, BE, SC> Environment<B, N, C, BE, SC>
where
    B: Block,
    N: Network<B>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    pub fn new(client: Arc<C>, network: NetworkBridge<B, H256, N>, select_chain: SC) -> Self {
        Environment {
            client,
            network,
            select_chain,
            _phantom: PhantomData,
        }
    }
}

impl<B, N, C, BE, SC> rush::Environment for Environment<B, N, C, BE, SC>
where
    B: Block,
    N: Network<B>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    type NodeId = NodeId;
    type Hash = H256;
    type BlockHash = B::Hash;
    type InstanceId = H256;

    type Crypto = ();
    type In = mpsc::UnboundedReceiver<NotificationIn<Self::BlockHash, Self::Hash>>;
    type Out = NotificationOutSender<B, Self::Hash>;
    type Error = NetworkError;

    fn finalize_block(&self, h: Self::BlockHash) {
        finalize_block(self.client.clone(), h);
    }

    fn check_extends_finalized(&self, h: Self::BlockHash) -> bool {
        let head_finalized = self.client.info().finalized_hash;
        if h == head_finalized {
            return false;
        }
        let lca = sp_blockchain::lowest_common_ancestor(self.client.as_ref(), h, head_finalized)
            .expect("No lowest common ancestor");
        lca.hash == head_finalized
    }

    fn best_block(&self) -> Self::BlockHash {
        self.select_chain
            .best_chain()
            .expect("No best chain")
            .hash()
    }

    fn consensus_data(&self) -> (Self::Out, Self::In) {
        self.network.communication()
    }

    fn hashing() -> Hashing<Self::Hash> {
        Box::new(|data| blake2_256(data).into())
    }
}

pub(crate) fn finalize_block<BE, B, C>(client: Arc<C>, hash: B::Hash)
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let block_number = match client.number(hash) {
        Ok(Some(number)) => number,
        _ => {
            error!(target: "env", "a block with hash {} should already be in chain", hash);
            return;
        }
    };
    let info = client.info();

    if info.finalized_number >= block_number {
        error!(target: "env", "trying to finalized a block with hash {} and number {}
               that is not greater than already finalized {}", hash, block_number, info.finalized_number);
        return;
    }

    let status = client.info();
    debug!(target: "env", "Finalizing block with hash {:?}. Previous best: #{:?}.", hash, status.finalized_number);

    let _update_res = client.lock_import_and_run(|import_op| {
        // NOTE: all other finalization logic should come here, inside the lock
        client.apply_finality(import_op, BlockId::Hash(hash), None, true)
    });

    let status = client.info();
    debug!(target: "env", "Finalized block with hash {:?}. Current best: #{:?}.", hash,status.finalized_number);
}
