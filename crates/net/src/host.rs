//! The authoritative side: the race the server runs, and what it does with each
//! person's input.
//!
//! [`Host`] holds the canonical race and one [`Seat`] per person. Every tick it
//! takes each seat's input for that tick out of the seat's buffer, **repeats the
//! seat's last input when there is none** (late, lost, or the person stopped
//! sending), derives the press and release edges from the held stream itself
//! ([`Held::snapshot`]), runs the tick, and sends every seat the confirmed record:
//! what it applied to each person and the state hash it arrived at
//! (ADR-0069, decision 3; `docs/networking/README.md`, decision 2). An input that
//! arrives for a tick already run is dropped, never applied late.
//!
//! It knows no clock and no socket: the server binary paces it at 60 Hz on the
//! wall clock and feeds it links, and a test drives it over [`crate::link::Loopback`].
//!
//! # Chosen, not measured
//!
//! - [`REDUNDANCY`]: how many of the newest confirmed ticks each datagram repeats.
//! - [`RESEND_CHUNK`]: how many ticks a resend carries per message.

use std::collections::BTreeMap;
use web_time::{Duration, Instant};

use oag_gameplay::input::Input;
use oag_gameplay::{MAX_PLAYERS, PlayerInputs};
use oag_raceplay::Race;

use crate::limits::LogLimit;
use crate::link::Link;
use crate::wire::board::Board;
use crate::wire::{Confirmed, Control, Datagram, Held, PROTOCOL, RaceParams, TickRecord};

/// The newest confirmed ticks each server datagram carries, at most (fewer when
/// they would not fit the link's datagram). Chosen, not measured: 16 ticks is
/// 267 ms of datagrams lost in a row before a client has to ask for a resend.
pub const REDUNDANCY: usize = 16;

/// Ticks per resend message. Chosen.
pub const RESEND_CHUNK: usize = 256;

/// How many ticks past the next one a seat's input may be for before it is dropped, so a seat
/// buffers fewer than 120 inputs. Chosen: two seconds, far beyond any client's lead.
pub const MAX_INPUT_AHEAD: u32 = 119;

/// The least time between two resend requests a seat is answered. Chosen.
pub const RESEND_INTERVAL: Duration = Duration::from_secs(1);

/// The most connections a race host keeps waiting for their hello; past it the oldest is
/// closed. Chosen, not measured: four times the seats of a full race
/// (`docs/networking/server-roadmap.md`, "V1: survive the network", the memory arithmetic).
pub const MAX_JOINING: usize = 48;

/// How long a connection may go without saying hello before it is closed. Chosen.
pub const HELLO_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a person with a seat in the running race keeps it after their link closed: the
/// lobby keeps their session that long so a reconnect under the session token can still take it
/// over and be handed the race's address, and the race host keeps the seat to hand it back
/// ([`Host::poll`]) and does not end the race for want of people before it is over.
/// A close usually reaches the lobby before the new link's hello does. Chosen.
pub const SEAT_GRACE: Duration = Duration::from_secs(30);

/// How long a seat that came back to a running race may take to load it again and say it is
/// ready before [`Host::drop_silent`] gives it up. Chosen, not measured.
pub const RELOAD_GRACE: Duration = Duration::from_secs(60);

/// One person's connection and input stream.
pub struct Seat {
    link: Box<dyn Link>,
    name: String,
    ready: bool,
    /// Inputs that arrived for ticks not yet run, by tick.
    inputs: BTreeMap<u32, Held>,
    /// The input applied last, repeated when the buffer has nothing for a tick.
    last: Held,
    /// The edge state the held stream has advanced, the server's own.
    edges: Input,
    /// The newest tick an input has arrived for.
    newest: Option<u32>,
    /// Why the seat left, once it has, and when.
    left: Option<String>,
    left_at: Option<Instant>,
    /// When anything last arrived from the seat.
    heard: Instant,
    /// When a resend request was last answered.
    resent: Option<Instant>,
    counts: SeatCounts,
    /// One warning a second about this seat.
    log: LogLimit,
}

impl Seat {
    /// The link is gone; the person has [`SEAT_GRACE`] to come back.
    fn leave(&mut self, why: String) {
        self.left = Some(why);
        self.left_at = Some(Instant::now());
    }

    /// The person quit or was refused: the grace is spent already.
    fn leave_for_good(&mut self, why: String) {
        self.leave(why);
        self.left_at = Instant::now().checked_sub(SEAT_GRACE).or(self.left_at);
    }
}

/// What happened to one seat's input since [`Host::take_counts`] was last asked.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SeatCounts {
    /// Ticks the seat's input was there in time.
    pub fresh: u32,
    /// Ticks the buffer was empty and the last input was repeated.
    pub repeated: u32,
    /// Inputs that arrived for a tick already run, dropped (each counted once).
    pub late: u32,
    /// The newest buffered tick less the tick about to run, at the last tick.
    pub lead: i32,
}

/// Where the race is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for every seat to join and load.
    Lobby,
    /// Ticking.
    Racing,
    /// Over: [`Host::end`] was called.
    Over,
}

/// The authoritative race and its seats.
pub struct Host {
    race: Box<Race>,
    params: RaceParams,
    fingerprint: u64,
    seats: Vec<Option<Seat>>,
    joining: Vec<(Box<dyn Link>, Instant)>,
    hello_timeout: Duration,
    /// One warning a second about refused connections.
    refusals: LogLimit,
    history: Vec<TickRecord>,
    phase: Phase,
    /// Seat tokens a lobby issued, by seat: when set, a hello is seated by its token.
    expected: Vec<(String, String)>,
}

impl Host {
    /// A host for `race`, already prepared ([`crate::race::prepare`]), seating
    /// `params.people` people.
    #[must_use]
    pub fn new(race: Box<Race>, params: RaceParams) -> Self {
        let fingerprint = crate::race::fingerprint(&race);
        let people = usize::from(params.people).clamp(1, MAX_PLAYERS);
        Self {
            race,
            params,
            fingerprint,
            seats: (0..people).map(|_| None).collect(),
            joining: Vec::new(),
            hello_timeout: HELLO_TIMEOUT,
            refusals: LogLimit::default(),
            history: Vec::new(),
            phase: Phase::Lobby,
            expected: Vec::new(),
        }
    }

    /// Seats only the people a lobby named: `seats[i]` is the (name, token) of seat `i`,
    /// and a hello is seated where its token says whatever order the clients arrive in.
    /// A hello without a listed token is refused by name.
    pub fn expect_seats(&mut self, seats: Vec<(String, String)>) {
        self.seats = (0..seats.len().clamp(1, MAX_PLAYERS))
            .map(|_| None)
            .collect();
        self.expected = seats;
    }

    /// The tick-0 hash every client must reproduce.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    /// The race, for the server's log.
    #[must_use]
    pub fn race(&self) -> &Race {
        &self.race
    }

    /// Where the race is.
    #[must_use]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The next tick to run.
    #[must_use]
    pub fn tick_number(&self) -> u32 {
        u32::try_from(self.history.len()).unwrap_or(u32::MAX)
    }

    /// The confirmed ticks so far, index = tick.
    #[must_use]
    pub fn history(&self) -> &[TickRecord] {
        &self.history
    }

    /// How many inputs are buffered for `slot`'s future ticks.
    #[must_use]
    pub fn buffered_inputs(&self, slot: usize) -> usize {
        self.seats
            .get(slot)
            .and_then(Option::as_ref)
            .map_or(0, |seat| seat.inputs.len())
    }

    /// A new connection. It is seated when its hello arrives.
    pub fn join(&mut self, link: Box<dyn Link>) {
        if self.joining.len() >= MAX_JOINING {
            self.joining.remove(0);
        }
        self.joining.push((link, Instant::now()));
    }

    /// Connections that have not yet said hello.
    #[must_use]
    pub fn joining(&self) -> usize {
        self.joining.len()
    }

    /// Sets how long a connection may stay silent before its hello ([`HELLO_TIMEOUT`]).
    pub fn set_hello_timeout(&mut self, timeout: Duration) {
        self.hello_timeout = timeout;
    }

    /// Whether every seat has joined and loaded the race.
    #[must_use]
    pub fn all_ready(&self) -> bool {
        self.seats
            .iter()
            .all(|seat| seat.as_ref().is_some_and(|seat| seat.ready))
    }

    /// The seats' names, in slot order, `None` for an empty one.
    #[must_use]
    pub fn seat_names(&self) -> Vec<Option<String>> {
        self.seats
            .iter()
            .map(|seat| seat.as_ref().map(|seat| seat.name.clone()))
            .collect()
    }

    /// Reads every link: hellos and readiness in the lobby, inputs and resend
    /// requests while racing.
    pub fn poll(&mut self) {
        self.poll_joining();
        let next = self.tick_number();
        let fingerprint = self.fingerprint;
        let racing = self.phase == Phase::Racing;
        for (slot, seat) in self.seats.iter_mut().enumerate() {
            let Some(seat) = seat.as_mut() else { continue };
            if seat.left.is_some() {
                continue;
            }
            if let Some(why) = seat.link.closed() {
                log::warn!("net: seat {slot} ({}) left: {why}", seat.name);
                seat.leave(why);
                continue;
            }
            while let Some(message) = seat.link.recv_control() {
                match message {
                    Ok(Control::Ready {
                        fingerprint: theirs,
                    }) if theirs == fingerprint => {
                        log::info!("net: seat {slot} ({}) is ready", seat.name);
                        seat.ready = true;
                        // A person who came back to a running race has missed its start.
                        if racing {
                            seat.link.send_control(&Control::Start { lead_ticks: 0 });
                        }
                    }
                    Ok(Control::Ready {
                        fingerprint: theirs,
                    }) => {
                        let reason = format!(
                            "your race starts at state {theirs:#018x} and the server's at \
                             {fingerprint:#018x}: different content or options"
                        );
                        log::warn!("net: seat {slot} ({}) refused: {reason}", seat.name);
                        seat.link.send_control(&Control::Reject {
                            reason: reason.clone(),
                        });
                        seat.leave_for_good(reason);
                    }
                    Ok(Control::Resend { .. })
                        if seat.resent.is_some_and(|at| at.elapsed() < RESEND_INTERVAL) => {}
                    Ok(Control::Resend { from }) => {
                        seat.resent = Some(Instant::now());
                        let from = from as usize;
                        let ticks = self.history.get(from..).unwrap_or(&[]);
                        for (n, chunk) in ticks.chunks(RESEND_CHUNK).enumerate() {
                            seat.link.send_control(&Control::Ticks(Confirmed {
                                first: u32::try_from(from + n * RESEND_CHUNK).unwrap_or(u32::MAX),
                                ticks: chunk.to_vec(),
                            }));
                        }
                    }
                    Ok(Control::End { reason, .. }) => {
                        log::info!("net: seat {slot} ({}) left: {reason}", seat.name);
                        seat.leave_for_good(reason);
                    }
                    Ok(other) => seat
                        .log
                        .warn(|| format!("net: seat {slot}: unexpected {other:?}")),
                    Err(e) => seat.log.warn(|| format!("net: seat {slot}: {e}")),
                }
            }
            while let Some(message) = seat.link.recv() {
                match message {
                    Ok(Datagram::Inputs {
                        seat: from,
                        first,
                        inputs,
                    }) if usize::from(from) == slot => {
                        seat.heard = Instant::now();
                        for (i, held) in inputs.into_iter().enumerate() {
                            let tick = first.saturating_add(u32::try_from(i).unwrap_or(u32::MAX));
                            if tick < next {
                                // Already run, or already counted late.
                                if seat.newest.is_none_or(|newest| tick > newest) {
                                    seat.counts.late += 1;
                                    seat.newest = Some(tick);
                                }
                                continue;
                            }
                            if tick - next >= MAX_INPUT_AHEAD {
                                continue;
                            }
                            seat.inputs.insert(tick, held);
                            seat.newest = Some(seat.newest.map_or(tick, |n| n.max(tick)));
                        }
                    }
                    Ok(other) => seat
                        .log
                        .warn(|| format!("net: seat {slot}: unexpected {other:?}")),
                    Err(e) => seat.log.warn(|| format!("net: seat {slot}: {e}")),
                }
            }
        }
    }

    fn poll_joining(&mut self) {
        let mut still = Vec::new();
        for (mut link, since) in std::mem::take(&mut self.joining) {
            let Some(message) = link.recv_control() else {
                if link.closed().is_none() && since.elapsed() < self.hello_timeout {
                    still.push((link, since));
                }
                continue;
            };
            let reason = match message {
                Ok(Control::Hello {
                    protocol,
                    name,
                    token,
                    ..
                }) if protocol == PROTOCOL => {
                    // `seat_at` has already sent a refusal its reason by name.
                    let placed = if self.expected.is_empty() {
                        self.seat(link, name)
                    } else if let Some(slot) = self
                        .expected
                        .iter()
                        .position(|(_, wanted)| !token.is_empty() && *wanted == token)
                    {
                        let name = self.expected[slot].0.clone();
                        if self.seat_has_left(slot) || self.seat_is_held_by_a_dead_link(slot) {
                            self.reseat(slot, link)
                        } else {
                            self.seat_at(Some(slot), link, name)
                        }
                    } else {
                        link.send_control(&Control::Reject {
                            reason: "this race seats the people its lobby named; \
                                     your seat token is not one of them"
                                .into(),
                        });
                        Err("unknown seat token".to_string())
                    };
                    if let Err(why) = placed {
                        self.refusals
                            .warn(|| format!("net: refused a connection: {why}"));
                    }
                    continue;
                }
                Ok(Control::Hello { protocol, .. }) => {
                    format!("this server speaks protocol {PROTOCOL}, you speak {protocol}")
                }
                Ok(other) => format!("expected a hello, got {other:?}"),
                Err(e) => format!("{e}"),
            };
            self.refusals
                .warn(|| format!("net: refused a connection: {reason}"));
            link.send_control(&Control::Reject { reason });
        }
        self.joining = still;
    }

    /// Seats `link`, whose hello (`name`) the caller has already read, in the first empty
    /// seat and welcomes it. A lobby that holds its welcomes until the head count is known
    /// ([`crate::lobby::Lobby`]) seats through this; [`Self::poll`] does the same for a link
    /// that says hello to the host itself. A link that cannot be seated is sent a reject
    /// naming why, and `Err` says the same.
    ///
    /// # Errors
    ///
    /// The race has started, or every seat is taken.
    pub fn seat(&mut self, link: Box<dyn Link>, name: String) -> Result<usize, String> {
        self.seat_at(None, link, name)
    }

    /// [`Self::seat`] into `slot` when given (it must be empty), else the first empty one.
    ///
    /// # Errors
    ///
    /// The race has started, or the seat is taken.
    pub fn seat_at(
        &mut self,
        slot: Option<usize>,
        mut link: Box<dyn Link>,
        name: String,
    ) -> Result<usize, String> {
        let free = match slot {
            Some(slot) => self
                .seats
                .get(slot)
                .is_some_and(Option::is_none)
                .then_some(slot),
            None => self.seats.iter().position(Option::is_none),
        };
        let reason = match free {
            Some(slot) if self.phase == Phase::Lobby => {
                log::info!("net: {name:?} takes seat {slot}");
                link.send_control(&Control::Welcome {
                    seat: u8::try_from(slot).unwrap_or(0),
                    params: self.params.clone(),
                    fingerprint: self.fingerprint,
                });
                self.seats[slot] = Some(Seat {
                    link,
                    name,
                    ready: false,
                    inputs: BTreeMap::new(),
                    last: Held::default(),
                    edges: Input::new(),
                    newest: None,
                    left: None,
                    left_at: None,
                    heard: Instant::now(),
                    resent: None,
                    log: LogLimit::default(),
                    counts: SeatCounts::default(),
                });
                return Ok(slot);
            }
            Some(_) => "the race has started".to_string(),
            None => "every seat is taken".to_string(),
        };
        link.send_control(&Control::Reject {
            reason: reason.clone(),
        });
        Err(reason)
    }

    /// Whether seat `slot` was taken and its person has since left, in a race that is running.
    fn seat_has_left(&self, slot: usize) -> bool {
        self.phase == Phase::Racing
            && self
                .seats
                .get(slot)
                .and_then(Option::as_ref)
                .is_some_and(|seat| seat.left.is_some())
    }

    /// Whether seat `slot` is taken in a race that is running and has not been given up, so a
    /// token arriving for it is its person coming back before the host noticed the old link was
    /// dead (a killed game sends no close frame; [`Self::drop_silent`] waits seconds, QUIC's
    /// idle timeout longer). The token is the proof of who it is, so the new link replaces the
    /// old one; the race never saw the seat leave.
    fn seat_is_held_by_a_dead_link(&self, slot: usize) -> bool {
        self.phase == Phase::Racing
            && self
                .seats
                .get(slot)
                .and_then(Option::as_ref)
                .is_some_and(|seat| seat.left.is_none())
    }

    /// Seats a returning person's new `link` in seat `slot`, which they left (or whose old
    /// link is still held, [`Self::seat_is_held_by_a_dead_link`]): the welcome
    /// again, then the seat counts as present from the next tick ([`TickRecord::retired`]
    /// clears and the craft is handed back). The seat keeps its held input and edge state, so
    /// the press and release edges go on from where the person's last input left them; what
    /// was buffered or counted as late for the gap is dropped. The client asks for the ticks
    /// it lacks ([`Control::Resend`]) and says it is ready, which is answered with the start.
    fn reseat(&mut self, slot: usize, mut link: Box<dyn Link>) -> Result<usize, String> {
        let Some(seat) = self.seats.get_mut(slot).and_then(Option::as_mut) else {
            return Err("every seat is taken".to_string());
        };
        log::info!("net: {:?} returns to seat {slot}", seat.name);
        link.send_control(&Control::Welcome {
            seat: u8::try_from(slot).unwrap_or(0),
            params: self.params.clone(),
            fingerprint: self.fingerprint,
        });
        seat.link = link;
        seat.left = None;
        seat.left_at = None;
        seat.ready = false;
        seat.inputs.clear();
        seat.newest = None;
        seat.heard = Instant::now();
        seat.resent = None;
        Ok(slot)
    }

    /// Tells every seat the race clock starts; the first [`Self::tick`] should run
    /// `lead_ticks` ticks from now.
    pub fn start(&mut self, lead_ticks: u16) {
        for seat in self.seats.iter_mut().flatten() {
            seat.heard = Instant::now();
            seat.link.send_control(&Control::Start { lead_ticks });
        }
        self.phase = Phase::Racing;
    }

    /// Runs one authoritative tick and sends every seat the confirmed record.
    pub fn tick(&mut self) {
        let tick = self.tick_number();
        let mut inputs = PlayerInputs::none();
        let mut applied = Vec::with_capacity(self.seats.len());
        for (slot, seat) in self.seats.iter_mut().enumerate() {
            let held = match seat.as_mut() {
                Some(seat) => {
                    seat.inputs.retain(|&t, _| t >= tick);
                    if let Some(held) = seat.inputs.remove(&tick) {
                        seat.counts.fresh += 1;
                        seat.last = held;
                    } else {
                        seat.counts.repeated += 1;
                    }
                    seat.counts.lead = seat
                        .newest
                        .map_or(-1, |newest| i64::from(newest) - i64::from(tick))
                        .clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                        as i32;
                    let snapshot = seat.last.snapshot(&mut seat.edges);
                    inputs.set(slot, snapshot);
                    seat.last
                }
                None => Held::default(),
            };
            applied.push(held);
        }
        let retired = self.retired_mask();
        crate::race::presence(&mut self.race, retired);
        self.race.tick_sim_only(&inputs);
        self.history.push(TickRecord {
            hash: self.race.sim.state_hash(),
            inputs: applied,
            retired,
        });
        self.broadcast();
    }

    /// The seats whose person is away now, as [`TickRecord::retired`] carries them.
    fn retired_mask(&self) -> u8 {
        self.seats
            .iter()
            .enumerate()
            .take(8)
            .filter(|(_, seat)| seat.as_ref().is_some_and(|seat| seat.left.is_some()))
            .fold(0, |mask, (slot, _)| mask | 1 << slot)
    }

    fn broadcast(&mut self) {
        let people = self.seats.len();
        let next = self.tick_number();
        for seat in self.seats.iter_mut().flatten() {
            if seat.left.is_some() {
                continue;
            }
            let room = seat.link.max_datagram().saturating_sub(16);
            let fit = (room / Confirmed::record_bytes(people)).clamp(1, REDUNDANCY);
            let first = self.history.len().saturating_sub(fit);
            let lead = seat
                .newest
                .map_or(-1, |newest| i64::from(newest) - i64::from(next))
                .clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16;
            seat.link.send(&Datagram::Confirmed {
                lead,
                confirmed: Confirmed {
                    first: u32::try_from(first).unwrap_or(u32::MAX),
                    ticks: self.history[first..].to_vec(),
                },
            });
        }
    }

    /// Retires every seat nothing has arrived from for `after` while the race runs: a vanished
    /// player (no close frame) would otherwise hold the seat until QUIC's 30 s idle timeout.
    /// Returns how many were retired. Chosen, not measured.
    pub fn drop_silent(&mut self, after: Duration) -> usize {
        let mut dropped = 0;
        for (slot, seat) in self.seats.iter_mut().enumerate() {
            let Some(seat) = seat.as_mut() else { continue };
            // A person who came back is loading the race again and says nothing until it is
            // ready, which on a slow machine is longer than the silence a flying seat may keep.
            let allowed = if seat.ready { after } else { after.max(RELOAD_GRACE) };
            if seat.left.is_none() && seat.heard.elapsed() > allowed {
                let why = format!("no input for {after:?}");
                log::warn!("net: seat {slot} ({}) left: {why}", seat.name);
                // Told, so a player who only stalled is taken back to the menus rather than left
                // watching a race that no longer moves for them.
                seat.link.send_control(&Control::End {
                    reason: format!("you were dropped from the race: {why}"),
                    board: Board::default(),
                });
                seat.leave(why);
                dropped += 1;
            }
        }
        dropped
    }

    /// Whether every person's craft has finished ([`Race::finished`]).
    #[must_use]
    pub fn finished(&self) -> bool {
        self.race.finished()
    }

    /// Whether a seat that was seated has left (a refused or closed connection, an end).
    #[must_use]
    pub fn departed(&self) -> bool {
        self.seats.iter().flatten().any(|seat| seat.left.is_some())
    }

    /// Whether every seat has left and none has been away for less than [`SEAT_GRACE`], the
    /// time a person has to return.
    #[must_use]
    pub fn deserted(&self) -> bool {
        self.seats.iter().all(|seat| {
            seat.as_ref().is_none_or(|seat| {
                seat.left.is_some() && seat.left_at.is_some_and(|at| at.elapsed() >= SEAT_GRACE)
            })
        })
    }

    /// Tells every seat the race is over, after the newest confirmed ticks on the
    /// reliable stream: the last datagrams may have been lost with nothing after
    /// them to show a client the gap.
    pub fn end(&mut self, reason: &str) {
        let first = self.history.len().saturating_sub(REDUNDANCY);
        let tail = Confirmed {
            first: u32::try_from(first).unwrap_or(u32::MAX),
            ticks: self.history[first..].to_vec(),
        };
        let board = self.board();
        for seat in self.seats.iter_mut().flatten() {
            seat.link.send_control(&Control::Ticks(tail.clone()));
            seat.link.send_control(&Control::End {
                reason: reason.to_string(),
                board: board.clone(),
            });
        }
        self.phase = Phase::Over;
    }

    /// The race's final board with the people's names, empty before the race has a result.
    #[must_use]
    pub fn board(&self) -> Board {
        crate::race::final_board(&self.race, &self.seat_names())
    }

    /// Each seat's counts since the last call, and resets them.
    pub fn take_counts(&mut self) -> Vec<Option<SeatCounts>> {
        self.seats
            .iter_mut()
            .map(|seat| {
                seat.as_mut().map(|seat| {
                    let counts = seat.counts;
                    seat.counts = SeatCounts {
                        lead: counts.lead,
                        ..SeatCounts::default()
                    };
                    counts
                })
            })
            .collect()
    }
}

impl std::fmt::Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Host")
            .field("phase", &self.phase)
            .field("tick", &self.tick_number())
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Seat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Seat")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}
