use std::{
    collections::HashMap,
    fmt::{Display, Error as FmtError, Formatter},
    sync::Arc,
};

use futures::channel::mpsc::{self, UnboundedSender};
use parking_lot::Mutex;

use crate::sync::{
    mock::{MockHeader, MockIdentifier, MockJustification, MockNotification},
    BlockIdentifier, BlockStatus, ChainStatus, ChainStatusNotifier, Finalizer, Header,
    Justification as JustificationT,
};

#[derive(Clone, Debug)]
struct MockBlock {
    header: MockHeader,
    justification: Option<MockJustification>,
}

impl MockBlock {
    fn new(header: MockHeader) -> Self {
        Self {
            header,
            justification: None,
        }
    }

    fn header(&self) -> MockHeader {
        self.header.clone()
    }

    fn finalize(&mut self, justification: MockJustification) {
        self.justification = Some(justification);
    }
}

#[derive(Clone, Debug)]
struct BackendStorage {
    blockchain: HashMap<MockIdentifier, MockBlock>,
    finalized: Vec<MockIdentifier>,
    best_block: MockIdentifier,
    genesis_block: MockIdentifier,
}

#[derive(Clone, Debug)]
pub struct Backend {
    inner: Arc<Mutex<BackendStorage>>,
    notification_sender: UnboundedSender<MockNotification>,
}

fn is_predecessor(
    storage: &HashMap<MockIdentifier, MockBlock>,
    mut header: MockHeader,
    maybe_predecessor: MockIdentifier,
) -> bool {
    while let Some(parent) = header.parent_id() {
        if header.id().number() != parent.number() + 1 {
            break;
        }
        if parent == maybe_predecessor {
            return true;
        }

        header = match storage.get(&parent) {
            Some(block) => block.header(),
            None => return false,
        }
    }
    false
}

impl Backend {
    pub fn setup() -> (Self, impl ChainStatusNotifier<MockIdentifier>) {
        let (notification_sender, notification_receiver) = mpsc::unbounded();

        (Backend::new(notification_sender), notification_receiver)
    }

    fn new(notification_sender: UnboundedSender<MockNotification>) -> Self {
        let header = MockHeader::random_parentless(0);
        let id = header.id();

        let block = MockBlock {
            header: header.clone(),
            justification: Some(MockJustification::for_header(header)),
        };

        let storage = Arc::new(Mutex::new(BackendStorage {
            blockchain: HashMap::from([(id.clone(), block)]),
            finalized: vec![id.clone()],
            best_block: id.clone(),
            genesis_block: id,
        }));

        Self {
            inner: storage,
            notification_sender,
        }
    }

    fn notify_imported(&self, id: MockIdentifier) {
        self.notification_sender
            .unbounded_send(MockNotification::BlockImported(id))
            .expect("notification receiver is open");
    }

    fn notify_finalized(&self, id: MockIdentifier) {
        self.notification_sender
            .unbounded_send(MockNotification::BlockFinalized(id))
            .expect("notification receiver is open");
    }

    pub fn import(&self, header: MockHeader) {
        let mut storage = self.inner.lock();

        let parent_id = match header.parent_id() {
            Some(id) => id,
            None => panic!("importing block without a parent: {:?}", header),
        };

        if storage.blockchain.contains_key(&header.id()) {
            panic!("importing an already imported block: {:?}", header)
        }

        if !storage.blockchain.contains_key(&parent_id) {
            panic!("importing block without an imported parent: {:?}", header)
        }

        if header.id().number() != parent_id.number() + 1 {
            panic!("importing block without a correct parent: {:?}", header)
        }

        if header.id().number() > storage.best_block.number()
            && is_predecessor(
                &storage.blockchain,
                header.clone(),
                storage.best_block.clone(),
            )
        {
            storage.best_block = header.id();
        }

        storage
            .blockchain
            .insert(header.id(), MockBlock::new(header.clone()));

        self.notify_imported(header.id());
    }
}

#[derive(Debug)]
pub struct FinalizerError;

impl Display for FinalizerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{:?}", self)
    }
}

impl Finalizer<MockJustification> for Backend {
    type Error = FinalizerError;

    fn finalize(&self, justification: MockJustification) -> Result<(), Self::Error> {
        if !justification.is_correct {
            panic!(
                "finalizing block with an incorrect justification: {:?}",
                justification
            );
        }

        let mut storage = self.inner.lock();

        let header = justification.header();
        let parent_id = match justification.header().parent_id() {
            Some(id) => id,
            None => panic!("finalizing block without a parent: {:?}", header),
        };

        let parent_block = match storage.blockchain.get(&parent_id) {
            Some(block) => block,
            None => panic!("finalizing block without an imported parent: {:?}", header),
        };

        if parent_block.justification.is_none() {
            panic!("finalizing block without a finalized parent: {:?}", header);
        }

        if &parent_id != storage.finalized.last().expect("there is a top finalized") {
            panic!(
                "finalizing block whose parent is not top finalized: {:?}. Top is {:?}",
                header,
                storage.finalized.last().expect("there is a top finalized")
            );
        }

        let id = justification.header().id();
        let block = match storage.blockchain.get_mut(&id) {
            Some(block) => block,
            None => panic!("finalizing a not imported block: {:?}", header),
        };

        block.finalize(justification);
        storage.finalized.push(id.clone());
        // In case finalization changes best block, we set best block, to top finalized.
        // Whenever a new import happens, best block will update anyway.
        if !is_predecessor(
            &storage.blockchain,
            storage
                .blockchain
                .get(&storage.best_block)
                .unwrap()
                .header(),
            id.clone(),
        ) {
            storage.best_block = id.clone()
        }
        self.notify_finalized(id);

        Ok(())
    }
}

#[derive(Debug)]
pub struct StatusError;

impl Display for StatusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{:?}", self)
    }
}

impl ChainStatus<MockJustification> for Backend {
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
            Ok(BlockStatus::Present(block.header()))
        }
    }

    fn finalized_at(&self, number: u32) -> Result<Option<MockJustification>, Self::Error> {
        let storage = self.inner.lock();
        let id = match storage.finalized.get(number as usize) {
            Some(id) => id,
            None => return Ok(None),
        };
        storage
            .blockchain
            .get(id)
            .ok_or(StatusError)
            .map(|b| b.justification.clone())
    }

    fn best_block(&self) -> Result<MockHeader, Self::Error> {
        let storage = self.inner.lock();
        let id = storage.best_block.clone();
        storage
            .blockchain
            .get(&id)
            .map(|b| b.header())
            .ok_or(StatusError)
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
}
