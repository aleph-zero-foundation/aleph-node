use std::{
    collections::HashSet,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use lru::LruCache;

use crate::{
    block::{Header, Justification},
    sync::PeerId,
    BlockId,
};

const MAX_KNOW_MOST: usize = 200;
const EXPECTED_MAX_IMPORT_TIME: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
struct Importing {
    since: Instant,
}

impl Importing {
    /// A new importing state that started just now.
    pub fn new() -> Self {
        Importing {
            since: Instant::now(),
        }
    }

    /// We consider the block optimistically imported, unless it's taking suspiciously long
    /// to actually finalize the import.
    pub fn imported(&self) -> bool {
        Instant::now().duration_since(self.since) <= EXPECTED_MAX_IMPORT_TIME
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
enum Importance {
    Auxiliary,
    Required,
    ExplicitlyRequired,
}

impl Importance {
    /// We want to import all required blocks.
    pub fn importable(&self) -> bool {
        use Importance::*;
        matches!(self, Required | ExplicitlyRequired)
    }

    /// We want to request only explicitly required blocks.
    pub fn requestable(&self) -> bool {
        use Importance::*;
        matches!(self, ExplicitlyRequired)
    }

    /// Set the importance to be explicitly required, returns whether anything changed.
    pub fn set_explicitly_required(&mut self) -> bool {
        use Importance::*;
        match self {
            ExplicitlyRequired => false,
            _ => {
                *self = ExplicitlyRequired;
                true
            }
        }
    }

    /// Set the importance to be required, returns whether anything changed.
    pub fn set_required(&mut self) -> bool {
        use Importance::*;
        match self {
            Auxiliary => {
                *self = Required;
                true
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
enum HeaderImportance {
    Imported,
    Importing(Importance, Importing),
    Unimported(Importance),
}

impl HeaderImportance {
    /// Whether the related block should be imported.
    pub fn importable(&self) -> bool {
        use HeaderImportance::*;
        match self {
            Imported => false,
            Importing(importance, importing) => importance.importable() && !importing.imported(),
            Unimported(importance) => importance.importable(),
        }
    }

    /// Whether the related block should be actively requested.
    pub fn requestable(&self) -> bool {
        use HeaderImportance::*;
        match self {
            Imported => false,
            Importing(importance, importing) => importance.requestable() && !importing.imported(),
            Unimported(importance) => importance.requestable(),
        }
    }

    /// Whether we consider the related block to be imported.
    pub fn imported(&self) -> bool {
        use HeaderImportance::*;
        match self {
            Imported => true,
            Importing(_, importing) => importing.imported(),
            Unimported(_) => false,
        }
    }

    /// Mark the start of an import.
    pub fn start_import(&mut self) {
        match self {
            HeaderImportance::Imported => (),
            HeaderImportance::Unimported(importance)
            | HeaderImportance::Importing(importance, _) => {
                *self = HeaderImportance::Importing(*importance, Importing::new())
            }
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
enum JustificationImportance {
    Imported,
    Importing(Importing),
    Unimported,
}

impl JustificationImportance {
    /// Whether we consider the related block to be imported.
    pub fn imported(&self) -> bool {
        use JustificationImportance::*;
        match self {
            Imported => true,
            Importing(importing) => importing.imported(),
            Unimported => false,
        }
    }

    /// Mark the start of an import.
    pub fn start_import(&mut self) {
        match self {
            JustificationImportance::Imported => (),
            JustificationImportance::Unimported | JustificationImportance::Importing(_) => {
                *self = JustificationImportance::Importing(Importing::new())
            }
        }
    }
}

impl From<HeaderImportance> for JustificationImportance {
    fn from(importance: HeaderImportance) -> Self {
        use JustificationImportance::*;
        match importance {
            HeaderImportance::Imported => Imported,
            HeaderImportance::Importing(_, importing) => Importing(importing),
            HeaderImportance::Unimported(_) => Unimported,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InnerVertex<J: Justification> {
    /// Empty Vertex.
    Empty { required: Importance },
    /// Vertex with added Header.
    Header {
        importance: HeaderImportance,
        header: J::Header,
    },
    /// Vertex with added Header and Justification.
    Justification {
        importance: JustificationImportance,
        justification: J,
        parent: BlockId,
    },
}

/// The complete vertex, including metadata about peers that know most about the data it refers to.
#[derive(Debug)]
pub struct Vertex<I: PeerId, J: Justification> {
    inner: InnerVertex<J>,
    know_most: LruCache<I, ()>,
}

impl<I: PeerId, J: Justification> Vertex<I, J> {
    /// Create a new empty vertex.
    pub fn new() -> Self {
        Vertex {
            inner: InnerVertex::Empty {
                required: Importance::Auxiliary,
            },
            know_most: LruCache::new(
                NonZeroUsize::new(MAX_KNOW_MOST).expect("the constant is not zero"),
            ),
        }
    }

    /// Whether we want the referenced block in our database.
    pub fn importable(&self) -> bool {
        use Importance::*;
        use InnerVertex::*;
        match self.inner {
            Empty {
                required: Required | ExplicitlyRequired,
            } => true,
            Header { importance, .. } => importance.importable(),
            Justification { importance, .. } => !importance.imported(),
            _ => false,
        }
    }

    /// Whether the referenced block should be requested.
    /// This ignores blocks requested due to justifications, as these will be requested separately.
    pub fn requestable(&self) -> bool {
        use Importance::*;
        use InnerVertex::*;
        match self.inner {
            Empty {
                required: ExplicitlyRequired,
            } => true,
            Header { importance, .. } => importance.requestable(),
            _ => false,
        }
    }

    /// Whether the vertex is imported.
    pub fn imported(&self) -> bool {
        use InnerVertex::*;
        match self.inner {
            Justification { importance, .. } => importance.imported(),
            Header { importance, .. } => importance.imported(),
            _ => false,
        }
    }

    /// Whether the vertex is currently importing.
    pub fn importing(&self) -> bool {
        use InnerVertex::*;
        match self.inner {
            Header {
                importance: HeaderImportance::Importing(_, importing),
                ..
            }
            | Justification {
                importance: JustificationImportance::Importing(importing),
                ..
            } => importing.imported(),
            _ => false,
        }
    }

    /// Whether the vertex represents imported justified block.
    /// Note that we want fully imported blocks, not just optimistically considered to be imported.
    pub fn justified_block(&self) -> bool {
        matches!(
            self.inner,
            InnerVertex::Justification {
                importance: JustificationImportance::Imported,
                ..
            }
        )
    }

    /// Deconstructs the vertex into a justification if it is ready to be imported,
    /// i.e. the related block has already been imported, otherwise returns it.
    pub fn ready(self) -> Result<J, Self> {
        match self.inner {
            InnerVertex::Justification {
                importance: JustificationImportance::Imported,
                justification,
                ..
            } => Ok(justification),
            _ => Err(self),
        }
    }

    /// The parent of the vertex, if known.
    pub fn parent(&self) -> Option<BlockId> {
        match &self.inner {
            InnerVertex::Empty { .. } => None,
            InnerVertex::Header { header, .. } => header.parent_id(),
            InnerVertex::Justification { parent, .. } => Some(parent.clone()),
        }
    }

    /// The header of the vertex, if known.
    pub fn header(&self) -> Option<J::Header> {
        match &self.inner {
            InnerVertex::Empty { .. } => None,
            InnerVertex::Header { header, .. } => Some(header.clone()),
            InnerVertex::Justification { justification, .. } => {
                Some(justification.header().clone())
            }
        }
    }

    /// The list of peers which know most about the data this vertex refers to.
    pub fn know_most(&self) -> HashSet<I> {
        self.know_most
            .iter()
            .map(|(peer, ())| peer)
            .cloned()
            .collect()
    }

    /// Set the vertex to be explicitly required, returns whether anything changed, i.e. the vertex
    /// was not explicitly required or imported before.
    pub fn set_explicitly_required(&mut self) -> bool {
        use HeaderImportance::*;
        use InnerVertex::*;
        match &mut self.inner {
            Empty { required }
            | Header {
                importance: Unimported(required) | Importing(required, _),
                ..
            } => required.set_explicitly_required(),
            _ => false,
        }
    }

    /// Set the vertex to be required, returns whether anything changed, i.e. the vertex was not
    /// required or imported before.
    pub fn set_required(&mut self) -> bool {
        use HeaderImportance::*;
        use InnerVertex::*;
        match &mut self.inner {
            Empty { required }
            | Header {
                importance: Unimported(required) | Importing(required, _),
                ..
            } => required.set_required(),
            _ => false,
        }
    }

    /// Mark the start of the related block being imported, if possible.
    pub fn start_import(&mut self) {
        use InnerVertex::*;
        match &mut self.inner {
            Header { importance, .. } => importance.start_import(),
            Justification { importance, .. } => importance.start_import(),
            _ => (),
        }
    }

    /// Adds a peer that knows most about the block this vertex refers to. Does nothing if we
    /// already have a justification.
    pub fn add_block_holder(&mut self, holder: Option<I>) {
        if let Some(holder) = holder {
            if !matches!(self.inner, InnerVertex::Justification { .. }) {
                self.know_most.put(holder, ());
            }
        }
    }

    /// Adds the information the header provides to the vertex.
    /// Returns whether this is a new header.
    pub fn insert_header(&mut self, header: J::Header, holder: Option<I>) -> bool {
        self.add_block_holder(holder);
        match self.inner {
            InnerVertex::Empty { required } => {
                self.inner = InnerVertex::Header {
                    importance: HeaderImportance::Unimported(required),
                    header,
                };
                true
            }
            _ => false,
        }
    }

    /// Adds the information the header provides to the vertex and marks it as imported. Returns
    /// whether it was not imported before.
    pub fn insert_body(&mut self, header: J::Header) -> bool {
        use InnerVertex::*;
        match &self.inner {
            Empty { .. }
            | Header {
                importance: HeaderImportance::Unimported(_) | HeaderImportance::Importing(_, _),
                ..
            } => {
                self.inner = Header {
                    header,
                    importance: HeaderImportance::Imported,
                };
                true
            }
            Justification {
                importance:
                    JustificationImportance::Unimported | JustificationImportance::Importing(_),
                parent,
                justification,
            } => {
                self.inner = Justification {
                    importance: JustificationImportance::Imported,
                    parent: parent.clone(),
                    justification: justification.clone(),
                };
                true
            }
            _ => false,
        }
    }

    /// Adds a justification to the vertex.
    pub fn insert_justification(&mut self, parent: BlockId, justification: J, holder: Option<I>) {
        use InnerVertex::*;
        match self.inner {
            Empty { .. } => {
                self.inner = Justification {
                    importance: JustificationImportance::Unimported,
                    parent,
                    justification,
                };
                self.know_most.clear();
                if let Some(peer) = holder {
                    self.know_most.put(peer, ());
                }
            }
            Header { importance, .. } => {
                self.inner = Justification {
                    importance: importance.into(),
                    parent,
                    justification,
                };
                self.know_most.clear();
                if let Some(peer) = holder {
                    self.know_most.put(peer, ());
                }
            }
            Justification { .. } => {
                if let Some(holder) = holder {
                    self.know_most.put(holder, ());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::{sleep, Duration};

    use super::Vertex;
    use crate::{
        block::{
            mock::{MockHeader, MockJustification},
            Header,
        },
        sync::MockPeerId,
        BlockId,
    };

    type MockVertex = Vertex<MockPeerId, MockJustification>;

    #[test]
    fn initially_empty() {
        let vertex = MockVertex::new();
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
        assert!(!vertex.imported());
        assert!(vertex.parent().is_none());
        assert!(vertex.know_most().is_empty());
        assert!(vertex.ready().is_err());
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
        assert!(vertex.importable());
        assert!(!vertex.requestable());
        assert!(!vertex.set_required());
        assert!(vertex.importable());
        assert!(!vertex.requestable());
    }

    #[test]
    fn empty_set_explicitly_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_explicitly_required());
        assert!(vertex.importable());
        assert!(vertex.requestable());
        assert!(!vertex.set_explicitly_required());
        assert!(vertex.importable());
        assert!(vertex.requestable());
    }

    #[test]
    fn empty_set_required_then_explicitly_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_required());
        assert!(vertex.importable());
        assert!(!vertex.requestable());
        assert!(vertex.set_explicitly_required());
        assert!(vertex.importable());
        assert!(vertex.requestable());
    }

    #[test]
    fn empty_set_explicitly_required_then_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_explicitly_required());
        assert!(vertex.importable());
        assert!(vertex.requestable());
        assert!(!vertex.set_required());
        assert!(vertex.importable());
        assert!(vertex.requestable());
    }

    #[test]
    fn empty_to_header() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        vertex.insert_header(header.clone(), Some(peer_id));
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
        assert!(!vertex.imported());
        assert_eq!(vertex.parent(), Some(parent));
        assert_eq!(vertex.header(), Some(header));
        assert!(vertex.know_most().contains(&peer_id));
        assert!(vertex.ready().is_err());
    }

    #[test]
    fn header_remembers_block_holders() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        vertex.insert_header(header, Some(peer_id));
        let other_peer_id = rand::random();
        vertex.add_block_holder(Some(other_peer_id));
        assert!(vertex.know_most().contains(&peer_id));
        assert!(vertex.know_most().contains(&other_peer_id));
    }

    #[test]
    fn header_set_required() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        vertex.insert_header(header, Some(peer_id));
        assert!(vertex.set_required());
        assert!(vertex.importable());
        assert!(!vertex.set_required());
        assert!(vertex.importable());
    }

    #[test]
    fn header_set_explicitly_required() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        vertex.insert_header(header, Some(peer_id));
        assert!(vertex.set_explicitly_required());
        assert!(vertex.importable());
        assert!(vertex.requestable());
        assert!(!vertex.set_explicitly_required());
        assert!(vertex.importable());
        assert!(vertex.requestable());
    }

    #[test]
    fn header_still_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_required());
        let peer_id = rand::random();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        vertex.insert_header(header, Some(peer_id));
        assert!(vertex.importable());
        assert!(!vertex.set_required());
        assert!(vertex.importable());
    }

    #[test]
    fn header_still_explicitly_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_explicitly_required());
        let peer_id = rand::random();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        vertex.insert_header(header, Some(peer_id));
        assert!(vertex.importable());
        assert!(vertex.requestable());
        assert!(!vertex.set_explicitly_required());
        assert!(vertex.importable());
        assert!(vertex.requestable());
    }

    #[test]
    fn empty_to_body() {
        let mut vertex = MockVertex::new();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        assert!(vertex.insert_body(header.clone()));
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
        assert!(vertex.imported());
        assert_eq!(vertex.parent(), Some(parent));
        assert_eq!(vertex.header(), Some(header));
        assert!(vertex.ready().is_err());
    }

    #[test]
    fn body_twice() {
        let mut vertex = MockVertex::new();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        assert!(vertex.insert_body(header.clone()));
        assert!(!vertex.insert_body(header.clone()));
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
        assert!(vertex.imported());
        assert_eq!(vertex.parent(), Some(parent));
        assert_eq!(vertex.header(), Some(header));
        assert!(vertex.ready().is_err());
    }

    #[test]
    fn header_to_body() {
        let mut vertex = MockVertex::new();
        let peer_id = rand::random();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        vertex.insert_header(header.clone(), Some(peer_id));
        assert!(vertex.insert_body(header.clone()));
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
        assert!(vertex.imported());
        assert_eq!(vertex.parent(), Some(parent));
        assert_eq!(vertex.header(), Some(header));
        assert!(vertex.ready().is_err());
    }

    #[test]
    fn body_set_required() {
        let mut vertex = MockVertex::new();
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        assert!(vertex.insert_body(header));
        assert!(!vertex.set_required());
        assert!(!vertex.importable());
        assert!(!vertex.set_explicitly_required());
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
    }

    #[test]
    fn body_no_longer_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_required());
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        assert!(vertex.insert_body(header));
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
    }

    #[test]
    fn body_no_longer_explicitly_required() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_explicitly_required());
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        assert!(vertex.insert_body(header));
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
    }

    #[test]
    fn importing_considered_imported() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_explicitly_required());
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        vertex.insert_header(header, None);
        assert!(vertex.importable());
        assert!(vertex.requestable());
        assert!(!vertex.importing());
        vertex.start_import();
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
        assert!(vertex.importing());
    }

    #[tokio::test]
    async fn importing_fails_after_delay() {
        let mut vertex = MockVertex::new();
        assert!(vertex.set_explicitly_required());
        let parent = BlockId::new_random(43);
        let header = parent.random_child();
        vertex.insert_header(header, None);
        assert!(vertex.importable());
        assert!(vertex.requestable());
        assert!(!vertex.importing());
        vertex.start_import();
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
        assert!(vertex.importing());
        sleep(Duration::from_secs(6)).await;
        assert!(vertex.importable());
        assert!(vertex.requestable());
        assert!(!vertex.importing());
    }

    #[test]
    fn empty_to_justification() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        let peer_id = rand::random();
        vertex.insert_justification(parent.clone(), justification, Some(peer_id));
        assert!(vertex.importable());
        assert!(!vertex.requestable());
        assert!(!vertex.imported());
        assert_eq!(vertex.parent(), Some(parent));
        assert!(vertex.know_most().contains(&peer_id));
        assert!(vertex.ready().is_err());
    }

    #[test]
    fn header_to_justification() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header.clone());
        let peer_id = rand::random();
        vertex.insert_header(header, Some(peer_id));
        vertex.insert_justification(parent.clone(), justification, None);
        assert!(vertex.importable());
        assert!(!vertex.requestable());
        assert!(!vertex.imported());
        assert_eq!(vertex.parent(), Some(parent));
        assert!(vertex.know_most().is_empty());
        assert!(vertex.ready().is_err());
    }

    #[test]
    fn body_to_justification() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header.clone());
        assert!(vertex.insert_body(header));
        vertex.insert_justification(parent.clone(), justification.clone(), None);
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
        assert!(vertex.imported());
        assert_eq!(vertex.parent(), Some(parent));
        assert_eq!(vertex.ready().expect("should be ready"), justification);
    }

    #[test]
    fn justification_set_required() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        let peer_id = rand::random();
        vertex.insert_justification(parent, justification, Some(peer_id));
        assert!(!vertex.set_required());
        assert!(vertex.importable());
        assert!(!vertex.requestable());
    }

    #[test]
    fn justification_set_explicitly_required() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        let peer_id = rand::random();
        vertex.insert_justification(parent, justification, Some(peer_id));
        assert!(!vertex.set_explicitly_required());
        assert!(vertex.importable());
        assert!(!vertex.requestable());
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
        vertex.insert_justification(parent, justification, Some(peer_id));
        assert!(vertex.importable());
        assert!(!vertex.requestable());
    }

    #[test]
    fn justification_no_longer_explicitly_required() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header);
        let peer_id = rand::random();
        assert!(vertex.set_explicitly_required());
        vertex.insert_justification(parent, justification, Some(peer_id));
        assert!(vertex.importable());
        assert!(!vertex.requestable());
    }

    #[test]
    fn my_body_is_ready() {
        let mut vertex = MockVertex::new();
        let parent_header = MockHeader::random_parentless(0);
        let header = parent_header.random_child();
        let parent = header.parent_id().expect("born of a parent");
        let justification = MockJustification::for_header(header.clone());
        vertex.insert_justification(parent.clone(), justification.clone(), None);
        assert!(vertex.insert_body(header));
        assert!(!vertex.importable());
        assert!(!vertex.requestable());
        assert!(vertex.imported());
        assert_eq!(vertex.parent(), Some(parent));
        assert_eq!(vertex.ready().expect("should be ready"), justification);
    }
}
