use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use aleph_primitives::{AlephSessionApi, BlockNumber, SessionAuthorityData};
use futures::StreamExt;
use log::{debug, error, trace};
use sc_client_api::{Backend, FinalityNotification};
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_runtime::{
    generic::BlockId,
    traits::{Block, Header},
};
use tokio::sync::{
    oneshot::{Receiver as OneShotReceiver, Sender as OneShotSender},
    RwLock,
};

use crate::{session::SessionBoundaryInfo, ClientForAleph, SessionId, SessionPeriod};

const PRUNING_THRESHOLD: u32 = 10;
type SessionMap = HashMap<SessionId, SessionAuthorityData>;
type SessionSubscribers = HashMap<SessionId, Vec<OneShotSender<SessionAuthorityData>>>;

pub trait AuthorityProvider {
    /// returns authority data for block
    fn authority_data(&self, block_number: BlockNumber) -> Option<SessionAuthorityData>;
    /// returns next session authority data where current session is for block
    fn next_authority_data(&self, block_number: BlockNumber) -> Option<SessionAuthorityData>;
}

/// Default implementation of authority provider trait.
/// If state pruning is on and set to `n`, will no longer be able to
/// answer for `num < finalized_number - n`.
pub struct AuthorityProviderImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    B: Block,
    BE: Backend<B> + 'static,
{
    client: Arc<C>,
    _phantom: PhantomData<(B, BE)>,
}

impl<C, B, BE> AuthorityProviderImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    B: Block,
    BE: Backend<B> + 'static,
{
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _phantom: PhantomData,
        }
    }
}

impl<C, B, BE> AuthorityProvider for AuthorityProviderImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    BE: Backend<B> + 'static,
{
    fn authority_data(&self, block_number: BlockNumber) -> Option<SessionAuthorityData> {
        match self
            .client
            .runtime_api()
            .authority_data(&BlockId::Number(block_number))
        {
            Ok(data) => Some(data),
            Err(_) => self
                .client
                .runtime_api()
                .authorities(&BlockId::Number(block_number))
                .map(|authorities| SessionAuthorityData::new(authorities, None))
                .ok(),
        }
    }

    fn next_authority_data(&self, block_number: BlockNumber) -> Option<SessionAuthorityData> {
        match self
            .client
            .runtime_api()
            .next_session_authority_data(&BlockId::Number(block_number))
            .map(|r| r.ok())
        {
            Ok(maybe_data) => maybe_data,
            Err(_) => self
                .client
                .runtime_api()
                .next_session_authorities(&BlockId::Number(block_number))
                .map(|r| {
                    r.map(|authorities| SessionAuthorityData::new(authorities, None))
                        .ok()
                })
                .ok()
                .flatten(),
        }
    }
}

#[async_trait::async_trait]
pub trait FinalityNotifier {
    async fn next(&mut self) -> Option<BlockNumber>;
    fn last_finalized(&self) -> BlockNumber;
}

/// Default implementation of finality notificator trait.
pub struct FinalityNotifierImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    BE: Backend<B> + 'static,
{
    notification_stream: TracingUnboundedReceiver<FinalityNotification<B>>,
    client: Arc<C>,
    _phantom: PhantomData<(B, BE)>,
}

impl<C, B, BE> FinalityNotifierImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    BE: Backend<B> + 'static,
{
    pub fn new(client: Arc<C>) -> Self {
        Self {
            notification_stream: client.finality_notification_stream(),
            client,
            _phantom: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<C, B, BE> FinalityNotifier for FinalityNotifierImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    BE: Backend<B> + 'static,
{
    async fn next(&mut self) -> Option<BlockNumber> {
        self.notification_stream
            .next()
            .await
            .map(|block| *block.header.number())
    }

    fn last_finalized(&self) -> BlockNumber {
        self.client.info().finalized_number
    }
}

#[derive(Clone, Debug)]
/// Wrapper around Mapping from sessionId to Vec of AuthorityIds allowing mutation
/// and hiding locking details
pub struct SharedSessionMap(Arc<RwLock<(SessionMap, SessionSubscribers)>>);

#[derive(Clone)]
/// Wrapper around Mapping from sessionId to Vec of AuthorityIds allowing only reads
pub struct ReadOnlySessionMap {
    inner: Arc<RwLock<(SessionMap, SessionSubscribers)>>,
}

impl SharedSessionMap {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new((HashMap::new(), HashMap::new()))))
    }

    pub async fn update(
        &mut self,
        id: SessionId,
        authority_data: SessionAuthorityData,
    ) -> Option<SessionAuthorityData> {
        let mut guard = self.0.write().await;

        // notify all subscribers about insertion and remove them from subscription
        if let Some(senders) = guard.1.remove(&id) {
            for sender in senders {
                if let Err(e) = sender.send(authority_data.clone()) {
                    error!(target: "aleph-session-updater", "Error while sending notification: {:?}", e);
                }
            }
        }

        guard.0.insert(id, authority_data)
    }

    async fn prune_below(&mut self, id: SessionId) {
        let mut guard = self.0.write().await;

        guard.0.retain(|&s, _| s >= id);
        guard.1.retain(|&s, _| s >= id);
    }

    pub fn read_only(&self) -> ReadOnlySessionMap {
        ReadOnlySessionMap {
            inner: self.0.clone(),
        }
    }
}

impl ReadOnlySessionMap {
    /// returns an end of the oneshot channel that fires a message if either authority data is already
    /// known for the session with id = `id` or when the data is inserted for this session.
    pub async fn subscribe_to_insertion(
        &self,
        id: SessionId,
    ) -> OneShotReceiver<SessionAuthorityData> {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        let mut guard = self.inner.write().await;

        if let Some(authority_data) = guard.0.get(&id) {
            // if the value is already present notify immediately
            sender
                .send(authority_data.clone())
                .expect("we control both ends");
        } else {
            guard.1.entry(id).or_insert_with(Vec::new).push(sender);
        }

        receiver
    }
}

/// Struct responsible for updating session map
pub struct SessionMapUpdater<AP, FN>
where
    AP: AuthorityProvider,
    FN: FinalityNotifier,
{
    session_map: SharedSessionMap,
    authority_provider: AP,
    finality_notifier: FN,
    session_info: SessionBoundaryInfo,
}

impl<AP, FN> SessionMapUpdater<AP, FN>
where
    AP: AuthorityProvider,
    FN: FinalityNotifier,
{
    pub fn new(authority_provider: AP, finality_notifier: FN, period: SessionPeriod) -> Self {
        Self {
            session_map: SharedSessionMap::new(),
            authority_provider,
            finality_notifier,
            session_info: SessionBoundaryInfo::new(period),
        }
    }

    /// returns readonly view of the session map
    pub fn readonly_session_map(&self) -> ReadOnlySessionMap {
        self.session_map.read_only()
    }

    /// Puts authority data for the next session into the session map
    async fn handle_first_block_of_session(&mut self, session_id: SessionId) {
        let first_block = self.session_info.first_block_of_session(session_id);
        debug!(target: "aleph-session-updater",
            "Handling first block #{:?} of session {:?}",
            first_block, session_id.0
        );

        if let Some(authority_data) = self.authority_provider.next_authority_data(first_block) {
            self.session_map
                .update(SessionId(session_id.0 + 1), authority_data)
                .await;
        } else {
            panic!("Authorities for next session {:?} must be available at first block #{:?} of current session", session_id.0, first_block);
        }

        if session_id.0 > PRUNING_THRESHOLD && session_id.0 % PRUNING_THRESHOLD == 0 {
            debug!(target: "aleph-session-updater",
                "Pruning session map below session #{:?}",
                session_id.0 - PRUNING_THRESHOLD
            );
            self.session_map
                .prune_below(SessionId(session_id.0 - PRUNING_THRESHOLD))
                .await;
        }
    }

    fn authorities_for_session(&mut self, session_id: SessionId) -> Option<SessionAuthorityData> {
        let first_block = self.session_info.first_block_of_session(session_id);
        self.authority_provider.authority_data(first_block)
    }

    /// Puts current and next session authorities in the session map.
    /// If previous authorities are still available in `AuthorityProvider`, also puts them in the session map.
    async fn catch_up(&mut self) -> SessionId {
        let last_finalized = self.finality_notifier.last_finalized();

        let current_session = self.session_info.session_id_from_block_num(last_finalized);
        let starting_session = SessionId(current_session.0.saturating_sub(PRUNING_THRESHOLD - 1));

        debug!(target: "aleph-session-updater",
            "Last finalized is {:?}; Catching up with authorities starting from session {:?} up to next session {:?}",
            last_finalized, starting_session.0, current_session.0 + 1
        );

        // lets catch up with previous sessions
        for session in starting_session.0..current_session.0 {
            let id = SessionId(session);
            if let Some(authority_data) = self.authorities_for_session(id) {
                self.session_map.update(id, authority_data).await;
            } else {
                debug!(target: "aleph-session-updater", "No authorities for session {:?} during catch-up. Most likely already pruned.", id.0)
            }
        }

        // lets catch up with previous session
        match self.authorities_for_session(current_session) {
            Some(current_authority_data) => {
                self.session_map
                    .update(current_session, current_authority_data)
                    .await
            }
            None => panic!(
                "Authorities for current session {:?} must be available from the beginning",
                current_session.0
            ),
        };

        self.handle_first_block_of_session(current_session).await;

        current_session
    }

    pub async fn run(mut self) {
        let mut last_updated = self.catch_up().await;

        while let Some(last_finalized) = self.finality_notifier.next().await {
            trace!(target: "aleph-session-updater", "got FinalityNotification about #{:?}", last_finalized);

            let session_id = self.session_info.session_id_from_block_num(last_finalized);

            if last_updated >= session_id {
                continue;
            }

            for session in (last_updated.0 + 1)..=session_id.0 {
                self.handle_first_block_of_session(SessionId(session)).await;
            }

            last_updated = session_id;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use aleph_primitives::BlockNumber;
    use futures_timer::Delay;
    use sc_utils::mpsc::tracing_unbounded;
    use tokio::sync::oneshot::error::TryRecvError;

    use super::*;
    use crate::session::testing::authority_data;

    const FIRST_THRESHOLD: u32 = PRUNING_THRESHOLD + 1;
    const SECOND_THRESHOLD: u32 = 2 * PRUNING_THRESHOLD + 1;

    impl ReadOnlySessionMap {
        async fn get(&self, id: SessionId) -> Option<SessionAuthorityData> {
            self.inner.read().await.0.get(&id).cloned()
        }
    }

    struct MockProvider {
        pub session_map: HashMap<BlockNumber, SessionAuthorityData>,
        pub next_session_map: HashMap<BlockNumber, SessionAuthorityData>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                session_map: HashMap::new(),
                next_session_map: HashMap::new(),
            }
        }

        fn add_session(&mut self, session_id: BlockNumber) {
            self.session_map
                .insert(session_id, authority_data_for_session(session_id));
            self.next_session_map
                .insert(session_id, authority_data_for_session(session_id + 1));
        }
    }
    impl AuthorityProvider for MockProvider {
        fn authority_data(&self, block_number: BlockNumber) -> Option<SessionAuthorityData> {
            self.session_map.get(&block_number).cloned()
        }

        fn next_authority_data(&self, block_number: BlockNumber) -> Option<SessionAuthorityData> {
            self.next_session_map.get(&block_number).cloned()
        }
    }

    struct MockNotifier {
        pub last_finalized: BlockNumber,
        pub receiver: TracingUnboundedReceiver<BlockNumber>,
    }

    impl MockNotifier {
        fn new(receiver: TracingUnboundedReceiver<BlockNumber>) -> Self {
            Self {
                receiver,
                last_finalized: 0,
            }
        }
    }

    #[async_trait::async_trait]
    impl FinalityNotifier for MockNotifier {
        async fn next(&mut self) -> Option<BlockNumber> {
            self.receiver.next().await
        }

        fn last_finalized(&self) -> BlockNumber {
            self.last_finalized
        }
    }

    fn authority_data_for_session(session_id: u32) -> SessionAuthorityData {
        authority_data(session_id * 4, (session_id + 1) * 4)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn genesis_catch_up() {
        let (_sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mock_notifier = MockNotifier::new(receiver);

        mock_provider.add_session(0);

        let updater = SessionMapUpdater::new(mock_provider, mock_notifier, SessionPeriod(1));
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run());

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        assert_eq!(
            session_map.get(SessionId(0)).await,
            Some(authority_data(0, 4))
        );
        assert_eq!(
            session_map.get(SessionId(1)).await,
            Some(authority_data(4, 8))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn updates_session_map_on_notifications() {
        let (sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mock_notificator = MockNotifier::new(receiver);

        mock_provider.add_session(0);
        mock_provider.add_session(1);
        mock_provider.add_session(2);

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator, SessionPeriod(1));
        let session_map = updater.readonly_session_map();

        for n in 1..3 {
            sender.unbounded_send(n).unwrap();
        }

        let _handle = tokio::spawn(updater.run());

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        assert_eq!(
            session_map.get(SessionId(0)).await,
            Some(authority_data(0, 4))
        );
        assert_eq!(
            session_map.get(SessionId(1)).await,
            Some(authority_data(4, 8))
        );
        assert_eq!(
            session_map.get(SessionId(2)).await,
            Some(authority_data(8, 12))
        );
        assert_eq!(
            session_map.get(SessionId(3)).await,
            Some(authority_data(12, 16))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn catch_up() {
        let (_sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mut mock_notificator = MockNotifier::new(receiver);

        mock_provider.add_session(0);
        mock_provider.add_session(1);
        mock_provider.add_session(2);

        mock_notificator.last_finalized = 2;

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator, SessionPeriod(1));
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run());

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        assert_eq!(
            session_map.get(SessionId(0)).await,
            Some(authority_data_for_session(0))
        );
        assert_eq!(
            session_map.get(SessionId(1)).await,
            Some(authority_data_for_session(1))
        );
        assert_eq!(
            session_map.get(SessionId(2)).await,
            Some(authority_data_for_session(2))
        );
        assert_eq!(
            session_map.get(SessionId(3)).await,
            Some(authority_data_for_session(3))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn catch_up_old_sessions() {
        let (_sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mut mock_notificator = MockNotifier::new(receiver);

        for i in 0..SECOND_THRESHOLD {
            mock_provider.add_session(i);
        }

        mock_notificator.last_finalized = 20;

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator, SessionPeriod(1));
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run());

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        for i in 0..FIRST_THRESHOLD {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                None,
                "Session {:?} should be pruned",
                i
            );
        }
        for i in FIRST_THRESHOLD..SECOND_THRESHOLD {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                Some(authority_data_for_session(i)),
                "Session {:?} should not be pruned",
                i
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deals_with_database_pruned_authorities() {
        let (_sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mut mock_notificator = MockNotifier::new(receiver);

        mock_provider.add_session(5);
        mock_notificator.last_finalized = 5;

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator, SessionPeriod(1));
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run());

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        for i in 0..5 {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                None,
                "Session {:?} should not be available",
                i
            );
        }

        assert_eq!(
            session_map.get(SessionId(5)).await,
            Some(authority_data_for_session(5))
        );
        assert_eq!(
            session_map.get(SessionId(6)).await,
            Some(authority_data_for_session(6))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn prunes_old_sessions() {
        let (sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mock_notificator = MockNotifier::new(receiver);

        for i in 0..SECOND_THRESHOLD {
            mock_provider.add_session(i);
        }

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator, SessionPeriod(1));
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run());

        for n in 1..FIRST_THRESHOLD {
            sender.unbounded_send(n).unwrap();
        }

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        for i in 0..=FIRST_THRESHOLD {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                Some(authority_data_for_session(i)),
                "Session {:?} should be available",
                i
            );
        }

        for i in (FIRST_THRESHOLD + 1)..=SECOND_THRESHOLD {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                None,
                "Session {:?} should not be avalable yet",
                i
            );
        }

        for n in FIRST_THRESHOLD..SECOND_THRESHOLD {
            sender.unbounded_send(n).unwrap();
        }

        Delay::new(Duration::from_millis(50)).await;

        for i in 0..(FIRST_THRESHOLD - 1) {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                None,
                "Session {:?} should be pruned",
                i
            );
        }

        for i in FIRST_THRESHOLD..=SECOND_THRESHOLD {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                Some(authority_data_for_session(i)),
                "Session {:?} should be avalable",
                i
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn subscription_with_already_defined_session_works() {
        let mut shared = SharedSessionMap::new();
        let readonly = shared.read_only();
        let session = SessionId(0);

        shared.update(session, authority_data(0, 2)).await;

        let mut receiver = readonly.subscribe_to_insertion(session).await;

        // we should have this immediately
        assert_eq!(Ok(authority_data(0, 2)), receiver.try_recv());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn notifies_on_insertion() {
        let mut shared = SharedSessionMap::new();
        let readonly = shared.read_only();
        let session = SessionId(0);
        let mut receiver = readonly.subscribe_to_insertion(session).await;

        // does not yet have any value
        assert_eq!(Err(TryRecvError::Empty), receiver.try_recv());
        shared.update(session, authority_data(0, 2)).await;
        assert_eq!(Ok(authority_data(0, 2)), receiver.await);
    }
}
