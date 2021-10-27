use crate::{
    aggregator::BlockSignatureAggregator,
    crypto::{AuthorityPen, AuthorityVerifier, KeyBox},
    data_io::{
        reduce_header_to_num, refresh_best_chain, AlephData, AlephDataFor, DataIO, DataStore,
    },
    default_aleph_config,
    finalization::should_finalize,
    justification::{
        AlephJustification, ChainCadence, JustificationHandler, JustificationNotification,
    },
    last_block_of_session,
    metrics::Checkpoint,
    network,
    network::{
        split_network, AlephNetworkData, ConsensusNetwork, DataNetwork, NetworkData, SessionManager,
    },
    session_id_from_block_num, AuthorityId, Future, Metrics, NodeIndex, SessionId, SessionMap,
};
use sp_keystore::CryptoStore;

use aleph_bft::{DelayConfig, OrderedBatch, SpawnHandle};
use aleph_primitives::{AlephSessionApi, SessionPeriod, UnitCreationDelay, KEY_TYPE};
use futures_timer::Delay;

use futures::{
    channel::{mpsc, oneshot},
    future::select,
    pin_mut, StreamExt,
};
use log::{debug, error, info, trace};

use parking_lot::Mutex;
use sc_client_api::backend::Backend;
use sp_api::{BlockId, NumberFor};
use sp_consensus::SelectChain;
use sp_runtime::traits::{Block, Header};
use std::default::Default;
use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};

pub struct AlephParams<B: Block, N, C, SC> {
    pub config: crate::AlephConfig<B, N, C, SC>,
}

pub async fn run_consensus_party<B, N, C, BE, SC>(aleph_params: AlephParams<B, N, C, SC>)
where
    B: Block,
    N: network::Network<B> + network::RequestBlocks<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
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
                keystore,
                justification_rx,
                metrics,
                session_period,
                millisecs_per_block,
                unit_creation_delay,
                ..
            },
    } = aleph_params;

    let session_authorities = Arc::new(Mutex::new(HashMap::new()));

    // NOTE: justifications are requested every so often
    let cadence = min(
        millisecs_per_block.0 * 2,
        millisecs_per_block.0 * session_period.0 as u64 / 10,
    );

    let chain_cadence = ChainCadence {
        session_period,
        justifications_cadence: Duration::from_millis(cadence),
    };

    let block_requester = network.clone();

    let handler = JustificationHandler::new(
        session_authorities.clone(),
        chain_cadence,
        block_requester.clone(),
        client.clone(),
        metrics.clone(),
    );

    let authority_justification_tx =
        run_justification_handler(handler, &spawn_handle.clone().into(), justification_rx);

    // Prepare and start the network
    let network =
        ConsensusNetwork::<NetworkData<B>, _, _>::new(network.clone(), "/cardinals/aleph/1".into());
    let session_manager = network.session_manager();

    let network_task = async move { network.run().await };
    spawn_handle.spawn("aleph/network", network_task);

    debug!(target: "afa", "Consensus network has started.");

    let party = ConsensusParty {
        session_manager,
        client,
        keystore,
        select_chain,
        block_requester,
        metrics,
        authority_justification_tx,
        session_authorities,
        session_period,
        spawn_handle: spawn_handle.into(),
        phantom: PhantomData,
        unit_creation_delay,
    };

    debug!(target: "afa", "Consensus party has started.");
    party.run().await;
    error!(target: "afa", "Consensus party has finished unexpectedly.");
}

async fn get_node_index(
    authorities: &[AuthorityId],
    keystore: Arc<dyn CryptoStore>,
) -> Option<NodeIndex> {
    let our_consensus_keys: HashSet<_> =
        keystore.keys(KEY_TYPE).await.unwrap().into_iter().collect();
    trace!(target: "afa", "Found {:?} consensus keys in our local keystore {:?}", our_consensus_keys.len(), our_consensus_keys);
    authorities
        .iter()
        .position(|pkey| our_consensus_keys.contains(&pkey.into()))
        .map(|id| id.into())
}

fn run_justification_handler<B, N, C, BE>(
    handler: JustificationHandler<B, N, C, BE>,
    spawn_handle: &crate::SpawnHandle,
    import_justification_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
) -> mpsc::UnboundedSender<JustificationNotification<B>>
where
    N: network::Network<B> + network::RequestBlocks<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    B: Block,
{
    let (authority_justification_tx, authority_justification_rx) = mpsc::unbounded();

    debug!(target: "afa", "JustificationHandler started");
    spawn_handle.spawn("aleph/justification_handler", async move {
        handler
            .run(authority_justification_rx, import_justification_rx)
            .await;
    });

    authority_justification_tx
}

struct ConsensusParty<B, C, BE, SC, RB>
where
    B: Block,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    RB: network::RequestBlocks<B> + 'static,
    NumberFor<B>: From<u32>,
{
    session_manager: SessionManager<NetworkData<B>>,
    session_authorities: Arc<Mutex<SessionMap>>,
    session_period: SessionPeriod,
    spawn_handle: crate::SpawnHandle,
    client: Arc<C>,
    select_chain: SC,
    keystore: Arc<dyn CryptoStore>,
    block_requester: RB,
    phantom: PhantomData<BE>,
    metrics: Option<Metrics<B::Header>>,
    authority_justification_tx: mpsc::UnboundedSender<JustificationNotification<B>>,
    unit_creation_delay: UnitCreationDelay,
}

async fn run_aggregator<B, C, BE>(
    mut aggregator: BlockSignatureAggregator<'_, B, KeyBox>,
    mut ordered_batch_rx: mpsc::UnboundedReceiver<OrderedBatch<AlephDataFor<B>>>,
    justification_tx: mpsc::UnboundedSender<JustificationNotification<B>>,
    client: Arc<C>,
    last_block_in_session: NumberFor<B>,
    metrics: Option<Metrics<B::Header>>,
    mut exit_rx: futures::channel::oneshot::Receiver<()>,
) where
    B: Block,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
{
    let mut last_finalized = client.info().finalized_hash;
    let mut last_block_seen = false;
    loop {
        tokio::select! {
            maybe_batch = ordered_batch_rx.next() => {
                if let Some(batch) = maybe_batch {
                    trace!(target: "afa", "Received batch {:?} in aggregator.", batch);
                    if last_block_seen {
                        //This is only for optimization purposes.
                        continue;
                    }
                    for new_block_data in batch {
                        if let Some(metrics) = &metrics {
                            metrics.report_block(new_block_data.hash, std::time::Instant::now(), Checkpoint::Ordered);
                        }
                        if let Some(data) = should_finalize(last_finalized, new_block_data, client.as_ref(), last_block_in_session) {
                            aggregator.start_aggregation(data.hash).await;
                            last_finalized = data.hash;
                            if data.number == last_block_in_session {
                                aggregator.notify_last_hash();
                                last_block_seen = true;
                                break;
                            }
                        }
                    }
                } else {
                    debug!(target: "afa", "Batches ended in aggregator. Terminating.");
                    break;
                }
            }
            multisigned_hash = aggregator.next_multisigned_hash() => {
                if let Some((hash, multisignature)) = multisigned_hash {
                    let number = client.number(hash).unwrap().unwrap();
                    // The unwrap might actually fail if data availability is not implemented correctly.
                    let notification = JustificationNotification {
                        justification: AlephJustification::new::<B>(multisignature),
                        hash,
                        number
                    };
                    if let Err(e) = justification_tx.unbounded_send(notification)  {
                        error!(target: "afa", "Issue with sending justification from Aggregator to JustificationHandler {:?}.", e);
                    }
                } else {
                    debug!(target: "afa", "The stream of multisigned hashes has ended. Terminating.");
                    break;
                }
            }
            _ = &mut exit_rx => {
                debug!(target: "afa", "Aggregator received exit signal. Terminating.");
                return;
            }
        }
    }
    debug!(target: "afa", "Aggregator awaiting an exit signal.");
    // this allows aggregator to exit after member,
    // otherwise it can exit too early and member complains about a channel to aggregator being closed
    let _ = exit_rx.await;
}

impl<B, C, BE, SC, RB> ConsensusParty<B, C, BE, SC, RB>
where
    B: Block,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    RB: network::RequestBlocks<B> + 'static,
    NumberFor<B>: From<u32>,
{
    async fn run_session_as_authority(
        &self,
        node_id: NodeIndex,
        multikeychain: KeyBox,
        data_network: DataNetwork<NetworkData<B>>,
        session_id: SessionId,
        authorities: Vec<AuthorityId>,
        exit_rx: futures::channel::oneshot::Receiver<()>,
    ) -> impl Future<Output = ()> {
        debug!(target: "afa", "Authority task {:?}", session_id);
        let last_block = last_block_of_session::<B>(session_id, self.session_period);
        let (ordered_batch_tx, ordered_batch_rx) = mpsc::unbounded();
        let (aleph_network_tx, data_store_rx) = mpsc::unbounded();
        let (data_store_tx, aleph_network_rx) = mpsc::unbounded();
        let mut data_store = DataStore::<B, C, BE, RB, AlephNetworkData<B>>::new(
            self.client.clone(),
            self.block_requester.clone(),
            data_store_tx,
            data_store_rx,
            Default::default(),
        );
        let (aleph_network, rmc_network, forwarder) =
            split_network(data_network, aleph_network_tx, aleph_network_rx);

        let consensus_config = create_aleph_config(
            authorities.len(),
            node_id,
            session_id,
            self.unit_creation_delay,
        );

        let best_header = self
            .select_chain
            .best_chain()
            .await
            .expect("No best chain.");
        let reduced_header = reduce_header_to_num(self.client.clone(), best_header, last_block);
        let proposed_block = Arc::new(Mutex::new(AlephData::new(
            reduced_header.hash(),
            *reduced_header.number(),
        )));
        let data_io = DataIO::<B> {
            ordered_batch_tx,
            proposed_block: proposed_block.clone(),
            metrics: self.metrics.clone(),
        };

        let (exit_member_tx, exit_member_rx) = oneshot::channel();
        let (exit_data_store_tx, exit_data_store_rx) = oneshot::channel();
        let (exit_aggregator_tx, exit_aggregator_rx) = oneshot::channel();
        let (exit_refresher_tx, exit_refresher_rx) = oneshot::channel();
        let (exit_forwarder_tx, exit_forwarder_rx) = oneshot::channel();

        let member_task = {
            let spawn_handle = self.spawn_handle.clone();
            let multikeychain = multikeychain.clone();
            async move {
                debug!(target: "afa", "Running the member task for {:?}", session_id.0);
                aleph_bft::run_session(
                    consensus_config,
                    aleph_network,
                    data_io,
                    multikeychain,
                    spawn_handle,
                    exit_member_rx,
                )
                .await;
                debug!(target: "afa", "Member task stopped for {:?}", session_id.0);
            }
        };

        let data_store_task = {
            async move {
                debug!(target: "afa", "Running the data store task for {:?}", session_id.0);
                data_store.run(exit_data_store_rx).await;
                debug!(target: "afa", "Data store task stopped for {:?}", session_id.0);
            }
        };

        let aggregator_task = {
            let client = self.client.clone();
            let justification_tx = self.authority_justification_tx.clone();
            let last_block = last_block_of_session::<B>(session_id, self.session_period);
            let metrics = self.metrics.clone();
            let multikeychain = multikeychain.clone();
            async move {
                let aggregator =
                    BlockSignatureAggregator::new(rmc_network, &multikeychain, metrics.clone());
                debug!(target: "afa", "Running the aggregator task for {:?}", session_id.0);
                run_aggregator(
                    aggregator,
                    ordered_batch_rx,
                    justification_tx,
                    client,
                    last_block,
                    metrics,
                    exit_aggregator_rx,
                )
                .await;
                debug!(target: "afa", "Aggregator task stopped for {:?}", session_id.0);
            }
        };

        let forwarder_task = async move {
            debug!(target: "afa", "Running the forwarder task for {:?}", session_id.0);
            pin_mut!(forwarder);
            select(forwarder, exit_forwarder_rx).await;
            debug!(target: "afa", "Forwarder task stopped for {:?}", session_id.0);
        };

        let refresher_task = {
            refresh_best_chain(
                self.select_chain.clone(),
                self.client.clone(),
                proposed_block,
                last_block,
                exit_refresher_rx,
            )
        };

        let member_handle = self
            .spawn_handle
            .spawn_essential("aleph/consensus_session_member", member_task);
        let data_store_handle = self
            .spawn_handle
            .spawn_essential("aleph/consensus_session_data_store", data_store_task);
        let aggregator_handle = self
            .spawn_handle
            .spawn_essential("aleph/consensus_session_aggregator", aggregator_task);
        let forwarder_handle = self
            .spawn_handle
            .spawn_essential("aleph/consensus_session_forwarder", forwarder_task);
        let refresher_handle = self
            .spawn_handle
            .spawn_essential("aleph/consensus_session_refresher", refresher_task);

        async move {
            let _ = exit_rx.await;
            // both member and aggregator are implicitly using forwarder,
            // so we should force them to exit first to avoid any panics, i.e. `send on closed channel`
            if let Err(e) = exit_member_tx.send(()) {
                debug!(target: "afa", "member was closed before terminating it manually: {:?}", e)
            }
            let _ = member_handle.await;

            if let Err(e) = exit_aggregator_tx.send(()) {
                debug!(target: "afa", "aggregator was closed before terminating it manually: {:?}", e)
            }
            let _ = aggregator_handle.await;

            if let Err(e) = exit_forwarder_tx.send(()) {
                debug!(target: "afa", "forwarder was closed before terminating it manually: {:?}", e)
            }
            let _ = forwarder_handle.await;

            if let Err(e) = exit_refresher_tx.send(()) {
                debug!(target: "afa", "refresh was closed before terminating it manually: {:?}", e)
            }
            let _ = refresher_handle.await;

            if let Err(e) = exit_data_store_tx.send(()) {
                debug!(target: "afa", "data store was closed before terminating it manually: {:?}", e)
            }
            let _ = data_store_handle.await;
            info!(target: "afa", "Terminated authority run of session {:?}", session_id);
        }
    }

    async fn run_session(&mut self, session_id: SessionId) {
        let authorities = {
            if session_id == SessionId(0) {
                self.client
                    .runtime_api()
                    .authorities(&BlockId::Number(0.into()))
                    .unwrap()
            } else {
                let last_prev =
                    last_block_of_session::<B>(SessionId(session_id.0 - 1), self.session_period);
                // We must read the authorities for next session of the latest block of the previous session.
                // The reason is that we are not guaranteed to have the first block of new session available yet.
                match self
                    .client
                    .runtime_api()
                    .next_session_authorities(&BlockId::Number(last_prev))
                {
                    Ok(authorities) => authorities
                        .expect("authorities must be available at last block of previous session"),
                    Err(e) => {
                        error!(target: "afa", "Error when getting authorities for session {:?} {:?}", session_id, e);
                        return;
                    }
                }
            }
        };
        self.session_authorities
            .lock()
            .insert(session_id, authorities.clone());
        let last_block = last_block_of_session::<B>(session_id, self.session_period);

        // Early skip attempt -- this will trigger during catching up (initial sync).
        if self.client.info().best_number >= last_block {
            // We need to give the JustificationHandler some time to pick up the keybox for the new session,
            // validate justifications and finalize blocks. We wait 2000ms in total, checking every 200ms
            // if the last block has been finalized.
            for attempt in 0..10 {
                // We don't wait before the first attempt.
                if attempt != 0 {
                    Delay::new(Duration::from_millis(200)).await;
                }
                let last_finalized_number = self.client.info().finalized_number;
                if last_finalized_number >= last_block {
                    debug!(target: "afa", "Skipping session {:?} early because block {:?} is already finalized", session_id, last_finalized_number);
                    return;
                }
            }
        }
        trace!(target: "afa", "Authorities for session {:?}: {:?}", session_id, authorities);
        let maybe_node_id = get_node_index(&authorities, self.keystore.clone()).await;

        let (exit_authority_tx, exit_authority_rx) = futures::channel::oneshot::channel();
        if let Some(node_id) = maybe_node_id {
            debug!(target: "afa", "Running session {:?} as authority id {:?}", session_id, node_id);
            let keybox = KeyBox::new(
                node_id,
                AuthorityVerifier::new(authorities.clone()),
                AuthorityPen::new(authorities[node_id.0].clone(), self.keystore.clone())
                    .await
                    .expect("The keys should sign successfully"),
            );
            let data_network = self
                .session_manager
                .start_session(session_id, keybox.clone())
                .await;

            let authority_task = self
                .run_session_as_authority(
                    node_id,
                    keybox,
                    data_network,
                    session_id,
                    authorities,
                    exit_authority_rx,
                )
                .await;
            self.spawn_handle
                .spawn("aleph/session_authority", authority_task);
        } else {
            debug!(target: "afa", "Running session {:?} as non-authority", session_id);
        }
        loop {
            let last_finalized_number = self.client.info().finalized_number;
            debug!(target: "afa", "Highest finalized: {:?} session {:?}", last_finalized_number, session_id);
            if last_finalized_number >= last_block {
                debug!(target: "afa", "Terminating session {:?}", session_id);
                break;
            }
            Delay::new(Duration::from_millis(1000)).await;
        }
        if maybe_node_id.is_some() {
            debug!(target: "afa", "Sending exit signal to the authority task.");
            let _ = exit_authority_tx.send(());
            self.session_manager.stop_session(session_id);
        }
    }

    fn prune_session_data(&self, prune_below: SessionId) {
        // In this method we make sure that the amount of data we keep in RAM in finality-aleph
        // does not grow with the size of the blockchain.
        debug!(target: "afa", "Pruning session data below {:?}.", prune_below);
        self.session_authorities
            .lock()
            .retain(|&s, _| s >= prune_below);
    }

    async fn run(mut self) {
        let last_finalized_number = self.client.info().finalized_number;
        let starting_session =
            session_id_from_block_num::<B>(last_finalized_number, self.session_period).0;
        for curr_id in starting_session.. {
            info!(target: "afa", "Running session {:?}.", curr_id);
            self.run_session(SessionId(curr_id)).await;
            if curr_id >= 10 && curr_id % 10 == 0 {
                self.prune_session_data(SessionId(curr_id - 10));
            }
        }
    }
}

pub(crate) fn create_aleph_config(
    n_members: usize,
    node_id: NodeIndex,
    session_id: SessionId,
    unit_creation_delay: UnitCreationDelay,
) -> aleph_bft::Config {
    let mut consensus_config = default_aleph_config(n_members.into(), node_id, session_id.0 as u64);
    consensus_config.max_round = 7000;
    let unit_creation_delay = Arc::new(move |t| {
        if t == 0 {
            Duration::from_millis(2000)
        } else {
            exponential_slowdown(t, unit_creation_delay.0 as f64, 5000, 1.005)
        }
    });
    let unit_broadcast_delay = Arc::new(|t| exponential_slowdown(t, 4000., 0, 2.));
    let delay_config = DelayConfig {
        tick_interval: Duration::from_millis(100),
        requests_interval: Duration::from_millis(3000),
        unit_broadcast_delay,
        unit_creation_delay,
    };
    consensus_config.delay_config = delay_config;
    consensus_config
}

pub fn exponential_slowdown(
    t: usize,
    base_delay: f64,
    start_exp_delay: usize,
    exp_base: f64,
) -> Duration {
    // This gives:
    // base_delay, for t <= start_exp_delay,
    // base_delay * exp_base^(t - start_exp_delay), for t > start_exp_delay.
    let delay = if t < start_exp_delay {
        base_delay
    } else {
        let power = t - start_exp_delay;
        base_delay * exp_base.powf(power as f64)
    };
    let delay = delay.round() as u64;
    // the above will make it u64::MAX if it exceeds u64
    Duration::from_millis(delay)
}

// TODO: :(
#[cfg(test)]
mod tests {}
