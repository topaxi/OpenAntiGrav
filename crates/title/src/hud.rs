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
        /// **Which of the four is up when is now ported, at reduced
        /// confidence relative to the widget existence question above.**
        /// [`hud-sight.md`]'s own `Hud_UpdateLeachBeamSight` (`0x00090d30`)
        /// reading (2026-09-15, confidence 88) found the four are not "all
        /// four, always": Outer, Middle and Inner each get one exclusive
        /// quarter of HD's own `0.5` s hold window (`t <= 0.125`s shows
        /// Outer, `0.125 < t <= 0.25` shows Middle, `0.25 < t <= 0.375` shows
        /// Inner, past that only BG shows until the lock completes), read off
        /// three literal TOC-relative float constants - not a distance table
        /// as an earlier pass of that page guessed. `oag_hud::sight_draw`
        /// (`leach_reveal_draws`) drives this off
        /// [`oag_race::sight::Sight::hold_progress`] - a 0..1 fraction of
        /// *this engine's own* `HOLD_SECONDS` (the PSP's `0.8`, not HD's
        /// `0.5` - see that constant's own doc), scaled onto the same
        /// quarters HD's own code uses rather than HD's absolute second
        /// marks. So the shape is measured and ported; the exact second at
        /// which each ring changes on this title is not HD's own.
        ///
        /// [`hud-sight.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md
        leach: Option<[&'static str; 4]>,
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
    /// (`crates/game/tests/vita_2048_hud_ground_truth.rs`'s
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
    /// `oag_hud::messages`. **Pulse only**: `Hud_UpdateMessages`
    /// (`0x0881f148`) is Pulse's own executable, and HD authors an `Info1` of its
    /// own in `HUD_Elim_info_text.xml` for something else
    /// (`docs/formats/hd-hud.md`), so it stays off there.
    pub message_slots: bool,
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
