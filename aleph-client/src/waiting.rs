use futures::StreamExt;
use log::info;
use primitives::{EraIndex, SessionIndex};
use subxt::events::StaticEvent;

use crate::{
    aleph_zero,
    api::session::events::NewSession,
    pallets::{session::SessionApi, staking::StakingApi},
    Connection,
};

pub enum BlockStatus {
    Best,
    Finalized,
}

#[async_trait::async_trait]
pub trait AlephWaiting {
    async fn wait_for_block<P: Fn(u32) -> bool + Send>(&self, predicate: P, status: BlockStatus);
    async fn wait_for_event<T: StaticEvent, P: Fn(&T) -> bool + Send>(
        &self,
        predicate: P,
        status: BlockStatus,
    ) -> T;
    async fn wait_for_era(&self, era: EraIndex, status: BlockStatus);
    async fn wait_for_session(&self, session: SessionIndex, status: BlockStatus);
}

#[async_trait::async_trait]
pub trait WaitingExt {
    async fn wait_for_n_sessions(&self, n: SessionIndex, status: BlockStatus);
    async fn wait_for_n_eras(&self, n: EraIndex, status: BlockStatus);
}

#[async_trait::async_trait]
impl AlephWaiting for Connection {
    async fn wait_for_block<P: Fn(u32) -> bool + Send>(&self, predicate: P, status: BlockStatus) {
        let mut block_sub = match status {
            BlockStatus::Best => self
                .client
                .blocks()
                .subscribe_best()
                .await
                .expect("Failed to subscribe to the best block stream!"),
            BlockStatus::Finalized => self
                .client
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
                .client
                .blocks()
                .subscribe_best()
                .await
                .expect("Failed to subscribe to the best block stream!"),
            BlockStatus::Finalized => self
                .client
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
            .client
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
impl WaitingExt for Connection {
    async fn wait_for_n_sessions(&self, n: SessionIndex, status: BlockStatus) {
        let current_session = self.get_session(None).await;

        self.wait_for_session(current_session + n, status).await;
    }

    async fn wait_for_n_eras(&self, n: EraIndex, status: BlockStatus) {
        let current_era = self.get_current_era(None).await;

        self.wait_for_era(current_era + n, status).await;
    }
}
