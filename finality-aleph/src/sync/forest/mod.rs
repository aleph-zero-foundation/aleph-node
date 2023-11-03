use std::{
    collections::{
        hash_map::{Entry, OccupiedEntry, VacantEntry},
        HashMap, HashSet, VecDeque,
    },
    fmt::{Display, Error as FmtError, Formatter},
};

use static_assertions::const_assert;

use crate::{
    aleph_primitives::DEFAULT_SESSION_PERIOD,
    sync::{data::BranchKnowledge, Block, BlockIdFor, ChainStatus, Header, Justification, PeerId},
    BlockIdentifier,
};

mod vertex;

use vertex::Vertex;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SpecialState {
    HopelessFork,
    BelowMinimal,
    HighestFinalized,
    TooNew,
}

enum VertexHandleMut<'a, I: PeerId, J: Justification> {
    Special(SpecialState),
    Unknown(VacantEntry<'a, BlockIdFor<J>, VertexWithChildren<I, J>>),
    Candidate(OccupiedEntry<'a, BlockIdFor<J>, VertexWithChildren<I, J>>),
}

enum VertexHandle<'a, I: PeerId, J: Justification> {
    Special(SpecialState),
    Unknown,
    Candidate(&'a VertexWithChildren<I, J>),
}

/// Our interest in a branch referred to by a vertex,
/// including all the information required to prepare a request.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Interest<I: PeerId, J: Justification> {
    /// We are not interested in requesting this branch.
    Uninterested,
    /// We would like to have this branch.
    Required {
        know_most: HashSet<I>,
        branch_knowledge: BranchKnowledge<J>,
    },
    /// We would like to have this branch ASAP.
    HighestJustified {
        know_most: HashSet<I>,
        branch_knowledge: BranchKnowledge<J>,
    },
}

/// What can go wrong when inserting data into the forest.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Error {
    HeaderMissingParentId,
    IncorrectParentState,
    IncorrectVertexState,
    ParentNotImported,
    TooNew,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            HeaderMissingParentId => write!(f, "header did not contain a parent ID"),
            IncorrectParentState => write!(
                f,
                "parent was in a state incompatible with importing this block"
            ),
            IncorrectVertexState => write!(f, "block in a state incompatible with importing"),
            ParentNotImported => {
                write!(f, "parent was not imported when attempting to import block")
            }
            TooNew => write!(f, "block is too new"),
        }
    }
}

/// What can go wrong when creating the forest.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum InitializationError<B, J, CS>
where
    B: Block,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
{
    Error(Error),
    ChainStatus(CS::Error),
}

impl<B, J, CS> Display for InitializationError<B, J, CS>
where
    B: Block,
    J: Justification<Header = B::Header>,
    CS: ChainStatus<B, J>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        match self {
            InitializationError::Error(e) => match e {
                Error::TooNew => write!(f, "there are more imported non-finalized blocks in the database that can fit into the forest – purge the block database and restart the node"),
                e => write!(f, "{e}"),
            },
            InitializationError::ChainStatus(e) => write!(f, "chain status error: {e}"),
        }
    }
}

pub struct VertexWithChildren<I: PeerId, J: Justification> {
    vertex: Vertex<I, J>,
    children: HashSet<BlockIdFor<J>>,
}

impl<I: PeerId, J: Justification> VertexWithChildren<I, J> {
    fn new() -> Self {
        Self {
            vertex: Vertex::new(),
            children: HashSet::new(),
        }
    }

    fn add_child(&mut self, child: BlockIdFor<J>) {
        self.children.insert(child);
    }
}

// How deep can the forest be, vaguely based on two sessions ahead, which is the most we expect to
// ever need worst case scenario.
//
// At least one session must fit into the Forest.
const MAX_DEPTH: u32 = 1800;
const_assert!(DEFAULT_SESSION_PERIOD <= MAX_DEPTH);

pub struct Forest<I, J>
where
    I: PeerId,
    J: Justification,
{
    vertices: HashMap<BlockIdFor<J>, VertexWithChildren<I, J>>,
    highest_justified: BlockIdFor<J>,
    justified_blocks: HashMap<u32, BlockIdFor<J>>,
    root_id: BlockIdFor<J>,
    root_children: HashSet<BlockIdFor<J>>,
    compost_bin: HashSet<BlockIdFor<J>>,
}

type Edge<J> = (BlockIdFor<J>, BlockIdFor<J>);

impl<I, J> Forest<I, J>
where
    I: PeerId,
    J: Justification,
{
    /// Creates a new forest and returns whether we have too many nonfinalized blocks in the DB.
    //TODO(A0-2984): the latter part of the result should be removed after legacy sync is excised
    pub fn new<B, CS>(chain_status: &CS) -> Result<(Self, bool), InitializationError<B, J, CS>>
    where
        B: Block<Header = J::Header>,
        CS: ChainStatus<B, J>,
    {
        let top_finalized = chain_status
            .top_finalized()
            .map_err(InitializationError::ChainStatus)?
            .header()
            .id();
        let mut forest = Self {
            vertices: HashMap::new(),
            highest_justified: top_finalized.clone(),
            justified_blocks: HashMap::new(),
            root_id: top_finalized.clone(),
            root_children: HashSet::new(),
            compost_bin: HashSet::new(),
        };

        // Populate the forest
        let mut deque = VecDeque::from([top_finalized]);
        while let Some(hash) = deque.pop_front() {
            let children = chain_status
                .children(hash)
                .map_err(InitializationError::ChainStatus)?;
            for header in children.iter() {
                if let Err(e) = forest.update_body(header) {
                    match e {
                        Error::TooNew => return Ok((forest, true)),
                        e => return Err(InitializationError::Error(e)),
                    }
                }
            }
            deque.extend(children.into_iter().map(|header| header.id()));
        }

        Ok((forest, false))
    }

    fn special_state(&self, id: &BlockIdFor<J>) -> Option<SpecialState> {
        use SpecialState::*;
        if id == &self.root_id {
            Some(HighestFinalized)
        } else if id.number() <= self.root_id.number() {
            Some(BelowMinimal)
        } else if id.number() > self.root_id.number() + MAX_DEPTH {
            Some(TooNew)
        } else if self.compost_bin.contains(id) {
            Some(HopelessFork)
        } else {
            None
        }
    }

    fn get_mut(&mut self, id: &BlockIdFor<J>) -> VertexHandleMut<I, J> {
        use VertexHandleMut::*;
        if let Some(state) = self.special_state(id) {
            Special(state)
        } else {
            match self.vertices.entry(id.clone()) {
                Entry::Occupied(entry) => Candidate(entry),
                Entry::Vacant(entry) => Unknown(entry),
            }
        }
    }

    fn get(&self, id: &BlockIdFor<J>) -> VertexHandle<I, J> {
        use VertexHandle::*;
        if let Some(state) = self.special_state(id) {
            Special(state)
        } else {
            match self.vertices.get(id) {
                Some(vertex) => Candidate(vertex),
                None => Unknown,
            }
        }
    }

    fn connect_parent(&mut self, id: &BlockIdFor<J>) {
        use SpecialState::*;
        use VertexHandleMut::*;
        if let Candidate(mut entry) = self.get_mut(id) {
            let vertex = entry.get_mut();
            let required = vertex.vertex.importable();
            if let Some(parent_id) = vertex.vertex.parent().cloned() {
                match self.get_mut(&parent_id) {
                    Unknown(entry) => {
                        entry
                            .insert(VertexWithChildren::new())
                            .add_child(id.clone());
                        if required {
                            self.set_required(&parent_id);
                        }
                    }
                    Special(HighestFinalized) => {
                        self.root_children.insert(id.clone());
                    }
                    Candidate(mut entry) => {
                        entry.get_mut().add_child(id.clone());
                        if required {
                            self.set_required(&parent_id);
                        }
                    }
                    Special(HopelessFork) | Special(BelowMinimal) => self.prune(id),
                    // should not happen
                    Special(TooNew) => (),
                };
            };
        };
    }

    fn set_required(&mut self, id: &BlockIdFor<J>) {
        if let VertexHandleMut::Candidate(mut entry) = self.get_mut(id) {
            let vertex = entry.get_mut();
            if vertex.vertex.set_required() {
                if let Some(id) = vertex.vertex.parent().cloned() {
                    self.set_required(&id);
                }
            }
        }
    }

    fn set_explicitly_required(&mut self, id: &BlockIdFor<J>) -> bool {
        match self.get_mut(id) {
            VertexHandleMut::Candidate(mut entry) => {
                match entry.get_mut().vertex.set_explicitly_required() {
                    true => {
                        if let Some(parent_id) = entry.get_mut().vertex.parent().cloned() {
                            self.set_required(&parent_id);
                        }
                        true
                    }
                    false => false,
                }
            }
            _ => false,
        }
    }

    fn insert_id(&mut self, id: BlockIdFor<J>, holder: Option<I>) -> Result<(), Error> {
        match self.special_state(&id) {
            Some(SpecialState::TooNew) => Err(Error::TooNew),
            Some(_) => Ok(()),
            _ => {
                self.vertices
                    .entry(id)
                    .or_insert_with(VertexWithChildren::new)
                    .vertex
                    .add_block_holder(holder);
                Ok(())
            }
        }
    }

    fn process_header(&mut self, header: &J::Header) -> Result<Edge<J>, Error> {
        Ok((
            header.id(),
            header.parent_id().ok_or(Error::HeaderMissingParentId)?,
        ))
    }

    /// Updates the provider block identifier, returns whether it became a new explicitly required.
    pub fn update_block_identifier(
        &mut self,
        id: &BlockIdFor<J>,
        holder: Option<I>,
        required: bool,
    ) -> Result<bool, Error> {
        self.insert_id(id.clone(), holder)?;
        match required {
            true => Ok(self.set_explicitly_required(id)),
            false => Ok(false),
        }
    }

    /// Updates the provided header, returns whether it became a new explicitly required.
    pub fn update_header(
        &mut self,
        header: &J::Header,
        holder: Option<I>,
        required: bool,
    ) -> Result<bool, Error> {
        let (id, parent_id) = self.process_header(header)?;
        self.insert_id(id.clone(), holder.clone())?;
        if let VertexHandleMut::Candidate(mut entry) = self.get_mut(&id) {
            entry.get_mut().vertex.insert_header(parent_id, holder);
            self.connect_parent(&id);
        }
        match required {
            true => Ok(self.set_explicitly_required(&id)),
            false => Ok(false),
        }
    }

    /// Updates the vertex related to the provided header marking it as imported.
    /// Returns errors when it's impossible to do consistently.
    pub fn update_body(&mut self, header: &J::Header) -> Result<(), Error> {
        use SpecialState::*;
        use VertexHandleMut::*;
        let (id, parent_id) = self.process_header(header)?;
        self.update_header(header, None, false)?;
        match self.get_mut(&parent_id) {
            Candidate(entry) => {
                if !entry.get().vertex.imported() {
                    return Err(Error::ParentNotImported);
                }
            }
            Special(HighestFinalized) => (),
            Unknown(_) | Special(HopelessFork) | Special(BelowMinimal) | Special(TooNew) => {
                return Err(Error::IncorrectParentState)
            }
        }
        match self.get_mut(&id) {
            Candidate(mut entry) => {
                let vertex = &mut entry.get_mut().vertex;
                vertex.insert_body(parent_id.clone());
                if vertex.justified_block() {
                    self.justified_blocks.insert(id.number(), id.clone());
                }
                Ok(())
            }
            _ => Err(Error::IncorrectVertexState),
        }
    }

    /// Updates the `highest_justified` if the given id is higher.
    fn try_update_highest_justified(&mut self, id: BlockIdFor<J>) -> bool {
        match id.number() > self.highest_justified.number() {
            true => {
                self.highest_justified = id;
                true
            }
            false => false,
        }
    }

    /// Updates the provided justification.
    /// Returns whether the vertex became the new highest justified header/block.
    pub fn update_justification(
        &mut self,
        justification: J,
        holder: Option<I>,
    ) -> Result<bool, Error> {
        let header = justification.header();
        if header.id().number() == 0 {
            // this is the genesis block
            return Ok(false);
        }
        let (id, parent_id) = self.process_header(header)?;
        self.update_header(header, None, false)?;
        Ok(match self.get_mut(&id) {
            VertexHandleMut::Candidate(mut entry) => {
                let vertex = &mut entry.get_mut().vertex;
                vertex.insert_justification(parent_id, justification, holder);
                if vertex.justified_block() {
                    self.justified_blocks.insert(id.number(), id.clone());
                }
                self.try_update_highest_justified(id.clone())
            }
            _ => false,
        })
    }

    fn prune(&mut self, id: &BlockIdFor<J>) {
        if let Some(VertexWithChildren { children, .. }) = self.vertices.remove(id) {
            self.compost_bin.insert(id.clone());
            for child in children {
                self.prune(&child);
            }
        }
    }

    fn prune_level(&mut self, level: u32) {
        let to_prune: Vec<_> = self
            .vertices
            .keys()
            .filter(|k| k.number() <= level)
            .cloned()
            .collect();
        for id in to_prune.into_iter() {
            self.prune(&id);
        }
        self.compost_bin.retain(|k| k.number() > level);
        self.justified_blocks.retain(|k, _| k > &level);
    }

    /// Attempt to finalize one block, returns the correct justification if successful.
    pub fn try_finalize(&mut self, number: &u32) -> Option<J> {
        if let Some(id) = self.justified_blocks.get(number) {
            if let Some(VertexWithChildren { vertex, children }) = self.vertices.remove(id) {
                match vertex.ready() {
                    // should always match, as the id is taken from self.justified_blocks
                    Ok(justification) => {
                        self.root_id = id.clone();
                        self.root_children = children;
                        self.prune_level(self.root_id.number());
                        return Some(justification);
                    }
                    Err(_vertex) => panic!("Block sync justified_blocks cache corrupted, please restart the Node and contact the developers"),
                }
            }
        }
        None
    }

    /// Returns the BranchKnowledge regarding the given block id,
    /// or None if there is no branch at all.
    fn branch_knowledge(&self, mut id: BlockIdFor<J>) -> Option<BranchKnowledge<J>> {
        use SpecialState::*;
        use VertexHandle::*;
        // traverse ancestors till we reach something imported or a parentless vertex
        loop {
            match self.get(&id) {
                Candidate(vertex) => {
                    // first encounter of an imported ancestor, return it
                    if vertex.vertex.imported() {
                        return Some(BranchKnowledge::TopImported(id));
                    }
                    // try update current id to parent_id
                    match vertex.vertex.parent().cloned() {
                        // it has a parent, continue
                        Some(parent_id) => id = parent_id,
                        // does not have parent, thus is the lowest known,
                        // and is not imported (a Candidate is not the HighestFinalized),
                        // return it
                        None => return Some(BranchKnowledge::LowestId(id)),
                    };
                }
                // we've reached the root, hence this is the top imported ancestor, return it
                Special(HighestFinalized) => {
                    return Some(BranchKnowledge::TopImported(id));
                }
                // either we don't know the requested id, or it will never connect to the root,
                // return None
                Special(HopelessFork) | Special(BelowMinimal) | Special(TooNew) | Unknown => {
                    return None
                }
            };
        }
    }

    /// Prepare additional info required to create a request for the branch.
    /// Returns `None` if we're not interested in the branch.
    fn prepare_request_info(&self, id: &BlockIdFor<J>) -> Option<(HashSet<I>, BranchKnowledge<J>)> {
        use VertexHandle::Candidate;
        match self.get(id) {
            Candidate(vertex) => {
                let know_most = vertex.vertex.know_most().clone();
                // request only requestable blocks, or the highest_justified block/header
                if !(vertex.vertex.requestable() || id == &self.highest_justified) {
                    return None;
                }
                // should always return Some, as the branch of a Candidate always exists
                self.branch_knowledge(id.clone())
                    .map(|branch_knowledge| (know_most, branch_knowledge))
            }
            // request only Candidates
            _ => None,
        }
    }

    /// How much interest we have for requesting the block.
    pub fn request_interest(&self, id: &BlockIdFor<J>) -> Interest<I, J> {
        match self.prepare_request_info(id) {
            Some((know_most, branch_knowledge)) => match &self.highest_justified == id {
                true => Interest::HighestJustified {
                    know_most,
                    branch_knowledge,
                },
                false => Interest::Required {
                    know_most,
                    branch_knowledge,
                },
            },
            None => Interest::Uninterested,
        }
    }

    /// Whether we would like to eventually import this block.
    pub fn importable(&self, id: &BlockIdFor<J>) -> bool {
        use VertexHandle::Candidate;
        match self.get(id) {
            Candidate(vertex) => vertex.vertex.importable(),
            _ => false,
        }
    }

    /// Whether this block should be skipped during importing.
    /// It either needs to be already imported, or too old to be checked.
    pub fn skippable(&self, id: &BlockIdFor<J>) -> bool {
        use SpecialState::{BelowMinimal, HighestFinalized};
        use VertexHandle::{Candidate, Special};
        match self.get(id) {
            Special(BelowMinimal | HighestFinalized) => true,
            Candidate(vertex) => vertex.vertex.imported(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Forest, Interest::*, MAX_DEPTH};
    use crate::{
        session::SessionBoundaryInfo,
        sync::{
            data::BranchKnowledge::*,
            mock::{Backend, MockHeader, MockJustification, MockPeerId},
            ChainStatus, Header, Justification,
        },
        SessionPeriod,
    };

    type MockForest = Forest<MockPeerId, MockJustification>;

    const SESSION_BOUNDARY_INFO: SessionBoundaryInfo = SessionBoundaryInfo::new(SessionPeriod(20));

    fn setup() -> (MockHeader, MockForest) {
        let (backend, _) = Backend::setup(SESSION_BOUNDARY_INFO);
        let header = backend
            .top_finalized()
            .expect("should return genesis")
            .header()
            .clone();
        let (forest, too_many_nonfinalized) = Forest::new(&backend).expect("should initialize");
        assert!(!too_many_nonfinalized);
        (header, forest)
    }

    #[test]
    fn initially_empty() {
        let (initial_header, mut forest) = setup();
        assert!(forest.try_finalize(&1).is_none());
        assert_eq!(forest.request_interest(&initial_header.id()), Uninterested);
        assert!(!forest.importable(&initial_header.id()));
    }

    #[test]
    fn accepts_first_unimportant_id() {
        let (initial_header, mut forest) = setup();
        let child = initial_header.random_child();
        let peer_id = rand::random();
        assert!(!forest
            .update_block_identifier(&child.id(), Some(peer_id), false)
            .expect("it's not too high"));
        assert!(forest.try_finalize(&1).is_none());
        assert_eq!(forest.request_interest(&child.id()), Uninterested);
        assert!(!forest.importable(&child.id()));
    }

    #[test]
    fn accepts_first_important_id() {
        let (initial_header, mut forest) = setup();
        let child = initial_header.random_child();
        let peer_id = rand::random();
        assert!(forest
            .update_block_identifier(&child.id(), Some(peer_id), true)
            .expect("it's not too high"));
        assert!(forest.try_finalize(&1).is_none());
        match forest.request_interest(&child.id()) {
            Required { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {other_state:?}."),
        }
        assert!(forest.importable(&child.id()));
        assert!(!forest
            .update_block_identifier(&child.id(), Some(peer_id), true)
            .expect("it's not too high"));
    }

    #[test]
    fn rejects_too_high_id() {
        let (initial_header, mut forest) = setup();
        let too_high = initial_header
            .random_branch()
            .nth(MAX_DEPTH as usize)
            .expect("the branch is infinite");
        let peer_id = rand::random();
        assert!(matches!(
            forest.update_block_identifier(&too_high.id(), Some(peer_id), true),
            Err(Error::TooNew)
        ));
    }

    #[test]
    fn accepts_first_unimportant_header() {
        let (initial_header, mut forest) = setup();
        let child = initial_header.random_child();
        let peer_id = rand::random();
        assert!(!forest
            .update_header(&child, Some(peer_id), false)
            .expect("header was correct"));
        assert!(forest.try_finalize(&1).is_none());
        assert_eq!(forest.request_interest(&child.id()), Uninterested);
        assert!(!forest.importable(&child.id()));
    }

    #[test]
    fn accepts_first_important_header() {
        let (initial_header, mut forest) = setup();
        let child = initial_header.random_child();
        let peer_id = rand::random();
        assert!(forest
            .update_header(&child, Some(peer_id), true)
            .expect("header was correct"));
        assert!(forest.try_finalize(&1).is_none());
        match forest.request_interest(&child.id()) {
            Required { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {other_state:?}."),
        }
        assert!(forest.importable(&child.id()));
        assert!(!forest
            .update_block_identifier(&child.id(), Some(peer_id), true)
            .expect("it's not too high"));
    }

    #[test]
    fn rejects_parentless_header() {
        let (_, mut forest) = setup();
        let parentless = MockHeader::random_parentless(43);
        let peer_id = rand::random();
        assert!(matches!(
            forest.update_header(&parentless, Some(peer_id), true),
            Err(Error::HeaderMissingParentId)
        ));
    }

    #[test]
    fn accepts_first_justification() {
        let (initial_header, mut forest) = setup();
        let child = MockJustification::for_header(initial_header.random_child());
        let peer_id = rand::random();
        assert!(forest
            .update_justification(child.clone(), Some(peer_id))
            .expect("header was correct"));
        assert!(forest.try_finalize(&1).is_none());
        match forest.request_interest(&child.header().id()) {
            HighestJustified { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected highest justified, got {other_state:?}."),
        }
        assert!(forest.importable(&child.id()));
    }

    #[test]
    fn ignores_genesis_justification() {
        let (_, mut forest) = setup();
        let parentless = MockJustification::for_header(MockHeader::random_parentless(0));
        let peer_id = rand::random();
        assert!(matches!(
            forest.update_justification(parentless, Some(peer_id)),
            Ok(false)
        ));
    }

    #[test]
    fn rejects_parentless_justification() {
        let (_, mut forest) = setup();
        let parentless = MockJustification::for_header(MockHeader::random_parentless(43));
        let peer_id = rand::random();
        assert!(matches!(
            forest.update_justification(parentless, Some(peer_id)),
            Err(Error::HeaderMissingParentId)
        ));
    }

    #[test]
    fn accepts_first_body() {
        let (initial_header, mut forest) = setup();
        let child = initial_header.random_child();
        forest.update_body(&child).expect("header was correct");
        assert!(forest.try_finalize(&1).is_none());
        assert_eq!(forest.request_interest(&child.id()), Uninterested);
        assert!(!forest.importable(&child.id()));
    }

    #[test]
    fn rejects_body_when_parent_unimported() {
        let (initial_header, mut forest) = setup();
        let child = initial_header.random_child();
        let grandchild = child.random_child();
        assert!(!forest
            .update_header(&child, None, false)
            .expect("header was correct"));
        assert!(matches!(
            forest.update_body(&grandchild),
            Err(Error::ParentNotImported)
        ));
        assert!(forest.try_finalize(&1).is_none());
        assert_eq!(forest.request_interest(&child.id()), Uninterested);
        assert!(!forest.importable(&child.id()));
        assert_eq!(forest.request_interest(&grandchild.id()), Uninterested);
        assert!(!forest.importable(&grandchild.id()));
    }

    #[test]
    fn finalizes_first_block() {
        let (initial_header, mut forest) = setup();
        let child = MockJustification::for_header(initial_header.random_child());
        let peer_id = rand::random();
        assert!(forest
            .update_justification(child.clone(), Some(peer_id))
            .expect("header was correct"));
        assert!(forest.try_finalize(&1).is_none());
        match forest.request_interest(&child.header().id()) {
            HighestJustified { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected highest justified, got {other_state:?}."),
        }
        assert!(forest.importable(&child.header().id()));
        forest
            .update_body(child.header())
            .expect("header was correct");
        assert_eq!(forest.try_finalize(&1).expect("the block is ready"), child);
    }

    #[test]
    fn required_becomes_highest_finalized() {
        let (initial_header, mut forest) = setup();
        let child = MockJustification::for_header(initial_header.random_child());
        let peer_id = rand::random();
        assert!(
            forest
                .update_header(child.header(), Some(peer_id), true)
                .expect("header was correct"),
            "should become required"
        );
        assert!(
            forest
                .update_justification(child.clone(), Some(peer_id))
                .expect("header was correct"),
            "should become highest justified"
        );
    }

    #[test]
    fn non_required_becomes_highest_finalized() {
        let (initial_header, mut forest) = setup();
        let child = MockJustification::for_header(initial_header.random_child());
        let grandchild = child.header().random_child();
        let peer_id = rand::random();
        assert!(
            !forest
                .update_header(child.header(), Some(peer_id), false)
                .expect("header was correct"),
            "should not become required"
        );
        assert!(
            forest
                .update_header(&grandchild, Some(peer_id), true)
                .expect("header was correct"),
            "should become required"
        );
        assert!(
            forest
                .update_justification(child.clone(), Some(peer_id))
                .expect("header was correct"),
            "should become highest justified"
        );
    }

    #[test]
    fn ancestor_does_not_become_highest_finalized() {
        let (initial_header, mut forest) = setup();
        let child = MockJustification::for_header(initial_header.random_child());
        let grandchild = MockJustification::for_header(child.header().random_child());
        let peer_id = rand::random();
        assert!(
            forest
                .update_justification(grandchild, Some(peer_id))
                .expect("header was correct"),
            "should become highest justified"
        );
        assert!(
            !forest
                .update_justification(child, Some(peer_id))
                .expect("header was correct"),
            "should not become highest justified"
        );
    }

    #[test]
    fn prunes_forks() {
        let (initial_header, mut forest) = setup();
        let child = MockJustification::for_header(initial_header.random_child());
        let fork_child = initial_header.random_child();
        let peer_id = rand::random();
        let fork_peer_id = rand::random();
        assert!(forest
            .update_header(&fork_child, Some(fork_peer_id), true)
            .expect("header was correct"));
        match forest.request_interest(&fork_child.id()) {
            Required { know_most, .. } => assert!(know_most.contains(&fork_peer_id)),
            other_state => panic!("Expected required, got {other_state:?}."),
        }
        assert!(forest.importable(&fork_child.id()));
        assert!(forest
            .update_justification(child.clone(), Some(peer_id))
            .expect("header was correct"));
        forest
            .update_body(child.header())
            .expect("header was correct");
        assert_eq!(forest.try_finalize(&1).expect("the block is ready"), child);
        assert_eq!(forest.request_interest(&fork_child.id()), Uninterested);
        assert!(!forest.importable(&fork_child.id()));
        assert_eq!(
            forest.update_header(&fork_child, Some(fork_peer_id), true),
            Ok(false)
        );
    }

    #[test]
    fn uninterested_in_forks() {
        let (initial_header, mut forest) = setup();
        let fork_branch: Vec<_> = initial_header.random_branch().take(2).collect();
        for header in &fork_branch {
            let peer_id = rand::random();
            assert!(forest
                .update_header(header, Some(peer_id), true)
                .expect("header was correct"));
            match forest.request_interest(&header.id()) {
                Required { know_most, .. } => assert!(know_most.contains(&peer_id)),
                other_state => panic!("Expected required, got {other_state:?}."),
            }
            assert!(forest.importable(&header.id()));
        }
        let child = MockJustification::for_header(initial_header.random_child());
        let peer_id = rand::random();
        assert!(forest
            .update_justification(child.clone(), Some(peer_id))
            .expect("header was correct"));
        forest
            .update_body(child.header())
            .expect("header was correct");
        assert_eq!(forest.try_finalize(&1).expect("the block is ready"), child);
        for header in fork_branch {
            assert_eq!(forest.request_interest(&header.id()), Uninterested);
            assert!(!forest.importable(&header.id()));
        }
    }

    #[test]
    fn updates_importability_on_parent_connect() {
        let (initial_header, mut forest) = setup();
        let branch: Vec<_> = initial_header.random_branch().take(4).collect();
        let header = &branch[0];
        let peer_id = rand::random();
        assert!(forest
            .update_header(header, Some(peer_id), true)
            .expect("header was correct"));
        match forest.request_interest(&header.id()) {
            Required { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected required, got {other_state:?}."),
        }
        assert!(forest.importable(&header.id()));
        let header = &branch[1];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        assert_eq!(forest.request_interest(&header.id()), Uninterested);
        assert!(!forest.importable(&header.id()));
        let header = &branch[3];
        let peer_id = rand::random();
        assert!(forest
            .update_header(header, Some(peer_id), true)
            .expect("header was correct"));
        match forest.request_interest(&header.id()) {
            Required { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected required, got {other_state:?}."),
        }
        assert!(forest.importable(&header.id()));
        let header = &branch[2];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        for header in branch.iter().take(3) {
            assert!(forest.importable(&header.id()));
        }
        assert!(matches!(
            forest.request_interest(&branch[3].id()),
            Required { .. }
        ));
    }

    #[test]
    fn finds_ancestors() {
        let (initial_header, mut forest) = setup();
        let branch: Vec<_> = initial_header.random_branch().take(4).collect();
        let header = &branch[0];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        assert_eq!(forest.request_interest(&header.id()), Uninterested);
        assert!(!forest.importable(&header.id()));
        // skip branch[1]
        let header = &branch[2];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        assert_eq!(forest.request_interest(&header.id()), Uninterested);
        assert!(!forest.importable(&header.id()));
        let header = &branch[3];
        let peer_id = rand::random();
        assert!(forest
            .update_header(header, Some(peer_id), true)
            .expect("header was correct"));
        match forest.request_interest(&header.id()) {
            Required {
                know_most,
                branch_knowledge,
            } => {
                assert!(know_most.contains(&peer_id));
                // we only know parent from branch[2], namely branch[1]
                assert_eq!(branch_knowledge, LowestId(branch[1].id()));
            }
            other_state => panic!("Expected required, got {other_state:?}."),
        }
        assert!(forest.importable(&header.id()));
        // fill the gap
        let header = &branch[1];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        for header in branch.iter().take(3) {
            assert!(matches!(
                forest.request_interest(&header.id()),
                Uninterested
            ));
            assert!(forest.importable(&header.id()));
        }
        match forest.request_interest(&branch[3].id()) {
            Required {
                branch_knowledge, ..
            } => {
                // now we know all ancestors
                assert_eq!(branch_knowledge, TopImported(initial_header.id()));
            }
            other_state => panic!("Expected required, got {other_state:?}."),
        }
        assert!(forest.importable(&branch[3].id()));
        forest.update_body(&branch[0]).expect("should import");
        forest.update_body(&branch[1]).expect("should import");
        match forest.request_interest(&branch[3].id()) {
            Required {
                branch_knowledge, ..
            } => {
                // we know all ancestors, three blocks were imported
                assert_eq!(branch_knowledge, TopImported(branch[1].id()));
            }
            other_state => panic!("Expected required, got {other_state:?}."),
        }
        assert!(forest.importable(&branch[3].id()));
    }

    const HUGE_BRANCH_LENGTH: usize = MAX_DEPTH as usize;

    #[test]
    fn finalizes_huge_branch() {
        let (initial_header, mut forest) = setup();
        let justifications: Vec<_> = initial_header
            .random_branch()
            .map(MockJustification::for_header)
            .take(HUGE_BRANCH_LENGTH)
            .collect();
        for justification in &justifications {
            let peer_id = rand::random();
            assert!(forest
                .update_justification(justification.clone(), Some(peer_id))
                .expect("header was correct"));
            match forest.request_interest(&justification.header().id()) {
                HighestJustified { know_most, .. } => assert!(know_most.contains(&peer_id)),
                other_state => panic!("Expected highest justified, got {other_state:?}."),
            }
            assert!(forest.importable(&justification.header().id()));
            forest
                .update_body(justification.header())
                .expect("header was correct");
        }
        for (number, justification) in justifications.into_iter().enumerate() {
            assert_eq!(
                forest
                    .try_finalize(&(number as u32 + 1))
                    .expect("the block is ready"),
                justification
            );
        }
    }

    #[test]
    fn finalizes_huge_branch_with_justification_holes() {
        let (initial_header, mut forest) = setup();
        let justifications: Vec<_> = initial_header
            .random_branch()
            .map(MockJustification::for_header)
            .take(HUGE_BRANCH_LENGTH)
            .enumerate()
            .collect();
        for (number, justification) in &justifications {
            if number.is_power_of_two() {
                let peer_id = rand::random();
                assert!(forest
                    .update_justification(justification.clone(), Some(peer_id))
                    .expect("header was correct"));
                match forest.request_interest(&justification.header().id()) {
                    HighestJustified { know_most, .. } => assert!(know_most.contains(&peer_id)),
                    other_state => panic!("Expected highest justified, got {other_state:?}."),
                }
                assert!(forest.importable(&justification.header().id()));
            }
            forest
                .update_body(justification.header())
                .expect("header was correct");
        }
        for (number, justification) in justifications.into_iter() {
            if number.is_power_of_two() {
                assert_eq!(
                    forest
                        .try_finalize(&(number as u32 + 1))
                        .expect("the block is ready"),
                    justification
                );
            } else {
                assert!(forest.try_finalize(&(number as u32 + 1)).is_none());
            }
        }
    }

    #[test]
    fn prunes_huge_branch() {
        let (initial_header, mut forest) = setup();
        let fork: Vec<_> = initial_header
            .random_branch()
            .take(HUGE_BRANCH_LENGTH)
            .collect();
        for header in &fork {
            let peer_id = rand::random();
            assert!(forest
                .update_header(header, Some(peer_id), true)
                .expect("header was correct"));
            assert!(forest.try_finalize(&1).is_none());
            match forest.request_interest(&header.id()) {
                Required { know_most, .. } => assert!(know_most.contains(&peer_id)),
                other_state => panic!("Expected required, got {other_state:?}."),
            }
            assert!(forest.importable(&header.id()));
        }
        let child = MockJustification::for_header(initial_header.random_child());
        let peer_id = rand::random();
        assert!(forest
            .update_justification(child.clone(), Some(peer_id))
            .expect("header was correct"));
        assert!(forest.try_finalize(&1).is_none());
        match forest.request_interest(&child.header().id()) {
            HighestJustified { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected highest justified, got {other_state:?}."),
        }
        assert!(forest.importable(&child.header().id()));
        forest
            .update_body(child.header())
            .expect("header was correct");
        assert_eq!(forest.try_finalize(&1).expect("the block is ready"), child);
        for header in &fork {
            assert_eq!(forest.request_interest(&header.id()), Uninterested);
            assert!(!forest.importable(&header.id()));
        }
    }

    #[test]
    fn updates_interest_on_huge_branch() {
        let (initial_header, mut forest) = setup();
        let branch: Vec<_> = initial_header
            .random_branch()
            .take(HUGE_BRANCH_LENGTH)
            .collect();
        for header in branch.iter().take(HUGE_BRANCH_LENGTH - 1) {
            let peer_id = rand::random();
            assert!(!forest
                .update_header(header, Some(peer_id), false)
                .expect("header was correct"));
            assert_eq!(forest.request_interest(&header.id()), Uninterested);
            assert!(!forest.importable(&header.id()));
        }
        let header = &branch[HUGE_BRANCH_LENGTH - 1];
        let peer_id = rand::random();
        assert!(forest
            .update_header(header, Some(peer_id), true)
            .expect("header was correct"));
        match forest.request_interest(&header.id()) {
            Required { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected required, got {other_state:?}."),
        }
        assert!(forest.importable(&header.id()));
        for header in branch.iter().take(HUGE_BRANCH_LENGTH - 1) {
            assert!(matches!(
                forest.request_interest(&header.id()),
                Uninterested
            ));
            assert!(forest.importable(&header.id()));
        }
    }
}
