use crate::{
    first_block_of_session, session_id_from_block_num, ClientForAleph, SessionId, SessionPeriod,
};
use aleph_primitives::{AlephSessionApi, AuthorityId};
use futures::StreamExt;
use log::{debug, error, trace};
use sc_client_api::{Backend, FinalityNotification};
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_runtime::{
    generic::BlockId,
    traits::{Block, Header, NumberFor},
    SaturatedConversion,
};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use tokio::sync::{
    oneshot::{Receiver as OneShotReceiver, Sender as OneShotSender},
    RwLock,
};

type SessionMap = HashMap<SessionId, Vec<AuthorityId>>;
type SessionSubscribers = HashMap<SessionId, Vec<OneShotSender<Vec<AuthorityId>>>>;

pub trait AuthorityProvider<B> {
    /// returns authorities for block
    fn authorities(&self, block: B) -> Option<Vec<AuthorityId>>;
    /// returns next session authorities where current session is for block
    fn next_authorities(&self, block: B) -> Option<Vec<AuthorityId>>;
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
    fn authorities(&self, num: NumberFor<B>) -> Option<Vec<AuthorityId>> {
        self.client
            .runtime_api()
            .authorities(&BlockId::Number(num))
            .ok()
    }

    fn next_authorities(&self, num: NumberFor<B>) -> Option<Vec<AuthorityId>> {
        self.client
            .runtime_api()
            .next_session_authorities(&BlockId::Number(num))
            .map(|r| r.ok())
            .ok()
            .flatten()
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

#[derive(Clone)]
/// Wrapper around Mapping from sessionId to Vec of AuthorityIds allowing mutation
/// and hiding locking details
struct SharedSessionMap(Arc<RwLock<(SessionMap, SessionSubscribers)>>);

#[derive(Clone)]
/// Wrapper around Mapping from sessionId to Vec of AuthorityIds allowing only reads
pub struct ReadOnlySessionMap {
    inner: Arc<RwLock<(SessionMap, SessionSubscribers)>>,
}

impl SharedSessionMap {
    fn new() -> Self {
        Self(Arc::new(RwLock::new((HashMap::new(), HashMap::new()))))
    }

    async fn update(
        &mut self,
        id: SessionId,
        authorities: Vec<AuthorityId>,
    ) -> Option<Vec<AuthorityId>> {
        let mut guard = self.0.write().await;

        // notify all subscribers about insertion and remove them from subscription
        if let Some(senders) = guard.1.remove(&id) {
            for sender in senders {
                if let Err(e) = sender.send(authorities.clone()) {
                    error!(target: "aleph-session-updater", "Error while sending notification: {:?}", e);
                }
            }
        }

        guard.0.insert(id, authorities)
    }

    async fn prune_below(&mut self, id: SessionId) {
        let mut guard = self.0.write().await;

        guard.0.retain(|&s, _| s >= id);
        guard.1.retain(|&s, _| s >= id);
    }

    fn read_only(&self) -> ReadOnlySessionMap {
        ReadOnlySessionMap {
            inner: self.0.clone(),
        }
    }
}

impl ReadOnlySessionMap {
    pub async fn get(&self, id: SessionId) -> Option<Vec<AuthorityId>> {
        self.inner.read().await.0.get(&id).cloned()
    }

    /// returns an end of the oneshot channel that fires a message if either authorities are already
    /// known for the session with id = `id` or when the authorities are inserted for this session.
    pub async fn subscribe_to_insertion(&self, id: SessionId) -> OneShotReceiver<Vec<AuthorityId>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        let mut guard = self.inner.write().await;

        if let Some(authorities) = guard.0.get(&id) {
            // if the value is already present notify immediately
            sender
                .send(authorities.clone())
                .expect("we control both ends");
        } else {
            guard.1.entry(id).or_insert_with(Vec::new).push(sender);
        }

        receiver
    }
}

fn get_authorities_for_session<AP, B>(
    authority_provider: &AP,
    session_id: SessionId,
    first_block: NumberFor<B>,
) -> Vec<AuthorityId>
where
    B: Block,
    AP: AuthorityProvider<NumberFor<B>>,
{
    if session_id == SessionId(0) {
        authority_provider
            .authorities(<NumberFor<B>>::saturated_from(0u32))
            .expect("Authorities for the session 0 must be available from the beginning")
    } else {
        authority_provider.next_authorities(first_block).unwrap_or_else(||
            panic!("Authorities for next session {:?} must be available at first block #{:?} of current session", session_id.0, first_block)
        )
    }
}

/// Returns None if the num is not a first block of some session otherwise returns Some(id) where
/// id is id of a session that block starts.
fn is_first_block<B: Block>(num: NumberFor<B>, period: SessionPeriod) -> Option<SessionId> {
    let session = session_id_from_block_num::<B>(num, period);

    if first_block_of_session::<B>(session, period) == num {
        Some(session)
    } else {
        None
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

    /// puts authorities for the next session into the session map
    async fn handle_first_block_of_session(&mut self, num: NumberFor<B>, session_id: SessionId) {
        debug!(target: "aleph-session-updater", "Handling first block #{:?} of session {:?}", num, session_id.0);
        let next_session = SessionId(session_id.0 + 1);
        let authority_provider = &self.authority_provider;
        self.session_map
            .update(
                next_session,
                get_authorities_for_session::<_, B>(authority_provider, next_session, num),
            )
            .await;

        // if this is the first session we also need to include starting authorities into the map
        if session_id.0 == 0 {
            let authority_provider = &self.authority_provider;
            self.session_map
                .update(
                    session_id,
                    get_authorities_for_session::<_, B>(authority_provider, session_id, num),
                )
                .await;
        }

        if session_id.0 >= 10 && session_id.0 % 10 == 0 {
            debug!(target: "aleph-session-updater", "Pruning session map below session #{:?}", session_id.0 - 10);
            self.session_map
                .prune_below(SessionId(session_id.0 - 10))
                .await;
        }
    }

    pub async fn run(mut self, period: SessionPeriod) {
        let mut notifications = self.finality_notificator.notification_stream();

        // lets catch up
        for block_num in 0..=self
            .finality_notificator
            .last_finalized()
            .saturated_into::<u32>()
        {
            let block_num = block_num.saturated_into();
            if let Some(session_id) = is_first_block::<B>(block_num, period) {
                self.handle_first_block_of_session(block_num, session_id)
                    .await;
            }
        }

        while let Some(FinalityNotification { header, .. }) = notifications.next().await {
            let last_finalized = header.number();
            trace!(target: "aleph-session-updater", "got FinalityNotification about #{:?}", last_finalized);

            if let Some(session_id) = is_first_block::<B>(*last_finalized, period) {
                // we have finalized first block of some session, now we can put the next known authorities into session map
                self.handle_first_block_of_session(*last_finalized, session_id)
                    .await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mocks::TBlock;
    use futures_timer::Delay;
    use sc_block_builder::BlockBuilderProvider;
    use sc_utils::mpsc::tracing_unbounded;
    use sp_consensus::BlockOrigin;
    use sp_runtime::testing::UintAuthorityId;
    use std::time::Duration;
    use substrate_test_runtime_client::{
        ClientBlockImportExt, DefaultTestClientBuilderExt, TestClient, TestClientBuilder,
        TestClientBuilderExt,
    };
    use tokio::sync::oneshot::error::TryRecvError;

    struct MockProvider {
        pub session_map: HashMap<NumberFor<TBlock>, Vec<AuthorityId>>,
        pub next_session_map: HashMap<NumberFor<TBlock>, Vec<AuthorityId>>,
    }

    struct MockNotificator {
        pub last_finalized: NumberFor<TBlock>,
        pub receiver:
            std::sync::Mutex<Option<TracingUnboundedReceiver<FinalityNotification<TBlock>>>>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                session_map: HashMap::new(),
                next_session_map: HashMap::new(),
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
        fn authorities(&self, b: NumberFor<TBlock>) -> Option<Vec<AuthorityId>> {
            self.session_map.get(&b).cloned()
        }

        fn next_authorities(&self, b: NumberFor<TBlock>) -> Option<Vec<AuthorityId>> {
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

    fn authorities(from: u64, to: u64) -> Vec<AuthorityId> {
        (from..to)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect()
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

        mock_provider.session_map.insert(0, authorities(0, 4));
        mock_provider.next_session_map.insert(0, authorities(4, 8));
        mock_provider.next_session_map.insert(1, authorities(8, 12));
        mock_provider
            .next_session_map
            .insert(2, authorities(12, 16));

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator);
        let session_map = updater.readonly_session_map();

        let blocks = n_new_blocks(&mut client, 2);
        let block_1 = blocks.get(0).cloned().unwrap();
        let block_2 = blocks.get(1).cloned().unwrap();
        sender
            .unbounded_send(FinalityNotification {
                hash: block_1.header.hash(),
                header: block_1.header,
            })
            .unwrap();
        sender
            .unbounded_send(FinalityNotification {
                hash: block_2.header.hash(),
                header: block_2.header,
            })
            .unwrap();

        let _handle = tokio::spawn(updater.run(SessionPeriod(1)));

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        assert_eq!(session_map.get(SessionId(0)).await, Some(authorities(0, 4)));
        assert_eq!(session_map.get(SessionId(1)).await, Some(authorities(4, 8)));
        assert_eq!(
            session_map.get(SessionId(2)).await,
            Some(authorities(8, 12))
        );
        assert_eq!(
            session_map.get(SessionId(3)).await,
            Some(authorities(12, 16))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn updates_session_map_on_catching_up() {
        let (_sender, receiver) = tracing_unbounded("test");
        let mut mock_provider = MockProvider::new();
        let mut mock_notificator = MockNotificator::new(receiver);

        mock_provider.session_map.insert(0, authorities(0, 4));
        mock_provider.next_session_map.insert(0, authorities(4, 8));
        mock_provider.next_session_map.insert(1, authorities(8, 12));
        mock_provider
            .next_session_map
            .insert(2, authorities(12, 16));

        mock_notificator.last_finalized = 2;

        let updater = SessionMapUpdater::new(mock_provider, mock_notificator);
        let session_map = updater.readonly_session_map();

        let _handle = tokio::spawn(updater.run(SessionPeriod(1)));

        // wait a bit
        Delay::new(Duration::from_millis(50)).await;

        assert_eq!(session_map.get(SessionId(0)).await, Some(authorities(0, 4)));
        assert_eq!(session_map.get(SessionId(1)).await, Some(authorities(4, 8)));
        assert_eq!(
            session_map.get(SessionId(2)).await,
            Some(authorities(8, 12))
        );
        assert_eq!(
            session_map.get(SessionId(3)).await,
            Some(authorities(12, 16))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn subscription_with_already_defined_session_works() {
        let mut shared = SharedSessionMap::new();
        let readonly = shared.read_only();
        let session = SessionId(0);

        shared.update(session, authorities(0, 2)).await;

        let mut receiver = readonly.subscribe_to_insertion(session).await;

        // we should have this immediately
        assert_eq!(Ok(authorities(0, 2)), receiver.try_recv());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn notifies_on_insertion() {
        let mut shared = SharedSessionMap::new();
        let readonly = shared.read_only();
        let session = SessionId(0);
        let mut receiver = readonly.subscribe_to_insertion(session).await;

        // does not yet have any value
        assert_eq!(Err(TryRecvError::Empty), receiver.try_recv());
        shared.update(session, authorities(0, 2)).await;
        assert_eq!(Ok(authorities(0, 2)), receiver.await);
    }
}
