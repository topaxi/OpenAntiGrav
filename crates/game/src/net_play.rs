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
//! What this leaves alone: the menus (the LAN and ONLINE rows stay greyed), the
//! sound, the HUD and the camera, which follow this machine's seat
//! (`Race::set_seat`). Weapons are off in a network race until they are server
//! events.
//!
//! `--net-bot` flies this machine's craft with the player's own driver, sent as
//! input like a pad's: a verification aid for a whole race with nobody at the
//! keys. `--net-shim RTT[:LOSS]` adds a round trip and a loss rate on this end, so
//! a run on localhost can be judged at a real network's numbers (chosen, not
//! measured).

use std::cell::{Cell, RefCell};

use anyhow::{Result, anyhow};
use oag_gameplay::InputSnapshot;
use oag_net::client::{Client, Phase, Stats, Welcome};
use oag_net::wire::RaceParams;
use oag_raceplay::{Options, Race};
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
        let mut client = Client::connect(link, &args.name);
        let asked = Instant::now();
        let welcome = loop {
            if let Some(welcome) = client.poll_welcome()? {
                break welcome;
            }
            if asked.elapsed() > Duration::from_secs(10) {
                return Err(anyhow!("{} answered but sent no welcome", args.addr));
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        println!(
            "net: seat {} of {} on {}, {} on {}, seed {}, weapons {}",
            welcome.seat,
            welcome.params.people,
            args.addr,
            welcome.params.title,
            if welcome.params.track.is_empty() {
                "the title's default circuit"
            } else {
                &welcome.params.track
            },
            welcome.params.seed,
            if welcome.params.weapons { "on" } else { "off" },
        );
        Ok(Self {
            client: RefCell::new(client),
            welcome,
            bot: args.bot,
            due: Cell::new(None),
            log: RefCell::new((Stats::default(), Instant::now())),
            board_printed: Cell::new(false),
        })
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
            seat_picks: Vec::new(),
            laps_override: None,
            eliminator_kill_target: None,
            zone_stage: None,
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
        oag_net::race::check_title(&self.welcome.params, title)?;
        oag_net::race::prepare(race, &self.welcome.params)?;
        let mut client = self.client.borrow_mut();
        client.ready(race)?;
        println!(
            "net: ready at state {:#018x}, waiting for the start",
            oag_net::race::fingerprint(race)
        );
        let asked = Instant::now();
        loop {
            client.poll();
            match client.phase() {
                Phase::Racing => return Ok(()),
                Phase::Over(why) => return Err(anyhow!("the server ended the race: {why}")),
                _ => {}
            }
            if asked.elapsed() > START_TIMEOUT {
                return Err(anyhow!("no start within {START_TIMEOUT:?}"));
            }
            std::thread::sleep(Duration::from_millis(5));
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
        let ticks = self.client.borrow_mut().step(race, &own);
        self.log_second(race);
        if self.over().is_some() && !self.board_printed.replace(true) {
            print_board(race);
        }
        ticks
    }

    /// Steps on the wall clock, one step per tick of time, until the race reaches
    /// `tick` or the server ends it: `--screenshot`'s loop, which has no frame
    /// clock.
    pub fn drive_to(&self, race: &mut Race, tick: u64, device: InputSnapshot) {
        let dt = Duration::from_secs_f64(f64::from(oag_core::tick::TickRate::DEFAULT.dt()));
        while race.sim.world.tick < tick && self.over().is_none() {
            let due = self.due.get().unwrap_or_else(Instant::now);
            let now = Instant::now();
            if now < due {
                std::thread::sleep(due - now);
            }
            self.due.set(Some(due.max(now - dt) + dt));
            self.step(race, device);
        }
    }

    /// Why the race is over, once the server has ended it or the link has gone.
    #[must_use]
    pub fn over(&self) -> Option<String> {
        match self.client.borrow().phase() {
            Phase::Over(why) => Some(why.clone()),
            _ => None,
        }
    }

    /// Tells the server this machine is leaving.
    pub fn leave(&self, reason: &str) {
        self.client.borrow_mut().leave(reason);
    }

    /// One line a second, so a run can be judged from its log: corrections, the
    /// ticks they replayed, the largest pop, the confirmed state hash agreeing,
    /// the lead and the prediction window.
    fn log_second(&self, race: &Race) {
        let stats = self.client.borrow_mut().take_stats();
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
        sum.lead = stats.lead;
        sum.window = stats.window;
        if log.1.elapsed() < Duration::from_secs(1) {
            return;
        }
        let s = std::mem::take(&mut log.0);
        log.1 = Instant::now();
        println!(
            "net: tick {:>6} | predicted {} confirmed {} | hash ok {} bad {} | corrections {} \
             replayed {} largest pop {:.2} largest error {:.2} | lead {} window {} | \
             +{} -{} stalls {} resends {}",
            race.sim.world.tick,
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
        );
    }
}

/// The board as this client has it when the server ends the race, in the
/// server's own `board` lines' shape, so the two logs compare line for line.
fn print_board(race: &Race) {
    let Some(board) = race.results() else {
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
    }
}
