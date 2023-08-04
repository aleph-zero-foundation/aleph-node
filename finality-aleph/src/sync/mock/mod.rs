use std::hash::Hash;

use parity_scale_codec::{Decode, Encode};
use sp_core::H256;

use crate::{
    sync::{Block, ChainStatusNotification, Header, Justification},
    BlockIdentifier,
};

mod backend;
mod status_notifier;

type MockNumber = u32;
type MockHash = H256;

pub use backend::Backend;

pub type MockPeerId = u32;

#[derive(Clone, Hash, Debug, PartialEq, Eq, Encode, Decode)]
pub struct MockIdentifier {
    number: MockNumber,
    hash: MockHash,
}

impl MockIdentifier {
    fn new(number: MockNumber, hash: MockHash) -> Self {
        MockIdentifier { number, hash }
    }

    pub fn new_random(number: MockNumber) -> Self {
        MockIdentifier::new(number, MockHash::random())
    }

    pub fn random_child(&self) -> MockHeader {
        let id = MockIdentifier::new_random(self.number + 1);
        let parent = Some(self.clone());
        MockHeader { id, parent }
    }

    pub fn random_branch(&self) -> impl Iterator<Item = MockHeader> {
        RandomBranch {
            parent: self.clone(),
        }
    }
}

impl BlockIdentifier for MockIdentifier {
    fn number(&self) -> u32 {
        self.number
    }
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, Encode, Decode)]
pub struct MockHeader {
    id: MockIdentifier,
    parent: Option<MockIdentifier>,
}

impl MockHeader {
    pub fn genesis() -> Self {
        MockHeader {
            id: MockIdentifier {
                number: 0,
                hash: MockHash::zero(),
            },
            parent: None,
        }
    }

    pub fn random_parentless(number: MockNumber) -> Self {
        let id = MockIdentifier::new_random(number);
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
    parent: MockIdentifier,
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
    type Identifier = MockIdentifier;

    fn id(&self) -> Self::Identifier {
        self.id.clone()
    }

    fn parent_id(&self) -> Option<Self::Identifier> {
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
    type Identifier = MockIdentifier;

    fn id(&self) -> Self::Identifier {
        self.header().id()
    }

    fn parent_id(&self) -> Option<Self::Identifier> {
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

impl Header for MockJustification {
    type Identifier = MockIdentifier;

    fn id(&self) -> Self::Identifier {
        self.header().id()
    }

    fn parent_id(&self) -> Option<Self::Identifier> {
        self.header().parent_id()
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
