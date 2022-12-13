mod nonvalidator_node;
mod validator_node;

use std::{future::Future, sync::Arc};

use aleph_primitives::{AuthorityId, SessionAuthorityData};
use codec::Encode;
use log::warn;
pub use nonvalidator_node::run_nonvalidator_node;
use sc_client_api::Backend;
use sc_network::NetworkService;
use sc_network_common::ExHashT;
use sp_runtime::{
    traits::{Block, Header, NumberFor},
    RuntimeAppPublic,
};
pub use validator_node::run_validator_node;

use crate::{
    crypto::AuthorityVerifier,
    finalization::AlephFinalizer,
    justification::{
        AlephJustification, JustificationHandler, JustificationRequestSchedulerImpl, SessionInfo,
        SessionInfoProvider, Verifier,
    },
    last_block_of_session, mpsc,
    mpsc::UnboundedSender,
    session_id_from_block_num,
    session_map::ReadOnlySessionMap,
    BlockchainBackend, JustificationNotification, Metrics, MillisecsPerBlock, SessionPeriod,
};

#[cfg(test)]
pub mod testing {
    pub use super::validator_node::new_pen;
}

/// Max amount of tries we can not update a finalized block number before we will clear requests queue
const MAX_ATTEMPTS: u32 = 5;

struct JustificationVerifier {
    authority_verifier: AuthorityVerifier,
    emergency_signer: Option<AuthorityId>,
}

impl From<SessionAuthorityData> for JustificationVerifier {
    fn from(authority_data: SessionAuthorityData) -> Self {
        JustificationVerifier {
            authority_verifier: AuthorityVerifier::new(authority_data.authorities().to_vec()),
            emergency_signer: authority_data.emergency_finalizer().clone(),
        }
    }
}

impl<B: Block> Verifier<B> for JustificationVerifier {
    fn verify(&self, justification: &AlephJustification, hash: B::Hash) -> bool {
        use AlephJustification::*;
        let encoded_hash = hash.encode();
        match justification {
            CommitteeMultisignature(multisignature) => match self
                .authority_verifier
                .is_complete(&encoded_hash, multisignature)
            {
                true => true,
                false => {
                    warn!(target: "aleph-justification", "Bad multisignature for block hash #{:?} {:?}", hash, multisignature);
                    false
                }
            },
            EmergencySignature(signature) => match &self.emergency_signer {
                Some(emergency_signer) => match emergency_signer.verify(&encoded_hash, signature) {
                    true => true,
                    false => {
                        warn!(target: "aleph-justification", "Bad emergency signature for block hash #{:?} {:?}", hash, signature);
                        false
                    }
                },
                None => {
                    warn!(target: "aleph-justification", "Received emergency signature for block with hash #{:?}, which has no emergency signer defined.", hash);
                    false
                }
            },
        }
    }
}

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
impl<B: Block> SessionInfoProvider<B, JustificationVerifier> for SessionInfoProviderImpl {
    async fn for_block_num(&self, number: NumberFor<B>) -> SessionInfo<B, JustificationVerifier> {
        let current_session = session_id_from_block_num::<B>(number, self.session_period);
        let last_block_height = last_block_of_session::<B>(current_session, self.session_period);
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
