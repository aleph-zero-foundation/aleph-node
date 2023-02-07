use std::collections::{
    hash_map::{Entry, OccupiedEntry, VacantEntry},
    HashMap, HashSet,
};

use crate::sync::{BlockIdFor, BlockIdentifier, Header, Justification, PeerId};

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

/// Our interest in a block referred to by a vertex, including the information about whom we expect to have the block.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Interest<I: PeerId> {
    /// We are not interested in this block.
    Uninterested,
    /// We would like to have this block.
    Required(HashSet<I>),
    /// We would like to have this block and its the highest on its branch.
    TopRequired(HashSet<I>),
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
    root_id: BlockIdFor<J>,
    root_children: HashSet<BlockIdFor<J>>,
    compost_bin: HashSet<BlockIdFor<J>>,
}

impl<I: PeerId, J: Justification> Forest<I, J> {
    pub fn new(highest_justified: BlockIdFor<J>) -> Self {
        Self {
            vertices: HashMap::new(),
            top_required: HashSet::new(),
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

    /// Updates the vertex related to the provided header marking it as imported. Returns whether
    /// it is now finalizable, or errors when it's impossible to do consistently.
    pub fn update_body(&mut self, header: &J::Header) -> Result<bool, Error> {
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
            Candidate(mut entry) => Ok(entry.get_mut().vertex.insert_body(parent_id.clone())),
            _ => Err(Error::IncorrectVertexState),
        }
    }

    /// Updates the provided justification, returns whether either finalization is now possible or
    /// the vertex became a new top required.
    pub fn update_justification(
        &mut self,
        justification: J,
        holder: Option<I>,
    ) -> Result<JustificationAddResult, Error> {
        use JustificationAddResult::*;
        let (id, parent_id) = self.process_header(justification.header())?;
        self.update_header(justification.header(), None, false)?;
        match self.get_mut(&id) {
            VertexHandle::Candidate(mut entry) => {
                match entry.get_mut().vertex.insert_justification(
                    parent_id.clone(),
                    justification,
                    holder,
                ) {
                    Noop => Ok(Noop),
                    Required => {
                        self.top_required.insert(id.clone());
                        self.set_required(&parent_id);
                        Ok(Required)
                    }
                    Finalizable => {
                        self.top_required.remove(&id);
                        Ok(Finalizable)
                    }
                }
            }
            _ => Ok(Noop),
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
    }

    /// Attempt to finalize one block, returns the correct justification if successful.
    pub fn try_finalize(&mut self) -> Option<J> {
        for child_id in self.root_children.clone().into_iter() {
            if let Some(VertexWithChildren { vertex, children }) = self.vertices.remove(&child_id) {
                match vertex.ready() {
                    Ok(justification) => {
                        self.root_id = child_id;
                        self.root_children = children;
                        self.prune_level(self.root_id.number());
                        return Some(justification);
                    }
                    Err(vertex) => {
                        self.vertices
                            .insert(child_id, VertexWithChildren { vertex, children });
                    }
                }
            }
        }
        None
    }

    /// How much interest we have for the block.
    pub fn state(&mut self, id: &BlockIdFor<J>) -> Interest<I> {
        match self.get_mut(id) {
            VertexHandle::Candidate(entry) => {
                let vertex = &entry.get().vertex;
                let know_most = vertex.know_most().clone();
                match vertex.required() {
                    true => match self.top_required.contains(id) {
                        true => Interest::TopRequired(know_most),
                        false => Interest::Required(know_most),
                    },
                    false => Interest::Uninterested,
                }
            }
            _ => Interest::Uninterested,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Forest, Interest::*, JustificationAddResult, MAX_DEPTH};
    use crate::sync::{
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
        assert!(forest.try_finalize().is_none());
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
        assert!(forest.try_finalize().is_none());
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
        assert!(forest.try_finalize().is_none());
        match forest.state(&child.id()) {
            TopRequired(holders) => assert!(holders.contains(&peer_id)),
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
        assert!(forest.try_finalize().is_none());
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
        assert!(forest.try_finalize().is_none());
        match forest.state(&child.id()) {
            TopRequired(holders) => assert!(holders.contains(&peer_id)),
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
        assert_eq!(
            forest
                .update_justification(child.clone(), Some(peer_id))
                .expect("header was correct"),
            JustificationAddResult::Required
        );
        assert!(forest.try_finalize().is_none());
        match forest.state(&child.header().id()) {
            TopRequired(holders) => assert!(holders.contains(&peer_id)),
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
        assert!(!forest.update_body(&child).expect("header was correct"));
        assert!(forest.try_finalize().is_none());
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
        assert!(forest.try_finalize().is_none());
        assert_eq!(forest.state(&child.id()), Uninterested);
        assert_eq!(forest.state(&grandchild.id()), Uninterested);
    }

    #[test]
    fn finalizes_first_block() {
        let (initial_header, mut forest) = setup();
        let child = MockJustification::for_header(initial_header.random_child());
        let peer_id = rand::random();
        assert_eq!(
            forest
                .update_justification(child.clone(), Some(peer_id))
                .expect("header was correct"),
            JustificationAddResult::Required
        );
        assert!(forest.try_finalize().is_none());
        match forest.state(&child.header().id()) {
            TopRequired(holders) => assert!(holders.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        assert!(forest
            .update_body(child.header())
            .expect("header was correct"));
        assert_eq!(forest.try_finalize().expect("the block is ready"), child);
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
            TopRequired(holders) => assert!(holders.contains(&fork_peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        assert_eq!(
            forest
                .update_justification(child.clone(), Some(peer_id))
                .expect("header was correct"),
            JustificationAddResult::Required
        );
        assert!(forest
            .update_body(child.header())
            .expect("header was correct"));
        assert_eq!(forest.try_finalize().expect("the block is ready"), child);
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
                TopRequired(holders) => assert!(holders.contains(&peer_id)),
                other_state => panic!("Expected top required, got {:?}.", other_state),
            }
        }
        let child = MockJustification::for_header(initial_header.random_child());
        let peer_id = rand::random();
        assert_eq!(
            forest
                .update_justification(child.clone(), Some(peer_id))
                .expect("header was correct"),
            JustificationAddResult::Required
        );
        assert!(forest
            .update_body(child.header())
            .expect("header was correct"));
        assert_eq!(forest.try_finalize().expect("the block is ready"), child);
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
            TopRequired(holders) => assert!(holders.contains(&peer_id)),
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
            TopRequired(holders) => assert!(holders.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        let header = &branch[2];
        let peer_id = rand::random();
        assert!(!forest
            .update_header(header, Some(peer_id), false)
            .expect("header was correct"));
        for header in branch.iter().take(3) {
            assert!(matches!(forest.state(&header.id()), Required(_)));
        }
        assert!(matches!(forest.state(&branch[3].id()), TopRequired(_)));
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
            assert_eq!(
                forest
                    .update_justification(justification.clone(), Some(peer_id))
                    .expect("header was correct"),
                JustificationAddResult::Required
            );
            match forest.state(&justification.header().id()) {
                TopRequired(holders) => assert!(holders.contains(&peer_id)),
                other_state => panic!("Expected top required, got {:?}.", other_state),
            }
            assert!(forest
                .update_body(justification.header())
                .expect("header was correct"));
        }
        for justification in justifications {
            assert_eq!(
                forest.try_finalize().expect("the block is ready"),
                justification
            );
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
            assert!(forest.try_finalize().is_none());
            match forest.state(&header.id()) {
                TopRequired(holders) => assert!(holders.contains(&peer_id)),
                other_state => panic!("Expected top required, got {:?}.", other_state),
            }
        }
        let child = MockJustification::for_header(initial_header.random_child());
        let peer_id = rand::random();
        assert_eq!(
            forest
                .update_justification(child.clone(), Some(peer_id))
                .expect("header was correct"),
            JustificationAddResult::Required
        );
        assert!(forest.try_finalize().is_none());
        match forest.state(&child.header().id()) {
            TopRequired(holders) => assert!(holders.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        assert!(forest
            .update_body(child.header())
            .expect("header was correct"));
        assert_eq!(forest.try_finalize().expect("the block is ready"), child);
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
            TopRequired(holders) => assert!(holders.contains(&peer_id)),
            other_state => panic!("Expected top required, got {:?}.", other_state),
        }
        for header in branch.iter().take(HUGE_BRANCH_LENGTH - 1) {
            assert!(matches!(forest.state(&header.id()), Required(_)));
        }
    }
}
