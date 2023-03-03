use futures::StreamExt;
use log::info;
use primitives::{EraIndex, SessionIndex};
use subxt::events::StaticEvent;

use crate::{
    aleph_zero,
    api::session::events::NewSession,
    connections::AsConnection,
    pallets::{session::SessionApi, staking::StakingApi},
};

/// When using waiting API, what kind of block status we should wait for.
pub enum BlockStatus {
    /// Wait for event or block to be in the best chain.
    Best,
    /// Wait for the event or block to be in the finalized chain.
    Finalized,
}

/// Waiting _for_ various events API
#[async_trait::async_trait]
pub trait AlephWaiting {
    /// Wait for a particular block to be in a [`BlockStatus`].
    /// Block number must match given predicate.
    /// * `predicate` - a `u32` -> `bool` functor, first argument is a block number
    /// * `status` - a [`BlockStatus`] of the block we wait for
    ///
    /// # Examples
    /// ```ignore
    /// let finalized = connection.connection.get_finalized_block_hash().await;
    ///     let finalized_number = connection
    ///         .connection
    ///         .get_block_number(finalized)
    ///         .await
    ///         .unwrap();
    ///     connection
    ///         .connection
    ///         .wait_for_block(|n| n > finalized_number, BlockStatus::Finalized)
    ///         .await;
    /// ```
    async fn wait_for_block<P: Fn(u32) -> bool + Send>(&self, predicate: P, status: BlockStatus);

    /// Wait for a particular event to be emitted on chain.
    /// * `predicate` - a predicate that has one argument (ref to an emitted event)
    /// * `status` - a [`BlockStatus`] of the event we wait for
    ///
    /// # Examples
    /// ```ignore
    /// let event = connection
    ///         .wait_for_event(
    ///             |event: &BanValidators| {
    ///                 info!("Received BanValidators event: {:?}", event.0);
    ///                 true
    ///             },
    ///             BlockStatus::Best,
    ///         )
    ///         .await;
    /// ```
    async fn wait_for_event<T: StaticEvent, P: Fn(&T) -> bool + Send>(
        &self,
        predicate: P,
        status: BlockStatus,
    ) -> T;

    /// Wait for given era to happen.
    /// * `era` - number of the era to wait for
    /// * `status` - a [`BlockStatus`] of the era we wait for
    async fn wait_for_era(&self, era: EraIndex, status: BlockStatus);

    /// Wait for given session to happen.
    /// * `session` - number of the session to wait for
    /// * `status` - a [`BlockStatus`] of the session we wait for
    async fn wait_for_session(&self, session: SessionIndex, status: BlockStatus);
}

/// nWaiting _from_ the current moment of time API
#[async_trait::async_trait]
pub trait WaitingExt {
    /// Wait for a given number of sessions to wait from a current session.
    /// `n` - number of sessions to wait from now
    /// * `status` - a [`BlockStatus`] of the session we wait for
    async fn wait_for_n_sessions(&self, n: SessionIndex, status: BlockStatus);

    /// Wait for a given number of eras to wait from a current era.
    /// `n` - number of eras to wait from now
    /// * `status` - a [`BlockStatus`] of the era we wait for
    async fn wait_for_n_eras(&self, n: EraIndex, status: BlockStatus);
}

#[async_trait::async_trait]
impl<C: AsConnection + Sync> AlephWaiting for C {
    async fn wait_for_block<P: Fn(u32) -> bool + Send>(&self, predicate: P, status: BlockStatus) {
        let mut block_sub = match status {
            BlockStatus::Best => self
                .as_connection()
                .as_client()
                .blocks()
                .subscribe_best()
                .await
                .expect("Failed to subscribe to the best block stream!"),
            BlockStatus::Finalized => self
                .as_connection()
                .as_client()
                .blocks()
                .subscribe_finalized()
                .await
                .expect("Failed to subscribe to the finalized block stream!"),
        };

        while let Some(Ok(block)) = block_sub.next().await {
            if predicate(block.number()) {
                return;
            }
        }
    }

    async fn wait_for_event<T: StaticEvent, P: Fn(&T) -> bool + Send>(
        &self,
        predicate: P,
        status: BlockStatus,
    ) -> T {
        let mut block_sub = match status {
            BlockStatus::Best => self
                .as_connection()
                .as_client()
                .blocks()
                .subscribe_best()
                .await
                .expect("Failed to subscribe to the best block stream!"),
            BlockStatus::Finalized => self
                .as_connection()
                .as_client()
                .blocks()
                .subscribe_finalized()
                .await
                .expect("Failed to subscribe to the finalized block stream!"),
        };

        info!(target: "aleph-client", "waiting for event {}.{}", T::PALLET, T::EVENT);

        while let Some(Ok(block)) = block_sub.next().await {
            let events = match block.events().await {
                Ok(events) => events,
                _ => continue,
            };

            for event in events.iter() {
                let event = event.expect("Failed to obtain event from the block!");
                if let Ok(Some(ev)) = event.as_event::<T>() {
                    if predicate(&ev) {
                        return ev;
                    }
                }
            }
        }

        panic!("No more blocks");
    }

    async fn wait_for_era(&self, era: EraIndex, status: BlockStatus) {
        let addrs = aleph_zero::api::constants().staking().sessions_per_era();
        let sessions_per_era = self
            .as_connection()
            .as_client()
            .constants()
            .at(&addrs)
            .expect("Failed to obtain sessions_per_era const!");
        let first_session_in_era = era * sessions_per_era;

        self.wait_for_session(first_session_in_era, status).await;
    }

    async fn wait_for_session(&self, session: SessionIndex, status: BlockStatus) {
        self.wait_for_event(|event: &NewSession| {
            info!(target: "aleph-client", "waiting for session {:?}, current session {:?}", session, event.session_index);
            event.session_index >= session
        }, status)
            .await;
    }
}

#[async_trait::async_trait]
impl<C: AsConnection + Sync> WaitingExt for C {
    async fn wait_for_n_sessions(&self, n: SessionIndex, status: BlockStatus) {
        let current_session = self.get_session(None).await;

        self.wait_for_session(current_session + n, status).await;
    }

    async fn wait_for_n_eras(&self, n: EraIndex, status: BlockStatus) {
        let current_era = self.get_current_era(None).await;

        self.wait_for_era(current_era + n, status).await;
    }
}
