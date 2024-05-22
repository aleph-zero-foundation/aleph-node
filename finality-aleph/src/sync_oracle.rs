use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use sp_consensus::SyncOracle as SyncOracleT;

const OFFLINE_THRESHOLD: Duration = Duration::from_secs(6);
const FAR_BEHIND_THRESHOLD: u32 = 15;
const MAJOR_SYNC_THRESHOLD: Duration = Duration::from_secs(10);

pub type MajorSyncIndicator = Arc<AtomicBool>;

/// A sync oracle implementation tracking how recently the node was far behind the highest known justification.
/// It defines being in major sync as being more than 15 blocks behind the highest known justification less than 10 seconds ago.
/// It defines being offline as not getting any update for at least 6 seconds (or never at all).
#[derive(Clone)]
pub struct SyncOracle {
    last_far_behind: Arc<Mutex<Instant>>,
    last_update: Arc<Mutex<Instant>>,
    // TODO: remove when SyncingService is no longer needed
    is_major_syncing: Arc<AtomicBool>,
}

impl SyncOracle {
    pub fn new() -> (Self, MajorSyncIndicator) {
        let is_major_syncing = Arc::new(AtomicBool::new(true));
        let oracle = SyncOracle {
            last_update: Arc::new(Mutex::new(Instant::now() - OFFLINE_THRESHOLD)),
            last_far_behind: Arc::new(Mutex::new(Instant::now())),
            is_major_syncing: is_major_syncing.clone(),
        };
        (oracle, is_major_syncing)
    }

    pub fn update_behind(&self, behind: u32) {
        let now = Instant::now();
        *self.last_update.lock() = now;
        if behind > FAR_BEHIND_THRESHOLD {
            *self.last_far_behind.lock() = now;
        }
        self.major_sync();
    }

    pub fn major_sync(&self) -> bool {
        let last_far_behind = self.last_far_behind.lock();
        let is_major_syncing = last_far_behind.elapsed() < MAJOR_SYNC_THRESHOLD;
        self.is_major_syncing
            .store(is_major_syncing, Ordering::Relaxed);
        is_major_syncing
    }
}

impl Default for SyncOracle {
    fn default() -> Self {
        SyncOracle::new().0
    }
}

impl SyncOracleT for SyncOracle {
    fn is_major_syncing(&self) -> bool {
        self.major_sync()
    }

    fn is_offline(&self) -> bool {
        self.last_update.lock().elapsed() > OFFLINE_THRESHOLD
    }
}
