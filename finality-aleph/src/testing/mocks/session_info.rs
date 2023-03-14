use std::sync::{Arc, Mutex};

use aleph_primitives::BlockNumber;

use crate::{
    justification::{AlephJustification, SessionInfo, SessionInfoProvider, Verifier},
    session::SessionBoundaryInfo as SessionBoundInfo,
    testing::mocks::{AcceptancePolicy, TBlock, THash},
    SessionPeriod,
};

pub struct VerifierWrapper {
    acceptance_policy: Arc<Mutex<AcceptancePolicy>>,
}

impl Verifier<TBlock> for VerifierWrapper {
    fn verify(&self, _justification: &AlephJustification, _hash: THash) -> bool {
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
impl SessionInfoProvider<TBlock, VerifierWrapper> for SessionInfoProviderImpl {
    async fn for_block_num(&self, number: BlockNumber) -> SessionInfo<TBlock, VerifierWrapper> {
        let current_session = self.session_info.session_id_from_block_num(number);
        SessionInfo {
            current_session,
            last_block_height: self.session_info.last_block_of_session(current_session),
            verifier: match &*self.acceptance_policy.lock().unwrap() {
                AcceptancePolicy::Unavailable => None,
                _ => Some(VerifierWrapper {
                    acceptance_policy: self.acceptance_policy.clone(),
                }),
            },
        }
    }
}
