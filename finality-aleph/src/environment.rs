use crate::{
    communication::network::{Network, NetworkBridge, NetworkError, NotificationOutSender},
    AuthorityKeystore, NodeId,
};
use futures::Stream;
use log::debug;
use rush::{EpochId, Hashing, NotificationIn};
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
    pub(crate) epoch_id: EpochId,
    pub(crate) auth_keystore: AuthorityKeystore,
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
    pub fn new(
        client: Arc<C>,
        network: NetworkBridge<B, H256, N>,
        auth_keystore: AuthorityKeystore,
        select_chain: SC,
        epoch_id: EpochId,
    ) -> Self {
        Environment {
            client,
            network,
            select_chain,
            epoch_id,
            auth_keystore,
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
    type In = Box<dyn Stream<Item = NotificationIn<Self::BlockHash, Self::Hash>> + Send + Unpin>;
    type Out = NotificationOutSender<B, Self::Hash>;
    type Error = NetworkError;

    fn finalize_block(&self, h: Self::BlockHash) {
        finalize_block(self.client.clone(), h);
    }

    fn check_extends_finalized(&self, h: Self::BlockHash) -> bool {
        let head_finalized = self.client.info().finalized_hash;
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
        self.network
            .communication(self.epoch_id, self.auth_keystore.clone())
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
    let status = client.info();
    debug!(target: "env", "Finalizing block with hash {:?}. Previous best: #{:?}.", hash, status.finalized_number);

    let _update_res = client.lock_import_and_run(|import_op| {
        // NOTE: all other finalization logic should come here, inside the lock
        client.apply_finality(import_op, BlockId::Hash(hash), None, true)
    });

    let status = client.info();
    debug!(target: "env", "Finalized block with hash {:?}. Current best: #{:?}.", hash,status.finalized_number);
}
