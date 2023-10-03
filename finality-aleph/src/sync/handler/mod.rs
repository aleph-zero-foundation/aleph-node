use core::marker::PhantomData;
use std::{
    cmp::max,
    collections::VecDeque,
    fmt::{Debug, Display, Error as FmtError, Formatter},
    iter,
};

use crate::{
    session::{SessionBoundaryInfo, SessionId},
    sync::{
        data::{BranchKnowledge, NetworkData, Request, State},
        forest::{
            Error as ForestError, ExtensionRequest, Forest,
            InitializationError as ForestInitializationError, Interest,
        },
        handler::request_handler::RequestHandler,
        Block, BlockImport, BlockStatus, ChainStatus, Finalizer, Header, Justification, PeerId,
        UnverifiedJustification, Verifier,
    },
    BlockId, BlockNumber, SyncOracle,
};

mod request_handler;
pub use request_handler::{Action, RequestHandlerError};

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

/// A handle for requesting Interest.
pub struct InterestProvider<'a, I, J>
where
    I: PeerId,
    J: Justification,
{
    forest: &'a Forest<I, J>,
}

impl<'a, I, J> InterestProvider<'a, I, J>
where
    I: PeerId,
    J: Justification,
{
    pub fn get(&self, id: &BlockId) -> Interest<I> {
        self.forest.request_interest(id)
    }
}

/// Types used by the Handler. For improved readability.
pub trait HandlerTypes {
    /// What can go wrong when handling a piece of data.
    type Error;
}

// This is only required because we don't control block imports
// and thus we can get notifications about blocks being imported that
// don't fit in the forest. This struct lets us work around this by
// manually syncing the forest after such an event.
//TODO(A0-2984): remove this after legacy sync is excised
enum MissedImportData {
    AllGood,
    MissedImports {
        highest_missed: BlockNumber,
        last_sync: BlockNumber,
    },
}

enum TrySyncError<B, J, CS>
where
    B: Block,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
{
    ChainStatus(CS::Error),
    Forest(ForestError),
}

impl MissedImportData {
    pub fn new() -> Self {
        Self::AllGood
    }

    pub fn update<B, J, CS>(
        &mut self,
        missed: BlockNumber,
        chain_status: &CS,
    ) -> Result<(), CS::Error>
    where
        B: Block,
        J: Justification<Header = B::Header>,
        CS: ChainStatus<B, J>,
    {
        use MissedImportData::*;
        match self {
            AllGood => {
                *self = MissedImports {
                    highest_missed: missed,
                    last_sync: chain_status.top_finalized()?.header().id().number(),
                }
            }
            MissedImports { highest_missed, .. } => *highest_missed = max(*highest_missed, missed),
        }
        Ok(())
    }

    pub fn try_sync<B, I, J, CS>(
        &mut self,
        chain_status: &CS,
        forest: &mut Forest<I, J>,
    ) -> Result<(), TrySyncError<B, J, CS>>
    where
        B: Block,
        I: PeerId,
        J: Justification<Header = B::Header>,
        CS: ChainStatus<B, J>,
    {
        use MissedImportData::*;
        if let MissedImports {
            highest_missed,
            last_sync,
        } = self
        {
            let top_finalized = chain_status
                .top_finalized()
                .map_err(TrySyncError::ChainStatus)?
                .header()
                .id();
            // we don't want this to happen too often, but it also cannot be too close to the max forest size, thus semi-random weird looking threshold
            if top_finalized.number() - *last_sync <= 1312 {
                return Ok(());
            }
            let mut to_import = VecDeque::from(
                chain_status
                    .children(top_finalized.clone())
                    .map_err(TrySyncError::ChainStatus)?,
            );
            while let Some(header) = to_import.pop_front() {
                if header.id().number() > *highest_missed {
                    break;
                }
                // we suppress all errors except `TooNew` since we are likely trying to mark things that are already marked and they would be throwing a lot of stuff
                match forest.update_body(&header) {
                    Ok(()) => (),
                    Err(ForestError::TooNew) => {
                        *last_sync = top_finalized.number();
                        return Ok(());
                    }
                    Err(e) => return Err(TrySyncError::Forest(e)),
                }
                to_import.extend(
                    chain_status
                        .children(header.id())
                        .map_err(TrySyncError::ChainStatus)?,
                );
            }
            *self = AllGood;
        }
        Ok(())
    }
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
    missed_import_data: MissedImportData,
    sync_oracle: SyncOracle,
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
    /// The state suggests we should try requesting a chain extension.
    ExtendChain,
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

    fn maybe_extend(new_info: bool) -> Self {
        match new_info {
            true => HandleStateAction::ExtendChain,
            false => HandleStateAction::Noop,
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
    RequestHandlerError(RequestHandlerError<CS::Error>),
    MissingJustification,
    BlockNotImportable,
    HeaderNotRequired,
    MissingFavouriteBlock,
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
            RequestHandlerError(e) => write!(f, "request handler error: {e}"),
            MissingJustification => write!(
                f,
                "justification for the last block of a past session missing"
            ),
            BlockNotImportable => {
                write!(f, "cannot import a block that we do not consider required")
            }
            HeaderNotRequired => write!(f, "header was not required, but it should have been"),
            MissingFavouriteBlock => {
                write!(f, "the favourite block is not present in the database")
            }
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

impl<B, J, CS, V, F> From<TrySyncError<B, J, CS>> for Error<B, J, CS, V, F>
where
    J: Justification,
    B: Block<Header = J::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
{
    fn from(e: TrySyncError<B, J, CS>) -> Self {
        use TrySyncError::*;
        match e {
            ChainStatus(e) => Error::ChainStatus(e),
            Forest(e) => Error::Forest(e),
        }
    }
}

impl<B, J, CS, V, F> From<RequestHandlerError<CS::Error>> for Error<B, J, CS, V, F>
where
    J: Justification,
    B: Block<Header = J::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
{
    fn from(e: RequestHandlerError<CS::Error>) -> Self {
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
        sync_oracle: SyncOracle,
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
            missed_import_data: MissedImportData::new(),
            sync_oracle,
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
                None => {
                    self.missed_import_data
                        .try_sync(&self.chain_status, &mut self.forest)?;
                    return Ok(());
                }
            };
        }
    }

    /// Inform the handler that a block has been imported.
    pub fn block_imported(
        &mut self,
        header: J::Header,
    ) -> Result<(), <Self as HandlerTypes>::Error> {
        if let Err(e) = self.forest.update_body(&header) {
            if matches!(e, ForestError::TooNew) {
                self.missed_import_data
                    .update(header.id().number(), &self.chain_status)
                    .map_err(Error::ChainStatus)?;
            }
            return Err(e.into());
        }
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

    /// Handle a chain extension request.
    ///
    /// First treats it as a request for our favourite block with their favourite block
    /// as the top imported.
    /// If that fails due to our favourite block not being a descendant of theirs,
    /// it falls back to attempting to send any finalized blocks the requester doesn't have.
    pub fn handle_chain_extension_request(
        &mut self,
        state: State<J>,
    ) -> Result<Action<B, J>, <Self as HandlerTypes>::Error> {
        let request = Request::new(
            self.forest.favourite_block(),
            BranchKnowledge::TopImported(state.favourite_block().id()),
            state.clone(),
        );
        match self.handle_request(request) {
            // Either we were trying to send too far in the future
            // or our favourite is not a descendant of theirs.
            // Either way, at least try sending some justifications.
            Ok(Action::Noop)
            | Err(Error::RequestHandlerError(RequestHandlerError::RootMismatch)) => {
                let request = Request::new(
                    state.top_justification().header().id(),
                    BranchKnowledge::TopImported(state.top_justification().header().id()),
                    state,
                );
                self.handle_request(request)
            }
            result => result,
        }
    }

    /// Handle a single unverified justification.
    /// Return whether this justification was higher than the previously known highest justification.
    fn handle_justification(
        &mut self,
        justification: J::Unverified,
        maybe_peer: Option<I>,
    ) -> Result<bool, <Self as HandlerTypes>::Error> {
        let justification = self
            .verifier
            .verify(justification)
            .map_err(Error::Verifier)?;
        let new_highest = self
            .forest
            .update_justification(justification, maybe_peer)?;
        self.try_finalize()?;
        self.sync_oracle
            .update_behind(self.forest.behind_finalization());
        Ok(new_highest)
    }

    /// Handle a justification from the user, returning whether it became the new highest justification.
    pub fn handle_justification_from_user(
        &mut self,
        justification: J::Unverified,
    ) -> Result<bool, <Self as HandlerTypes>::Error> {
        self.handle_justification(justification, None)
    }

    /// Handle a state response returning whether it resulted in a new highest justified block
    /// and possibly an error.
    ///
    /// If no error is returned, it means that the whole state response was processed
    /// correctly. Otherwise, the response might have been processed partially, or
    /// dropped. In any case, the Handler finishes in a sane state.
    pub fn handle_state_response(
        &mut self,
        justification: J::Unverified,
        maybe_justification: Option<J::Unverified>,
        peer: I,
    ) -> (bool, Option<<Self as HandlerTypes>::Error>) {
        let mut new_highest = false;

        for justification in iter::once(justification).chain(maybe_justification) {
            new_highest = match self.handle_justification(justification, Some(peer.clone())) {
                Ok(new_highest) => new_highest,
                Err(e) => return (new_highest, Some(e)),
            } || new_highest;
        }

        (new_highest, None)
    }

    /// Handle a request response returning whether it resulted in a new highest justified block
    /// and possibly an error.
    ///
    /// If no error is returned, it means that the whole request response was processed
    /// correctly. Otherwise, the response might have been processed partially, or
    /// dropped. In any case, the Handler finishes in a sane state.
    ///
    /// Note that this method does not verify nor import blocks. The received blocks
    /// are stored in a buffer, and might be silently discarded in the future
    /// if the import fails.
    pub fn handle_request_response(
        &mut self,
        response_items: ResponseItems<B, J>,
        peer: I,
    ) -> (bool, Option<<Self as HandlerTypes>::Error>) {
        let mut new_highest = false;
        // Lets us import descendands of importable blocks, useful for favourite blocks.
        let mut last_imported_block: Option<BlockId> = None;
        for item in response_items {
            match item {
                ResponseItem::Justification(j) => {
                    match self.handle_justification(j, Some(peer.clone())) {
                        Ok(highest) => new_highest = new_highest || highest,
                        Err(e) => return (new_highest, Some(e)),
                    }
                }
                ResponseItem::Header(h) => {
                    if self.forest.skippable(&h.id()) {
                        continue;
                    }
                    if !self.forest.importable(&h.id()) {
                        return (new_highest, Some(Error::HeaderNotRequired));
                    }
                    if let Err(e) = self.forest.update_header(&h, Some(peer.clone()), true) {
                        return (new_highest, Some(Error::Forest(e)));
                    }
                }
                ResponseItem::Block(b) => {
                    if self.forest.skippable(&b.header().id()) {
                        continue;
                    }
                    match self.forest.importable(&b.header().id())
                        || last_imported_block
                            .map(|id| id == b.header().id())
                            .unwrap_or(false)
                    {
                        true => {
                            last_imported_block = Some(b.header().id());
                            self.block_importer.import_block(b);
                        }
                        false => return (new_highest, Some(Error::BlockNotImportable)),
                    };
                }
            }
        }

        (new_highest, None)
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
        let remote_top_number = state.top_justification().header().id().number();
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
            None => Ok(HandleStateAction::maybe_extend(
                self.handle_justification(state.top_justification(), Some(peer.clone()))?
                    || self
                        .forest
                        .update_header(&state.favourite_block(), Some(peer), false)?,
            )),
            // same session
            Some(0) => match remote_top_number >= local_top_number {
                // remote top justification higher than ours, we can import the justification
                true => Ok(HandleStateAction::maybe_extend(
                    self.handle_justification(state.top_justification(), Some(peer.clone()))?
                        || self.forest.update_header(
                            &state.favourite_block(),
                            Some(peer),
                            false,
                        )?,
                )),
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
        use BlockStatus::*;
        let top_justification = self
            .chain_status
            .top_finalized()
            .map_err(Error::ChainStatus)?
            .into_unverified();
        let favourite_block = match self
            .chain_status
            .status_of(self.forest.favourite_block())
            .map_err(Error::ChainStatus)?
        {
            Justified(justification) => justification.header().clone(),
            Present(header) => header,
            Unknown => return Err(Error::MissingFavouriteBlock),
        };
        Ok(State::new(top_justification, favourite_block))
    }

    /// A handle for requesting Interest.
    pub fn interest_provider(&self) -> InterestProvider<I, J> {
        InterestProvider {
            forest: &self.forest,
        }
    }

    /// Handle an internal block request.
    /// Returns `true` if this was the first time something indicated interest in this block.
    pub fn handle_internal_request(
        &mut self,
        id: &BlockId,
    ) -> Result<bool, <Self as HandlerTypes>::Error> {
        let should_request = self.forest.update_block_identifier(id, None, true)?;

        Ok(should_request)
    }

    /// Returns the extension request we could be making right now.
    pub fn extension_request(&self) -> ExtensionRequest<I> {
        self.forest.extension_request()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{DatabaseIO, HandleStateAction, HandleStateAction::*, Handler};
    use crate::{
        session::{SessionBoundaryInfo, SessionId},
        sync::{
            data::{BranchKnowledge::*, NetworkData, Request, ResponseItem, ResponseItems, State},
            forest::{ExtensionRequest, Interest},
            handler::Action,
            mock::{Backend, MockBlock, MockHeader, MockJustification, MockPeerId},
            Block, BlockImport, ChainStatus,
            ChainStatusNotification::*,
            ChainStatusNotifier, Header, Justification,
        },
        BlockId, BlockNumber, SessionPeriod, SyncOracle,
    };

    type TestHandler =
        Handler<MockBlock, MockPeerId, MockJustification, Backend, Backend, Backend, Backend>;
    type MockResponseItems = ResponseItems<MockBlock, MockJustification>;

    const SESSION_BOUNDARY_INFO: SessionBoundaryInfo = SessionBoundaryInfo::new(SessionPeriod(20));

    fn setup() -> (
        TestHandler,
        Backend,
        impl ChainStatusNotifier<MockHeader>,
        BlockId,
    ) {
        let (backend, notifier) = Backend::setup(SESSION_BOUNDARY_INFO);
        let verifier = backend.clone();
        let database_io = DatabaseIO::new(backend.clone(), backend.clone(), backend.clone());
        let handler = Handler::new(
            database_io,
            verifier,
            SyncOracle::new(),
            SESSION_BOUNDARY_INFO,
        )
        .expect("mock backend works");
        let genesis = backend.top_finalized().expect("genesis").header().id();
        (handler, backend, notifier, genesis)
    }

    fn import_branch(backend: &mut Backend, branch_length: usize) -> Vec<MockHeader> {
        let result: Vec<_> = backend
            .top_finalized()
            .expect("mock backend works")
            .header()
            .random_branch()
            .take(branch_length)
            .collect();
        for header in &result {
            backend.import_block(MockBlock::new(header.clone(), true));
        }
        result
    }

    fn assert_dangling_branch_required(
        handler: &TestHandler,
        top: &BlockId,
        bottom: &BlockId,
        know_most: HashSet<MockPeerId>,
    ) {
        assert_eq!(
            handler.interest_provider().get(bottom),
            Interest::Uninterested,
            "should not be interested in the bottom"
        );
        assert_eq!(
            handler.interest_provider().get(top),
            Interest::Required {
                know_most,
                branch_knowledge: LowestId(bottom.clone()),
            },
            "should require the top"
        );
    }

    fn grow_light_branch_till(
        handler: &mut TestHandler,
        bottom: &BlockId,
        top: &BlockNumber,
        peer_id: MockPeerId,
    ) -> Vec<MockHeader> {
        assert!(top > &bottom.number(), "must not be empty");
        grow_light_branch(handler, bottom, (top - bottom.number()) as usize, peer_id)
    }

    fn grow_light_branch(
        handler: &mut TestHandler,
        bottom: &BlockId,
        length: usize,
        peer_id: MockPeerId,
    ) -> Vec<MockHeader> {
        let branch: Vec<_> = bottom.random_branch().take(length).collect();
        let top = branch.last().expect("branch should not be empty").id();

        assert!(
            handler.handle_internal_request(&top).expect("should work"),
            "should be newly required"
        );
        assert_eq!(
            handler.interest_provider().get(&top),
            Interest::Required {
                know_most: HashSet::new(),
                branch_knowledge: LowestId(top.clone()),
            },
            "should be required"
        );

        let (new_highest_justified, maybe_error) = handler.handle_request_response(
            branch
                .iter()
                .cloned()
                .rev()
                .map(ResponseItem::Header)
                .collect(),
            peer_id,
        );

        assert!(!new_highest_justified);
        assert!(maybe_error.is_none(), "should work");

        branch
    }

    fn create_dangling_branch(
        handler: &mut TestHandler,
        height: BlockNumber,
        length: usize,
        peer_id: MockPeerId,
    ) -> (BlockId, BlockId) {
        let bottom = BlockId::new_random(height);
        let top = grow_light_branch(handler, &bottom, length, peer_id)
            .last()
            .expect("branch should not be empty")
            .id();
        (bottom, top)
    }

    struct BranchResponseContent {
        headers: bool,
        blocks: bool,
        justifications: bool,
    }

    fn branch_response(
        branch: Vec<MockHeader>,
        content: BranchResponseContent,
    ) -> MockResponseItems {
        let mut response = vec![];
        if content.headers {
            response.extend(branch.iter().cloned().rev().map(ResponseItem::Header));
        }
        if content.blocks {
            response.extend(
                branch
                    .iter()
                    .cloned()
                    .map(|header| ResponseItem::Block(MockBlock::new(header, true))),
            );
        }
        if content.justifications {
            response.extend(
                branch
                    .into_iter()
                    .map(MockJustification::for_header)
                    .map(ResponseItem::Justification),
            );
        }
        response
    }

    async fn grow_trunk(
        handler: &mut TestHandler,
        backend: &mut Backend,
        notifier: &mut impl ChainStatusNotifier<MockHeader>,
        bottom: &BlockId,
        length: usize,
    ) -> BlockId {
        let branch: Vec<_> = bottom.random_branch().take(length).collect();
        let top = branch.last().expect("should not be empty").id();
        for header in branch.iter() {
            let block = MockBlock::new(header.clone(), true);
            let justification = MockJustification::for_header(header.clone());
            handler
                .handle_justification_from_user(justification)
                .expect("should work");
            backend.import_block(block);
            match notifier.next().await {
                Ok(BlockImported(header)) => {
                    handler.block_imported(header).expect("should work");
                }
                _ => panic!("should notify about imported block"),
            }
            match notifier.next().await {
                Ok(BlockFinalized(finalized_header)) => assert_eq!(
                    header, &finalized_header,
                    "should finalize the current header"
                ),
                _ => panic!("should notify about finalized block"),
            }
        }
        top
    }

    async fn mark_branch_imported(
        handler: &mut TestHandler,
        notifier: &mut impl ChainStatusNotifier<MockHeader>,
        branch: &Vec<MockHeader>,
    ) {
        for expected_header in branch {
            match notifier.next().await {
                Ok(BlockImported(header)) => {
                    assert_eq!(
                        &header, expected_header,
                        "should import header from the provided branch"
                    );
                    handler.block_imported(header).expect("should work");
                }
                _ => panic!("should import header from the provided branch"),
            }
        }
    }

    async fn consume_branch_finalized_notifications(
        notifier: &mut impl ChainStatusNotifier<MockHeader>,
        branch: &Vec<MockHeader>,
    ) {
        for expected_header in branch {
            match notifier.next().await {
                Ok(BlockFinalized(header)) => {
                    assert_eq!(
                        &header, expected_header,
                        "should finalize header from the provided branch"
                    );
                }
                _ => panic!("should finalize header from the provided branch"),
            }
        }
    }

    #[tokio::test]
    async fn accepts_response_twice() {
        let (mut handler, _backend, mut notifier, genesis) = setup();
        let branch = grow_light_branch(&mut handler, &genesis, 15, 4);
        let response = branch_response(
            branch.clone(),
            BranchResponseContent {
                headers: false,
                blocks: true,
                justifications: true,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(response.clone(), 7);
        assert!(new_info);
        assert!(maybe_error.is_none());
        mark_branch_imported(&mut handler, &mut notifier, &branch).await;
        let (new_info, maybe_error) = handler.handle_request_response(response, 8);
        assert!(!new_info);
        assert!(maybe_error.is_none());
    }

    #[tokio::test]
    async fn accepts_long_response_after_handling_short_one() {
        let (mut handler, _backend, mut notifier, genesis) = setup();
        let branch = grow_light_branch(&mut handler, &genesis, 35, 4);

        let short_response = branch_response(
            branch[..15].to_vec(),
            BranchResponseContent {
                headers: false,
                blocks: true,
                justifications: false,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(short_response, 2);
        assert!(!new_info);
        assert!(maybe_error.is_none());
        mark_branch_imported(&mut handler, &mut notifier, &branch[..15].to_vec()).await;

        let mid_response = branch_response(
            branch.to_vec(),
            BranchResponseContent {
                headers: false,
                blocks: true,
                justifications: false,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(mid_response, 3);
        assert!(!new_info);
        assert!(maybe_error.is_none());
        mark_branch_imported(&mut handler, &mut notifier, &branch[15..].to_vec()).await;
    }

    #[tokio::test]
    async fn handles_multiple_overlapping_responses() {
        let (mut handler, _backend, mut notifier, genesis) = setup();
        let branch = grow_light_branch(&mut handler, &genesis, 35, 4);

        // 15 blocks and justifications -> top is 15, new highest justification
        let short_response = branch_response(
            branch[..15].to_vec(),
            BranchResponseContent {
                headers: false,
                blocks: true,
                justifications: true,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(short_response, 2);
        assert!(new_info);
        assert!(maybe_error.is_none());
        mark_branch_imported(&mut handler, &mut notifier, &branch[..15].to_vec()).await;
        consume_branch_finalized_notifications(&mut notifier, &branch[..15].to_vec()).await;

        // 25 blocks -> top is 15, highest block is 25
        let mid_response = branch_response(
            branch[..25].to_vec(),
            BranchResponseContent {
                headers: false,
                blocks: true,
                justifications: false,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(mid_response, 3);
        assert!(!new_info);
        assert!(maybe_error.is_none());
        mark_branch_imported(&mut handler, &mut notifier, &branch[15..25].to_vec()).await;

        // 35 blocks -> top is 15, highest block is 35
        let long_response_blocks_only = branch_response(
            branch.clone(),
            BranchResponseContent {
                headers: false,
                blocks: true,
                justifications: false,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(long_response_blocks_only, 2);
        assert!(!new_info);
        assert!(maybe_error.is_none());
        mark_branch_imported(&mut handler, &mut notifier, &branch[25..].to_vec()).await;

        // 35 blocks, justifications, and headers (just for fun) ->
        // top is 35, new highest justification
        let full_response = branch_response(
            branch.clone(),
            BranchResponseContent {
                headers: true,
                blocks: true,
                justifications: true,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(full_response.clone(), 2);
        assert!(new_info);
        assert!(maybe_error.is_none());
        consume_branch_finalized_notifications(&mut notifier, &branch[15..].to_vec()).await;
    }

    #[tokio::test]
    async fn finalizes_with_justification_gaps() {
        let (mut handler, _backend, mut notifier, genesis) = setup();
        let mut bottom = genesis;
        let peer_id = 0;
        for session in 0.. {
            let top = SESSION_BOUNDARY_INFO.last_block_of_session(SessionId(session));
            let branch = grow_light_branch_till(&mut handler, &bottom, &top, peer_id);
            bottom = branch.last().expect("should not be empty").id();
            // import blocks
            let response_items = branch_response(
                branch.clone(),
                BranchResponseContent {
                    headers: true,
                    blocks: true,
                    justifications: false,
                },
            );
            let (new_info, maybe_error) = handler.handle_request_response(response_items, peer_id);
            assert!(!new_info, "should not import justification");
            assert!(maybe_error.is_none(), "should work");
            mark_branch_imported(&mut handler, &mut notifier, &branch).await;
            // increasingly larger gaps
            let partial = branch.clone()[session as usize + 1..].to_vec();
            if partial.is_empty() {
                break;
            }
            let response_items = branch_response(
                partial.clone(),
                BranchResponseContent {
                    headers: false,
                    blocks: false,
                    justifications: true,
                },
            );
            let (new_info, maybe_error) = handler.handle_request_response(response_items, peer_id);
            assert!(new_info);
            assert!(maybe_error.is_none(), "should work");
            // get notification about finalized end-of-session block
            match notifier.next().await {
                Ok(BlockFinalized(header)) => {
                    assert_eq!(header.id().number(), top, "should finalize the top block")
                }
                _ => panic!("should notify about finalized block"),
            };
        }
    }

    #[tokio::test]
    async fn skips_justification_gap_with_last_of_current_session_only() {
        let (mut handler, _backend, mut notifier, genesis) = setup();
        let last_block_of_first_session = SESSION_BOUNDARY_INFO.last_block_of_session(SessionId(0));
        let last_block_of_second_session =
            SESSION_BOUNDARY_INFO.last_block_of_session(SessionId(1));
        let peer_id = 0;
        let branch_low = grow_light_branch_till(
            &mut handler,
            &genesis,
            &last_block_of_first_session,
            peer_id,
        );
        let finalizing_justification =
            MockJustification::for_header(branch_low.last().expect("should not be empty").clone());
        let branch_high = grow_light_branch_till(
            &mut handler,
            &finalizing_justification.header().id(),
            &last_block_of_second_session,
            peer_id,
        );
        let response_items = branch_response(
            branch_low
                .iter()
                .cloned()
                .chain(branch_high.iter().cloned())
                .collect(),
            BranchResponseContent {
                headers: true,
                blocks: true,
                justifications: false,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(response_items, peer_id);
        assert!(!new_info, "should not import justification");
        assert!(maybe_error.is_none(), "should work");

        mark_branch_imported(&mut handler, &mut notifier, &branch_low).await;
        mark_branch_imported(&mut handler, &mut notifier, &branch_high).await;

        let all_but_two = branch_response(
            branch_low
                .iter()
                .rev()
                .skip(1)
                .rev()
                .skip(1)
                .cloned()
                .chain(branch_high.iter().cloned())
                .collect(),
            BranchResponseContent {
                headers: false,
                blocks: false,
                justifications: true,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(all_but_two, peer_id);
        let highest = branch_high.last().expect("should not be empty").id();
        assert!(new_info, "should import justifications");
        assert!(maybe_error.is_none(), "should work");

        assert_eq!(
            handler
                .state()
                .expect("should work")
                .top_justification()
                .header()
                .id(),
            genesis,
            "should not finalize anything yet"
        );
        handler
            .handle_justification_from_user(finalizing_justification)
            .expect("should work");
        assert_eq!(
            handler
                .state()
                .expect("should work")
                .top_justification()
                .header()
                .id(),
            highest,
            "should finalize everything"
        );
    }

    #[test]
    fn creates_dangling_branch() {
        let (mut handler, _backend, _notifier, _genesis) = setup();
        let peer_id = 0;
        let (bottom, top) = create_dangling_branch(&mut handler, 25, 10, peer_id);
        assert_dangling_branch_required(&handler, &top, &bottom, HashSet::from_iter(vec![peer_id]));
    }

    #[tokio::test]
    async fn uninterested_in_dangling_branch() {
        let (mut handler, _backend, mut notifier, genesis) = setup();

        // grow the branch that will be finalized
        let branch = grow_light_branch(&mut handler, &genesis, 15, 4);

        // grow the dangling branch that will be pruned
        let peer_id = 3;
        let (bottom, top) = create_dangling_branch(&mut handler, 10, 20, peer_id);
        assert_dangling_branch_required(&handler, &top, &bottom, HashSet::from_iter(vec![peer_id]));

        // begin finalizing the main branch
        let response = branch_response(
            branch,
            BranchResponseContent {
                headers: false,
                blocks: true,
                justifications: true,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(response, 7);
        assert!(new_info, "should create new highest justified");
        assert!(maybe_error.is_none(), "should work");

        // check that still not finalized
        assert!(
            handler.interest_provider().get(&top) != Interest::Uninterested,
            "should still be interested"
        );

        // finalize
        while let Ok(BlockImported(header)) = notifier.next().await {
            handler.block_imported(header).expect("should work");
        }

        // check if dangling branch was pruned
        assert_eq!(
            handler.interest_provider().get(&top),
            Interest::Uninterested,
            "should be uninterested"
        );
    }

    #[tokio::test]
    async fn uninterested_in_dangling_branch_when_connected_below_finalized() {
        let (mut handler, _backend, mut notifier, genesis) = setup();

        // grow the branch that will be finalized
        let branch = grow_light_branch(&mut handler, &genesis, 15, 4);

        // grow the dangling branch that will be pruned
        let fork_peer = 6;
        let fork_bottom = BlockId::new_random(15);
        let fork_child = fork_bottom.random_child();
        let fork = grow_light_branch(&mut handler, &fork_child.id(), 10, fork_peer);
        let fork_top = fork.last().expect("fork not empty").id();

        // finalize the main branch
        let response = branch_response(
            branch.clone(),
            BranchResponseContent {
                headers: false,
                blocks: true,
                justifications: true,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(response, 7);
        assert!(new_info, "should create new highest justified");
        assert!(maybe_error.is_none(), "should work");
        let mut idx = 0;
        while let Ok(BlockImported(header)) = notifier.next().await {
            assert_eq!(
                header, branch[idx],
                "should be importing the main branch in order"
            );
            handler.block_imported(header).expect("should work");
            idx += 1;
        }

        // check that the fork is still interesting
        assert_eq!(
            handler.interest_provider().get(&fork_top),
            Interest::Required {
                know_most: HashSet::from_iter(vec![fork_peer]),
                branch_knowledge: LowestId(fork_child.id()),
            },
            "should be required"
        );

        // import fork_child that connects the fork to fork_bottom,
        // which is at the same height as an already finalized block
        let response = branch_response(
            vec![fork_child],
            BranchResponseContent {
                headers: true,
                blocks: false,
                justifications: false,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(response, 12);
        assert!(!new_info, "should not create new highest justified");
        assert!(maybe_error.is_none(), "should work");

        // check that the fork is pruned
        assert_eq!(
            handler.interest_provider().get(&fork_top),
            Interest::Uninterested,
            "should be uninterested"
        );
    }

    #[tokio::test]
    async fn uninterested_in_dangling_branch_when_connected_to_composted_header() {
        let (mut handler, _backend, mut notifier, genesis) = setup();

        // grow the branch that will be finalized
        let branch = grow_light_branch(&mut handler, &genesis, 15, 4);

        // grow the branch that will be pruned in the first place
        let fork_bottom = branch[10].id();
        let fork = grow_light_branch(&mut handler, &fork_bottom, 15, 5);

        // grow the dangling branch that will be pruned
        let fork_peer = 6;
        let further_fork_bottom = fork.last().expect("branch not empty").id();
        let further_fork_child = further_fork_bottom.random_child();
        let further_fork = grow_light_branch(&mut handler, &further_fork_child.id(), 10, fork_peer);
        let fork_top = further_fork.last().expect("fork not empty").id();

        // finalize the main branch
        let response = branch_response(
            branch.clone(),
            BranchResponseContent {
                headers: false,
                blocks: true,
                justifications: true,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(response, 7);
        assert!(new_info, "should create new highest justified");
        assert!(maybe_error.is_none(), "should work");
        let mut idx = 0;
        while let Ok(BlockImported(header)) = notifier.next().await {
            assert_eq!(
                header, branch[idx],
                "should be importing the main branch in order"
            );
            handler.block_imported(header).expect("should work");
            idx += 1;
        }

        // check if the bottom part of the fork was pruned
        assert_eq!(
            handler.interest_provider().get(&further_fork_bottom),
            Interest::Uninterested,
            "should be uninterested"
        );

        // check that the fork is still interesting
        assert_eq!(
            handler.interest_provider().get(&fork_top),
            Interest::Required {
                know_most: HashSet::from_iter(vec![fork_peer]),
                branch_knowledge: LowestId(further_fork_child.id()),
            },
            "should be required"
        );

        // check that further_fork_child is higher than top finalized
        assert!(
            further_fork_child.id().number()
                > handler
                    .state()
                    .expect("should work")
                    .top_justification()
                    .header()
                    .id()
                    .number()
        );

        // import further_fork_child that connects the fork to further_fork_bottom,
        // which is composted
        let response = branch_response(
            vec![further_fork_child],
            BranchResponseContent {
                headers: true,
                blocks: false,
                justifications: false,
            },
        );
        let (new_info, maybe_error) = handler.handle_request_response(response, 12);
        assert!(!new_info, "should not create new highest justified");
        assert!(maybe_error.is_none(), "should work");

        // check that the fork is pruned
        assert_eq!(
            handler.interest_provider().get(&fork_top),
            Interest::Uninterested,
            "should be uninterested"
        );
    }

    #[tokio::test]
    async fn syncs_to_a_long_trunk() {
        let (mut handler, mut backend, mut notifier, _genesis) = setup();
        let (mut syncing_handler, _syncing_backend, mut syncing_notifier, genesis) = setup();
        let _top_main = grow_trunk(&mut handler, &mut backend, &mut notifier, &genesis, 2345).await;
        let peer_id = 0;
        let syncing_peer_id = 1;
        loop {
            // syncing peer broadcasts the state
            let state = syncing_handler.state().expect("should work");

            // peer responds
            let response = match handler
                .handle_state(state, syncing_peer_id)
                .expect("should create response")
            {
                Response(data) => data,
                ExtendChain => panic!("should not request anything from the syncing peer"),
                Noop => break,
            };
            let (justification, maybe_justification) = match response {
                NetworkData::StateBroadcastResponse(justification, maybe_justification) => {
                    (justification, maybe_justification)
                }
                _ => panic!("should create state broadcast response"),
            };

            // syncing peer processes the response and sends a request
            let mut target_id = justification.header().id();
            if let Some(justification) = &maybe_justification {
                target_id = justification.header().id();
            }
            let (new_info, maybe_error) =
                syncing_handler.handle_state_response(justification, maybe_justification, peer_id);
            assert!(maybe_error.is_none(), "should work");
            assert!(new_info, "should want to request");
            let branch_knowledge = match syncing_handler.extension_request() {
                ExtensionRequest::HighestJustified {
                    id,
                    branch_knowledge,
                    ..
                } => {
                    assert_eq!(id, target_id, "should want to request target_id");
                    branch_knowledge
                }
                _ => panic!("should want to extend"),
            };
            let state = syncing_handler.state().expect("should work");
            let request = Request::new(target_id.clone(), branch_knowledge, state);

            // peer responds
            let response_items = match handler.handle_request(request).expect("should work") {
                Action::Response(items) => items,
                _ => panic!("should prepare response"),
            };

            // syncing peer processes the response
            let (new_info, maybe_error) =
                syncing_handler.handle_request_response(response_items.clone(), peer_id);
            assert!(maybe_error.is_none(), "should work");
            assert!(!new_info, "should already know about target_id");

            // syncing peer finalizes received blocks
            let response_headers: Vec<_> = response_items
                .into_iter()
                .filter_map(|item| {
                    if let ResponseItem::Block(block) = item {
                        Some(block.header().clone())
                    } else {
                        None
                    }
                })
                .collect();
            let mut idx = 0;
            while let Ok(notification) = syncing_notifier.next().await {
                match notification {
                    BlockImported(header) => {
                        assert_eq!(header, response_headers[idx], "should import in order");
                        syncing_handler.block_imported(header).expect("should work");
                        idx += 1;
                    }
                    BlockFinalized(header) if header.id() == target_id => break,
                    _ => (),
                }
            }
        }
    }

    #[test]
    fn finalizes_imported_and_justified() {
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let header = import_branch(&mut backend, 1)[0].clone();
        handler
            .block_imported(header.clone())
            .expect("importing in order");
        let justification = MockJustification::for_header(header);
        let peer = rand::random();
        assert!(handler
            .handle_justification(justification.clone().into_unverified(), Some(peer))
            .expect("correct justification"));
        assert_eq!(
            backend.top_finalized().expect("mock backend works"),
            justification
        );
    }

    #[test]
    fn requests_missing_justifications_without_blocks() {
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let peer = rand::random();
        // skip the first justification, now every next added justification
        // should make us want to request a chain extension
        for justification in import_branch(&mut backend, 5)
            .into_iter()
            .map(MockJustification::for_header)
            .skip(1)
        {
            assert!(handler
                .handle_justification(justification.clone().into_unverified(), Some(peer))
                .expect("correct justification"));
        }
    }

    #[test]
    fn requests_missing_justifications_with_blocks() {
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let peer = rand::random();
        let justifications: Vec<MockJustification> = import_branch(&mut backend, 5)
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
            assert!(handler
                .handle_justification(justification.clone().into_unverified(), Some(peer))
                .expect("correct justification"));
        }
    }

    #[test]
    fn initializes_forest_properly() {
        let (mut backend, _keep) = Backend::setup(SESSION_BOUNDARY_INFO);
        let header = import_branch(&mut backend, 1)[0].clone();
        // header already imported, Handler should initialize Forest properly
        let verifier = backend.clone();
        let database_io = DatabaseIO::new(backend.clone(), backend.clone(), backend.clone());
        let mut handler = Handler::new(
            database_io,
            verifier,
            SyncOracle::new(),
            SessionBoundaryInfo::new(SessionPeriod(20)),
        )
        .expect("mock backend works");
        let justification = MockJustification::for_header(header);
        let peer: MockPeerId = rand::random();
        assert!(handler
            .handle_justification(justification.clone().into_unverified(), Some(peer))
            .expect("correct justification"));
        // should be auto-finalized, if Forest knows about imported body
        assert_eq!(
            backend.top_finalized().expect("mock backend works"),
            justification
        );
    }

    #[test]
    fn finalizes_justified_and_imported() {
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let header = import_branch(&mut backend, 1)[0].clone();
        let justification = MockJustification::for_header(header.clone());
        let peer = rand::random();
        assert!(handler
            .handle_justification(justification.clone().into_unverified(), Some(peer))
            .expect("correct justification"));
        handler.block_imported(header).expect("importing in order");
        assert_eq!(
            backend.top_finalized().expect("mock backend works"),
            justification
        );
    }

    #[test]
    fn handles_state_with_large_difference() {
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let initial_state = handler.state().expect("state works");
        let peer = rand::random();
        let justifications: Vec<MockJustification> = import_branch(&mut backend, 43)
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
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let initial_state = handler.state().expect("state works");
        let peer = rand::random();
        let justifications: Vec<MockJustification> = import_branch(&mut backend, 23)
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
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let initial_state = handler.state().expect("state works");
        let peer = rand::random();
        let justifications: Vec<MockJustification> = import_branch(&mut backend, 13)
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
        handler: &mut TestHandler,
        backend: &mut Backend,
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

        for header in headers.clone().into_iter().skip(finalize_up_to - 1) {
            handler.block_imported(header).expect("importing in order");
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
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let initial_state = handler.state().expect("state works");

        let (justifications, _) = setup_request_tests(&mut handler, &mut backend, 100, 100);

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
        pub fn from_response_items(response_items: MockResponseItems) -> Vec<SimplifiedItem> {
            response_items
                .into_iter()
                .map(|it| match it {
                    ResponseItem::Justification(j) => Self::J(j.header().id().number()),
                    ResponseItem::Header(h) => Self::H(h.id().number()),
                    ResponseItem::Block(b) => Self::B(b.id().number()),
                })
                .collect()
        }
    }

    #[test]
    fn handles_request_with_lowest_id() {
        use SimplifiedItem::*;
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let initial_state = handler.state().expect("state works");

        let (_, blocks) = setup_request_tests(&mut handler, &mut backend, 100, 20);

        let requested_id = blocks[30].clone().id();
        let lowest_id = blocks[25].clone().id();

        // request block #31, with the last known header equal to block #26
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
            H(26),
            H(25),
            H(24),
            H(23),
            H(22),
            H(21),
            H(20),
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
        let (mut handler, mut backend, _keep, _genesis) = setup();
        setup_request_tests(&mut handler, &mut backend, 100, 20);

        let header = MockHeader::random_parentless(105);
        let state = State::new(MockJustification::for_header(header.clone()), header);
        let requested_id = BlockId::new_random(120);
        let lowest_id = BlockId::new_random(110);

        let request = Request::new(requested_id.clone(), LowestId(lowest_id), state);

        match handler.handle_request(request).expect("correct request") {
            Action::RequestBlock(id) => assert_eq!(id, requested_id),
            other_action => panic!("expected a response with justifications, got {other_action:?}"),
        }
    }

    #[test]
    fn handles_request_with_top_imported() {
        use SimplifiedItem::*;
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let initial_state = handler.state().expect("state works");

        let (_, blocks) = setup_request_tests(&mut handler, &mut backend, 100, 20);

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
    fn handles_chain_extension_request_for_just_blocks() {
        use SimplifiedItem::*;
        let (mut handler, mut backend, _keep, _genesis) = setup();

        let (justifications, blocks) = setup_request_tests(&mut handler, &mut backend, 30, 20);

        let remote_favourite_block = blocks[24].header().clone();
        // The justification hole means there is only 10 of 'em.
        let remote_top_justification = justifications[9].clone().into_unverified();
        let remote_state = State::new(remote_top_justification, remote_favourite_block);

        let expected_response_items = vec![B(26), B(27), B(28), B(29), B(30)];

        match handler
            .handle_chain_extension_request(remote_state)
            .expect("correct request")
        {
            Action::Response(response_items) => {
                assert_eq!(
                    SimplifiedItem::from_response_items(response_items),
                    expected_response_items
                )
            }
            other_action => panic!("expected a response, got {other_action:?}"),
        }
    }

    #[test]
    fn handles_chain_extension_request_with_justifications() {
        use SimplifiedItem::*;
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let remote_state = handler.state().expect("state should work");

        setup_request_tests(&mut handler, &mut backend, 30, 20);

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
        ];

        match handler
            .handle_chain_extension_request(remote_state)
            .expect("correct request")
        {
            Action::Response(response_items) => {
                assert_eq!(
                    SimplifiedItem::from_response_items(response_items),
                    expected_response_items
                )
            }
            other_action => panic!("expected a response, got {other_action:?}"),
        }
    }

    #[test]
    fn handles_forked_chain_extension_request() {
        use SimplifiedItem::*;
        let (mut handler, mut backend, _keep, _genesis) = setup();

        let (justifications, _) = setup_request_tests(&mut handler, &mut backend, 30, 20);

        let remote_favourite_block = backend
            .top_finalized()
            .expect("backend works")
            .header()
            .random_branch()
            .nth(7)
            .expect("it's infinite");
        let remote_top_justification = justifications[0].clone().into_unverified();
        let remote_state = State::new(remote_top_justification, remote_favourite_block);

        let expected_response_items = vec![
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
        ];

        match handler
            .handle_chain_extension_request(remote_state)
            .expect("correct request")
        {
            Action::Response(response_items) => {
                assert_eq!(
                    SimplifiedItem::from_response_items(response_items),
                    expected_response_items
                )
            }
            other_action => panic!("expected a response, got {other_action:?}"),
        }
    }

    #[test]
    fn handles_ancient_chain_extension_request() {
        use SimplifiedItem::*;
        let (mut handler, mut backend, _keep, _genesis) = setup();

        let remote_state = handler.state().expect("state should work");

        setup_request_tests(&mut handler, &mut backend, 60, 40);

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
            J(20),
            B(20),
            J(21),
            B(21),
            J(22),
            B(22),
            J(23),
            B(23),
            J(24),
            B(24),
            J(25),
            B(25),
            J(26),
            B(26),
            J(27),
            B(27),
            J(28),
            B(28),
            J(29),
            B(29),
            J(39),
            B(30),
            B(31),
            B(32),
            B(33),
            B(34),
            B(35),
            B(36),
            B(37),
            B(38),
            B(39),
        ];

        match handler
            .handle_chain_extension_request(remote_state)
            .expect("correct request")
        {
            Action::Response(response_items) => {
                assert_eq!(
                    SimplifiedItem::from_response_items(response_items),
                    expected_response_items
                )
            }
            other_action => panic!("expected a response, got {other_action:?}"),
        }
    }

    #[test]
    fn handles_new_internal_request() {
        let (mut handler, mut backend, _keep, _genesis) = setup();
        let _ = handler.state().expect("state works");
        let headers = import_branch(&mut backend, 2);

        assert!(handler.handle_internal_request(&headers[1].id()).unwrap());
        assert!(!handler.handle_internal_request(&headers[1].id()).unwrap());
    }

    //TODO(A0-2984): remove this after legacy sync is excised
    #[tokio::test]
    async fn works_with_overzealous_imports() {
        let (mut handler, mut backend, mut notifier, genesis) = setup();
        let branch: Vec<_> = genesis.random_branch().take(2137).collect();
        for header in branch.iter() {
            let block = MockBlock::new(header.clone(), true);
            backend.import_block(block);
            match notifier.next().await {
                Ok(BlockImported(header)) => {
                    // we ignore failures, as we expect some
                    let _ = handler.block_imported(header);
                }
                _ => panic!("should notify about imported block"),
            }
        }
        for header in branch.iter() {
            let justification = MockJustification::for_header(header.clone());
            handler
                .handle_justification_from_user(justification)
                .expect("should work");
            match notifier.next().await {
                Ok(BlockFinalized(finalized_header)) => assert_eq!(
                    header, &finalized_header,
                    "should finalize the current header"
                ),
                _ => panic!("should notify about finalized block"),
            }
        }
    }
}
