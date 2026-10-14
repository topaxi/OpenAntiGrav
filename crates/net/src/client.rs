//! The predicting side: a client racing ahead of the server and correcting itself.
//!
//! [ADR-0069](../../../docs/architecture/adr/0069-remote-craft-are-predicted-in-one-timeline-and-corrected-on-a-threshold.md)'s
//! shape, built:
//!
//! - **One present timeline.** The race the player sees runs ahead of the server:
//!   the own craft on this machine's real input, every other person's craft on the
//!   last input the server confirmed for them (held, so no new press or release).
//! - **A confirmed timeline beside it.** The server sends the inputs it applied
//!   and the state hash it reached ([`crate::wire`]); the client applies the same
//!   inputs to its copy of the server's state and checks the hash on every tick.
//!   The copy is the client's own deterministic re-run, so it costs no state on the
//!   wire, and a hash that disagrees is a determinism bug or different content
//!   ([`Stats::hash_mismatches`]), never a misprediction.
//! - **Correct on a threshold.** When a confirmed tick arrives, the craft as the
//!   client predicted them at that tick are compared with the confirmed ones; if any
//!   is further than [`THRESHOLD`] away, the present is rebuilt from the confirmed
//!   state by replaying the ticks since (`Race::reconcile_observed`), and a remote
//!   person's craft glides onto the result in the picture only (`Race::reconcile`'s
//!   smoothing). Under the threshold nothing is replayed.
//!
//! The two timelines share one `Race`: the confirmed state is swapped in, stepped
//! with `Race::tick_sim_only` and swapped out again, so keeping it costs the ticks
//! themselves and no copy of the state.
//!
//! # Chosen, not measured
//!
//! - [`THRESHOLD`]: 1 world unit, ADR-0069's figure.
//! - [`LEAD_TARGET`], [`LEAD_LOW`], [`LEAD_HIGH`], [`ADJUST_COOLDOWN`]: how far ahead
//!   of the server the client keeps its input, and how it steers there (one extra
//!   or one skipped tick at a time).
//! - [`MAX_WINDOW`]: how far the present may run ahead of the last confirmed tick
//!   before the client stops and waits (a stalled link).
//! - [`INPUT_REDUNDANCY`]: how many of its newest inputs each datagram repeats.

use std::collections::{BTreeMap, VecDeque};

use oag_core::math::Vec3;
use oag_gameplay::input::{Button, Input};
use oag_gameplay::{InputSnapshot, MAX_SHIPS, PlayerInputs};
use oag_raceplay::{Race, RaceSim};

use web_time::Instant;

use crate::link::Link;
use crate::stats::PathStats;
use crate::wire::{Confirmed, Control, Datagram, Held, PROTOCOL, RaceParams, TickRecord};

/// A craft further than this from where the server has it forces a correction.
/// World units; a craft is about 13 long. Chosen, not measured (ADR-0069).
pub const THRESHOLD: f32 = 1.0;

/// The lead the client steers toward: ticks of its input the server holds beyond
/// the tick it is about to run. Chosen.
pub const LEAD_TARGET: i16 = 2;
/// Below this the client runs an extra tick. Chosen.
pub const LEAD_LOW: i16 = 1;
/// Above this the client skips a tick. Chosen.
pub const LEAD_HIGH: i16 = 5;
/// Ticks between two adjustments, so a report the round trip has made stale is
/// not acted on twice. Chosen: 200 ms.
pub const ADJUST_COOLDOWN: u32 = 12;
/// The most ticks the present runs ahead of the confirmed state before the client
/// waits. Chosen: half a second.
pub const MAX_WINDOW: u32 = 30;
/// Inputs per datagram: this tick's and the seven before. Chosen.
pub const INPUT_REDUNDANCY: usize = 8;

/// What the server told this client when it was seated.
#[derive(Debug, Clone, PartialEq)]
pub struct Welcome {
    /// The grid slot this client flies.
    pub seat: usize,
    /// The race to load.
    pub params: RaceParams,
    /// The tick-0 hash to reproduce.
    pub fingerprint: u64,
}

/// Where the client is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Hello sent, no welcome yet.
    Joining,
    /// Welcomed; loading the race.
    Loading,
    /// Loaded and ready; waiting for the start.
    Ready,
    /// The clock is running.
    Racing,
    /// Over, and why.
    Over(String),
}

/// Counts since [`Client::take_stats`] was last asked, for a log line a second.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Stats {
    /// Ticks predicted (the present advanced).
    pub predicted: u32,
    /// Ticks confirmed (the server's state advanced here).
    pub confirmed: u32,
    /// Confirmed ticks whose hash matched the server's.
    pub hash_matches: u32,
    /// Confirmed ticks whose hash did not. Always a bug or different content.
    pub hash_mismatches: u32,
    /// Corrections made (the threshold crossed).
    pub corrections: u32,
    /// Ticks replayed by those corrections.
    pub replayed: u32,
    /// The largest distance a correction moved a craft, in world units.
    pub largest_pop: f32,
    /// The largest prediction error seen at a confirmed tick, corrected or not.
    pub largest_error: f32,
    /// Steps the client waited because the present was [`MAX_WINDOW`] ahead.
    pub stalls: u32,
    /// Extra ticks run and ticks skipped to steer the lead.
    pub sped_up: u32,
    /// See [`Self::sped_up`].
    pub slowed_down: u32,
    /// The lead the server last reported.
    pub lead: i16,
    /// Present tick less confirmed tick, at the last step.
    pub window: u32,
    /// Datagrams lost in a row past the redundancy, recovered by a resend.
    pub resends: u32,
    /// Milliseconds from sending a tick's input to that tick coming back confirmed,
    /// summed over [`Self::confirm_count`] ticks. The round trip plus the server's
    /// input buffer.
    pub confirm_sum_ms: f32,
    /// See [`Self::confirm_sum_ms`].
    pub confirm_count: u32,
    /// The change between consecutive confirm times, summed over
    /// [`Self::jitter_count`] pairs.
    pub jitter_sum_ms: f32,
    /// See [`Self::jitter_sum_ms`].
    pub jitter_count: u32,
}

/// One person's connection to a server, and the two timelines.
pub struct Client {
    link: Box<dyn Link>,
    phase: Phase,
    welcome: Option<Welcome>,
    /// The confirmed state, `None` while it is swapped into the race.
    confirmed: Option<RaceSim>,
    /// The world tick the confirmed state is at.
    confirmed_tick: u32,
    confirmed_edges: Vec<Input>,
    pending: BTreeMap<u32, TickRecord>,
    /// This machine's inputs from the confirmed tick on: (tick, wire, as applied).
    own: VecDeque<(u32, Held, InputSnapshot)>,
    own_edges: Input,
    /// Where every craft was predicted to be after each tick: (world tick, poses).
    predicted: VecDeque<(u32, [Vec3; MAX_SHIPS])>,
    /// Each person's last confirmed input, what their craft is predicted on.
    remote_last: Vec<Held>,
    lead: Option<i16>,
    last_adjust: u32,
    resend_asked: Option<u32>,
    bot: Option<oag_ai::Driver>,
    /// Whether the results board has been taken from the confirmed state.
    board_confirmed: bool,
    stats: Stats,
    /// When each own input still awaiting its confirmation was sent: (tick, time).
    sent_at: VecDeque<(u32, Instant)>,
    /// The last confirm time, for the jitter.
    last_confirm_ms: Option<f32>,
}

fn positions(sim: &RaceSim) -> [Vec3; MAX_SHIPS] {
    std::array::from_fn(|slot| sim.world.ships[slot].physics.body.position)
}

/// A held input as a prediction applies it: held since before, no new edges.
fn held_on(held: Held) -> InputSnapshot {
    let mut edges = Input::new();
    edges.begin_frame(held.buttons);
    held.snapshot(&mut edges)
}

impl Client {
    /// Says hello on `link`, as `name`.
    pub fn connect(link: Box<dyn Link>, name: &str) -> Self {
        Self::connect_as(link, name, "")
    }

    /// Says hello on `link`, as `name`, asking to fly `team` (empty: the host's choice).
    pub fn connect_as(mut link: Box<dyn Link>, name: &str, team: &str) -> Self {
        link.send_control(&Control::Hello {
            protocol: PROTOCOL,
            name: name.to_string(),
            team: team.to_string(),
        });
        Self {
            link,
            phase: Phase::Joining,
            welcome: None,
            confirmed: None,
            confirmed_tick: 0,
            confirmed_edges: Vec::new(),
            pending: BTreeMap::new(),
            own: VecDeque::new(),
            own_edges: Input::new(),
            predicted: VecDeque::new(),
            remote_last: Vec::new(),
            lead: None,
            last_adjust: 0,
            resend_asked: None,
            bot: None,
            board_confirmed: false,
            stats: Stats::default(),
            sent_at: VecDeque::new(),
            last_confirm_ms: None,
        }
    }

    /// Where the client is.
    #[must_use]
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// The welcome, once it has come.
    #[must_use]
    pub fn welcome(&self) -> Option<&Welcome> {
        self.welcome.as_ref()
    }

    /// Reads the link until the welcome arrives. `Err` for a refusal or a closed
    /// link; `Ok(None)` while waiting.
    ///
    /// # Errors
    ///
    /// The server refused, or the link closed.
    pub fn poll_welcome(&mut self) -> anyhow::Result<Option<Welcome>> {
        while let Some(message) = self.link.recv_control() {
            match message? {
                Control::Welcome {
                    seat,
                    params,
                    fingerprint,
                } => {
                    let welcome = Welcome {
                        seat: usize::from(seat),
                        params,
                        fingerprint,
                    };
                    self.welcome = Some(welcome.clone());
                    self.phase = Phase::Loading;
                    return Ok(Some(welcome));
                }
                Control::Reject { reason } => return Err(anyhow::anyhow!("refused: {reason}")),
                other => log::warn!("net: unexpected before the welcome: {other:?}"),
            }
        }
        if let Some(why) = self.link.closed() {
            return Err(anyhow::anyhow!("the connection closed: {why}"));
        }
        Ok(None)
    }

    /// Hands over the loaded race: checks its tick-0 state against the server's,
    /// marks this machine's seat on it, and tells the server it is ready.
    ///
    /// # Errors
    ///
    /// No welcome yet, or the race starts in a different state from the server's
    /// (different content or options), which is refused here rather than raced.
    pub fn ready(&mut self, race: &mut Race) -> anyhow::Result<()> {
        let welcome = self
            .welcome
            .clone()
            .ok_or_else(|| anyhow::anyhow!("ready before the welcome"))?;
        let ours = crate::race::fingerprint(race);
        if ours != welcome.fingerprint {
            let reason = format!(
                "this race starts at state {ours:#018x} and the server's at {:#018x}: \
                 different content or options",
                welcome.fingerprint
            );
            self.link.send_control(&Control::End {
                reason: reason.clone(),
            });
            return Err(anyhow::anyhow!(reason));
        }
        let people = usize::from(welcome.params.people);
        race.set_seat(Some(welcome.seat));
        self.confirmed = Some(race.snapshot_sim());
        self.confirmed_tick = u32::try_from(race.sim.world.tick).unwrap_or(0);
        self.confirmed_edges = vec![Input::new(); people];
        self.remote_last = vec![Held::default(); people];
        self.link
            .send_control(&Control::Ready { fingerprint: ours });
        self.phase = Phase::Ready;
        Ok(())
    }

    /// Reads the link: the start, confirmed ticks, resends, the end.
    pub fn poll(&mut self) {
        while let Some(message) = self.link.recv_control() {
            match message {
                Ok(Control::Start { lead_ticks }) => {
                    log::info!("net: the race starts (server lead {lead_ticks} ticks)");
                    self.phase = Phase::Racing;
                }
                Ok(Control::Ticks(confirmed)) => self.stash(confirmed),
                Ok(Control::End { reason }) | Ok(Control::Reject { reason }) => {
                    log::info!("net: the server ended the race: {reason}");
                    self.phase = Phase::Over(reason);
                }
                Ok(other) => log::warn!("net: unexpected {other:?}"),
                Err(e) => log::warn!("net: {e}"),
            }
        }
        while let Some(message) = self.link.recv() {
            match message {
                Ok(Datagram::Confirmed { lead, confirmed }) => {
                    self.lead = Some(lead);
                    self.stash(confirmed);
                }
                Ok(other) => log::warn!("net: unexpected {other:?}"),
                Err(e) => log::warn!("net: {e}"),
            }
        }
        if !matches!(self.phase, Phase::Over(_))
            && let Some(why) = self.link.closed()
        {
            self.phase = Phase::Over(format!("the connection closed: {why}"));
        }
    }

    fn stash(&mut self, confirmed: Confirmed) {
        for (i, record) in confirmed.ticks.into_iter().enumerate() {
            let tick = confirmed
                .first
                .saturating_add(u32::try_from(i).unwrap_or(u32::MAX));
            if tick >= self.confirmed_tick {
                self.pending.entry(tick).or_insert(record);
            }
        }
    }

    /// What this machine's input is predicted as for slot `slot` this tick.
    fn inputs_for(&self, own: InputSnapshot) -> PlayerInputs {
        let seat = self.welcome.as_ref().map_or(0, |w| w.seat);
        let mut inputs = PlayerInputs::none();
        for (slot, held) in self.remote_last.iter().enumerate() {
            inputs.set(slot, if slot == seat { own } else { held_on(*held) });
        }
        inputs
    }

    /// Steps the client once per tick of the frame clock: reads the link, advances
    /// the confirmed state with whatever arrived (correcting the present when it
    /// must), then predicts 0, 1 or 2 ticks with `own` as this machine's input.
    /// Returns how many ticks it predicted, each one a drawn `Race::tick`.
    pub fn step(&mut self, race: &mut Race, own: &InputSnapshot) -> u32 {
        self.poll();
        if matches!(self.phase, Phase::Over(_)) {
            // What the server confirmed before it ended still lands.
            self.advance_confirmed(race);
            return 0;
        }
        if self.phase != Phase::Racing {
            return 0;
        }
        self.advance_confirmed(race);
        let present = u32::try_from(race.sim.world.tick).unwrap_or(u32::MAX);
        self.stats.window = present.saturating_sub(self.confirmed_tick);
        let mut ticks = 1;
        if self.stats.window >= MAX_WINDOW {
            self.stats.stalls += 1;
            ticks = 0;
        } else if let Some(lead) = self.lead
            && present.saturating_sub(self.last_adjust) >= ADJUST_COOLDOWN
        {
            if lead < LEAD_LOW {
                ticks = 2;
                self.stats.sped_up += 1;
                self.last_adjust = present;
            } else if lead > LEAD_HIGH {
                ticks = 0;
                self.stats.slowed_down += 1;
                self.last_adjust = present;
            }
        }
        for _ in 0..ticks {
            self.predict(race, own);
        }
        self.stats.lead = self.lead.unwrap_or(0);
        ticks
    }

    fn predict(&mut self, race: &mut Race, own: &InputSnapshot) {
        let tick = u32::try_from(race.sim.world.tick).unwrap_or(u32::MAX);
        let held = Held::of(own);
        let snapshot = held.snapshot(&mut self.own_edges);
        let inputs = self.inputs_for(snapshot);
        race.tick(&inputs);
        self.own.push_back((tick, held, snapshot));
        self.sent_at.push_back((tick, Instant::now()));
        self.predicted.push_back((tick + 1, positions(&race.sim)));
        self.stats.predicted += 1;
        let seat = self.welcome.as_ref().map_or(0, |w| w.seat);
        let from = self.own.len().saturating_sub(INPUT_REDUNDANCY);
        let first = self.own.get(from).map_or(tick, |(t, _, _)| *t);
        self.link.send(&Datagram::Inputs {
            seat: u8::try_from(seat).unwrap_or(0),
            first,
            inputs: self
                .own
                .iter()
                .skip(from)
                .map(|(_, held, _)| *held)
                .collect(),
        });
    }

    /// Applies every confirmed tick that has arrived in order, checks each hash,
    /// and corrects the present if the prediction at the newest one was off by more
    /// than [`THRESHOLD`].
    fn advance_confirmed(&mut self, race: &mut Race) {
        let start = self.confirmed_tick;
        if !self.pending.contains_key(&start) {
            // A gap: ask for it on the reliable stream, once per gap.
            if self.pending.keys().next().is_some_and(|&t| t > start)
                && self.resend_asked != Some(start)
            {
                self.link.send_control(&Control::Resend { from: start });
                self.resend_asked = Some(start);
                self.stats.resends += 1;
            }
            return;
        }
        let Some(confirmed) = self.confirmed.take() else {
            return;
        };
        let present = std::mem::replace(&mut race.sim, confirmed);
        let seat = self.welcome.as_ref().map_or(0, |w| w.seat);
        while let Some(record) = self.pending.remove(&self.confirmed_tick) {
            let mut inputs = PlayerInputs::none();
            for (slot, held) in record.inputs.iter().enumerate() {
                if let Some(edges) = self.confirmed_edges.get_mut(slot) {
                    inputs.set(slot, held.snapshot(edges));
                }
                if slot != seat
                    && let Some(last) = self.remote_last.get_mut(slot)
                {
                    *last = *held;
                }
            }
            race.tick_sim_only(&inputs);
            if race.sim.state_hash() == record.hash {
                self.stats.hash_matches += 1;
            } else {
                self.stats.hash_mismatches += 1;
                log::error!(
                    "net: state hash disagrees with the server's after tick {}: a determinism \
                     bug or different content",
                    self.confirmed_tick
                );
            }
            self.note_confirmed(self.confirmed_tick);
            self.confirmed_tick += 1;
            self.stats.confirmed += 1;
        }
        self.pending.retain(|&t, _| t >= self.confirmed_tick);
        // The board is the server's: taken again from the confirmed state the first
        // time it is finished (the prediction finished first and took its own).
        if !self.board_confirmed && race.finished() {
            race.recapture_results();
            self.board_confirmed = true;
        }
        let confirmed = std::mem::replace(&mut race.sim, present);
        let now = self.confirmed_tick;
        while self.own.front().is_some_and(|(t, _, _)| *t < now) {
            self.own.pop_front();
        }
        while self.predicted.front().is_some_and(|(t, _)| *t < now) {
            self.predicted.pop_front();
        }
        let present_tick = u32::try_from(race.sim.world.tick).unwrap_or(0);
        if present_tick <= now {
            // The server is ahead of this client's present: take its state.
            race.restore_sim(&confirmed);
            self.own.clear();
            self.predicted.clear();
            self.confirmed = Some(confirmed);
            return;
        }
        let error = self.predicted.front().filter(|(t, _)| *t == now).map_or(
            f32::INFINITY,
            |(_, predicted)| {
                let real = positions(&confirmed);
                let ships = usize::from(confirmed.world.ship_count).min(MAX_SHIPS);
                (0..ships)
                    .map(|slot| (predicted[slot] - real[slot]).length())
                    .fold(0.0, f32::max)
            },
        );
        if error.is_finite() {
            self.stats.largest_error = self.stats.largest_error.max(error);
        }
        if error > THRESHOLD {
            self.correct(race, &confirmed);
        }
        self.confirmed = Some(confirmed);
    }

    /// Times how long the input for `tick` took to come back confirmed. A tick the
    /// client never predicted (the server was ahead) has no send time and is skipped.
    fn note_confirmed(&mut self, tick: u32) {
        while self.sent_at.front().is_some_and(|(t, _)| *t < tick) {
            self.sent_at.pop_front();
        }
        if let Some((t, sent)) = self.sent_at.front().copied()
            && t == tick
        {
            self.sent_at.pop_front();
            let ms = sent.elapsed().as_secs_f32() * 1000.0;
            self.stats.confirm_sum_ms += ms;
            self.stats.confirm_count += 1;
            if let Some(last) = self.last_confirm_ms {
                self.stats.jitter_sum_ms += (ms - last).abs();
                self.stats.jitter_count += 1;
            }
            self.last_confirm_ms = Some(ms);
        }
    }

    /// The transport's counters, see [`Link::path`].
    #[must_use]
    pub fn path(&self) -> Option<PathStats> {
        self.link.path()
    }

    /// Rebuilds the present from `confirmed` by replaying the ticks since.
    fn correct(&mut self, race: &mut Race, confirmed: &RaceSim) {
        let own: Vec<InputSnapshot> = self.own.iter().map(|(_, _, snapshot)| *snapshot).collect();
        let inputs: Vec<PlayerInputs> = own.iter().map(|o| self.inputs_for(*o)).collect();
        let mut replayed = VecDeque::with_capacity(inputs.len());
        let reconciled = race.reconcile_observed(confirmed, inputs, |sim| {
            replayed.push_back((u32::try_from(sim.world.tick).unwrap_or(0), positions(sim)));
        });
        self.predicted = replayed;
        self.stats.corrections += 1;
        self.stats.replayed += u32::try_from(reconciled.ticks).unwrap_or(u32::MAX);
        let pop = reconciled.pops.iter().copied().fold(0.0, f32::max);
        self.stats.largest_pop = self.stats.largest_pop.max(pop);
    }

    /// This machine's input when it flies itself (`--net-bot`): what the player's
    /// own driver would do, as a pad would send it. Cross is held through the
    /// countdown, the way a player holds it.
    pub fn bot_input(&mut self, race: &mut Race) -> InputSnapshot {
        let seat = self.welcome.as_ref().map_or(0, |w| w.seat);
        let wants = race.autopilot_wants(seat, &mut self.bot);
        let gated = oag_race::RaceState::thrust_gated(race.sim.world.tick);
        let mut buttons = Input::new();
        buttons.begin_frame(if gated || wants.thrust > 0.5 {
            Button::Cross.bit()
        } else {
            0
        });
        InputSnapshot {
            buttons,
            stick_x: wants.steer_x,
            stick_y: -wants.steer_y,
            airbrake_left: wants.airbrake_left,
            airbrake_right: wants.airbrake_right,
        }
        .sanitised()
    }

    /// Tells the server this client is leaving.
    pub fn leave(&mut self, reason: &str) {
        self.link.send_control(&Control::End {
            reason: reason.to_string(),
        });
    }

    /// The counts since the last call, and resets them (the lead and window kept).
    pub fn take_stats(&mut self) -> Stats {
        let stats = self.stats;
        self.stats = Stats {
            lead: stats.lead,
            window: stats.window,
            ..Stats::default()
        };
        stats
    }

    /// The tick the confirmed state is at.
    #[must_use]
    pub fn confirmed_tick(&self) -> u32 {
        self.confirmed_tick
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("phase", &self.phase)
            .field("confirmed_tick", &self.confirmed_tick)
            .finish_non_exhaustive()
    }
}
