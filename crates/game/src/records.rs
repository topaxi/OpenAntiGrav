//! What a race leaves behind after the window closes: the best lap, the best
//! total time and the last result, per circuit, mode and speed class.
//!
//! Read from and written to `<config dir>/oag/records.toml` - a sibling of
//! [`crate::settings`]'s `settings.toml` and [`crate::pilots`]'s
//! `pilots/*.toml`, in the same directory, and following the shape those two
//! already established: `#[serde(default)]` on every field, so an older file
//! keeps loading as this grows, and no version key - the file migrates by
//! what shape it is, not by a number it carries.
//!
//! # Where this is captured, and where it is not
//!
//! **Never from inside [`crate::race::Race::tick`].** The simulation does not
//! know persistence exists - see `CLAUDE.md`'s core principle - so nothing
//! here is called from the gameplay crates at all. The composition root reads
//! [`crate::race::Race`]'s already-public state (`world`, `finished`,
//! `places`) from outside the tick, at two moments: the tick a race's own
//! finish condition is first true, and whenever a player leaves a race that
//! has not - `escape`, or an unfinished `--race` run quitting outright. That
//! second path is why this cannot be built as "capture the finish and nothing
//! else": [`oag_race::Mode::laps_target`] is `None` for `SpeedLap` and
//! `Zone`, so [`crate::race::Race::finished`] never turns `true` for either
//! of them - and Speed Lap is the one mode whose entire point is a fast lap
//! time. Leaving a race is the only exit every mode has in common, so a best
//! lap set there is captured on the way out rather than never.
//!
//! **What a hard quit (window close, a killed process) still loses**: a lap
//! set in Speed Lap or Zone between the last `escape`/finish and the crash.
//! Nothing here runs on a timer or a background thread - see the module doc's
//! opening rule again - so there is no cheaper fix than writing on every
//! personal-best lap, which was deliberately not built: it would turn a
//! several-times-a-race lap crossing into a filesystem write, for a case
//! (`kill -9` mid-lap) this project has no other autosave story for either.
//! Recorded as a known gap rather than silently accepted.
//!
//! # The schema is chosen, not measured, and carries no confidence score
//!
//! The original stores race results in its own PSP save-data format, on its
//! own trigger (`"Race End Save"` - see `crate::scoreboard`'s module doc for
//! where that state name was found and why its own screen is not built). This
//! project has no obligation to match that layout: nothing here loads an
//! original save, so there is no original shape to be unfaithful to. Every
//! field below, and the file format itself, is this project's own choice.
//!
//! # Deliberately not [`crate::settings::Settings`]'s own rule on a bad file
//!
//! `settings.rs` treats a malformed value as an error that reaches the
//! player rather than a silently discarded default - "a typo should be
//! visible, not swallowed". A settings file with one bad key is small enough
//! to read and fix, and the game does not start until it is.
//!
//! A records file is a different kind of loss: it is the only copy of
//! however many races' worth of times a player has driven, nobody edits it by
//! hand, and refusing to boot over one bad row would hold an unrelated race
//! hostage to a write this module made itself. So the rule here is the
//! opposite one, on purpose - the same kind of deliberate divergence
//! [`crate::pilots`]'s own module doc records for why *that* format does not
//! rewrite a hand-authored file:
//!
//! - **One malformed row does not lose the rest.** [`parse`] decodes the
//!   `records` array one entry at a time and keeps every row that decodes,
//!   dropping only the ones that do not - see its own doc.
//! - **A file that will not parse as TOML at all is moved aside, never
//!   overwritten in place.** [`load`] renames it to `records.toml.invalid`
//!   next to itself and starts empty, so whatever was in it is still on disk
//!   for a player or a future version to recover, and the next [`save`]
//!   cannot collide with the backup's name.
//! - **Nothing here ever panics.** [`load`] has no `Result` for exactly this
//!   reason: every failure it can hit - no config directory, a permissions
//!   error, a corrupt file - degrades to an empty [`Store`] plus a logged
//!   line, not a boot failure and not an `unwrap`.
//!
//! # Where a career system attaches
//!
//! [`Record`] carries a best lap, a best total time and the last result and
//! nothing else - no medal, no unlock, no tournament standing. Those are a
//! sibling member's own finding, not this pass's guess: a medal threshold or
//! a points value invented here would be exactly the kind of thing
//! `CLAUDE.md` forbids presenting as the original's behaviour. The shape
//! leaves two ways to grow when that lands, and both are additive:
//!
//! - A new **field** on [`Record`], `#[serde(default)]` like every field
//!   already here, for something that is still one number per
//!   circuit/mode/class - a medal tier, say.
//! - A new **sibling table** in the same file, alongside `[[records]]`, for
//!   something that is not shaped like this key at all - a tournament
//!   standing spans several circuits, not one.
//!
//! Neither needs [`Key`], [`parse`] or [`Store::record`] to change.
//!
//! # No circuit list is hardcoded here, and none should ever be added
//!
//! [`Key::track`] is whatever entry name the race that just ran actually
//! loaded - the disc's own `.vex` path, not a name this module invented or
//! enumerated. A title with no circuits played yet simply has no rows; the
//! store is populated by racing, the same way [`crate::pilots::Roster`] is
//! populated by a player's own files rather than by a list this project
//! ships.

use std::path::PathBuf;

use anyhow::{Context, Result};
use log::{error, warn};
use serde::{Deserialize, Serialize};

/// Which circuit/mode/class a [`Record`] is about.
///
/// Every part is lower-cased on construction - see [`Key::new`] - so a class
/// spelled `"Venom"` in one place and `"venom"` in another lands on the same
/// row rather than opening a second one silently. This is a lookup key, never
/// shown to a player, so the case that survives here is not the case a menu
/// would want to display.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    /// [`oag_title::Title::name`], lower-cased.
    pub title: String,
    /// The archive entry name of the circuit's `.vex`, exactly as
    /// [`crate::catalogue::Track::entry_name`] spells it, lower-cased.
    ///
    /// **The disc's own path, not [`crate::catalogue::Track::id`] alone.**
    /// The entry name already carries the reversed-direction distinction
    /// (`track.vex` against `track_reversed.vex`), so a forward and a
    /// reversed run of the same circuit are two rows rather than one
    /// overwriting the other - which is correct, they are different lines to
    /// drive.
    ///
    /// **Never left as the caller's own "nothing was named" value.**
    /// `race::Options::track` is `None` on the ordinary `--race` invocation
    /// with no `--track` flag, because `race::load` resolves its own
    /// default track *internally* and never hands the resolved name back -
    /// confirmed live, the first version of this fallback wrote every such
    /// race under one shared `"(no circuit)"` bucket regardless of which
    /// circuit was actually driven. `Stage::build_race_stage` now falls back
    /// to `title.race.track` (`oag_title::race::RaceDefaults::track`) first,
    /// itself the disc's own stated default and the same value `race::load`
    /// falls back to - so `"(no circuit)"` is reached only when a source
    /// offers no circuit at all. See [`Key::new`].
    pub track: String,
    /// [`oag_race::Mode::name`] - already lower-cased with underscores, so
    /// this is carried through unchanged.
    pub mode: String,
    /// The speed class, spelled the way [`crate::race::Options::class`]
    /// carries it, lower-cased.
    pub class: String,
}

/// Named for [`Key::track`]'s last-resort fallback: a race whose source
/// offered no circuit at all still has to record *something* rather than
/// fail the capture. See `Session::handle_menu`'s own "this source offers no
/// circuit at all" warning, which is the only place this can be reached
/// from now that `Stage::build_race_stage` falls back to `title.race.track`
/// first - see [`Key::track`]'s own doc.
const NO_CIRCUIT: &str = "(no circuit)";

impl Key {
    /// Builds a key, lower-casing every part - see the struct's own doc for
    /// why. `track` is `None` only on the path [`NO_CIRCUIT`] documents.
    #[must_use]
    pub fn new(title: &str, track: Option<&str>, mode: &str, class: &str) -> Self {
        Self {
            title: title.trim().to_ascii_lowercase(),
            track: track.unwrap_or(NO_CIRCUIT).trim().to_ascii_lowercase(),
            mode: mode.trim().to_ascii_lowercase(),
            class: class.trim().to_ascii_lowercase(),
        }
    }
}

/// One race's outcome, read off [`crate::race::Race`]'s already-public state
/// from *outside* the tick - see the module doc's "where this is captured"
/// section for why there are exactly two call sites and no others.
///
/// Deliberately primitives rather than a `&Race` or a `&Board`: this keeps
/// `records.rs` free of any dependency on `oag-race`, `oag-gameplay` or this
/// crate's own `race` module beyond what a caller already read off public
/// fields, which is what makes [`Store::record`] and [`laps_completed`]
/// testable with no disc, no GPU and no simulation step at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    /// Whether the race had reached its own finish condition - never
    /// re-derived here, always [`crate::race::Race::finished`]'s own answer.
    pub finished: bool,
    /// The player's own race position at the moment of capture, from
    /// [`crate::race::Race::places`]. Always `Some` for a race that had a
    /// [`crate::race::Race`] to read at all.
    pub place: Option<u8>,
    /// Laps completed - see [`laps_completed`], which is what every caller
    /// builds this field with.
    pub laps_completed: u32,
    /// The player's own finishing tick if they had one, or however far the
    /// clock had got when the race was left. Ticks, never a formatted
    /// string - `crate::hud::format_lap_time` takes ticks too, so nothing
    /// here forces a caller to format before it can persist.
    pub tick: u64,
    /// The player's own quickest completed lap, in ticks - `None` if they
    /// never finished one.
    pub best_lap_ticks: Option<u32>,
}

/// Laps completed, for the `laps_completed` field of an [`Observation`].
///
/// [`crate::scoreboard::build`]'s own rule, replicated here rather than
/// shared: that function takes a whole `Board` this module has no business
/// depending on - see [`Observation`]'s own doc - and the arithmetic is three
/// lines. A finished craft's own lap counter reads one past the target by the
/// time its finish tick is written, which is what the cap is for.
#[must_use]
pub fn laps_completed(lap: u32, finished: bool, laps_target: Option<u32>) -> u32 {
    match (finished, laps_target) {
        (true, Some(target)) => target,
        _ => lap.saturating_sub(1),
    }
}

/// One circuit/mode/class's whole history: the two records worth keeping
/// forever, and the last race run under this key.
///
/// `#[serde(default)]` on every field but the four key ones, which
/// [`parse`] would already refuse a row missing - so a field this build adds
/// later is absent rather than refused on a file an older version wrote, the
/// same promise `settings.rs`'s own `Settings` makes. See the module doc's
/// "chosen, not measured" and "where a career system attaches" sections.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// See [`Key::title`].
    pub title: String,
    /// See [`Key::track`].
    pub track: String,
    /// See [`Key::mode`].
    pub mode: String,
    /// See [`Key::class`].
    pub class: String,
    /// The quickest completed lap ever recorded on this row, in ticks.
    /// `None` until one race here completes one.
    #[serde(default)]
    pub best_lap_ticks: Option<u32>,
    /// The quickest *finished* race on this row, in ticks. `None` until a
    /// race here actually finishes - an abandoned Speed Lap contributes to
    /// [`Self::best_lap_ticks`] alone, never to this.
    #[serde(default)]
    pub best_total_ticks: Option<u64>,
    /// Whether the most recent race on this row finished.
    #[serde(default)]
    pub last_finished: bool,
    /// The most recent race's position, from [`Observation::place`].
    #[serde(default)]
    pub last_place: Option<u8>,
    /// The most recent race's completed laps.
    #[serde(default)]
    pub last_laps_completed: u32,
    /// The most recent race's own tick - the finish tick if it had one, the
    /// tick it was left on otherwise.
    #[serde(default)]
    pub last_tick: u64,
    /// The most recent race's own best lap, which may differ from
    /// [`Self::best_lap_ticks`] whenever this race was not the fastest one
    /// ever driven on this row.
    #[serde(default)]
    pub last_best_lap_ticks: Option<u32>,
}

impl Record {
    fn key(&self) -> Key {
        Key {
            title: self.title.clone(),
            track: self.track.clone(),
            mode: self.mode.clone(),
            class: self.class.clone(),
        }
    }
}

/// Every row [`load`] found or [`Store::record`] has added since, in one
/// file.
///
/// `records` is private so every write goes through [`Store::record`], which
/// is what keeps the array sorted - `save` writes it back in a stable order
/// on every run, the same reason [`crate::settings::Settings::render_profiles`]
/// is a `BTreeMap` rather than a `HashMap`: an unordered rewrite is a diff
/// with nothing changed in it every single launch.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Store {
    records: Vec<Record>,
}

impl Store {
    /// Every row, in the file's own stable order.
    #[must_use]
    pub fn rows(&self) -> &[Record] {
        &self.records
    }

    /// The row `key` names, if one exists yet.
    #[must_use]
    pub fn get(&self, key: &Key) -> Option<&Record> {
        self.records.iter().find(|record| record.key() == *key)
    }

    /// Merges one race's [`Observation`] into the row `key` names, creating
    /// it if this is the first result ever recorded there.
    ///
    /// **Best-of, never overwritten downward.** [`Record::best_lap_ticks`]
    /// and [`Record::best_total_ticks`] only ever shrink, and the second
    /// stays untouched entirely when `obs.finished` is `false` - an
    /// abandoned race has no total time to compare. The `last_*` fields
    /// always take `obs`'s own values, whatever they say: that is what "the
    /// last race" means, better or worse than the one before it.
    pub fn record(&mut self, key: Key, obs: Observation) {
        let row = match self.records.iter_mut().find(|record| record.key() == key) {
            Some(row) => row,
            None => {
                self.records.push(Record {
                    title: key.title,
                    track: key.track,
                    mode: key.mode,
                    class: key.class,
                    ..Record::default()
                });
                self.records
                    .last_mut()
                    .expect("just pushed onto this exact vec")
            }
        };
        if let Some(lap) = obs.best_lap_ticks {
            row.best_lap_ticks = Some(row.best_lap_ticks.map_or(lap, |best| best.min(lap)));
        }
        if obs.finished {
            row.best_total_ticks = Some(
                row.best_total_ticks
                    .map_or(obs.tick, |best| best.min(obs.tick)),
            );
        }
        row.last_finished = obs.finished;
        row.last_place = obs.place;
        row.last_laps_completed = obs.laps_completed;
        row.last_tick = obs.tick;
        row.last_best_lap_ticks = obs.best_lap_ticks;

        // Stable, so the file `save` writes is the same shape every run bar
        // the numbers that actually changed - see the struct's own doc.
        self.records.sort_by_key(Record::key);
    }
}

/// Parses `text` as a records file, keeping every row that decodes and
/// noting - never discarding the whole file over - one that does not.
///
/// **A malformed row is not the same failure as a malformed document.** This
/// walks the `records` array one entry at a time; a row missing one of the
/// four key fields, or carrying a value of the wrong shape, is dropped and
/// named in the returned notes, and every other row is kept. See the module
/// doc's "deliberately not `Settings`'s own rule" section for why that is the
/// opposite of what `settings.rs` does on purpose.
///
/// # Errors
///
/// Only when `text` is not valid TOML *at all* - there is no `records` array
/// to salvage rows from at that point. [`load`] is what decides what happens
/// to the file when that happens; this function only reports it.
pub fn parse(text: &str) -> Result<(Store, Vec<String>)> {
    let table: toml::Table = text.parse().context("parsing records")?;
    let mut notes = Vec::new();
    let mut records = Vec::new();
    match table.get("records") {
        Some(toml::Value::Array(rows)) => {
            for (index, row) in rows.iter().enumerate() {
                match row.clone().try_into::<Record>() {
                    Ok(record) => records.push(record),
                    Err(e) => notes.push(format!("records[{index}] dropped: {e}")),
                }
            }
        }
        // Absent is the ordinary case of a fresh file; present-but-wrong-shape
        // is a hand edit or a future format this build cannot read, and
        // starts empty the same way rather than refusing to boot over it.
        Some(_) => notes.push("`records` is not an array of tables; ignoring it".to_string()),
        None => {}
    }
    records.sort_by_key(Record::key);
    Ok((Store { records }, notes))
}

/// Where the records file lives: `<config dir>/oag/records.toml`, beside
/// `settings.toml` and the `pilots/` directory.
#[must_use]
pub fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("oag").join("records.toml"))
}

const HEADER: &str = "\
# OpenAntiGrav race records - best lap, best total time and the last result,
# per circuit, mode and speed class.
#
# This project's own schema, not the original's save format - see
# `crates/game/src/records.rs`'s module doc. Rewritten on every change with
# every row present; a row missing a key this build does not recognise is
# dropped rather than failing the file, and hand edits are not preserved
# across a rewrite the way a pilot file's comments are.

";

/// Loads the records file, or starts empty when there is none yet, it cannot
/// be read, or it will not parse.
///
/// **Never fails, and never panics** - see the module doc's own section on
/// why this diverges from [`crate::settings::load`]. Every failure this can
/// hit degrades to [`Store::default`] plus a logged line:
///
/// - No config directory on this platform: silent, the same case
///   [`crate::settings::load`] and [`crate::pilots::load`] both treat as
///   "nothing to persist to" rather than an error.
/// - No file yet: silent, the ordinary case of a player who has not finished
///   a race since installing this.
/// - The file exists and will not read (permissions, mid-write on another
///   process): logged, starts empty.
/// - The file reads but is not valid TOML: logged, moved aside to
///   `records.toml.invalid` next to itself rather than overwritten in place,
///   starts empty. A second bad file in a row simply replaces the first
///   `.invalid` backup - this keeps one, not a growing pile.
/// - The file is valid TOML but one or more rows do not decode: logged per
///   row, every other row is kept. This is the ordinary partial-corruption
///   case the whole design exists for.
#[must_use]
pub fn load() -> Store {
    let Some(path) = path() else {
        return Store::default();
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Store::default(),
        Err(e) => {
            error!(
                "could not read {}: {e:#}; starting with no race records",
                path.display()
            );
            return Store::default();
        }
    };
    match parse(&text) {
        Ok((store, notes)) => {
            for note in &notes {
                warn!("{}: {note}", path.display());
            }
            store
        }
        Err(e) => {
            let backup = path.with_file_name(format!(
                "{}.invalid",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
            match std::fs::rename(&path, &backup) {
                Ok(()) => error!(
                    "{} would not parse ({e:#}); moved aside to {} and starting with no race \
                     records",
                    path.display(),
                    backup.display()
                ),
                Err(rename_err) => error!(
                    "{} would not parse ({e:#}) and could not be moved aside ({rename_err:#}); \
                     starting with no race records, but the broken file is untouched",
                    path.display()
                ),
            }
            Store::default()
        }
    }
}

/// Writes `store` back to [`path`], creating the directory if this is the
/// first record saved on this machine.
///
/// # Errors
///
/// Propagates a directory that cannot be created and a file that cannot be
/// written - the same contract [`crate::settings::save`] makes, and the same
/// thing the caller does with the failure: log it and carry on, never panic
/// or abort a race over a write that did not land.
pub fn save(store: &Store) -> Result<()> {
    let Some(path) = path() else {
        return Ok(());
    };
    let text = format!(
        "{HEADER}{}",
        toml::to_string_pretty(store).context("serialising race records")?
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests;
