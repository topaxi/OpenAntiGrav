//! A title's two Zone-mode voice ladders (the zone-count milestone and the
//! speed-class step) and its start-of-race voice.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

// Referenced only from doc comments below - intra-doc links need the name in
// scope, but nothing here calls them.
#[allow(unused_imports)]
use super::{ZoneCircuit, ZoneCraft, ZonePalette, ZoneStages};

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
}

impl SequenceTick {
    /// Master ticks per second, or [`None`] when the build's tick is not known.
    #[must_use]
    pub fn ticks_per_second(self) -> Option<f64> {
        match self {
            Self::Unknown => None,
            Self::Psp => Some(44_100.0 * 3.0 / 512.0),
            Self::Ps3 => Some(240.0),
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
        matches!(self, Self::Ps3)
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
/// Read directly against `hdfury-ps3-eu-dec.iso` (`Data\Sound\speech_class.bnk`
/// extracted and parsed with `oag_formats::sblk`, not assumed from a name
/// list): a **dedicated** fifteen-cue bank, unmixed with any other voice line,
/// naming `ZONEMALE` at cue `0` followed by fourteen cues at consecutive
/// indices `1`-`14` - `MR_SVE`, `MR_VEN`, `MR_SFL`, `MR_FLA`, `MR_SRA`,
/// `MR_RAP`, `MR_SPH`, `MR_PHA`, `MR_SUP`, `MR_ZEN`, `MR_SUZ`, `MR_Z_SUB`,
/// `MR_Z_M1`, `MR_Z_SUP`. The same fourteen names, in the same order, also
/// appear inside the general `speech_zone.bnk` [`ZoneAnnouncer::bank`] reads,
/// at cue indices `26`-`39` - two copies of the same ladder, one purpose-built
/// and one folded into the general Zone voice bank.
///
/// **The order is a 14/14 match against [`ZoneStages`]'s own fourteen
/// non-`Start` stage names** - Sub Venom, Venom, Sub Flash, Flash, ..., Mach
/// 1, Supersonic - one cue per non-`Start` stage, contiguous, with no gap.
/// Read by cue *index*, not by parsing the name: `MR_SUP` alone reads as
/// either "Super Phantom" or "Supersonic", and only the position in a
/// fourteen-long, gap-free run resolves it (`MR_Z_SUP`, at the very end,
/// gets the name `MR_SUP`'s own spelling would suggest for itself). See
/// `docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`.
///
/// # What is not established
///
/// **No trigger for this ladder has been read out of HD's own executable.**
/// Nothing traces a call site that plays `MR_VEN` when the speed class steps.
/// This is exactly the footing [`ZoneAnnouncer`]'s own milestone ladder is
/// already wired on for this title - see that type's own confidence note -
/// so playing this cue on the same [`ZoneStages::stage_for`] edge the HUD
/// text and the colour grade already key off is not a new class of
/// inference, and keeps all three from being able to disagree about when a
/// class changed.
///
/// **A candidate for the *non-verbal* half of a class change was found and is
/// deliberately left unwired.** `env0_zone.bnk` (HD's Zone environment sound
/// bank) names a cue `ZONEBAR_TRANS` - plausibly "Zone bar transition," the
/// HUD ladder's own animation - but nothing traces when it plays either, and
/// its name is a single unread label rather than a fourteen-way order match.
/// See `docs/formats/psp-audio.md` for both findings side by side, including
/// a prior claim on that page (`HBEAT_ZCHANGE`) that this session's direct
/// extraction found does not exist in the shipped bank at all - corrected
/// there rather than repeated here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneClassAnnouncer {
    /// `Data\Sound\speech_class.bnk` on HD/Fury - the dedicated bank, not the
    /// general `speech_zone.bnk` the same fourteen cues are also folded into.
    pub bank: &'static str,
    /// The fourteen cue names, in [`ZoneStages`]'s own stage order:
    /// `classes[0]` is stage `1`'s cue, not stage `0`'s - HD's speech bank
    /// names no cue for `Start` at all.
    pub classes: &'static [&'static str],
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
}
