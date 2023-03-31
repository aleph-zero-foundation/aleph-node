use std::sync::{Arc, Mutex};

use aleph_primitives::BlockNumber;

use super::TBlockIdentifier;
use crate::{
    justification::{AlephJustification, SessionInfo, SessionInfoProvider, Verifier},
    session::SessionBoundaryInfo as SessionBoundInfo,
    testing::mocks::AcceptancePolicy,
    SessionPeriod,
};

pub struct VerifierWrapper {
    acceptance_policy: Arc<Mutex<AcceptancePolicy>>,
}

impl Verifier<TBlockIdentifier> for VerifierWrapper {
    fn verify(&self, _justification: &AlephJustification, _block_id: &TBlockIdentifier) -> bool {
        self.acceptance_policy.lock().unwrap().accepts()
    }
}

pub struct SessionInfoProviderImpl {
    session_info: SessionBoundInfo,
    acceptance_policy: Arc<Mutex<AcceptancePolicy>>,
}

impl SessionInfoProviderImpl {
    pub fn new(session_period: SessionPeriod, acceptance_policy: AcceptancePolicy) -> Self {
        Self {
            session_info: SessionBoundInfo::new(session_period),
            acceptance_policy: Arc::new(Mutex::new(acceptance_policy)),
        }
    }
}

#[async_trait::async_trait]
impl SessionInfoProvider<TBlockIdentifier, VerifierWrapper> for SessionInfoProviderImpl {
    async fn for_block_num(
        &self,
        number: BlockNumber,
    ) -> SessionInfo<TBlockIdentifier, VerifierWrapper> {
        let current_session = self.session_info.session_id_from_block_num(number);
        SessionInfo::new(
            current_session,
            self.session_info.last_block_of_session(current_session),
            match &*self.acceptance_policy.lock().unwrap() {
                AcceptancePolicy::Unavailable => None,
                _ => Some(VerifierWrapper {
                    acceptance_policy: self.acceptance_policy.clone(),
                }),
            },
        )
    }
}
