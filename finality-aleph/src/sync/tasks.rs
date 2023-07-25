use std::{
    collections::HashSet,
    fmt::{Display, Error as FmtError, Formatter},
    time::Duration,
};

use rand::{thread_rng, Rng};

use crate::{
    sync::{
        data::{BranchKnowledge, Request, State},
        forest::Interest,
        handler::InterestProvider,
        BlockIdFor, Header, Justification, PeerId,
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

enum RequestKind {
    HighestJustified,
    Block,
}

impl Display for RequestKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use RequestKind::*;
        match self {
            HighestJustified => write!(f, "highest justified"),
            Block => write!(f, "block"),
        }
    }
}

impl RequestKind {
    fn should_request<I: PeerId, J: Justification>(
        &self,
        interest: Interest<I, J>,
    ) -> Option<(BranchKnowledge<J>, HashSet<I>)> {
        use Interest::*;
        match (interest, self) {
            (
                Required {
                    know_most,
                    branch_knowledge,
                },
                RequestKind::Block,
            )
            | (
                HighestJustified {
                    know_most,
                    branch_knowledge,
                },
                RequestKind::HighestJustified,
            ) => Some((branch_knowledge, know_most)),
            _ => None,
        }
    }
}

/// A task for requesting blocks. Keeps track of how many times it was executed and what kind of
/// request it is.
pub struct RequestTask<BI: BlockIdentifier> {
    id: BI,
    kind: RequestKind,
    tries: u32,
}

impl<BI: BlockIdentifier> Display for RequestTask<BI> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(
            f,
            "{} request for {:?}, attempt {}",
            self.kind, self.id, self.tries
        )
    }
}

/// Data that can be used to generate a request given our state.
pub struct PreRequest<I: PeerId, J: Justification> {
    id: BlockIdFor<J>,
    branch_knowledge: BranchKnowledge<J>,
    know_most: HashSet<I>,
}

impl<I: PeerId, J: Justification> PreRequest<I, J> {
    fn new(id: BlockIdFor<J>, branch_knowledge: BranchKnowledge<J>, know_most: HashSet<I>) -> Self {
        PreRequest {
            id,
            branch_knowledge,
            know_most,
        }
    }

    /// Convert to a request and recipients given a state.
    pub fn with_state(self, state: State<J>) -> (Request<J>, HashSet<I>) {
        let PreRequest {
            id,
            branch_knowledge,
            know_most,
        } = self;
        (Request::new(id, branch_knowledge, state), know_most)
    }
}

type DelayedTask<BI> = (RequestTask<BI>, Duration);

/// What do to with the task, either ignore or perform a request and add a delayed task.
pub enum Action<I: PeerId, J: Justification> {
    Ignore,
    Request(PreRequest<I, J>, DelayedTask<BlockIdFor<J>>),
}

impl<BI: BlockIdentifier> RequestTask<BI> {
    fn new(id: BI, kind: RequestKind) -> Self {
        RequestTask { id, kind, tries: 0 }
    }

    /// A new task for requesting highest justified block with the provided ID.
    pub fn new_highest_justified(id: BI) -> Self {
        RequestTask::new(id, RequestKind::HighestJustified)
    }

    /// A new task for requesting block with the provided ID.
    pub fn new_block(id: BI) -> Self {
        RequestTask::new(id, RequestKind::Block)
    }

    /// Process the task.
    pub fn process<I, J>(self, interest_provider: InterestProvider<I, J>) -> Action<I, J>
    where
        I: PeerId,
        J: Justification,
        J::Header: Header<Identifier = BI>,
    {
        let RequestTask { id, kind, tries } = self;
        match kind.should_request(interest_provider.get(&id)) {
            Some((branch_knowledge, know_most)) => {
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
                            kind,
                            tries,
                        },
                        delay_for_attempt(tries),
                    ),
                )
            }
            None => Action::Ignore,
        }
    }
}
