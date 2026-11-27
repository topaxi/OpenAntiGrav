//! `--connect`: this game as a client of an `oag-server`.
//!
//! The race is the server's: [`NetPlay::connect`] joins before anything loads,
//! [`NetPlay::race_options`] overwrites every option the simulation reads with the
//! server's, and [`NetPlay::join`] applies what a network race holds fixed
//! (`oag_net::race::prepare`), proves the race starts in the server's state (the
//! tick-0 hash) and waits for the start. From then on the race is stepped through
//! [`NetPlay::step`], which predicts, sends this machine's input and corrects on the
//! server's word (`oag_net::client`), instead of `Race::tick`.
//!
//! The menus' LAN page (`session::lan`) joins the same way a frame at a time:
//! [`Pending`] waits for the welcome, [`NetPlay::begin_join`] and
//! [`NetPlay::poll_start`] are [`NetPlay::join`]'s two halves, so the window never blocks.
//!
//! What this leaves alone: the sound, the HUD and the camera, which follow this
//! machine's seat (`Race::set_seat`). Weapons are off in a network race until they
//! are server events.
//!
//! `--net-bot` flies this machine's craft with the player's own driver, sent as
//! input like a pad's: a verification aid for a whole race with nobody at the
//! keys. `--net-shim RTT[:LOSS]` adds a round trip and a loss rate on this end, so
//! a run on localhost can be judged at a real network's numbers (chosen, not
//! measured).

pub mod reconnect;
pub mod rejoin;

use std::cell::{Cell, RefCell};

use anyhow::{Result, anyhow};
use oag_gameplay::InputSnapshot;
use oag_net::client::{Client, Phase, Stats, Welcome};
use oag_net::redial::{Dialer, Identity, Redial};
use oag_net::stats::{Collector, NetStats};
use oag_net::wire::RaceParams;
use oag_raceplay::{Options, Race};
use rejoin::Rejoin;
use web_time::{Duration, Instant};

/// How long [`NetPlay::join`] waits for the other seats to load. Chosen.
const START_TIMEOUT: Duration = Duration::from_secs(120);

/// What `--connect` and its companions asked for.
#[derive(Debug, Clone)]
pub struct Connect {
    /// `host:port`.
    pub addr: String,
    /// The server certificate's SHA-256, from `--cert-hash`.
    pub cert_hash: [u8; 32],
    /// `--net-shim`, if given.
    pub shim: Option<oag_net::link::Conditions>,
    /// `--net-bot`.
    pub bot: bool,
    /// How this client names itself in the server's log.
    pub name: String,
    /// The seat token an online lobby handed this person for this race ([`crate::online`]),
    /// empty for a plain `--connect`.
    pub token: String,
    /// Where the lobby's word arrives that the room closed under this race, for a race that
    /// came from a lobby.
    #[cfg(not(target_arch = "wasm32"))]
    pub lobby: Option<crate::online::LobbyWord>,
    /// What a race from a lobby is held to ([`Pending::guard`]): the hash of the race the lobby
    /// issued, all zero for none.
    #[cfg(not(target_arch = "wasm32"))]
    pub params: [u8; 32],
    /// A player host's public key, all zero for a server or a plain `--connect`.
    #[cfg(not(target_arch = "wasm32"))]
    pub host_key: [u8; 32],
    /// The profile's key, which signs this seat's inputs in a player-hosted race.
    #[cfg(not(target_arch = "wasm32"))]
    pub signer: Option<std::sync::Arc<oag_net::online::hostkey::PlayerKey>>,
}

/// A connection to a server and the client half of the protocol.
pub struct NetPlay {
    client: RefCell<Client>,
    welcome: Welcome,
    bot: bool,
    /// When the next paced step is due (`--screenshot`'s loop, which has no frame
    /// clock of its own).
    due: Cell<Option<Instant>>,
    /// The stats since the last log line, and when it was printed.
    log: RefCell<(Stats, Instant)>,
    /// Whether the board has been printed at the end.
    board_printed: Cell<bool>,
    /// The fingerprint of this client's recording of the finished race
    /// ([`oag_replay::Replay::fingerprint`]), taken once when the race is over.
    fingerprint: Cell<Option<u64>>,
    /// Whether this client's board has been compared with the server's.
    board_checked: Cell<bool>,
    /// The name of the person in each seat (an online lobby's room), given to the race so its
    /// end screens call people by name; empty for a plain `--connect`.
    names: RefCell<Vec<String>>,
    /// The sparkline and hash totals, and the last whole second the overlay reads.
    /// See [`oag_net::stats`].
    overlay: RefCell<(Collector, Option<NetStats>)>,
    /// The way back to the seat after the link closes; absent for a connection that holds no
    /// seat token, which nobody could be given back.
    redial: RefCell<Option<(Redial, Box<dyn Dialer>)>>,
    /// What the record of this seat holds ([`crate::rejoin_record`]).
    rejoin: Option<Rejoin>,
    record: RefCell<Record>,
    /// Confirmed ticks whose state hash matched the server's, and those whose did not, since
    /// the join.
    hashes: Cell<(u32, u32)>,
    /// The lobby said the room closed under this race, and when this client heard it.
    closed: RefCell<Option<(String, Instant)>>,
    #[cfg(not(target_arch = "wasm32"))]
    lobby: RefCell<Option<crate::online::LobbyWord>>,
}

/// How long an exiting `--online` host waits to tell the lobby how its race ended. Chosen: the
/// host thread lets the last ticks reach its seats for a second before it reports.
#[cfg(not(target_arch = "wasm32"))]
const REPORT_WAIT: Duration = Duration::from_secs(4);

/// How long after the lobby says the room closed the race is ended here if its host has not
/// ended it itself. Chosen: a host that leaves in order sends its own end and the final ticks
/// first, and those are the better ending.
const CLOSED_GRACE: Duration = Duration::from_secs(1);

/// Where the record of the seat is written and when it last was.
#[derive(Debug, Default)]
struct Record {
    /// A file other than the state directory's, for a test.
    file: Option<std::path::PathBuf>,
    last: Option<Instant>,
}

/// A connection that has said hello and is waiting for the server's welcome: the menus'
/// LAN screens poll it a frame at a time, where `--connect` blocks on it
/// ([`NetPlay::connect`]).
pub struct Pending {
    client: Option<Client>,
    bot: bool,
    /// Where it connected, for the log line.
    label: String,
    /// Who it is and where, for the redial; see [`NetPlay::redial`].
    seat: Option<(Rejoin, Option<oag_net::link::Conditions>)>,
    /// A dialler to use instead of the transport's, for a test.
    dialer: Option<Box<dyn Dialer>>,
}

impl Pending {
    /// Says hello over `link`, named `name`, with the seat `token` a lobby issued (empty
    /// otherwise). `bot` is `--net-bot`.
    #[must_use]
    pub fn new(
        link: Box<dyn oag_net::link::Link>,
        name: &str,
        team: &str,
        token: &str,
        bot: bool,
        label: &str,
        rejoin: Option<(Rejoin, Option<oag_net::link::Conditions>)>,
    ) -> Self {
        let seat = rejoin.filter(|_| !token.is_empty());
        Self {
            client: Some(Client::connect_seat(link, name, team, token)),
            bot,
            label: label.to_string(),
            seat,
            dialer: None,
        }
    }

    /// Holds the host to the race the lobby issued (`params`), to a player host's signed
    /// checkpoints (`host_key`, all zero for a server), and signs this seat's inputs with
    /// `signer` in a player-hosted race
    /// (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
    /// "The anti-cheat path").
    #[cfg(not(target_arch = "wasm32"))]
    pub fn guard(
        &mut self,
        params: [u8; 32],
        host_key: [u8; 32],
        signer: Option<std::sync::Arc<oag_net::online::hostkey::PlayerKey>>,
    ) {
        if let Some(client) = self.client.as_mut() {
            client.guard(params, host_key, signer);
        }
    }

    /// Redials with `dialer` instead of the transport's: a test with no socket.
    #[must_use]
    pub fn with_dialer(mut self, dialer: Box<dyn Dialer>) -> Self {
        self.dialer = Some(dialer);
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn dialer(rejoin: &Rejoin, shim: Option<oag_net::link::Conditions>) -> Option<Box<dyn Dialer>> {
        Some(Box::new(rejoin::WtDialer::new(rejoin, shim)))
    }

    #[cfg(target_arch = "wasm32")]
    fn dialer(_: &Rejoin, _: Option<oag_net::link::Conditions>) -> Option<Box<dyn Dialer>> {
        None
    }

    /// Reads the link once: the [`NetPlay`] once the welcome has come, `None` before.
    ///
    /// # Errors
    ///
    /// The server refused (it names why), or the link closed.
    pub fn poll(&mut self) -> Result<Option<NetPlay>> {
        let Some(client) = self.client.as_mut() else {
            return Ok(None);
        };
        let Some(welcome) = client.poll_welcome()? else {
            return Ok(None);
        };
        let client = self.client.take().expect("polled just above");
        let seat = self.seat.take();
        let given = self.dialer.take();
        let rejoin = seat.as_ref().map(|(rejoin, _)| rejoin.clone());
        let redial = seat.and_then(|(rejoin, shim)| {
            let dialer = given.or_else(|| Self::dialer(&rejoin, shim))?;
            let identity = Identity {
                name: rejoin.name,
                team: rejoin.team,
                token: rejoin.token,
            };
            Some((Redial::new(identity), dialer))
        });
        println!(
            "net: seat {} of {} on {}, {} on {}, seed {}, weapons {}",
            welcome.seat,
            welcome.params.people,
            self.label,
            welcome.params.title,
            if welcome.params.track.is_empty() {
                "the title's default circuit"
            } else {
                &welcome.params.track
            },
            welcome.params.seed,
            if welcome.params.weapons { "on" } else { "off" },
        );
        Ok(Some(NetPlay {
            client: RefCell::new(client),
            welcome,
            bot: self.bot,
            due: Cell::new(None),
            log: RefCell::new((Stats::default(), Instant::now())),
            board_printed: Cell::new(false),
            fingerprint: Cell::new(None),
            board_checked: Cell::new(false),
            names: RefCell::new(Vec::new()),
            overlay: RefCell::new((Collector::new(), None)),
            redial: RefCell::new(redial),
            rejoin,
            record: RefCell::new(Record::default()),
            hashes: Cell::new((0, 0)),
            closed: RefCell::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            lobby: RefCell::new(None),
        }))
    }
}

impl Drop for Pending {
    /// Backing out before the welcome tells the host this seat is gone.
    fn drop(&mut self) {
        if let Some(client) = self.client.as_mut() {
            client.leave("the player backed out");
        }
    }
}

impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pending")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for NetPlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetPlay")
            .field("welcome", &self.welcome)
            .finish_non_exhaustive()
    }
}

impl NetPlay {
    /// Connects and waits for the server's welcome: the seat and the race.
    ///
    /// # Errors
    ///
    /// The connection failed (a wrong `--cert-hash` names the TLS error), the
    /// server refused, or no welcome came within ten seconds.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn connect(args: &Connect) -> Result<Self> {
        let link = oag_net::wt::connect(&args.addr, args.cert_hash)?;
        let link: Box<dyn oag_net::link::Link> = match args.shim {
            Some(conditions) => Box::new(oag_net::link::Shim::new(link, conditions)),
            None => Box::new(link),
        };
        let rejoin = Rejoin {
            addr: args.addr.clone(),
            cert_hash: args.cert_hash,
            room: 0,
            team: String::new(),
            name: args.name.clone(),
            token: args.token.clone(),
        };
        let mut pending = Pending::new(
            link,
            &args.name,
            "",
            &args.token,
            args.bot,
            &args.addr,
            Some((rejoin, args.shim)),
        );
        pending.guard(args.params, args.host_key, args.signer.clone());
        let asked = Instant::now();
        loop {
            if let Some(net) = pending.poll()? {
                *net.lobby.borrow_mut() = args.lobby.clone();
                return Ok(net);
            }
            if asked.elapsed() > Duration::from_secs(10) {
                return Err(anyhow!("{} answered but sent no welcome", args.addr));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// The race the server is running.
    #[must_use]
    pub fn params(&self) -> &RaceParams {
        &self.welcome.params
    }

    /// The seat (grid slot) this machine flies.
    #[must_use]
    pub fn seat(&self) -> usize {
        self.welcome.seat
    }

    /// `options` with everything the simulation reads replaced by the server's:
    /// the presentation (`source`, the DLC, the render switches) stays this
    /// machine's own.
    ///
    /// # Errors
    ///
    /// A mode, difficulty or scheme this build does not know.
    pub fn race_options(&self, options: Options) -> Result<Options> {
        let server = oag_net::race::options(&self.welcome.params, &options.source)?;
        Ok(Options {
            track: server.track,
            team: server.team,
            class: server.class,
            mode: server.mode,
            difficulty: server.difficulty,
            seed: server.seed,
            people: server.people,
            views: 1,
            opponents: server.opponents,
            weapons_override: server.weapons_override,
            no_driver_for_class: server.no_driver_for_class,
            craft_source: None,
            foreign_track: None,
            hull_variant: None,
            skin: None,
            opponent_teams: Vec::new(),
            grid_teams: Vec::new(),
            seat_picks: server.seat_picks,
            laps_override: server.laps_override,
            eliminator_kill_target: None,
            eliminator_score_target: None,
            zone_stage: None,
            network: true,
            ..options
        })
    }

    /// Makes `race` this machine's copy of the server's (the fixed settings and
    /// this seat), proves it starts in the server's state, says it is ready, and
    /// waits for the start. Blocks until every seat has loaded.
    ///
    /// # Errors
    ///
    /// A different title, a different start state (different content or options),
    /// or no start within two minutes.
    pub fn join(&self, race: &mut Race, title: &str) -> Result<()> {
        self.begin_join(race, title)?;
        let asked = Instant::now();
        loop {
            if self.poll_start()? {
                return Ok(());
            }
            if asked.elapsed() > START_TIMEOUT {
                return Err(anyhow!("no start within {START_TIMEOUT:?}"));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// The first half of [`Self::join`]: makes `race` the server's, proves its start state
    /// and says this machine is ready. The menus' LAN races poll [`Self::poll_start`] a frame
    /// at a time after this instead of blocking the window.
    ///
    /// # Errors
    ///
    /// A different title or a different start state.
    pub fn begin_join(&self, race: &mut Race, title: &str) -> Result<()> {
        oag_net::race::check_title(&self.welcome.params, title)?;
        oag_net::race::prepare(race, &self.welcome.params)?;
        race.core.set_person_names(self.names.borrow().clone());
        let mut client = self.client.borrow_mut();
        client.ready(race)?;
        println!(
            "net: ready at state {:#018x}, waiting for the start",
            oag_net::race::fingerprint(race)
        );
        Ok(())
    }

    /// Reads the link once; whether the server has started the race.
    ///
    /// # Errors
    ///
    /// The server ended the race before it began.
    pub fn poll_start(&self) -> Result<bool> {
        let mut client = self.client.borrow_mut();
        client.poll();
        match client.phase() {
            Phase::Racing => {
                drop(client);
                self.save_record();
                Ok(true)
            }
            Phase::Over(why) => Err(anyhow!("the server ended the race: {why}")),
            Phase::Lost(why) if self.redial.borrow().is_none() => Err(anyhow!("{why}")),
            _ => Ok(false),
        }
    }

    /// This machine's input this tick: the device's, or the driver's under
    /// `--net-bot`.
    pub fn own_input(&self, race: &mut Race, device: InputSnapshot) -> InputSnapshot {
        if self.bot {
            self.client.borrow_mut().bot_input(race)
        } else {
            device
        }
    }

    /// One step of the frame clock: see `oag_net::client::Client::step`. Returns
    /// how many ticks the race advanced (0, 1 or 2), each a drawn `Race::tick`.
    pub fn step(&self, race: &mut Race, device: InputSnapshot) -> u32 {
        let own = self.own_input(race, device);
        self.heed_lobby();
        self.redial_step();
        let ticks = self.client.borrow_mut().step(race, &own);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(word) = self.lobby.borrow().as_ref()
            && let Some(proof) = self.take_report()
        {
            word.reports
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(proof);
        }
        self.keep_record();
        self.log_second(race);
        if self.over().is_some() && !self.board_printed.replace(true) {
            print_board(race);
            self.fingerprint
                .set(race.recorded().map(|run| run.fingerprint()));
            if let Some(fingerprint) = self.fingerprint.get() {
                log::debug!("net: this client's recording of the race is {fingerprint:#018x}");
            }
        }
        self.check_server_board(race);
        ticks
    }

    /// The case against a player host this client found, once (the lobby judges it).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn take_report(&self) -> Option<oag_net::fair::Proof> {
        let proof = self.client.borrow_mut().take_report()?;
        println!(
            "net: the host's race does not hold up ({:?} up to tick {}): reporting it to the lobby",
            proof.claim, proof.checkpoint.ticks
        );
        Some(proof)
    }

    /// The lobby says the room closed under this race (its host left, or was banned): the race
    /// ends here with `reason` unless its host ends it first.
    pub fn lobby_closed(&self, reason: &str) {
        self.closed
            .borrow_mut()
            .get_or_insert_with(|| (reason.to_string(), Instant::now()));
    }

    /// Ends the race on the lobby's word, once the host has had its moment to end it itself.
    fn heed_lobby(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(word) = self.lobby.borrow().as_ref()
            && let Some(reason) = word
                .closed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
        {
            self.lobby_closed(&reason);
        }
        let closed = self.closed.borrow();
        let Some((reason, heard)) = closed.as_ref() else {
            return;
        };
        let mut client = self.client.borrow_mut();
        if heard.elapsed() >= CLOSED_GRACE && !matches!(client.phase(), Phase::Over(_)) {
            log::warn!("net: the lobby closed the room: {reason}");
            client.give_up(reason);
        }
    }

    /// Dials again when the link has closed under the race ([`oag_net::redial`]).
    fn redial_step(&self) {
        let mut redial = self.redial.borrow_mut();
        let Some((redial, dialer)) = redial.as_mut() else {
            return;
        };
        let was_lost = matches!(self.client.borrow().phase(), Phase::Lost(_));
        redial.step(
            &mut self.client.borrow_mut(),
            dialer.as_mut(),
            Instant::now(),
        );
        if was_lost && redial.attempts() == 1 {
            log::debug!("net: the link closed; dialling again");
        }
    }

    /// Points the record of the seat at `file`, where a test reads it back.
    pub fn record_in(&self, file: std::path::PathBuf) {
        self.record.borrow_mut().file = Some(file);
    }

    fn record_file(&self) -> Option<std::path::PathBuf> {
        self.record
            .borrow()
            .file
            .clone()
            .or_else(crate::rejoin_record::path)
    }

    /// Writes the record of the seat: where the race is and the token that takes the seat back.
    fn save_record(&self) {
        let (Some(rejoin), Some(file)) = (&self.rejoin, self.record_file()) else {
            return;
        };
        match crate::rejoin_record::store_to(
            &file,
            &self.welcome.params.title,
            rejoin,
            crate::rejoin_record::now(),
        ) {
            Ok(()) => self.record.borrow_mut().last = Some(Instant::now()),
            Err(e) => log::warn!("net: could not keep the seat for a rejoin: {e:#}"),
        }
    }

    /// Refreshes the record while the race runs, and deletes it once the race is over.
    fn keep_record(&self) {
        if self.rejoin.is_none() {
            return;
        }
        if self.over().is_some() {
            self.clear_record();
        } else if self.record.borrow().last.is_some_and(|at| {
            at.elapsed() >= Duration::from_secs(crate::rejoin_record::REFRESH_SECONDS)
        }) {
            self.save_record();
        }
    }

    /// Deletes the record: the race is over or this person left it.
    pub fn clear_record(&self) {
        if self.rejoin.is_some()
            && self.record.borrow_mut().last.take().is_some()
            && let Some(file) = self.record_file()
        {
            crate::rejoin_record::clear_at(&file);
        }
    }

    /// Whether the link is gone and this machine is dialling for its seat: the race stands
    /// still meanwhile.
    #[must_use]
    pub fn reconnecting(&self) -> bool {
        self.redial.borrow().is_some()
            && matches!(
                self.client.borrow().phase(),
                Phase::Lost(_) | Phase::Reconnecting
            )
    }

    /// Whole seconds of the redial's patience left, while the game is waiting to get its seat
    /// back (what the RECONNECTING overlay counts down).
    #[must_use]
    pub fn reconnect_seconds_left(&self) -> Option<u32> {
        if !self.reconnecting() {
            return None;
        }
        let left = self.redial.borrow().as_ref()?.0.left(Instant::now())?;
        Some(u32::try_from(left.as_secs() + u64::from(left.subsec_nanos() > 0)).unwrap_or(0))
    }

    /// The player leaves a race whose link is gone: the race is over for this client at once.
    pub fn give_up(&self) {
        self.client
            .borrow_mut()
            .give_up("the player left while reconnecting");
    }

    /// Names the person in each seat, by seat, before [`Self::begin_join`].
    pub fn set_names(&self, names: Vec<String>) {
        *self.names.borrow_mut() = names;
    }

    /// The server's final board, once its `End` carried one.
    #[must_use]
    pub fn server_board(&self) -> Option<oag_net::wire::board::Board> {
        self.client.borrow().final_board().cloned()
    }

    /// Compares this client's board with the server's, once both exist, and says which. They
    /// are the same race (the tick hash proves the state), so a difference is a bug.
    fn check_server_board(&self, race: &Race) {
        if self.board_checked.get() || !race.core.sim.finished() || race.core.results().is_none() {
            return;
        }
        let Some(theirs) = self.server_board() else {
            return;
        };
        self.board_checked.set(true);
        let ours = oag_net::race::final_board(race, &[]);
        if ours.same_race_as(&theirs) {
            println!(
                "net: the server's board matches this client's ({} rows)",
                ours.rows.len()
            );
        } else {
            log::error!(
                "net: the server's board differs from this client's: {theirs:?} against {ours:?}"
            );
            println!("net: the server's board DIFFERS from this client's");
        }
    }

    /// Steps on the wall clock, one step per tick of time, until the race reaches
    /// `tick` or the server ends it: `--screenshot`'s loop, which has no frame
    /// clock.
    pub fn drive_to(&self, race: &mut Race, tick: u64, device: InputSnapshot) {
        let dt = Duration::from_secs_f64(f64::from(oag_core::tick::TickRate::DEFAULT.dt()));
        while race.core.sim.world.tick < tick && self.over().is_none() {
            let due = self.due.get().unwrap_or_else(Instant::now);
            let now = Instant::now();
            if now < due {
                std::thread::sleep(due - now);
            }
            self.due.set(Some(due.max(now - dt) + dt));
            self.step(race, device);
        }
    }

    /// The `options` table a recording of this room's race carries: what `prepare` applied, so
    /// the file rebuilds as the room's race and not a local one.
    ///
    /// # Errors
    ///
    /// A difficulty or control scheme this build does not know.
    pub fn replay_options(&self) -> Result<std::collections::BTreeMap<String, String>> {
        oag_net::race::replay_options(&self.welcome.params)
    }

    /// The fingerprint of this client's recording of the finished race, kept locally: equal to
    /// the server's own once the server sends it, so the replay is the real run (the
    /// server-confirmed ticks, which is all a predicting client records).
    #[must_use]
    pub fn replay_fingerprint(&self) -> Option<u64> {
        self.fingerprint.get()
    }

    /// How many confirmed ticks matched the server's state hash, and how many did not: the
    /// second is always a bug or different content.
    #[must_use]
    pub fn hashes(&self) -> (u32, u32) {
        self.hashes.get()
    }

    /// Why the race is over, once the server has ended it or the link has gone.
    #[must_use]
    pub fn over(&self) -> Option<String> {
        match self.client.borrow().phase() {
            Phase::Over(why) => Some(why.clone()),
            Phase::Lost(why) if self.redial.borrow().is_none() => Some(why.clone()),
            _ => None,
        }
    }

    /// Tells the server this machine is leaving.
    pub fn leave(&self, reason: &str) {
        self.client.borrow_mut().leave(reason);
    }

    /// The network block of the `Dev` performance overlay: the last whole second's
    /// figures (the ones the log line printed) and the live sparkline. `None` before
    /// the first second has passed.
    #[must_use]
    pub fn readout(&self) -> Option<oag_present::perf::net::Readout> {
        let overlay = self.overlay.borrow();
        let net = overlay.1.as_ref()?;
        Some(oag_present::perf::net::Readout {
            rtt_ms: net.rtt_ms,
            confirm_ms: net.confirm_ms,
            jitter_ms: net.jitter_ms,
            rx_bytes_s: net.rx_bytes_s,
            tx_bytes_s: net.tx_bytes_s,
            rx_datagrams_s: net.rx_datagrams_s,
            tx_datagrams_s: net.tx_datagrams_s,
            loss_percent: net.loss_percent,
            cwnd: net.cwnd,
            lead: net.lead,
            window: net.window,
            server_tick: net.server_tick,
            local_tick: net.local_tick,
            corrections: net.corrections,
            replayed: net.replayed,
            largest_pop: net.largest_pop,
            largest_error: net.largest_error,
            hash_ok: net.hash_ok,
            hash_bad: net.hash_bad,
            stalls: net.stalls,
            resends: net.resends,
            shim: net.shim.map(|s| {
                (
                    s.conditions.one_way.as_secs_f32() * 2000.0,
                    s.conditions.loss * 100.0,
                    s.dropped,
                )
            }),
            history: overlay.0.history().collect(),
        })
    }

    /// One line a second, so a run can be judged from its log: corrections, the
    /// ticks they replayed, the largest pop, the confirmed state hash agreeing,
    /// the lead and the prediction window, and how late a shot first showed.
    fn log_second(&self, race: &Race) {
        let stats = self.client.borrow_mut().take_stats();
        let (ok, bad) = self.hashes.get();
        self.hashes
            .set((ok + stats.hash_matches, bad + stats.hash_mismatches));
        let mut log = self.log.borrow_mut();
        let sum = &mut log.0;
        sum.predicted += stats.predicted;
        sum.confirmed += stats.confirmed;
        sum.hash_matches += stats.hash_matches;
        sum.hash_mismatches += stats.hash_mismatches;
        sum.corrections += stats.corrections;
        sum.replayed += stats.replayed;
        sum.largest_pop = sum.largest_pop.max(stats.largest_pop);
        sum.largest_error = sum.largest_error.max(stats.largest_error);
        sum.stalls += stats.stalls;
        sum.sped_up += stats.sped_up;
        sum.slowed_down += stats.slowed_down;
        sum.resends += stats.resends;
        sum.confirm_sum_ms += stats.confirm_sum_ms;
        sum.confirm_count += stats.confirm_count;
        sum.jitter_sum_ms += stats.jitter_sum_ms;
        sum.jitter_count += stats.jitter_count;
        sum.first_seen += stats.first_seen;
        sum.first_seen_latest = sum.first_seen_latest.max(stats.first_seen_latest);
        sum.restored += stats.restored;
        sum.restored_gap_max = sum.restored_gap_max.max(stats.restored_gap_max);
        self.overlay.borrow_mut().0.step(&stats);
        sum.lead = stats.lead;
        sum.window = stats.window;
        if log.1.elapsed() < Duration::from_secs(1) {
            return;
        }
        let s = std::mem::take(&mut log.0);
        let seconds = log.1.elapsed().as_secs_f32();
        log.1 = Instant::now();
        let (srv, local) = (
            self.client.borrow().confirmed_tick(),
            u32::try_from(race.core.sim.world.tick).unwrap_or(u32::MAX),
        );
        let path = self.client.borrow().path();
        let net = {
            let mut overlay = self.overlay.borrow_mut();
            let net = overlay.0.second(&s, path, seconds, srv, local);
            overlay.1 = Some(net.clone());
            net
        };
        println!(
            "net: tick {:>6} | predicted {} confirmed {} | hash ok {} bad {} | corrections {} \
             replayed {} largest pop {:.2} largest error {:.2} | lead {} window {} | \
             +{} -{} stalls {} resends {} | quic rtt {} ms confirm {} ms jitter {} ms \
             rx {:.0} B/s {:.0} d/s tx {:.0} B/s {:.0} d/s loss {} | \
             shots first seen {} oldest {} ticks, put back {} after {} ticks",
            race.core.sim.world.tick,
            s.predicted,
            s.confirmed,
            s.hash_matches,
            s.hash_mismatches,
            s.corrections,
            s.replayed,
            s.largest_pop,
            s.largest_error,
            s.lead,
            s.window,
            s.sped_up,
            s.slowed_down,
            s.stalls,
            s.resends,
            fmt_ms(net.rtt_ms),
            fmt_ms(net.confirm_ms),
            fmt_ms(net.jitter_ms),
            net.rx_bytes_s,
            net.rx_datagrams_s,
            net.tx_bytes_s,
            net.tx_datagrams_s,
            net.loss_percent
                .map_or_else(|| "-".to_string(), |l| format!("{l:.1}%")),
            s.first_seen,
            s.first_seen_latest,
            s.restored,
            s.restored_gap_max,
        );
    }
}

/// The network block of the `Dev` overlay into a capture, which has no frame loop to
/// draw the rest of it: the panel at the top right of `view`, the way the window
/// places it when nothing is above it.
///
/// # Errors
///
/// The overlay renderer did not build.
pub fn draw_overlay(
    net: &NetPlay,
    gpu: (&wgpu::Device, &wgpu::Queue, wgpu::TextureFormat),
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    size: (u32, u32),
) -> Result<()> {
    let readout = net.readout();
    crate::touch_controls::overlay_pass(gpu, encoder, view, size, |_, space| {
        oag_present::perf::net::draw_list(readout.as_ref(), space.size.0, 0.0).0
    })
}

fn fmt_ms(value: Option<f32>) -> String {
    value.map_or_else(|| "-".to_string(), |v| format!("{v:.1}"))
}

/// The board as this client has it when the server ends the race, in the
/// server's own `board` lines' shape, so the two logs compare line for line.
fn print_board(race: &Race) {
    let Some(board) = race.core.results() else {
        println!("net: the race ended with no board on this client");
        return;
    };
    for (place, row) in board.rows.iter().enumerate() {
        println!(
            "board {}: slot {}{} laps {} finish tick {:?} best lap ticks {:?}",
            place + 1,
            row.slot,
            if row.player { " (person)" } else { "" },
            row.laps_completed,
            row.finish_tick,
            row.best_lap_ticks
        );
    }
}

impl Drop for NetPlay {
    /// A client that goes says so, so the server stops waiting on its seat.
    fn drop(&mut self) {
        if self.over().is_none() {
            self.leave("the client closed");
        }
        // A `--online` run that hosted the race is about to exit: the lobby is told how the
        // race ended first, so the room gets its board instead of losing its host.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(word) = self.lobby.borrow().as_ref() {
            use std::sync::atomic::Ordering;
            let asked = Instant::now();
            let hosted = word.unreported.load(Ordering::Relaxed);
            while word.unreported.load(Ordering::Relaxed) && asked.elapsed() < REPORT_WAIT {
                std::thread::sleep(Duration::from_millis(20));
            }
            if hosted {
                // The report is queued on the lobby link; give it the moment to leave.
                std::thread::sleep(Duration::from_millis(300));
            }
        }
        // A game that is going down on a panic keeps the record: it is what the next one
        // rejoins from.
        if !std::thread::panicking() {
            self.clear_record();
        }
    }
}
