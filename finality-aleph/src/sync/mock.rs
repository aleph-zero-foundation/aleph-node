use codec::{Decode, Encode};

use crate::sync::{BlockIdentifier, Header, Justification};

pub type MockPeerId = u32;

#[derive(Clone, Hash, Debug, PartialEq, Eq, Encode, Decode)]
pub struct MockIdentifier {
    number: u32,
    hash: u32,
}

impl MockIdentifier {
    fn new(number: u32, hash: u32) -> Self {
        MockIdentifier { number, hash }
    }

    pub fn new_random(number: u32) -> Self {
        MockIdentifier::new(number, rand::random())
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

    pub fn random_parentless(number: u32) -> Self {
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
}

impl MockJustification {
    pub fn for_header(header: MockHeader) -> Self {
        MockJustification { header }
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
