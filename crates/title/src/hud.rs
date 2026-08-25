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

/// `reference` with its file extension replaced by `extension`.
///
/// The rule [`HudArt::texture_extension`] names, applied. A reference with no
/// extension gains one; a dot inside a directory name is not an extension and
/// is left alone, which is what keeps `Data\v1.2\atlas` from becoming
/// `Data\v1`. Separators and case are untouched - the archive readers fold
/// both.
#[must_use]
pub fn replace_extension(reference: &str, extension: &str) -> String {
    let stem = match reference.rfind('.') {
        // Only a *file* extension: a dot in a directory name is not one.
        Some(at) if !reference[at..].contains(['/', '\\']) => &reference[..at],
        _ => reference,
    };
    format!("{stem}{extension}")
}

/// How a title's HUD sprites reach the screen: where their pixels come from,
/// which of them are up whenever the HUD is, and the one colour this build
/// substitutes.
///
/// A fifth axis, on the same terms as [`HudLayouts`] above: it exists because
/// the corpus measurably disagrees on all three rows, not because a HUD
/// obviously needs a table. Every one of them was a `const` inside
/// `oag_game::hud` giving Pulse's answer to every title until 2026-08-25.
///
/// | | Pulse / Pure | HD / Fury |
/// | --- | --- | --- |
/// | [`Self::texture_extension`] | `None` - the layouts name shipped entries | `.gtf` |
/// | [`Self::always_on`] | seven names | fifteen names |
/// | [`Self::pickup_backdrop_colour`] | `HudBGColour` | `None` - drawn as authored |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudArt {
    /// The extension a layout's `src=` reference takes to become an archive
    /// entry name, when the disc does not ship the declared spelling.
    ///
    /// `None` when a reference *is* an entry name, which is both PSP titles'
    /// answer: their layouts name `.mip` files and their discs carry `.mip`
    /// files. (The PS2 pressing's `.pct` rewrite is a different question -
    /// which *pressing* of one title keeps a texture where - and stays in
    /// `oag_pulse::read_image`, which every source goes through either way.)
    ///
    /// HD is why this is a row: its HUD layouts name the **exporter's input**
    /// rather than the shipped file, so read literally 192 of its 1,029 sprites
    /// are dangling references. See `oag_hd::hud::texture_entry`, which is this
    /// row applied, and [`replace_extension`], which applies it. Confidence 85.
    ///
    /// The declared name is always tried first, so a title filling this in is
    /// never worse off than one that leaves it `None`.
    ///
    /// An extension, not a function pointer, because [`Title`] compares by
    /// value and two `fn` items have no meaningful comparison - and because a
    /// rule spelled as data is one a reader can check against the disc.
    ///
    /// [`Title`]: crate::Title
    pub texture_extension: Option<&'static str>,
    /// The sprite widgets drawn whenever this title's HUD is up.
    ///
    /// **An allow-list, deliberately**, for the reason `oag_game::hud`'s label
    /// list is one: a layout carries every widget every mode and state could
    /// want - warnings, opponent tags, weapon sights, mode-specific pieces -
    /// and drawing them all at once is a picture the original never shows. A
    /// widget not named here is not drawn, so the difference between "no value"
    /// and "not wired up" stays visible instead of hiding behind an invisible
    /// quad.
    ///
    /// A name absent from a given layout is simply not found, so one list
    /// serves all of a title's modes.
    pub always_on: &'static [&'static str],
    /// The layout constant substituted for the pickup backdrop's own colour, or
    /// `None` to draw it as the layout authors it.
    ///
    /// Pulse needs the substitution and it is measured: its `PickupBackground`
    /// samples a **filled** hexagon whose alpha is 255, authored in the same
    /// opaque white as the icon that sits on it, so drawn as authored the icon
    /// is invisible. `oag_game::hud` carries the measurement and the reasoning.
    ///
    /// HD authors the same widget as a hexagon **outline**, and a race frame
    /// captured off the running original shows it in its authored grey with the
    /// picture behind it showing through the middle. So HD needs no
    /// substitution, and applying Pulse's turned its backdrop into the
    /// quarter-alpha smudge an HD race drew until 2026-08-25.
    pub pickup_backdrop_colour: Option<&'static str>,
}
