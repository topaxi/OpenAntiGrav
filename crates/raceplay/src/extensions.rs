//! Simulation mods in a race: the one call per tick, the event buffer, the command
//! sinks and each module's state blob (`docs/architecture/wasm-extensions/hooks.md`).
//!
//! - **Event**: `wall-contact{slot}`, recorded by [`Race::count_wall_contact`] for every
//!   racing craft, a person's or the AI's, on each tick it touches a wall (the same test
//!   [`RaceSim::wall_contact_ticks`] counts). Raised after the call, so it reaches the
//!   guest in the next tick's observation: a one-tick latency, identical everywhere.
//! - **The call**: [`Race::tick_extensions`], between `advance_cannons` and the people's
//!   steps (phase C of `hooks.md`). Modules run in id order; each sees the same events.
//! - **Command**: `scale-thrust{slot, factor}`, `factor` in `[0, 1]`, **multiplied** into
//!   `Ship::pending_thrust_scale`, which this tick's step consumes for a person's craft
//!   (`player_step.rs`) and an AI craft (`field.rs`) alike. Multiplied, not assigned, so a
//!   Leach Beam drain armed at the end of the last tick is kept.
//! - **State**: each module's blob and fault tick live in [`RaceSim`], cloned with a
//!   snapshot and hashed by [`Extensions::hash_into`]. A race with no module hashes
//!   nothing here, so every vanilla hash is unchanged.
//!
//! Behind the `extensions` cargo feature: off, [`prepare`] refuses every source by name
//! and nothing of wasmtime is built or linked.

use std::sync::Arc;

mod track;

pub use track::{Track, TrackProps};

/// The most simulation modules one race runs. Chosen.
pub const MAX_SIM_MODS: usize = 4;

/// Reads a module's bytes, on the load thread.
pub type Reader = Arc<dyn Fn() -> Result<Vec<u8>, String> + Send + Sync>;

/// Where a [`Source`] runs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Scope {
    /// Every race of the mod's titles: a rules module.
    #[default]
    Rules,
    /// Only a race of this track file (`[extension.track]`); `None` is a track the
    /// composition root could not place on disk (a mod that is not a folder), which no
    /// race matches.
    Track(Option<std::path::PathBuf>),
}

impl Scope {
    /// Whether a race of `foreign_track` (the track file it was asked to race, if any)
    /// runs it.
    fn runs_on(&self, foreign_track: Option<&std::path::Path>) -> bool {
        let Self::Track(track) = self else {
            return true;
        };
        let (Some(track), Some(raced)) = (track, foreign_track) else {
            return false;
        };
        let same = |path: &std::path::Path| path.canonicalize().unwrap_or_else(|_| path.into());
        same(track) == same(raced)
    }
}

/// A simulation module a mod that is on offers this race: what the composition root
/// found in `oag-mod.toml`, with a reader for the `.wasm` so nothing is opened until
/// the race loads it.
#[derive(Clone)]
pub struct Source {
    /// The mod's id; modules run in the order these sort.
    pub mod_id: String,
    /// The module's path in the mod, for the load report.
    pub module: String,
    /// `[mod] titles`, lowercased and hyphenated: the module runs only on these; empty
    /// is every title.
    pub titles: Vec<String>,
    /// `[extension] api`: the ABI major the module was built against.
    pub api: u32,
    /// `[extension] memory_pages`.
    pub memory_pages: u32,
    /// `[extension] state_bytes`.
    pub state_bytes: u32,
    /// Where it runs.
    pub scope: Scope,
    /// Reads the module.
    pub read: Reader,
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Source")
            .field("mod_id", &self.mod_id)
            .field("module", &self.module)
            .field("api", &self.api)
            .field("memory_pages", &self.memory_pages)
            .field("state_bytes", &self.state_bytes)
            .finish_non_exhaustive()
    }
}

/// Finds the [`Source`]s at race load, on the load thread: the composition root's
/// search of the mods that are on, so not even a manifest is read on the thread that
/// draws (`Session::launch_race` runs on it).
#[derive(Clone)]
pub struct Finder(pub Arc<dyn Fn() -> Result<Vec<Source>, String> + Send + Sync>);

impl std::fmt::Debug for Finder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Finder")
    }
}

/// [`Options::extensions`](crate::Options::extensions) plus what
/// [`Options::find_extensions`](crate::Options::find_extensions) finds, its failure
/// in `report`.
#[must_use]
pub fn gather(
    sources: &[Source],
    finder: Option<&Finder>,
    report: &mut Vec<String>,
) -> Vec<Source> {
    let mut all = sources.to_vec();
    if let Some(finder) = finder {
        match (finder.0)() {
            Ok(found) => all.extend(found),
            Err(e) => push(
                report,
                format!("mods: {e}; the race runs no simulation mod"),
            ),
        }
    }
    all
}

/// The modules the load compiled, for [`Race::start`](crate::Race::start).
#[derive(Debug, Clone, Default)]
pub struct Prepared {
    #[cfg(feature = "extensions")]
    modules: Vec<(String, oag_modhost::Module)>,
    /// Whether a track module is among them: the load then builds the host title's
    /// rain for it ([`Self::has_track_module`]).
    track_module: bool,
    /// The track as the modules read it.
    #[cfg(feature = "extensions")]
    region: track::Region,
}

impl Prepared {
    /// Whether a module bound to this race's track file loaded.
    #[must_use]
    pub fn has_track_module(&self) -> bool {
        self.track_module
    }
}

/// Reads and compiles the `sources` that are for `title` (its slug) and, for a track
/// module, for `foreign_track` (the track file raced), at race load, and lays out
/// `track` for them. A module that fails leaves its mod out and says why in `report`;
/// the race runs without it. A track past the ABI's caps leaves every module out, by
/// name.
#[must_use]
pub fn prepare(
    sources: &[Source],
    title: &str,
    foreign_track: Option<&std::path::Path>,
    track: Track<'_>,
    report: &mut Vec<String>,
) -> Prepared {
    let sources: Vec<&Source> = sources
        .iter()
        .filter(|s| s.titles.is_empty() || s.titles.iter().any(|t| t.eq_ignore_ascii_case(title)))
        .filter(|s| s.scope.runs_on(foreign_track))
        .collect();
    #[cfg(not(feature = "extensions"))]
    {
        let _ = track;
        for source in sources {
            push(
                report,
                format!(
                    "mod {}: {} left out: this build has no `extensions` feature",
                    source.mod_id, source.module
                ),
            );
        }
        Prepared::default()
    }
    #[cfg(feature = "extensions")]
    {
        if sources.is_empty() {
            return Prepared::default();
        }
        let region = match track::encode(track) {
            Ok(region) => region,
            Err(e) => {
                push(
                    report,
                    format!("mods: the track carries {e}; the race runs no simulation mod"),
                );
                return Prepared::default();
            }
        };
        let mut sorted = sources;
        sorted.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
        let mut modules = Vec::new();
        let mut track_module = false;
        for source in sorted {
            let label = format!("mod {}: {}", source.mod_id, source.module);
            if modules.len() == MAX_SIM_MODS {
                push(
                    report,
                    format!(
                        "{label} left out: a race runs at most {MAX_SIM_MODS} simulation modules"
                    ),
                );
                continue;
            }
            if source.api != oag_ext_abi::ABI {
                push(
                    report,
                    format!(
                        "{label} left out: built for API {}, this host is API {}",
                        source.api,
                        oag_ext_abi::ABI
                    ),
                );
                continue;
            }
            let limits = oag_modhost::Limits {
                memory_pages: source.memory_pages,
                state_bytes: source.state_bytes,
                fuel: oag_modhost::FUEL_PER_TICK,
            };
            match (source.read)()
                .map_err(|e| format!("cannot be read: {e}"))
                .and_then(|wasm| {
                    oag_modhost::Module::compile(&wasm, limits).map_err(|e| e.to_string())
                }) {
                Ok(module) => {
                    push(
                        report,
                        format!(
                            "{label}: {} loaded, {} state bytes, {} fuel a tick; the race is MODDED and records no replay",
                            match source.scope {
                                Scope::Rules => "simulation module",
                                Scope::Track(_) => "track module",
                            },
                            source.state_bytes,
                            oag_modhost::FUEL_PER_TICK
                        ),
                    );
                    track_module |= matches!(source.scope, Scope::Track(_));
                    modules.push((source.mod_id.clone(), module));
                }
                Err(e) => push(report, format!("{label} left out: {e}")),
            }
        }
        Prepared {
            modules,
            track_module,
            region,
        }
    }
}

/// A load report line, also logged at `info`: a modded race says so at the default
/// filter, where the rest of the report is `debug`.
fn push(report: &mut Vec<String>, line: String) {
    log::info!("{line}");
    report.push(line);
}

/// The race-wide outputs of `set-weather`, `set-sun` and `grade-sky` this tick. Each is
/// its own identity when unsent, so a race without a module draws the vanilla frame.
/// Two modules sending the same command: the later module in id order wins (chosen).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sky {
    /// `set-weather`'s rain strength, `None` when not sent.
    pub rain: Option<f32>,
    /// `set-sun`'s intensity; `1.0` when not sent.
    pub sun: f32,
    /// `grade-sky`'s grey; `0.0` when not sent.
    pub grey: f32,
}

impl Default for Sky {
    fn default() -> Self {
        Self {
            rain: None,
            sun: 1.0,
            grey: 0.0,
        }
    }
}

/// One module's numbers for the dev overlay, after the last tick. An output, never
/// state, never hashed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// The mod's id.
    pub mod_id: String,
    /// Fuel the last call used.
    pub fuel_used: u64,
    /// The most any call this race used.
    pub fuel_peak: u64,
    /// Fuel a call may use.
    pub fuel_budget: u64,
    /// The last call's wall time, microseconds.
    pub call_us: f32,
    /// The tick the module faulted on, and why; it has not run since.
    pub faulted: Option<(u64, String)>,
}

/// The race's simulation modules: their state (hashed) and their reports (not).
#[derive(Debug, Clone, Default)]
pub struct Extensions {
    #[cfg(feature = "extensions")]
    on: on::On,
}

impl Extensions {
    /// The state for the modules `prepared` holds, blobs zeroed.
    #[must_use]
    pub fn new(prepared: Prepared) -> Self {
        #[cfg(not(feature = "extensions"))]
        {
            let _ = prepared;
            Self::default()
        }
        #[cfg(feature = "extensions")]
        {
            Self {
                on: on::On::new(prepared.modules, prepared.region),
            }
        }
    }

    /// Whether any module runs in this race: the race is MODDED.
    #[must_use]
    pub fn any(&self) -> bool {
        #[cfg(feature = "extensions")]
        {
            !self.on.slots.is_empty()
        }
        #[cfg(not(feature = "extensions"))]
        {
            false
        }
    }

    /// A weapon pad just drew `weapon`: whether a module's `pickup-roll` turns this grant
    /// into the mod's own pickup. Counts the grant. `false` in every race without one.
    pub(super) fn rolls(&mut self, weapon: oag_tables::weapons::Weapon) -> bool {
        #[cfg(feature = "extensions")]
        {
            self.on.rolls(weapon)
        }
        #[cfg(not(feature = "extensions"))]
        {
            let _ = weapon;
            false
        }
    }

    /// `slot`'s `pickup-tint` this tick: an output for the HUD, `None` without one.
    #[must_use]
    pub fn pickup_tint(&self, slot: usize) -> Option<[f32; 4]> {
        #[cfg(feature = "extensions")]
        {
            self.on.tints.get(slot).copied().flatten()
        }
        #[cfg(not(feature = "extensions"))]
        {
            let _ = slot;
            None
        }
    }

    /// What a module's race-wide outputs ask this tick: `None` strength is no rain.
    /// Held for the view half, never hashed, `Sky::default()` in every race without one.
    #[must_use]
    pub fn sky(&self) -> Sky {
        #[cfg(feature = "extensions")]
        {
            self.on.sky
        }
        #[cfg(not(feature = "extensions"))]
        {
            Sky::default()
        }
    }

    /// The `fire-effect`s queued since the view last drew them.
    #[cfg(feature = "view")]
    pub(super) fn take_effects(&mut self) -> Vec<(usize, oag_title::effects::Trigger)> {
        #[cfg(feature = "extensions")]
        {
            std::mem::take(&mut self.on.effects)
        }
        #[cfg(not(feature = "extensions"))]
        {
            Vec::new()
        }
    }

    /// What speed pad `pad` pushes `slot` by this tick: the pad's race-wide
    /// `scale-pad` times the slot's. Exactly `1.0` in every race without a module.
    pub(super) fn pad_factor(&self, slot: usize, pad: usize) -> f32 {
        #[cfg(feature = "extensions")]
        {
            self.on.pad_factor(slot, pad)
        }
        #[cfg(not(feature = "extensions"))]
        {
            let _ = (slot, pad);
            1.0
        }
    }

    /// `slot` entered speed pad `pad` (`speed-pad`), or, `weapon`, weapon pad `pad`.
    pub(super) fn pad_entered(&mut self, slot: usize, pad: usize, weapon: bool) {
        #[cfg(feature = "extensions")]
        self.on.pad_entered(slot, pad, weapon);
        #[cfg(not(feature = "extensions"))]
        let _ = (slot, pad, weapon);
    }

    /// Which of the track's triggers `slot` is inside this tick: any point of its
    /// sweep (`Race::pad_sweep`) in the box. Tested only while a module runs.
    pub(super) fn test_triggers(&mut self, slot: usize, sweep: &[oag_core::math::Vec3]) {
        #[cfg(feature = "extensions")]
        self.on.test_triggers(slot, sweep);
        #[cfg(not(feature = "extensions"))]
        let _ = (slot, sweep);
    }

    /// The mod's own pickup's fire (`absorbed == false`) or absorb, raised for the module.
    pub(super) fn pickup_used(&mut self, slot: usize, absorbed: bool) {
        #[cfg(feature = "extensions")]
        self.on.pickup_used(slot, absorbed);
        #[cfg(not(feature = "extensions"))]
        let _ = (slot, absorbed);
    }

    /// `slot`'s mod engine scale for this tick's step, reset to `1.0` as it is taken:
    /// `1.0` in every race without a module.
    pub(super) fn take_boost(&mut self, slot: usize) -> f32 {
        #[cfg(feature = "extensions")]
        {
            if self.on.slots.is_empty() {
                return 1.0;
            }
            std::mem::replace(&mut self.on.boost[slot], 1.0)
        }
        #[cfg(not(feature = "extensions"))]
        {
            let _ = slot;
            1.0
        }
    }

    /// Records a wall contact for `slot` this tick (at most once per slot per tick).
    pub(super) fn wall_contact(&mut self, slot: usize) {
        #[cfg(feature = "extensions")]
        self.on.wall_contact(slot);
        #[cfg(not(feature = "extensions"))]
        let _ = slot;
    }

    /// The modules' state into the race hash: nothing at all when no module runs.
    pub(super) fn hash_into(&self, hasher: &mut oag_core::hash::StateHasher) {
        #[cfg(feature = "extensions")]
        self.on.hash_into(hasher);
        #[cfg(not(feature = "extensions"))]
        let _ = hasher;
    }

    /// Module `index`'s state blob (id order), as its last call left it: for tests and
    /// tools; nothing simulated reads it from here.
    #[must_use]
    pub fn state(&self, index: usize) -> Option<&[u8]> {
        #[cfg(feature = "extensions")]
        {
            self.on.slots.get(index).map(|slot| slot.blob.as_slice())
        }
        #[cfg(not(feature = "extensions"))]
        {
            let _ = index;
            None
        }
    }

    /// The `fire-effect`s queued this tick, not yet drawn.
    #[cfg(all(test, feature = "extensions"))]
    pub(crate) fn effects_for_tests(&self) -> Vec<(usize, oag_title::effects::Trigger)> {
        self.on.effects.clone()
    }

    /// Module `index`'s state blob.
    #[cfg(all(test, feature = "extensions"))]
    pub(crate) fn blob_for_tests(&self, index: usize) -> Vec<u8> {
        self.on.slot_blob(index)
    }

    /// Each module's numbers after the last tick, in id order.
    #[must_use]
    pub fn reports(&self) -> Vec<Report> {
        #[cfg(feature = "extensions")]
        {
            self.on.reports()
        }
        #[cfg(not(feature = "extensions"))]
        {
            Vec::new()
        }
    }
}

impl crate::Race {
    /// Phase C: each module's `tick`, its commands into this tick's step. With no
    /// module, returns at once and touches nothing.
    pub(super) fn tick_extensions(&mut self, inputs: &oag_gameplay::PlayerInputs) {
        #[cfg(feature = "extensions")]
        on::tick(&mut self.sim, inputs);
        #[cfg(not(feature = "extensions"))]
        let _ = inputs;
    }

    /// The modules' `fire-effect`s, each at its craft's nozzle (the hull's back end),
    /// played from the title's own effect for that trigger; a title without one draws
    /// nothing (the load report names every absent effect).
    #[cfg(feature = "view")]
    pub(super) fn draw_extension_effects(&mut self) {
        for (slot, trigger) in self.sim.extensions.take_effects() {
            let Some(effect) = self.view.handles.get(trigger).cloned() else {
                continue;
            };
            let ship = &self.sim.world.ships[slot];
            let model = crate::drawable::model_matrix_of(ship);
            let tail = oag_core::math::Vec3::new(0.0, 0.0, ship.handling.dimensions.length * 0.5);
            self.view
                .stage
                .play(&effect, model.transform_point3(tail), 1.0);
        }
    }

    /// The race-wide outputs of the modules this tick (`set-sun`, `grade-sky`,
    /// `set-weather`): [`Sky::default`] in a race without a module.
    #[must_use]
    pub fn extension_sky(&self) -> Sky {
        self.sim.extensions.sky()
    }

    /// The modules' numbers for the dev overlay ([`Report`]).
    #[must_use]
    pub fn extension_reports(&self) -> Vec<Report> {
        self.sim.extensions.reports()
    }

    /// Module `index`'s state blob ([`Extensions::state`]).
    #[must_use]
    pub fn extension_state(&self, index: usize) -> Option<&[u8]> {
        self.sim.extensions.state(index)
    }

    /// Whether a simulation module runs in this race (MODDED: no replay, README 4.4).
    #[must_use]
    pub fn is_modded(&self) -> bool {
        self.sim.extensions.any()
    }
}

#[cfg(feature = "extensions")]
mod on {
    use oag_ext_abi::{Command, Event, MAX_EVENTS, Pickup};
    use oag_modhost::{Module, Observation};

    use super::Report;
    use crate::sim::RaceSim;

    #[derive(Debug, Clone)]
    pub(super) struct Slot {
        mod_id: String,
        module: Module,
        pub(super) blob: Vec<u8>,
        faulted: Option<(u64, String)>,
        fuel_used: u64,
        fuel_peak: u64,
        call_us: f32,
    }

    #[derive(Debug, Clone, Default)]
    pub(super) struct On {
        pub(super) slots: Vec<Slot>,
        /// One bit per slot that touched a wall since the last call: the event buffer,
        /// at most one `wall-contact` per craft per tick, raised in slot order.
        walls: u32,
        /// Each slot's engine scale above 1.0 for this tick's step
        /// (`Environment::extension_thrust`), set at the call and taken by the step.
        pub(super) boost: [f32; oag_gameplay::MAX_SHIPS],
        /// The module that owns the mod pickup kind (its index plus one, 0 for none):
        /// the first to give, clear or roll it. Another module doing so faults.
        owner: u8,
        /// The `pickup-roll` rule (weapon, every) and how many grants of that weapon
        /// it has counted.
        roll: Option<(oag_tables::weapons::Weapon, u8)>,
        rolled: u32,
        /// `pickup-fire` / `pickup-absorbed` raised since the last call, one bit a slot.
        fires: u32,
        absorbs: u32,
        /// The track as the modules read it.
        region: super::track::Region,
        /// Each speed pad's race-wide `scale-pad`, held until replaced.
        pad_scale: Vec<f32>,
        /// Each slot's `scale-pad` this tick.
        slot_pad_scale: [f32; oag_gameplay::MAX_SHIPS],
        /// The speed pad and the weapon pad each slot entered since the last call.
        speed_entered: [Option<u16>; oag_gameplay::MAX_SHIPS],
        weapon_entered: [Option<u16>; oag_gameplay::MAX_SHIPS],
        /// The triggers each slot is inside now, and was inside at the last call: the
        /// difference is the `trigger-enter` and `trigger-exit` events.
        inside: [u64; oag_gameplay::MAX_SHIPS],
        reported: [u64; oag_gameplay::MAX_SHIPS],
        /// Outputs, never hashed: each slot's `pickup-tint` this tick, and the
        /// `fire-effect`s the view has still to draw.
        pub(super) tints: [Option<[f32; 4]>; oag_gameplay::MAX_SHIPS],
        pub(super) effects: Vec<(usize, oag_title::effects::Trigger)>,
        /// The race-wide outputs of this tick.
        pub(super) sky: super::Sky,
    }

    impl On {
        pub(super) fn new(modules: Vec<(String, Module)>, region: super::track::Region) -> Self {
            Self {
                slots: modules
                    .into_iter()
                    .map(|(mod_id, module)| Slot {
                        blob: vec![0; module.limits().state_bytes as usize],
                        mod_id,
                        module,
                        faulted: None,
                        fuel_used: 0,
                        fuel_peak: 0,
                        call_us: 0.0,
                    })
                    .collect(),
                walls: 0,
                boost: [1.0; oag_gameplay::MAX_SHIPS],
                owner: 0,
                roll: None,
                rolled: 0,
                fires: 0,
                absorbs: 0,
                pad_scale: vec![1.0; region.speed_pads],
                region,
                slot_pad_scale: [1.0; oag_gameplay::MAX_SHIPS],
                speed_entered: [None; oag_gameplay::MAX_SHIPS],
                weapon_entered: [None; oag_gameplay::MAX_SHIPS],
                inside: [0; oag_gameplay::MAX_SHIPS],
                reported: [0; oag_gameplay::MAX_SHIPS],
                tints: [None; oag_gameplay::MAX_SHIPS],
                effects: Vec::new(),
                sky: super::Sky::default(),
            }
        }

        #[cfg(test)]
        pub(super) fn slot_blob(&self, index: usize) -> Vec<u8> {
            self.slots[index].blob.clone()
        }

        pub(super) fn rolls(&mut self, weapon: oag_tables::weapons::Weapon) -> bool {
            let Some((rolled, every)) = self.roll else {
                return false;
            };
            if weapon != rolled {
                return false;
            }
            self.rolled += 1;
            self.rolled.is_multiple_of(u32::from(every))
        }

        pub(super) fn pad_factor(&self, slot: usize, pad: usize) -> f32 {
            if self.slots.is_empty() {
                return 1.0;
            }
            self.pad_scale.get(pad).copied().unwrap_or(1.0) * self.slot_pad_scale[slot]
        }

        pub(super) fn pad_entered(&mut self, slot: usize, pad: usize, weapon: bool) {
            if self.slots.is_empty() {
                return;
            }
            let entered = if weapon {
                &mut self.weapon_entered
            } else {
                &mut self.speed_entered
            };
            entered[slot] = u16::try_from(pad).ok();
        }

        pub(super) fn test_triggers(&mut self, slot: usize, sweep: &[oag_core::math::Vec3]) {
            if self.slots.is_empty() || self.region.triggers.is_empty() {
                return;
            }
            self.inside[slot] = self
                .region
                .triggers
                .iter()
                .enumerate()
                .filter(|(_, b)| sweep.iter().any(|p| b.contains(p.to_array())))
                .fold(0, |mask, (i, _)| mask | 1 << i);
        }

        pub(super) fn pickup_used(&mut self, slot: usize, absorbed: bool) {
            if absorbed {
                self.absorbs |= 1 << slot;
            } else {
                self.fires |= 1 << slot;
            }
        }

        pub(super) fn wall_contact(&mut self, slot: usize) {
            if !self.slots.is_empty() {
                self.walls |= 1 << slot;
            }
        }

        pub(super) fn hash_into(&self, hasher: &mut oag_core::hash::StateHasher) {
            if self.slots.is_empty() {
                return;
            }
            hasher.write_u32(self.slots.len() as u32);
            hasher.write_u32(self.walls);
            for boost in self.boost {
                hasher.write_f32(boost);
            }
            hasher.write_u8(self.owner);
            match self.roll {
                None => hasher.write_u8(0),
                Some((weapon, every)) => {
                    hasher.write_u8(weapon as u8 + 1);
                    hasher.write_u8(every);
                }
            }
            hasher.write_u32(self.rolled);
            hasher.write_u32(self.fires);
            hasher.write_u32(self.absorbs);
            for scale in self.pad_scale.iter().chain(&self.slot_pad_scale) {
                hasher.write_f32(*scale);
            }
            for entered in self.speed_entered.iter().chain(&self.weapon_entered) {
                hasher.write_u32(entered.map_or(0, |pad| u32::from(pad) + 1));
            }
            for mask in self.inside.iter().chain(&self.reported) {
                hasher.write_u64(*mask);
            }
            for slot in &self.slots {
                hasher.write(&slot.blob);
                match &slot.faulted {
                    None => hasher.write_u8(0),
                    Some((tick, _)) => {
                        hasher.write_u8(1);
                        hasher.write_u64(*tick);
                    }
                }
            }
        }

        pub(super) fn reports(&self) -> Vec<Report> {
            self.slots
                .iter()
                .map(|slot| Report {
                    mod_id: slot.mod_id.clone(),
                    fuel_used: slot.fuel_used,
                    fuel_peak: slot.fuel_peak,
                    fuel_budget: slot.module.limits().fuel,
                    call_us: slot.call_us,
                    faulted: slot.faulted.clone(),
                })
                .collect()
        }
    }

    /// `play-cue`'s sounds as the engine's cues.
    fn cue(cue: oag_ext_abi::Cue) -> oag_gameplay::cue::Cue {
        use oag_ext_abi::Cue as Abi;
        use oag_gameplay::cue::Cue;
        match cue {
            Abi::SpeedupPad => Cue::SpeedupPad,
            Abi::Turbo => Cue::Turbo,
            Abi::Collision => Cue::Collision,
            Abi::Absorb => Cue::Absorb,
            Abi::Shield => Cue::Shield,
            Abi::ShieldActive => Cue::ShieldActive,
        }
    }

    /// `fire-effect`'s effects as the title's triggers.
    fn effect(effect: oag_ext_abi::Effect) -> oag_title::effects::Trigger {
        use oag_title::effects::Trigger;
        match effect {
            oag_ext_abi::Effect::CollisionSpark => Trigger::CollisionSpark,
            oag_ext_abi::Effect::HitSpark => Trigger::HitSpark,
        }
    }

    /// The ABI's weapon vocabulary and the tables' weapons, both ways. The tables' own
    /// `Extension` is the mod's pickup, which the ABI names [`Pickup::Ours`].
    fn pickup(weapon: oag_tables::weapons::Weapon) -> Pickup {
        use oag_ext_abi::Weapon as Abi;
        use oag_tables::weapons::Weapon;
        Pickup::Weapon(match weapon {
            Weapon::Rocket => Abi::Rocket,
            Weapon::Missile => Abi::Missile,
            Weapon::Quake => Abi::Quake,
            Weapon::Cannon => Abi::Cannon,
            Weapon::Turbo => Abi::Turbo,
            Weapon::Shield => Abi::Shield,
            Weapon::Autopilot => Abi::Autopilot,
            Weapon::Plasma => Abi::Plasma,
            Weapon::Bomb => Abi::Bomb,
            Weapon::Mine => Abi::Mine,
            Weapon::LeachBeam => Abi::LeachBeam,
            Weapon::Repulser => Abi::Repulser,
            Weapon::Shuriken => Abi::Shuriken,
            Weapon::Disruptor => Abi::Disruptor,
            Weapon::Extension => return Pickup::Ours,
        })
    }

    fn weapon(weapon: oag_ext_abi::Weapon) -> oag_tables::weapons::Weapon {
        use oag_ext_abi::Weapon as Abi;
        use oag_tables::weapons::Weapon;
        match weapon {
            Abi::Rocket => Weapon::Rocket,
            Abi::Missile => Weapon::Missile,
            Abi::Quake => Weapon::Quake,
            Abi::Cannon => Weapon::Cannon,
            Abi::Turbo => Weapon::Turbo,
            Abi::Shield => Weapon::Shield,
            Abi::Autopilot => Weapon::Autopilot,
            Abi::Plasma => Weapon::Plasma,
            Abi::Bomb => Weapon::Bomb,
            Abi::Mine => Weapon::Mine,
            Abi::LeachBeam => Weapon::LeachBeam,
            Abi::Repulser => Weapon::Repulser,
            Abi::Shuriken => Weapon::Shuriken,
            Abi::Disruptor => Weapon::Disruptor,
        }
    }

    /// The observation's craft records: copies of the world, never a pointer into it.
    fn crafts(sim: &RaceSim, inputs: &oag_gameplay::PlayerInputs) -> Vec<oag_ext_abi::Craft> {
        sim.world
            .ships
            .iter()
            .enumerate()
            .map(|(slot, ship)| {
                let person = sim.world.person_flown(slot);
                let fire_held = person
                    && inputs
                        .get(slot)
                        .buttons
                        .is_held(oag_gameplay::input::Button::Square);
                // Racing and past the start: a craft on the grid has no thrust yet.
                let racing = ship.physics.craft_state == oag_physics::damage::CraftState::Racing
                    && !oag_race::RaceState::thrust_gated(sim.world.tick);
                oag_ext_abi::Craft {
                    pickup: ship.pickup.weapon.map_or(Pickup::None, pickup),
                    person,
                    fire_held,
                    racing,
                    lap: u8::try_from(ship.standing.lap).unwrap_or(u8::MAX),
                    laps_target: sim
                        .world
                        .laps_target()
                        .map_or(0, |n| u8::try_from(n).unwrap_or(u8::MAX)),
                    shield: ship.physics.shield,
                    shield_max: ship.handling.dimensions.shield,
                    speed_kmh: ship.physics.body.linear_velocity.length()
                        * oag_core::math::SPEED_TO_KMH,
                }
            })
            .collect()
    }

    /// `shield-delta`'s sink: a loss through the weapon-damage path (so the destroyed
    /// edge, the shield pickup's immunity and the critical crossing behave as a hit's),
    /// at its full amount; a gain through the pool's clamp. A race whose rules take no
    /// damage (Time Trial, Speed Lap) refuses a loss: the mod cannot wreck a craft where
    /// nothing else can.
    fn shield_delta(sim: &mut RaceSim, slot: usize, amount: f32) {
        let rules = sim.damage_rules();
        let ship = &mut sim.world.ships[slot];
        let dimensions = ship.handling.dimensions;
        if amount >= 0.0 {
            oag_physics::damage::add(&mut ship.physics, &dimensions, amount);
        } else if rules.damage {
            oag_physics::damage::apply_weapon(
                &mut ship.physics,
                &dimensions,
                -amount,
                oag_physics::damage::DamageRules {
                    weapons: true,
                    ..rules
                },
            );
        }
    }

    /// `give-pickup`, `clear-pickup` and `pickup-roll` from module `index`: the first
    /// module to send one owns the mod pickup kind for the race, and another one sending
    /// one faults. A race without weapons gives nothing and rolls nothing.
    fn pickup_command(sim: &mut RaceSim, index: usize, command: Command) {
        let tick = sim.world.tick;
        let on = &mut sim.extensions.on;
        let me = index as u8 + 1;
        if on.owner != 0 && on.owner != me {
            let owner = on.slots[usize::from(on.owner - 1)].mod_id.clone();
            let module = &mut on.slots[index];
            if module.faulted.is_none() {
                let why = format!("the mod pickup belongs to mod {owner}");
                log::warn!(
                    "mod {}: {why}, at tick {tick}; off for the rest of the race",
                    module.mod_id
                );
                module.faulted = Some((tick, why));
            }
            return;
        }
        on.owner = me;
        if !sim.weapons_on {
            return;
        }
        let extension = oag_tables::weapons::Weapon::Extension;
        match command {
            // The slot's `last` stays what it was: the repeat rule is about the title's
            // own weapons.
            Command::GivePickup { slot } => {
                let pickup = &mut sim.world.ships[slot.index()].pickup;
                if pickup.weapon.is_none() {
                    pickup.weapon = Some(extension);
                }
            }
            Command::ClearPickup { slot } => {
                let pickup = &mut sim.world.ships[slot.index()].pickup;
                if pickup.weapon == Some(extension) {
                    pickup.take();
                }
            }
            Command::PickupRoll { weapon, every } => {
                let weapon = self::weapon(weapon);
                let on = &mut sim.extensions.on;
                if on.roll != Some((weapon, every)) {
                    on.roll = Some((weapon, every));
                    on.rolled = 0;
                }
            }
            _ => {}
        }
    }

    /// What happened since the last call, per slot in slot order: a wall contact, a
    /// fire, an absorb, the speed pad and the weapon pad entered, the triggers left and
    /// the triggers entered (each in index order). Past `MAX_EVENTS` the rest are
    /// dropped.
    fn events(on: &On) -> Vec<Event> {
        (0..oag_gameplay::MAX_SHIPS)
            .filter_map(oag_ext_abi::Slot::new)
            .flat_map(|slot| {
                let i = slot.index();
                let bit = |mask: u32| mask & slot.bit() != 0;
                let (inside, before) = (on.inside[i], on.reported[i]);
                let triggers = |mask: u64| (0..64_u16).filter(move |t| mask & (1 << t) != 0);
                [
                    bit(on.walls).then_some(Event::WallContact { slot }),
                    bit(on.fires).then_some(Event::PickupFire { slot }),
                    bit(on.absorbs).then_some(Event::PickupAbsorbed { slot }),
                    on.speed_entered[i].map(|pad| Event::SpeedPad { slot, pad }),
                    on.weapon_entered[i].map(|pad| Event::WeaponPad { slot, pad }),
                ]
                .into_iter()
                .flatten()
                .chain(
                    triggers(before & !inside)
                        .map(move |trigger| Event::TriggerExit { slot, trigger }),
                )
                .chain(
                    triggers(inside & !before)
                        .map(move |trigger| Event::TriggerEnter { slot, trigger }),
                )
            })
            .take(MAX_EVENTS)
            .collect()
    }

    pub(super) fn tick(sim: &mut RaceSim, inputs: &oag_gameplay::PlayerInputs) {
        if sim.extensions.on.slots.is_empty() {
            return;
        }
        let crafts = crafts(sim, inputs);
        let tick = sim.world.tick;
        let active = sim
            .world
            .ships
            .iter()
            .enumerate()
            .fold(0_u32, |mask, (slot, ship)| {
                mask | u32::from(ship.active) << slot
            });
        let on = &mut sim.extensions.on;
        // A boost the step never took (a craft that did not step) does not carry over.
        on.boost = [1.0; oag_gameplay::MAX_SHIPS];
        on.slot_pad_scale = [1.0; oag_gameplay::MAX_SHIPS];
        let events = events(on);
        on.walls = 0;
        on.fires = 0;
        on.absorbs = 0;
        on.speed_entered = [None; oag_gameplay::MAX_SHIPS];
        on.weapon_entered = [None; oag_gameplay::MAX_SHIPS];
        on.reported = on.inside;
        on.tints = [None; oag_gameplay::MAX_SHIPS];
        on.sky = super::Sky::default();
        // This tick's alone: a tick that draws nothing (a replayed one, a server's) drops
        // them rather than piling them up.
        on.effects.clear();
        let region = on.region.clone();
        let observation = Observation {
            tick,
            active,
            events: &events,
            crafts: &crafts,
            track: &region.bytes,
            speed_pads: region.speed_pads,
        };
        let mut commands = Vec::new();
        // Every module's checked commands, in id order: applied once all have run, so
        // each saw the same observation.
        let mut applied = Vec::new();
        for (index, slot) in on.slots.iter_mut().enumerate() {
            if slot.faulted.is_some() {
                slot.fuel_used = 0;
                continue;
            }
            // Timed for the overlay only; nothing simulated reads it.
            let started = web_time::Instant::now();
            let outcome = slot
                .module
                .tick(&observation, &mut slot.blob, &mut commands);
            slot.call_us = started.elapsed().as_secs_f32() * 1.0e6;
            let cost = match outcome {
                Ok(cost) => {
                    applied.extend(commands.drain(..).map(|command| (index, command)));
                    cost
                }
                Err((fault, cost)) => {
                    log::warn!(
                        "mod {}: {fault} at tick {tick}; off for the rest of the race",
                        slot.mod_id
                    );
                    slot.faulted = Some((tick, fault.to_string()));
                    cost
                }
            };
            slot.fuel_used = cost.fuel_used;
            slot.fuel_peak = slot.fuel_peak.max(cost.fuel_used);
        }
        for (index, command) in applied {
            match command {
                // Below 1.0 the Leach Beam's path, multiplied so a drain is kept; above it
                // the mod's own engine scale, multiplied so two modules compose. Both are
                // consumed by this tick's step.
                Command::ScaleThrust { slot, factor } if factor <= 1.0 => {
                    sim.world.ships[slot.index()].pending_thrust_scale *= factor;
                }
                Command::ScaleThrust { slot, factor } => {
                    let boost = &mut sim.extensions.on.boost[slot.index()];
                    *boost = (*boost * factor).min(oag_ext_abi::MAX_THRUST_FACTOR);
                }
                Command::ShieldDelta { slot, amount } => shield_delta(sim, slot.index(), amount),
                // Race-wide and held: the later module in id order wins.
                Command::ScalePad { pad, factor } => {
                    sim.extensions.on.pad_scale[usize::from(pad)] = factor;
                }
                // Per slot, this tick: two modules compose, bounded as a factor is.
                Command::ScaleSlotPads { slot, factor } => {
                    let scale = &mut sim.extensions.on.slot_pad_scale[slot.index()];
                    *scale = (*scale * factor).min(oag_ext_abi::MAX_PAD_FACTOR);
                }
                Command::PlayCue { slot, cue } => {
                    sim.cues.push(oag_gameplay::cue::CueEvent::new(
                        self::cue(cue),
                        slot.index(),
                    ));
                }
                Command::FireEffect { slot, effect } => sim
                    .extensions
                    .on
                    .effects
                    .push((slot.index(), self::effect(effect))),
                Command::PickupTint { slot, rgba } => {
                    let oag_ext_abi::Rgba { r, g, b, a } = rgba;
                    sim.extensions.on.tints[slot.index()] =
                        Some([r, g, b, a].map(|c| f32::from(c) / 255.0));
                }
                Command::SetWeather { strength, .. } => {
                    sim.extensions.on.sky.rain = Some(strength);
                }
                Command::SetSun { intensity } => sim.extensions.on.sky.sun = intensity,
                Command::GradeSky { grey } => sim.extensions.on.sky.grey = grey,
                Command::GivePickup { .. }
                | Command::ClearPickup { .. }
                | Command::PickupRoll { .. } => pickup_command(sim, index, command),
            }
        }
    }
}
