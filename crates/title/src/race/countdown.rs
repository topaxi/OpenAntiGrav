//! A title's pre-race countdown voice: which speech bank each race mode reads
//! `ready` and `go` from.
//!
//! Split out of `race.rs` beside [`super::announcer`], which is the Zone-mode
//! voice ladder this is the start-of-race sibling of.

/// The banks a title's countdown voice is read from, per mode.
///
/// # What the original does, measured
///
/// On Pulse (PSP) the race start plays exactly two cues, both by name and both
/// through the dry path `Sound_PlayNamedInSlot` (no emitter, volume `0x400`):
/// `"ready"` when the intro substate machine hands over to the countdown state,
/// and `"go"` 180 ticks later when that state's timer runs out. Which bank the
/// names resolve in is the speech bank `World_LoadTrack` opened for the mode:
/// `speech.bnk` for every mode but two, `speech_elim.bnk` for Eliminator
/// (`g_game_mode == 8`) and `speech_zone.bnk` for Zone (`g_game_mode == 6`).
/// `ready` is 3.73 s of voice in `speech.bnk`, 3.00 s in `speech_elim.bnk` and
/// 2.48 s in `speech_zone.bnk`; `go` is 0.80 s in all three.
///
/// The default bank is [`super::SoundBanks::speech`]; this holds the two that
/// replace it. `None` on a title whose start-of-race voice has not been
/// measured, which is every title but Pulse: Pure's and HD's own
/// `speech_zone.bnk` also carry `ready` and `go`, but nothing here says when
/// or from where those titles play them.
///
/// See `docs/ghidra/functions/psp-pulse-usa/countdown-voice.md` and
/// `docs/formats/psp-audio.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountdownVoice {
    /// The Eliminator bank, `Data\Sound\speech_elim.bnk` on Pulse.
    pub eliminator_bank: &'static str,
    /// The Zone bank, `Data\Sound\speech_zone.bnk` on Pulse.
    pub zone_bank: &'static str,
}
