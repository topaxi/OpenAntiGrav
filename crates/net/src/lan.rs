//! A game hosting a LAN race in its own process: the same authoritative host
//! `oag-server` runs ([`crate::host::Host`], [`crate::serve::run`]), on a thread of its
//! own, with a lobby in front of it.
//!
//! [`Hosted::spawn`] binds the game port on every interface, starts answering discovery
//! queries ([`crate::discovery`]) and returns, with the **host's own end of a
//! [`Loopback`] link already waiting first in the lobby**: the host plays as a client of
//! its own server over that link, through the same [`crate::client::Client`] a joiner
//! uses, so there is one path to the race and the host is seat 0 by construction (a
//! WebTransport connection to itself would be the same code with TLS in the way, and
//! could be beaten to seat 0 by a fast joiner).
//!
//! The lobby holds every welcome until the host presses START ([`Hosted::start`]), because
//! the welcome carries the race and its head count. Then the thread loads the race
//! (`oag_net::race::load`, the server's path), seats the waiting links in arrival order,
//! waits for every seat to report the same tick-0 hash and runs the race.
//!
//! **Chosen, not measured:** START is allowed with the host alone (a race against the
//! AI over the network code is still a valid race), the load timeout, the port. The
//! host's machine is trusted (decision 3 of `docs/networking/README.md`): the certificate
//! hash goes out in the discovery answer.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use anyhow::{Context, Result};
use web_time::{Duration, Instant};

use crate::discovery::{Advert, Responder};
use crate::host::Host;
use crate::link::{Link, Loopback};
use crate::lobby::Lobby;
use crate::serve::{self, Pace};
use crate::wire::{Control, PROTOCOL, RaceParams};
use crate::wt::{Identity, Listener};

/// The port the game listens on unless asked otherwise. Chosen; `oag-server` uses 4433.
pub const DEFAULT_PORT: u16 = 4435;

/// How long a seat may take to load the race and report ready. Chosen.
const LOAD_TIMEOUT: Duration = Duration::from_secs(180);

/// What a host is told to race and how to present itself.
#[derive(Debug, Clone)]
pub struct Config {
    /// The title's disc image or folder, as the host's game opened it.
    pub source: String,
    /// The race, with `title` filled in. `people` is replaced by the head count at START.
    pub params: RaceParams,
    /// What the host calls itself in the discovery answer.
    pub name: String,
    /// The circuit as a joiner's list should show it.
    pub track_label: String,
    /// The game port; 0 lets the system pick.
    pub port: u16,
    /// The UDP port discovery queries are answered on; [`crate::discovery::PORT`] for a game.
    pub discovery_port: u16,
}

/// Where a hosted race is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Collecting people.
    Lobby,
    /// START pressed: the race is loading, the seats are loading it.
    Loading,
    /// Ticking.
    Racing,
    /// Over, and why.
    Over(String),
    /// Could not go on, and why.
    Failed(String),
}

/// What the thread reports to the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// Where the race is.
    pub phase: Phase,
    /// The names waiting, the host's own first.
    pub names: Vec<String>,
}

/// A race hosted by this process.
pub struct Hosted {
    status: Arc<Mutex<Status>>,
    go: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    port: u16,
    hash: [u8; 32],
    announcing: bool,
}

impl Hosted {
    /// Starts hosting. Returns the host and the host player's own end of the lobby link.
    ///
    /// # Errors
    ///
    /// The certificate could not be made or the game port could not be bound. Discovery
    /// not binding (another host on this machine) is not an error: the game still
    /// listens, and [`Self::announcing`] says it cannot be found.
    pub fn spawn(config: Config) -> Result<(Self, Loopback)> {
        let identity = Identity::self_signed(&[])?;
        let listener = Listener::bind(
            SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), config.port),
            &identity,
        )
        .context("binding the game port")?;
        let port = listener.local_addr().port();
        let responder = match Responder::on(config.discovery_port) {
            Ok(responder) => Some(responder),
            Err(e) => {
                log::warn!("lan: cannot answer discovery ({e}); joiners must type this address");
                None
            }
        };
        let announcing = responder.is_some();
        let (mine, theirs) = Loopback::pair();
        let status = Arc::new(Mutex::new(Status {
            phase: Phase::Lobby,
            names: Vec::new(),
        }));
        let go = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let hash = identity.hash();
        let thread = {
            let shared = (status.clone(), go.clone(), stop.clone());
            std::thread::Builder::new()
                .name("lan-host".into())
                .spawn(move || {
                    let end = run(config, listener, responder, hash, theirs, &shared);
                    let mut status = shared.0.lock().unwrap_or_else(|e| e.into_inner());
                    match end {
                        Ok(why) => status.phase = Phase::Over(why),
                        Err(e) => {
                            log::error!("lan: {e:#}");
                            status.phase = Phase::Failed(format!("{e:#}"));
                        }
                    }
                })
                .context("starting the host thread")?
        };
        Ok((
            Self {
                status,
                go,
                stop,
                thread: Some(thread),
                port,
                hash,
                announcing,
            },
            mine,
        ))
    }

    /// The game port actually bound.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The certificate hash joiners pin.
    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    /// Whether discovery queries are being answered.
    #[must_use]
    pub fn announcing(&self) -> bool {
        self.announcing
    }

    /// Where the race is and who waits.
    #[must_use]
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// START: fixes the head count at whoever has said hello and loads the race.
    pub fn start(&self) {
        self.go.store(true, Ordering::Relaxed);
    }
}

impl Drop for Hosted {
    /// Stops hosting. The thread is joined when it is quick to leave (the lobby, a running
    /// race) so the port is free for the next host; a race that is still loading is left
    /// to finish and exit on its own.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let waited = Instant::now();
        while let Some(thread) = self.thread.take() {
            if thread.is_finished() {
                let _ = thread.join();
                break;
            }
            if waited.elapsed() > Duration::from_secs(2) {
                break;
            }
            self.thread = Some(thread);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

type Shared = (Arc<Mutex<Status>>, Arc<AtomicBool>, Arc<AtomicBool>);

fn set(shared: &Shared, phase: Option<Phase>, names: Option<Vec<String>>) {
    let mut status = shared.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(phase) = phase {
        status.phase = phase;
    }
    if let Some(names) = names {
        status.names = names;
    }
}

fn run(
    config: Config,
    mut listener: Listener,
    responder: Option<Responder>,
    hash: [u8; 32],
    own: Loopback,
    shared: &Shared,
) -> Result<String> {
    let stopped = || shared.2.load(Ordering::Relaxed);
    let mut lobby = Lobby::new();
    lobby.join(Box::new(own));
    while !shared.1.load(Ordering::Relaxed) {
        if stopped() {
            return Ok("the host left the lobby".into());
        }
        while let Some(link) = listener.accept() {
            log::info!("lan: connection from {}", link.peer());
            lobby.join(Box::new(link));
        }
        lobby.poll();
        let names = lobby.names();
        if let Some(responder) = &responder {
            responder.answer(&Advert {
                protocol: PROTOCOL,
                port: listener.local_addr().port(),
                hash,
                name: config.name.clone(),
                title: config.params.title.clone(),
                track: config.track_label.clone(),
                people: u8::try_from(names.len()).unwrap_or(u8::MAX),
            });
        }
        set(shared, None, Some(names));
        std::thread::sleep(Duration::from_millis(5));
    }
    // The people who said hello by now are the race's people; a latecomer is refused.
    drop(responder);
    set(shared, Some(Phase::Loading), None);
    lobby.poll();
    let mut params = config.params.clone();
    params.people = u8::try_from(lobby.len().clamp(1, oag_gameplay::MAX_PLAYERS)).unwrap_or(1);
    let loaded = crate::race::load(&params, &config.source);
    let (race, _title) = match loaded {
        Ok(loaded) => loaded,
        Err(e) => {
            let reason = format!("the host could not load the race: {e:#}");
            for (_, mut link) in lobby.take() {
                link.send_control(&Control::Reject {
                    reason: reason.clone(),
                });
            }
            return Err(e.context("loading the hosted race"));
        }
    };
    let mut host = Host::new(race, params);
    log::info!("lan: fingerprint {:#018x}", host.fingerprint());
    for (name, link) in lobby.take() {
        if let Err(why) = host.seat(link, name.clone()) {
            log::warn!("lan: {name:?} not seated: {why}");
        }
    }
    let asked = Instant::now();
    while !host.all_ready() {
        if stopped() {
            host.end("the host stopped");
            return Ok("the host stopped".into());
        }
        while let Some(link) = listener.accept() {
            host.join(Box::new(link));
        }
        host.poll();
        // A seat that is refused (different content) or closes while the others load ends
        // the start: nobody is left waiting on a seat that will never be ready.
        if asked.elapsed() > LOAD_TIMEOUT || host.departed() {
            host.end("a player did not load the race");
            anyhow::bail!("a player did not load the race: {:?}", host.seat_names());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    set(shared, Some(Phase::Racing), None);
    serve::run(
        &mut host,
        Pace::default(),
        &mut || {
            listener
                .accept()
                .map(|link| Box::new(link) as Box<dyn Link>)
        },
        &stopped,
        &mut |line| log::info!("lan: {line}"),
    );
    Ok(if stopped() {
        "the host stopped".into()
    } else {
        "the race ended".into()
    })
}

impl std::fmt::Debug for Hosted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hosted").finish_non_exhaustive()
    }
}
