use std::{
    collections::{
        hash_map::{Entry, OccupiedEntry, VacantEntry},
        HashMap, HashSet,
    },
    fmt::{Display, Error as FmtError, Formatter},
};

use crate::sync::{
    data::BranchKnowledge, BlockIdFor, BlockIdentifier, Header, Justification, PeerId,
};

mod vertex;

pub use vertex::JustificationAddResult;
use vertex::Vertex;

pub struct JustificationWithParent<J: Justification> {
    pub justification: J,
    pub parent: BlockIdFor<J>,
}

enum VertexHandle<'a, I: PeerId, J: Justification> {
    HopelessFork,
    BelowMinimal,
    HighestFinalized,
    Unknown(VacantEntry<'a, BlockIdFor<J>, VertexWithChildren<I, J>>),
    Candidate(OccupiedEntry<'a, BlockIdFor<J>, VertexWithChildren<I, J>>),
}

/// Our interest in a block referred to by a vertex,
/// including all the information required to prepare a request.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Interest<I: PeerId, J: Justification> {
    /// We are not interested in this block.
    Uninterested,
    /// We would like to have this block.
    Required {
        know_most: HashSet<I>,
        branch_knowledge: BranchKnowledge<J>,
    },
    /// We would like to have this block and its the highest on its branch.
    TopRequired {
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
            TooNew => write!(f, "block too new to be considered"),
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
const MAX_DEPTH: u32 = 1800;

pub struct Forest<I: PeerId, J: Justification> {
    vertices: HashMap<BlockIdFor<J>, VertexWithChildren<I, J>>,
    top_required: HashSet<BlockIdFor<J>>,
    justified_blocks: HashMap<u32, BlockIdFor<J>>,
    root_id: BlockIdFor<J>,
    root_children: HashSet<BlockIdFor<J>>,
    compost_bin: HashSet<BlockIdFor<J>>,
}

impl<I: PeerId, J: Justification> Forest<I, J> {
    pub fn new(highest_justified: BlockIdFor<J>) -> Self {
        Self {
            vertices: HashMap::new(),
            top_required: HashSet::new(),
            justified_blocks: HashMap::new(),
            root_id: highest_justified,
            root_children: HashSet::new(),
            compost_bin: HashSet::new(),
        }
    }

    fn get_mut(&mut self, id: &BlockIdFor<J>) -> VertexHandle<I, J> {
        use VertexHandle::*;
        if id == &self.root_id {
            HighestFinalized
        } else if id.number() <= self.root_id.number() {
            BelowMinimal
        } else if self.compost_bin.contains(id) {
            HopelessFork
        } else {
            match self.vertices.entry(id.clone()) {
                Entry::Occupied(entry) => Candidate(entry),
                Entry::Vacant(entry) => Unknown(entry),
            }
        }
    }

    fn prune(&mut self, id: &BlockIdFor<J>) {
        self.top_required.remove(id);
        if let Some(VertexWithChildren { children, .. }) = self.vertices.remove(id) {
            self.compost_bin.insert(id.clone());
            for child in children {
                self.prune(&child);
            }
        }
    }

    fn connect_parent(&mut self, id: &BlockIdFor<J>) {
        use VertexHandle::*;
        if let Candidate(mut entry) = self.get_mut(id) {
            let vertex = entry.get_mut();
            let required = vertex.vertex.required();
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
                    HighestFinalized => {
                        self.root_children.insert(id.clone());
                    }
                    Candidate(mut entry) => {
                        entry.get_mut().add_child(id.clone());
                        if required {
                            self.set_required(&parent_id);
                        }
                    }
                    HopelessFork | BelowMinimal => self.prune(id),
                };
            };
        };
    }

    fn set_required(&mut self, id: &BlockIdFor<J>) {
        self.top_required.remove(id);
        if let VertexHandle::Candidate(mut entry) = self.get_mut(id) {
            let vertex = entry.get_mut();
            if vertex.vertex.set_required() {
                if let Some(id) = vertex.vertex.parent().cloned() {
                    self.set_required(&id);
                }
            }
        }
    }

    fn set_top_required(&mut self, id: &BlockIdFor<J>) -> bool {
        match self.get_mut(id) {
            VertexHandle::Candidate(mut entry) => match entry.get_mut().vertex.set_required() {
                true => {
                    if let Some(parent_id) = entry.get_mut().vertex.parent().cloned() {
                        self.set_required(&parent_id);
                    }
                    self.top_required.insert(id.clone());
                    true
                }
                false => false,
            },
            _ => false,
        }
    }

    fn insert_id(&mut self, id: BlockIdFor<J>, holder: Option<I>) -> Result<(), Error> {
        if id.number() > self.root_id.number() + MAX_DEPTH {
            return Err(Error::TooNew);
        }
        self.vertices
            .entry(id)
            .or_insert_with(VertexWithChildren::new)
            .vertex
            .add_block_holder(holder);
        Ok(())
    }

    fn process_header(
        &mut self,
        header: &J::Header,
    ) -> Result<(BlockIdFor<J>, BlockIdFor<J>), Error> {
        Ok((
            header.id(),
            header.parent_id().ok_or(Error::HeaderMissingParentId)?,
        ))
    }

    /// Updates the provider block identifier, returns whether it became a new top required.
    pub fn update_block_identifier(
        &mut self,
        id: &BlockIdFor<J>,
        holder: Option<I>,
        required: bool,
    ) -> Result<bool, Error> {
        self.insert_id(id.clone(), holder)?;
        match required {
            true => Ok(self.set_top_required(id)),
            false => Ok(false),
        }
    }

    /// Updates the provided header, returns whether it became a new top required.
    pub fn update_header(
        &mut self,
        header: &J::Header,
        holder: Option<I>,
        required: bool,
    ) -> Result<bool, Error> {
        let (id, parent_id) = self.process_header(header)?;
        self.insert_id(id.clone(), holder.clone())?;
        if let VertexHandle::Candidate(mut entry) = self.get_mut(&id) {
            entry.get_mut().vertex.insert_header(parent_id, holder);
            self.connect_parent(&id);
        }
        match required {
            true => Ok(self.set_top_required(&id)),
            false => Ok(false),
        }
    }

    /// Updates the vertex related to the provided header marking it as imported.
    /// Returns errors when it's impossible to do consistently.
    pub fn update_body(&mut self, header: &J::Header) -> Result<(), Error> {
        use VertexHandle::*;
        let (id, parent_id) = self.process_header(header)?;
        self.update_header(header, None, false)?;
        match self.get_mut(&parent_id) {
            Candidate(entry) => {
                if !entry.get().vertex.imported() {
                    return Err(Error::ParentNotImported);
                }
            }
            HighestFinalized => (),
            Unknown(_) | HopelessFork | BelowMinimal => return Err(Error::IncorrectParentState),
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

    /// Updates the provided justification.
    /// Returns whether the vertex became a new top required.
    pub fn update_justification(
        &mut self,
        justification: J,
        holder: Option<I>,
    ) -> Result<bool, Error> {
        use JustificationAddResult::*;
        let (id, parent_id) = self.process_header(justification.header())?;
        self.update_header(justification.header(), None, false)?;
        match self.get_mut(&id) {
            VertexHandle::Candidate(mut entry) => {
                let vertex = &mut entry.get_mut().vertex;
                match vertex.insert_justification(parent_id.clone(), justification, holder) {
                    Noop => Ok(false),
                    Required => {
                        self.top_required.insert(id.clone());
                        self.set_required(&parent_id);
                        Ok(true)
                    }
                    Finalizable => {
                        self.top_required.remove(&id);
                        self.justified_blocks.insert(id.number(), id.clone());
                        Ok(false)
                    }
                }
            }
            _ => Ok(false),
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
        // cached as ready
        if let Some(id) = self.justified_blocks.get(number) {
            if let Some(VertexWithChildren { vertex, children }) = self.vertices.remove(id) {
                match vertex.ready() {
                    // ready indeed
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
    fn branch_knowledge(&mut self, mut id: BlockIdFor<J>) -> Option<BranchKnowledge<J>> {
        use VertexHandle::*;
        // traverse ancestors till we reach something imported or a parentless vertex
        loop {
            match self.get_mut(&id) {
                Candidate(entry) => {
                    // first encounter of an imported ancestor, return it
                    if entry.get().vertex.imported() {
                        return Some(BranchKnowledge::TopImported(id));
                    }
                    // try update current id to parent_id
                    match entry.get().vertex.parent().cloned() {
                        // it has a parent, continue
                        Some(parent_id) => id = parent_id,
                        // does not have parent, thus is the lowest known,
                        // and is not imported (a Candidate is not the HighestFinalized),
                        // return it
                        None => return Some(BranchKnowledge::LowestId(id)),
                    };
                }
                // we've reached the root, hence this is the top imported ancestor, return it
                HighestFinalized => {
                    return Some(BranchKnowledge::TopImported(id));
                }
                // either we don't know the requested id, or it will never connect to the root,
                // return None
                HopelessFork | BelowMinimal | Unknown(_) => return None,
            };
        }
    }

    /// Prepare additional info required to create a request for the block.
    /// Returns `None` if we're not interested in the block.
    fn prepare_request_info(
        &mut self,
        id: &BlockIdFor<J>,
    ) -> Option<(HashSet<I>, BranchKnowledge<J>)> {
        use VertexHandle::Candidate;
        match self.get_mut(id) {
            Candidate(entry) => {
                // request only required blocks
                if !&entry.get().vertex.required() {
                    return None;
                }
                let know_most = entry.get().vertex.know_most().clone();
                // should always return Some, as the branch of a Candidate always exists
                self.branch_knowledge(id.clone())
                    .map(|branch_knowledge| (know_most, branch_knowledge))
            }
            // request only Candidates
            _ => None,
        }
    }

    /// How much interest we have for the block.
    pub fn state(&mut self, id: &BlockIdFor<J>) -> Interest<I, J> {
        match self.prepare_request_info(id) {
            Some((know_most, branch_knowledge)) => match self.top_required.contains(id) {
                true => Interest::TopRequired {
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
}

#[cfg(test)]
mod tests {
    use super::{Error, Forest, Interest::*, MAX_DEPTH};
    use crate::sync::{
        data::BranchKnowledge::*,
        mock::{MockHeader, MockJustification, MockPeerId},
        Header, Justification,
    };

    type MockForest = Forest<MockPeerId, MockJustification>;

    fn setup() -> (MockHeader, MockForest) {
        let header = MockHeader::random_parentless(0);
        let forest = Forest::new(header.id());
        (header, forest)
    }

    #[test]
    fn initially_empty() {
        let (initial_header, mut forest) = setup();
        assert!(forest.try_finalize(&1).is_none());
        assert_eq!(forest.state(&initial_header.id()), Uninterested);
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
        assert_eq!(forest.state(&child.id()), Uninterested);
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
        match forest.state(&child.id()) {
            TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
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
        assert_eq!(
            forest.update_block_identifier(&too_high.id(), Some(peer_id), true),
            Err(Error::TooNew)
        );
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
        assert_eq!(forest.state(&child.id()), Uninterested);
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
        match forest.state(&child.id()) {
            TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
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
        match forest.state(&child.header().id()) {
            TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
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
        assert_eq!(forest.state(&child.id()), Uninterested);
    }

    #[test]
    fn rejects_body_when_parent_unimported() {
        let (initial_header, mut forest) = setup();
        let child = initial_header.random_child();
        let grandchild = child.random_child();
        assert!(!forest
            .update_header(&child, None, false)
            .expect("header was correct"));
        assert_eq!(
            forest.update_body(&grandchild),
            Err(Error::ParentNotImported)
        );
        assert!(forest.try_finalize(&1).is_none());
        assert_eq!(forest.state(&child.id()), Uninterested);
        assert_eq!(forest.state(&grandchild.id()), Uninterested);
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
        match forest.state(&child.header().id()) {
            TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        forest
            .update_body(child.header())
            .expect("header was correct");
        assert_eq!(forest.try_finalize(&1).expect("the block is ready"), child);
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
        match forest.state(&fork_child.id()) {
            TopRequired { know_most, .. } => assert!(know_most.contains(&fork_peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        assert!(forest
            .update_justification(child.clone(), Some(peer_id))
            .expect("header was correct"));
        forest
            .update_body(child.header())
            .expect("header was correct");
        assert_eq!(forest.try_finalize(&1).expect("the block is ready"), child);
        assert_eq!(forest.state(&fork_child.id()), Uninterested);
        assert!(!forest
            .update_header(&fork_child, Some(fork_peer_id), true)
            .expect("header was correct"));
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
            match forest.state(&header.id()) {
                TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
                other_state => panic!("Expected top required, got {:?}.", other_state),
            }
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
            assert_eq!(forest.state(&header.id()), Uninterested);
        }
    }

    #[test]
    fn updates_interest_on_parent_connect() {
        let (initial_header, mut forest) = setup();
        let branch: Vec<_> = initial_header.random_branch().take(4).collect();
        let header = &branch[0];
        let peer_id = rand::random();
        assert!(forest
            .update_header(header, Some(peer_id), true)
            .expect("header was correct"));
        match forest.state(&header.id()) {
            TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        let header = &branch[1];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        assert_eq!(forest.state(&header.id()), Uninterested);
        let header = &branch[3];
        let peer_id = rand::random();
        assert!(forest
            .update_header(header, Some(peer_id), true)
            .expect("header was correct"));
        match forest.state(&header.id()) {
            TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        let header = &branch[2];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        for header in branch.iter().take(3) {
            assert!(matches!(forest.state(&header.id()), Required { .. }));
        }
        assert!(matches!(forest.state(&branch[3].id()), TopRequired { .. }));
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
        assert_eq!(forest.state(&header.id()), Uninterested);
        // skip branch[1]
        let header = &branch[2];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        assert_eq!(forest.state(&header.id()), Uninterested);
        let header = &branch[3];
        let peer_id = rand::random();
        assert!(forest
            .update_header(header, Some(peer_id), true)
            .expect("header was correct"));
        match forest.state(&header.id()) {
            TopRequired {
                know_most,
                branch_knowledge,
            } => {
                assert!(know_most.contains(&peer_id));
                // we only know parent from branch[2], namely branch[1]
                assert_eq!(branch_knowledge, LowestId(branch[1].id()));
            }
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        // fill the gap
        let header = &branch[1];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        for header in branch.iter().take(3) {
            assert!(matches!(forest.state(&header.id()), Required { .. }));
        }
        match forest.state(&branch[3].id()) {
            TopRequired {
                branch_knowledge, ..
            } => {
                // now we know all ancestors
                assert_eq!(branch_knowledge, TopImported(initial_header.id()));
            }
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        forest.update_body(&branch[0]).expect("should import");
        forest.update_body(&branch[1]).expect("should import");
        match forest.state(&branch[3].id()) {
            TopRequired {
                branch_knowledge, ..
            } => {
                // we know all ancestors, three blocks were imported
                assert_eq!(branch_knowledge, TopImported(branch[1].id()));
            }
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
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
            match forest.state(&justification.header().id()) {
                TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
                other_state => panic!("Expected top required, got {:?}.", other_state),
            }
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
                match forest.state(&justification.header().id()) {
                    TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
                    other_state => panic!("Expected top required, got {:?}.", other_state),
                }
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
            match forest.state(&header.id()) {
                TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
                other_state => panic!("Expected top required, got {:?}.", other_state),
            }
        }
        let child = MockJustification::for_header(initial_header.random_child());
        let peer_id = rand::random();
        assert!(forest
            .update_justification(child.clone(), Some(peer_id))
            .expect("header was correct"));
        assert!(forest.try_finalize(&1).is_none());
        match forest.state(&child.header().id()) {
            TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        forest
            .update_body(child.header())
            .expect("header was correct");
        assert_eq!(forest.try_finalize(&1).expect("the block is ready"), child);
        for header in &fork {
            assert_eq!(forest.state(&header.id()), Uninterested);
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
            assert_eq!(forest.state(&header.id()), Uninterested);
        }
        let header = &branch[HUGE_BRANCH_LENGTH - 1];
        let peer_id = rand::random();
        assert!(forest
            .update_header(header, Some(peer_id), true)
            .expect("header was correct"));
        match forest.state(&header.id()) {
            TopRequired { know_most, .. } => assert!(know_most.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        for header in branch.iter().take(HUGE_BRANCH_LENGTH - 1) {
            assert!(matches!(forest.state(&header.id()), Required { .. }));
        }
    }
}
