use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use sp_consensus::SyncOracle as SyncOracleT;

// This should never exceed 1800 due to the structure of the forest, thus 9000 is a decent marker of being uninitialized.
const UNINITIALIZED_THRESHOLD: u32 = 9000;
const OFFLINE_THRESHOLD: Duration = Duration::from_secs(6);

/// A sync oracle implementation tracking how far behind the highest known justification the node is.
/// It defines being in major sync as knowing of any justification of an unknown block.
/// It defines being offline as not getting any update for at least 6 seconds (or never at all).
#[derive(Clone)]
pub struct SyncOracle {
    behind: Arc<AtomicU32>,
    last_update: Arc<Mutex<Instant>>,
}

impl SyncOracle {
    pub fn new() -> Self {
        SyncOracle {
            behind: Arc::new(AtomicU32::new(UNINITIALIZED_THRESHOLD + 1)),
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn update_behind(&self, behind: u32) {
        self.behind.store(behind, Ordering::Relaxed);
        *self.last_update.lock() = Instant::now();
    }
}

impl Default for SyncOracle {
    fn default() -> Self {
        SyncOracle::new()
    }
}

impl SyncOracleT for SyncOracle {
    fn is_major_syncing(&self) -> bool {
        self.behind.load(Ordering::Relaxed) > 0
    }

    fn is_offline(&self) -> bool {
        self.last_update.lock().elapsed() > OFFLINE_THRESHOLD
            || self.behind.load(Ordering::Relaxed) > UNINITIALIZED_THRESHOLD
    }
}
