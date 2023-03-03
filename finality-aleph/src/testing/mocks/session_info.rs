use std::sync::{Arc, Mutex};

use crate::{
    justification::{AlephJustification, SessionInfo, SessionInfoProvider, Verifier},
    last_block_of_session, session_id_from_block_num,
    testing::mocks::{AcceptancePolicy, TBlock, THash, TNumber},
    SessionPeriod,
};

pub(crate) struct VerifierWrapper {
    acceptance_policy: Arc<Mutex<AcceptancePolicy>>,
}

impl Verifier<TBlock> for VerifierWrapper {
    fn verify(&self, _justification: &AlephJustification, _hash: THash) -> bool {
        self.acceptance_policy.lock().unwrap().accepts()
    }
}

pub(crate) struct SessionInfoProviderImpl {
    session_period: SessionPeriod,
    acceptance_policy: Arc<Mutex<AcceptancePolicy>>,
}

impl SessionInfoProviderImpl {
    pub(crate) fn new(session_period: SessionPeriod, acceptance_policy: AcceptancePolicy) -> Self {
        Self {
            session_period,
            acceptance_policy: Arc::new(Mutex::new(acceptance_policy)),
        }
    }
}

#[async_trait::async_trait]
impl SessionInfoProvider<TBlock, VerifierWrapper> for SessionInfoProviderImpl {
    async fn for_block_num(&self, number: TNumber) -> SessionInfo<TBlock, VerifierWrapper> {
        let current_session = session_id_from_block_num(number, self.session_period);
        SessionInfo {
            current_session,
            last_block_height: last_block_of_session(current_session, self.session_period),
            verifier: match &*self.acceptance_policy.lock().unwrap() {
                AcceptancePolicy::Unavailable => None,
                _ => Some(VerifierWrapper {
                    acceptance_policy: self.acceptance_policy.clone(),
                }),
            },
        }
    }
}
