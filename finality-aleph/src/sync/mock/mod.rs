use std::hash::Hash;

use codec::{Decode, Encode};
use sp_core::H256;

use crate::sync::{
    BlockIdentifier, BlockStatus, ChainStatusNotification, Header, Justification as JustificationT,
};

mod backend;
mod status_notifier;
mod verifier;

type MockNumber = u32;
type MockHash = H256;

pub use backend::Backend;
pub use verifier::MockVerifier;

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
    fn new(id: MockIdentifier, parent: Option<MockIdentifier>) -> Self {
        MockHeader { id, parent }
    }

    pub fn random_parentless(number: MockNumber) -> Self {
        let id = MockIdentifier::new_random(number);
        MockHeader { id, parent: None }
    }

    pub fn random_child(&self) -> Self {
        let id = MockIdentifier::new_random(self.id.number() + 1);
        let parent = Some(self.id.clone());
        MockHeader { id, parent }
    }

    pub fn random_branch(&self) -> impl Iterator<Item = Self> {
        RandomBranch {
            parent: self.clone(),
        }
    }
}

struct RandomBranch {
    parent: MockHeader,
}

impl Iterator for RandomBranch {
    type Item = MockHeader;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.parent.random_child();
        self.parent = result.clone();
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

    pub fn for_header_incorrect(header: MockHeader) -> Self {
        Self {
            header,
            is_correct: false,
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

impl JustificationT for MockJustification {
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
type MockBlockStatus = BlockStatus<MockJustification>;
