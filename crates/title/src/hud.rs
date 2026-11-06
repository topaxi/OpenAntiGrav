//! Which layout a title's in-race HUD is read from, per mode this engine runs.
//!
//! An axis on the same terms as [`crate::boot`], [`crate::menu`] and
//! [`crate::race`]: it exists because titles measurably disagree.
//!
//! | | Pulse / Pure | HD / Fury |
//! | --- | --- | --- |
//! | single race | `Data\XML\Arcade_HUD.xml` | `/data/xml/arcade_hud.xml` |
//! | time trial | `Data\XML\TimeTrial_HUD.xml` | `/data/xml/timetrial_hud.xml` |
//! | speed lap | *the time trial's* | `/data/xml/speedlap_hud.xml` |
//! | zone | `Data\XML\Zone_HUD.xml` | `/data/xml/zone_hud.xml` |
//!
//! **Speed lap is the row that diverges.** Neither PSP disc ships a
//! `SpeedLap_HUD.xml` (the name hashes to `1af0a646` and no entry carries it),
//! so Pulse and Pure draw the time trial's layout, which is why
//! `docs/ui/hud.md` counts five layouts for six modes. HD ships a separate one.
//!
//! # The other three rows agree
//!
//! Pure carries Pulse's three spellings verbatim, and HD's fold onto them
//! through PSARC path normalisation (`Data\XML\Arcade_HUD.xml` becomes
//! `data/xml/arcade_hud.xml`, which HD's manifest stores). Each row is still a
//! per-title measurement that came out the same: when one stops agreeing (2048
//! is a different engine generation) it changes in the title package, not the
//! engine.
//!
//! # Modes as fields, not a map
//!
//! `oag_race::Mode` is the engine's type and a title package must not grow one
//! (the rule `oag_pulse::race` states for the Zone hull), so this has one field
//! per mode the engine runs and `oag_raceplay::hud_layout` does the mapping. A
//! layout for a mode the engine has no rules for stays a constant in the
//! title's own crate, like HD's Detonator, Duel and MPTag roots.

/// The in-race HUD root a title authors for each mode.
///
/// A **root**, not a whole layout: HD composes a mode's HUD from a shell plus up
/// to sixteen `<LoadXML SrcRel=>` fragments, spliced by `oag_hud::compose`. Both
/// PSP titles ship self-contained files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudLayouts {
    /// Single race.
    pub arcade: &'static str,
    /// Time trial.
    pub time_trial: &'static str,
    /// Speed lap. Equal to [`Self::time_trial`] on a title that ships no
    /// separate layout (both PSP discs): a measured fact, not a fallback.
    pub speed_lap: &'static str,
    /// Zone.
    pub zone: &'static str,
    /// Eliminator. Not `Option`, like [`Self::speed_lap`]: a title with no
    /// dedicated layout falls back to one that exists. Pulse and HD author
    /// `Elimination_HUD.xml`/`elimination_hud.xml`; Pure's own has not been
    /// checked for, so it reuses [`Self::arcade`].
    pub elimination: &'static str,
    /// The title's own two-player split-screen layouts, `None` where it ships
    /// none (both PSP titles and the PS2 pressing).
    pub split: Option<SplitLayouts>,
    /// This project's split-screen layout for the title, as an entry name
    /// (`oag_hud::split_hud` reads it): where each widget goes in a view, for
    /// every view count and shape [`Self::split`] does not cover. Looked up
    /// through the asset layers, so a player's override wins; the shipped one is
    /// in `assets/fallback/`. `None` draws the whole-screen layout scaled into
    /// each view.
    pub split_layout: Option<&'static str>,
    /// The layout whose `PlrTag` text is the face of a multiplayer race's player tags
    /// (Pulse's `MPTag_HUD.xml`; `Multiplayer Tags`). `None` where the tags are not wired.
    pub player_tags: Option<&'static str>,
    /// The disc's `HUD Style` list, one entry per skin, default first. Empty
    /// where the title authors none (Pulse, Pure, 2048). A skin's own
    /// [`HudSkin::layouts`] carries an empty list, so the lookup never recurses.
    pub skins: &'static [HudSkin],
}

/// One entry of the disc's `HUD Style` list (`additional_definition.xml`: `HD`,
/// `2097`, `WIP3OUT`) and the roots the executable's race managers load for it.
///
/// A mode the executable names no skinned file for keeps the default skin's
/// root in [`Self::layouts`], as the original does (HD's Eliminator has no
/// `2097_HUD\` or `wo3_HUD\` literal, although the disc ships one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSkin {
    /// The list entry as the disc spells it, which is also what the setting stores.
    pub name: &'static str,
    /// The roots a race of this skin reads.
    pub layouts: &'static HudLayouts,
    /// The language-plugin font slot of this skin's own typeface (`2097HUD`,
    /// `wo3HUD`), the face a layout's `font="2097HUD"` labels draw in. `None` for
    /// the default skin.
    pub font_role: Option<&'static str>,
    /// The title's [`HudArt`] minus what the skin does not author, `None` for the
    /// title's own. A retro skin lays out its own shield hexagon and hides its
    /// shield text (`color="0"`), so the default skin's runtime shield colours
    /// and fill, read off HD's frames, must not draw over it.
    pub art: Option<&'static HudArt>,
    /// Loyalty the disc's `PI_HUD` row asks before the skin is offered (`Unlock
    /// loyalty=`); `0` where not locked.
    pub loyalty: u32,
}

impl HudLayouts {
    /// The skin a setting names, matched ignoring case, or `None` for an empty
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
/// `splitscreen_hud_0.xml` and `_1.xml` are both 1920x1080 and the second is the
/// first shifted down by half the screen height (the widgets' extents differ by
/// exactly 540 in y), so player `i`'s layout is drawn at the identity over the
/// whole screen and the view is the half it sits in. The `vert_` family is the
/// left and right split, which this engine's layout does not cut; neither covers
/// more than two players.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitLayouts {
    /// Single race, tournament and Head2Head, player 0 then player 1.
    pub arcade: [&'static str; 2],
    /// Eliminator, player 0 then player 1.
    pub elimination: [&'static str; 2],
    /// The height of one view in the layout's own grid. The reticle projects
    /// into a screen of this height and the layout's width.
    pub view_height: u32,
    /// The left-and-right twins of the above, `None` where the title authors none.
    pub vertical: Option<VerticalLayouts>,
}

/// A title's authored split-screen HUD for two players cut left and right: the
/// `vert_` family beside [`SplitLayouts`].
///
/// HD authors each as the whole 1920x1080 screen with player `i`'s layout in the
/// `i`th half of its width, the second the first moved right by exactly half the
/// screen (`crates/game/tests/split_hud_ground_truth.rs`), so a layout is drawn
/// at the identity and the view is the column it sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalLayouts {
    /// Single race, tournament and Head2Head, player 0 then player 1.
    pub arcade: [&'static str; 2],
    /// Eliminator, player 0 then player 1.
    pub elimination: [&'static str; 2],
    /// The width of one view in the layout's own grid.
    pub view_width: u32,
}

/// `reference` with its file extension replaced by `extension`.
///
/// The rule [`HudArt::texture_extension`] names, applied. A reference with no
/// extension gains one; a dot in a directory name is not an extension
/// (`Data\v1.2\atlas` does not become `Data\v1`). Separators and case are
/// untouched, the archive readers fold both.
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
/// just spelling.
///
/// Both PSP titles author it as `<Mode3D>` **models**: one corner bracket
/// instanced four times at the corners of a box that opens and closes, plus a
/// closed box in the middle, each instance written a position *and a rotation*.
/// Wipeout HD authors concentric `<Image>` **sprites** at fixed sizes, with a
/// separate pair for the locked state, and rotates nothing.
///
/// What they share is the placement law recovered from `HudSight_Update`
/// (`docs/ghidra/functions/psp-pulse-usa/lock-sight.md`): where the centre goes,
/// how it chases, when the lock is taken. So the axis is on the *art*, not three
/// copies of the law.
///
/// The placeholder idiom is shared too, which says the reading is right: every
/// sight widget on both dialects is authored with its centre at
/// `(-width/2, +height/2)` of that title's screen (`(-240, 136)` on the PSP's
/// 480x272, `(-960, 540)` on HD's 1920x1080), a placeholder the runtime
/// overwrites.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sights {
    /// Four instances of one corner-bracket model at the corners of the box,
    /// plus one closed box at its centre. Both PSP titles.
    ///
    /// The four sit at `±extent`, a quarter turn apart; the inner leads them.
    /// Sizes come from the models' own quads.
    Brackets {
        /// The four bracket widgets, in the order the original's slot run binds
        /// them - which is also the order the four rotations are indexed by.
        brackets: [&'static str; 4],
        /// The closed box at the middle.
        inner: &'static str,
        /// The LeachBeam's own four, or `None` for a title that authors none.
        ///
        /// **A second set of four, not a recolour**: `HudSight_Bind`
        /// (`0x0881b604`) binds nine widgets over *three* models (four Missile
        /// corner brackets, one closed box, four LeachBeam hollow arrowheads).
        /// Same placement law and rotations, different texture, **no inner**.
        ///
        /// `None` is Pure's measured answer: its `Data\XML\Arcade_HUD.xml` has
        /// `missile_sight_1` … `_4` and `missile_sight_inner` and no
        /// `leachbeam_sight_*`, which agrees with its weapon table authoring no
        /// `<Weapon type="LeachBeam">`. `None` draws nothing for a LeachBeam
        /// rather than lending it the Missile's brackets.
        leach: Option<[&'static str; 4]>,
    },
    /// Concentric sprites at one centre, at the sizes the layout authors.
    ///
    /// Wipeout HD. Nothing rotates or is offset, so the *layout* carries the
    /// geometry and this carries which widget is up when.
    Concentric {
        /// Drawn whenever the reticle has anything, seeking or locked.
        seeking: &'static [&'static str],
        /// Shown once the lock is taken, in place of `MissileSightMiddle`.
        ///
        /// **Read from `Hud_UpdateMissileSight` (`0x0008a5b8`), not from the
        /// names** (`docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`,
        /// "Correction, 2026-10-19"). Seeking clears these two widgets' visible bit
        /// and sets `MissileSightMiddle`'s; locked sets these two and hides the
        /// Middle ring, and `Outer`/`Inner` stop turning at minus and plus thirty
        /// degrees. Confidence 75: the visible-bit writes and angle literals are
        /// read directly; the short shrink the Middle ring does before it hides is
        /// not ported (keyed to a free-running phase).
        locked: &'static [&'static str],
        /// The LeachBeam's own reticle on this dialect, or `None` for a title
        /// that authors none.
        ///
        /// **One set of four, with no `locked` counterpart**, unlike
        /// [`Self::seeking`]/[`Self::locked`]. Read off HD's composed
        /// `Arcade_HUD.xml` (`docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`,
        /// "2026-09-15"): `LeachBeamSightBG`/`Outer`/`Middle`/`Inner` at
        /// 176/176/128/80 px, all centred on the `(-960, 540)` placeholder, and no
        /// `LeachBeamSight*LockedOn*` name in any composed layout. Confidence 90.
        /// 2048's composed layouts carry the same four names at 88/88/64/40 px.
        ///
        /// **Which of the four is up** is a free-running loop, not lock progress:
        /// `Hud_UpdateLeachBeamSight` (`0x00090d30`) adds the raw frame time to an
        /// accumulator zeroed at `0.5` s; Outer shows to `0.125`, Middle to
        /// `0.25`, Inner to `0.375`, the backdrop alone after (three TOC-resolved
        /// literals, confidence 88). Drawn off
        /// [`oag_race::sight::Sight::leach_cycle`].
        leach: Option<[&'static str; 4]>,
    },
    /// This title's reticle has not been read.
    ///
    /// Nothing is drawn (the CLAUDE.md answer for an asset it cannot place). No
    /// shipped title uses it; a fourth title is a row, not a panic.
    Unread,
}

impl Sights {
    /// Whether a held LeachBeam's reticle runs the LeachBeam's own law
    /// (`FUN_0881e8c8`: spinning arrowheads, no hold timer, lock taken on
    /// arrival) rather than the Missile's: the PSP dialect with a LeachBeam set,
    /// which is Pulse. HD's concentric rings and Pure's no-LeachBeam brackets keep
    /// the Missile's; HD's own update is a different function nobody has read.
    #[must_use]
    pub const fn runs_leach_law(&self) -> bool {
        matches!(self, Self::Brackets { leach: Some(_), .. })
    }
}

/// A title's speed and shield gauge when it authors them as `<Mode3D><Model>`
/// widgets: one `.vex` per bar fill and one for the outline drawn over both.
///
/// **Pure's dialect** (`TimeTrial_HUD.xml`, `Arcade_HUD.xml`): `Speed3D`
/// (`Data\HUD\Bar_1.vex`), `Shield3D` (`Bar_3.vex`) and `Overlay3D`
/// (`Bar_outline_1.vex`), all at `x="394" y="20"` in an orthographic block whose
/// `y` counts from the **bottom** of the screen. A fill is a flat polygon strip
/// with a texture opaque on its left half and clear on its right; the game
/// scrubs the model's animation by the bar's fraction (`Hud_UpdateSpeedBar`,
/// `0x0881eabc`: speed over 700; `0x0881f13c`: the eased shield percent over
/// 100), so the texture slides under a clamped edge and the fill grows left to
/// right. The outline carries both boxes and the `km/h` caption in one texture.
/// See `docs/formats/pure-status.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarModels {
    /// The widget whose fill follows the speed.
    pub speed: &'static str,
    /// The widget whose fill follows the shield.
    pub shield: &'static str,
    /// The widget drawn over both fills: the two outlines and the unit caption.
    pub outline: &'static str,
}

/// How a title's HUD sprites reach the screen: where their pixels come from,
/// which are up whenever the HUD is, and the one colour this build substitutes.
///
/// An axis on the same terms as [`HudLayouts`]: the corpus measurably disagrees
/// on all of these rows.
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
/// | [`Self::bar_models`] | `None` - the bars are `<Image>` sprites | Pure: three `<Mode3D>` models; HD `None` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudArt {
    /// The extension a layout's `src=` reference takes to become an archive
    /// entry name, when the disc does not ship the declared spelling.
    ///
    /// `None` when a reference *is* an entry name (both PSP titles: layouts name
    /// `.mip` files and the discs carry them). The PS2 pressing's `.pct` rewrite
    /// is a different question and stays in `oag_pulse::read_image`.
    ///
    /// HD needs it: its layouts name the **exporter's input**, not the shipped
    /// file, so read literally 192 of its 1,029 sprites dangle. See
    /// `oag_hd::hud::texture_entry` and [`replace_extension`]. Confidence 85.
    /// The declared name is always tried first.
    ///
    /// An extension, not a `fn`, because [`Title`] compares by value and a rule
    /// spelled as data is one a reader can check against the disc.
    ///
    /// [`Title`]: crate::Title
    pub texture_extension: Option<&'static str>,
    /// The sprite widgets drawn whenever this title's HUD is up.
    ///
    /// **An allow-list, deliberately**, as `oag_hud`'s label list is: a layout
    /// carries every widget every mode and state could want (warnings, opponent
    /// tags, sights), and drawing them all is a picture the original never shows.
    /// A widget not named is not drawn, so "no value" and "not wired up" stay
    /// distinguishable. A name absent from a layout is not found, so one list
    /// serves all of a title's modes.
    pub always_on: &'static [&'static str],
    /// Whether this title's HUD is **raster art authored for a small screen**,
    /// which makes a stretch filter (`oag_display`'s `HudScale`) worth offering:
    /// Pulse's 256x256 sheet and 25 px face on a 480x272 PSP or 640x448 PS2
    /// panel, Pure's the same. `false` for HD, 2048 and Omega (sheets at 4x,
    /// faces authored for a 1080p grid). Census of each title's sheet and grid:
    /// `docs/ui/hud-skins.md`.
    pub raster: bool,
    /// Whether a **right- or centre-aligned string ends on its last glyph's
    /// ink**, not its advance: the alignment width is the advances less that
    /// glyph's trailing bearing (advance less cell width).
    ///
    /// HD **measured**, `docs/ui/hud.md`, "HD HUD text: ink-aligned ends and
    /// half-unit glyph quads". Omega inherits HD's HUD engine and font,
    /// unmeasured. Pulse and Pure keep a narrower rule (right-aligned only, on
    /// `Self::raster`); 2048 is unmeasured and `false`.
    pub ink_aligned_text: bool,
    /// Whether a glyph's quad is **its cell grown by half a grid unit on each
    /// side, with the uv grown by half a texel**, whatever the text scale,
    /// instead of by the face's authored `borderExtendPixels` at that scale.
    ///
    /// HD **measured**, same section: the glyph is magnified 1.5 % past its
    /// scale. Omega inherits HD's HUD engine, unmeasured. Pulse's PSP path does
    /// the same on its own flag (`Self::raster`); 2048 is unmeasured.
    pub half_unit_glyph_quads: bool,
    /// How this title authors its lock-on reticle. See [`Sights`].
    ///
    /// Not part of [`Self::always_on`]: sights are up only while a locking
    /// weapon is held and something is lockable, a runtime question that list
    /// cannot express.
    pub sights: &'static Sights,
    /// The layout constant substituted for the pickup backdrop's own colour, or
    /// `None` to draw it as the layout authors it.
    ///
    /// Pulse needs it (measured): its `PickupBackground` samples a **filled**
    /// hexagon of alpha 255, authored in the same opaque white as the icon on it,
    /// so drawn as authored the icon is invisible. `oag_hud` carries the
    /// reasoning. HD authors a hexagon **outline** that a captured race frame
    /// shows in its authored grey with the picture showing through, so it needs
    /// no substitution.
    pub pickup_backdrop_colour: Option<&'static str>,
    /// A per-weapon backdrop colour overriding [`Self::pickup_backdrop_colour`]
    /// for the slots this title has measured, or `None` for a title that has
    /// measured none.
    ///
    /// Indexed in `oag_tables::weapons::Weapon::ALL`'s order - `[Rocket,
    /// Missile, Quake, Cannon, Turbo, Shield, Autopilot, Plasma, Bomb, Mine,
    /// LeachBeam, Repulser, Shuriken]` - rather than carrying that type: the
    /// vocabulary lives here, `Weapon` and the index stay in `oag_hud`. A slot's
    /// own `None` draws [`Self::pickup_backdrop_colour`].
    ///
    /// The runtime writer that tints `PickupBackground` is unrecovered; the
    /// *picture* is not, see `oag_pulse::hud::PICKUP_COLOURS` for the confidence
    /// split between the three categories.
    ///
    /// Values are `0xAARRGGBB`, the convention of every HUD layout colour.
    pub pickup_colours: Option<[Option<u32>; 14]>,
    /// How [`Self::pickup_colours`]' backdrop word fades in on a grant, holds,
    /// and fades out after use, or `None` for a title that draws no per-weapon
    /// backdrop (Pure, 2048). See [`PickupFade`].
    pub pickup_fade: Option<PickupFade>,
    /// The `<Mode3D><Model>` widget that draws a weapon's own icon, or `None` for
    /// a title whose icons are `<Image>` sprites (or unmeasured).
    ///
    /// **An axis in the [`Sights`] sense, not a second [`Self::pickup_colours`].**
    /// Pulse's icons are `<Image>` sprites named `<Type>Icon` with authored-white
    /// art that needs [`Self::pickup_colours`]'s substitution. Pure's are
    /// `<Mode3D><Model>`s named `<TYPE>_icon` that already carry their final
    /// `colour` in the XML. A title with both kinds has not been measured.
    ///
    /// Indexed in `Weapon::ALL`'s order, as [`Self::pickup_colours`] is. A slot's
    /// `None` means the disc authors no icon for that weapon (measured absent,
    /// not unmeasured). Pure has ten of thirteen: no `Cannon`, `LeachBeam`,
    /// `Repulser` or `Shuriken` icon exists on its disc (`Shuriken` is a weapon
    /// `oag_gameplay::pickup::IMPLEMENTED` hands out that predates Pure's
    /// roster, so a Pure race can hold a weapon its HUD has no icon for).
    /// `Quake`'s slot is filled because the disc authors the widget.
    pub pickup_icon_models: Option<[Option<&'static str>; 14]>,
    /// The `<Mode3D><Model>` widget the icon in [`Self::pickup_icon_models`] sits
    /// on, or `None`.
    ///
    /// That dialect's equivalent of `PickupBackground`: Pure's `weapon_icon_grid`,
    /// at the same placeholder position as every icon and with no `colour` of its
    /// own, so it needs no substitution. Moot whenever
    /// [`Self::pickup_icon_models`] is `None`.
    pub pickup_icon_backdrop_model: Option<&'static str>,
    /// The speed and shield gauge when drawn as three `<Mode3D><Model>` widgets
    /// instead of `<Image>` bars, or `None`. See [`BarModels`].
    pub bar_models: Option<BarModels>,
    /// The four captions the Time Trial clock takes when the title races a medal
    /// ladder, as `[bronze, silver, gold, record]` string keys, or `None` for a
    /// title that counts to `RECORD` or a campaign cell's own target (Pulse, HD).
    /// Pure's, read off `0x08a46428`-`0x08a46458`: `HUD_Bronze time` and its
    /// three siblings. The stored best's standing on the circuit's gold/silver/
    /// bronze figures picks the caption and the target:
    /// `docs/ghidra/functions/psp-pure-usa/hud-time-ladder.md`.
    pub time_ladder_captions: Option<[&'static str; 4]>,
    /// A per-weapon source-rectangle rewrite for a title whose pickup slot is
    /// *one* icon widget rather than a named widget per weapon, or `None`.
    ///
    /// **2048's dialect.** `HUD_pickups.xml`'s `PickupIcon` is the one widget
    /// every weapon draws through; the runtime rewrites its source rectangle.
    /// Confirmed from `Hud_UpdatePickupIcon`
    /// (`docs/ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md`): the
    /// XML authors no per-weapon anything to select between.
    ///
    /// `[U, V, W, H]` in atlas pixels, `u16` because the executable's table is raw
    /// integers (`oag_hud::Sprite::uv` converts on use), indexed in
    /// `oag_tables::weapons::Weapon::ALL`'s order as [`Self::pickup_colours`] is.
    /// A slot's `None` is a weapon the UV table has no entry for
    /// (`Repulser`/`Shuriken`, unimplemented on 2048 per
    /// `docs/gameplay/pickups.md`); every weapon 2048 grants has an entry.
    pub pickup_icon_uv: Option<[Option<[u16; 4]>; 14]>,
    /// What this title calls each rung of its Zone escalation ladder, or `None`
    /// if not read. See [`ZoneSpeedClasses`]. The *names* only: which rung a zone
    /// is on is [`crate::RaceDefaults::zone_stages`], kept separate because the
    /// one title with both recovered them from different places.
    pub zone_speed_classes: Option<&'static ZoneSpeedClasses>,
    /// Whether `ShieldBarText` carries a `%` after its digits.
    ///
    /// Pulse's reference frame reads `100%`. Three HD/Fury frames of the running
    /// original (`data/reference/hd-capture/talons-matched/{00,01,03}.png`: full
    /// shield, 529 km/h mid-race, 98% after a hit) read a bare number, so this is
    /// `false` for HD, not unmeasured. Neither disc authors a `%` companion the
    /// way `SpeedBarTextKMH` is for `SpeedBarText`, so it is a title fact, not
    /// readable off the layout. `true` (Pulse's reading) for every title not yet
    /// checked, which is all but HD.
    ///
    /// HD's is also read off its executable: `Hud_UpdateShieldReadout` builds the
    /// digits by hand and appends nothing
    /// (`docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md`).
    pub shield_percent: bool,
    /// Whether the `SubWeapon` text widget - the line under the held pickup's
    /// icon, where the Cannon's rounds left and the Autopilot's seconds left
    /// print - is drawn.
    ///
    /// **Every title's executable writes it, in the same function**: Pulse's
    /// `PlayerStatus_Update` (`0x0883b3b8`), HD's `0x000cb928`, Omega's
    /// `FUN_012e5470`, 2048's `FUN_811b1ee2` and, Autopilot half only, Pure's
    /// `0x08922340` (`docs/ghidra/functions/psp-pulse-usa/sub-weapon.md`). So the
    /// per-title question is where the layout puts the widget. `true` where it
    /// sits in the held icon's cluster: Pulse and Pure under the hexagon, HD and
    /// Omega in `PickupParent`.
    ///
    /// **2048 is `false`, unmeasured rather than absent.** Its `SubWeapon` is
    /// authored in `HUD_pickups.xml`'s top-centre cluster with the grant
    /// announcement, while the carried icon sits bottom-left in
    /// `HUD_damage_indicator.xml`; whether that cluster is up outside the
    /// announcement is not read.
    pub sub_weapon_line: bool,
    /// The speed, in km/h, at which this title's `SpeedBar` reads full.
    ///
    /// **HD and Omega: 700, read off the executable.** `Hud_UpdateSpeedReadout`
    /// (`0x00086e88`) multiplies the speed by `0x008a7708`, `0x3abb3ee7` = 1/700.
    /// The reference frames (`talons-matched/00.png`, `01.png`) fit it better than
    /// 600: at 529 km/h the fill ends at 82% where 700 predicts 76% and 600
    /// predicts 88%; at 74 km/h, 6% against 11% and 12%.
    ///
    /// **Pulse: 700, read off the executable** (`Hud_UpdateSpeedBar`,
    /// `0x0881a3e0`, `speed / 700.0`; live reads of the fill's width matched on 59
    /// of 60 ticks).
    ///
    /// **Pure: 700, read off the executable** (`0x0881eabc`, `div.s` by
    /// `0x442f0000`; `docs/ghidra/functions/psp-pure-usa/hud-speed-gauge.md`).
    ///
    /// **2048: 600, chosen, not measured.** Nothing established what sets that
    /// bar's maximum. The exhaust ramp saturates at 600 km/h
    /// (`oag_fx::exhaust::RAMP_FLOOR_KMH` plus `RAMP_SPAN_KMH`) but that is a
    /// visual effect, not a bar maximum. A reference frame at a known speed would
    /// settle it.
    pub speed_full_kmh: u32,
    /// The font role this title's language plugins name for the HUD's value face
    /// (lap and position fractions, speed, time), as
    /// `oag_ui::language::Language::font` resolves it.
    ///
    /// Pulse, Pure and HD's `Data\Plugins\*\Definition.xml` carry
    /// `<Font><Values name="HUD" ...>`; 2048 disagrees: its
    /// `english/Definition.xml` has no `HUD` slot and names `2048HUD`
    /// (`Data\XML\2048_hud\font\2048_hud.fnt`), confidence 90 -
    /// `docs/formats/2048-frontend.md`, "The language plugins carry a HUD font
    /// role too".
    ///
    /// A per-title role rather than a shared literal because a shared `HUD`
    /// drew 2048's HUD in the 5x7 fallback, and because two of 2048's seventeen
    /// plugins (`korean`, `traditionalchinese`) carry a leftover `HUD`/`HUDSmall`
    /// role pointing at files the title does not ship
    /// (`Data\FE\Fonts\PulseHud.fnt`/`koreanHudSmall.fnt`), which a first-match
    /// search across the loaded plugins could pick up. Asking for `2048HUD`
    /// avoids both.
    ///
    /// The value lives in each title crate's own `ART` constant; this field is
    /// the axis.
    pub hud_font_role: &'static str,
    /// The font role this title's language plugins name for the HUD's caption
    /// face (the labels beside [`Self::hud_font_role`]'s values), or `None` when
    /// they name no distinct role. `Some("HUDSmall")` for Pulse, Pure and HD, as
    /// [`Self::hud_font_role`] measured for `"HUD"`.
    ///
    /// **2048 is `None`, measured at two levels.** None of its seventeen language
    /// plugins carries a second `<Font>` slot for captions, and composing the
    /// seven roots of the played skin (`oag_2048::hud::skins::PLAYED`) finds zero
    /// `font="HUDSmall"` widgets
    /// (`crates/hud/tests/vita_2048_hud_ground_truth.rs`,
    /// `the_played_skins_layouts_author_no_hudsmall_widget_at_all`). A
    /// `font="HUDSmall"` role exists in the archive (261 widgets) but only in
    /// the three unplayed skins (`wo3_hud`, `2097_hud`, the bare root).
    ///
    /// The visible size split is a scale, not a second file: a Vita3K race frame
    /// (`docs/formats/2048-hud.md`, e.g. `21-race2-start.png`) shows `LAP`,
    /// `TOTAL`, `CURRENT`, `POS`, `XP` smaller than the values beside them, and
    /// the composed layout explains it: `LapTxt` is authored at `scale=0.6`
    /// beside `Laps` at `scale=1.0`, both `font="HUD"`; `RaceXPTxt`/`RaceXP` are
    /// both `0.6`. `oag_hud::widget::Label::scale` carries the size on one atlas.
    ///
    /// So `oag_raceplay::hud::hud_font` falling back to [`Self::hud_font_role`]
    /// when this is `None` is the reading the played skin's layouts already
    /// assume.
    pub hud_small_font_role: Option<&'static str>,
    /// Whether `TotalTime` and `TotalTimeTxt` are up only in the timed modes,
    /// Time Trial and Speed Lap, and hidden in every other.
    ///
    /// **Pulse's rule, read off the executable and seen on a running original.**
    /// `PlayerStatus_Update` (`0x0883b3b8`) writes the clock's target field to
    /// `-1` every tick and overwrites it only when `g_game_mode` is Time Trial
    /// (5), Multiplayer Time Trial (`0x11`), Speed Lap (10) or Free Play (7);
    /// `Hud_UpdateTimeCluster` (`0x0881c9d0`) clears the visible bit on both
    /// widgets while it reads `-1`. A PPSSPP Eliminator frame (`g_game_mode` 8)
    /// shows no `TOTAL`, and both widgets' flag words read live with that bit
    /// clear.
    /// See `docs/ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md`.
    ///
    /// `false` for every other title: 2048's frame shows `TOTAL` and `POS`
    /// together in a race with a field (`docs/formats/2048-hud.md`), so the rule
    /// is not engine-wide.
    pub total_time_timed_modes_only: bool,
    /// Whether `TotalTime` reads in hundredths (`0.00.00`) rather than tenths
    /// (`0.00.0`).
    ///
    /// **Measured on 2048**: Vita frames read `TOTAL 0.29.76` over `CURRENT
    /// 0.29.7` in a native race and `TOTAL 0.00.00` in the HD Campaign's Vineta K
    /// grid. `false` for every title not checked (`docs/formats/2048-hud.md`).
    pub total_time_hundredths: bool,
    /// Whether the Eliminator's `PosTag0`-`PosTag7` rows and `KillsText` header
    /// are drawn as `"<team> <kills>"` and `KILLS (n)`. **Pulse only**: read off
    /// its executable (`Hud_UpdateKillColumn`) and a live frame. HD's Eliminator
    /// draws `DATA00`'s layout, which has no `KillsText` and whose rows print a
    /// score, `NAME ( score )`, not kills (`docs/ui/hud.md`, "HD's Eliminator race
    /// line"), so it stays off there.
    pub kill_column: bool,
    /// Whether the four `Info1`-`Info4` widgets are the HUD's message lines
    /// (`oag_hud::messages`). Pulse, Pure and HD author the same four widgets in
    /// every race layout (`InfoTextParent` on HD) and each executable's message
    /// routine drains a flag word into them; the slot law (0.8 s gate, four
    /// seconds of life, `sin(pi * (1 - (t/life)^6))` alpha) reads the same on all
    /// three. What differs is [`Self::lap_hud`]. 2048 and Omega are off: their
    /// layouts are not censused.
    pub message_slots: bool,
    /// What the lap-crossing code raises through those lines and the split gap,
    /// read from this title's own executable. `None` where nobody has read it: no
    /// lap line and no gap, rather than Pulse's answer borrowed. See
    /// [`crate::lap_hud`].
    pub lap_hud: Option<crate::lap_hud::LapHud>,
    /// What this title's per-tick HUD update writes over its layout: the shield
    /// readout's runtime colours and fill, and which segments of the lap and
    /// place arcs are up. See [`RuntimeHud`].
    ///
    /// `None` for every title but HD, whose `Hud_UpdateShieldReadout`,
    /// `Hud_UpdateLapCounter` and `Hud_UpdatePositionCounter` are what
    /// `oag_hd::hud::RUNTIME` carries. Pulse's shield rule has a different shape
    /// (`oag_hud::Readout::shield_forced_red`).
    pub runtime: Option<&'static RuntimeHud>,
    /// The lap counter's lap-one shift: `None` where unmeasured.
    pub lap_one_shift: Option<LapOneShift>,
    /// The alpha the original multiplies into its lit HUD text and sprite quads: `None` draws
    /// them at the authored alpha. See [`LitAlpha`].
    pub lit_alpha: Option<crate::pre_race::Sourced<LitAlpha>>,
    /// The drop shadow the original draws under every text string: `None` draws none. See
    /// [`TextShadow`].
    pub text_shadow: Option<crate::pre_race::Sourced<TextShadow>>,
    /// The alpha, over 255, the original's HUD text outline pass draws at where it ignores the
    /// layout's `BorderColor` alpha: `None` draws the layout's own. See [`TextOutlineAlpha`].
    pub text_outline_alpha: Option<crate::pre_race::Sourced<TextOutlineAlpha>>,
    /// How the speed and shield bars' fill, marker and label follow the bar's
    /// tip: `None` where unmeasured, which leaves the plain left-anchored crop.
    pub bar_tip: Option<BarTip>,
    /// The boost's multi-copy smear and the glow stamp under the speed bar: `None` where
    /// unmeasured, which draws each widget once and stamps nothing. See [`BoostSmear`].
    pub boost_smear: Option<BoostSmear>,
    /// The whole HUD sliding with the drawn lean: `None` where unmeasured, which leaves the
    /// HUD fixed on the screen. See [`HudSway`].
    pub sway: Option<crate::pre_race::Sourced<HudSway>>,
    /// The player's `Safe Area Setting` (the SCREEN SIZE row), which draws the whole HUD and the
    /// front end smaller about the screen's centre. HD's is the disc's own; every other title
    /// carries [`SafeArea::SHARED`], and `None` draws 1:1 with no row. See [`SafeArea`].
    pub safe_area: Option<crate::pre_race::Sourced<SafeArea>>,
    /// The player's `Dynamic HUD` option, which can switch the whole HUD off: `None` where the
    /// title's options have no such list. See [`DynamicHudList`].
    pub dynamic_hud: Option<crate::pre_race::Sourced<DynamicHudList>>,
    /// Pilot Assist's HUD indicator: the widget names, and where a layout that
    /// does not author them gets them from. `None` without an indicator.
    ///
    /// **Its own field, not part of [`RuntimeHud`]**: a title with an indicator
    /// and none of HD's measured rules (Pulse, Pure) must not be handed them.
    pub assist: Option<AssistIndicator>,
    /// How the 3 s `3`, `2`, `1`, `GO` countdown a split race runs on resume is
    /// drawn. See [`ResumeCountdown`].
    pub resume_countdown: ResumeCountdown,
}

/// How a resume countdown reaches the screen on a title.
///
/// **Chosen, not measured**: the original is single-player and has no resume
/// countdown, so this says only whether the title *has screen-space art* for
/// one. Pulse's layouts author a `Cockpit321Go` widget and
/// `Cockpit_321GO.vex`; Pure, HD, 2048 and Omega have none in a layout this
/// build reads (Pure authors `Ready_GO.vex`, HD's `hud_ready_go.xml` is
/// disabled), so they draw the count as text in the title's `HUD` face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeCountdown {
    /// Play the title's authored overlay (`oag_game::hud_countdown`). If it did
    /// not load, nothing is drawn and the loader says so: never a font stand-in.
    Authored,
    /// Draw `3`, `2`, `1` and the `GO` string in the HUD value face.
    Text,
}

/// The widgets a title's HUD update recolours, crops or shows at runtime, and
/// the values it does it with; the layout authors none of them.
///
/// The values are the executable's; the rules that apply them live in
/// `oag_hud::runtime`.
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

/// The alpha byte the original's HUD writes into the vertex colour of its lit
/// text and sprite quads: the authored alpha times `byte / 255`.
///
/// HD **measured** (`docs/formats/hd-hud.md`, "The 0.8 is the vertex colour's
/// alpha byte"): the lap `1` reads `ff ff 00 cc` where the layout authors alpha
/// `ff`, the Record time `0x90` where it authors `0xB4`. The black
/// outline/shadow (`0x40`), the speed bar's gradient and the shield bar (`0xFF`)
/// are not scaled. Stored as the byte because [`HudArt`] compares by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LitAlpha {
    /// The multiplier's numerator over 255.
    pub byte: u8,
}

impl LitAlpha {
    /// `alpha` (0..1) as the original's byte arithmetic leaves it: `round(a * 255) * byte / 255`.
    #[must_use]
    pub fn apply(self, alpha: f32) -> f32 {
        let authored = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
        (authored * u32::from(self.byte) / 255) as f32 / 255.0
    }
}

/// The alpha of an outlined HUD font's border pass, replacing the layout's `BorderColor` alpha.
///
/// Pure **measured** (`docs/formats/pure-status.md`, "Pure's HUD text outline"): a PPSSPP GE
/// list of a Time Trial frame draws every HUD string as two `RECTANGLES` prims over one 4-bit
/// font texture, vertex colour white, the first under a black CLUT whose alpha is the `.fnt`
/// palette's alpha times `0.40` on all 11 strings of the frame (`0x66`), the second under a white
/// CLUT carrying the palette's grey as alpha. The layouts author `0x40000000` and `0x30000000`;
/// neither reaches the first CLUT. Stored as the byte because [`HudArt`] compares by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextOutlineAlpha {
    /// The outline's alpha over 255.
    pub byte: u8,
}

impl TextOutlineAlpha {
    /// `byte / 255`.
    #[must_use]
    pub fn alpha(self) -> f32 {
        f32::from(self.byte) / 255.0
    }
}

/// The black copy of a text string the original draws under it, offset up and
/// left.
///
/// HD **measured** (`docs/formats/hd-hud.md`, the RPCS3 draw capture): every
/// glyph quad of the lit HUD text has a twin drawn before it with the same size
/// and uv, `-1` HUD unit on both axes, vertex colour `00 00 00 40`
/// (`HudBGColour`, **not** scaled by [`LitAlpha`]). The HD font atlas is plain
/// coverage, so the twin is the glyph's own shape, not a halo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextShadow {
    /// Added to the string's `x`, in layout units.
    pub dx: i32,
    /// Added to the string's `y`, in layout units.
    pub dy: i32,
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
/// HD's `Hud_Update` (`0x0009e704`-`0x0009e754`) writes one translation into the
/// top-level `"HUD"` and `"InGame"` screens it looked up by name at load, and
/// every layout's root is `<Screen name="HUD">`, so every widget the race draws
/// moves with it. On a running original a held full stick moved the whole HUD
/// by 13.3 px at 1280x720, all five regions alike, and each plateau of the
/// lean's follower (`ship+0x6a58`) did the same at its own size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSway {
    /// Layout units the HUD moves along **x** per unit of the drawn lean's
    /// follower (`oag_physics::ShipState::steer_lean_follower`): HD's `-15.0`
    /// literal, so a stick to the right slides the HUD left.
    pub x_per_lean: i16,
}

impl HudArt {
    /// The scale this title's `Safe Area Setting` draws its HUD and its front end at, for the
    /// player's `percent` (`None`: the title's default): `1.0` where the title has no such
    /// option. See [`SafeArea`].
    #[must_use]
    pub fn safe_area_scale(&self, percent: Option<u8>) -> f32 {
        self.safe_area.map_or(1.0, |area| area.value.scale(percent))
    }

    /// The percent the SCREEN SIZE row shows for the player's `percent` (`None`: the title's
    /// default): held to the slider's range and step. `100` where the title has no option.
    #[must_use]
    pub fn safe_area_percent(&self, percent: Option<u8>) -> u8 {
        self.safe_area
            .map_or(100, |area| area.value.snapped(percent))
    }
}

/// A title's `Safe Area Setting`: a slider in percent whose value is how much of
/// the screen the HUD uses, the whole HUD drawn at `percent / 100` about the
/// screen's centre.
///
/// HD authors it in its Game Options (`additional_definition.xml`, `Slider
/// name="Safe Area Setting" default="90%"`, `minSlide="80" maxSlide="100"
/// incSlide="2"`). The scale reading is inferred from one live read: the save
/// the HD comparison pairs were restored from holds `96.0` in the option's global
/// (`+0x104` of the object behind the TOC slot at `0x8a5bb4`), and a correlation
/// of that frame's HUD against ours gives `0.964`-`0.968`. Poking `90.0` into
/// the global did not move the HUD, so the consumer is unread.
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
    /// The slider a title gets when its original authors none, so the player can still pull
    /// the screen in off a rounded corner or an overscanned edge: HD's range and step, and a
    /// default of `100` because these titles draw 1:1 and look unchanged until it is moved.
    pub const SHARED: Self = Self {
        default_percent: 100,
        min_percent: 80,
        max_percent: 100,
        step_percent: 2,
    };

    /// A player's `percent` (`None` for the title's default) clamped to the slider's range and
    /// snapped to its step, as a slider would hold it.
    #[must_use]
    pub fn snapped(self, percent: Option<u8>) -> u8 {
        let value = percent
            .unwrap_or(self.default_percent)
            .clamp(self.min_percent, self.max_percent);
        let step = self.step_percent.max(1);
        self.min_percent + (value - self.min_percent) / step * step
    }

    /// The HUD's scale for a player's `percent` (`None` for the title's default).
    #[must_use]
    pub fn scale(self, percent: Option<u8>) -> f32 {
        f32::from(self.snapped(percent)) / 100.0
    }

    /// Every value the slider offers, lowest first.
    pub fn values(self) -> impl Iterator<Item = u8> {
        (self.min_percent..=self.max_percent).step_by(usize::from(self.step_percent.max(1)))
    }
}

/// A title's `Dynamic HUD` option: a list on its Game Options page whose
/// `FE_OFF` entry draws no HUD at all.
///
/// Pulse authors it (`Additional_Definition.xml`, `List name="Dynamic HUD"
/// global="Dynamic HUD" Default="FE_DEFAULT"`) and `Hud_Update` reads it:
/// `FE_OFF` clears the HUD's shown byte (`+0x121`), which the song display and
/// the HUD's own gate both read. Poking it to zero in a running race removed
/// every HUD element, `docs/ghidra/functions/psp-pulse-usa/now-playing.md`.
/// `FE_CONCAVE` and `FE_FLAT` set bit 1 of `+0xd0` on the two panel groups,
/// which is not decoded, so this build offers `FE_DEFAULT` and `FE_OFF` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicHudList {
    /// The list's entries as the disc spells them (string ids), in its order; the first is the
    /// list's default.
    pub entries: &'static [&'static str],
    /// The entry that draws no HUD.
    pub off: &'static str,
}

/// The speed and shield bars' fill law: the fill's tip is where the marker and
/// the value label sit.
///
/// Pulse's `FUN_0881a3e0` (speed bar) and `Hud_UpdateEnergyBar` (`0x0881c638`)
/// run one law: the value as a fraction `f` of full, then `f' = floor + span *
/// f`, the fill's pixel width `floor(f' * background width)`, its texture window
/// the **right** `f'` of the art (so the pointed end is always the tip), and the
/// marker and label at `authored x + width + tip_dx`. Only the speed bar reads a
/// speed under `idle_kmh` as zero.
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

/// What a speed pad's boost does to the HUD (Pulse PSP, `Hud_Update`
/// `0x0881bf50`).
///
/// While the exhaust flare's boost timer runs, the game counts `n = trunc(peak *
/// w)`, `w` a triangle of `timer / seconds` (`0` at both ends, `1` in the
/// middle), and draws every widget `n` times at `alpha / n`, each shifted
/// further from the screen's centre along the line through it; the speed bar's
/// fill also stamps the bloom's glow mask with `n / peak * 255`.
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

/// The eased fade a held pickup's backdrop and icon play: `Hud_UpdatePickupWidgets`
/// on Pulse (`0x0881cdc4`), `Hud_UpdatePickupBar` on HD (`0x0008df30`).
///
/// The law (easing shape, `0.5` cap, `0.4` icon gate, `0.1` hold) lives in
/// `oag_hud::pickup_fade`; this is the per-title part. Provenance is on each
/// `ART` and in `docs/ghidra/functions/psp-pulse-usa/pickup-colour.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickupFade {
    /// The timer's rate in hundredths of a unit per second (`u16` so the art stays
    /// `Eq`): the timer gains `dt * rate / 100` per update. Pulse `120`, HD `150`.
    pub rate_percent: u16,
    /// Whether the backdrop also folds out in width and height as it grows (Pulse's
    /// `+0x9c`/`+0xa0` pair). HD's matching store is unread, so HD and Omega keep it `false`.
    pub size_pulse: bool,
}

/// The dim the pickup bar is drawn in while it has nothing to show: the
/// executable resets its widgets to white each tick and writes this colour over
/// the ones with no content (HD's `0x0008df30`, `0x5affffff`, alpha 0.35).
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
/// **`ids[n]` is the name of stage `n`**, the index
/// `oag_raceplay::zone_grade::ZoneGrade` shows and `--zone-stage` selects, not a
/// zone number. `None` is a rung with no name of its own, like the bottom of
/// HD's ladder.
///
/// HD's fifteen rungs are not one per zone: a reference frame showing
/// `SUB-VENOM` at zone 1 and `VENOM` at zone 2 is equally consistent with a
/// first band one zone wide, the shape `oag_2048::race::ZONE_STAGES` has (`0`-`1`,
/// then `2`-`8`), and play shows not every zone is a class bump. What turns a
/// zone counter into a rung is recovered on 2048, unrecovered on HD, and asked
/// for through [`crate::RaceDefaults::zone_stages`], not guessed here.
///
/// Only the **names** are carried: recoverable from the disc alone, and the same
/// list whatever drives the index. They are localised, so the caller resolves
/// each id against the language plugin's table, as an `idstring` caption is. A
/// stage past the end takes the last rung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneSpeedClasses {
    /// One entry per rung, in stage order from zero. See the type's docs.
    pub ids: &'static [Option<&'static str>],
    /// Where each rung begins, as a zone number, for a title whose HUD names the
    /// class **off the zone** rather than the grade's stage.
    ///
    /// Pulse: `FUN_0881ad10` walks the zone counter down a ladder of thresholds
    /// with no stage in between (Pulse has no grade). Rung `n` begins at
    /// `rung_zones[n]`, ascending, the same length as [`Self::ids`]. `None` keeps
    /// the stage-keyed reading HD uses.
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

    /// The bottom rung has no name: HD's `0 Start` has no speed class beside it.
    #[test]
    fn an_unnamed_rung_reads_as_nothing_rather_than_as_the_next_one() {
        assert_eq!(LADDER.id_for(0), None);
        assert_eq!(LADDER.id_for(1), Some("A"));
    }

    /// Past the top, the top holds.
    #[test]
    fn a_stage_past_the_end_takes_the_last_rung() {
        assert_eq!(LADDER.id_for(2), Some("B"));
        assert_eq!(LADDER.id_for(99), Some("B"));
    }

    /// An empty ladder names nothing rather than indexing out of bounds.
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

    #[test]
    fn the_lit_alpha_byte_reproduces_the_two_captured_points() {
        let lit = LitAlpha { byte: 0xCC };
        assert_eq!((lit.apply(1.0) * 255.0).round() as u32, 0xCC);
        assert_eq!(
            (lit.apply(f32::from(0xB4_u8) / 255.0) * 255.0).round() as u32,
            0x90
        );
        assert_eq!(
            (lit.apply(f32::from(0x5A_u8) / 255.0) * 255.0).round() as u32,
            0x48
        );
        assert_eq!(lit.apply(0.0), 0.0);
    }
}
