use futures::{Sink, Stream};
use log::debug;
use sc_client_api::backend::Backend;

use sp_consensus::SelectChain;
use sp_core::{blake2_256, H256};
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Header as HeaderT},
};
use std::{marker::PhantomData, sync::Arc};

use rush::{Hashing, NotificationIn, NotificationOut};

use crate::NodeId;

pub struct Environment<C, N, B: BlockT, BE, SC> {
    pub(crate) client: Arc<C>,
    pub(crate) network: N,
    pub(crate) select_chain: SC,
    pub(crate) _phantom_block: std::marker::PhantomData<B>,
    pub(crate) _phantom_backend: std::marker::PhantomData<BE>,
}

impl<C, N, B: BlockT, BE, SC> Environment<C, N, B, BE, SC>
where
    B: BlockT,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    N: Send + Sync + 'static,
{
    pub fn new(client: Arc<C>, network: N, select_chain: SC) -> Self {
        Environment {
            client,
            network,
            select_chain,
            _phantom_block: PhantomData,
            _phantom_backend: PhantomData,
        }
    }
}

impl<C, N, B: BlockT, BE, SC> rush::Environment for Environment<C, N, B, BE, SC>
where
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
    SC: SelectChain<B> + 'static,
{
    type NodeId = NodeId;
    type Hash = H256;
    type BlockHash = B::Hash;
    type InstanceId = H256;

    type Crypto = ();
    type In = Box<dyn Stream<Item = NotificationIn<Self::BlockHash, Self::Hash>> + Send + Unpin>;
    type Out = Box<
        dyn Sink<NotificationOut<Self::BlockHash, Self::Hash>, Error = Self::Error> + Send + Unpin,
    >;
    type Error = ();

    fn finalize_block(&self, h: Self::BlockHash) {
        finalize_block(self.client.clone(), h);
    }

    fn check_extends_finalized(&self, h: Self::BlockHash) -> bool {
        let head_finalized = self.client.info().finalized_hash;
        let lca =
            sp_blockchain::lowest_common_ancestor(self.client.as_ref(), h, head_finalized).unwrap();
        lca.hash == head_finalized
    }

    fn best_block(&self) -> Self::BlockHash {
        self.select_chain.best_chain().unwrap().hash()
    }

    fn consensus_data(&self) -> (Self::Out, Self::In) {
        todo!()
    }

    fn hashing() -> Hashing<Self::Hash> {
        Box::new(|data| blake2_256(data).into())
    }
}

pub(crate) fn finalize_block<BE, B, C>(client: Arc<C>, hash: B::Hash)
where
    B: BlockT,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let status = client.info();
    debug!(target: "env", "Finalizing block with hash {:?}. Previous best: #{:?}.", hash, status.finalized_number);

    let _update_res = client.lock_import_and_run(|import_op| {
        // NOTE: all other finalization logic should come here, inside the lock
        client.apply_finality(import_op, BlockId::Hash(hash), None, true)
    });

    let status = client.info();
    debug!(target: "env", "Finalized block with hash {:?}. Current best: #{:?}.", hash,status.finalized_number);
}
