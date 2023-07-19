use core::marker::PhantomData;
use std::{
    fmt::{Debug, Display, Error as FmtError, Formatter},
    iter,
};

use crate::{
    session::{SessionBoundaryInfo, SessionId},
    sync::{
        data::{NetworkData, Request, State},
        forest::{Error as ForestError, Forest, InitializationError as ForestInitializationError},
        handler::request_handler::{RequestHandler, RequestHandlerError},
        Block, BlockIdFor, BlockImport, ChainStatus, Finalizer, Header, Justification, PeerId,
        Verifier,
    },
    BlockIdentifier,
};

mod request_handler;
pub use request_handler::Action;

use crate::sync::data::{ResponseItem, ResponseItems};

/// Handles for interacting with the blockchain database.
pub struct DatabaseIO<B, J, CS, F, BI>
where
    B: Block,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    chain_status: CS,
    finalizer: F,
    block_importer: BI,
    _phantom: PhantomData<(B, J)>,
}

impl<B, J, CS, F, BI> DatabaseIO<B, J, CS, F, BI>
where
    B: Block,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    pub fn new(chain_status: CS, finalizer: F, block_importer: BI) -> Self {
        Self {
            chain_status,
            finalizer,
            block_importer,
            _phantom: PhantomData,
        }
    }
}

/// Types used by the Handler. For improved readability.
pub trait HandlerTypes {
    /// What can go wrong when handling a piece of data.
    type Error;
}

/// Handler for data incoming from the network.
pub struct Handler<B, I, J, CS, V, F, BI>
where
    B: Block,
    I: PeerId,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    chain_status: CS,
    verifier: V,
    finalizer: F,
    forest: Forest<I, J>,
    session_info: SessionBoundaryInfo,
    block_importer: BI,
    phantom: PhantomData<B>,
}

/// What actions can the handler recommend as a reaction to some data.
#[derive(Clone, Debug)]
pub enum HandleStateAction<B, J>
where
    B: Block,
    J: Justification<Header = B::Header>,
{
    /// A response for the peer that sent us the data.
    Response(NetworkData<B, J>),
    /// A request for the highest justified block that should be performed periodically.
    HighestJustified(BlockIdFor<J>),
    /// Do nothing.
    Noop,
}

impl<B, J> HandleStateAction<B, J>
where
    B: Block,
    J: Justification<Header = B::Header>,
{
    fn response(justification: J::Unverified, other_justification: Option<J::Unverified>) -> Self {
        Self::Response(NetworkData::StateBroadcastResponse(
            justification,
            other_justification,
        ))
    }
}

impl<B, J> From<Option<BlockIdFor<J>>> for HandleStateAction<B, J>
where
    B: Block,
    J: Justification<Header = B::Header>,
{
    fn from(value: Option<BlockIdFor<J>>) -> Self {
        match value {
            Some(id) => Self::HighestJustified(id),
            None => Self::Noop,
        }
    }
}

/// What can go wrong when handling a piece of data.
#[derive(Clone, Debug)]
pub enum Error<B, J, CS, V, F>
where
    J: Justification,
    B: Block<Header = J::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
{
    Verifier(V::Error),
    ChainStatus(CS::Error),
    Finalizer(F::Error),
    Forest(ForestError),
    ForestInitialization(ForestInitializationError<B, J, CS>),
    RequestHandlerError(RequestHandlerError<J, CS::Error>),
    MissingJustification,
    BlockNotImportable,
}

impl<B, J, CS, V, F> Display for Error<B, J, CS, V, F>
where
    J: Justification,
    B: Block<Header = J::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            Verifier(e) => write!(f, "verifier error: {e}"),
            ChainStatus(e) => write!(f, "chain status error: {e}"),
            Finalizer(e) => write!(f, "finalized error: {e}"),
            Forest(e) => write!(f, "forest error: {e}"),
            ForestInitialization(e) => write!(f, "forest initialization error: {e}"),
            MissingJustification => write!(
                f,
                "justification for the last block of a past session missing"
            ),
            BlockNotImportable => {
                write!(f, "cannot import a block that we do not consider required")
            }
            RequestHandlerError(e) => write!(f, "request handler error: {e}"),
        }
    }
}

impl<B, J, CS, V, F> From<ForestError> for Error<B, J, CS, V, F>
where
    J: Justification,
    B: Block<Header = J::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
{
    fn from(e: ForestError) -> Self {
        Error::Forest(e)
    }
}
impl<B, J, CS, V, F> From<RequestHandlerError<J, CS::Error>> for Error<B, J, CS, V, F>
where
    J: Justification,
    B: Block<Header = J::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
{
    fn from(e: RequestHandlerError<J, CS::Error>) -> Self {
        Error::RequestHandlerError(e)
    }
}

impl<B, I, J, CS, V, F, BI> HandlerTypes for Handler<B, I, J, CS, V, F, BI>
where
    B: Block,
    I: PeerId,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    type Error = Error<B, J, CS, V, F>;
}

impl<B, I, J, CS, V, F, BI> Handler<B, I, J, CS, V, F, BI>
where
    B: Block,
    I: PeerId,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    /// New handler with the provided chain interfaces.
    pub fn new(
        database_io: DatabaseIO<B, J, CS, F, BI>,
        verifier: V,
        session_info: SessionBoundaryInfo,
    ) -> Result<Self, <Self as HandlerTypes>::Error> {
        let DatabaseIO {
            chain_status,
            finalizer,
            block_importer,
            ..
        } = database_io;
        let forest = Forest::new(&chain_status).map_err(Error::ForestInitialization)?;
        Ok(Handler {
            chain_status,
            verifier,
            finalizer,
            forest,
            session_info,
            block_importer,
            phantom: PhantomData,
        })
    }

    fn try_finalize(&mut self) -> Result<(), <Self as HandlerTypes>::Error> {
        let mut number = self
            .chain_status
            .top_finalized()
            .map_err(Error::ChainStatus)?
            .header()
            .id()
            .number()
            + 1;
        loop {
            while let Some(justification) = self.forest.try_finalize(&number) {
                self.finalizer
                    .finalize(justification)
                    .map_err(Error::Finalizer)?;
                number += 1;
            }
            number = self
                .session_info
                .last_block_of_session(self.session_info.session_id_from_block_num(number));
            match self.forest.try_finalize(&number) {
                Some(justification) => {
                    self.finalizer
                        .finalize(justification)
                        .map_err(Error::Finalizer)?;
                    number += 1;
                }
                None => return Ok(()),
            };
        }
    }

    /// Inform the handler that a block has been imported.
    pub fn block_imported(
        &mut self,
        header: J::Header,
    ) -> Result<(), <Self as HandlerTypes>::Error> {
        self.forest.update_body(&header)?;
        self.try_finalize()
    }

    /// Handle a request for potentially substantial amounts of data.
    ///
    /// Returns what action we should take in response to the request.
    /// We either do nothing, request new interesting block to us or send a response containing
    /// path of justifications, blocks and headers. We try to be as helpful as we can, sometimes
    /// including more information from what was requested, sometimes ignoring their requested id
    /// if we know it makes sense.
    pub fn handle_request(
        &mut self,
        request: Request<J>,
    ) -> Result<Action<B, J>, <Self as HandlerTypes>::Error> {
        let request_handler = RequestHandler::new(&self.chain_status, &self.session_info);

        Ok(match request_handler.action(request)? {
            Action::RequestBlock(id)
                if !self.forest.update_block_identifier(&id, None, true)? =>
            {
                Action::Noop
            }
            action => action,
        })
    }

    /// Handle a single unverified justification.
    /// Return `Some(id)` if this justification was higher than the previously known highest justification.
    fn handle_justification(
        &mut self,
        justification: J::Unverified,
        maybe_peer: Option<I>,
    ) -> Result<Option<BlockIdFor<J>>, <Self as HandlerTypes>::Error> {
        let justification = self
            .verifier
            .verify(justification)
            .map_err(Error::Verifier)?;
        let id = justification.header().id();
        let maybe_id = match self
            .forest
            .update_justification(justification, maybe_peer)?
        {
            true => Some(id),
            false => None,
        };
        self.try_finalize()?;
        Ok(maybe_id)
    }

    /// Handle a justification from user returning the action we should take.
    pub fn handle_justification_from_user(
        &mut self,
        justification: J::Unverified,
    ) -> Result<Option<BlockIdFor<J>>, <Self as HandlerTypes>::Error> {
        self.handle_justification(justification, None)
    }

    /// Handle a state response returning the action we should take, and possibly an error.
    pub fn handle_state_response(
        &mut self,
        justification: J::Unverified,
        maybe_justification: Option<J::Unverified>,
        peer: I,
    ) -> (Option<BlockIdFor<J>>, Option<<Self as HandlerTypes>::Error>) {
        let mut maybe_id = None;

        for justification in iter::once(justification).chain(maybe_justification) {
            maybe_id = match self.handle_justification(justification, Some(peer.clone())) {
                Ok(id) => id,
                Err(e) => return (maybe_id, Some(e)),
            };
        }

        (maybe_id, None)
    }

    /// Handle a request response returning the id of the new highest justified block
    /// if there is some, and possibly an error.
    pub fn handle_request_response(
        &mut self,
        response_items: ResponseItems<B, J>,
        peer: I,
    ) -> (Option<BlockIdFor<J>>, Option<<Self as HandlerTypes>::Error>) {
        let mut highest_justified = None;
        for item in response_items {
            match item {
                ResponseItem::Justification(j) => {
                    match self.handle_justification(j, Some(peer.clone())) {
                        Ok(Some(id)) => highest_justified = Some(id),
                        Err(e) => return (highest_justified, Some(e)),
                        _ => {}
                    }
                }
                ResponseItem::Header(h) => {
                    if let Err(e) = self.forest.update_required_header(&h, Some(peer.clone())) {
                        return (highest_justified, Some(Error::Forest(e)));
                    }
                }
                ResponseItem::Block(b) => {
                    match self.forest.importable(&b.header().id()) {
                        true => self.block_importer.import_block(b),
                        false => return (highest_justified, Some(Error::BlockNotImportable)),
                    };
                }
            }
        }

        (highest_justified, None)
    }

    fn last_justification_unverified(
        &self,
        session: SessionId,
    ) -> Result<J::Unverified, <Self as HandlerTypes>::Error> {
        use Error::*;
        Ok(self
            .chain_status
            .finalized_at(self.session_info.last_block_of_session(session))
            .map_err(ChainStatus)?
            .has_justification()
            .ok_or(MissingJustification)?
            .into_unverified())
    }

    /// Handle a state broadcast returning the actions we should take in response.
    pub fn handle_state(
        &mut self,
        state: State<J>,
        peer: I,
    ) -> Result<HandleStateAction<B, J>, <Self as HandlerTypes>::Error> {
        use Error::*;
        let remote_top_number = state.top_justification().id().number();
        let local_top = self.chain_status.top_finalized().map_err(ChainStatus)?;
        let local_top_number = local_top.header().id().number();
        let remote_session = self
            .session_info
            .session_id_from_block_num(remote_top_number);
        let local_session = self
            .session_info
            .session_id_from_block_num(local_top_number);
        match local_session.0.checked_sub(remote_session.0) {
            // remote session number larger than ours, we can try to import the justification
            None => Ok(self
                .handle_justification(state.top_justification(), Some(peer))?
                .into()),
            // same session
            Some(0) => match remote_top_number >= local_top_number {
                // remote top justification higher than ours, we can import the justification
                true => Ok(self
                    .handle_justification(state.top_justification(), Some(peer))?
                    .into()),
                // remote top justification lower than ours, we can send a response
                false => Ok(HandleStateAction::response(
                    local_top.into_unverified(),
                    None,
                )),
            },
            // remote lags one session behind
            Some(1) => Ok(HandleStateAction::response(
                self.last_justification_unverified(remote_session)?,
                Some(local_top.into_unverified()),
            )),
            // remote lags multiple sessions behind
            Some(2..) => Ok(HandleStateAction::response(
                self.last_justification_unverified(remote_session)?,
                Some(self.last_justification_unverified(SessionId(remote_session.0 + 1))?),
            )),
        }
    }

    /// The current state of our database.
    pub fn state(&self) -> Result<State<J>, <Self as HandlerTypes>::Error> {
        let top_justification = self
            .chain_status
            .top_finalized()
            .map_err(Error::ChainStatus)?
            .into_unverified();
        Ok(State::new(top_justification))
    }

    /// The forest held by this handler, read only.
    pub fn forest(&self) -> &Forest<I, J> {
        &self.forest
    }

    /// Handle an internal block request.
    /// Returns `true` if this was the first time something indicated interest in this block.
    pub fn handle_internal_request(
        &mut self,
        id: &BlockIdFor<J>,
    ) -> Result<bool, <Self as HandlerTypes>::Error> {
        let should_request = self.forest.update_block_identifier(id, None, true)?;

        Ok(should_request)
    }
}

#[cfg(test)]
mod tests {
    use super::{DatabaseIO, HandleStateAction, Handler};
    use crate::{
        session::SessionBoundaryInfo,
        sync::{
            data::{BranchKnowledge::*, NetworkData, Request, ResponseItem, ResponseItems, State},
            handler::Action,
            mock::{
                Backend, MockBlock, MockHeader, MockIdentifier, MockJustification, MockPeerId,
                MockVerifier,
            },
            ChainStatus, Header, Justification,
        },
        BlockIdentifier, BlockNumber, SessionPeriod,
    };

    type MockHandler =
        Handler<MockBlock, MockPeerId, MockJustification, Backend, MockVerifier, Backend, Backend>;

    const SESSION_PERIOD: usize = 20;

    fn setup() -> (MockHandler, Backend, impl Send) {
        let (backend, _keep) = Backend::setup(SESSION_PERIOD);
        let verifier = MockVerifier {};
        let database_io = DatabaseIO::new(backend.clone(), backend.clone(), backend.clone());
        let handler = Handler::new(
            database_io,
            verifier,
            SessionBoundaryInfo::new(SessionPeriod(20)),
        )
        .expect("mock backend works");
        (handler, backend, _keep)
    }

    fn import_branch(backend: &Backend, branch_length: usize) -> Vec<MockHeader> {
        let result: Vec<_> = backend
            .best_block()
            .expect("mock backend works")
            .random_branch()
            .take(branch_length)
            .collect();
        for header in &result {
            backend.import(header.clone());
        }
        result
    }

    #[test]
    fn finalizes_imported_and_justified() {
        let (mut handler, backend, _keep) = setup();
        let header = import_branch(&backend, 1)[0].clone();
        handler
            .block_imported(header.clone())
            .expect("importing in order");
        let justification = MockJustification::for_header(header);
        let peer = rand::random();
        assert!(
            handler
                .handle_justification(justification.clone().into_unverified(), Some(peer))
                .expect("correct justification")
                == Some(justification.id())
        );
        assert_eq!(
            backend.top_finalized().expect("mock backend works"),
            justification
        );
    }

    #[test]
    fn requests_missing_justifications_without_blocks() {
        let (mut handler, backend, _keep) = setup();
        let peer = rand::random();
        // skip the first justification, now every next added justification
        // should spawn a new task
        for justification in import_branch(&backend, 5)
            .into_iter()
            .map(MockJustification::for_header)
            .skip(1)
        {
            assert!(
                handler
                    .handle_justification(justification.clone().into_unverified(), Some(peer))
                    .expect("correct justification")
                    == Some(justification.id())
            );
        }
    }

    #[test]
    fn requests_missing_justifications_with_blocks() {
        let (mut handler, backend, _keep) = setup();
        let peer = rand::random();
        let justifications: Vec<MockJustification> = import_branch(&backend, 5)
            .into_iter()
            .map(MockJustification::for_header)
            .collect();
        for justification in justifications.iter() {
            handler
                .block_imported(justification.header().clone())
                .expect("importing in order");
        }
        // skip the first justification, now every next added justification
        // should spawn a new task
        for justification in justifications.into_iter().skip(1) {
            assert!(
                handler
                    .handle_justification(justification.clone().into_unverified(), Some(peer))
                    .expect("correct justification")
                    == Some(justification.id())
            );
        }
    }

    #[test]
    fn initializes_forest_properly() {
        let (backend, _keep) = Backend::setup(SESSION_PERIOD);
        let header = import_branch(&backend, 1)[0].clone();
        // header already imported, Handler should initialize Forest properly
        let verifier = MockVerifier {};
        let database_io = DatabaseIO::new(backend.clone(), backend.clone(), backend.clone());
        let mut handler = Handler::new(
            database_io,
            verifier,
            SessionBoundaryInfo::new(SessionPeriod(20)),
        )
        .expect("mock backend works");
        let justification = MockJustification::for_header(header);
        let peer: MockPeerId = rand::random();
        assert!(
            handler
                .handle_justification(justification.clone().into_unverified(), Some(peer))
                .expect("correct justification")
                == Some(justification.id())
        );
        // should be auto-finalized, if Forest knows about imported body
        assert_eq!(
            backend.top_finalized().expect("mock backend works"),
            justification
        );
    }

    #[test]
    fn finalizes_justified_and_imported() {
        let (mut handler, backend, _keep) = setup();
        let header = import_branch(&backend, 1)[0].clone();
        let justification = MockJustification::for_header(header.clone());
        let peer = rand::random();
        match handler
            .handle_justification(justification.clone().into_unverified(), Some(peer))
            .expect("correct justification")
        {
            Some(id) => assert_eq!(id, header.id()),
            None => panic!("expected an id, got nothing"),
        }
        handler.block_imported(header).expect("importing in order");
        assert_eq!(
            backend.top_finalized().expect("mock backend works"),
            justification
        );
    }

    #[test]
    fn handles_state_with_large_difference() {
        let (mut handler, backend, _keep) = setup();
        let initial_state = handler.state().expect("state works");
        let peer = rand::random();
        let justifications: Vec<MockJustification> = import_branch(&backend, 43)
            .into_iter()
            .map(MockJustification::for_header)
            .collect();
        let last_from_first_session = justifications[18].clone().into_unverified();
        let last_from_second_session = justifications[38].clone().into_unverified();
        for justification in justifications.into_iter() {
            handler
                .block_imported(justification.header().clone())
                .expect("importing in order");
            handler
                .handle_justification(justification.clone().into_unverified(), Some(peer))
                .expect("correct justification");
        }
        match handler
            .handle_state(initial_state, peer)
            .expect("correct justification")
        {
            HandleStateAction::Response(NetworkData::StateBroadcastResponse(
                justification,
                maybe_justification,
            )) => {
                assert_eq!(justification, last_from_first_session);
                assert_eq!(maybe_justification, Some(last_from_second_session));
            }
            other_action => panic!("expected a response with justifications, got {other_action:?}"),
        }
    }

    #[test]
    fn handles_state_with_medium_difference() {
        let (mut handler, backend, _keep) = setup();
        let initial_state = handler.state().expect("state works");
        let peer = rand::random();
        let justifications: Vec<MockJustification> = import_branch(&backend, 23)
            .into_iter()
            .map(MockJustification::for_header)
            .collect();
        let last_from_first_session = justifications[18].clone().into_unverified();
        let top = justifications[22].clone().into_unverified();
        for justification in justifications.into_iter() {
            handler
                .block_imported(justification.header().clone())
                .expect("importing in order");
            handler
                .handle_justification(justification.clone().into_unverified(), Some(peer))
                .expect("correct justification");
        }
        match handler
            .handle_state(initial_state, peer)
            .expect("correct justification")
        {
            HandleStateAction::Response(NetworkData::StateBroadcastResponse(
                justification,
                maybe_justification,
            )) => {
                assert_eq!(justification, last_from_first_session);
                assert_eq!(maybe_justification, Some(top));
            }
            other_action => panic!("expected a response with justifications, got {other_action:?}"),
        }
    }

    #[test]
    fn handles_state_with_small_difference() {
        let (mut handler, backend, _keep) = setup();
        let initial_state = handler.state().expect("state works");
        let peer = rand::random();
        let justifications: Vec<MockJustification> = import_branch(&backend, 13)
            .into_iter()
            .map(MockJustification::for_header)
            .collect();
        let top = justifications[12].clone().into_unverified();
        for justification in justifications.into_iter() {
            handler
                .block_imported(justification.header().clone())
                .expect("importing in order");
            handler
                .handle_justification(justification.clone().into_unverified(), Some(peer))
                .expect("correct justification");
        }
        match handler
            .handle_state(initial_state, peer)
            .expect("correct justification")
        {
            HandleStateAction::Response(NetworkData::StateBroadcastResponse(
                justification,
                maybe_justification,
            )) => {
                assert_eq!(justification, top);
                assert!(maybe_justification.is_none());
            }
            other_action => panic!("expected a response with justifications, got {other_action:?}"),
        }
    }

    fn setup_request_tests(
        handler: &mut MockHandler,
        backend: &Backend,
        branch_length: usize,
        finalize_up_to: usize,
    ) -> (Vec<MockJustification>, Vec<MockBlock>) {
        let peer = rand::random();
        let headers = import_branch(backend, branch_length);
        let mut justifications: Vec<_> = headers
            .clone()
            .into_iter()
            .take(finalize_up_to - 1) // 0 is already imported
            .map(MockJustification::for_header)
            .collect();
        for justification in &justifications {
            let number = justification.header().id().number();
            handler
                .block_imported(justification.header().clone())
                .expect("importing in order");
            // skip some justifications, but always keep the last of the session
            // justifications right before the last will be skipped in response
            if number % 20 < 10 || number % 20 > 14 {
                handler
                    .handle_justification(justification.clone().into_unverified(), Some(peer))
                    .expect("correct justification");
            }
        }

        let blocks = headers
            .into_iter()
            .map(|h| backend.block(h.id()).unwrap().unwrap())
            .collect();

        // filter justifications, these are supposed to be included in the response
        justifications.retain(|j| {
            let number = j.header().id().number();
            number % 20 < 10 || number % 20 == 19
        });

        (justifications, blocks)
    }

    #[test]
    fn handles_request_too_far_into_future() {
        let (mut handler, backend, _keep) = setup();
        let initial_state = handler.state().expect("state works");

        let (justifications, _) = setup_request_tests(&mut handler, &backend, 100, 100);

        let requested_id = justifications.last().unwrap().header().id();
        let request = Request::new(requested_id.clone(), LowestId(requested_id), initial_state);

        match handler.handle_request(request).expect("correct request") {
            Action::Noop => {}
            other_action => panic!("expected a response with justifications, got {other_action:?}"),
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    enum SimplifiedItem {
        J(BlockNumber),
        B(BlockNumber),
        H(BlockNumber),
    }

    impl SimplifiedItem {
        pub fn from_response_items(
            response_items: ResponseItems<MockBlock, MockJustification>,
        ) -> Vec<SimplifiedItem> {
            response_items
                .into_iter()
                .map(|it| match it {
                    ResponseItem::Justification(j) => Self::J(j.id().number()),
                    ResponseItem::Header(h) => Self::H(h.id().number()),
                    ResponseItem::Block(b) => Self::B(b.id().number()),
                })
                .collect()
        }
    }

    #[test]
    fn handles_request_with_lowest_id() {
        use SimplifiedItem::*;
        let (mut handler, backend, _keep) = setup();
        let initial_state = handler.state().expect("state works");

        let (justifications, blocks) = setup_request_tests(&mut handler, &backend, 100, 20);

        let requested_id = blocks[30].clone().id();
        let lowest_id = justifications
            .last()
            .expect("at least 20 finalized blocks")
            .clone()
            .header()
            .id();

        // request block #31, with the last known header equal to last finalized block of the previous session
        // so block #19
        let request = Request::new(requested_id, LowestId(lowest_id), initial_state);

        let expected_response_items = vec![
            J(1),
            B(1),
            J(2),
            B(2),
            J(3),
            B(3),
            J(4),
            B(4),
            J(5),
            B(5),
            J(6),
            B(6),
            J(7),
            B(7),
            J(8),
            B(8),
            J(9),
            B(9),
            J(19),
            H(18),
            H(17),
            H(16),
            H(15),
            H(14),
            H(13),
            H(12),
            H(11),
            H(10),
            B(10),
            B(11),
            B(12),
            B(13),
            B(14),
            B(15),
            B(16),
            B(17),
            B(18),
            B(19),
            B(20),
            B(21),
            B(22),
            B(23),
            B(24),
            B(25),
            B(26),
            B(27),
            B(28),
            B(29),
            B(30),
            B(31),
        ];
        match handler.handle_request(request).expect("correct request") {
            Action::Response(response_items) => {
                assert_eq!(
                    SimplifiedItem::from_response_items(response_items),
                    expected_response_items
                )
            }
            other_action => panic!("expected a response with justifications, got {other_action:?}"),
        }
    }
    #[test]
    fn handles_request_with_unknown_id() {
        let (mut handler, backend, _keep) = setup();
        setup_request_tests(&mut handler, &backend, 100, 20);

        let state = State::new(MockJustification::for_header(
            MockHeader::random_parentless(105),
        ));
        let requested_id = MockIdentifier::new_random(120);
        let lowest_id = MockIdentifier::new_random(110);

        let request = Request::new(requested_id.clone(), LowestId(lowest_id), state);

        match handler.handle_request(request).expect("correct request") {
            Action::RequestBlock(id) => assert_eq!(id, requested_id),
            other_action => panic!("expected a response with justifications, got {other_action:?}"),
        }
    }

    #[test]
    fn handles_request_with_top_imported() {
        use SimplifiedItem::*;
        let (mut handler, backend, _keep) = setup();
        let initial_state = handler.state().expect("state works");

        let (_, blocks) = setup_request_tests(&mut handler, &backend, 100, 20);

        let requested_id = blocks[30].clone().id();
        let top_imported = blocks[25].clone().id();

        // request block #31, with the top imported block equal to block #26
        let request = Request::new(requested_id, TopImported(top_imported), initial_state);

        let expected_response_items = vec![
            J(1),
            J(2),
            J(3),
            J(4),
            J(5),
            J(6),
            J(7),
            J(8),
            J(9),
            J(19),
            B(27),
            B(28),
            B(29),
            B(30),
            B(31),
        ];

        match handler.handle_request(request).expect("correct request") {
            Action::Response(response_items) => {
                assert_eq!(
                    SimplifiedItem::from_response_items(response_items),
                    expected_response_items
                )
            }
            other_action => panic!("expected a response with justifications, got {other_action:?}"),
        }
    }

    #[test]
    fn handles_new_internal_request() {
        let (mut handler, backend, _keep) = setup();
        let _ = handler.state().expect("state works");
        let headers = import_branch(&backend, 2);

        assert!(handler.handle_internal_request(&headers[1].id()).unwrap());
        assert!(!handler.handle_internal_request(&headers[1].id()).unwrap());
    }
}
