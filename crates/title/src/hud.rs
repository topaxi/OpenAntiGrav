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

/// How a title authors its lock-on reticle.
///
/// **An axis in the [ADR-0022] sense**: the two answers differ in shape, not
/// just in spelling, and neither is derivable from the other.
///
/// Both PSP titles author the reticle as `<Mode3D>` **models** - one corner
/// bracket instanced four times at the corners of a box that opens and closes,
/// plus a closed box in the middle - and the runtime writes each instance a
/// position *and a rotation*. Wipeout HD authors it as concentric `<Image>`
/// **sprites** at fixed authored sizes, with a separate pair for the locked
/// state, and rotates nothing.
///
/// What the two share is the placement law recovered from `HudSight_Update`
/// (`docs/ghidra/functions/psp-pulse-usa/lock-sight.md`): where the centre goes,
/// how it chases, and when the lock is taken. That is why this is an axis on the
/// *art* rather than three copies of the law.
///
/// **The placeholder idiom is shared too, which is what says the reading is
/// right.** Every sight widget on both dialects is authored at a position whose
/// centre is `(-width/2, +height/2)` of that title's own screen - `(-240, 136)`
/// on the PSP's 480x272, `(-960, 540)` on HD's 1920x1080 - so all of them are
/// placeholders the runtime overwrites, in the same way and by the same
/// arithmetic.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sights {
    /// Four instances of one corner-bracket model at the corners of the box,
    /// plus one closed box at its centre. Both PSP titles.
    ///
    /// The four are placed at `±extent` and rotated a quarter turn apart; the
    /// inner leads them. Sizes come from the models' own quads.
    Brackets {
        /// The four bracket widgets, in the order the original's slot run binds
        /// them - which is also the order the four rotations are indexed by.
        brackets: [&'static str; 4],
        /// The closed box at the middle.
        inner: &'static str,
    },
    /// Concentric sprites at one centre, at the sizes the layout authors.
    ///
    /// Wipeout HD. Nothing rotates and nothing is offset: every widget is
    /// centred on the reticle and drawn at its authored size, so the *layout*
    /// carries the geometry and this carries only which widget is up when.
    Concentric {
        /// Drawn whenever the reticle has anything, seeking or locked.
        seeking: &'static [&'static str],
        /// Drawn in addition once the lock is taken.
        ///
        /// Read off the widget names rather than out of HD's own code, which
        /// nothing here has disassembled - `MissileSightLockedOnLines` and
        /// `MissileSightLockedOnMiddle` say what they are. Confidence **70**:
        /// the names are unambiguous about *what* these are and not about
        /// whether the seeking set stays up underneath them. Drawn additively
        /// here, which is the reading that shows every authored widget rather
        /// than hiding some on a guess.
        locked: &'static [&'static str],
    },
    /// This title's reticle has not been read.
    ///
    /// Nothing is drawn, which is this project's answer for an asset it cannot
    /// place - see `CLAUDE.md`. No shipped title uses it today; it exists so a
    /// fourth is a row rather than a panic.
    Unread,
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
/// | [`Self::pickup_colours`] | eleven of thirteen weapons | `None` - unmeasured |
/// | [`Self::pickup_icon_models`] | `None` - icons are `<Image>` sprites | `None` - unmeasured |
/// | [`Self::pickup_icon_backdrop_model`] | `None` - moot with the row above | `None` - moot with the row above |
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
    /// How this title authors its lock-on reticle. See [`Sights`].
    ///
    /// Not part of [`Self::always_on`] and deliberately so: the sights are up
    /// only while a locking weapon is held and something is lockable, which is
    /// a runtime question that list cannot express.
    pub sights: &'static Sights,
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
    /// A per-weapon backdrop colour that overrides [`Self::pickup_backdrop_colour`]
    /// for the slots this title has actually measured, or `None` for a title
    /// that has measured none.
    ///
    /// Indexed in `oag_formats::weapons::Weapon::ALL`'s declared order -
    /// `[Rocket, Missile, Quake, Cannon, Turbo, Shield, Autopilot, Plasma,
    /// Bomb, Mine, LeachBeam, Repulser, Shuriken]` - rather than carrying that
    /// type, the same reason this crate is `oag-disc` and nothing heavier per
    /// its own module doc: the vocabulary lives here, the `Weapon` type and
    /// the index into it stay in `oag_game::hud`, which already depends on
    /// `oag-formats` to draw a pickup at all. A slot's own `None` draws
    /// [`Self::pickup_backdrop_colour`] instead, the same way that field's
    /// `None` falls back to the layout's own authored colour - two widening
    /// rings, each optional.
    ///
    /// **Why a colour per weapon at all**: `oag_game::hud::pickup_sprites`
    /// drew every pickup in one placeholder colour until 2026-09-04, because
    /// the runtime writer that tints `PickupBackground` was unrecovered. It
    /// still is, but the *picture* is no longer unread - see
    /// `oag_pulse::hud::PICKUP_COLOURS` for what closed it and the confidence
    /// split between the three categories.
    ///
    /// Values are `0xAARRGGBB`, the same convention every colour in a HUD
    /// layout is written in, so a reader comparing this table against one
    /// does not have to convert anything by hand.
    pub pickup_colours: Option<[Option<u32>; 13]>,
    /// The `<Mode3D><Model>` widget that draws a weapon's own icon, when this
    /// title has one, or `None` for a title whose icons are `<Image>` sprites
    /// (or unmeasured).
    ///
    /// **An axis in the [`Sights`] sense, not a second [`Self::pickup_colours`]
    /// table.** Pulse's icon widgets are `<Image>` sprites named `<Type>Icon`
    /// with authored-white art that needs [`Self::pickup_colours`]'s runtime
    /// substitution to read as anything but a hexagon outline. Pure's are
    /// `<Mode3D><Model>`s named `<TYPE>_icon`, and each one this table names
    /// already carries its own final `colour` in the XML - a different widget
    /// kind that needs no substitution at all, not the same fact spelled
    /// differently. A title with both kinds has not been measured; this field
    /// exists for the model kind exclusively.
    ///
    /// Indexed in `Weapon::ALL`'s declared order, the same convention
    /// [`Self::pickup_colours`] uses and for the same reason: the vocabulary
    /// lives here, `Weapon` and the index into it stay in `oag_game::hud`. A
    /// slot's own `None` means this title's disc authors no icon for that
    /// weapon at all - measured absent, not unmeasured, the distinction
    /// [`Self::pickup_colours`]'s own doc draws. Pure's own table has ten of
    /// thirteen filled: no `Cannon`, `LeachBeam` or `Repulser` icon exists on
    /// its disc, and neither does `Shuriken` - a weapon `oag_gameplay::
    /// pickup::IMPLEMENTED` hands out that predates this title's own roster,
    /// so a Pure race can genuinely hold a weapon its HUD has no icon for.
    /// `Quake`'s slot is filled even though nothing hands a Quake out today:
    /// the disc authors the widget, so recording it costs nothing and saves a
    /// second reading the day `Quake` joins `IMPLEMENTED`.
    pub pickup_icon_models: Option<[Option<&'static str>; 13]>,
    /// The `<Mode3D><Model>` widget the icon in [`Self::pickup_icon_models`]
    /// sits on, or `None`.
    ///
    /// The [`Self::pickup_icon_models`] dialect's own equivalent of
    /// `PickupBackground` - Pure's `weapon_icon_grid`, authored at the same
    /// placeholder position as every icon and with no `colour` of its own, so
    /// it needs no substitution the icon models beside it do not either. Moot
    /// whenever [`Self::pickup_icon_models`] is `None`.
    pub pickup_icon_backdrop_model: Option<&'static str>,
    /// What this title calls each rung of its Zone escalation ladder, or `None`
    /// for a title whose ladder has not been read.
    ///
    /// See [`ZoneSpeedClasses`]. The *names* only: which rung a given zone is
    /// on is [`crate::RaceDefaults::zone_stages`], and the two are separate
    /// because the one title that has both recovered them from different
    /// places.
    pub zone_speed_classes: Option<&'static ZoneSpeedClasses>,
}

/// The name of each rung of a title's Zone escalation ladder, as string-table
/// ids.
///
/// # A rung, not a zone
///
/// **`ids[n]` is the name of stage `n`**, the same index
/// `oag_game::race::zone_grade::ZoneGrade` shows and `--zone-stage` selects -
/// not a zone number. `None` in a slot is a rung with no name of its own, which
/// is what the bottom of HD's ladder is.
///
/// That distinction is the correction this type was rewritten for. A first pass
/// read HD's fifteen rungs as one per zone, off a reference frame showing
/// `SUB-VENOM` at zone 1 and `VENOM` at zone 2; **the maintainer's own play says
/// otherwise - not every zone is a class bump** - and the frame is equally
/// consistent with a ladder whose first band happens to be one zone wide, which
/// is exactly the shape `oag_2048::race::ZONE_STAGES` has (`0`-`1`, then
/// `2`-`8`). So what turns a zone counter into a rung stays where it was:
/// recovered on 2048, unrecovered on HD, and asked for through
/// [`crate::RaceDefaults::zone_stages`] rather than guessed at here.
///
/// This carries only the **names**, which are recoverable from the disc alone
/// and are the same list whatever drives the index.
///
/// # Ids rather than text
///
/// The strings are localised, so the caller resolves each against the language
/// plugin's own table - the same way an `idstring` caption is resolved.
///
/// A stage past the end takes the last rung, the clamp every ladder in this
/// lineage ends with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneSpeedClasses {
    /// One entry per rung, in stage order from zero. See the type's docs.
    pub ids: &'static [Option<&'static str>],
}

impl ZoneSpeedClasses {
    /// The id for `stage`, clamped to the last rung. `None` for a rung with no
    /// name and for an empty ladder.
    #[must_use]
    pub fn id_for(&self, stage: u32) -> Option<&'static str> {
        let last = self.ids.len().checked_sub(1)?;
        let rung = usize::try_from(stage).unwrap_or(last);
        self.ids.get(rung.min(last)).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LADDER: ZoneSpeedClasses = ZoneSpeedClasses {
        ids: &[None, Some("A"), Some("B")],
    };

    /// The bottom rung has no name, and saying so is the point of the `Option`:
    /// HD's `0 Start` is a palette with no speed class beside it.
    #[test]
    fn an_unnamed_rung_reads_as_nothing_rather_than_as_the_next_one() {
        assert_eq!(LADDER.id_for(0), None);
        assert_eq!(LADDER.id_for(1), Some("A"));
    }

    /// Past the top, the top holds - the clamp every ladder in this lineage
    /// ends with.
    #[test]
    fn a_stage_past_the_end_takes_the_last_rung() {
        assert_eq!(LADDER.id_for(2), Some("B"));
        assert_eq!(LADDER.id_for(99), Some("B"));
    }

    /// A ladder with nothing in it names nothing, rather than indexing out of
    /// bounds.
    #[test]
    fn an_empty_ladder_names_nothing() {
        let empty = ZoneSpeedClasses { ids: &[] };
        assert_eq!(empty.id_for(0), None);
        assert_eq!(empty.id_for(7), None);
    }
}
