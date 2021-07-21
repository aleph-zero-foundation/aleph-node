use crate::{
    data_io::DataIO,
    default_aleph_config,
    finalization::{
        chain_extension, finalize_block, finalize_block_as_authority, BlockSignatureAggregator,
    },
    justification::JustificationHandler,
    network,
    network::{split_network, ConsensusNetwork, NetworkData, SessionManager},
    AuthorityId, AuthorityKeystore, JustificationNotification, KeyBox, Metrics, MultiKeychain,
    NodeIndex, SessionId, SpawnHandle,
};
use aleph_primitives::{AlephSessionApi, Session, ALEPH_ENGINE_ID};
use futures::{channel::mpsc, stream::FuturesUnordered, FutureExt, StreamExt};
use log::{debug, error, info};
use sc_client_api::backend::Backend;
use sc_service::SpawnTaskHandle;
use sp_api::{BlockId, NumberFor};
use sp_consensus::SelectChain;
use sp_runtime::traits::Block;
use std::{collections::HashMap, marker::PhantomData, sync::Arc};

pub struct AlephParams<B: Block, N, C, SC> {
    pub config: crate::AlephConfig<B, N, C, SC>,
}

pub async fn run_consensus_party<B, N, C, BE, SC>(aleph_params: AlephParams<B, N, C, SC>)
where
    B: Block,
    N: network::Network<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B, AuthorityId, NumberFor<B>>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    let AlephParams {
        config:
            crate::AlephConfig {
                network,
                client,
                select_chain,
                spawn_handle,
                auth_keystore,
                justification_rx,
                metrics,
                ..
            },
    } = aleph_params;

    let handler_rx = run_justification_handler(&spawn_handle.clone().into(), justification_rx);
    let party = ConsensusParty::new(
        network,
        client,
        select_chain,
        spawn_handle,
        auth_keystore,
        handler_rx,
        metrics,
    );

    debug!(target: "afa", "Consensus party has started.");
    party.run().await;
    error!(target: "afa", "Consensus party has finished unexpectedly.");
}

fn get_node_index(authorities: &[AuthorityId], my_id: &AuthorityId) -> Option<NodeIndex> {
    authorities
        .iter()
        .position(|a| a == my_id)
        .map(|id| id.into())
}

fn run_justification_handler<B: Block>(
    spawn_handle: &SpawnHandle,
    justification_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
) -> mpsc::UnboundedReceiver<JustificationNotification<B>> {
    let (finalization_proposals_tx, finalization_proposals_rx) = mpsc::unbounded();
    let handler = JustificationHandler::new(finalization_proposals_tx, justification_rx);

    debug!(target: "afa", "JustificationHandler started");
    spawn_handle
        .0
        .spawn("aleph/justification_handler", async move {
            handler.run().await;
        });

    finalization_proposals_rx
}

struct ConsensusParty<B, N, C, BE, SC>
where
    B: Block,
    N: network::Network<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B, AuthorityId, NumberFor<B>>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    NumberFor<B>: From<u32>,
{
    network: N,
    sessions: HashMap<u32, Session<AuthorityId, NumberFor<B>>>,
    spawn_handle: SpawnHandle,
    client: Arc<C>,
    select_chain: SC,
    auth_keystore: AuthorityKeystore,
    phantom: PhantomData<BE>,
    finalization_proposals_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
    metrics: Option<Metrics<B::Header>>,
}

/// If we are on the authority list for the given session, runs an
/// AlephBFT task and returns `true` upon completion. Otherwise, immediately returns `false`.
async fn maybe_run_session_as_authority<B, C, BE, SC>(
    auth_keystore: AuthorityKeystore,
    client: Arc<C>,
    session_manager: &SessionManager<NetworkData<B>>,
    session: Session<AuthorityId, NumberFor<B>>,
    spawn_handle: SpawnHandle,
    select_chain: SC,
    metrics: Option<Metrics<B::Header>>,
) -> bool
where
    B: Block,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B, AuthorityId, NumberFor<B>>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    let authority = auth_keystore.authority_id();
    let node_id = match get_node_index(&session.authorities, authority) {
        Some(node_id) => node_id,
        None => return false,
    };
    let current_stop_h = session.stop_h;
    let (ordered_batch_tx, ordered_batch_rx) = mpsc::unbounded();
    let (exit_tx, exit_rx) = futures::channel::oneshot::channel();

    let keybox = KeyBox {
        auth_keystore: auth_keystore.clone(),
        authorities: session.authorities.clone(),
        id: node_id,
    };
    let multikeychain = MultiKeychain::new(keybox);
    let session_id = SessionId(session.session_id as u64);

    let data_network = session_manager
        .start_session(session_id, multikeychain.clone())
        .await;

    let (aleph_network, rmc_network, forwarder) = split_network(data_network);

    spawn_handle.0.spawn("forward-data", forwarder);

    let consensus_config = default_aleph_config(
        session.authorities.len().into(),
        node_id,
        session_id.0 as u64,
    );
    let data_io = DataIO {
        select_chain: select_chain.clone(),
        metrics: metrics.clone(),
        ordered_batch_tx,
    };
    let aleph_task = {
        let multikeychain = multikeychain.clone();
        let spawn_handle = spawn_handle.clone();
        async move {
            let member =
                aleph_bft::Member::new(data_io, &multikeychain, consensus_config, spawn_handle);
            member.run_session(aleph_network, exit_rx).await;
        }
    };
    spawn_handle.0.spawn("aleph/consensus_session", aleph_task);

    debug!(target: "afa", "Consensus party #{} has started.", session_id.0);

    let mut aggregator = BlockSignatureAggregator::new(rmc_network, &multikeychain);

    let ordered_hashes = ordered_batch_rx.map(futures::stream::iter).flatten();
    let mut finalizable_chain =
        chain_extension(ordered_hashes, client.clone(), current_stop_h).fuse();

    loop {
        tokio::select! {
            hash = finalizable_chain.next(), if !finalizable_chain.is_done() => {
                if let Some(hash) = hash {
                    if let Some(ref m) = metrics {
                        m.report_block(hash, std::time::Instant::now(), "aggregation-start");
                    };
                    aggregator.start_aggregation(hash).await;
                } else {
                    aggregator.finish().await;
                    debug!(target: "afa", "hashes to sign ended");
                }
            },
            multisigned_hash = aggregator.next_multisigned_hash(), if !aggregator.is_finished() => {
                if let Some((hash, _multisignature)) = multisigned_hash {
                    // TODO: justify with the multisignature.
                    if let Some(ref m) = metrics {
                        m.report_block(hash, std::time::Instant::now(), "finalize");
                    };
                    let finalization_result = finalize_block_as_authority(client.clone(), hash, &auth_keystore);
                    if let Err(err) = finalization_result {
                        error!(target: "afa", "failed to finalize a block: {:?}", err);
                    }
                } else {
                    break;
                }
            },
            else => {
                debug!(target: "afa", "finished party {:?} with finalized block at {:?}", session_id.0, client.info().finalized_number);
                break;
            }
        }
    }
    exit_tx.send(()).expect("consensus task should not fail");
    true
}

impl<B, N, C, BE, SC> ConsensusParty<B, N, C, BE, SC>
where
    B: Block,
    N: network::Network<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B, AuthorityId, NumberFor<B>>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    NumberFor<B>: From<u32>,
{
    pub(crate) fn new(
        network: N,
        client: Arc<C>,
        select_chain: SC,
        spawn_handle: SpawnTaskHandle,
        auth_keystore: AuthorityKeystore,
        finalization_proposals_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
        metrics: Option<Metrics<B::Header>>,
    ) -> Self {
        Self {
            network,
            client,
            auth_keystore,
            select_chain,
            finalization_proposals_rx,
            metrics,
            spawn_handle: spawn_handle.into(),
            sessions: HashMap::new(),
            phantom: PhantomData,
        }
    }
    async fn run_session(
        &mut self,
        session_manger: &SessionManager<NetworkData<B>>,
        session_id: u32,
    ) {
        let prev_block_number = match session_id.checked_sub(1) {
            None => 0.into(),
            Some(prev_id) => {
                self.sessions
                    .get(&prev_id)
                    .expect("The current session should be known already")
                    .stop_h
            }
        };
        let session = match self
            .client
            .runtime_api()
            .current_session(&BlockId::Number(prev_block_number))
        {
            Ok(session) => {
                self.sessions.insert(session_id, session.clone());
                session
            }
            _ => {
                error!(target: "afa", "No session found for current block #{}", 0);
                return;
            }
        };

        let proposals_task = {
            let client = self.client.clone();
            let current_stop_h = session.stop_h;
            let finalization_proposals_rx = &mut self.finalization_proposals_rx;
            async move {
                while client.info().finalized_number < current_stop_h {
                    if let Some(proposal) = finalization_proposals_rx.next().await {
                        // TODO: check if we should do this
                        let finalization_result = finalize_block(
                            client.clone(),
                            proposal.hash,
                            proposal.number,
                            Some((ALEPH_ENGINE_ID, proposal.justification)),
                        );
                        if let Err(err) = finalization_result {
                            error!(target: "afa", "failed to finalize a block using some received justification: {:?}", err);
                        }
                    } else {
                        debug!(target: "afa", "the channel of proposed blocks closed unexpectedly");
                        break;
                    }
                }
            }
        };
        // returns true if we participated in the session
        debug!(target: "afa", "Starting session nr {:?} -- {:?}", session_id, session);

        let session_task = maybe_run_session_as_authority(
            self.auth_keystore.clone(),
            self.client.clone(),
            session_manger,
            session,
            self.spawn_handle.clone(),
            self.select_chain.clone(),
            self.metrics.clone(),
        );

        // We run concurrently `proposal_task` and `session_task` until either
        // * `proposal_tasks` terminates, or
        // * `session_task` terminates AND returns true.

        let tasks: FuturesUnordered<_> = vec![
            proposals_task.map(|_| true).left_future(),
            session_task.right_future(),
        ]
        .into_iter()
        .collect();

        tasks.filter(|b| std::future::ready(*b)).next().await;
    }

    async fn run(mut self) {
        // Prepare and start the network
        let network = ConsensusNetwork::<NetworkData<B>, _, _>::new(
            self.network.clone(),
            "/cardinals/aleph/1".into(),
        );
        let session_manager = network.session_manager();

        let task = async move { network.run().await };
        self.spawn_handle.0.spawn("aleph/network", task);
        debug!(target: "afa", "Consensus network has started.");

        for curr_id in 0.. {
            info!(target: "afa", "Running session {:?}.", curr_id);
            self.run_session(&session_manager, curr_id).await
        }
    }
}

// TODO: :(
#[cfg(test)]
mod tests {}
