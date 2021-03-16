use crate::{environment::Environment, NodeId, SpawnHandle};
use rush::{Config as ConsensusConfig, Consensus};
use sc_client_api::backend::Backend;
use sp_runtime::traits::Block as BlockT;
use std::{marker::PhantomData, sync::Arc};

pub struct Config {
    pub consensus: ConsensusConfig<NodeId>,
}

pub(crate) struct ConsensusParty<C, N, B, BE>
where
    B: BlockT,
    BE: Backend<B> + 'static,
    C: crate::ClientForAleph<B, BE> + 'static,
    N: 'static,
{
    env: Arc<Environment<C, N, B, BE>>,
    consensus: Consensus<Environment<C, N, B, BE>>,
}

impl<C, N, B, BE> ConsensusParty<C, N, B, BE>
where
    B: BlockT,
    BE: Backend<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    N: Send + Sync + 'static,
{
    pub(crate) fn new(conf: Config, client: Arc<C>, network: N) -> Self {
        let env = Arc::new(Environment {
            client,
            network,
            _phantom_block: PhantomData,
            _phantom_backend: PhantomData,
        });
        let consensus = Consensus::new(conf.consensus, env.clone());

        ConsensusParty { env, consensus }
    }

    pub(crate) async fn run(self, spawn_handle: SpawnHandle) {
        // TODO now it runs just a single instance of consensus but later it will
        // orchestrate managing multiple instances for differents epochs
        self.consensus.run(spawn_handle).await
    }
}
