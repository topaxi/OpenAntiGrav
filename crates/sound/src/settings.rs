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
    /// `~SHIELD`). See [`crate::sfx::Cue::bus`] and
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
}
