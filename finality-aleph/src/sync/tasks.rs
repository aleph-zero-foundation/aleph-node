use std::{
    collections::HashSet,
    fmt::{Display, Error as FmtError, Formatter},
    time::Duration,
};

use rand::{thread_rng, Rng};

use crate::{
    block::{Justification, UnverifiedHeader, UnverifiedHeaderFor},
    sync::{
        data::{MaybeHeader, PreRequest},
        forest::Interest,
        handler::InterestProvider,
        PeerId,
    },
    BlockId,
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
pub struct RequestTask {
    id: BlockId,
    tries: u32,
}

impl Display for RequestTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "block request for {:?}, attempt {}", self.id, self.tries)
    }
}

type DelayedTask = (RequestTask, Duration);

/// What do to with the task, either ignore or perform a request and add a delayed task.
pub enum Action<UH: UnverifiedHeader, I: PeerId> {
    Ignore,
    Request(PreRequest<UH, I>, DelayedTask),
}

impl RequestTask {
    /// A new task for requesting block with the provided ID.
    pub fn new(id: BlockId) -> Self {
        RequestTask { id, tries: 0 }
    }

    /// Process the task.
    pub fn process<I, J>(
        self,
        interest_provider: InterestProvider<I, J>,
    ) -> Action<UnverifiedHeaderFor<J>, I>
    where
        I: PeerId,
        J: Justification,
    {
        let RequestTask { id, tries } = self;
        match interest_provider.get(&id) {
            Interest::Required {
                header,
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
                let pre_request = match header {
                    MaybeHeader::Header(header) => {
                        PreRequest::new(header, branch_knowledge, know_most)
                    }
                    MaybeHeader::Id(id) => {
                        PreRequest::new_headerless(id, branch_knowledge, know_most)
                    }
                };
                Action::Request(
                    pre_request,
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
