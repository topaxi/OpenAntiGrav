//! Simulation mods in a race: the one call per tick, the event buffer, the command
//! sinks and each module's state blob (`docs/architecture/wasm-extensions/hooks.md`).
//!
//! - **Event**: `wall-contact{slot}`, recorded by [`Race::count_wall_contact`] for every
//!   racing craft, a person's or the AI's, on each tick it touches a wall (the same test
//!   [`RaceSim::wall_contact_ticks`] counts). Raised after the call, so it reaches the
//!   guest in the next tick's observation: a one-tick latency, identical everywhere.
//! - **Event**: `hit{slot, amount, weapon}`, folded in by [`Extensions::weapon_hits`] from the
//!   `WeaponHit` each weapon site fills, a hit a Shield absorbed included; one per craft
//!   per tick, delivered at the next call. Every hit source runs after the call.
//! - **Commands**: `give-shield` (the title's Shield for its authored time, never
//!   refreshing a running one), `hold-shield` (floors the timer at two ticks) and
//!   `clear-shield` (zeroes it), all landing at the call, before the step.
//! - **The call**: [`Race::tick_extensions`], between `advance_cannons` and the people's
//!   steps (phase C of `hooks.md`). Modules run in id order; each sees the same events.
//! - **Command**: `scale-thrust{slot, factor}`, `factor` in `[0, 1]`, **multiplied** into
//!   `Ship::pending_thrust_scale`, which this tick's step consumes for a person's craft
//!   (`player_step.rs`) and an AI craft (`field.rs`) alike. Multiplied, not assigned, so a
//!   Leach Beam drain armed at the end of the last tick is kept.
//! - **Track**: the region [`prepare`] lays out at load for a guest's `oag_track` buffer
//!   (pads, triggers, author-defined properties; `track`). Pad entries and trigger
//!   enters and exits are events; `scale-pad` is read by `Race::test_speedup_pads`.
//! - **State**: each module's blob and fault tick live in [`RaceSim`], cloned with a
//!   snapshot and hashed by [`Extensions::hash_into`]. A race with no module hashes
//!   nothing here, so every vanilla hash is unchanged.
//!
//! Behind the `extensions` cargo feature: off, [`prepare`] refuses every source by name
//! and nothing of wasmtime is built or linked.

use std::sync::Arc;

#[cfg(feature = "extensions")]
mod on;
mod route;
#[cfg(feature = "extensions")]
mod shield;
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
    /// The mod's own text for a language: its `strings/<language>.toml` over its
    /// `strings/english.toml`, by string id (the same files its menu rows use).
    pub strings: StringsReader,
    /// The mod's `[icons]`: name and a reader for the PNG.
    pub icons: Vec<(String, Reader)>,
    /// The mod's `[effects]`, in name order: name and the TOML's path in the mod. A
    /// module's `fire-mod-effect` names one by its place in this list.
    pub effects: Vec<(String, String)>,
    /// Reads any file of the mod by its path (an effect's TOML and PNGs).
    pub files: Files,
    /// The mod's `[pickup]`: the look of the pickup it hands out. A mod without one that
    /// sends `give-pickup` or `pickup-roll` is faulted for the race, naming the table.
    pub pickup: Option<oag_tables::pickup_look::PickupLook>,
}

/// Reads a file of a mod by its path in the mod, on the load thread.
pub type Files = Arc<dyn Fn(&str) -> Result<Vec<u8>, String> + Send + Sync>;

/// A mod's own text by string id.
pub type Strings = std::collections::BTreeMap<String, String>;

/// Reads a mod's [`Strings`] for the game's language (`None`: English alone), on the
/// load thread. A language the mod ships no file for gives English's.
pub type StringsReader = Arc<dyn Fn(Option<&str>) -> Strings + Send + Sync>;

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
    /// Each module's mod's own text for the race's language, in module order.
    #[cfg(feature = "extensions")]
    strings: Vec<Strings>,
    /// Each module's mod's `[pickup]`, in module order.
    #[cfg(feature = "extensions")]
    pickups: Vec<Option<oag_tables::pickup_look::PickupLook>>,
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
    language: Option<&str>,
    report: &mut Vec<String>,
) -> Prepared {
    let sources: Vec<&Source> = sources
        .iter()
        .filter(|s| s.titles.is_empty() || s.titles.iter().any(|t| t.eq_ignore_ascii_case(title)))
        .filter(|s| s.scope.runs_on(foreign_track))
        .collect();
    #[cfg(not(feature = "extensions"))]
    {
        let _ = (track, language);
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
                    format!(
                        "mods: track {} carries {e}; the race runs no simulation mod",
                        track.name
                    ),
                );
                return Prepared::default();
            }
        };
        let mut sorted = sources;
        sorted.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
        let mut modules = Vec::new();
        let mut strings = Vec::new();
        let mut pickups = Vec::new();
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
                    strings.push((source.strings)(language));
                    pickups.push(source.pickup.clone());
                }
                Err(e) => push(report, format!("{label} left out: {e}")),
            }
        }
        Prepared {
            modules,
            track_module,
            region,
            strings,
            pickups,
        }
    }
}

/// The PNGs of the `[icons]` of every mod that is on for this race (`title`, and
/// `foreign_track` for a track module), decoded for the HUD's sheet under the keys
/// [`mod_icon_src`] gives. Read on the load thread with the rest of the load. One that
/// is missing or does not decode draws nothing, and `report` names it.
#[cfg(feature = "view")]
#[must_use]
pub fn mod_icons(
    sources: &[Source],
    title: &str,
    foreign_track: Option<&std::path::Path>,
    report: &mut Vec<String>,
) -> Vec<oag_hud::sprite::DecodedImage> {
    #[cfg(not(feature = "extensions"))]
    {
        let _ = (sources, title, foreign_track, report);
        Vec::new()
    }
    #[cfg(feature = "extensions")]
    {
        let mut seen = std::collections::BTreeSet::new();
        let mut out = Vec::new();
        for source in for_race(sources, title, foreign_track) {
            if !seen.insert(source.mod_id.clone()) {
                continue;
            }
            for (name, read) in &source.icons {
                let label = format!("mod {}: icon {name}", source.mod_id);
                match read().and_then(|bytes| decode_png(&bytes)) {
                    Ok((width, height, rgba)) => out.push(oag_hud::sprite::DecodedImage {
                        src: mod_icon_src(&source.mod_id, name),
                        width,
                        height,
                        rgba,
                        quad_extent: None,
                        quad_centre: None,
                        blend: None,
                    }),
                    Err(e) => push(report, format!("{label} left out: {e}; draws nothing")),
                }
            }
        }
        out
    }
}

/// The sources of the mods that are on for a race of `title` (and `foreign_track`, for a
/// track module).
#[cfg(all(feature = "view", feature = "extensions"))]
pub(crate) fn for_race<'a>(
    sources: &'a [Source],
    title: &'a str,
    foreign_track: Option<&'a std::path::Path>,
) -> impl Iterator<Item = &'a Source> {
    sources
        .iter()
        .filter(move |s| {
            s.titles.is_empty() || s.titles.iter().any(|t| t.eq_ignore_ascii_case(title))
        })
        .filter(move |s| s.scope.runs_on(foreign_track))
}

/// A PNG as RGBA8: `(width, height, pixels)`.
#[cfg(all(feature = "view", feature = "extensions"))]
pub(crate) fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let frame = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let pixels = &buf[..frame.buffer_size()];
    let rgba: Vec<u8> = match frame.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 0xff])
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 0xff]).collect(),
        png::ColorType::Indexed => return Err("an indexed PNG survived expansion".into()),
    };
    Ok((frame.width, frame.height, rgba))
}

/// The sheet key of a mod's own icon.
#[must_use]
pub fn mod_icon_src(mod_id: &str, name: &str) -> String {
    format!("mod:{mod_id}/{name}")
}

/// The text the string id `key` names in a mod's `strings`; `None` when it has none,
/// and the item draws nothing.
#[must_use]
pub fn resolve_string(strings: Option<&Strings>, key: &str) -> Option<String> {
    strings?.get(key).cloned()
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
                on: on::On::new(
                    prepared.modules,
                    prepared.region,
                    prepared.strings,
                    prepared.pickups,
                ),
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

    /// `slot`'s `hud-*` outputs this tick, for the HUD: recolours, shakes, a bar's added
    /// share, text and icons. Quiet without a module. Never hashed.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn hud(&self, slot: usize, tick: u64) -> oag_hud::ext_draws::Outputs {
        #[cfg(feature = "extensions")]
        {
            self.on.hud_outputs(slot, tick)
        }
        #[cfg(not(feature = "extensions"))]
        {
            let _ = (slot, tick);
            oag_hud::ext_draws::Outputs::default()
        }
    }

    /// The look of `slot`'s held mod pickup this tick, for the HUD to keep while the pickup
    /// fades out after its use. Never hashed.
    #[must_use]
    pub fn pickup_look(&self, slot: usize) -> Option<oag_tables::pickup_look::PickupLook> {
        #[cfg(feature = "extensions")]
        {
            self.on.pickup_look(slot)
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
    pub(super) fn take_effects(&mut self) -> Vec<(usize, crate::extension_effects::Fired)> {
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

    /// Records this step's weapon hits for the modules' `hit` event.
    pub(super) fn weapon_hits(&mut self, hits: &[oag_weapons::projectile::WeaponHit]) {
        #[cfg(feature = "extensions")]
        self.on.weapon_hits(hits);
        #[cfg(not(feature = "extensions"))]
        let _ = hits;
    }

    /// The slots a `give-shield` started this tick, one bit each, cleared as taken.
    #[cfg(feature = "extensions")]
    pub(super) fn take_shield_gives(&mut self) -> u32 {
        std::mem::take(&mut self.on.gave)
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
    pub(crate) fn effects_for_tests(&self) -> Vec<(usize, crate::extension_effects::Fired)> {
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
        {
            on::tick(&mut self.sim, inputs);
            let gave = self.sim.extensions.take_shield_gives();
            for slot in (0..oag_gameplay::MAX_SHIPS).filter(|slot| gave & (1 << slot) != 0) {
                self.shield_activate(slot);
            }
        }
        #[cfg(not(feature = "extensions"))]
        let _ = inputs;
    }

    /// The modules' `fire-effect`s, each at its craft's nozzle (the hull's back end),
    /// played from the title's own effect for that trigger or the mod's own effect file;
    /// one that is absent draws nothing (the load report names every one).
    #[cfg(feature = "view")]
    pub(super) fn draw_extension_effects(&mut self) {
        for (slot, fired) in self.sim.extensions.take_effects() {
            self.play_fired(slot, &fired);
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
