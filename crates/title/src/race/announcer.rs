//! A title's two Zone-mode voice ladders (the zone-count milestone and the
//! speed-class step) and its start-of-race voice.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

// Referenced only from doc comments below - intra-doc links need the name in
// scope, but nothing here calls them.
#[allow(unused_imports)]
use super::{ZoneCircuit, ZoneCraft, ZonePalette, ZoneStages};
use crate::Origin;

/// A title's Zone-mode announcer: the milestone numbers it names a voice line
/// for, and the bank that line is in.
///
/// # Every title's ladder is its own disc data, not one shared table
///
/// `Data\Sound\speech_zone.bnk` is one path all three measured titles carry -
/// see `docs/formats/psp-audio.md`'s bank table - but what it names differs:
/// Pulse announces every five zones to 30 then every ten to 100 (thirteen
/// cues), Pure stops naming every five after 30 and jumps straight to 75
/// (ten cues, plus a `bronze`/`silver`/`gold` medal set this port does not
/// read), and Wipeout HD keeps naming every five all the way to 50 before
/// switching to tens (fifteen cues, plus the fourteen speed-class lines
/// [`ZoneClassAnnouncer`] carries - see the same doc page). A milestone
/// number is therefore a title fact and not an engine constant, the same way
/// [`ZoneCircuit`] and [`ZoneCraft`] are.
///
/// # The cue name is always `zone_<n>`
///
/// Measured on all three: every numbered cue in every title's own
/// `speech_zone.bnk` is spelled exactly that way, so [`Self::cue_name`] is one
/// function rather than a per-title table of names.
///
/// # Confidence
///
/// **95** for "the bank exists, holds these cues, under this name" - read
/// directly off the shipped audio with `oag-wad sounds`, not inferred. **75**
/// for "the run-time zone counter reaching this number is what plays this
/// cue" - traced end to end only on Pulse's executable
/// (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md#the-ten-second-step`),
/// where the milestone table's own thresholds and the bank's own cue order
/// agree ordinally but the instruction that finally selects a waveform sits
/// outside this project's Ghidra database. Pure and HD's own executables have
/// not been read for this at all; their ladders are attributed by the pattern
/// three-titles-and-no-hole already established for the other two Zone axes,
/// not by a second traced call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneAnnouncer {
    /// `Data\Sound\speech_zone.bnk` on every title measured so far.
    pub bank: &'static str,
    /// The zone numbers this title's bank names a `zone_<n>` cue for, ascending.
    pub milestones: &'static [u16],
    /// What a cue's per-grain delays are counted in. See [`SequenceTick`].
    pub tick: SequenceTick,
}

/// The unit a SCREAM cue's delay words are counted in, as far as it is known
/// for a title.
///
/// A cue such as Pulse's `zone_5` is a timeline of words on authored delays,
/// and turning a delay into time needs the engine's master tick. It is a
/// property of the *build*, not of the bank's byte order: the PSP's is
/// 258.4 Hz, measured live (`docs/ghidra/functions/psp-pulse-usa/sound.md`,
/// "The master tick and the delay word"), and the PS3's is 240 Hz, read from
/// the executable and measured live in RPCS3
/// (`docs/ghidra/functions/ps3-hdfury-eu/sound.md`, "The master tick").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceTick {
    /// Not measured for this title's build. A sequence cue is not laid out on
    /// a timeline; the flat pick is kept rather than lend another build's rate.
    Unknown,
    /// The PSP's 258.4 Hz master tick (`44100 * 3 / 512`). **Measured** on
    /// Pulse's PSP build; for a title that ships the same cue on another
    /// platform (Pulse's PS2 pressing) it is **lent, chosen, not measured** -
    /// the words and their order are authored either way, and the alternative
    /// is playing one of them at random.
    Psp,
    /// The PS3's 240 Hz master tick (`48000 * 2.56 / 512`): Wipeout HD/Fury's
    /// SCREAM adds 2.56 to an accumulator once per audio-thread pass, and a
    /// pass waits for two 256-frame `cellAudio` blocks at 48 kHz. **Measured**
    /// on HD's EU build in RPCS3 (239.4 to 239.9 ticks per second over four
    /// 10 s windows). Omega's PS4 build is a different binary and is not
    /// covered.
    Ps3,
    /// The Vita's 240 Hz master tick, Wipeout 2048's SCREAM 7.0.29: the mixer
    /// accumulates the frames it renders and runs one tick per
    /// `48000 / (240 * m)` frames, `m` a float initialised to `1.0` in
    /// `FUN_8134af2c` and written nowhere else (`0x8153bc9c`, read by the
    /// tick loop `FUN_81354b74` and the pitch scale `FUN_8135718a`).
    /// **Read from the executable, not run** (no Vita capture): confidence 85.
    /// Only the countdown's own cues use it ([`CountdownVoice::tick`]); 2048's
    /// other cues keep the flat pick until a lane widens it.
    Vita,
}

impl SequenceTick {
    /// Master ticks per second, or [`None`] when the build's tick is not known.
    #[must_use]
    pub fn ticks_per_second(self) -> Option<f64> {
        match self {
            Self::Unknown => None,
            Self::Psp => Some(44_100.0 * 3.0 / 512.0),
            Self::Ps3 | Self::Vita => Some(240.0),
        }
    }

    /// Whether a `0x19` alternate group keeps its count in the operand's high
    /// byte, as HD's handler reads it (`19 07 02 00` is seven alternates of two
    /// commands), against the low byte on the PSP.
    ///
    /// Like [`Self::follows_gotos`] it is only switched on for the cues whose
    /// lane measured it: HD's front-end cues. HD's race cues keep the flat pick
    /// they were measured with.
    #[must_use]
    pub fn alternates_in_high_byte(self) -> bool {
        matches!(self, Self::Ps3)
    }

    /// Whether a cue walk follows `goto` (`0x24`) to its marker (`0x23`).
    ///
    /// Read on both builds' executables, but **only switched on where a cue
    /// needs it and the title's audio was not already being played**: HD's
    /// `c_CLEAR` is a goto and its marker and nothing else. Pulse keeps the
    /// walk it was measured with, so its cue census does not move in the change
    /// that measured HD.
    #[must_use]
    pub fn follows_gotos(self) -> bool {
        matches!(self, Self::Ps3 | Self::Vita)
    }

    /// Whether the build's command handlers are Wipeout 2048's (`0x2e`, `0x29`,
    /// `0x04` and the low-byte marker id), against Pulse's and HD's.
    #[must_use]
    pub fn vita_handlers(self) -> bool {
        matches!(self, Self::Vita)
    }
}

impl ZoneAnnouncer {
    /// The cue name for one of [`Self::milestones`], e.g. `"zone_5"`.
    #[must_use]
    pub fn cue_name(milestone: u16) -> String {
        format!("zone_{milestone}")
    }
}

/// A title's Zone-mode **speed-class** announcer: the voice line called out
/// on the same speed-class steps [`ZoneStages`] and [`ZonePalette`] already
/// carry, and the bank it is in.
///
/// # Distinct from [`ZoneAnnouncer`]
///
/// That type calls a zone *count* ("Zone 5", cue `zone_5`). This one calls
/// the speed *class* a Zone race has escalated to ("Venom", "Flash", ... up
/// to "Supersonic") - the same fifteen-rung ladder HD/Fury's `SpeedClass` and
/// `NextSpeedClass` HUD widgets show and its `.effectSettings` palette
/// escalates through. It is a second, independent axis of the same event,
/// not a variant reading of the first.
///
/// # What is measured
///
/// The lines are the fourteen `MR_*` cues of `Data\Sound\speech_zone.bnk`, at
/// consecutive cue indices `26`-`39` of the copy a Zone race has loaded
/// (Fury's `DATA00`, 43 cues): `MR_SVE`, `MR_VEN`, `MR_SFL`, `MR_FLA`,
/// `MR_SRA`, `MR_RAP`, `MR_SPH`, `MR_PHA`, `MR_SUP`, `MR_ZEN`, `MR_SUZ`,
/// `MR_Z_SUB`, `MR_Z_M1`, `MR_Z_SUP`. The order is a 14/14 match against
/// [`ZoneStages`]'s own fourteen non-`Start` stage names, read by cue
/// *index*: `MR_SUP` alone reads as either "Super Phantom" or "Supersonic",
/// and only the position in a gap-free run resolves it. The dedicated
/// `speech_class.bnk` (`DATA01`) carries the same names but a Zone race on
/// the Fury disc does not load it (the live manager's speech slot holds
/// `speech_zone.bnk`).
///
/// A line is a timeline, not a waveform: a delay word and a `0x1a` with
/// operand 0 (a fixed 200 master ticks, 0.83 s at 240 Hz), then two key-ons
/// on one tick at +30 and +330 degrees. Playing it is
/// `oag_sound::sfx::ClassAnnouncer`'s job.
///
/// # The trigger
///
/// Measured live on the Fury EU disc in a plain Zone race
/// (`docs/ghidra/functions/ps3-hdfury-eu/sound.md`, "The class-step cues"):
/// at a natural step the race queues the new class's `MR_*` name through
/// `Sound_QueueAnnouncerCue` (caller `0x762c0`), the announcer drain plays it
/// from the speech slot 2 to 10 ticks later, and the stage-blend update plays
/// [`Self::tone`] immediately before the stage commit. Confidence 88 for the
/// name and bank, 85 for the tone. **Chosen, not measured:** the original
/// queues the line a few ticks before the commit; the port plays line and tone
/// on the [`ZoneStages::stage_for`] edge the HUD text and colour grade already
/// key off, so all three cannot disagree about when a class changed.
///
/// **Not this ladder's tone:** `ZONEBAR_TRANS` and `ZONEADVANCE` belong to
/// Zone Battle's `Zone_UpdateCraftClass` and never play in plain Zone
/// (confidence 80).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneClassAnnouncer {
    /// `Data\Sound\speech_zone.bnk` on HD/Fury, the bank a Zone race has loaded.
    pub bank: &'static str,
    /// The fourteen cue names, in [`ZoneStages`]'s own stage order:
    /// `classes[0]` is stage `1`'s cue, not stage `0`'s - HD's speech bank
    /// names no cue for `Start` at all.
    pub classes: &'static [&'static str],
    /// What the lines' delay words are counted in.
    pub tick: SequenceTick,
    /// The tone played with every class step. `None` where unread.
    pub tone: Option<ZoneClassTone>,
}

/// The non-verbal cue a Zone class step plays beside its voice line.
///
/// HD/Fury's is `light_wipe` out of the race's environment bank, the stage's
/// colour wipe keyed by the stage-blend update (`Environment_UpdateStageBlend`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneClassTone {
    /// The bank it is named in. A race loads one copy, so the copy the archive
    /// set resolves first is the live one.
    pub bank: &'static str,
    /// The cue's name.
    pub cue: &'static str,
    /// What the cue's per-grain delays are counted in.
    pub tick: SequenceTick,
}

impl ZoneClassAnnouncer {
    /// The cue name for `stage`, or `None` for stage `0` (`Start`, which this
    /// ladder names nothing for) or a stage past [`Self::classes`]' own count.
    #[must_use]
    pub fn cue_name(&self, stage: u32) -> Option<&'static str> {
        let index = usize::try_from(stage.checked_sub(1)?).ok()?;
        self.classes.get(index).copied()
    }
}

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
/// replace it. [`Self::origin`] says whose timing a title plays: Pulse's and
/// Pure's (PSP USA) is measured; HD's cues and triggers are measured and its
/// spacing is Pulse's (inherited); 2048 and Omega lend Pulse's (chosen, not
/// measured).
///
/// See `docs/ghidra/functions/psp-pulse-usa/countdown-voice.md` and
/// `docs/formats/psp-audio.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountdownVoice {
    /// The Eliminator bank, `Data\Sound\speech_elim.bnk` on Pulse.
    pub eliminator_bank: &'static str,
    /// The Zone bank, `Data\Sound\speech_zone.bnk` on Pulse.
    pub zone_bank: &'static str,
    /// Where this title's `ready` and `go` (which cues, in which bank, 180 ticks
    /// apart, `go` on the last gated tick) came from: [`Origin::Measured`] when
    /// the title's own cue-start log shows it.
    pub origin: Origin,
    /// What the countdown's delays are counted in. Not the title's other cues'
    /// tick ([`ZoneAnnouncer::tick`]): a title opts its countdown in on its own.
    pub tick: SequenceTick,
    /// Whether the cue names are looked up by FNV hash of the
    /// name (2048's `SBlk` version 5 banks) rather than spelled in the bank.
    pub hashed: bool,
    /// The start a title's **own events** play instead of `ready` and `go`, or
    /// `None` when every race plays the named pair.
    pub event: Option<CountdownEvent>,
    /// Whether [`Self::event`] is the start of **every** race, named modes
    /// included, rather than of the title's own events alone: Omega posts its
    /// pair on every mode but the demo
    /// (`docs/ghidra/functions/ps4-omega-eu/wwise-sound.md`).
    pub event_every_mode: bool,
    /// The cue played with `ready` in a named mode (`HBEAT` on HD and 2048): the
    /// start beat, three groups of voices on the master tick. `None` where the
    /// title's start has none (Pulse's measured log shows none).
    pub beat: Option<&'static str>,
}

/// The start a race built from a title's own event (not a named mode) plays.
///
/// 2048's race-state setter (`0x811743e4`) and the countdown's end
/// (`0x81175d12`) pick on the mode id: below `0x17` (every named mode) it plays
/// `ready` and `go` from the speech bank, from `0x17` up (the CRC ids of its own
/// campaign and multiplayer events) it plays `321_NGP` and `Go_NGP` from a bank
/// of its own, `docs/ghidra/functions/vita-2048-eu-v104/countdown-voice.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountdownEvent {
    /// The bank both cues are in, `Data\audio\sound\Weapons_NGP.bnk` on 2048.
    pub bank: &'static str,
    /// The cue played where a named mode plays `ready`.
    pub ready: StartCue,
    /// The cue played where a named mode plays `go`.
    pub go: StartCue,
}

/// One cue of a [`CountdownEvent`]: played, or authored and left unplayed.
///
/// An unplayed cue is **silence and a loader report line**, never the flat pick
/// of its waveforms: a cue the mixer cannot render as authored must not become
/// a different sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartCue {
    /// Played, under this name in [`CountdownEvent::bank`].
    Plays(&'static str),
    /// Authored, not played: what it is and what is missing, for the report.
    Unplayed {
        name: &'static str,
        why: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::SequenceTick;

    #[test]
    fn each_build_names_its_own_rate_and_an_unmeasured_one_names_none() {
        assert_eq!(SequenceTick::Unknown.ticks_per_second(), None);
        // 44,100 Hz, three ticks per 512 frames.
        let psp = SequenceTick::Psp.ticks_per_second().expect("measured");
        assert!((psp - 258.398_437_5).abs() < 1e-9);
        // 48,000 Hz, 2.56 ticks per 512 frames.
        assert_eq!(SequenceTick::Ps3.ticks_per_second(), Some(240.0));
    }

    #[test]
    fn only_the_ps3_walk_follows_gotos() {
        assert!(!SequenceTick::Unknown.follows_gotos());
        assert!(!SequenceTick::Psp.follows_gotos());
        assert!(SequenceTick::Ps3.follows_gotos());
    }

    #[test]
    fn the_vita_counts_240_ticks_and_reads_its_own_handlers() {
        assert_eq!(SequenceTick::Vita.ticks_per_second(), Some(240.0));
        assert!(SequenceTick::Vita.follows_gotos());
        assert!(SequenceTick::Vita.vita_handlers());
        assert!(!SequenceTick::Ps3.vita_handlers());
    }
}
