use std::collections::HashSet;

use crate::sync::{BlockIdFor, Justification, PeerId};

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
enum HeaderImportance {
    Auxiliary,
    Required,
    Imported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InnerVertex<J: Justification> {
    /// Empty Vertex.
    Empty { required: bool },
    /// Vertex with added Header.
    Header {
        importance: HeaderImportance,
        parent: BlockIdFor<J>,
    },
    /// Vertex with added Header and Justification.
    Justification {
        imported: bool,
        justification: J,
        parent: BlockIdFor<J>,
    },
}

/// The complete vertex, including metadata about peers that know most about the data it refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vertex<I: PeerId, J: Justification> {
    inner: InnerVertex<J>,
    know_most: HashSet<I>,
}

/// What can happen when we add a justification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JustificationAddResult {
    Noop,
    Required,
    Finalizable,
}

impl<I: PeerId, J: Justification> Vertex<I, J> {
    /// Create a new empty vertex.
    pub fn new() -> Self {
        Vertex {
            inner: InnerVertex::Empty { required: false },
            know_most: HashSet::new(),
        }
    }

    /// Whether the vertex is required.
    pub fn required(&self) -> bool {
        use InnerVertex::*;
        matches!(
            self.inner,
            Empty { required: true }
                | Header {
                    importance: HeaderImportance::Required,
                    ..
                }
                | Justification {
                    imported: false,
                    ..
                }
        )
    }

    /// Whether the vertex is imported.
    pub fn imported(&self) -> bool {
        use InnerVertex::*;
        matches!(
            self.inner,
            Header {
                importance: HeaderImportance::Imported,
                ..
            } | Justification { imported: true, .. }
        )
    }

    /// Whether the vertex represents imported justified block.
    pub fn justified_block(&self) -> bool {
        matches!(
            self.inner,
            InnerVertex::Justification { imported: true, .. }
        )
    }

    /// Deconstructs the vertex into a justification if it is ready to be imported,
    /// i.e. the related block has already been imported, otherwise returns it.
    pub fn ready(self) -> Result<J, Self> {
        match self.inner {
            InnerVertex::Justification {
                imported: true,
                justification,
                ..
            } => Ok(justification),
            _ => Err(self),
        }
    }

    /// The parent of the vertex, if known.
    pub fn parent(&self) -> Option<&BlockIdFor<J>> {
        match &self.inner {
            InnerVertex::Empty { .. } => None,
            InnerVertex::Header { parent, .. } => Some(parent),
            InnerVertex::Justification { parent, .. } => Some(parent),
        }
    }

    /// The list of peers which know most about the data this vertex refers to.
    pub fn know_most(&self) -> &HashSet<I> {
        &self.know_most
    }

    /// Set the vertex to be required, returns whether anything changed, i.e. the vertex was not
    /// required or imported before.
    pub fn set_required(&mut self) -> bool {
        use InnerVertex::*;
        match &self.inner {
            Empty { required: false } => {
                self.inner = Empty { required: true };
                true
            }
            Header {
                importance: HeaderImportance::Auxiliary,
                parent,
            } => {
                self.inner = Header {
                    importance: HeaderImportance::Required,
                    parent: parent.clone(),
                };
                true
            }
            _ => false,
        }
    }

    /// Adds a peer that knows most about the block this vertex refers to. Does nothing if we
    /// already have a justification.
    pub fn add_block_holder(&mut self, holder: Option<I>) {
        if let Some(holder) = holder {
            if !matches!(self.inner, InnerVertex::Justification { .. }) {
                self.know_most.insert(holder);
            }
        }
    }

    /// Adds the information the header provides to the vertex.
    pub fn insert_header(&mut self, parent: BlockIdFor<J>, holder: Option<I>) {
        self.add_block_holder(holder);
        if let InnerVertex::Empty { required } = self.inner {
            let importance = match required {
                false => HeaderImportance::Auxiliary,
                true => HeaderImportance::Required,
            };
            self.inner = InnerVertex::Header { importance, parent };
        }
    }

    /// Adds the information the header provides to the vertex and marks it as imported. Returns
    /// whether finalization is now possible.
    pub fn insert_body(&mut self, parent: BlockIdFor<J>) -> bool {
        use InnerVertex::*;
        match &self.inner {
            Empty { .. } | Header { .. } => {
                self.inner = Header {
                    parent,
                    importance: HeaderImportance::Imported,
                };
                false
            }
            Justification {
                imported: false,
                parent,
                justification,
            } => {
                self.inner = Justification {
                    imported: true,
                    parent: parent.clone(),
                    justification: justification.clone(),
                };
                true
            }
            _ => false,
        }
    }

    /// Adds a justification to the vertex. Returns whether either the finalization is now possible
    /// or the vertex became required.
    pub fn insert_justification(
        &mut self,
        parent: BlockIdFor<J>,
        justification: J,
        holder: Option<I>,
    ) -> JustificationAddResult {
        use InnerVertex::*;
        match self.inner {
            Justification { .. } => {
                if let Some(holder) = holder {
                    self.know_most.insert(holder);
                }
                JustificationAddResult::Noop
            }
            Empty { required: true }
            | Header {
                importance: HeaderImportance::Required,
                ..
            } => {
                self.inner = Justification {
                    imported: false,
                    parent,
                    justification,
                };
                self.know_most = holder.into_iter().collect();
                JustificationAddResult::Noop
            }
            Empty { required: false }
            | Header {
                importance: HeaderImportance::Auxiliary,
                ..
            } => {
                self.inner = Justification {
                    imported: false,
                    parent,
                    justification,
                };
                self.know_most = holder.into_iter().collect();
                JustificationAddResult::Required
            }
            Header {
                importance: HeaderImportance::Imported,
                ..
            } => {
                self.inner = Justification {
                    imported: true,
                    parent,
                    justification,
                };
                // No need to modify know_most, as we now know everything we need.
                JustificationAddResult::Finalizable
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{JustificationAddResult, Vertex};
    use crate::sync::{
        mock::{MockHeader, MockIdentifier, MockJustification, MockPeerId},
        Header,
    };

    type MockVertex = Vertex<MockPeerId, MockJustification>;

    #[test]
    fn initially_empty() {
        let vertex = MockVertex::new();
        assert!(!vertex.required());
        assert!(!vertex.imported());
        assert!(vertex.parent().is_none());
        assert!(vertex.know_most().is_empty());
        assert_eq!(vertex.clone().ready(), Err(vertex));
    }

    #[test]
    fn empty_remembers_block_holders() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        vertex.add_block_holder(Some(peer_id));
        assert!(vertex.know_most().contains(&peer_id));
    }

    #[test]
    fn empty_set_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_required());
        assert!(vertex.required());
        assert!(!vertex.set_required());
        assert!(vertex.required());
    }

    #[test]
    fn empty_to_header() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        let parent = MockIdentifier::new_random(43);
        vertex.insert_header(parent.clone(), Some(peer_id));
        assert!(!vertex.required());
        assert!(!vertex.imported());
        assert_eq!(vertex.parent(), Some(&parent));
        assert!(vertex.know_most().contains(&peer_id));
        assert_eq!(vertex.clone().ready(), Err(vertex));
    }

    #[test]
    fn header_remembers_block_holders() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        let parent = MockIdentifier::new_random(43);
        vertex.insert_header(parent, Some(peer_id));
        let other_peer_id = rand::random();
        vertex.add_block_holder(Some(other_peer_id));
        assert!(vertex.know_most().contains(&peer_id));
        assert!(vertex.know_most().contains(&other_peer_id));
    }

    #[test]
    fn header_set_required() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        let parent = MockIdentifier::new_random(43);
        vertex.insert_header(parent, Some(peer_id));
        assert!(vertex.set_required());
        assert!(vertex.required());
        assert!(!vertex.set_required());
        assert!(vertex.required());
    }

    #[test]
    fn header_still_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_required());
        let peer_id = rand::random();
        let parent = MockIdentifier::new_random(43);
        vertex.insert_header(parent, Some(peer_id));
        assert!(vertex.required());
        assert!(!vertex.set_required());
        assert!(vertex.required());
    }

    #[test]
    fn empty_to_body() {
        let mut vertex = MockVertex::new();
        let parent = MockIdentifier::new_random(43);
        assert!(!vertex.insert_body(parent.clone()));
        assert!(!vertex.required());
        assert!(vertex.imported());
        assert_eq!(vertex.parent(), Some(&parent));
        assert_eq!(vertex.clone().ready(), Err(vertex));
    }

    #[test]
    fn header_to_body() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        let parent = MockIdentifier::new_random(43);
        vertex.insert_header(parent.clone(), Some(peer_id));
        assert!(!vertex.insert_body(parent.clone()));
        assert!(!vertex.required());
        assert!(vertex.imported());
        assert_eq!(vertex.parent(), Some(&parent));
        assert_eq!(vertex.clone().ready(), Err(vertex));
    }

    #[test]
    fn body_set_required() {
        let mut vertex = MockVertex::new();
        let parent = MockIdentifier::new_random(43);
        assert!(!vertex.insert_body(parent));
        assert!(!vertex.set_required());
        assert!(!vertex.required());
    }

    #[test]
    fn body_no_longer_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_required());
        let parent = MockIdentifier::new_random(43);
        assert!(!vertex.insert_body(parent));
        assert!(!vertex.required());
    }

    #[test]
    fn empty_to_justification() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        let peer_id = rand::random();
        assert_eq!(
            vertex.insert_justification(parent.clone(), justification, Some(peer_id)),
            JustificationAddResult::Required
        );
        assert!(vertex.required());
        assert!(!vertex.imported());
        assert_eq!(vertex.parent(), Some(&parent));
        assert!(vertex.know_most().contains(&peer_id));
        assert_eq!(vertex.clone().ready(), Err(vertex));
    }

    #[test]
    fn header_to_justification() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        let peer_id = rand::random();
        vertex.insert_header(parent.clone(), Some(peer_id));
        assert_eq!(
            vertex.insert_justification(parent.clone(), justification, None),
            JustificationAddResult::Required
        );
        assert!(vertex.required());
        assert!(!vertex.imported());
        assert_eq!(vertex.parent(), Some(&parent));
        assert!(vertex.know_most().is_empty());
        assert_eq!(vertex.clone().ready(), Err(vertex));
    }

    #[test]
    fn body_to_justification() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        assert!(!vertex.insert_body(parent.clone()));
        assert_eq!(
            vertex.insert_justification(parent.clone(), justification.clone(), None),
            JustificationAddResult::Finalizable
        );
        assert!(!vertex.required());
        assert!(vertex.imported());
        assert_eq!(vertex.parent(), Some(&parent));
        assert_eq!(vertex.ready(), Ok(justification));
    }

    #[test]
    fn justification_set_required() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        let peer_id = rand::random();
        assert_eq!(
            vertex.insert_justification(parent, justification, Some(peer_id)),
            JustificationAddResult::Required
        );
        assert!(!vertex.set_required());
        assert!(vertex.required());
    }

    #[test]
    fn justification_still_required() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        let peer_id = rand::random();
        assert!(vertex.set_required());
        assert_eq!(
            vertex.insert_justification(parent, justification, Some(peer_id)),
            JustificationAddResult::Noop
        );
        assert!(vertex.required());
    }

    #[test]
    fn my_body_is_ready() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        assert_eq!(
            vertex.insert_justification(parent.clone(), justification.clone(), None),
            JustificationAddResult::Required
        );
        assert!(vertex.insert_body(parent.clone()));
        assert!(!vertex.required());
        assert!(vertex.imported());
        assert_eq!(vertex.parent(), Some(&parent));
        assert_eq!(vertex.ready(), Ok(justification));
    }
}
