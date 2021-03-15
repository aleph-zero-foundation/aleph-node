use futures::{Future, Sink, Stream};
use sc_client_api::backend::Backend;
use sp_core::{blake2_256, H256};
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

use rush::{Hashing, NotificationIn, NotificationOut};

use crate::NodeId;

pub struct Environment<C, N, B: BlockT, BE> {
    client: Arc<C>,
    network: N,
    _phantom_block: std::marker::PhantomData<B>,
    _phantom_backend: std::marker::PhantomData<BE>,
}

impl<C, N, B: BlockT, BE> rush::Environment for Environment<C, N, B, BE>
where
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
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

    fn finalize_block(&mut self, _h: Self::BlockHash) {
        todo!()
    }

    fn check_extends_finalized(&self, _h: Self::BlockHash) -> bool {
        todo!()
    }

    fn best_block(&self) -> Self::BlockHash {
        todo!()
    }

    fn check_available(
        &self,
        _h: Self::BlockHash,
    ) -> Box<dyn Future<Output = Result<(), Self::Error>> + Send + Sync + Unpin> {
        todo!()
    }

    fn consensus_data(&self) -> (Self::Out, Self::In) {
        todo!()
    }

    fn hashing() -> Hashing<Self::Hash> {
        Box::new(|data| blake2_256(data).into())
    }
}
