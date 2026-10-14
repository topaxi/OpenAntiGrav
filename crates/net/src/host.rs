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

use oag_gameplay::input::Input;
use oag_gameplay::{MAX_PLAYERS, PlayerInputs};
use oag_raceplay::Race;

use crate::link::Link;
use crate::wire::{Confirmed, Control, Datagram, Held, PROTOCOL, RaceParams, TickRecord};

/// The newest confirmed ticks each server datagram carries, at most (fewer when
/// they would not fit the link's datagram). Chosen, not measured: 16 ticks is
/// 267 ms of datagrams lost in a row before a client has to ask for a resend.
pub const REDUNDANCY: usize = 16;

/// Ticks per resend message. Chosen.
pub const RESEND_CHUNK: usize = 256;

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
    /// Why the seat left, once it has.
    left: Option<String>,
    counts: SeatCounts,
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
    joining: Vec<Box<dyn Link>>,
    history: Vec<TickRecord>,
    phase: Phase,
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
            history: Vec::new(),
            phase: Phase::Lobby,
        }
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

    /// A new connection. It is seated when its hello arrives.
    pub fn join(&mut self, link: Box<dyn Link>) {
        self.joining.push(link);
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
        for (slot, seat) in self.seats.iter_mut().enumerate() {
            let Some(seat) = seat.as_mut() else { continue };
            if seat.left.is_some() {
                continue;
            }
            if let Some(why) = seat.link.closed() {
                log::warn!("net: seat {slot} ({}) left: {why}", seat.name);
                seat.left = Some(why);
                continue;
            }
            while let Some(message) = seat.link.recv_control() {
                match message {
                    Ok(Control::Ready {
                        fingerprint: theirs,
                    }) if theirs == fingerprint => {
                        log::info!("net: seat {slot} ({}) is ready", seat.name);
                        seat.ready = true;
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
                        seat.left = Some(reason);
                    }
                    Ok(Control::Resend { from }) => {
                        let from = from as usize;
                        for chunk in self.history.get(from..).unwrap_or(&[]).chunks(RESEND_CHUNK) {
                            seat.link.send_control(&Control::Ticks(Confirmed {
                                first: u32::try_from(from).unwrap_or(u32::MAX),
                                ticks: chunk.to_vec(),
                            }));
                        }
                    }
                    Ok(Control::End { reason }) => {
                        log::info!("net: seat {slot} ({}) left: {reason}", seat.name);
                        seat.left = Some(reason);
                    }
                    Ok(other) => log::warn!("net: seat {slot}: unexpected {other:?}"),
                    Err(e) => log::warn!("net: seat {slot}: {e}"),
                }
            }
            while let Some(message) = seat.link.recv() {
                match message {
                    Ok(Datagram::Inputs {
                        seat: from,
                        first,
                        inputs,
                    }) if usize::from(from) == slot => {
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
                            seat.inputs.insert(tick, held);
                            seat.newest = Some(seat.newest.map_or(tick, |n| n.max(tick)));
                        }
                    }
                    Ok(other) => log::warn!("net: seat {slot}: unexpected {other:?}"),
                    Err(e) => log::warn!("net: seat {slot}: {e}"),
                }
            }
        }
    }

    fn poll_joining(&mut self) {
        let mut still = Vec::new();
        for mut link in std::mem::take(&mut self.joining) {
            let Some(message) = link.recv_control() else {
                if link.closed().is_none() {
                    still.push(link);
                }
                continue;
            };
            let reason = match message {
                Ok(Control::Hello { protocol, name }) if protocol == PROTOCOL => {
                    match self.seats.iter().position(Option::is_none) {
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
                                counts: SeatCounts::default(),
                            });
                            continue;
                        }
                        Some(_) => "the race has started".to_string(),
                        None => "every seat is taken".to_string(),
                    }
                }
                Ok(Control::Hello { protocol, .. }) => {
                    format!("this server speaks protocol {PROTOCOL}, you speak {protocol}")
                }
                Ok(other) => format!("expected a hello, got {other:?}"),
                Err(e) => format!("{e}"),
            };
            log::warn!("net: refused a connection: {reason}");
            link.send_control(&Control::Reject { reason });
        }
        self.joining = still;
    }

    /// Tells every seat the race clock starts; the first [`Self::tick`] should run
    /// `lead_ticks` ticks from now.
    pub fn start(&mut self, lead_ticks: u16) {
        for seat in self.seats.iter_mut().flatten() {
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
        self.race.tick_sim_only(&inputs);
        self.history.push(TickRecord {
            hash: self.race.sim.state_hash(),
            inputs: applied,
        });
        self.broadcast();
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

    /// Whether every person's craft has finished ([`Race::finished`]).
    #[must_use]
    pub fn finished(&self) -> bool {
        self.race.finished()
    }

    /// Whether every seat has left.
    #[must_use]
    pub fn deserted(&self) -> bool {
        self.seats
            .iter()
            .all(|seat| seat.as_ref().is_none_or(|seat| seat.left.is_some()))
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
        for seat in self.seats.iter_mut().flatten() {
            seat.link.send_control(&Control::Ticks(tail.clone()));
            seat.link.send_control(&Control::End {
                reason: reason.to_string(),
            });
        }
        self.phase = Phase::Over;
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
