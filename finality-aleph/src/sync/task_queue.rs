use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fmt::{Debug, Formatter},
};

use log::warn;
use tokio::time::{sleep, Duration, Instant};

use crate::sync::LOG_TARGET;

#[derive(Clone)]
struct ScheduledTask<T> {
    task: T,
    scheduled_time: Instant,
}

impl<T> Eq for ScheduledTask<T> {}

impl<T> PartialEq for ScheduledTask<T> {
    fn eq(&self, other: &Self) -> bool {
        other.scheduled_time.eq(&self.scheduled_time)
    }
}

impl<T> PartialOrd for ScheduledTask<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for ScheduledTask<T> {
    /// Compare tasks so that earlier times come first in a max-heap.
    fn cmp(&self, other: &Self) -> Ordering {
        other.scheduled_time.cmp(&self.scheduled_time)
    }
}

#[derive(Clone, Default)]
pub struct TaskQueue<T> {
    queue: BinaryHeap<ScheduledTask<T>>,
}

impl<T> Debug for TaskQueue<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskQueue")
            .field("task count", &self.queue.len())
            .finish()
    }
}

/// Implements a queue allowing for scheduling tasks for some time in the future.
///
/// Does not actually execute any tasks, is used for ordering in time only.
impl<T> TaskQueue<T> {
    /// Creates an empty queue.
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
        }
    }

    /// Schedules `task` for after `delay`.
    pub fn schedule_in(&mut self, task: T, delay: Duration) {
        let scheduled_time = match Instant::now().checked_add(delay) {
            Some(time) => time,
            None => {
                warn!(
                    target: LOG_TARGET,
                    "Could not schedule task in {:?}. Instant out of bound.", delay
                );
                return;
            }
        };
        self.queue.push(ScheduledTask {
            task,
            scheduled_time,
        });
    }

    /// Awaits for the first and most overdue task and returns it. Returns `None` if there are no tasks.
    pub async fn pop(&mut self) -> Option<T> {
        let scheduled_task = self.queue.peek()?;

        let duration = scheduled_task
            .scheduled_time
            .saturating_duration_since(Instant::now());
        if !duration.is_zero() {
            sleep(duration).await;
        }
        self.queue.pop().map(|t| t.task)
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::{timeout, Duration};

    use super::TaskQueue;

    #[tokio::test]
    async fn test_scheduling() {
        let mut q = TaskQueue::new();
        q.schedule_in(2, Duration::from_millis(50));
        q.schedule_in(1, Duration::from_millis(20));

        assert!(timeout(Duration::from_millis(5), q.pop()).await.is_err());
        assert_eq!(
            timeout(Duration::from_millis(20), q.pop()).await,
            Ok(Some(1))
        );
        assert!(timeout(Duration::from_millis(10), q.pop()).await.is_err());
        assert_eq!(
            timeout(Duration::from_millis(50), q.pop()).await,
            Ok(Some(2))
        );
    }
}
