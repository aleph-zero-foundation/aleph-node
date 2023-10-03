use std::hash::Hash;

use parity_scale_codec::{Decode, Encode};
use primitives::{BlockHash, BlockNumber};

use crate::{
    sync::{Block, ChainStatusNotification, Header, Justification, UnverifiedJustification},
    BlockId,
};

mod backend;
mod status_notifier;

pub use backend::Backend;

pub type MockPeerId = u32;

impl BlockId {
    pub fn new_random(number: BlockNumber) -> Self {
        Self::new(BlockHash::random(), number)
    }

    pub fn random_child(&self) -> MockHeader {
        let id = Self::new_random(self.number + 1);
        let parent = Some(self.clone());
        MockHeader { id, parent }
    }

    pub fn random_branch(&self) -> impl Iterator<Item = MockHeader> {
        RandomBranch {
            parent: self.clone(),
        }
    }
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, Encode, Decode)]
pub struct MockHeader {
    id: BlockId,
    parent: Option<BlockId>,
}

impl MockHeader {
    pub fn genesis() -> Self {
        MockHeader {
            id: BlockId {
                number: 0,
                hash: BlockHash::zero(),
            },
            parent: None,
        }
    }

    pub fn random_parentless(number: BlockNumber) -> Self {
        let id = BlockId::new_random(number);
        MockHeader { id, parent: None }
    }

    pub fn random_child(&self) -> Self {
        self.id.random_child()
    }

    pub fn random_branch(&self) -> impl Iterator<Item = Self> {
        self.id.random_branch()
    }
}

struct RandomBranch {
    parent: BlockId,
}

impl Iterator for RandomBranch {
    type Item = MockHeader;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.parent.random_child();
        self.parent = result.id();
        Some(result)
    }
}

impl Header for MockHeader {
    fn id(&self) -> BlockId {
        self.id.clone()
    }

    fn parent_id(&self) -> Option<BlockId> {
        self.parent.clone()
    }
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, Encode, Decode)]
pub struct MockBlock {
    header: MockHeader,
    justification: Option<MockJustification>,
    is_correct: bool,
}

impl MockBlock {
    pub fn new(header: MockHeader, is_correct: bool) -> Self {
        Self {
            header,
            justification: None,
            is_correct,
        }
    }

    fn finalize(&mut self, justification: MockJustification) {
        self.justification = Some(justification);
    }

    pub fn verify(&self) -> bool {
        self.is_correct
    }
}

impl Header for MockBlock {
    fn id(&self) -> BlockId {
        self.header().id()
    }

    fn parent_id(&self) -> Option<BlockId> {
        self.header().parent_id()
    }
}

impl Block for MockBlock {
    type Header = MockHeader;

    fn header(&self) -> &Self::Header {
        &self.header
    }
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, Encode, Decode)]
pub struct MockJustification {
    header: MockHeader,
    is_correct: bool,
}

impl MockJustification {
    pub fn for_header(header: MockHeader) -> Self {
        Self {
            header,
            is_correct: true,
        }
    }
}

impl UnverifiedJustification for MockJustification {
    type Header = MockHeader;

    fn header(&self) -> &Self::Header {
        &self.header
    }
}

impl Justification for MockJustification {
    type Header = MockHeader;
    type Unverified = Self;

    fn header(&self) -> &Self::Header {
        &self.header
    }

    fn into_unverified(self) -> Self::Unverified {
        self
    }
}

type MockNotification = ChainStatusNotification<MockHeader>;
