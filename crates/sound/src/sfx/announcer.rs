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
    /// Each stage's line as the flat set of its cue's waveforms.
    lines: BTreeMap<u32, Loaded>,
    /// Each stage's line as the timeline its cue authors (a lead-in, then the
    /// voices), where the walk was complete. A line with none here plays its
    /// flat pick.
    line_timelines: BTreeMap<u32, Vec<Timeline>>,
    /// The tone every class step plays beside its line,
    /// [`oag_title::ZoneClassAnnouncer::tone`], as its timeline variants.
    /// Empty when the title names none or it did not build: a tone has no flat
    /// stand-in, one waveform of a layered wash is not the sound.
    tone: Vec<Timeline>,
    /// Same shape as [`Announcer::report`].
    pub report: Vec<String>,
}

impl ClassAnnouncer {
    /// Reads and decodes every stage [`oag_title::ZoneClassAnnouncer`] names,
    /// when this title has one. Never fails - see [`Announcer::load`].
    ///
    /// Both banks are read by name, so the copy the archive set resolves first
    /// is the one used: on the Fury disc `DATA00`'s `speech_zone.bnk` and
    /// `env0_zone.bnk`, the copies a Zone race has loaded.
    #[must_use]
    pub fn load(archives: &mut Archives, table: Option<&oag_title::ZoneClassAnnouncer>) -> Self {
        let mut out = Self::default();
        let Some(table) = table else {
            return out;
        };

        if let Some(blob) = read_bank(archives, table.bank, &mut out.report)
            && let Some(bank) = parse_bank(&blob, table.bank, &mut out.report)
        {
            for stage in 1..=u32::try_from(table.classes.len()).unwrap_or(0) {
                let Some(name) = table.cue_name(stage) else {
                    continue;
                };
                let label = format!("{name} (stage {stage})");
                if let Some(loaded) = load_reported(&bank, name, &label, &mut out.report) {
                    out.lines.insert(stage, loaded);
                }
                if let Some(timelines) =
                    timelines_reported(&bank, name, table.tick, false, &mut out.report)
                {
                    out.line_timelines.insert(stage, timelines);
                }
            }
        }

        if let Some(tone) = table.tone {
            out.tone = load_tone(archives, tone, &mut out.report);
        }
        out
    }

    /// The voices of one class-step tone, or `None` when the title names none
    /// or it did not load.
    #[must_use]
    pub fn tone_voices(&self, rng: &mut Rng) -> Option<Vec<CueVoice>> {
        variant(&self.tone, rng).map(|timeline| timeline.voices(rng))
    }

    /// The voices of `stage`'s line: its authored timeline when it built one,
    /// else one waveform chosen as [`Self::pick`] does. `None` when the cue did
    /// not load or the title names none for the stage.
    #[must_use]
    pub fn line_voices(&self, stage: u32, rng: &mut Rng) -> Option<Vec<CueVoice>> {
        if let Some(timelines) = self.line_timelines.get(&stage)
            && let Some(timeline) = variant(timelines, rng)
        {
            return Some(timeline.voices(rng));
        }
        let (sound, looping) = self.pick(stage, rng)?;
        Some(vec![CueVoice::plain(sound, looping)])
    }

    /// Whether this title's ladder decoded no speed classes at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.tone.is_empty()
    }

    /// One waveform for `stage`'s line, chosen by `rng` among alternates; `None`
    /// when the cue did not load or the title names none (stage `0`, `Start`, on
    /// every title measured so far).
    #[must_use]
    pub fn pick(&self, stage: u32, rng: &mut Rng) -> Option<(Arc<Sound>, bool)> {
        pick(self.lines.get(&stage)?, rng)
    }
}

/// One of `timelines`, drawn when the cue has alternates.
fn variant<'a>(timelines: &'a [Timeline], rng: &mut Rng) -> Option<&'a Timeline> {
    let index = match u32::try_from(timelines.len()) {
        Ok(0) | Err(_) => return None,
        Ok(1) => 0,
        Ok(len) => rng.below(len) as usize,
    };
    timelines.get(index)
}

fn pick(loaded: &Loaded, rng: &mut Rng) -> Option<(Arc<Sound>, bool)> {
    let index = match u32::try_from(loaded.waveforms.len()) {
        Ok(len) if len > 1 => rng.below(len) as usize,
        _ => 0,
    };
    let (sound, looping) = loaded.waveforms.get(index)?;
    Some((Arc::clone(sound), *looping))
}

/// The tone's timeline variants out of the copy of its bank the archive set
/// resolves first, which is the copy a race has loaded (`DATA00`'s on Fury:
/// 67 cues, `light_wipe` at cue 63; `docs/ghidra/functions/ps3-hdfury-eu/sound.md`,
/// "The class-step cues"). A tone that does not build is silence and a report
/// line.
fn load_tone(
    archives: &mut Archives,
    tone: oag_title::ZoneClassTone,
    report: &mut Vec<String>,
) -> Vec<Timeline> {
    let oag_title::ZoneClassTone {
        bank: bank_name,
        cue,
        tick,
    } = tone;
    let Some(blob) = read_bank(archives, bank_name, report) else {
        return Vec::new();
    };
    let Some(bank) = parse_bank(&blob, bank_name, report) else {
        return Vec::new();
    };
    if bank.cue_named(cue).is_none() {
        report.push(format!("class announcer: {bank_name} names no {cue}"));
        return Vec::new();
    }
    // `light_wipe` keys voices in the rear half of the pan law, which the
    // mixer does not model; the front-end reading plays them centred
    // (`layers::timelines`'s `front_end`, which on the PS3 tick also reads a
    // `0x19` group's count in the high byte, as HD's handler does; this cue has
    // no `0x19`).
    timelines_reported(&bank, cue, tick, true, report).unwrap_or_default()
}

/// `name`'s timelines, with the load report's line for what came of it.
fn timelines_reported(
    bank: &sblk::Bank,
    name: &str,
    tick: oag_title::SequenceTick,
    front_end: bool,
    report: &mut Vec<String>,
) -> Option<Vec<Timeline>> {
    match layers::timelines(bank, name, tick, front_end, false, false, true) {
        Ok(Some(timelines)) => {
            report.push(format!(
                "class announcer: {name} plays its timeline: {} variant(s) of {} voice(s)",
                timelines.len(),
                timelines.iter().map(|t| t.layers.len()).max().unwrap_or(0)
            ));
            Some(timelines)
        }
        Ok(None) => {
            report.push(format!("class announcer: {name} has no timeline"));
            None
        }
        Err(e) => {
            report.push(format!("class announcer: {name} timeline not built: {e}"));
            None
        }
    }
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
