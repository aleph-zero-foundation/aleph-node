use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use aleph_primitives::{AlephSessionApi, SessionAuthorityData};
use futures::StreamExt;
use log::{debug, error, trace};
use sc_client_api::{Backend, FinalityNotification};
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_runtime::{
    generic::BlockId,
    traits::{Block, Header, NumberFor},
    SaturatedConversion,
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

fn get_authority_data_for_session<AP, B>(
    authority_provider: &AP,
    session_id: SessionId,
    first_block: NumberFor<B>,
) -> SessionAuthorityData
where
    B: Block,
    AP: AuthorityProvider<NumberFor<B>>,
{
    if session_id == SessionId(0) {
        authority_provider
            .authority_data(<NumberFor<B>>::saturated_from(0u32))
            .expect("Authorities for the session 0 must be available from the beginning")
    } else {
        authority_provider.next_authority_data(first_block).unwrap_or_else(||
            panic!("Authorities for next session {:?} must be available at first block #{:?} of current session", session_id.0, first_block)
        )
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
    _phantom: PhantomData<B>,
}

impl<AP, FN, B> SessionMapUpdater<AP, FN, B>
where
    AP: AuthorityProvider<NumberFor<B>>,
    FN: FinalityNotificator<FinalityNotification<B>, NumberFor<B>>,
    B: Block,
{
    pub fn new(authority_provider: AP, finality_notificator: FN) -> Self {
        Self {
            session_map: SharedSessionMap::new(),
            authority_provider,
            finality_notificator,
            _phantom: PhantomData,
        }
    }

    /// returns readonly view of the session map
    pub fn readonly_session_map(&self) -> ReadOnlySessionMap {
        self.session_map.read_only()
    }

    /// puts authority data for the next session into the session map
    async fn handle_first_block_of_session(&mut self, num: NumberFor<B>, session_id: SessionId) {
        debug!(target: "aleph-session-updater", "Handling first block #{:?} of session {:?}", num, session_id.0);
        let next_session = SessionId(session_id.0 + 1);
        let authority_provider = &self.authority_provider;
        self.session_map
            .update(
                next_session,
                get_authority_data_for_session::<_, B>(authority_provider, next_session, num),
            )
            .await;

        // if this is the first session we also need to include starting authority data into the map
        if session_id.0 == 0 {
            let authority_provider = &self.authority_provider;
            self.session_map
                .update(
                    session_id,
                    get_authority_data_for_session::<_, B>(authority_provider, session_id, num),
                )
                .await;
        }

        if session_id.0 >= PRUNING_THRESHOLD && session_id.0 % PRUNING_THRESHOLD == 0 {
            debug!(target: "aleph-session-updater", "Pruning session map below session #{:?}", session_id.0 - PRUNING_THRESHOLD);
            self.session_map
                .prune_below(SessionId(session_id.0 - PRUNING_THRESHOLD))
                .await;
        }
    }

    async fn update_session(&mut self, session_id: SessionId, period: SessionPeriod) {
        let first_block = first_block_of_session::<B>(session_id, period);
        self.handle_first_block_of_session(first_block, session_id)
            .await;
    }

    fn catch_up_boundaries(&self, period: SessionPeriod) -> (SessionId, SessionId) {
        let last_finalized = self.finality_notificator.last_finalized();

        let current_session = session_id_from_block_num::<B>(last_finalized, period);
        let starting_session = SessionId(current_session.0.saturating_sub(PRUNING_THRESHOLD));

        (starting_session, current_session)
    }

    pub async fn run(mut self, period: SessionPeriod) {
        let mut notifications = self.finality_notificator.notification_stream();

        let (starting_session, current_session) = self.catch_up_boundaries(period);

        // lets catch up
        for session in starting_session.0..=current_session.0 {
            self.update_session(SessionId(session), period).await;
        }

        let mut last_updated = current_session;

        while let Some(FinalityNotification { header, .. }) = notifications.next().await {
            let last_finalized = header.number();
            trace!(target: "aleph-session-updater", "got FinalityNotification about #{:?}", last_finalized);

            let session_id = session_id_from_block_num::<B>(*last_finalized, period);

            if last_updated >= session_id {
                continue;
            }

            for session in (last_updated.0 + 1)..=session_id.0 {
                self.update_session(SessionId(session), period).await;
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
    use sc_utils::mpsc::tracing_unbounded;
    use sp_consensus::BlockOrigin;
    use sp_runtime::testing::UintAuthorityId;
    use substrate_test_runtime_client::{
        ClientBlockImportExt, DefaultTestClientBuilderExt, TestClient, TestClientBuilder,
        TestClientBuilderExt,
    };
    use tokio::sync::oneshot::error::TryRecvError;

    use super::*;
    use crate::testing::mocks::TBlock;

    struct MockProvider {
        pub session_map: HashMap<NumberFor<TBlock>, SessionAuthorityData>,
        pub next_session_map: HashMap<NumberFor<TBlock>, SessionAuthorityData>,
        pub asked_for: Arc<Mutex<Vec<NumberFor<TBlock>>>>,
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
                asked_for: Arc::new(Mutex::new(Vec::new())),
            }
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
            let mut asked = self.asked_for.lock().unwrap();
            asked.push(b);
            self.session_map.get(&b).cloned()
        }

        fn next_authority_data(&self, b: NumberFor<TBlock>) -> Option<SessionAuthorityData> {
            let mut asked = self.asked_for.lock().unwrap();
            asked.push(b);
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

    fn authority_data(from: u64, to: u64) -> SessionAuthorityData {
        SessionAuthorityData::new(
            (from..to)
                .map(|id| UintAuthorityId(id).to_public_key())
                .collect(),
            None,
        )
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

    #[tokio::test(flavor = "multi_thread")]
    async fn updates_session_map_on_notifications() {
        let mut client = Arc::new(TestClientBuilder::new().build());
        let (sender, receiver) = tracing_unbounded("test");
        let mut mock_provider = MockProvider::new();
        let mock_notificator = MockNotificator::new(receiver);

        mock_provider.session_map.insert(0, authority_data(0, 4));
        mock_provider
            .next_session_map
            .insert(0, authority_data(4, 8));
        mock_provider
            .next_session_map
            .insert(1, authority_data(8, 12));
        mock_provider
            .next_session_map
            .insert(2, authority_data(12, 16));

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator);
        let session_map = updater.readonly_session_map();

        let blocks = n_new_blocks(&mut client, 2);
        let block_1 = blocks.get(0).cloned().unwrap();
        let block_2 = blocks.get(1).cloned().unwrap();
        sender
            .unbounded_send(FinalityNotification {
                hash: block_1.header.hash(),
                header: block_1.header,
                tree_route: Arc::new([]),
                stale_heads: Arc::new([]),
            })
            .unwrap();
        sender
            .unbounded_send(FinalityNotification {
                hash: block_2.header.hash(),
                header: block_2.header,
                tree_route: Arc::new([]),
                stale_heads: Arc::new([]),
            })
            .unwrap();

        let _handle = tokio::spawn(updater.run(SessionPeriod(1)));

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
    async fn updates_session_map_on_catching_up() {
        let (_sender, receiver) = tracing_unbounded("test");
        let mut mock_provider = MockProvider::new();
        let mut mock_notificator = MockNotificator::new(receiver);

        mock_provider.session_map.insert(0, authority_data(0, 4));
        mock_provider
            .next_session_map
            .insert(0, authority_data(4, 8));
        mock_provider
            .next_session_map
            .insert(1, authority_data(8, 12));
        mock_provider
            .next_session_map
            .insert(2, authority_data(12, 16));

        mock_notificator.last_finalized = 2;

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator);
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run(SessionPeriod(1)));

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
    async fn prunes_old_sessions() {
        let (_sender, receiver) = tracing_unbounded("test");
        let mut mock_provider = MockProvider::new();
        let mut mock_notificator = MockNotificator::new(receiver);

        mock_provider.session_map.insert(0, authority_data(0, 4));
        for i in 0..=2 * PRUNING_THRESHOLD {
            mock_provider.next_session_map.insert(
                i as u64,
                authority_data(4 * (i + 1) as u64, 4 * (i + 2) as u64),
            );
        }

        mock_notificator.last_finalized = 20;

        let asked = mock_provider.asked_for.clone();
        let updater = SessionMapUpdater::new(mock_provider, mock_notificator);
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run(SessionPeriod(1)));

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        {
            let asked = asked.lock().unwrap();
            assert_eq!((10..=20).into_iter().collect::<Vec<_>>(), *asked);
        }
        for i in 0..=20 - PRUNING_THRESHOLD {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                None,
                "Session {:?} should be pruned",
                i
            );
        }
        for i in 21 - PRUNING_THRESHOLD..=20 {
            assert_eq!(
                session_map.get(SessionId(i)).await,
                Some(authority_data(4 * i as u64, 4 * (i + 1) as u64)),
                "Session {:?} should not be pruned",
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
