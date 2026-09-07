//! Cancellation for assistant requests that are already on the wire.
//!
//! A frontend request that is already on the wire cannot be called back, so
//! the network work runs in its own task and this registry keeps the handle.
//! Cancelling
//! aborts that task, which drops the HTTP request rather than leaving it
//! running and quietly billed while the user waits for nothing.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::task::JoinHandle;

use crate::error::{AppError, AppResult};

#[derive(Default)]
pub struct InFlight {
    tasks: Mutex<HashMap<u64, JoinHandle<()>>>,
}

impl InFlight {
    /// Takes ownership of a running request. A second request registered under
    /// the same id means the first one is orphaned, so it is aborted here
    /// rather than left running with nobody waiting for it.
    pub fn register(&self, id: u64, handle: JoinHandle<()>) -> AppResult<()> {
        if let Some(replaced) = self.lock()?.insert(id, handle) {
            replaced.abort();
        }
        Ok(())
    }

    /// Drops a finished request without aborting anything.
    pub fn finish(&self, id: u64) -> AppResult<()> {
        self.lock()?.remove(&id);
        Ok(())
    }

    /// Aborts a running request. Reports whether there was one to abort, so a
    /// Cancel that arrives after the answer did is not treated as a failure.
    pub fn cancel(&self, id: u64) -> AppResult<bool> {
        match self.lock()?.remove(&id) {
            Some(handle) => {
                handle.abort();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, HashMap<u64, JoinHandle<()>>>> {
        self.tasks
            .lock()
            .map_err(|_| AppError::runtime("the in-flight request lock was poisoned"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::{channel, Receiver, Sender};

    /// A task that parks until it is aborted, plus the channel that proves it.
    /// `done` closes when the task is dropped, so the assertions are about what
    /// actually happened to the task rather than about timing.
    fn parked_task() -> (JoinHandle<()>, Sender<()>, Receiver<()>) {
        let (release, mut wait) = channel::<()>(1);
        let (done, closed) = channel::<()>(1);

        let handle = tokio::spawn(async move {
            let _done = done;
            let _ = wait.recv().await;
        });

        (handle, release, closed)
    }

    #[tokio::test]
    async fn cancelling_aborts_the_request_and_reports_that_it_did() {
        let inflight = InFlight::default();
        let (handle, _release, mut closed) = parked_task();

        inflight.register(1, handle).unwrap();
        assert!(inflight.cancel(1).unwrap());
        assert_eq!(closed.recv().await, None, "the task was dropped");

        // The panel can send Cancel after the answer already arrived.
        assert!(!inflight.cancel(1).unwrap());
    }

    #[tokio::test]
    async fn finishing_leaves_nothing_to_cancel_and_aborts_nothing() {
        let inflight = InFlight::default();
        let (handle, release, mut closed) = parked_task();

        inflight.register(2, handle).unwrap();
        inflight.finish(2).unwrap();
        assert!(!inflight.cancel(2).unwrap());

        // Still alive: it ends when its own work does, not when it is forgotten.
        release.send(()).await.unwrap();
        assert_eq!(closed.recv().await, None);
    }

    #[tokio::test]
    async fn reusing_an_id_abandons_nothing_still_running() {
        let inflight = InFlight::default();
        let (first, _first_release, mut first_closed) = parked_task();
        let (second, _second_release, _) = parked_task();

        inflight.register(3, first).unwrap();
        inflight.register(3, second).unwrap();

        assert_eq!(first_closed.recv().await, None, "the orphan was aborted");
        assert!(inflight.cancel(3).unwrap(), "the newer request is the live one");
    }

    #[tokio::test]
    async fn requests_are_tracked_separately() {
        let inflight = InFlight::default();
        let (first, _first_release, _) = parked_task();
        let (second, _second_release, mut second_closed) = parked_task();

        inflight.register(4, first).unwrap();
        inflight.register(5, second).unwrap();

        assert!(inflight.cancel(5).unwrap());
        assert_eq!(second_closed.recv().await, None);
        assert!(inflight.cancel(4).unwrap(), "the other request was untouched");
    }
}
