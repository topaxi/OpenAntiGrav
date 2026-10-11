//! The team and circuit name voices of Pulse's selection screens.
//!
//! `FRONT_END_VO.bnk`'s `TMTX_NAME` and `TRTX_NAME` are one cue each holding
//! every name, guarded by a runtime global: `Sound_PlayNamedInSlot` writes its
//! last argument into slot 2 (the team) or 1 (the circuit) of the 32-byte
//! table at `0x08ac3247`, and each command's `0x22` guard skips unless that
//! slot holds its own value. The value is the `soundregister` the plugin
//! `Definition.xml` authors on the team's or circuit's own node. Recovered in
//! `docs/ghidra/functions/psp-pulse-usa/menu-sounds.md`, "The name voices".
//!
//! So this plays the cue the way the original does and not one picked
//! waveform: each value is the walk of its cue with the global set, a guarded
//! pair of key-ons for a team (two grains ten master ticks apart) and the
//! circuit's own child cue for a circuit.

use std::collections::BTreeMap;

use oag_core::Rng;

use super::layers::{self, Timeline};
use super::{Audio, VoicePlace, start_voices};

/// The table slot `TRTX_NAME`'s guards read.
const CIRCUIT_SLOT: usize = 1;
/// The table slot `TMTX_NAME`'s guards read.
const TEAM_SLOT: usize = 2;
/// The cue holding every team name.
const TEAM_CUE: &str = "TMTX_NAME";
/// The cue holding every circuit name.
const CIRCUIT_CUE: &str = "TRTX_NAME";

/// The team and circuit name voices of one title, one timeline per value.
#[derive(Debug, Clone, Default)]
pub struct NameVoices {
    teams: BTreeMap<u8, Timeline>,
    circuits: BTreeMap<u8, Timeline>,
    report: Vec<String>,
}

impl NameVoices {
    /// Reads the title's name-voice bank. Never fails: a title without one
    /// ([`oag_title::FrontEndSounds::voices`]), a bank that will not read or a
    /// value that will not decode is a report line and silence.
    #[must_use]
    pub fn load(
        archives: &mut oag_assets::Archives,
        banks: &oag_title::SoundBanks,
        tick: oag_title::SequenceTick,
    ) -> Self {
        let mut out = Self::default();
        let Some(entry) = banks.frontend.and_then(|front| front.voices) else {
            return out;
        };
        let blob = match archives.read_name(entry) {
            Ok(blob) => blob,
            Err(e) => {
                out.report.push(format!("sfx: {entry} not read: {e}"));
                return out;
            }
        };
        let bank = match oag_formats::sblk::Bank::parse(&blob) {
            Ok(bank) => bank,
            Err(e) => {
                out.report.push(format!("sfx: {entry} not parsed: {e}"));
                return out;
            }
        };
        let values = 1..=oag_formats::sblk::timeline::GLOBAL_SLOTS as u8;
        for (cue, slot, into) in [
            (TEAM_CUE, TEAM_SLOT, &mut out.teams),
            (CIRCUIT_CUE, CIRCUIT_SLOT, &mut out.circuits),
        ] {
            match layers::guarded(&bank, cue, slot, values.clone(), tick) {
                Ok(timelines) => *into = timelines,
                Err(e) => out.report.push(format!("sfx: {cue} not decoded: {e}")),
            }
        }
        out.report.push(format!(
            "sfx: {entry} -> {} team name voice(s), {} circuit name voice(s)",
            out.teams.len(),
            out.circuits.len()
        ));
        out
    }

    /// What loading did, one line per cue.
    #[must_use]
    pub fn report(&self) -> &[String] {
        &self.report
    }

    /// Whether the team whose `soundregister` is `value` has a voice.
    #[must_use]
    pub fn has_team(&self, value: u8) -> bool {
        self.teams.contains_key(&value)
    }

    /// Whether the circuit whose `soundregister` is `value` has a voice.
    #[must_use]
    pub fn has_circuit(&self, value: u8) -> bool {
        self.circuits.contains_key(&value)
    }

    /// Starts the team's name, dry. `false` when `value` has no voice (the
    /// original's own `10` selects no command either) and nothing is
    /// substituted for it.
    pub fn play_team(&self, audio: &Audio, rng: &mut Rng, value: u8) -> bool {
        Self::play(audio, rng, self.teams.get(&value))
    }

    /// Starts the circuit's name, dry. See [`Self::play_team`].
    pub fn play_circuit(&self, audio: &Audio, rng: &mut Rng, value: u8) -> bool {
        Self::play(audio, rng, self.circuits.get(&value))
    }

    fn play(audio: &Audio, rng: &mut Rng, timeline: Option<&Timeline>) -> bool {
        let Some(timeline) = timeline else {
            return false;
        };
        let voices = timeline.voices(rng);
        audio.output.with_mixer(|mixer| {
            !start_voices(mixer, &voices, oag_audio::Bus::Speech, VoicePlace::DRY).is_empty()
        })
    }
}
