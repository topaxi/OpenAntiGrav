//! A title's race cues played as the Wwise events its executable posts for
//! them: [`oag_title::WwiseCues`], one row per cue of this port.
//!
//! Each posted cue loads as `<cue>__<slot>` (every emitter but the player's)
//! and, where the bank authors it, `<cue>_PC__<slot>` (the player's), each
//! through [`super::wwise_start`]: the player's event is often a relay to the
//! other, but not always (Omega's `NGP_MissileFire_PC` sets states besides). A
//! player event that is authored and does not load leaves the cue silent from
//! the player's emitter rather than playing the other event in its place.
//! `docs/ghidra/functions/ps4-omega-eu/wwise-cues.md`.

use std::collections::BTreeSet;

use log::{debug, trace};
use oag_assets::source::Archives;
use oag_core::Rng;
use oag_formats::wwise::{Bank, Kind, name_hash};

use super::banks::Banks;
use super::{Cue, CueTables};

/// One bank slot's archive entries, read once each.
struct SlotBlobs {
    slot: &'static str,
    blobs: Vec<(&'static str, Vec<u8>)>,
    /// The id of every event the slot's entries define.
    events: BTreeSet<u32>,
}

/// The id of every event `blob` defines; none when it is not a Wwise bank.
fn event_ids(blob: &[u8]) -> BTreeSet<u32> {
    Bank::parse(blob).map_or_else(
        |_| BTreeSet::new(),
        |bank| {
            bank.objects()
                .iter()
                .filter(|o| o.kind == Kind::Event)
                .map(|o| o.id)
                .collect()
        },
    )
}

impl Banks {
    /// Loads every cue `cues` posts, beside what [`Self::load`] decoded: a
    /// loaded event replaces the cue's named-bank sound. Never fails; one
    /// report line per cue either way, and one `debug` summary.
    ///
    /// A random container's child is drawn once here, seeded by the event's
    /// name, so a race is repeatable and every post of a cue plays the same
    /// pick (`docs/formats/wwise.md`, "Sound containers").
    pub fn load_wwise_cues(&mut self, archives: &mut Archives, cues: &oag_title::WwiseCues) {
        let slots: Vec<SlotBlobs> = cues
            .banks
            .iter()
            .map(|bank| {
                let blobs: Vec<(&'static str, Vec<u8>)> = bank
                    .entries
                    .iter()
                    .filter_map(|&entry| match archives.read_name(entry) {
                        Ok(blob) => Some((entry, blob)),
                        Err(e) => {
                            self.report.push(format!("sfx: {entry} not read: {e}"));
                            None
                        }
                    })
                    .collect();
                let events = blobs.iter().flat_map(|(_, b)| event_ids(b)).collect();
                SlotBlobs {
                    slot: bank.slot,
                    blobs,
                    events,
                }
            })
            .collect();
        let (mut played, mut player) = (0, 0);
        for cue in Cue::ALL {
            let Some(&(_, row)) = cues.cues.iter().find(|(name, _)| *name == cue.name()) else {
                self.report
                    .push(format!("sfx: {}: no Wwise cue read for it", cue.name()));
                continue;
            };
            let (posted, slot) = match row {
                oag_title::WwiseCue::Posts { cue, slot } => (cue, slot),
                oag_title::WwiseCue::Unplayed { why } => {
                    self.report
                        .push(format!("sfx: {} not played: {why}", cue.name()));
                    continue;
                }
            };
            let Some(blobs) = slots.iter().find(|s| s.slot == slot) else {
                self.report
                    .push(format!("sfx: {posted}: no bank for the slot {slot}"));
                continue;
            };
            if self.load_posted(cue, &format!("{posted}__{slot}"), blobs, false) {
                played += 1;
            }
            // The player's version where the bank authors one; where it does
            // not, the player hears what everyone does.
            let mine = format!("{posted}_PC__{slot}");
            if blobs.events.contains(&name_hash(&mine)) {
                if self.load_posted(cue, &mine, blobs, true) {
                    player += 1;
                } else {
                    self.player_silent.insert(cue);
                }
            }
        }
        let line = format!(
            "sfx: {played} of {} race cues play their Wwise event, {player} with a player version",
            Cue::ALL.len()
        );
        debug!("{line}");
        self.report.push(line);
    }

    /// Loads `event` out of the first of `blobs` that defines it: as `cue`'s
    /// sound, or as its player version.
    fn load_posted(&mut self, cue: Cue, event: &str, blobs: &SlotBlobs, player: bool) -> bool {
        let mut why = Vec::new();
        for (entry, blob) in &blobs.blobs {
            let mut rng = Rng::new(u64::from(name_hash(event)));
            match super::wwise_start::load(blob, event, &mut rng) {
                Ok(loaded) => {
                    let line = format!(
                        "sfx: {event} -> {} voice(s) from {entry}, one child of {} container(s)",
                        loaded.timeline.layers.len(),
                        loaded.picks.len()
                    );
                    trace!("{line}");
                    self.report.push(line);
                    self.report
                        .extend(loaded.notes.iter().map(|n| format!("sfx: {event}: {n}")));
                    if player {
                        self.player_timelines.insert(cue, vec![loaded.timeline]);
                    } else {
                        self.sounds.insert(cue, loaded.loaded);
                        self.wwise.insert(cue);
                        self.timelines.insert(cue, vec![loaded.timeline]);
                        self.programs.remove(&cue);
                    }
                    return true;
                }
                Err(e) => why.push(format!("{entry}: {e:#}")),
            }
        }
        let line = format!("sfx: {event} not loaded: {}", why.join("; "));
        trace!("{line}");
        self.report.push(line);
        false
    }
}
