//! Zone mode's announcer: the milestone voice lines a title calls out as the run
//! passes each threshold in its own ladder.
//!
//! Split out of [`super`] like [`super::Engine`]: a law with its own load step
//! and name lookup, not a cue from [`super::Cue`]'s closed set.
//! [`oag_title::ZoneAnnouncer`] explains why the ladder is per-title data (Pulse,
//! Pure and HD each ship a different one).
//!
//! Evidence: `docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`
//! and [`zone-mode.md`](../../../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md#the-ten-second-step):
//! confidence 95 that each named milestone's bank cue exists and decodes,
//! confidence 75 that the zone counter reaching that number is the trigger,
//! traced end to end only on Pulse (the table's thresholds and the bank's cue
//! order agree ordinally, but the final dispatch is outside this project's
//! Ghidra database).
//!
//! A title with no [`oag_title::ZoneAnnouncer`] (2048 today, whose `data.psarc`
//! is not extracted here) loads an empty announcer and plays nothing, as
//! [`super::Banks`] does for an unresolved cue.
//!
//! [`ClassAnnouncer`] is the sibling keyed by speed-class stage, see
//! [`oag_title::ZoneClassAnnouncer`].

use std::collections::BTreeMap;
use std::sync::Arc;

use oag_assets::source::Archives;
use oag_audio::Sound;
use oag_core::Rng;
use oag_formats::sblk;

use super::compose::compose_sequence;
use super::layers::{self, CueVoice, Timeline};
use super::{Loaded, load_named_cue};

/// One title's decoded Zone announcer: every milestone its ladder names that
/// decoded, keyed by the zone number that plays it.
#[derive(Debug, Default, Clone)]
pub struct Announcer {
    lines: BTreeMap<u16, Loaded>,
    /// What loading did, for the race's report (as [`super::Banks::report`]).
    pub report: Vec<String>,
}

impl Announcer {
    /// Reads and decodes every milestone [`oag_title::ZoneAnnouncer`] names.
    ///
    /// Never fails: no announcer, an unreadable bank or one undecodable cue is a
    /// report line and silence for that milestone, as in `Banks::load`.
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
            // A sequence is played as one, not sampled: Pulse's `zone_N` is ZONE,
            // the number and CLEAR on authored delays (`super::compose`). Any other
            // shape keeps the flat pick below.
            match compose_sequence(&bank, &name, table.tick) {
                Ok(Some(line)) => {
                    report.push(format!(
                        "announcer: {name} -> sequence of {} grain(s), {:.2}s",
                        line.grains,
                        line.sound.seconds()
                    ));
                    lines.insert(
                        milestone,
                        Loaded {
                            waveforms: vec![(line.sound, false)],
                        },
                    );
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    report.push(format!("announcer: {name} not composed: {e}"));
                    continue;
                }
            }
            match load_named_cue(&bank, &name) {
                Ok((loaded, skipped)) => {
                    let undecoded = if skipped == 0 {
                        String::new()
                    } else {
                        format!(", {skipped} skipped: decoded to no samples")
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

    /// Whether this title's ladder decoded no milestones at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// One waveform for a milestone's line, chosen by `rng` among alternates;
    /// `None` when that cue did not load or the title names no such milestone.
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

/// One title's decoded Zone speed-class announcer, [`Announcer`]'s sibling keyed
/// by stage number. [`oag_title::ZoneClassAnnouncer`] holds the evidence for the
/// call site; the trigger is the [`oag_title::ZoneStages::stage_for`] edge the
/// HUD text and colour grade key off, computed by the caller.
#[derive(Debug, Default, Clone)]
pub struct ClassAnnouncer {
    lines: BTreeMap<u32, Loaded>,
    /// The tone every class step plays beside its line,
    /// [`oag_title::ZoneClassAnnouncer::tone`]: its timeline variants, or
    /// [`Self::tone_flat`] where no timeline was built.
    tone: Vec<Timeline>,
    tone_flat: Option<Loaded>,
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
            return Self {
                lines,
                report,
                ..Self::default()
            };
        };

        if let Some(blob) = read_bank(archives, table.bank, &mut report)
            && let Some(bank) = parse_bank(&blob, table.bank, &mut report)
        {
            for stage in 1..=u32::try_from(table.classes.len()).unwrap_or(0) {
                let Some(name) = table.cue_name(stage) else {
                    continue;
                };
                if let Some(loaded) =
                    load_reported(&bank, name, &format!("{name} (stage {stage})"), &mut report)
                {
                    lines.insert(stage, loaded);
                }
            }
        }

        let (tone, tone_flat) = table
            .tone
            .map(|tone| load_tone(archives, tone, &mut report))
            .unwrap_or_default();

        Self {
            lines,
            tone,
            tone_flat,
            report,
        }
    }

    /// The voices of one class-step tone, or `None` when the title names none
    /// or it did not load.
    #[must_use]
    pub fn tone_voices(&self, rng: &mut Rng) -> Option<Vec<CueVoice>> {
        let index = match u32::try_from(self.tone.len()) {
            Ok(0) | Err(_) => {
                let (sound, looping) = pick(self.tone_flat.as_ref()?, rng)?;
                return Some(vec![CueVoice::plain(sound, looping)]);
            }
            Ok(1) => 0,
            Ok(len) => rng.below(len) as usize,
        };
        Some(self.tone[index].voices(rng))
    }

    /// Whether this title's ladder decoded no speed classes at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.tone.is_empty() && self.tone_flat.is_none()
    }

    /// One waveform for `stage`'s line, chosen by `rng` among alternates; `None`
    /// when the cue did not load or the title names none (stage `0`, `Start`, on
    /// every title measured so far).
    #[must_use]
    pub fn pick(&self, stage: u32, rng: &mut Rng) -> Option<(Arc<Sound>, bool)> {
        pick(self.lines.get(&stage)?, rng)
    }
}

fn pick(loaded: &Loaded, rng: &mut Rng) -> Option<(Arc<Sound>, bool)> {
    let index = match u32::try_from(loaded.waveforms.len()) {
        Ok(len) if len > 1 => rng.below(len) as usize,
        _ => 0,
    };
    let (sound, looping) = loaded.waveforms.get(index)?;
    Some((Arc::clone(sound), *looping))
}

/// The tone out of the first copy of its bank that names its cue. HD/Fury
/// ships two `env0_zone.bnk`s and only `DATA01`'s carries `ZONEBAR_TRANS`;
/// `DATA00`'s, the one [`Archives::read_name`] reaches first, is Fury's
/// (`docs/ghidra/functions/ps3-hdfury-eu/sound.md`, "The class-step tone's
/// bank").
fn load_tone(
    archives: &mut Archives,
    tone: oag_title::ZoneClassTone,
    report: &mut Vec<String>,
) -> (Vec<Timeline>, Option<Loaded>) {
    let oag_title::ZoneClassTone {
        bank: bank_name,
        cue,
        tick,
    } = tone;
    let copies = archives.read_every_name(bank_name);
    if copies.is_empty() {
        report.push(format!("class announcer: {bank_name} not read"));
        return (Vec::new(), None);
    }
    for (archive, blob) in &copies {
        let Some(bank) = parse_bank(blob, bank_name, report) else {
            continue;
        };
        if bank.cue_named(cue).is_none() {
            continue;
        }
        let Some(loaded) = load_reported(&bank, cue, &format!("{cue} ({archive})"), report) else {
            return (Vec::new(), None);
        };
        return match layers::timelines(&bank, cue, tick, false, false, false) {
            Ok(Some(timelines)) => {
                report.push(format!(
                    "class announcer: {cue} plays its timeline: {} variant(s) of {} voice(s)",
                    timelines.len(),
                    timelines.iter().map(|t| t.layers.len()).max().unwrap_or(0)
                ));
                (timelines, None)
            }
            other => {
                if let Err(e) = other {
                    report.push(format!(
                        "class announcer: {cue} timeline not built, flat pick kept: {e}"
                    ));
                }
                (Vec::new(), Some(loaded))
            }
        };
    }
    report.push(format!(
        "class announcer: {cue} names no cue in any of {} cop(ies) of {bank_name}",
        copies.len()
    ));
    (Vec::new(), None)
}

fn read_bank(archives: &mut Archives, name: &str, report: &mut Vec<String>) -> Option<Vec<u8>> {
    archives
        .read_name(name)
        .map_err(|e| report.push(format!("class announcer: {name} not read: {e}")))
        .ok()
}

fn parse_bank<'a>(blob: &'a [u8], name: &str, report: &mut Vec<String>) -> Option<sblk::Bank<'a>> {
    sblk::Bank::parse(blob)
        .map_err(|e| report.push(format!("class announcer: {name} not decoded: {e}")))
        .ok()
}

fn load_reported(
    bank: &sblk::Bank,
    name: &str,
    label: &str,
    report: &mut Vec<String>,
) -> Option<Loaded> {
    match load_named_cue(bank, name) {
        Ok((loaded, skipped)) => {
            let undecoded = if skipped == 0 {
                String::new()
            } else {
                format!(", {skipped} skipped: decoded to no samples")
            };
            report.push(format!(
                "class announcer: {label} -> {} waveform(s){undecoded}",
                loaded.waveforms.len()
            ));
            Some(loaded)
        }
        Err(e) => {
            report.push(format!("class announcer: {name} not loaded: {e}"));
            None
        }
    }
}
