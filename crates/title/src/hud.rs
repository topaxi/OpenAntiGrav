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
//! one field per mode this engine runs and `oag_raceplay::hud_layout` does
//! the mapping. A title that ships a layout for a mode this engine has no rules
//! for keeps it as a constant in its own crate, the way HD's Detonator, Duel
//! and MPTag roots do.

/// The in-race HUD root a title authors for each mode.
///
/// A **root**, not a whole layout: HD composes a mode's HUD out of a shell plus
/// up to sixteen `<LoadXML SrcRel=>` fragments, and splicing those is
/// `oag_hud::compose`'s job. Both PSP titles ship self-contained files,
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
    /// Eliminator, from 2026-09-08.
    ///
    /// **Not `Option`, on the same reasoning [`Self::speed_lap`] states**: a
    /// title that ships no dedicated layout falls back to one that exists
    /// rather than leaving a gap this engine would have to special-case.
    /// Pulse and HD both author `Elimination_HUD.xml`/`elimination_hud.xml`
    /// and use it; Pure, whose own Eliminator layout has not been checked
    /// for, reuses [`Self::arcade`] the way it already reuses
    /// [`Self::time_trial`] for [`Self::speed_lap`] - a title fact stated
    /// once here rather than a `None` every caller would have to handle.
    pub elimination: &'static str,
    /// The title's own two-player split-screen layouts, `None` where it ships
    /// none (both PSP titles and the PS2 pressing).
    pub split: Option<SplitLayouts>,
    /// This project's own split-screen layout for the title, as an entry name
    /// (`oag_hud::split_hud` reads it): where each of the title's widgets goes in
    /// a view, for every view count and shape the authored [`Self::split`] does not
    /// cover. It is looked up through the asset layers, so a player's override beats
    /// it and the disc's own copy (there is none) would sit between; the shipped
    /// one is in `assets/fallback/`. `None` draws the whole-screen layout scaled
    /// into each view instead.
    pub split_layout: Option<&'static str>,
    /// The disc's `HUD Style` list, one entry per skin, default first. Empty on a title
    /// that authors no such list (Pulse, Pure, 2048). A skin's own [`HudSkin::layouts`]
    /// carries an empty list, so the lookup never recurses.
    pub skins: &'static [HudSkin],
}

/// One entry of the disc's `HUD Style` list (`additional_definition.xml`: `HD`, `2097`,
/// `WIP3OUT`) and the roots the executable's race managers load for it.
///
/// A mode the executable names no skinned file for keeps the default skin's root in
/// [`Self::layouts`], as the original does (HD's Eliminator has no `2097_HUD\` or
/// `wo3_HUD\` literal, although the disc ships one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSkin {
    /// The list entry as the disc spells it, which is also what the setting stores.
    pub name: &'static str,
    /// The roots a race of this skin reads.
    pub layouts: &'static HudLayouts,
    /// The language-plugin font slot of this skin's own typeface (`2097HUD`, `wo3HUD`),
    /// the face a layout's `font="2097HUD"` labels draw in. `None` for the default skin.
    pub font_role: Option<&'static str>,
    /// The title's [`HudArt`] with what the skin does not author taken out, `None` for the
    /// title's own. A retro skin lays out its own shield hexagon and hides its shield text
    /// (`color="0"`), so the default skin's runtime shield colours and fill, read off HD's
    /// frames, must not draw over it.
    pub art: Option<&'static HudArt>,
    /// Loyalty the disc's `PI_HUD` row asks before the skin is offered (`Unlock
    /// loyalty=`); `0` where the row is not locked.
    pub loyalty: u32,
}

impl HudLayouts {
    /// The skin a setting names, matched without regard to case, or `None` for an empty
    /// or unknown name (the caller falls back to [`Self::default_skin`] and reports).
    #[must_use]
    pub fn skin(&self, name: Option<&str>) -> Option<&'static HudSkin> {
        let name = name?.trim();
        self.skins
            .iter()
            .find(|skin| skin.name.eq_ignore_ascii_case(name))
    }

    /// The list's default skin (its first entry), `None` on a title with no list.
    #[must_use]
    pub fn default_skin(&self) -> Option<&'static HudSkin> {
        self.skins.first()
    }
}

/// A title's authored two-player split-screen HUD: one root per player, for the
/// two views of a top-and-bottom split.
///
/// **Authored for the whole screen, one layout per player.** HD's
/// `splitscreen_hud_0.xml` and `_1.xml` are both 1920x1080 and the second is
/// the first shifted down by half the screen height (measured: the widgets'
/// extents differ by exactly 540 in y), so player `i`'s HUD is drawn with
/// player `i`'s layout at the identity, over the whole screen, with the view
/// being the half the layout sits in. The `vert_` family is the left and right
/// split, which this engine's layout does not cut; and neither covers more than
/// two players.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitLayouts {
    /// Single race, tournament and Head2Head, player 0 then player 1.
    pub arcade: [&'static str; 2],
    /// Eliminator, player 0 then player 1.
    pub elimination: [&'static str; 2],
    /// The height of one view in the layout's own grid: the half of the screen a
    /// player's layout is authored into. The reticle projects into a screen of
    /// this height and the layout's width.
    pub view_height: u32,
    /// The left-and-right twins of the above, for a race cut vertically, `None` where the
    /// title authors none.
    pub vertical: Option<VerticalLayouts>,
}

/// A title's authored split-screen HUD for two players cut left and right: the `vert_`
/// family beside [`SplitLayouts`].
///
/// HD authors each as the whole 1920x1080 screen with player `i`'s layout in the `i`th
/// half of its width, the second the first moved right by exactly half the screen
/// (`crates/game/tests/split_hud_ground_truth.rs`), so a layout is drawn at the identity over
/// the screen and the view is the column it sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalLayouts {
    /// Single race, tournament and Head2Head, player 0 then player 1.
    pub arcade: [&'static str; 2],
    /// Eliminator, player 0 then player 1.
    pub elimination: [&'static str; 2],
    /// The width of one view in the layout's own grid: the half of the screen a player's
    /// layout is authored into.
    pub view_width: u32,
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
        /// The LeachBeam's own four, or `None` for a title that authors none.
        ///
        /// **The second lockable weapon draws a second set of four**, and it is
        /// a set rather than a recolour: `HudSight_Bind` (`0x0881b604`) binds
        /// nine widgets over *three* models - four instances of the Missile's
        /// corner bracket, one closed box, and four instances of the
        /// LeachBeam's hollow arrowhead. Same placement law, same four
        /// rotations, different texture, and **no inner** on this one.
        ///
        /// `None` is Pure's answer and it is measured rather than assumed: its
        /// `Data\XML\Arcade_HUD.xml` carries `missile_sight_1` … `_4` and
        /// `missile_sight_inner` and no `leachbeam_sight_*` at all, which
        /// agrees with its weapon table authoring no `<Weapon
        /// type="LeachBeam">`. A title with `None` here draws nothing for a
        /// LeachBeam rather than lending it the Missile's brackets.
        leach: Option<[&'static str; 4]>,
    },
    /// Concentric sprites at one centre, at the sizes the layout authors.
    ///
    /// Wipeout HD. Nothing rotates and nothing is offset: every widget is
    /// centred on the reticle and drawn at its authored size, so the *layout*
    /// carries the geometry and this carries only which widget is up when.
    Concentric {
        /// Drawn whenever the reticle has anything, seeking or locked.
        seeking: &'static [&'static str],
        /// Shown once the lock is taken, in place of `MissileSightMiddle`.
        ///
        /// **Read from `Hud_UpdateMissileSight` (`0x0008a5b8`, 2026-10-19), no
        /// longer from the names.** Seeking clears these two widgets' visible
        /// bit and sets `MissileSightMiddle`'s; locked sets these two and
        /// hides the Middle ring, and `Outer`/`Inner` stop turning and rest at
        /// minus and plus thirty degrees. Confidence 75: the visible-bit
        /// writes and the angle literals are read directly; the short shrink
        /// the Middle ring does before it hides is not ported (it is keyed to
        /// a free-running phase, see `hud-sight.md`).
        locked: &'static [&'static str],
        /// The LeachBeam's own reticle on this dialect, or `None` for a title
        /// that authors none.
        ///
        /// **One set of four, with no `locked` counterpart at all** -
        /// unlike [`Self::seeking`]/[`Self::locked`] above. Measured
        /// directly off HD's own composed `Arcade_HUD.xml`
        /// (2026-09-15): `LeachBeamSightBG`/`Outer`/`Middle`/`Inner` are the
        /// whole set, at 176/176/128/80 px and all centred on the same
        /// `(-960, 540)` placeholder every other sight widget on this title
        /// uses, and grepping every composed layout for a
        /// `LeachBeamSight*LockedOn*` name finds none. Confidence 90 - a
        /// literal read of the shipped XML, not an inference. 2048's own
        /// composed layouts carry the identical four-name, no-`LockedOn`
        /// shape at 88/88/64/40 px, corroborating it in a second title on
        /// the same dialect.
        ///
        /// **Which of the four is up when** is a free-running loop, not lock
        /// progress. `Hud_UpdateLeachBeamSight` (`0x00090d30`) adds the raw
        /// frame time to an accumulator and zeroes it at `0.5` s; Outer shows
        /// to `0.125`, Middle to `0.25`, Inner to `0.375`, the backdrop alone
        /// after (three TOC-resolved literals, confidence 88). Drawn off
        /// [`oag_race::sight::Sight::leach_cycle`]. See `hud-sight.md`.
        leach: Option<[&'static str; 4]>,
    },
    /// This title's reticle has not been read.
    ///
    /// Nothing is drawn, which is this project's answer for an asset it cannot
    /// place - see `CLAUDE.md`. No shipped title uses it today; it exists so a
    /// fourth is a row rather than a panic.
    Unread,
}

impl Sights {
    /// Whether a held LeachBeam's reticle runs the LeachBeam's own law
    /// (`FUN_0881e8c8`: spinning arrowheads, no hold timer, the lock taken on
    /// arrival) rather than the Missile's: the PSP dialect with a LeachBeam set,
    /// which is Pulse. HD's concentric rings and Pure's no-LeachBeam brackets keep
    /// the Missile's, because HD's own update is a different function and nothing
    /// has read it.
    #[must_use]
    pub const fn runs_leach_law(&self) -> bool {
        matches!(self, Self::Brackets { leach: Some(_), .. })
    }
}

/// How a title's HUD sprites reach the screen: where their pixels come from,
/// which of them are up whenever the HUD is, and the one colour this build
/// substitutes.
///
/// A fifth axis, on the same terms as [`HudLayouts`] above: it exists because
/// the corpus measurably disagrees on all three rows, not because a HUD
/// obviously needs a table. Every one of them was a `const` inside
/// `oag_hud` giving Pulse's answer to every title until 2026-08-25.
///
/// | | Pulse / Pure | HD / Fury |
/// | --- | --- | --- |
/// | [`Self::texture_extension`] | `None` - the layouts name shipped entries | `.gtf` |
/// | [`Self::always_on`] | seven names | fifteen names |
/// | [`Self::pickup_backdrop_colour`] | `HudBGColour` | `None` - drawn as authored |
/// | [`Self::pickup_colours`] | eleven of thirteen weapons | `None` - unmeasured |
/// | [`Self::pickup_icon_models`] | `None` - icons are `<Image>` sprites | `None` - unmeasured |
/// | [`Self::pickup_icon_backdrop_model`] | `None` - moot with the row above | `None` - moot with the row above |
/// | [`Self::pickup_icon_uv`] | `None` - one widget per weapon | `None` - one widget per weapon |
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
    /// **An allow-list, deliberately**, for the reason `oag_hud`'s label
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
    /// Whether this title's HUD is **raster art authored for a small screen**,
    /// which is what makes a stretch filter (`oag_display`'s `HudScale`)
    /// worth offering: Pulse's 256x256 sheet and 25 px face on a 480x272 PSP or
    /// 640x448 PS2 panel, Pure's the same. `false` for HD, 2048 and Omega,
    /// whose HUD sheets ship at 4x (1024) and whose faces are authored for a
    /// 1080p grid - there is no stretch to sharpen. Measured as a census of
    /// each title's sheet and grid, `docs/ui/hud-skins.md`.
    pub raster: bool,
    /// Whether a **right- or centre-aligned string ends on its last glyph's ink**, not on its advance:
    /// the width the alignment uses is the advances less that glyph's trailing bearing (its advance
    /// less its cell width).
    ///
    /// HD **measured** (2026-10-22, `hd-sharp-hud`): `LAP`, `Record` and `0 kmh` (right-aligned) sat
    /// 2.8, 3.9 and about 4 px left of the original's at 1920x1080, `100` and the lap digit (centred)
    /// 1.7 px, on all 13 cold pairs alike, and the whole-HUD, bars and left-aligned `KM/H` sat within
    /// 0.1 px; with the rule on, the text crops' phase-correlation peak is within 0.3 px on the pairs
    /// measured. Omega inherits HD's HUD engine and font, unmeasured. Pulse and Pure keep their
    /// narrower, measured rule (right-aligned only, on `Self::raster`); 2048 is unmeasured and
    /// `false`.
    pub ink_aligned_text: bool,
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
    /// is invisible. `oag_hud` carries the measurement and the reasoning.
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
    /// Indexed in `oag_tables::weapons::Weapon::ALL`'s declared order -
    /// `[Rocket, Missile, Quake, Cannon, Turbo, Shield, Autopilot, Plasma,
    /// Bomb, Mine, LeachBeam, Repulser, Shuriken]` - rather than carrying that
    /// type, the same reason this crate is `oag-disc` and nothing heavier per
    /// its own module doc: the vocabulary lives here, the `Weapon` type and
    /// the index into it stay in `oag_hud`, which already depends on
    /// `oag-formats` to draw a pickup at all. A slot's own `None` draws
    /// [`Self::pickup_backdrop_colour`] instead, the same way that field's
    /// `None` falls back to the layout's own authored colour - two widening
    /// rings, each optional.
    ///
    /// **Why a colour per weapon at all**: `oag_hud::pickup_sprites`
    /// drew every pickup in one placeholder colour until 2026-09-04, because
    /// the runtime writer that tints `PickupBackground` was unrecovered. It
    /// still is, but the *picture* is no longer unread - see
    /// `oag_pulse::hud::PICKUP_COLOURS` for what closed it and the confidence
    /// split between the three categories.
    ///
    /// Values are `0xAARRGGBB`, the same convention every colour in a HUD
    /// layout is written in, so a reader comparing this table against one
    /// does not have to convert anything by hand.
    pub pickup_colours: Option<[Option<u32>; 14]>,
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
    /// lives here, `Weapon` and the index into it stay in `oag_hud`. A
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
    pub pickup_icon_models: Option<[Option<&'static str>; 14]>,
    /// The `<Mode3D><Model>` widget the icon in [`Self::pickup_icon_models`]
    /// sits on, or `None`.
    ///
    /// The [`Self::pickup_icon_models`] dialect's own equivalent of
    /// `PickupBackground` - Pure's `weapon_icon_grid`, authored at the same
    /// placeholder position as every icon and with no `colour` of its own, so
    /// it needs no substitution the icon models beside it do not either. Moot
    /// whenever [`Self::pickup_icon_models`] is `None`.
    pub pickup_icon_backdrop_model: Option<&'static str>,
    /// A per-weapon source-rectangle rewrite for a title whose pickup slot is
    /// *one* icon widget rather than Pulse/HD's thirteen named ones, or `None`
    /// for a title that names a widget per weapon (or has not measured this).
    ///
    /// **2048's dialect, not a third variant of [`Self::pickup_icon_models`].**
    /// `HUD_pickups.xml`'s `PickupIcon` is the single widget every weapon's
    /// icon draws through; the runtime rewrites its source rectangle rather
    /// than the engine picking between differently-named widgets the way
    /// `oag_hud::pickup_icon_name` does for Pulse and HD, or between
    /// `<Mode3D><Model>`s the way [`Self::pickup_icon_models`] does for Pure.
    /// Confirmed from `Hud_UpdatePickupIcon`
    /// (`docs/ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md`),
    /// not inferred from the layout alone: the XML authors no per-weapon
    /// anything for this widget to select between.
    ///
    /// `[U, V, W, H]` in atlas pixels, `u16` rather than `f32` because the
    /// executable's own table is raw integers - `oag_hud::Sprite::uv`
    /// converts on use, the same four components it carries - indexed in
    /// `oag_tables::weapons::Weapon::ALL`'s declared order, the same axis
    /// [`Self::pickup_colours`] and [`Self::pickup_icon_models`] use and for
    /// the same reason: the `Weapon` type itself stays out of this crate. A
    /// slot's own `None` is a weapon this title's UV table has no entry for
    /// (`Repulser`/`Shuriken`, unimplemented on 2048 per
    /// `docs/gameplay/pickups.md`) rather than an unmeasured one - every
    /// weapon 2048 actually grants has a table entry.
    pub pickup_icon_uv: Option<[Option<[u16; 4]>; 14]>,
    /// What this title calls each rung of its Zone escalation ladder, or `None`
    /// for a title whose ladder has not been read.
    ///
    /// See [`ZoneSpeedClasses`]. The *names* only: which rung a given zone is
    /// on is [`crate::RaceDefaults::zone_stages`], and the two are separate
    /// because the one title that has both recovered them from different
    /// places.
    pub zone_speed_classes: Option<&'static ZoneSpeedClasses>,
    /// Whether `ShieldBarText` carries a `%` after its digits.
    ///
    /// Pulse's own reference frame reads `100%`, which is what
    /// `oag_hud::draw::text_for` drew for every title until this axis
    /// existed. Three HD/Fury frames of the running original
    /// (`data/reference/hd-capture/talons-matched/{00,01,03}.png`: full shield
    /// at the grid, 529 km/h mid-race, and 98% shield after a hit) all read a
    /// bare number - `100`, `100` and `98`, no `%` in any of them - so this is
    /// `false` for HD specifically rather than `None`/unmeasured. Neither
    /// disc's `ShieldBarText` authors a `%`-suffix companion the way
    /// `SpeedBarTextKMH` does for `SpeedBarText`'s unit, so this cannot be
    /// read off the layout the way [`Self::always_on`]'s sibling axis
    /// `speed_unit` is - it has to be a title fact instead. `true` (Pulse's
    /// reading) for every title that has not had its own frame checked,
    /// which is every title but HD today.
    ///
    /// HD's is read off its executable too since 2026-09-23:
    /// `Hud_UpdateShieldReadout` builds the digits by hand and appends
    /// nothing (`docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md`).
    pub shield_percent: bool,
    /// The speed, in km/h, at which this title's `SpeedBar` reads full.
    ///
    /// **HD and Omega: 700, read off the executable.** `Hud_UpdateSpeedReadout`
    /// (`0x00086e88`) multiplies the speed by `0x008a7708`, `0x3abb3ee7` =
    /// 1/700, and crops the fill to the result. The two reference frames
    /// (`talons-matched/00.png`, `01.png`) fit it no worse than 600: at 529
    /// km/h the fill ends at 82% of the bar's width where 700 predicts 76% and
    /// 600 predicts 88%; at 74 km/h it ends at 6% against 11% and 12%.
    ///
    /// **Pulse: 700, read off the executable** (`Hud_UpdateSpeedBar`, `0x0881a3e0`,
    /// `speed / 700.0`; live reads of the fill's width matched it on 59 of 60 ticks).
    ///
    /// **Pure and 2048: 600, ours, not recovered.** Nothing has
    /// established what sets those bars' maximum. 600 is an inference: the
    /// exhaust's own ramp saturates at 600 km/h (`oag_fx::exhaust::RAMP_FLOOR_KMH`
    /// plus `RAMP_SPAN_KMH`, both recovered), a ramp for a visual effect and not a
    /// bar maximum. A reference frame at a known speed would settle it.
    pub speed_full_kmh: u32,
    /// The font role this title's language plugins name for the HUD's value
    /// face - the big lap fraction, position fraction, speed and time
    /// readouts - as `oag_ui::language::Language::font` resolves it.
    ///
    /// **A sixth axis inside this one type, not a second table**: the same
    /// disagreement [`Self::texture_extension`] exists for, one role name
    /// lower. Four of the five titles measure the identical literal
    /// `"HUD"` - Pulse, Pure and HD's own `Data\Plugins\*\Definition.xml`
    /// (or `english/Definition.xml`'s equivalent) all carry
    /// `<Font><Values name="HUD" ...>` - so `"HUD"` in each of those four
    /// title crates is what that title's own plugin spells, not a shared
    /// default reached for four times. 2048 is the one that disagrees: its
    /// `english/Definition.xml` carries no `HUD` slot at all and names
    /// `2048HUD` instead (`Data\XML\2048_hud\font\2048_hud.fnt`), confidence
    /// 90 - see `docs/formats/2048-frontend.md`'s "The language plugins
    /// carry a HUD font role too" section.
    ///
    /// **Why a per-title role rather than the shared literal every caller
    /// used to reach for**: `oag_raceplay::hud::hud_font` used to ask
    /// every source for the literal `oag_ui::language::roles::HUD`, which
    /// drew 2048's HUD in the 5x7 fallback - no 2048 plugin fills that
    /// role - and had a second failure mode besides: two of 2048's
    /// seventeen plugins (`korean`, `traditionalchinese`) carry a
    /// *leftover* `HUD`/`HUDSmall` role pointing at files this title does
    /// not ship (`Data\FE\Fonts\PulseHud.fnt`/`koreanHudSmall.fnt`), which a
    /// first-match search across every loaded plugin could and did pick up
    /// regardless of which language was asked for. Naming the role this
    /// title's own plugin actually fills closes both: the search asks for
    /// `2048HUD` and neither leftover plugin has ever filled that.
    ///
    /// This crate holds no title's data as a rule, so the value lives in
    /// each title crate's own `ART` constant - this field is the axis, not
    /// an answer.
    pub hud_font_role: &'static str,
    /// The font role this title's language plugins name for the HUD's
    /// caption face - the labels beside [`Self::hud_font_role`]'s values -
    /// or `None` when this title's own plugins name no distinct role for it.
    ///
    /// `Some("HUDSmall")` for Pulse, Pure and HD, all measured the same way
    /// [`Self::hud_font_role`]'s doc states for their `"HUD"` row.
    ///
    /// **2048 is `None`, and it is a real gap in the font-role vocabulary
    /// rather than an oversight or a borrowed default - measured at two
    /// levels, not assumed at either.** Every one of its seventeen language
    /// plugins was read and not one carries a second `<Font>` slot for the
    /// caption face - `2048HUD` is the entire HUD vocabulary its
    /// `Definition.xml` authors. **And composing every one of the seven
    /// roots the played skin (`oag_2048::hud::skins::PLAYED`) actually
    /// carries finds zero `font="HUDSmall"` widgets in any of them** - every
    /// label in that skin is `font="HUD"`
    /// (`crates/hud/tests/vita_2048_hud_ground_truth.rs`'s
    /// `the_played_skins_layouts_author_no_hudsmall_widget_at_all`), so no
    /// widget the played HUD draws will ever ask this build to resolve a
    /// caption role in the first place - the gap costs nothing real. A
    /// `font="HUDSmall"` role does exist in this archive, 261 widgets' worth,
    /// but only in the three *unplayed* skins (`wo3_hud`, `2097_hud`, the
    /// bare root) `SpArcadeRaceManager_Construct` and its siblings never
    /// read.
    ///
    /// **The visible size split is measured too, and it is not a second
    /// file.** A Vita3K race frame
    /// (`data/reference/2048-frontend/06-attract-mode-demo-race.png`'s
    /// in-race captures under `docs/formats/2048-hud.md`, e.g.
    /// `21-race2-start.png`) shows caption text (`LAP`, `TOTAL`, `CURRENT`,
    /// `POS`, `XP`) visibly smaller than the value text (`1/3`, `8/8`)
    /// beside it, and the composed layout explains it exactly: `LapTxt`
    /// (`"LAP"`) is authored at `scale=0.6` beside `Laps` (`"1/3"`) at
    /// `scale=1.0`, both `font="HUD"`; `RaceXPTxt`/`RaceXP` (`"XP"`/its
    /// value) are both `scale=0.6`, which is why that pair reads as one
    /// size in the frame while `LAP`/`1/3` reads as two. Every widget's own
    /// `oag_hud::widget::Label::scale` carries the size, on one atlas -
    /// not a guess standing in for an unlocated second `.fnt`.
    ///
    /// So `oag_raceplay::hud::hud_font` falling back to
    /// [`Self::hud_font_role`]'s own face when this is `None` is not merely
    /// the reading that draws *something* recognisable rather than
    /// nothing - it is the reading the played skin's own layouts already
    /// assume, since none of them ever names a second face to fall back
    /// *from*.
    pub hud_small_font_role: Option<&'static str>,
    /// Whether `TotalTime` and `TotalTimeTxt` are up only in the timed modes,
    /// Time Trial and Speed Lap, and hidden in every other.
    ///
    /// **Pulse's rule, read off the executable and seen on a running
    /// original.** `PlayerStatus_Update` (`0x0883b3b8`) writes the clock's
    /// target field to `-1` every tick and overwrites it only when
    /// `g_game_mode` is Time Trial (5), Multiplayer Time Trial (`0x11`), Speed
    /// Lap (10) or Free Play (7); `Hud_UpdateTimeCluster` (`0x0881c9d0`) clears
    /// the visible bit on both widgets while it reads `-1`. A PPSSPP frame of
    /// an Eliminator race (`g_game_mode` 8) shows no `TOTAL` in the top-right
    /// corner, and the two widgets' flag words read live with that bit clear.
    /// See `docs/ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md`.
    ///
    /// `false` for every title that has not had this checked, which is every
    /// title but Pulse: 2048's own frame shows `TOTAL` and `POS` together in a
    /// race with a field (`docs/formats/2048-hud.md`), so the rule is not a
    /// property of the engine.
    pub total_time_timed_modes_only: bool,
    /// Whether the Eliminator's `PosTag0`-`PosTag7` rows and `KillsText`
    /// header are drawn as `"<team> <kills>"` and `KILLS (n)`. **Pulse only**:
    /// read off its executable (`Hud_UpdateKillColumn`) and a live frame.
    /// Wipeout HD authors `KillsText` and six `PosTag` slots of its own
    /// (`docs/formats/hd-hud.md`) and nothing has measured what it writes in
    /// them, so it stays off there rather than borrowing Pulse's answer.
    pub kill_column: bool,
    /// Whether the four `Info1`-`Info4` widgets are the HUD's message lines -
    /// `oag_hud::messages`. Pulse, Pure and HD author the same four widgets in
    /// every race layout (`InfoTextParent` on HD) and each executable's message
    /// routine drains a flag word into them; the slot law (a 0.8 s gate, four
    /// seconds of life, `sin(pi * (1 - (t/life)^6))` alpha) reads the same on
    /// all three. What differs per title is [`Self::lap_hud`]. 2048 and Omega
    /// are off: their layouts are not censused.
    pub message_slots: bool,
    /// What the lap-crossing code raises through those lines and the split gap,
    /// read from this title's own executable. `None` where nobody has read it:
    /// no lap line is raised and no gap runs, rather than Pulse's answer being
    /// borrowed. See [`crate::lap_hud`].
    pub lap_hud: Option<crate::lap_hud::LapHud>,
    /// What this title's per-tick HUD update writes over its layout: the
    /// shield readout's runtime colours and fill, and which segments of the
    /// lap and place arcs are up. See [`RuntimeHud`].
    ///
    /// `None` for every title whose executable has not been read for it -
    /// every title but HD, whose `Hud_UpdateShieldReadout`,
    /// `Hud_UpdateLapCounter` and `Hud_UpdatePositionCounter` are what
    /// `oag_hd::hud::RUNTIME` carries. Pulse's own shield rule is a
    /// different shape and lives on `oag_hud::Readout::shield_forced_red`.
    pub runtime: Option<&'static RuntimeHud>,
    /// The lap counter's lap-one shift: `None` where unmeasured.
    pub lap_one_shift: Option<LapOneShift>,
    /// How the speed and shield bars' fill, marker and label follow the bar's
    /// tip: `None` where unmeasured, which leaves the plain left-anchored crop.
    pub bar_tip: Option<BarTip>,
    /// The boost's multi-copy smear and the glow stamp under the speed bar: `None` where
    /// unmeasured, which draws each widget once and stamps nothing. See [`BoostSmear`].
    pub boost_smear: Option<BoostSmear>,
    /// The whole HUD sliding with the drawn lean: `None` where unmeasured, which leaves the
    /// HUD fixed on the screen. See [`HudSway`].
    pub sway: Option<crate::pre_race::Sourced<HudSway>>,
    /// The player's `Safe Area Setting`, which draws the whole HUD smaller about the screen's
    /// centre: `None` where the title has no such option. See [`SafeArea`].
    pub safe_area: Option<crate::pre_race::Sourced<SafeArea>>,
    /// Pilot Assist's HUD indicator: the widget names, and where a layout that
    /// does not author them gets them from. `None` on a title with no
    /// indicator.
    ///
    /// **Its own field and not part of [`RuntimeHud`]**, which carries HD's
    /// measured shield readout and arcs: a title with an indicator and none of
    /// those rules (Pulse, Pure) must not be handed them to get one.
    pub assist: Option<AssistIndicator>,
    /// How the 3 s `3`, `2`, `1`, `GO` countdown a split race runs on resume is
    /// drawn. See [`ResumeCountdown`].
    pub resume_countdown: ResumeCountdown,
}

/// How a resume countdown reaches the screen on a title.
///
/// **Ours, chosen, not measured**: the original is single-player and has no
/// resume countdown, so this says only whether the title *has screen-space art*
/// to play for one. Pulse's HUD layouts author a `Cockpit321Go` widget and its
/// `Cockpit_321GO.vex`; Pure, HD, 2048 and Omega have no such widget in a HUD
/// layout this build reads (Pure authors `Ready_GO.vex`, HD's `hud_ready_go.xml`
/// is disabled), so they draw the count as text in the title's own `HUD` face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeCountdown {
    /// Play the title's authored overlay (`oag_game::hud_countdown`). If it did
    /// not load, nothing is drawn and the loader says so: never a font stand-in.
    Authored,
    /// Draw `3`, `2`, `1` and the `GO` string in the HUD value face.
    Text,
}

/// The widgets a title's HUD update recolours, crops or shows at runtime,
/// and the values it does it with - the layout authors none of them.
///
/// The values are the executable's; the rules that apply them live in
/// `oag_hud::runtime`, and this type holds nothing a rule could not
/// name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeHud {
    /// The shield readout.
    pub shield: ShieldReadout,
    /// The lap arc: segment `k` is up while `k >= laps - lap`, so the last
    /// `segments - (laps - lap)` are up and one more lights as each lap
    /// starts. Nothing is up on a race with no lap count.
    pub lap_arc: SegmentArc,
    /// The place arc: segment `k` is up while `k <= segments - place`, so
    /// last place shows one and first place shows all of them. `segments`
    /// is the executable's own literal, not the field size.
    pub place_arc: SegmentArc,
    /// The pickup bar's idle look, `None` where unmeasured.
    pub pickup_idle: Option<PickupIdle>,
}

/// The lap counter closes up on lap one: with a lap target and the lap equal to
/// one, the executable moves the two glyphs after the digit left by [`Self::dx`]
/// (a one-digit lap leaves the gap a two-digit one fills). Pulse's
/// `Hud_UpdateLapCounter` (`0x0881a690`), `-17.0` in the 480-wide layout space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LapOneShift {
    /// The widgets that move: the separator and the lap total.
    pub widgets: &'static [&'static str],
    /// Added to their authored `x` on lap one.
    pub dx: i32,
}

/// The race HUD slides across the screen with the ship's drawn lean, as a whole.
///
/// Wipeout HD's `Hud_Update` (`0x0009e704`-`0x0009e754`) writes one translation into the
/// top-level `"HUD"` and `"InGame"` screens it looked up by name at load, and every layout's
/// root element is `<Screen name="HUD">`, so every widget the race draws moves with it: the
/// bars, the clocks, the message lines, the pickup slot and the fixed reticle. On a
/// running original a held full stick moved the whole HUD by 13.3 px at 1280x720, all five
/// regions measured alike, and each plateau of the drawn lean's follower (`ship+0x6a58`) did
/// the same at its own size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSway {
    /// Layout units the HUD moves along **x** per unit of the drawn lean's follower
    /// (`oag_physics::ShipState::steer_lean_follower`): HD's `-15.0` literal, so a stick to the right
    /// slides the HUD to the left.
    pub x_per_lean: i16,
}

/// A title's `Safe Area Setting`: a slider in percent whose value is how much of the screen the
/// HUD uses, the whole HUD drawn at `percent / 100` about the screen's centre.
///
/// HD authors the slider in its Game Options (`additional_definition.xml`, `Slider
/// name="Safe Area Setting" default="90%"`, `minSlide="80" maxSlide="100" incSlide="2"`). The
/// scale reading is inferred from one live read: the save the HD comparison pairs were restored
/// from holds `96.0` in the option's global (`+0x104` of the object behind the TOC slot at
/// `0x8a5bb4`), and a correlation of that frame's HUD against ours puts the HUD at
/// `0.964`-`0.968` of the screen. Poking `90.0` into the global and restarting the race did
/// not move the HUD, so the consumer is unread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafeArea {
    /// The value a fresh profile starts at.
    pub default_percent: u8,
    /// The slider's lowest value.
    pub min_percent: u8,
    /// The slider's highest value.
    pub max_percent: u8,
    /// The slider's step.
    pub step_percent: u8,
}

impl SafeArea {
    /// The HUD's scale for a player's `percent` (`None` for the title's default): clamped to the
    /// slider's range and snapped to its step, as a slider would hold it.
    #[must_use]
    pub fn scale(self, percent: Option<u8>) -> f32 {
        let value = percent
            .unwrap_or(self.default_percent)
            .clamp(self.min_percent, self.max_percent);
        let step = self.step_percent.max(1);
        let snapped = self.min_percent + (value - self.min_percent) / step * step;
        f32::from(snapped) / 100.0
    }
}

/// The speed and shield bars' fill law: the tip of the fill is where the marker
/// and the value label sit.
///
/// Pulse's `FUN_0881a3e0` (the speed bar) and `Hud_UpdateEnergyBar`
/// (`0x0881c638`) run one law: the value as a fraction `f` of full, then
/// `f' = floor + span * f`, the fill's pixel width `floor(f' * background width)`,
/// its texture window the **right** `f'` of the art (so the art's pointed end is
/// always the tip), and the marker and the label moved to `authored x + width +
/// tip_dx`. The speed bar alone reads a speed under `idle_kmh` as zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarTip {
    /// The fraction a bar shows at zero, in thousandths: `40` is `0.04`.
    pub floor_permille: u16,
    /// The rest of the range, in thousandths: `960` is `0.96`.
    pub span_permille: u16,
    /// Added to the tip's pixel width for the marker and label: `-6`.
    pub tip_dx: i16,
    /// A speed under this reads as zero, in km/h: `5`.
    pub idle_kmh: u16,
    /// The bars this law drives.
    pub bars: &'static [TipBar],
}

/// What a speed pad's boost does to the HUD (Pulse PSP, `Hud_Update` `0x0881bf50`).
///
/// While the exhaust flare's boost timer runs, the game counts `n = trunc(peak * w)`, `w` a
/// triangle of `timer / seconds` that is `0` at both ends and `1` at the middle, and draws every
/// widget `n` times, each at `alpha / n` and each shifted a little further from the screen's
/// centre along the line through it; the speed bar's fill also stamps the bloom's glow mask with
/// `n / peak * 255`, so the bloom the frame already has blooms the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoostSmear {
    /// The copy count at the middle of the timer, and the divisor of the stamp: `10`.
    pub peak: u8,
    /// The point the copies spread away from, in the layout's grid: `[240, 136]`.
    pub centre: [i16; 2],
    /// The widget whose fill stamps the glow mask.
    pub stamp: &'static str,
}

/// One bar of a [`BarTip`] law.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TipBar {
    /// The fill widget.
    pub fill: &'static str,
    /// The marker that rides the tip, if the layout authors one.
    pub mark: &'static str,
    /// The value label that rides the tip.
    pub text: &'static str,
    /// What the bar shows.
    pub source: TipSource,
}

/// The value a [`TipBar`] shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipSource {
    /// The speed over [`HudArt::speed_full_kmh`]; the label is the speed in
    /// whole km/h, truncated, with its unit.
    Speed,
    /// The shield as a percentage of the pool; the label is the percentage
    /// truncated, then `%`.
    Shield,
}

/// The dim the pickup bar is drawn in while it has nothing to show: the
/// executable resets its widgets to white each tick and writes this colour
/// over the ones with no content (HD's `0x0008df30`, `0x5affffff`, alpha 0.35).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickupIdle {
    /// The hexagon behind a held pickup's icon: dim while nothing is held.
    pub backdrop: &'static str,
    /// The two pills beside it that carry the recent damage and absorb
    /// readouts: dim while no hit is showing.
    pub hit_pills: &'static [&'static str],
    /// The dim, as `0xAARRGGBB`.
    pub argb: u32,
}

/// Pilot Assist's four indicator widgets; an arrow lists its child images too, which
/// the layout authors nested under it and the original shows with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssistIndicator {
    /// Up while the option is on.
    pub background: &'static str,
    /// Up for three seconds after a correction.
    pub main: &'static str,
    /// Blinks after a correction off a left-hand wall.
    pub left: &'static [&'static str],
    /// Blinks after a correction off a right-hand wall.
    pub right: &'static [&'static str],
    /// The layout fragment holding these widgets, for a title whose own layouts
    /// do not include them. Read through the source's archives, so on a title
    /// whose disc and DLC lack it the fallback layer answers (`oag_assets::layers`).
    /// `None` where the disc's layouts embed the widgets themselves, as HD's do.
    pub fragment: Option<&'static str>,
}

/// A shield readout made of a background, a fill cropped from the top, and
/// a number, recoloured every tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldReadout {
    /// The fill widget, cropped to the shield fraction with its bottom edge
    /// fixed.
    pub fill: &'static str,
    /// The background widget under it, which flashes [`Self::warning_rgb`].
    pub background: &'static str,
    /// The number, drawn in the fill's colour.
    pub text: &'static str,
    /// The fill's and the number's colour, `0xRRGGBB`, drawn opaque.
    pub rgb: u32,
    /// The background's colour on a flash's "on" phase, `0xRRGGBB`.
    pub warning_rgb: u32,
    /// At or under this many percent the readout flashes.
    pub critical_percent: u32,
    /// Flash phases per second: a phase is on when `floor(t * this)` is even.
    pub phases_per_second: u32,
}

/// A ring of numbered segment widgets, `<prefix>0` .. `<prefix><segments - 1>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentArc {
    /// The widget-name prefix the segment number is appended to.
    pub prefix: &'static str,
    /// How many segments the ring has.
    pub segments: u32,
}

/// The name of each rung of a title's Zone escalation ladder, as string-table
/// ids.
///
/// # A rung, not a zone
///
/// **`ids[n]` is the name of stage `n`**, the same index
/// `oag_raceplay::zone_grade::ZoneGrade` shows and `--zone-stage` selects -
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
    /// Where each rung begins, as a zone number, for a title whose HUD names
    /// the class **off the zone** instead of off the grade's stage.
    ///
    /// Pulse is that title: `FUN_0881ad10` walks the zone counter down a
    /// ladder of thresholds with no stage in between (Pulse has no grade, so no
    /// stage). Rung `n` begins at `rung_zones[n]`, ascending, the same length as
    /// [`Self::ids`]. `None` keeps the stage-keyed reading HD uses.
    pub rung_zones: Option<&'static [u32]>,
    /// From this zone on the `SpeedClassTxt` caption (`speed class`) is blanked.
    ///
    /// Pulse's top rung writes an empty string over it. `None` never blanks.
    pub caption_hidden_from_zone: Option<u32>,
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

    /// The rung the HUD names at `stage` and `zone`: the zone's own rung where
    /// the title keys the class on the zone ([`Self::rung_zones`]), the grade's
    /// `stage` otherwise.
    #[must_use]
    pub fn rung(&self, stage: u32, zone: u32) -> u32 {
        match self.rung_zones {
            Some(starts) => {
                let began = starts.iter().take_while(|&&start| start <= zone).count();
                u32::try_from(began.saturating_sub(1)).unwrap_or(0)
            }
            None => stage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LADDER: ZoneSpeedClasses = ZoneSpeedClasses {
        ids: &[None, Some("A"), Some("B")],
        rung_zones: None,
        caption_hidden_from_zone: None,
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
        let empty = ZoneSpeedClasses {
            ids: &[],
            rung_zones: None,
            caption_hidden_from_zone: None,
        };
        assert_eq!(empty.id_for(0), None);
        assert_eq!(empty.id_for(7), None);
    }
}
