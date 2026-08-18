//! Which layout a title's in-race HUD is read from, per mode this engine runs.
//!
//! The fourth axis a second corpus has forced, on the same terms as
//! [`crate::boot`], [`crate::menu`] and [`crate::race`]: it exists because two
//! titles measurably disagree, not because a HUD obviously needs a table.
//!
//! | | Pulse / Pure | HD / Fury |
//! | --- | --- | --- |
//! | single race | `Data\XML\Arcade_HUD.xml` | `/data/xml/arcade_hud.xml` |
//! | time trial | `Data\XML\TimeTrial_HUD.xml` | `/data/xml/timetrial_hud.xml` |
//! | speed lap | *the time trial's* | `/data/xml/speedlap_hud.xml` |
//! | zone | `Data\XML\Zone_HUD.xml` | `/data/xml/zone_hud.xml` |
//!
//! **The row that diverges is speed lap**, and it is the whole reason this type
//! exists rather than a comment. Neither PSP disc ships a `SpeedLap_HUD.xml` at
//! all - checked directly on both, the name hashes to `1af0a646` and no entry
//! carries it - so on Pulse and Pure speed lap draws the time trial's layout,
//! which is why `docs/ui/hud.md` counts five layouts for six modes. HD ships a
//! separate one. A caller cannot derive either arrangement from the other, and
//! before this axis every title was served Pulse's answer (finding S2 of the
//! 2026-08-18 review).
//!
//! # The three other rows agree, and that is a measurement too
//!
//! Pure carries all three of Pulse's spellings verbatim, and HD's fold onto
//! them through PSARC path normalisation - `Data\XML\Arcade_HUD.xml` normalises
//! to `data/xml/arcade_hud.xml`, which is what HD's manifest stores. So the
//! engine worked on three titles while asking every one of them Pulse's
//! question.
//!
//! **That coincidence is exactly what this type is for.** A table whose rows
//! agree is not a table with one value in it: each row is a per-title
//! measurement that happens to have come out the same, and the day one stops
//! agreeing - 2048 is a different engine generation - it changes in the title
//! package that measured it rather than in the engine. HD already proves the
//! point on the one row where nobody had looked.
//!
//! # Modes, deliberately as fields rather than as a map
//!
//! `oag_race::Mode` is the engine's type and a title package must not grow one,
//! which is the rule `oag_pulse::race` states for the Zone hull. So this carries
//! one field per mode this engine runs and `oag_game::race::hud_layout` does
//! the mapping. A title that ships a layout for a mode this engine has no rules
//! for keeps it as a constant in its own crate, the way HD's Detonator, Duel
//! and MPTag roots do.

/// The in-race HUD root a title authors for each mode.
///
/// A **root**, not a whole layout: HD composes a mode's HUD out of a shell plus
/// up to sixteen `<LoadXML SrcRel=>` fragments, and splicing those is
/// `oag_game::hud::compose`'s job. Both PSP titles ship self-contained files,
/// which compose to themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudLayouts {
    /// Single race.
    pub arcade: &'static str,
    /// Time trial.
    pub time_trial: &'static str,
    /// Speed lap - the one row the corpus disagrees on. Equal to
    /// [`Self::time_trial`] on a title that ships no separate layout, which is
    /// a measured fact about both PSP discs and not a fallback.
    pub speed_lap: &'static str,
    /// Zone.
    pub zone: &'static str,
}
