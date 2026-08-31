//! A title's two Zone-mode voice ladders: the zone-count milestone and the
//! speed-class step.
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
