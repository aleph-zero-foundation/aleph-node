mod validator_node;

use std::{future::Future, sync::Arc};

use aleph_primitives::BlockNumber;
use sc_client_api::Backend;
use sc_network::NetworkService;
use sc_network_common::ExHashT;
use sp_runtime::traits::{Block, Header};
pub use validator_node::run_validator_node;

use crate::{
    finalization::AlephFinalizer,
    justification::{
        JustificationHandler, JustificationNotificationFor, JustificationRequestSchedulerImpl,
        SessionInfo, SessionInfoProvider,
    },
    mpsc,
    mpsc::UnboundedSender,
    session::SessionBoundaryInfo,
    session_map::ReadOnlySessionMap,
    sync::SessionVerifier,
    BlockchainBackend, HashNum, IdentifierFor, JustificationNotification, Metrics,
    MillisecsPerBlock, SessionPeriod,
};

#[cfg(test)]
pub mod testing {
    pub use super::validator_node::new_pen;
}

/// Max amount of tries we can not update a finalized block number before we will clear requests queue
const MAX_ATTEMPTS: u32 = 5;

struct JustificationParams<B, H, C, BB>
where
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    H: ExHashT,
{
    pub network: Arc<NetworkService<B, H>>,
    pub client: Arc<C>,
    pub blockchain_backend: BB,
    pub justification_rx: mpsc::UnboundedReceiver<JustificationNotification<IdentifierFor<B>>>,
    pub metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    pub session_period: SessionPeriod,
    pub millisecs_per_block: MillisecsPerBlock,
    pub session_map: ReadOnlySessionMap,
}

struct SessionInfoProviderImpl {
    session_authorities: ReadOnlySessionMap,
    session_info: SessionBoundaryInfo,
}

impl SessionInfoProviderImpl {
    fn new(session_authorities: ReadOnlySessionMap, session_period: SessionPeriod) -> Self {
        Self {
            session_authorities,
            session_info: SessionBoundaryInfo::new(session_period),
        }
    }
}

#[async_trait::async_trait]
impl<H> SessionInfoProvider<HashNum<H>, SessionVerifier> for SessionInfoProviderImpl
where
    H: Header<Number = BlockNumber>,
{
    async fn for_block_num(&self, number: BlockNumber) -> SessionInfo<HashNum<H>, SessionVerifier> {
        let current_session = self.session_info.session_id_from_block_num(number);
        let last_block_height = self.session_info.last_block_of_session(current_session);
        let verifier = self
            .session_authorities
            .get(current_session)
            .await
            .map(|authority_data| authority_data.into());

        SessionInfo::new(current_session, last_block_height, verifier)
    }
}

fn setup_justification_handler<B, H, C, BB, BE>(
    just_params: JustificationParams<B, H, C, BB>,
) -> (
    UnboundedSender<JustificationNotificationFor<B>>,
    impl Future<Output = ()>,
)
where
    B: Block,
    B::Header: Header<Number = BlockNumber>,
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
