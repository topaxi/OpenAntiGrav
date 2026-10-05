//! The persisted audio volumes and the music source.
//!
//! Lives with the sound crate because every field is one of its own types; `oag-game`
//! embeds it as the `[audio]` section of its settings file.

use serde::{Deserialize, Serialize};

use crate::{MusicSource, Volume};

/// How loud each bus is.
///
/// One row today, and the buses are `oag_audio::Bus`'s own - so this section
/// grows an entry per bus rather than per sound. A typed percentage rather than
/// a bare `u32` for the reason [`oag_display::display::Brightness`] is one: a value
/// out of range is a file that fails to load with a message, not a gain of 4000
/// discovered by ear.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// How loud the music bus is, as a percentage. Defaults to 100.
    ///
    /// **Full by default on purpose**, though "full" is no longer gain `1.0`.
    /// At 100 the slider itself is unattenuated, but the music bus also
    /// carries [`oag_audio::mixer::MUSIC_MASTER_TRIM`] - the original's own
    /// fixed trim on music alone, `0.44`, found and confirmed live in
    /// `MusicPlayer_Init` and its two gain-computation callers (see that
    /// constant's doc comment and
    /// [`audio-levels.md`](../../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md)).
    /// So a `--dump-audio` capture of the PS2 archive at this default is the
    /// disc's own PCM scaled by exactly `0.44`, not a byte-for-byte match -
    /// dividing the dump back out by `MUSIC_MASTER_TRIM` (or comparing at
    /// `sfx_volume = 0`, `music_volume = 100 / 0.44`-equivalent is not
    /// representable, so divide the samples instead) is what recovers the
    /// old decode-fidelity check. See [`crate::DUMP_SAMPLE_RATE`].
    ///
    /// **[`Self::master_volume`] has to be 100 for a comparison against the
    /// original's own mix**, and it did not exist when the paragraph above
    /// was first written. Both are read off the player's own settings file
    /// on the dump path, so any such comparison wants a pinned config rather
    /// than the machine's - the command is on
    /// [`audio-levels.md`](../../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md).
    #[serde(default)]
    pub music_volume: Volume,
    /// How loud the effects bus is, as a percentage. Defaults to 100.
    ///
    /// The original's options menu has exactly two volumes and this is the
    /// other one: `"Music Volume"` at `0x08a78658` and `"SFX Volume"` at
    /// `0x08a78668`, with nothing else. See
    /// `docs/architecture/adr/0018-audio-mixer-architecture.md`.
    #[serde(default)]
    pub sfx_volume: Volume,
    /// How loud voice lines are, as a percentage. Defaults to 100.
    ///
    /// **A third row the original's menu does not have, on a line the original
    /// does draw**: a cue is a voice line when it lives in `speech.bnk` rather
    /// than beside the effects, which is already how `oag_sound::sfx`
    /// tells `shieldactive` from the `~SHIELD` loop it fires with. See
    /// [`crate::sfx::Cue::bus`] and
    /// `docs/architecture/adr/0027-three-mix-buses.md`.
    ///
    /// **Two cues reach it today** - the shield callout and the Autopilot's
    /// one-second warning - because those are the only voice lines whose
    /// trigger has been recovered. Every other announcer line on the disc is
    /// decoded and unwired, so this row will get quieter to a player who never
    /// collects a pickup and louder to nobody. `HANDOVER.md` lists them.
    #[serde(default)]
    pub speech_volume: Volume,
    /// How loud everything is, after the three above. Defaults to 100.
    ///
    /// **Ours, and it is a knob the original's menu does not have.** What it
    /// does have is the *master itself*: `Audio_Init` (`0x089906e4`) opens
    /// group `0x10` at `0x400` and `Audio_OutputThread` (`0x0898ca54`) hands
    /// `master << 5` to the DAC, so this is the recovered stage of the chain
    /// with a row attached rather than a gain invented for the port. See
    /// [`audio-levels.md`](../../../docs/ghidra/functions/psp-pulse-usa/audio-levels.md).
    ///
    /// It is exposed because the mix has no headroom and the console's answer
    /// to that is to saturate ([`oag_audio::Mixer::render`]): eight craft on
    /// the grid sum past full scale on the effects bus alone, so a player who
    /// wants the race not to distort needs one control that moves both buses
    /// together rather than two they have to keep in step.
    #[serde(default)]
    pub master_volume: Volume,
    /// Which release's encode of the soundtrack plays: `auto`, `psp` or `ps2`.
    ///
    /// **`auto`, the booted disc, by default.** Only the sixteen soundtrack
    /// tracks have a proven counterpart on the other release - see
    /// `docs/formats/ps2-audio.md` - so this moves those and nothing else.
    /// Voice, every sound bank, and **the PSP front end's own music** stay
    /// where the game booted from; that last one has no PS2 counterpart at all,
    /// so a PSP boot sounds the same at every value of this key. See
    /// [`MusicSource`], which spells out why.
    ///
    /// The menu row is offered only on a machine that has both discs, but the
    /// key is always in this file, because a settings file is carried between
    /// machines and a value it cannot honour falls back to the booted disc
    /// rather than to silence.
    #[serde(default)]
    pub music_source: MusicSource,
}
