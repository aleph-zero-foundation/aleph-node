use crate::{
    data_io::DataIO,
    finalization::{
        check_extends_last_finalized, finalize_block, finalize_block_as_authority,
        reduce_block_up_to,
    },
    hash,
    justification::JustificationHandler,
    network,
    network::{ConsensusNetwork, SessionManager},
    AuthorityId, AuthorityKeystore, ConsensusConfig, JustificationNotification, KeyBox, NodeIndex,
    SessionId, SpawnHandle,
};
use aleph_primitives::{AlephSessionApi, Session, ALEPH_ENGINE_ID};
use futures::{channel::mpsc, select, stream::SelectAll, StreamExt};
use log::{debug, error};
use rush::OrderedBatch;
use sc_client_api::backend::Backend;
use sc_service::SpawnTaskHandle;
use sp_api::{BlockId, NumberFor};
use sp_consensus::SelectChain;
use sp_runtime::traits::{BlakeTwo256, Block};
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
                authority,
                justification_rx,
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
        authority,
        handler_rx,
    );

    debug!(target: "afa", "Consensus party has started.");
    party.run().await;
    error!(target: "afa", "Consensus party has finished unexpectedly.");
}

fn create_session<B, SC>(
    authority: AuthorityId,
    auth_keystore: AuthorityKeystore,
    select_chain: SC,
    spawn_handle: SpawnHandle,
    session: Session<AuthorityId, NumberFor<B>>,
    session_manager: &SessionManager,
) -> (
    SessionInstance<B>,
    Option<mpsc::UnboundedReceiver<OrderedBatch<B::Hash>>>,
)
where
    B: Block,
    SC: SelectChain<B> + 'static,
{
    // If we are in session authorities run consensus.
    let node_id = match get_node_index(&session.authorities, &authority) {
        Some(node_id) => node_id,
        None => {
            return (
                SessionInstance {
                    session,
                    exit_tx: None,
                },
                None,
            )
        }
    };
    let (ordered_batch_tx, ordered_batch_rx) = mpsc::unbounded();
    let (exit_tx, exit_rx) = futures::channel::oneshot::channel();

    let session_id = session.session_id as u64;
    let session_network =
        session_manager.start_session(SessionId(session_id), session.authorities.clone());

    let data_io = DataIO {
        select_chain,
        ordered_batch_tx,
    };

    let consensus_config = ConsensusConfig {
        node_id,
        session_id,
        n_members: session.authorities.len().into(),
        create_lag: std::time::Duration::from_millis(500),
    };

    let spawn_clone = spawn_handle.clone();
    let authorities = session.authorities.clone();

    let task = async move {
        let keybox = KeyBox {
            auth_keystore,
            authorities,
            id: node_id,
        };
        let member = rush::Member::<hash::Wrapper<BlakeTwo256>, _, _, _, _>::new(
            data_io,
            &keybox,
            session_network,
            consensus_config,
        );
        member.run_session(spawn_clone, exit_rx).await;
    };

    spawn_handle.0.spawn("aleph/consensus_session", task);
    debug!(target: "afa", "Consensus party #{} has started.", session.session_id);

    (
        SessionInstance {
            session,
            exit_tx: Some(exit_tx),
        },
        Some(ordered_batch_rx),
    )
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

struct SessionInstance<B>
where
    B: Block,
{
    pub(crate) session: Session<AuthorityId, NumberFor<B>>,
    pub(crate) exit_tx: Option<futures::channel::oneshot::Sender<()>>,
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
    sessions: HashMap<u32, SessionInstance<B>>,
    spawn_handle: SpawnHandle,
    client: Arc<C>,
    select_chain: SC,
    auth_keystore: AuthorityKeystore,
    authority: AuthorityId,
    phantom: PhantomData<BE>,
    finalization_proposals_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
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
        authority: AuthorityId,
        finalization_proposals_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
    ) -> Self {
        Self {
            network,
            client,
            auth_keystore,
            select_chain,
            authority,
            finalization_proposals_rx,
            spawn_handle: spawn_handle.into(),
            sessions: HashMap::new(),
            phantom: PhantomData,
        }
    }

    async fn run(mut self) {
        // Prepare and start the network
        let network = ConsensusNetwork::new(self.network.clone(), "/cardinals/aleph/1");
        let session_manager = network.session_manager();

        let task = async move { network.run().await };
        self.spawn_handle.0.spawn("aleph/network", task);
        debug!(target: "afa", "Consensus network has started.");

        // a set of streams of (u32, OrderedBatch<B::Hash>>) pairs
        let mut proposition_select = SelectAll::new();

        let mut waiting_blocks = HashMap::<u32, Vec<B::Hash>>::new();

        let genesis_session = match self
            .client
            .runtime_api()
            .current_session(&BlockId::Number(0.into()))
        {
            Ok(session) => session,
            _ => {
                error!(target: "afa", "No session found for current block #{}", 0);
                return;
            }
        };

        // Start new session if we are in the authority set.
        let (instance, proposition_rx) = create_session(
            self.authority.clone(),
            self.auth_keystore.clone(),
            self.select_chain.clone(),
            self.spawn_handle.clone(),
            genesis_session,
            &session_manager,
        );
        self.sessions.insert(0, instance);
        if let Some(proposition_rx) = proposition_rx {
            proposition_select.push(futures::stream::repeat(0).zip(proposition_rx));
        }

        for curr_id in 0.. {
            // Stopping block is the last before the new session kick ins
            let current_stop_h = self
                .sessions
                .get(&curr_id)
                .expect("The current session should be known already")
                .session
                .stop_h;

            if let Some(hashes) = waiting_blocks.remove(&curr_id) {
                for hash in hashes {
                    self.handle_proposal(hash, current_stop_h);
                }
            }

            while self.client.info().finalized_number < current_stop_h {
                select! {
                    x = proposition_select.next() => {
                        match x {
                            Some((id, batch)) if id == curr_id => {
                                for hash in batch {
                                    self.handle_proposal(hash, current_stop_h);
                                }
                            },
                            Some((id, _)) if id < curr_id => {
                                debug!(target: "afa", "Received finalization proposal for past round #{}", id);
                            },
                            Some((id, batch)) => {
                                debug!(target: "afa", "Received finalization proposal for future round #{}, storing it for later consideration", id);
                                waiting_blocks.entry(id).or_insert_with(Vec::new).extend(batch);
                            },
                            None => {},
                        };
                    },
                    x = self.finalization_proposals_rx.next() => {
                        if let Some(proposal) = x {
                            // TODO: check if we should do this
                            finalize_block(self.client.clone(), proposal.hash, proposal.number, Some((
                                ALEPH_ENGINE_ID,
                                proposal.justification
                            )));
                        }
                    },
                    complete => {
                        error!(target: "afa", "Proposal channel and proposition_select channels finished");

                        // if this condition is false no hopes for restarting.
                        if self.client.info().finalized_number != current_stop_h {
                            return;
                        }
                    }
                }
            }

            if let Some(instance) = self.sessions.remove(&curr_id) {
                if let Some(exit_tx) = instance.exit_tx {
                    // Signal the end of the session
                    debug!(target: "afa", "Signaling end of the consensus party #{}.", curr_id);
                    exit_tx.send(()).expect("Closing member session");
                }

                self.sessions.insert(
                    curr_id,
                    SessionInstance {
                        session: instance.session,
                        exit_tx: None,
                    },
                );
            }

            debug!(target: "afa", "Moving to new session #{}.", curr_id + 1);

            // We are asking for the next_session on the last block, which we know that is finalized, of the current session.
            // We are calling the `current_session` function not the `next_session` the sessions rotates on the last blocks of the session.
            let next_session = match self
                .client
                .runtime_api()
                .current_session(&BlockId::Number(current_stop_h))
            {
                Ok(session) => session,
                _ => {
                    error!(target: "afa", "No next session found for current block #{}", current_stop_h);
                    return;
                }
            };
            // Start new session if we are in the authority set.
            let (instance, proposition_rx) = create_session(
                self.authority.clone(),
                self.auth_keystore.clone(),
                self.select_chain.clone(),
                self.spawn_handle.clone(),
                next_session,
                &session_manager,
            );
            self.sessions.insert(curr_id + 1, instance);
            if let Some(proposition_rx) = proposition_rx {
                proposition_select.push(futures::stream::repeat(curr_id + 1).zip(proposition_rx));
            }
        }
    }

    fn handle_proposal(&mut self, h: B::Hash, max_height: NumberFor<B>) {
        if let Some(reduced) = reduce_block_up_to(self.client.clone(), h, max_height) {
            if check_extends_last_finalized(self.client.clone(), reduced) {
                finalize_block_as_authority(self.client.clone(), reduced, &self.auth_keystore);
            }
        }
    }
}

// TODO: :(
#[cfg(test)]
mod tests {}
