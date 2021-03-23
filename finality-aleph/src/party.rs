use crate::{
    communication::network::{Network, NetworkBridge},
    environment::Environment,
    AuthorityKeystore, NodeId, SpawnHandle,
};
use rush::{Config as ConsensusConfig, Consensus, EpochId};
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_runtime::traits::Block;
use std::sync::Arc;

pub(crate) struct ConsensusParty<B, N, C, BE, SC>
where
    B: Block,
    N: Network<B>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    consensus: Consensus<Environment<B, N, C, BE, SC>>,
}

impl<B, N, C, BE, SC> ConsensusParty<B, N, C, BE, SC>
where
    B: Block,
    N: Network<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    pub(crate) fn new(
        conf: ConsensusConfig<NodeId>,
        client: Arc<C>,
        network: N,
        select_chain: SC,
        auth_keystore: AuthorityKeystore,
        epoch_id: EpochId,
    ) -> Self {
        let network_bridge = NetworkBridge::new(network, None, None);
        let env = Arc::new(Environment::new(
            client,
            network_bridge,
            auth_keystore,
            select_chain,
            epoch_id,
        ));
        let consensus = Consensus::new(conf, env.clone());

        ConsensusParty { consensus }
    }

    pub(crate) async fn run(self, spawn_handle: SpawnHandle) {
        // TODO now it runs just a single instance of consensus but later it will
        // orchestrate managing multiple instances for differents epochs
        let (_exit, exit) = tokio::sync::oneshot::channel();
        log::debug!(target: "afa", "Consensus party has started");
        self.consensus.run(spawn_handle, exit).await;
        log::debug!(target: "afa", "Consensus party has stopped");
    }
}
