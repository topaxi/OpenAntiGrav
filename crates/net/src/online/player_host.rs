//! A player's machine as the race host of a lobby room it owns: a listener opened when the
//! room is offered (so the lobby can probe it), and a thread that runs each race the lobby
//! sends, the same way a race child does ([`super::race_proc::run_race`])
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! decision 1).
//!
//! The host's own player is a client of it like everybody else: it dials this listener on the
//! loopback address with the seat token the job gives seat 0.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;

use anyhow::{Context, Result};
use web_time::{Duration, Instant};

use super::hostkey::PlayerKey;
use super::race_proc::{Ended, Limits, Loader, run_race};
use crate::fair::{Ledger, SharedLedger};
use crate::host::Host;
use crate::wire::online::RaceJob;
use crate::wt::{Identity, LinkLimits, Listener};

/// What the seats are told when the host's player leaves a race it hosts.
pub const HOST_LEFT: &str = "the host left: a player-hosted race ends with its host";

/// What the host thread reports, each once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The race of this room is loaded: its seats may dial.
    Up(u32),
    /// The race of this room is over (or could not be loaded), and why.
    Over(u32, Ended),
}

struct Job {
    room: u32,
    job: RaceJob,
    load: Loader,
    key: Option<Arc<PlayerKey>>,
    ledger: SharedLedger,
    #[cfg(feature = "test-cheats")]
    cheat: crate::host::Cheat,
}

/// A listener and the thread that hosts races on it.
pub struct PlayerHost {
    port: u16,
    hash: [u8; 32],
    jobs: mpsc::Sender<Job>,
    events: mpsc::Receiver<Event>,
    stop: Arc<AtomicBool>,
    end_race: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// The player's key, which signs each race's checkpoints ([`Self::sign_with`]).
    key: Option<Arc<PlayerKey>>,
    /// The signed datagram behind each input the race under way applied.
    ledger: SharedLedger,
    #[cfg(feature = "test-cheats")]
    cheat: crate::host::Cheat,
}

impl PlayerHost {
    /// Binds `listen` under a fresh certificate and starts the thread, which answers the
    /// lobby's probe and waits for a race.
    ///
    /// # Errors
    ///
    /// The certificate could not be made or the port could not be bound.
    pub fn bind(listen: SocketAddr, limits: Limits) -> Result<Self> {
        let identity = Identity::self_signed(&[])?;
        let listener = Listener::bind(listen, &identity, LinkLimits::RACE)
            .with_context(|| format!("binding the hosting port {}", listen.port()))?;
        let port = listener.local_addr().port();
        let (jobs, job_rx) = mpsc::channel();
        let (event_tx, events) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let end_race = Arc::new(AtomicBool::new(false));
        let flags = (stop.clone(), end_race.clone());
        let thread = std::thread::Builder::new()
            .name("oag-player-host".into())
            .spawn(move || run(listener, &job_rx, &event_tx, limits, &flags))
            .context("starting the host thread")?;
        Ok(Self {
            port,
            hash: identity.hash(),
            jobs,
            events,
            stop,
            end_race,
            thread: Some(thread),
            key: None,
            ledger: SharedLedger::default(),
            #[cfg(feature = "test-cheats")]
            cheat: crate::host::Cheat::None,
        })
    }

    /// The port actually bound.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The certificate hash the offer names and the seats pin.
    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    /// Signs every race's checkpoints with the player's `key` and takes each seat's inputs
    /// only signed by the key its job names
    /// (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
    /// "The anti-cheat path").
    pub fn sign_with(&mut self, key: Arc<PlayerKey>) {
        self.key = Some(key);
    }

    /// Makes the races this host runs commit `cheat` (tests only).
    #[cfg(feature = "test-cheats")]
    pub fn set_cheat(&mut self, cheat: crate::host::Cheat) {
        self.cheat = cheat;
    }

    /// Hosts `job`, the race of `room`, built by `load`.
    pub fn launch(&self, room: u32, job: RaceJob, load: Loader) {
        self.end_race.store(false, Ordering::Relaxed);
        if let Ok(mut ledger) = self.ledger.lock() {
            *ledger = Ledger::default();
        }
        let _ = self.jobs.send(Job {
            room,
            job,
            load,
            key: self.key.clone(),
            ledger: self.ledger.clone(),
            #[cfg(feature = "test-cheats")]
            cheat: self.cheat,
        });
    }

    /// The signed datagram behind the input the race under way applied for `seat` at `tick`,
    /// encoded, or empty when there is none: the answer to the lobby's challenge.
    #[must_use]
    pub fn show_input(&self, seat: u8, tick: u32) -> Vec<u8> {
        self.ledger
            .lock()
            .ok()
            .and_then(|ledger| ledger.find(usize::from(seat), tick))
            .unwrap_or_default()
    }

    /// How many signed inputs the race under way has applied.
    #[must_use]
    pub fn signed_inputs(&self) -> u64 {
        self.ledger.lock().map_or(0, |ledger| ledger.count())
    }

    /// Ends the race under way, telling its seats the host left.
    pub fn end_race(&self) {
        self.end_race.store(true, Ordering::Relaxed);
    }

    /// What has happened since the last call.
    pub fn poll(&mut self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

impl Drop for PlayerHost {
    /// Stops hosting. The thread is joined when it is quick to leave, so the seats are told
    /// and the port is free again; a race that is still loading is left to notice by itself.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let waited = Instant::now();
        while let Some(thread) = self.thread.take() {
            if thread.is_finished() {
                let _ = thread.join();
                break;
            }
            if waited.elapsed() > Duration::from_secs(3) {
                break;
            }
            self.thread = Some(thread);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl std::fmt::Debug for PlayerHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerHost")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

fn run(
    mut listener: Listener,
    jobs: &mpsc::Receiver<Job>,
    events: &mpsc::Sender<Event>,
    limits: Limits,
    (stop, end_race): &(Arc<AtomicBool>, Arc<AtomicBool>),
) {
    while !stop.load(Ordering::Relaxed) {
        // Between races nobody has a seat here: the lobby's probe needs only the handshake.
        while let Some(link) = listener.accept() {
            drop(link);
        }
        let Job {
            room,
            job,
            load,
            key,
            ledger,
            #[cfg(feature = "test-cheats")]
            cheat,
        } = match jobs.try_recv() {
            Ok(job) => job,
            Err(mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
        };
        let stopped = || stop.load(Ordering::Relaxed) || end_race.load(Ordering::Relaxed);
        let outcome = (|| -> Result<Ended, String> {
            #[allow(unused_mut)]
            let mut params = job.params.clone();
            #[cfg(feature = "test-cheats")]
            if cheat == crate::host::Cheat::Seed {
                params.seed ^= 0x5EED;
            }
            let race = load(&params).map_err(|e| format!("{e:#}"))?;
            let mut host = Host::new(race, params);
            host.set_stop_reason(HOST_LEFT);
            if let Some(key) = key {
                host.sign_with(key, job.keys.clone(), ledger);
            }
            #[cfg(feature = "test-cheats")]
            host.set_cheat(cheat);
            let _ = events.send(Event::Up(room));
            run_race(
                &mut host,
                &mut listener,
                &job.seats,
                limits,
                &stopped,
                &mut |line| log::debug!("hosting: {line}"),
            )
        })();
        let ended = match outcome {
            Ok(ended) => ended,
            Err(why) => Ended::without_board(why),
        };
        let _ = events.send(Event::Over(room, ended));
    }
}
