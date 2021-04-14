use crate::{
    environment::{Environment, Network},
    AuthorityId, AuthorityKeystore, NodeId, SpawnHandle,
};

use rush::{Config as ConsensusConfig, Consensus};
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_core::{Blake2Hasher, Hasher, H256};
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
    env: Environment<B, H256, N, C, BE, SC>,
    consensus: Consensus<H256, NodeId>,
}

impl<B, N: Network<B>, C, BE, SC> ConsensusParty<B, N, C, BE, SC>
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
        authorities: Vec<AuthorityId>,
    ) -> Self {
        let hashing = Blake2Hasher::hash;
        let mut env = Environment::new(
            client,
            network,
            select_chain,
            None,
            None,
            authorities,
            auth_keystore,
            hashing,
        );
        let (tx_out, rx_in, tx_order) = env.consensus_data();

        let consensus = Consensus::new(conf, rx_in, tx_out, tx_order, hashing);

        ConsensusParty { env, consensus }
    }

    pub(crate) async fn run(self, spawn_handle: SpawnHandle) {
        // TODO now it runs just a single instance of consensus but later it will
        // orchestrate managing multiple instances for differents epoch

        rush::SpawnHandle::spawn(&spawn_handle.clone(), "aleph/environment", self.env);
        log::debug!(target: "afa", "Environment has started");

        let (_exit, exit) = tokio::sync::oneshot::channel();
        log::debug!(target: "afa", "Consensus party has started");
        self.consensus.run(spawn_handle, exit).await;
        log::debug!(target: "afa", "Consensus party has stopped");
    }
}
