//! A number asked for once a period on a thread of its own, the last answer published for a
//! frame loop that must not wait on whoever answers (an X server round trip, a JNI call).

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

/// No answer yet, or none that could be given.
const UNKNOWN: u32 = 0;

/// A thread calling `read` every period and keeping its last answer. Dropping it ends the thread
/// at its next wake.
#[derive(Debug)]
pub struct Poller {
    value: Arc<AtomicU32>,
    _stop: Sender<()>,
}

impl Poller {
    /// Starts a thread called `name` that calls `read` at once and then every `every`. A round
    /// that answers `None` (or zero) leaves the last answer standing.
    ///
    /// # Errors
    ///
    /// The thread could not be started.
    pub fn start(
        name: &'static str,
        every: Duration,
        mut read: impl FnMut() -> Option<u32> + Send + 'static,
    ) -> std::io::Result<Self> {
        let value = Arc::new(AtomicU32::new(UNKNOWN));
        let (stop, wake) = mpsc::channel::<()>();
        let shared = Arc::clone(&value);
        std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                loop {
                    if let Some(answer) = read().filter(|&answer| answer != UNKNOWN) {
                        shared.store(answer, Ordering::Relaxed);
                    }
                    if wake.recv_timeout(every) != Err(RecvTimeoutError::Timeout) {
                        return;
                    }
                }
            })?;
        Ok(Self { value, _stop: stop })
    }

    /// The last answer, or `None` while there has been none.
    #[must_use]
    pub fn latest(&self) -> Option<u32> {
        Some(self.value.load(Ordering::Relaxed)).filter(|&value| value != UNKNOWN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

    const WAIT: Duration = Duration::from_secs(10);
    const SOON: Duration = Duration::from_millis(1);

    /// A reader the test steps: it reports that it was entered (so the previous answer is
    /// published), then answers what the test sends.
    fn stepped() -> (
        impl FnMut() -> Option<u32> + Send + 'static,
        Receiver<()>,
        SyncSender<Option<u32>>,
    ) {
        let (entered, entries) = sync_channel(1);
        let (answers, next) = sync_channel::<Option<u32>>(1);
        let read = move || {
            let _ = entered.send(());
            next.recv().ok().flatten()
        };
        (read, entries, answers)
    }

    #[test]
    fn a_round_with_no_answer_keeps_the_last_one() {
        let (read, entries, answers) = stepped();
        let poller = Poller::start("poller-test", SOON, read).expect("thread");
        let step = |answer| {
            answers.send(answer).expect("reader alive");
            entries.recv_timeout(WAIT).expect("next round");
        };
        entries.recv_timeout(WAIT).expect("first round");
        assert_eq!(poller.latest(), None);
        step(None);
        assert_eq!(poller.latest(), None, "a failure before any answer");
        step(Some(120_000));
        assert_eq!(poller.latest(), Some(120_000));
        step(None);
        assert_eq!(poller.latest(), Some(120_000), "a failure after an answer");
        step(Some(0));
        assert_eq!(poller.latest(), Some(120_000), "zero is no answer");
        step(Some(60_000));
        assert_eq!(poller.latest(), Some(60_000));
    }

    #[test]
    fn dropping_it_ends_the_thread() {
        let (alive, gone) = sync_channel::<()>(1);
        let poller = Poller::start("poller-test", SOON, move || {
            let _held = &alive;
            None
        })
        .expect("thread");
        drop(poller);
        assert_eq!(
            gone.recv_timeout(WAIT),
            Err(RecvTimeoutError::Disconnected),
            "the reader is dropped with its thread"
        );
    }
}
