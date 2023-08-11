use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::{Display, Error as FmtError, Formatter},
    sync::Arc,
};

use futures::channel::mpsc::{self, UnboundedSender};
use parking_lot::Mutex;

use crate::{
    nodes::VERIFIER_CACHE_SIZE,
    session::{SessionBoundaryInfo, SessionId},
    sync::{
        mock::{MockBlock, MockHeader, MockIdentifier, MockJustification, MockNotification},
        Block, BlockImport, BlockStatus, ChainStatus, ChainStatusNotifier, FinalizationStatus,
        Finalizer, Header, Justification as JustificationT, Verifier,
    },
    BlockIdentifier,
};

#[derive(Clone, Debug)]
struct BackendStorage {
    session_boundary_info: SessionBoundaryInfo,
    blockchain: HashMap<MockIdentifier, MockBlock>,
    finalized: Vec<MockIdentifier>,
    prune_candidates: HashSet<MockIdentifier>,
}

#[derive(Clone, Debug)]
pub struct Backend {
    inner: Arc<Mutex<BackendStorage>>,
    notification_sender: UnboundedSender<MockNotification>,
}

fn is_predecessor(
    blockchain: &HashMap<MockIdentifier, MockBlock>,
    id: &MockIdentifier,
    maybe_predecessor: &MockIdentifier,
    definitely_not: &HashSet<MockIdentifier>,
    definitely: &HashSet<MockIdentifier>,
) -> bool {
    let mut header = blockchain.get(id).expect("should exist").header();
    while let Some(parent) = header.parent_id() {
        if header.id().number() != parent.number() + 1 {
            break;
        }
        if parent.number() < maybe_predecessor.number() {
            break;
        }
        if &parent == maybe_predecessor {
            return true;
        }
        if definitely.contains(&parent) {
            return true;
        }
        if definitely_not.contains(&parent) {
            return false;
        }
        header = match blockchain.get(&parent) {
            Some(block) => block.header(),
            None => return false,
        }
    }
    false
}

impl Backend {
    pub fn setup(
        session_boundary_info: SessionBoundaryInfo,
    ) -> (Self, impl ChainStatusNotifier<MockHeader>) {
        let (notification_sender, notification_receiver) = mpsc::unbounded();

        (
            Backend::new(notification_sender, session_boundary_info),
            notification_receiver,
        )
    }

    fn new(
        notification_sender: UnboundedSender<MockNotification>,
        session_boundary_info: SessionBoundaryInfo,
    ) -> Self {
        // genesis has fixed hash to allow creating multiple compatible Backends
        let header = MockHeader::genesis();
        let id = header.id();

        let block = MockBlock {
            header: header.clone(),
            justification: Some(MockJustification::for_header(header)),
            is_correct: true,
        };

        let storage = Arc::new(Mutex::new(BackendStorage {
            session_boundary_info,
            blockchain: HashMap::from([(id.clone(), block)]),
            finalized: vec![id],
            prune_candidates: HashSet::new(),
        }));

        Self {
            inner: storage,
            notification_sender,
        }
    }

    fn notify_imported(&self, header: MockHeader) {
        self.notification_sender
            .unbounded_send(MockNotification::BlockImported(header))
            .expect("notification receiver is open");
    }

    fn notify_finalized(&self, header: MockHeader) {
        self.notification_sender
            .unbounded_send(MockNotification::BlockFinalized(header))
            .expect("notification receiver is open");
    }

    fn prune(&self) {
        let top_finalized_id = &self
            .top_finalized()
            .expect("should be at least genesis")
            .header()
            .id();
        let mut storage = self.inner.lock();
        let mut to_prune = HashSet::new();
        let mut definitely_correct = HashSet::new();
        for id in &storage.prune_candidates {
            if storage.finalized.get(id.number() as usize) == Some(id)
                || is_predecessor(
                    &storage.blockchain,
                    id,
                    top_finalized_id,
                    &to_prune,
                    &definitely_correct,
                )
            {
                definitely_correct.insert(id.clone());
            } else {
                to_prune.insert(id.clone());
            }
        }
        for id in to_prune {
            storage.blockchain.remove(&id);
            storage.prune_candidates.remove(&id);
        }
    }
}

#[derive(Debug)]
pub struct FinalizerError;

impl Display for FinalizerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{self:?}")
    }
}

impl Finalizer<MockJustification> for Backend {
    type Error = FinalizerError;

    fn finalize(&self, justification: MockJustification) -> Result<(), Self::Error> {
        if !justification.is_correct {
            panic!("finalizing block with an incorrect justification: {justification:?}");
        }

        let mut storage = self.inner.lock();

        let header = justification.header();
        let parent_id = match justification.header().parent_id() {
            Some(id) => id,
            None => panic!("finalizing block without specified parent: {header:?}"),
        };

        if storage.blockchain.get(&parent_id).is_none() {
            panic!("finalizing block without imported parent: {header:?}")
        }

        let header = justification.header().clone();
        let finalizing_id = header.id();
        let block = match storage.blockchain.get_mut(&finalizing_id) {
            Some(block) => block,
            None => panic!("finalizing a not imported block: {header:?}"),
        };

        block.finalize(justification);

        let last_number = match storage.finalized.last() {
            Some(id) => id.number,
            None => 0,
        };

        // Check if the previous block was finalized, or this is the last block of the current
        // session
        let allowed_numbers = match storage.finalized.last() {
            Some(id) => [
                id.number + 1,
                storage.session_boundary_info.last_block_of_session(
                    storage
                        .session_boundary_info
                        .session_id_from_block_num(id.number + 1),
                ),
            ],
            None => [
                0,
                storage
                    .session_boundary_info
                    .last_block_of_session(SessionId(0)),
            ],
        };

        if !allowed_numbers.contains(&finalizing_id.number) {
            panic!("finalizing a block that is not a child of top finalized (round {:?}), nor the last of a session (round {:?}): round {:?}", allowed_numbers[0], allowed_numbers[1], finalizing_id.number);
        }

        let mut blocks_to_finalize = VecDeque::new();
        let mut block_to_finalize = finalizing_id.clone();

        // Finalize also blocks that lead up to last finalized
        while block_to_finalize.number != last_number {
            blocks_to_finalize.push_front(block_to_finalize.clone());
            block_to_finalize = storage
                .blockchain
                .get(&block_to_finalize)
                .expect("We already checked that")
                .header
                .parent
                .clone()
                .expect("We already checked parent exists");
        }

        // Actually check if we are not finalizing fork
        let first_to_finalize = blocks_to_finalize
            .front()
            .expect("At least one block is being finalized");

        let parent_of_first_block_to_finalize = storage
            .blockchain
            .get(first_to_finalize)
            .expect("We already checked that")
            .header
            .parent
            .clone()
            .expect("We already checked parent exists");
        let last_finalized = storage
            .finalized
            .last()
            .expect("At least one block is always finalized");
        if parent_of_first_block_to_finalize != *last_finalized {
            panic!("finalizing a block that is not a child of top finalized.");
        }

        for id in &blocks_to_finalize {
            storage.prune_candidates.remove(id);
        }
        storage.finalized.extend(blocks_to_finalize);
        std::mem::drop(storage);
        self.prune();

        self.notify_finalized(header);

        Ok(())
    }
}

impl BlockImport<MockBlock> for Backend {
    fn import_block(&mut self, block: MockBlock) {
        if !block.verify() {
            return;
        }

        let top_finalized_number = self
            .top_finalized()
            .expect("should be at least genesis")
            .header()
            .id()
            .number();
        let mut storage = self.inner.lock();

        let parent_id = match block.parent_id() {
            Some(id) => id,
            None => return,
        };

        if storage.blockchain.contains_key(&block.id())
            || !storage.blockchain.contains_key(&parent_id)
            || block.id().number() != parent_id.number() + 1
            || block.id().number() <= top_finalized_number
        {
            return;
        }

        storage.prune_candidates.insert(block.id());
        storage.blockchain.insert(block.id(), block.clone());

        self.notify_imported(block.header);
    }
}

#[derive(Debug)]
pub struct StatusError;

impl Display for StatusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{self:?}")
    }
}

impl ChainStatus<MockBlock, MockJustification> for Backend {
    type Error = StatusError;

    fn status_of(&self, id: MockIdentifier) -> Result<BlockStatus<MockJustification>, Self::Error> {
        let storage = self.inner.lock();
        let block = match storage.blockchain.get(&id) {
            Some(block) => block,
            None => return Ok(BlockStatus::Unknown),
        };

        if let Some(justification) = block.justification.clone() {
            Ok(BlockStatus::Justified(justification))
        } else {
            Ok(BlockStatus::Present(block.header().clone()))
        }
    }

    fn block(&self, id: MockIdentifier) -> Result<Option<MockBlock>, Self::Error> {
        Ok(self.inner.lock().blockchain.get(&id).cloned())
    }

    fn finalized_at(
        &self,
        number: u32,
    ) -> Result<FinalizationStatus<MockJustification>, Self::Error> {
        use FinalizationStatus::*;

        if number > self.top_finalized()?.header.id.number {
            return Ok(NotFinalized);
        }

        let storage = self.inner.lock();
        let id = match storage.finalized.get(number as usize) {
            Some(id) => id,
            None => return Ok(NotFinalized),
        };

        let block = storage.blockchain.get(id).ok_or(StatusError)?;

        if let Some(j) = block.justification.clone() {
            return Ok(FinalizedWithJustification(j));
        }

        Ok(FinalizedByDescendant(block.header.clone()))
    }

    fn best_block(&self) -> Result<MockHeader, Self::Error> {
        Err(Self::Error {})
    }

    fn top_finalized(&self) -> Result<MockJustification, Self::Error> {
        let storage = self.inner.lock();
        let id = storage
            .finalized
            .last()
            .expect("there is a top finalized")
            .clone();
        storage
            .blockchain
            .get(&id)
            .and_then(|b| b.justification.clone())
            .ok_or(StatusError)
    }

    fn children(&self, id: MockIdentifier) -> Result<Vec<MockHeader>, Self::Error> {
        match self.status_of(id.clone())? {
            BlockStatus::Unknown => Err(StatusError),
            _ => {
                let storage = self.inner.lock();
                for (stored_id, block) in storage.blockchain.iter() {
                    if stored_id.number() == id.number + 1 {
                        return Ok(Vec::from([block.header().clone()]));
                    }
                }
                Ok(Vec::new())
            }
        }
    }
}

#[derive(Debug)]
pub enum VerifierError {
    IncorrectJustification,
    IncorrectSession,
}

impl Display for VerifierError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{:?}", self)
    }
}

impl Verifier<MockJustification> for Backend {
    type Error = VerifierError;

    fn verify(
        &mut self,
        justification: MockJustification,
    ) -> Result<MockJustification, Self::Error> {
        let top_number = self
            .top_finalized()
            .expect("should be at least genesis")
            .header
            .id
            .number;
        let storage = self.inner.lock();
        let current_session = storage
            .session_boundary_info
            .session_id_from_block_num(top_number);
        let justification_session = storage
            .session_boundary_info
            .session_id_from_block_num(justification.id().number);
        if justification_session.0 > current_session.0 + 1
            || current_session.0 + 1 - justification_session.0 >= VERIFIER_CACHE_SIZE as u32
        {
            return Err(Self::Error::IncorrectSession);
        }
        match justification.is_correct {
            true => Ok(justification),
            false => Err(Self::Error::IncorrectJustification),
        }
    }
}
