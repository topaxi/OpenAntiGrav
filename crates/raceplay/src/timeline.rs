//! A second copy of a race, moved to any tick of its recording, and the keyframes that make
//! that quick: what a replay viewer and a photo rewind share.
//!
//! The decisions and the numbers are
//! `docs/architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md`; how the pieces
//! fit is `docs/architecture/replay-timeline.md`.
//!
//! **A [`Timeline`] never touches the race a person is driving.** It owns a probe, a [`Race`]
//! of its own, and reads a live race through `&Race` only, keeping no reference to it. Nothing
//! here needs the `view` feature, so a host with no renderer can verify a replay.
//!
//! # What a tick number means
//!
//! Keyframe `T`, and a probe at `T`, is the state after `T` ticks ran, **before the events due
//! at `T` are applied**: that is the point where a straight pass stands between tick `T - 1`
//! and tick `T`. Moving forward applies a tick's events, then runs it, so an event on a
//! keyframe's own tick and one among the resimulated ticks both land between the same two ticks
//! as on the recorded run.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use oag_replay::{Desync, Replay, Verifier};
use oag_thread::Task;

use crate::replay::{ApplyError, start_recorded};
use crate::{Race, RaceSim, Setup};

/// The ticks between keyframes before any thinning. Chosen: it bounds a seek to 59 ticks.
pub const KEYFRAME_TICKS: u64 = 60;

/// The bytes of keyframes a timeline holds before it thins them. Chosen, for the web build's
/// 4 GiB address space and a phone (ADR-0087, decision 2).
pub const KEYFRAME_CEILING: usize = 16 * 1024 * 1024;

/// How the opening pass over a recording stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    /// Still building; [`Timeline::prefix`] grows.
    Running,
    /// The whole recording reproduced.
    Complete,
    /// A stored hash disagreed. The prefix ends at the last one that agreed.
    Desynced(Desync),
    /// A recorded event is not one this build applies. The prefix ends before it.
    Refused(ApplyError),
    /// The worker panicked or never started. The prefix ends where it had got to.
    Lost,
}

/// Why [`Timeline::open`] turned a recording down.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// The race could not be started from the header's settings.
    #[error("the replay's race cannot be started: {0:#}")]
    Start(anyhow::Error),
    /// The race started is not the recorded one: a tick-zero [`Desync`].
    #[error(transparent)]
    Desync(#[from] Desync),
}

/// Why a seek did not arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SeekError {
    /// The tick is past the built prefix, or before the first keyframe.
    #[error("tick {tick} is outside the built prefix, which ends at {prefix:?}")]
    NotBuilt {
        /// The tick asked for.
        tick: u64,
        /// [`Timeline::prefix`] when it was asked.
        prefix: Option<u64>,
    },
    /// A recorded event could not be applied on the way.
    #[error(transparent)]
    Event(#[from] ApplyError),
}

/// The keyframes, oldest first, at multiples of an interval that doubles at the ceiling.
#[derive(Debug)]
struct Keyframes {
    interval: u64,
    frames: Vec<(u64, RaceSim)>,
    bytes: usize,
    ceiling: usize,
}

impl Keyframes {
    fn new(ceiling: usize) -> Self {
        Self {
            interval: KEYFRAME_TICKS,
            frames: Vec::new(),
            bytes: 0,
            ceiling,
        }
    }

    /// Whether a keyframe at `tick` would be kept. The first is, whatever its tick: it is where
    /// a timeline cut from a race already under way starts.
    fn wants(&self, tick: u64) -> bool {
        match self.frames.last() {
            None => true,
            Some((last, _)) => tick > *last && tick.is_multiple_of(self.interval),
        }
    }

    fn push(&mut self, tick: u64, sim: RaceSim) {
        self.bytes += sim.snapshot_bytes();
        self.frames.push((tick, sim));
        while self.bytes > self.ceiling && self.frames.len() > 2 {
            self.thin();
        }
    }

    /// Doubles the interval and drops every keyframe off it, the first excepted.
    fn thin(&mut self) {
        self.interval *= 2;
        let first = self.frames.first().map(|(tick, _)| *tick);
        let interval = self.interval;
        self.frames
            .retain(|(tick, _)| Some(*tick) == first || tick.is_multiple_of(interval));
        self.bytes = self
            .frames
            .iter()
            .map(|(_, sim)| sim.snapshot_bytes())
            .sum();
    }

    /// The last keyframe at or before `tick`.
    fn at_or_before(&self, tick: u64) -> Option<&(u64, RaceSim)> {
        let after = self.frames.partition_point(|(at, _)| *at <= tick);
        after.checked_sub(1).map(|index| &self.frames[index])
    }
}

/// What the worker and the timeline share.
#[derive(Debug)]
struct Built {
    keyframes: Keyframes,
    /// The last tick known good, `None` until the first keyframe is in.
    prefix: Option<u64>,
    pass: Pass,
}

#[derive(Debug)]
struct Shared {
    built: Mutex<Built>,
    /// Set when the timeline is dropped: nobody will read what the pass goes on to build.
    stop: AtomicBool,
}

impl Shared {
    fn new(ceiling: usize, pass: Pass) -> Arc<Self> {
        Arc::new(Self {
            built: Mutex::new(Built {
                keyframes: Keyframes::new(ceiling),
                prefix: None,
                pass,
            }),
            stop: AtomicBool::new(false),
        })
    }

    /// The lock is held for a push or a restore, so the page's thread may take it. A panicking holder leaves nothing half-written that a read could trip on.
    fn built(&self) -> std::sync::MutexGuard<'_, Built> {
        oag_thread::lock(&self.built).unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// A probe race, its keyframes, and the recording that moves it between them.
#[derive(Debug)]
pub struct Timeline {
    /// The inputs, hashes and events. `None` for a live race that records nothing: a seek then
    /// stops at a whole keyframe.
    replay: Option<Arc<Replay>>,
    shared: Arc<Shared>,
    probe: Race,
    /// The tick the probe stands at.
    at: u64,
    pass: Option<Task<()>>,
}

impl Timeline {
    /// Opens `replay` over the race `setup` starts, and begins the opening pass on a worker.
    ///
    /// The pass plays every input, checks every stored hash and cuts the keyframes. The probe
    /// stands at tick 0 and [`Self::prefix`] says how far it may go, as a video player's
    /// buffered bar does.
    ///
    /// # Errors
    ///
    /// [`OpenError`]: the race cannot be started, or it is not the recorded one.
    pub fn open(setup: Setup, replay: Replay) -> Result<Self, OpenError> {
        Self::open_within(setup, replay, KEYFRAME_CEILING)
    }

    /// [`Self::open`] with `ceiling` bytes of keyframes instead of [`KEYFRAME_CEILING`].
    ///
    /// # Errors
    ///
    /// As [`Self::open`].
    pub fn open_within(setup: Setup, replay: Replay, ceiling: usize) -> Result<Self, OpenError> {
        let probe = start_recorded(setup.clone(), &replay.header).map_err(OpenError::Start)?;
        Verifier::new(&replay, probe.sim.state_hash())?;
        let replay = Arc::new(replay);
        let shared = Shared::new(ceiling, Pass::Running);
        let pass = Task::spawn("oag-timeline", {
            let replay = Arc::clone(&replay);
            let shared = Arc::clone(&shared);
            move || run_pass(&shared, &replay, setup)
        });
        Ok(Self {
            replay: Some(replay),
            shared,
            probe,
            at: 0,
            pass: Some(pass),
        })
    }

    /// A timeline over the race being driven, cut from where `race` stands now. Feed it with
    /// [`Self::keep`] and move it after [`Self::visit`].
    #[must_use]
    pub fn live(setup: Setup, race: &Race) -> Self {
        let shared = Shared::new(KEYFRAME_CEILING, Pass::Complete);
        shared
            .built()
            .keyframes
            .push(race.sim.world.tick, race.snapshot_sim());
        Self::over(Race::start(setup), shared, race.sim.world.tick)
    }

    /// A timeline whose keyframes are `shared`, moved by `probe`.
    fn over(probe: Race, shared: Arc<Shared>, tick: u64) -> Self {
        Self {
            replay: None,
            shared,
            probe,
            at: tick,
            pass: None,
        }
    }

    /// Takes the live race's keyframe if one falls due: called straight after a tick, before
    /// anything the session writes between ticks, so a keyframe stands where a pass would cut
    /// it.
    ///
    /// A first keyframe from before the first tick is handed the plans that tick learned, so
    /// a rewind into the race's first second does not learn them again.
    pub fn keep(&mut self, race: &Race) {
        keep_in(&self.shared, race);
    }

    /// Enters the live race as it stands: the probe moves to it, and every tick up to it can be
    /// sought. `race` is only read; the race goes on from exactly here when the visit ends.
    pub fn visit(&mut self, race: &Race) {
        let tick = race.sim.world.tick;
        #[cfg(feature = "view")]
        {
            self.replay = race.recorded().map(Arc::new);
        }
        self.probe.restore_sim(&race.sim);
        self.at = tick;
        self.shared.built().prefix = Some(tick);
    }

    /// How the opening pass stands.
    pub fn pass(&mut self) -> Pass {
        if self.pass.as_ref().is_some_and(Task::is_finished) {
            self.collect();
        }
        self.shared.built().pass
    }

    /// Waits for the opening pass. Not in the browser: the page's thread may not block, so a
    /// caller there polls [`Self::pass`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wait(&mut self) -> Pass {
        self.collect();
        self.shared.built().pass
    }

    fn collect(&mut self) {
        let Some(mut task) = self.pass.take() else {
            return;
        };
        let joined = task.join();
        let mut built = self.shared.built();
        if !matches!(joined, Some(Ok(()))) || built.pass == Pass::Running {
            built.pass = Pass::Lost;
        }
    }

    /// The last tick the probe may stand at, `None` until the first keyframe is in. Every tick
    /// up to it agreed with the recording.
    #[must_use]
    pub fn prefix(&self) -> Option<u64> {
        self.shared.built().prefix
    }

    /// The race the probe stands in, to draw or read.
    #[must_use]
    pub fn probe(&self) -> &Race {
        &self.probe
    }

    /// The recording a seek plays between keyframes: the race's own, as of the last
    /// [`Self::visit`]. `None` for a race that records nothing.
    #[must_use]
    pub fn recording(&self) -> Option<Arc<Replay>> {
        self.replay.clone()
    }

    /// The tick the probe stands at.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.at
    }

    /// The ticks between keyframes now, which the ceiling may have doubled.
    #[must_use]
    pub fn interval(&self) -> u64 {
        self.shared.built().keyframes.interval
    }

    /// How many keyframes are held.
    #[must_use]
    pub fn keyframes(&self) -> usize {
        self.shared.built().keyframes.frames.len()
    }

    /// Moves the probe to `tick`: the keyframe at or before it, then a silent resimulation.
    /// Returns the tick reached, which with no recording is that keyframe's.
    ///
    /// # Errors
    ///
    /// [`SeekError`]; the probe then stands wherever it got to, [`Self::tick`].
    pub fn seek(&mut self, tick: u64) -> Result<u64, SeekError> {
        {
            let built = self.shared.built();
            let reachable = built.prefix.is_some_and(|prefix| tick <= prefix);
            let frame = built.keyframes.at_or_before(tick).filter(|_| reachable);
            let Some((from, sim)) = frame else {
                return Err(SeekError::NotBuilt {
                    tick,
                    prefix: built.prefix,
                });
            };
            // A probe already between the keyframe and the target only has to walk on.
            if !(*from..=tick).contains(&self.at) {
                self.probe.restore_sim(sim);
                self.at = *from;
            }
        }
        if self.replay.is_some() {
            while self.at < tick {
                self.advance()?;
            }
        }
        Ok(self.at)
    }

    /// One quiet tick forward, or `None` at the end of the prefix or with no recording.
    ///
    /// # Errors
    ///
    /// A recorded event could not be applied.
    pub fn step(&mut self) -> Result<Option<u64>, ApplyError> {
        if self.replay.is_none() || self.prefix().is_none_or(|prefix| self.at >= prefix) {
            return Ok(None);
        }
        self.advance()?;
        Ok(Some(self.at))
    }

    fn advance(&mut self) -> Result<(), ApplyError> {
        let Some(replay) = &self.replay else {
            return Ok(());
        };
        for event in replay.events_at(self.at) {
            self.probe.apply_event(event)?;
        }
        self.probe.tick_sim_only(&replay.inputs_at(self.at));
        self.at += 1;
        Ok(())
    }
}

/// The keyframes of a race being driven, kept from its first tick before anyone asks to rewind,
/// with the probe that moves between them started only on the first [`Self::visit`]: a race that
/// never rewinds pays the keyframes alone (a few microseconds a second) and no second [`Race`].
#[derive(Debug)]
pub struct Keeper {
    setup: Setup,
    shared: Arc<Shared>,
    timeline: Option<Timeline>,
}

impl Keeper {
    /// Keeps `race`, which stands at its first tick, and holds `setup` to start a probe from.
    #[must_use]
    pub fn new(setup: Setup, race: &Race) -> Self {
        let shared = Shared::new(KEYFRAME_CEILING, Pass::Complete);
        shared
            .built()
            .keyframes
            .push(race.sim.world.tick, race.snapshot_sim());
        Self {
            setup,
            shared,
            timeline: None,
        }
    }

    /// As [`Timeline::keep`]: after every tick of the live race.
    pub fn keep(&mut self, race: &Race) {
        keep_in(&self.shared, race);
    }

    /// Cuts the first keyframe again from `race`, which stands at the tick it was started at
    /// and has had every writer a session applies before its first tick (the AI choice, a
    /// campaign's skill, the opponent pilot). A [`Self::new`] made at the race's build misses
    /// those, and a rewind that pre-rolls from that keyframe flies the pre-roll with another
    /// driver. Does nothing once the race has ticked: later keyframes are cut by
    /// [`Self::keep`].
    pub fn recut_first(&mut self, race: &Race) {
        let mut built = self.shared.built();
        let tick = race.sim.world.tick;
        if built.keyframes.frames.len() == 1 && built.keyframes.frames[0].0 == tick {
            built.keyframes.frames.clear();
            built.keyframes.bytes = 0;
            built.keyframes.push(tick, race.snapshot_sim());
        }
    }

    /// The timeline over the keyframes so far, with its probe at the live race's tick
    /// ([`Timeline::visit`]). The first visit starts the probe.
    pub fn visit(&mut self, race: &Race) -> &mut Timeline {
        let tick = race.sim.world.tick;
        let timeline = self.timeline.get_or_insert_with(|| {
            Timeline::over(probe_of(&self.setup, race), Arc::clone(&self.shared), tick)
        });
        timeline.visit(race);
        timeline
    }

    /// The timeline a [`Self::visit`] started, to seek again without visiting.
    pub fn timeline(&mut self) -> Option<&mut Timeline> {
        self.timeline.as_mut()
    }

    /// How many keyframes are held.
    #[must_use]
    pub fn keyframes(&self) -> usize {
        self.shared.built().keyframes.frames.len()
    }
}

/// The probe for `race`: started from `setup` the way its recording says the race was launched
/// (the scheme, the difficulty, the seats), so that what it plays between keyframes is what the
/// race played. A race that records nothing gets the plain start.
fn probe_of(setup: &Setup, race: &Race) -> Race {
    #[cfg(feature = "view")]
    if let Some(replay) = race.recorded()
        && let Ok(probe) = start_recorded(setup.clone(), &replay.header)
    {
        return probe;
    }
    #[cfg(not(feature = "view"))]
    let _ = race;
    Race::start(setup.clone())
}

fn keep_in(shared: &Shared, race: &Race) {
    let tick = race.sim.world.tick;
    let mut built = shared.built();
    if let Some((_, first)) = built.keyframes.frames.first_mut()
        && !first.plans_built
        && race.sim.plans_built
    {
        first.share_plans_of(&race.sim);
    }
    if built.keyframes.wants(tick) {
        built.keyframes.push(tick, race.snapshot_sim());
    }
}

impl Drop for Timeline {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
    }
}

/// The opening pass, on its own race: every input played, every hash checked, a keyframe every
/// [`KEYFRAME_TICKS`], published once a stored hash has vouched for it.
fn run_pass(shared: &Shared, replay: &Replay, setup: Setup) {
    let Ok(mut race) = start_recorded(setup, &replay.header) else {
        return;
    };
    // Before keyframe 0, so that it carries the plans and a probe restored from it never
    // learns them on the caller's thread. The plans read the started race and no tick.
    race.ensure_speed_plans();
    let Ok(mut verifier) = Verifier::new(replay, race.sim.state_hash()) else {
        return;
    };
    let mut unvouched = vec![(0, race.snapshot_sim())];
    let mut last_good = 0;
    publish(shared, &mut unvouched, last_good);
    while let Some(inputs) = verifier.next_inputs() {
        if shared.stop.load(Ordering::Relaxed) {
            return;
        }
        for event in verifier.events_due() {
            if let Err(why) = race.apply_event(event) {
                shared.built().pass = Pass::Refused(why);
                return;
            }
        }
        race.tick_sim_only(&inputs);
        let vouched = verifier.hash_due();
        if let Err(desync) = verifier.after_tick(|| race.sim.state_hash()) {
            shared.built().pass = Pass::Desynced(desync);
            return;
        }
        let tick = verifier.tick();
        if tick.is_multiple_of(KEYFRAME_TICKS) {
            unvouched.push((tick, race.snapshot_sim()));
        }
        if vouched {
            last_good = tick;
            publish(shared, &mut unvouched, last_good);
        }
    }
    // The ticks after the last stored hash have nothing to disagree with.
    publish(shared, &mut unvouched, verifier.tick());
    shared.built().pass = Pass::Complete;
}

fn publish(shared: &Shared, frames: &mut Vec<(u64, RaceSim)>, prefix: u64) {
    let mut built = shared.built();
    for (tick, sim) in frames.drain(..) {
        if built.keyframes.wants(tick) {
            built.keyframes.push(tick, sim);
        }
    }
    built.prefix = Some(prefix);
}
