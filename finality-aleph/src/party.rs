use crate::{environment::Environment, NodeId, SpawnHandle};
use rush::{Config as ConsensusConfig, Consensus};
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

pub struct Config {
    pub consensus: ConsensusConfig<NodeId>,
}

pub(crate) struct ConsensusParty<C, N, B, BE, SC>
where
    B: BlockT,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    C: crate::ClientForAleph<B, BE> + 'static,
    N: 'static,
{
    env: Arc<Environment<C, N, B, BE, SC>>,
    consensus: Consensus<Environment<C, N, B, BE, SC>>,
}

impl<C, N, B, BE, SC> ConsensusParty<C, N, B, BE, SC>
where
    B: BlockT,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    N: Send + Sync + 'static,
{
    pub(crate) fn new(conf: Config, client: Arc<C>, network: N, select_chain: SC) -> Self {
        let env = Arc::new(Environment::new(client, network, select_chain));
        let consensus = Consensus::new(conf.consensus, env.clone());

        ConsensusParty { env, consensus }
    }

    pub(crate) async fn run(self, spawn_handle: SpawnHandle) {
        // TODO now it runs just a single instance of consensus but later it will
        // orchestrate managing multiple instances for differents epochs
        self.consensus.run(spawn_handle).await
    }
}
