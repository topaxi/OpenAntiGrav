//! The persisted audio volumes and the music source. Lives with the sound crate
//! because every field is one of its own types; `oag-game` embeds it as the
//! `[audio]` section of its settings file.

use serde::{Deserialize, Serialize};

use crate::{MusicSource, Volume};

/// How loud each bus is.
///
/// One row per `oag_audio::Bus`. A typed percentage, as
/// [`oag_display::display::Brightness`]: an out-of-range value fails to load
/// with a message rather than being a gain of 4000 found by ear.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// How loud the music bus is, as a percentage. Defaults to 100.
    ///
    /// Full by default on purpose, though not gain `1.0`: the music bus also
    /// carries [`oag_audio::mixer::MUSIC_MASTER_TRIM`], the original's fixed
    /// `0.44` on music alone (`MusicPlayer_Init` and its two gain callers, see
    /// that constant and
    /// [`audio-levels.md`](../../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md)).
    /// So a `--dump-audio` capture of the PS2 archive at the default is the
    /// disc's PCM scaled by exactly `0.44`; divide the samples by it to recover
    /// the decode-fidelity check. See [`crate::DUMP_SAMPLE_RATE`].
    ///
    /// [`Self::master_volume`] must be 100 for a comparison against the
    /// original's mix. Both are read from the player's own settings file on the
    /// dump path, so such a comparison wants a pinned config (command in
    /// [`audio-levels.md`](../../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md)).
    #[serde(default)]
    pub music_volume: Volume,
    /// How loud the effects bus is, as a percentage. Defaults to 100.
    ///
    /// The original's options menu has two volumes and this is the other:
    /// `"Music Volume"` at `0x08a78658` and `"SFX Volume"` at `0x08a78668`. See
    /// `docs/architecture/adr/0018-audio-mixer-architecture.md`.
    #[serde(default)]
    pub sfx_volume: Volume,
    /// How loud voice lines are, as a percentage. Defaults to 100.
    ///
    /// A third row the original's menu does not have, on a line the original
    /// does draw: a cue is a voice line when it lives in `speech.bnk` rather than
    /// beside the effects (how `oag_sound::sfx` tells `shieldactive` from
    /// `~SHIELD`). See [`crate::sfx::CueTables::bus`] and
    /// `docs/architecture/adr/0027-three-mix-buses.md`.
    ///
    /// Two cues reach it today (the shield callout and the Autopilot's one-second
    /// warning), the only voice lines with a recovered trigger; the rest are
    /// decoded and unwired. `HANDOVER.md` lists them.
    #[serde(default)]
    pub speech_volume: Volume,
    /// How loud everything is, after the three above. Defaults to 100.
    ///
    /// Ours: a knob the original's menu does not have, though the master itself
    /// is recovered: `Audio_Init` (`0x089906e4`) opens group `0x10` at `0x400`
    /// and `Audio_OutputThread` (`0x0898ca54`) hands `master << 5` to the DAC
    /// ([`audio-levels.md`](../../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md)).
    ///
    /// Exposed because the mix has no headroom and the console saturates
    /// ([`oag_audio::Mixer::render`]): eight craft on the grid sum past full
    /// scale on the effects bus alone, so one control moves both buses together.
    #[serde(default)]
    pub master_volume: Volume,
    /// Which release's encode of the soundtrack plays: `auto`, `psp` or `ps2`.
    ///
    /// `auto`, the booted disc, by default. Only the sixteen soundtrack tracks
    /// have a proven counterpart (`docs/formats/ps2-audio.md`); voice, every
    /// sound bank and the PSP front end's music (no PS2 counterpart, so a PSP
    /// boot sounds the same at every value) stay on the booted disc. See
    /// [`MusicSource`].
    ///
    /// The menu row is offered only with both discs, but the key is always in the
    /// file: a settings file moves between machines and an unhonourable value
    /// falls back to the booted disc, not silence.
    #[serde(default)]
    pub music_source: MusicSource,
    /// How a placed sound (a craft, a weapon) is heard to be ahead of or behind
    /// the camera: `auto` (the default), `original`, `rear_cue` or `headphones`.
    /// `auto` is `headphones` on a headphone output and `rear_cue` on any other.
    /// See [`oag_audio::SpatialChoice`]. Music and unplaced cues are never
    /// touched.
    ///
    /// While an output with a stable id is active this is that output's own
    /// choice, read back from [`Self::spatial_devices`] whenever it becomes the
    /// active output (see [`Self::recall_spatial`]).
    #[serde(default)]
    pub spatial_audio: oag_audio::SpatialChoice,
    /// The SPATIAL AUDIO value picked while a given output was active, oldest
    /// first, at most [`SPATIAL_DEVICES_MAX`]. `auto` is never stored: picking
    /// it forgets the output's entry. Exists because some outputs (headphones on
    /// a USB codec that reports itself as speakers) cannot be told from their
    /// route, so `auto` alone guesses wrong there.
    #[serde(default)]
    pub spatial_devices: Vec<SpatialDevice>,
    /// Folders of songs the race playlist may draw from, beside the discs.
    ///
    /// Each is `{ path = "...", enabled = true }`. Added by editing this file: the
    /// menu lists them on the MUSIC page and switches them, but has no text input
    /// yet to name one. A folder may sit on a slow mount (NFS, an SD card); it is
    /// read on a worker and a hung one never stalls a frame (`oag_sound::folders`).
    #[serde(default)]
    pub music_folders: Vec<crate::pool::MusicFolder>,
    /// The MUSIC page's switches, by source id, for the sources the player has
    /// switched. Absent means the default: the booted disc on, every other
    /// source off. A folder's `enabled` is its own default until it is here.
    #[serde(default)]
    pub music_switches: std::collections::BTreeMap<String, bool>,
}

/// How many outputs [`Settings::spatial_devices`] remembers.
pub const SPATIAL_DEVICES_MAX: usize = 16;

/// A SPATIAL AUDIO value remembered for one output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialDevice {
    /// The output's stable id (`oag_audio::Route::device`).
    pub device: String,
    /// What the player picked while it was active.
    pub mode: oag_audio::SpatialMode,
}

impl Settings {
    /// Records a SPATIAL AUDIO pick made while `device` is the active output.
    ///
    /// `auto` forgets the output's entry; anything else sets it as the most
    /// recent, evicting the oldest past [`SPATIAL_DEVICES_MAX`]. With no stable
    /// id (`None`) nothing is remembered and the pick is only the current value.
    pub fn choose_spatial(&mut self, device: Option<&str>, choice: oag_audio::SpatialChoice) {
        self.spatial_audio = choice;
        let Some(device) = device else { return };
        self.spatial_devices.retain(|entry| entry.device != device);
        let mode = match choice {
            oag_audio::SpatialChoice::Auto => return,
            oag_audio::SpatialChoice::Original => oag_audio::SpatialMode::Original,
            oag_audio::SpatialChoice::RearCue => oag_audio::SpatialMode::RearCue,
            oag_audio::SpatialChoice::Headphones => oag_audio::SpatialMode::Headphones,
        };
        self.spatial_devices.push(SpatialDevice {
            device: device.to_string(),
            mode,
        });
        let extra = self
            .spatial_devices
            .len()
            .saturating_sub(SPATIAL_DEVICES_MAX);
        self.spatial_devices.drain(..extra);
    }

    /// Makes `device`'s remembered value the current one (`auto` when it has
    /// none), as it becomes the active output. A remembered value wins for its
    /// own output only.
    pub fn recall_spatial(&mut self, device: &str) {
        self.spatial_audio = self
            .spatial_devices
            .iter()
            .find(|entry| entry.device == device)
            .map_or(oag_audio::SpatialChoice::Auto, |entry| entry.mode.into());
    }
}

#[cfg(test)]
mod tests;
