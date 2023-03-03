use std::{
    collections::HashMap,
    marker::PhantomData,
    time::{Duration, Instant},
};

use log::{debug, info, trace};

use crate::{
    network::{
        session::{Authentication, SessionHandler},
        AddressingInformation,
    },
    NodeIndex,
};

/// Handles creating and rebroadcasting discovery messages.
pub struct Discovery<A: AddressingInformation> {
    cooldown: Duration,
    last_broadcast: HashMap<NodeIndex, Instant>,
    _phantom: PhantomData<A>,
}

impl<A: AddressingInformation> Discovery<A> {
    /// Create a new discovery handler with the given response/broadcast cooldown.
    pub fn new(cooldown: Duration) -> Self {
        Discovery {
            cooldown,
            last_broadcast: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Returns a message that should be sent as part of authority discovery at this moment.
    pub fn discover_authorities(
        &mut self,
        handler: &SessionHandler<A>,
    ) -> Option<Authentication<A>> {
        let authentication = match handler.authentication() {
            Some(authentication) => authentication,
            None => return None,
        };

        let missing_authorities = handler.missing_nodes();
        let node_count = handler.node_count();
        info!(target: "aleph-network", "{}/{} authorities known for session {}.", node_count.0-missing_authorities.len(), node_count.0, handler.session_id().0);
        Some(authentication)
    }

    fn should_rebroadcast(&self, node_id: &NodeIndex) -> bool {
        match self.last_broadcast.get(node_id) {
            Some(instant) => Instant::now() > *instant + self.cooldown,
            None => true,
        }
    }

    /// Processes the provided authentication and returns any new address we should
    /// be connected to if we want to stay connected to the committee and an optional
    /// message that we should send as a result of it.
    pub fn handle_authentication(
        &mut self,
        authentication: Authentication<A>,
        handler: &mut SessionHandler<A>,
    ) -> (Option<A>, Option<Authentication<A>>) {
        debug!(target: "aleph-network", "Handling broadcast with authentication {:?}.", authentication);
        let address = match handler.handle_authentication(authentication.clone()) {
            Some(address) => Some(address),
            None => return (None, None),
        };
        let node_id = authentication.0.creator();
        if !self.should_rebroadcast(&node_id) {
            return (address, None);
        }
        trace!(target: "aleph-network", "Rebroadcasting {:?}.", authentication);
        self.last_broadcast.insert(node_id, Instant::now());
        (address, Some(authentication))
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::Discovery;
    use crate::{
        network::{
            clique::mock::{random_address, MockAddressingInformation},
            mock::crypto_basics,
            session::{authentication, Authentication, SessionHandler},
        },
        SessionId,
    };

    const NUM_NODES: u8 = 7;
    const MS_COOLDOWN: u64 = 200;

    fn addresses() -> Vec<MockAddressingInformation> {
        (0..NUM_NODES).map(|_| random_address()).collect()
    }

    async fn build_number(
        num_nodes: u8,
    ) -> (
        Discovery<MockAddressingInformation>,
        Vec<SessionHandler<MockAddressingInformation>>,
        SessionHandler<MockAddressingInformation>,
    ) {
        let crypto_basics = crypto_basics(num_nodes.into()).await;
        let mut handlers = Vec::new();
        for (authority_index_and_pen, address) in crypto_basics.0.into_iter().zip(addresses()) {
            handlers.push(
                SessionHandler::new(
                    Some(authority_index_and_pen),
                    crypto_basics.1.clone(),
                    SessionId(43),
                    address,
                )
                .await,
            );
        }
        let non_validator = SessionHandler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        (
            Discovery::new(Duration::from_millis(MS_COOLDOWN)),
            handlers,
            non_validator,
        )
    }

    async fn build() -> (
        Discovery<MockAddressingInformation>,
        Vec<SessionHandler<MockAddressingInformation>>,
        SessionHandler<MockAddressingInformation>,
    ) {
        build_number(NUM_NODES).await
    }

    #[tokio::test]
    async fn broadcasts_when_clueless() {
        for num_nodes in 2..NUM_NODES {
            let (mut discovery, mut handlers, _) = build_number(num_nodes).await;
            let handler = &mut handlers[0];
            let maybe_authentication = discovery.discover_authorities(handler);
            assert_eq!(
                maybe_authentication.expect("there is an authentication"),
                handler
                    .authentication()
                    .expect("the handler has an authentication"),
            );
        }
    }

    #[tokio::test]
    async fn non_validator_discover_authorities_returns_empty_vector() {
        let (mut discovery, _, non_validator) = build().await;
        let maybe_authentication = discovery.discover_authorities(&non_validator);
        assert!(maybe_authentication.is_none());
    }

    #[tokio::test]
    async fn rebroadcasts_and_accepts_addresses() {
        let (mut discovery, mut handlers, _) = build().await;
        let authentication = authentication(&handlers[1]);
        let handler = &mut handlers[0];
        let (address, command) = discovery.handle_authentication(authentication.clone(), handler);
        assert_eq!(address, Some(authentication.0.address()));
        assert!(matches!(command, Some(
                rebroadcast_authentication,
            ) if rebroadcast_authentication == authentication));
    }

    #[tokio::test]
    async fn non_validator_rebroadcasts() {
        let (mut discovery, handlers, mut non_validator) = build().await;
        let authentication = authentication(&handlers[1]);
        let (address, command) =
            discovery.handle_authentication(authentication.clone(), &mut non_validator);
        assert_eq!(address, Some(authentication.0.address()));
        assert!(matches!(command, Some(
                rebroadcast_authentication,
            ) if rebroadcast_authentication == authentication));
    }

    #[tokio::test]
    async fn does_not_rebroadcast_wrong_authentications() {
        let (mut discovery, mut handlers, _) = build().await;
        let Authentication(auth_data, _) = authentication(&handlers[1]);
        let Authentication(_, signature) = authentication(&handlers[2]);
        let authentication = Authentication(auth_data, signature);
        let handler = &mut handlers[0];
        let (address, command) = discovery.handle_authentication(authentication, handler);
        assert!(address.is_none());
        assert!(command.is_none());
    }

    #[tokio::test]
    async fn rebroadcasts_after_cooldown() {
        let (mut discovery, mut handlers, _) = build().await;
        let authentication = authentication(&handlers[1]);
        let handler = &mut handlers[0];
        discovery.handle_authentication(authentication.clone(), handler);
        sleep(Duration::from_millis(MS_COOLDOWN + 5));
        let (address, command) = discovery.handle_authentication(authentication.clone(), handler);
        assert_eq!(address, Some(authentication.0.address()));
        assert!(matches!(command, Some(
                rebroadcast_authentication,
            ) if rebroadcast_authentication == authentication));
    }
}
