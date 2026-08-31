//! Zone mode's announcer: the milestone voice lines a title calls out as the
//! run passes each threshold in its own ladder.
//!
//! Split out of [`super`] for the same reason [`super::Engine`] is: this is a
//! law with its own load step and its own name lookup, not a cue drawn from
//! [`super::Cue`]'s closed, statically-named set. It cannot be one of those -
//! [`oag_title::ZoneAnnouncer`]'s own docs lay out why the milestone ladder is
//! per-title data (Pulse, Pure and Wipeout HD each ship a different one) rather
//! than an engine-wide constant, and [`super::Cue::ALL`] is sized for a fixed
//! set of names known at compile time.
//!
//! See `docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`
//! and [`zone-mode.md`](../../../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md#the-ten-second-step)
//! for the evidence: confidence 95 that each named milestone's bank cue exists
//! and decodes, confidence 75 that the run's own zone counter reaching that
//! number is the trigger - traced end to end only on Pulse, where the
//! milestone table's thresholds and the bank's cue order agree ordinally but
//! the final dispatch sits in code outside this project's Ghidra database.
//!
//! A title with no [`oag_title::ZoneAnnouncer`] - Wipeout 2048 today, whose
//! `data.psarc` is not extracted in this tree - loads an empty announcer and
//! every milestone plays nothing, the same degrade [`super::Banks`] gives a
//! cue whose bank did not resolve.
//!
//! [`ClassAnnouncer`] is the sibling of this milestone ladder: same shape,
//! same never-fails load, keyed by speed-class stage instead of zone-count
//! milestone. See [`oag_title::ZoneClassAnnouncer`].

use std::collections::BTreeMap;
use std::sync::Arc;

use oag_assets::source::Archives;
use oag_audio::Sound;
use oag_core::Rng;
use oag_formats::sblk;

use super::{Loaded, load_named_cue};

/// One title's decoded Zone announcer: every milestone its ladder names that
/// actually decoded, keyed by the zone number that plays it.
#[derive(Debug, Default, Clone)]
pub struct Announcer {
    lines: BTreeMap<u16, Loaded>,
    /// What loading did, for the race's own report - the same shape
    /// [`super::Banks::report`] keeps.
    pub report: Vec<String>,
}

impl Announcer {
    /// Reads and decodes every milestone [`oag_title::ZoneAnnouncer`] names,
    /// when this title has one.
    ///
    /// **Never fails.** A title with no announcer, a bank that will not read,
    /// or one numbered cue that will not decode are all report lines and
    /// silence for that milestone, on the same terms `Banks::load` already
    /// keeps for its own cues - nothing here stops a race from starting.
    #[must_use]
    pub fn load(archives: &mut Archives, table: Option<&oag_title::ZoneAnnouncer>) -> Self {
        let mut lines = BTreeMap::new();
        let mut report = Vec::new();

        let Some(table) = table else {
            return Self { lines, report };
        };

        let blob = match archives.read_name(table.bank) {
            Ok(blob) => blob,
            Err(e) => {
                report.push(format!("announcer: {} not read: {e}", table.bank));
                return Self { lines, report };
            }
        };
        let bank = match sblk::Bank::parse(&blob) {
            Ok(bank) => bank,
            Err(e) => {
                report.push(format!("announcer: {} not decoded: {e}", table.bank));
                return Self { lines, report };
            }
        };

        for &milestone in table.milestones {
            let name = oag_title::ZoneAnnouncer::cue_name(milestone);
            match load_named_cue(&bank, &name) {
                Ok((loaded, skipped)) => {
                    let undecoded = if skipped == 0 {
                        String::new()
                    } else {
                        format!(", {skipped} skipped as not PS-ADPCM")
                    };
                    report.push(format!(
                        "announcer: {name} -> {} waveform(s){undecoded}",
                        loaded.waveforms.len()
                    ));
                    lines.insert(milestone, loaded);
                }
                Err(e) => report.push(format!("announcer: {name} not loaded: {e}")),
            }
        }

        Self { lines, report }
    }

    /// Whether this title's ladder decoded no milestones at all - no
    /// [`oag_title::ZoneAnnouncer`], a bank that would not read, or every
    /// numbered cue failing to decode.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// One waveform for a milestone's line, chosen by `rng` when it has
    /// alternates - `None` when this milestone's cue did not load, or this
    /// title names no milestone by that number at all.
    #[must_use]
    pub fn pick(&self, milestone: u16, rng: &mut Rng) -> Option<(Arc<Sound>, bool)> {
        let loaded = self.lines.get(&milestone)?;
        let index = match u32::try_from(loaded.waveforms.len()) {
            Ok(len) if len > 1 => rng.below(len) as usize,
            _ => 0,
        };
        let (sound, looping) = &loaded.waveforms[index];
        Some((Arc::clone(sound), *looping))
    }
}

/// One title's decoded Zone **speed-class** announcer - [`Announcer`]'s
/// sibling, keyed by stage number rather than zone-number milestone.
///
/// See [`oag_title::ZoneClassAnnouncer`] for the evidence a call site plays
/// this is on the same footing [`Announcer`]'s own milestone ladder already
/// stands on for this title, and for why the trigger this fires on is the
/// same [`oag_title::ZoneStages::stage_for`] edge the HUD text and colour
/// grade already key off, computed by the caller rather than here - this
/// type only knows which cue a stage number wants, not when a race reaches
/// one.
#[derive(Debug, Default, Clone)]
pub struct ClassAnnouncer {
    lines: BTreeMap<u32, Loaded>,
    /// Same shape as [`Announcer::report`].
    pub report: Vec<String>,
}

impl ClassAnnouncer {
    /// Reads and decodes every stage [`oag_title::ZoneClassAnnouncer`] names,
    /// when this title has one. Never fails - see [`Announcer::load`].
    #[must_use]
    pub fn load(archives: &mut Archives, table: Option<&oag_title::ZoneClassAnnouncer>) -> Self {
        let mut lines = BTreeMap::new();
        let mut report = Vec::new();

        let Some(table) = table else {
            return Self { lines, report };
        };

        let blob = match archives.read_name(table.bank) {
            Ok(blob) => blob,
            Err(e) => {
                report.push(format!("class announcer: {} not read: {e}", table.bank));
                return Self { lines, report };
            }
        };
        let bank = match sblk::Bank::parse(&blob) {
            Ok(bank) => bank,
            Err(e) => {
                report.push(format!("class announcer: {} not decoded: {e}", table.bank));
                return Self { lines, report };
            }
        };

        for stage in 1..=u32::try_from(table.classes.len()).unwrap_or(0) {
            // `table.cue_name` cannot fail inside this range - it is the same
            // arithmetic the loop bound is built from - so an early `continue`
            // here would never fire; kept as `let Some` anyway rather than an
            // `unwrap`, on the same "never trust a length twice" terms
            // `load_named_cue`'s own callers already keep.
            let Some(name) = table.cue_name(stage) else {
                continue;
            };
            match load_named_cue(&bank, name) {
                Ok((loaded, skipped)) => {
                    let undecoded = if skipped == 0 {
                        String::new()
                    } else {
                        format!(", {skipped} skipped as not PS-ADPCM")
                    };
                    report.push(format!(
                        "class announcer: {name} (stage {stage}) -> {} waveform(s){undecoded}",
                        loaded.waveforms.len()
                    ));
                    lines.insert(stage, loaded);
                }
                Err(e) => report.push(format!("class announcer: {name} not loaded: {e}")),
            }
        }

        Self { lines, report }
    }

    /// Whether this title's ladder decoded no speed classes at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// One waveform for `stage`'s line, chosen by `rng` when it has
    /// alternates - `None` when this stage's cue did not load, or this title
    /// names no cue for it at all (stage `0`, `Start`, on every title
    /// measured so far).
    #[must_use]
    pub fn pick(&self, stage: u32, rng: &mut Rng) -> Option<(Arc<Sound>, bool)> {
        let loaded = self.lines.get(&stage)?;
        let index = match u32::try_from(loaded.waveforms.len()) {
            Ok(len) if len > 1 => rng.below(len) as usize,
            _ => 0,
        };
        let (sound, looping) = &loaded.waveforms[index];
        Some((Arc::clone(sound), *looping))
    }
}
