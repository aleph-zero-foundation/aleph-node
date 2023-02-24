use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use aleph_primitives::{AlephSessionApi, SessionAuthorityData};
use futures::StreamExt;
use log::{debug, error, trace};
use sc_client_api::{Backend, FinalityNotification};
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_runtime::{
    generic::BlockId,
    traits::{Block, Header, NumberFor},
};
use tokio::sync::{
    oneshot::{Receiver as OneShotReceiver, Sender as OneShotSender},
    RwLock,
};

use crate::{
    first_block_of_session, session_id_from_block_num, ClientForAleph, SessionId, SessionPeriod,
};

const PRUNING_THRESHOLD: u32 = 10;
type SessionMap = HashMap<SessionId, SessionAuthorityData>;
type SessionSubscribers = HashMap<SessionId, Vec<OneShotSender<SessionAuthorityData>>>;

pub trait AuthorityProvider<B> {
    /// returns authority data for block
    fn authority_data(&self, block: B) -> Option<SessionAuthorityData>;
    /// returns next session authority data where current session is for block
    fn next_authority_data(&self, block: B) -> Option<SessionAuthorityData>;
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

impl<C, B, BE> AuthorityProvider<NumberFor<B>> for AuthorityProviderImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    B: Block,
    BE: Backend<B> + 'static,
{
    fn authority_data(&self, num: NumberFor<B>) -> Option<SessionAuthorityData> {
        match self
            .client
            .runtime_api()
            .authority_data(&BlockId::Number(num))
        {
            Ok(data) => Some(data),
            Err(_) => self
                .client
                .runtime_api()
                .authorities(&BlockId::Number(num))
                .map(|authorities| SessionAuthorityData::new(authorities, None))
                .ok(),
        }
    }

    fn next_authority_data(&self, num: NumberFor<B>) -> Option<SessionAuthorityData> {
        match self
            .client
            .runtime_api()
            .next_session_authority_data(&BlockId::Number(num))
            .map(|r| r.ok())
        {
            Ok(maybe_data) => maybe_data,
            Err(_) => self
                .client
                .runtime_api()
                .next_session_authorities(&BlockId::Number(num))
                .map(|r| {
                    r.map(|authorities| SessionAuthorityData::new(authorities, None))
                        .ok()
                })
                .ok()
                .flatten(),
        }
    }
}

pub trait FinalityNotificator<B, N> {
    fn notification_stream(&mut self) -> TracingUnboundedReceiver<B>;
    fn last_finalized(&self) -> N;
}

/// Default implementation of finality notificator trait.
pub struct FinalityNotificatorImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    B: Block,
    BE: Backend<B> + 'static,
{
    client: Arc<C>,
    _phantom: PhantomData<(B, BE)>,
}

impl<C, B, BE> FinalityNotificatorImpl<C, B, BE>
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

impl<C, B, BE> FinalityNotificator<FinalityNotification<B>, NumberFor<B>>
    for FinalityNotificatorImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    B: Block,
    BE: Backend<B> + 'static,
{
    fn notification_stream(&mut self) -> TracingUnboundedReceiver<FinalityNotification<B>> {
        self.client.finality_notification_stream()
    }

    fn last_finalized(&self) -> NumberFor<B> {
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
    pub async fn get(&self, id: SessionId) -> Option<SessionAuthorityData> {
        self.inner.read().await.0.get(&id).cloned()
    }

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
pub struct SessionMapUpdater<AP, FN, B>
where
    AP: AuthorityProvider<NumberFor<B>>,
    FN: FinalityNotificator<FinalityNotification<B>, NumberFor<B>>,
    B: Block,
{
    session_map: SharedSessionMap,
    authority_provider: AP,
    finality_notificator: FN,
    period: SessionPeriod,
    _phantom: PhantomData<B>,
}

impl<AP, FN, B> SessionMapUpdater<AP, FN, B>
where
    AP: AuthorityProvider<NumberFor<B>>,
    FN: FinalityNotificator<FinalityNotification<B>, NumberFor<B>>,
    B: Block,
{
    pub fn new(authority_provider: AP, finality_notificator: FN, period: SessionPeriod) -> Self {
        Self {
            session_map: SharedSessionMap::new(),
            authority_provider,
            finality_notificator,
            period,
            _phantom: PhantomData,
        }
    }

    /// returns readonly view of the session map
    pub fn readonly_session_map(&self) -> ReadOnlySessionMap {
        self.session_map.read_only()
    }

    /// Puts authority data for the next session into the session map
    async fn handle_first_block_of_session(&mut self, session_id: SessionId) {
        let first_block = first_block_of_session(session_id, self.period);
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
        let first_block = first_block_of_session(session_id, self.period);
        self.authority_provider.authority_data(first_block)
    }

    /// Puts current and next session authorities in the session map.
    /// If previous authorities are still available in `AuthorityProvider`, also puts them in the session map.
    async fn catch_up(&mut self) -> SessionId {
        let last_finalized = self.finality_notificator.last_finalized();

        let current_session = session_id_from_block_num(last_finalized, self.period);
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
        let mut notifications = self.finality_notificator.notification_stream();
        let mut last_updated = self.catch_up().await;

        while let Some(FinalityNotification { header, .. }) = notifications.next().await {
            let last_finalized = header.number();
            trace!(target: "aleph-session-updater", "got FinalityNotification about #{:?}", last_finalized);

            let session_id = session_id_from_block_num(*last_finalized, self.period);

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
    use std::{sync::Mutex, time::Duration};

    use futures_timer::Delay;
    use sc_block_builder::BlockBuilderProvider;
    use sc_client_api::FinalizeSummary;
    use sc_utils::mpsc::tracing_unbounded;
    use sp_consensus::BlockOrigin;
    use substrate_test_runtime_client::{
        ClientBlockImportExt, DefaultTestClientBuilderExt, TestClient, TestClientBuilder,
        TestClientBuilderExt,
    };
    use tokio::sync::oneshot::error::TryRecvError;

    use super::*;
    use crate::{session::testing::authority_data, testing::mocks::TBlock};

    struct MockProvider {
        pub session_map: HashMap<NumberFor<TBlock>, SessionAuthorityData>,
        pub next_session_map: HashMap<NumberFor<TBlock>, SessionAuthorityData>,
    }

    struct MockNotificator {
        pub last_finalized: NumberFor<TBlock>,
        pub receiver: Mutex<Option<TracingUnboundedReceiver<FinalityNotification<TBlock>>>>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                session_map: HashMap::new(),
                next_session_map: HashMap::new(),
            }
        }

        fn add_session(&mut self, session_id: u64) {
            self.session_map
                .insert(session_id, authority_data_for_session(session_id));
            self.next_session_map
                .insert(session_id, authority_data_for_session(session_id + 1));
        }
    }

    impl MockNotificator {
        fn new(receiver: TracingUnboundedReceiver<FinalityNotification<TBlock>>) -> Self {
            Self {
                receiver: std::sync::Mutex::new(Some(receiver)),
                last_finalized: 0,
            }
        }
    }

    impl AuthorityProvider<NumberFor<TBlock>> for MockProvider {
        fn authority_data(&self, b: NumberFor<TBlock>) -> Option<SessionAuthorityData> {
            self.session_map.get(&b).cloned()
        }

        fn next_authority_data(&self, b: NumberFor<TBlock>) -> Option<SessionAuthorityData> {
            self.next_session_map.get(&b).cloned()
        }
    }

    impl FinalityNotificator<FinalityNotification<TBlock>, NumberFor<TBlock>> for MockNotificator {
        fn notification_stream(
            &mut self,
        ) -> TracingUnboundedReceiver<FinalityNotification<TBlock>> {
            self.receiver.get_mut().unwrap().take().unwrap()
        }

        fn last_finalized(&self) -> NumberFor<TBlock> {
            self.last_finalized
        }
    }

    fn n_new_blocks(client: &mut Arc<TestClient>, n: u64) -> Vec<TBlock> {
        (0..n)
            .map(|_| {
                let block = client
                    .new_block(Default::default())
                    .unwrap()
                    .build()
                    .unwrap()
                    .block;

                futures::executor::block_on(client.import(BlockOrigin::Own, block.clone()))
                    .unwrap();
                block
            })
            .collect()
    }

    fn authority_data_for_session(session_id: u64) -> SessionAuthorityData {
        authority_data(session_id * 4, (session_id + 1) * 4)
    }

    fn to_notification(block: TBlock) -> FinalityNotification<TBlock> {
        let (sender, _) = tracing_unbounded("test", 1);
        let summary = FinalizeSummary {
            header: block.header,
            finalized: vec![],
            stale_heads: vec![],
        };

        FinalityNotification::from_summary(summary, sender)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn genesis_catch_up() {
        let (_sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mock_notificator = MockNotificator::new(receiver);

        mock_provider.add_session(0);

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator, SessionPeriod(1));
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
        let mut client = Arc::new(TestClientBuilder::new().build());
        let (sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mock_notificator = MockNotificator::new(receiver);

        mock_provider.add_session(0);
        mock_provider.add_session(1);
        mock_provider.add_session(2);

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator, SessionPeriod(1));
        let session_map = updater.readonly_session_map();

        for block in n_new_blocks(&mut client, 2) {
            sender.unbounded_send(to_notification(block)).unwrap();
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
        let mut mock_notificator = MockNotificator::new(receiver);

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
        let mut mock_notificator = MockNotificator::new(receiver);

        for i in 0..=2 * PRUNING_THRESHOLD {
            mock_provider.add_session(i as u64);
        }

        mock_notificator.last_finalized = 20;

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator, SessionPeriod(1));
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run());

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        for i in 0..=PRUNING_THRESHOLD {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                None,
                "Session {:?} should be pruned",
                i
            );
        }
        for i in PRUNING_THRESHOLD + 1..=2 * PRUNING_THRESHOLD {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                Some(authority_data_for_session(i as u64)),
                "Session {:?} should not be pruned",
                i
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deals_with_database_pruned_authorities() {
        let (_sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mut mock_notificator = MockNotificator::new(receiver);

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
        let mut client = Arc::new(TestClientBuilder::new().build());
        let (sender, receiver) = tracing_unbounded("test", 1_000);
        let mut mock_provider = MockProvider::new();
        let mock_notificator = MockNotificator::new(receiver);

        for i in 0..=2 * PRUNING_THRESHOLD {
            mock_provider.add_session(i as u64);
        }

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator, SessionPeriod(1));
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run());

        let mut blocks = n_new_blocks(&mut client, 2 * PRUNING_THRESHOLD as u64);

        for block in blocks.drain(..PRUNING_THRESHOLD as usize) {
            sender.unbounded_send(to_notification(block)).unwrap();
        }

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        for i in 0..=PRUNING_THRESHOLD + 1 {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                Some(authority_data_for_session(i as u64)),
                "Session {:?} should be available",
                i
            );
        }

        for i in PRUNING_THRESHOLD + 2..=21 {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                None,
                "Session {:?} should not be avalable yet",
                i
            );
        }

        for block in blocks {
            sender.unbounded_send(to_notification(block)).unwrap();
        }

        Delay::new(Duration::from_millis(50)).await;

        for i in 0..PRUNING_THRESHOLD {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                None,
                "Session {:?} should be pruned",
                i
            );
        }

        for i in PRUNING_THRESHOLD + 1..=21 {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                Some(authority_data_for_session(i as u64)),
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
