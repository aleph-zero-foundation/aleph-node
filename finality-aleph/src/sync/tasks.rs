use std::{
    collections::HashSet,
    fmt::{Display, Error as FmtError, Formatter},
    time::Duration,
};

use rand::{thread_rng, Rng};

use crate::{
    sync::{
        data::PreRequest, forest::Interest, handler::InterestProvider, BlockIdFor, Header,
        Justification, PeerId,
    },
    BlockIdentifier,
};

const MIN_DELAY: Duration = Duration::from_millis(300);
const ADDITIONAL_DELAY: Duration = Duration::from_millis(200);

// The delay is the minimum delay, plus uniformly randomly chosen multiple of additional delay,
// linear with the ettempt number.
fn delay_for_attempt(attempt: u32) -> Duration {
    MIN_DELAY
        + ADDITIONAL_DELAY
            .mul_f32(thread_rng().gen())
            .saturating_mul(attempt)
}

/// A task for requesting blocks. Keeps track of how many times it was executed.
pub struct RequestTask<BI: BlockIdentifier> {
    id: BI,
    tries: u32,
}

impl<BI: BlockIdentifier> Display for RequestTask<BI> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "block request for {:?}, attempt {}", self.id, self.tries)
    }
}

type DelayedTask<BI> = (RequestTask<BI>, Duration);

/// What do to with the task, either ignore or perform a request and add a delayed task.
pub enum Action<I: PeerId, J: Justification> {
    Ignore,
    Request(PreRequest<I, J>, DelayedTask<BlockIdFor<J>>),
}

impl<BI: BlockIdentifier> RequestTask<BI> {
    /// A new task for requesting block with the provided ID.
    pub fn new(id: BI) -> Self {
        RequestTask { id, tries: 0 }
    }

    /// Process the task.
    pub fn process<I, J>(self, interest_provider: InterestProvider<I, J>) -> Action<I, J>
    where
        I: PeerId,
        J: Justification,
        J::Header: Header<Identifier = BI>,
    {
        let RequestTask { id, tries } = self;
        match interest_provider.get(&id) {
            Interest::Required {
                branch_knowledge,
                know_most,
            } => {
                // Every second time we request from a random peer rather than the one we expect to
                // have it.
                let know_most = match tries % 2 == 0 {
                    true => know_most,
                    false => HashSet::new(),
                };
                let tries = tries + 1;
                Action::Request(
                    PreRequest::new(id.clone(), branch_knowledge, know_most),
                    (
                        RequestTask {
                            id: id.clone(),
                            tries,
                        },
                        delay_for_attempt(tries),
                    ),
                )
            }
            Interest::Uninterested => Action::Ignore,
        }
    }
}
