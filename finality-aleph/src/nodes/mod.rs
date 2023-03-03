mod nonvalidator_node;
mod validator_node;

use std::{future::Future, sync::Arc};

pub use nonvalidator_node::run_nonvalidator_node;
use sc_client_api::Backend;
use sc_network::NetworkService;
use sc_network_common::ExHashT;
use sp_runtime::traits::{Block, Header, NumberFor};
pub use validator_node::run_validator_node;

use crate::{
    finalization::AlephFinalizer,
    justification::{
        JustificationHandler, JustificationRequestSchedulerImpl, SessionInfo, SessionInfoProvider,
    },
    last_block_of_session, mpsc,
    mpsc::UnboundedSender,
    session_id_from_block_num,
    session_map::ReadOnlySessionMap,
    sync::SessionVerifier,
    BlockchainBackend, JustificationNotification, Metrics, MillisecsPerBlock, SessionPeriod,
};

#[cfg(test)]
pub mod testing {
    pub use super::validator_node::new_pen;
}

/// Max amount of tries we can not update a finalized block number before we will clear requests queue
const MAX_ATTEMPTS: u32 = 5;

struct JustificationParams<B: Block, H: ExHashT, C, BB> {
    pub network: Arc<NetworkService<B, H>>,
    pub client: Arc<C>,
    pub blockchain_backend: BB,
    pub justification_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
    pub metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    pub session_period: SessionPeriod,
    pub millisecs_per_block: MillisecsPerBlock,
    pub session_map: ReadOnlySessionMap,
}

struct SessionInfoProviderImpl {
    session_authorities: ReadOnlySessionMap,
    session_period: SessionPeriod,
}

impl SessionInfoProviderImpl {
    fn new(session_authorities: ReadOnlySessionMap, session_period: SessionPeriod) -> Self {
        Self {
            session_authorities,
            session_period,
        }
    }
}

#[async_trait::async_trait]
impl<B: Block> SessionInfoProvider<B, SessionVerifier> for SessionInfoProviderImpl {
    async fn for_block_num(&self, number: NumberFor<B>) -> SessionInfo<B, SessionVerifier> {
        let current_session = session_id_from_block_num(number, self.session_period);
        let last_block_height = last_block_of_session(current_session, self.session_period);
        let verifier = self
            .session_authorities
            .get(current_session)
            .await
            .map(|authority_data| authority_data.into());

        SessionInfo {
            current_session,
            last_block_height,
            verifier,
        }
    }
}

fn setup_justification_handler<B, H, C, BB, BE>(
    just_params: JustificationParams<B, H, C, BB>,
) -> (
    UnboundedSender<JustificationNotification<B>>,
    impl Future<Output = ()>,
)
where
    B: Block,
    H: ExHashT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    BB: BlockchainBackend<B> + 'static + Send,
{
    let JustificationParams {
        network,
        client,
        blockchain_backend,
        justification_rx,
        metrics,
        session_period,
        millisecs_per_block,
        session_map,
    } = just_params;

    let handler = JustificationHandler::new(
        SessionInfoProviderImpl::new(session_map, session_period),
        network,
        blockchain_backend,
        AlephFinalizer::new(client),
        JustificationRequestSchedulerImpl::new(&session_period, &millisecs_per_block, MAX_ATTEMPTS),
        metrics,
        Default::default(),
    );

    let (authority_justification_tx, authority_justification_rx) = mpsc::unbounded();
    (authority_justification_tx, async move {
        handler
            .run(authority_justification_rx, justification_rx)
            .await;
    })
}
