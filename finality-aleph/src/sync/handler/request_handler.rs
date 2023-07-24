use core::{default::Default, marker::PhantomData};
use std::fmt::{Display, Formatter};

use primitives::BlockNumber;

use crate::{
    session::{SessionBoundaryInfo, SessionId},
    sync::{
        data::{BranchKnowledge, ResponseItem},
        handler::Request,
        Block, BlockIdFor, BlockStatus, ChainStatus, FinalizationStatus, Header, Justification,
    },
    BlockIdentifier,
};

#[derive(Debug, Clone)]
pub enum RequestHandlerError<J: Justification, T: Display> {
    MissingBlock(BlockIdFor<J>),
    MissingParent(BlockIdFor<J>),
    RootMismatch,
    LastBlockOfSessionNotJustified,
    ChainStatusError(T),
}

impl<J: Justification, T: Display> From<T> for RequestHandlerError<J, T> {
    fn from(value: T) -> Self {
        RequestHandlerError::ChainStatusError(value)
    }
}

impl<J: Justification, T: Display> Display for RequestHandlerError<J, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestHandlerError::RootMismatch => write!(
                f,
                "invalid request, top_justification is not an ancestor of target"
            ),
            RequestHandlerError::MissingParent(id) => write!(f, "missing parent of block {id:?}"),
            RequestHandlerError::MissingBlock(id) => write!(f, "missing block {id:?}"),
            RequestHandlerError::ChainStatusError(e) => write!(f, "{e}"),
            RequestHandlerError::LastBlockOfSessionNotJustified => {
                write!(f, "last block of finalized session not justified")
            }
        }
    }
}

type Chunk<B, J> = Vec<ResponseItem<B, J>>;

pub trait HandlerTypes {
    type Justification: Justification;
    type ChainStatusError: Display;
}

type HandlerResult<T, HT> = Result<
    T,
    RequestHandlerError<
        <HT as HandlerTypes>::Justification,
        <HT as HandlerTypes>::ChainStatusError,
    >,
>;

#[derive(Debug)]
enum HeadOfChunk<J: Justification> {
    Justification(J),
    Header(J::Header),
}

impl<J: Justification> HeadOfChunk<J> {
    pub fn id(&self) -> BlockIdFor<J> {
        match self {
            HeadOfChunk::Justification(j) => j.header().id(),
            HeadOfChunk::Header(h) => h.id(),
        }
    }

    pub fn parent_id(&self) -> Option<BlockIdFor<J>> {
        match self {
            HeadOfChunk::Justification(j) => j.header().parent_id(),
            HeadOfChunk::Header(h) => h.parent_id(),
        }
    }

    pub fn is_justification(&self) -> bool {
        matches!(self, HeadOfChunk::Justification(_))
    }
}

#[derive(PartialEq, Eq, Debug)]
enum State {
    EverythingButHeader,
    Everything,
    OnlyJustification,
}

struct StepResult<B, J>
where
    B: Block,
    J: Justification<Header = B::Header>,
{
    pre_chunk: PreChunk<B, J>,
    state: State,
    head: HeadOfChunk<J>,
}

impl<B, J> StepResult<B, J>
where
    B: Block,
    J: Justification<Header = B::Header>,
{
    fn new(head: HeadOfChunk<J>, state: State) -> Self {
        Self {
            pre_chunk: PreChunk::new(&head),
            state,
            head,
        }
    }

    pub fn current_id(&self) -> BlockIdFor<J> {
        self.head.id()
    }

    pub fn update<CS: ChainStatus<B, J>>(
        &mut self,
        chain_status: &CS,
    ) -> Result<bool, RequestHandlerError<J, CS::Error>> {
        match self.state {
            State::EverythingButHeader => self.add_block(self.head.id(), chain_status)?,
            State::Everything if self.head.is_justification() => {
                self.add_block(self.head.id(), chain_status)?
            }
            State::Everything => self.add_block_and_header(self.head.id(), chain_status)?,
            _ => {}
        }

        self.head = self.next_head(chain_status)?;

        Ok(self.head.is_justification())
    }

    fn add_block<CS: ChainStatus<B, J>>(
        &mut self,
        id: BlockIdFor<J>,
        chain_status: &CS,
    ) -> Result<(), RequestHandlerError<J, CS::Error>> {
        let block = chain_status
            .block(id.clone())?
            .ok_or(RequestHandlerError::MissingBlock(id))?;
        self.pre_chunk.add_block(block);

        Ok(())
    }

    fn add_block_and_header<CS: ChainStatus<B, J>>(
        &mut self,
        id: BlockIdFor<J>,
        chain_status: &CS,
    ) -> Result<(), RequestHandlerError<J, CS::Error>> {
        let block = chain_status
            .block(id.clone())?
            .ok_or(RequestHandlerError::MissingBlock(id))?;
        self.pre_chunk.add_block_and_header(block);
        Ok(())
    }

    fn next_head<CS: ChainStatus<B, J>>(
        &self,
        chain_status: &CS,
    ) -> Result<HeadOfChunk<J>, RequestHandlerError<J, CS::Error>> {
        let parent_id = self
            .head
            .parent_id()
            .ok_or(RequestHandlerError::MissingParent(self.head.id()))?;

        let head = match chain_status.status_of(parent_id.clone())? {
            BlockStatus::Justified(j) => HeadOfChunk::Justification(j),
            BlockStatus::Present(h) => HeadOfChunk::Header(h),
            BlockStatus::Unknown => return Err(RequestHandlerError::MissingBlock(parent_id)),
        };

        Ok(head)
    }

    pub fn start_sending_headers(&mut self) {
        self.state = State::Everything;
    }

    pub fn stop_sending_blocks(&mut self) {
        self.state = State::OnlyJustification;
    }

    pub fn finish(self) -> (Chunk<B, J>, State, HeadOfChunk<J>) {
        let chunk = self.pre_chunk.into_chunk();

        (chunk, self.state, self.head)
    }
}

#[derive(Debug)]
pub enum Action<B, J>
where
    B: Block,
    J: Justification<Header = B::Header>,
{
    RequestBlock(BlockIdFor<J>),
    Response(Vec<ResponseItem<B, J>>),
    Noop,
}

impl<B, J> Action<B, J>
where
    B: Block,
    J: Justification<Header = B::Header>,
{
    fn request_block(id: BlockIdFor<J>) -> Self {
        Action::RequestBlock(id)
    }

    fn new(response_items: Vec<ResponseItem<B, J>>) -> Self {
        match response_items.is_empty() {
            true => Action::Noop,
            false => Action::Response(response_items),
        }
    }
}

#[derive(Default)]
struct PreChunk<B, J>
where
    B: Block,
    J: Justification<Header = B::Header>,
{
    pub just: Option<J>,
    pub blocks: Vec<B>,
    pub headers: Vec<J::Header>,
}

impl<B, J> PreChunk<B, J>
where
    B: Block,
    J: Justification<Header = B::Header>,
{
    fn new(head: &HeadOfChunk<J>) -> Self {
        match head {
            HeadOfChunk::Justification(j) => Self::from_just(j.clone()),
            HeadOfChunk::Header(_) => Self::empty(),
        }
    }

    fn empty() -> Self {
        Self {
            just: None,
            blocks: vec![],
            headers: vec![],
        }
    }

    fn from_just(justification: J) -> Self {
        Self {
            just: Some(justification),
            blocks: vec![],
            headers: vec![],
        }
    }

    fn into_chunk(mut self) -> Chunk<B, J> {
        let mut chunks = vec![];

        if let Some(j) = self.just {
            chunks.push(ResponseItem::Justification(j.into_unverified()));
        }

        chunks.extend(self.headers.into_iter().map(ResponseItem::Header));

        self.blocks.reverse();
        chunks.extend(self.blocks.into_iter().map(ResponseItem::Block));

        chunks
    }

    pub fn add_block(&mut self, b: B) {
        self.blocks.push(b);
    }

    pub fn add_block_and_header(&mut self, b: B) {
        self.headers.push(b.header().clone());
        self.blocks.push(b);
    }
}

pub struct RequestHandler<'a, B, J, CS>
where
    B: Block,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
{
    chain_status: &'a CS,
    session_info: &'a SessionBoundaryInfo,
    _phantom: PhantomData<(B, J)>,
}

impl<'a, B, J, CS> HandlerTypes for RequestHandler<'a, B, J, CS>
where
    B: Block,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
{
    type Justification = J;
    type ChainStatusError = CS::Error;
}

impl<'a, B, J, CS> RequestHandler<'a, B, J, CS>
where
    B: Block,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
{
    pub fn new(chain_status: &'a CS, session_info: &'a SessionBoundaryInfo) -> Self {
        Self {
            chain_status,
            session_info,
            _phantom: PhantomData,
        }
    }

    fn upper_limit(&self, id: BlockIdFor<J>) -> BlockNumber {
        let session = self.session_info.session_id_from_block_num(id.number());
        self.session_info
            .last_block_of_session(SessionId(session.0 + 1))
    }

    fn is_result_complete(
        &self,
        result: &mut StepResult<B, J>,
        branch_knowledge: &BranchKnowledge<J>,
        to: &BlockIdFor<J>,
    ) -> HandlerResult<bool, Self> {
        Ok(match branch_knowledge {
            _ if result.current_id() == *to => true,
            _ if result.current_id().number() <= to.number() => {
                return Err(RequestHandlerError::RootMismatch);
            }
            BranchKnowledge::LowestId(id) if *id == result.current_id() => {
                result.start_sending_headers();
                result.update(self.chain_status)?
            }
            BranchKnowledge::TopImported(id) if *id == result.current_id() => {
                result.stop_sending_blocks();
                result.update(self.chain_status)?
            }
            _ => result.update(self.chain_status)?,
        })
    }

    fn step(
        &self,
        state: State,
        from: HeadOfChunk<J>,
        to: &BlockIdFor<J>,
        branch_knowledge: &BranchKnowledge<J>,
    ) -> HandlerResult<Option<StepResult<B, J>>, Self> {
        if from.id() == *to {
            return Ok(None);
        }
        let mut result = StepResult::new(from, state);

        while !self.is_result_complete(&mut result, branch_knowledge, to)? {}

        Ok(Some(result))
    }

    fn response_items(
        self,
        mut head: HeadOfChunk<J>,
        branch_knowledge: BranchKnowledge<J>,
        to: BlockIdFor<J>,
    ) -> HandlerResult<Vec<ResponseItem<B, J>>, Self> {
        let mut response_items = vec![];
        let mut state = State::EverythingButHeader;

        while let Some(result) = self.step(state, head, &to, &branch_knowledge)? {
            let (chunk, new_state, new_head) = result.finish();

            state = new_state;
            head = new_head;
            response_items.push(chunk);
        }

        response_items.reverse();

        Ok(response_items.into_iter().flatten().collect())
    }

    fn adjust_head(
        &self,
        head: HeadOfChunk<J>,
        our_top_justification: J,
        upper_limit: BlockNumber,
    ) -> HandlerResult<HeadOfChunk<J>, Self> {
        let target = head.id();
        let our_top_justification_number = our_top_justification.header().id().number();

        Ok(if target.number() > our_top_justification_number {
            head
        } else if upper_limit > our_top_justification_number {
            HeadOfChunk::Justification(our_top_justification)
        } else {
            match self.chain_status.finalized_at(upper_limit)? {
                FinalizationStatus::FinalizedWithJustification(j) => HeadOfChunk::Justification(j),
                _ => return Err(RequestHandlerError::LastBlockOfSessionNotJustified),
            }
        })
    }

    pub fn action(self, request: Request<J>) -> HandlerResult<Action<B, J>, Self> {
        let our_top_justification = self.chain_status.top_finalized()?;
        let top_justification = request.state().top_justification();
        let target = request.target_id();

        let upper_limit = self.upper_limit(top_justification.id());

        // request too far into future
        if target.number() > upper_limit {
            return Ok(Action::Noop);
        }

        let head = match self.chain_status.status_of(target.clone())? {
            BlockStatus::Unknown => return Ok(Action::request_block(target.clone())),
            BlockStatus::Justified(justification) => HeadOfChunk::Justification(justification),
            BlockStatus::Present(header) => HeadOfChunk::Header(header),
        };

        let head = self.adjust_head(head, our_top_justification, upper_limit)?;

        let response_items = self.response_items(
            head,
            request.branch_knowledge().clone(),
            top_justification.id(),
        )?;

        Ok(Action::new(response_items))
    }
}
