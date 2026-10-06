//! Wipeout 2048's in-race HUD layout entries.
//!
//! Entry names in the [ADR-0022] sense - *which* files this package ships -
//! the same division [`oag_hd::hud`] draws. Everything that reads them is
//! `oag_hud`.
//!
//! # Four skin sets, and `2048_hud` is the one a race actually plays
//!
//! The archive holds 26 `*_HUD.xml` documents in four groups: eight directly
//! under `Data\XML\` (the same eight [`oag_hd::hud::ROOTS`] names, Detonator
//! and Duel included, which this build's [`oag_race::Mode`] has no rules for
//! and never reads), plus one skin set each under `2048_hud\`, `2097_hud\` and
//! `wo3_hud\`. All 26 compose with nothing missing and nothing skipped -
//! `crates/game/tests/vita_2048_hud_ground_truth.rs`.
//!
//! **Which skin a race reads is no longer a guess.** Two of this title's own
//! race-manager constructors hard-code a `2048_hud\` path in their HUD
//! construction - `SpArcadeRaceManager_Construct` and three further managers
//! for Elimination, SpeedLap/TimeTrial and Zone - and searching the executable
//! for `wo3_hud`/`2097_hud` finds **zero** references anywhere. Full evidence
//! in
//! [race-hud-selection.md](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/vita-2048-eu-v104/race-hud-selection.md).
//! The bare root set is real but is what `DemoRaceManager_Construct` reads -
//! the attract-mode demo, not a race a player starts - so [`LAYOUTS`] points
//! at [`skins::PLAYED`] rather than the bare root [`layouts`] this module
//! still carries for [`skins::DEMO`] and for completeness against
//! [`oag_hd::hud`]'s own shape.
//!
//! **`2097_hud` and `wo3_hud` are present and unreachable from any code path
//! found so far**, not proven dead: `search_strings` only walks Ghidra's
//! auto-detected string table, which is demonstrably incomplete for this
//! binary (see the caveat in race-hud-selection.md). Recorded and not acted
//! on.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`oag_hd::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/hud.rs

/// The layout each mode this engine runs reads its HUD from, as
/// [`oag_title::Title::hud`] carries it.
///
/// [`skins::PLAYED`], not the bare root - see the module docs. **Speed lap and
/// time trial are two distinct files here**, unlike either PSP disc, which is
/// itself evidence rather than a default: `FUN_812c1722` (documented in
/// race-hud-selection.md) branches on a runtime flag between exactly these
/// two paths.
pub const LAYOUTS: &oag_title::HudLayouts = &oag_title::HudLayouts {
    arcade: skins::played::ARCADE,
    time_trial: skins::played::TIME_TRIAL,
    speed_lap: skins::played::SPEED_LAP,
    zone: skins::played::ZONE,
    elimination: skins::played::ELIMINATION,
};

/// How 2048's HUD sprites reach the screen, as [`oag_title::Title::hud_art`]
/// carries it.
pub const ART: &oag_title::HudArt = &oag_title::HudArt {
    // **Measured.** The Vita's texture container is `.gxt`; a layout's `src`
    // is either already spelled that way (the `2048_hud` skin's own five
    // references) or carries the HD-inherited `.gtf`/`.mip`/.tga` spelling
    // the root/`wo3_hud`/`2097_hud` skins still author, unconverted. All
    // nineteen distinct references across all 26 composed layouts resolve
    // once this rule is applied -
    // `crates/game/tests/vita_2048_hud_ground_truth.rs`. Confidence 90, on the
    // same terms as [`oag_hd::hud::texture_entry`]'s own row: every reference
    // this title's HUD carries resolves under it, none needed the literal
    // spelling.
    //
    // Until 2026-08-26 this constant was `"gxt"`, missing the leading dot -
    // `oag_title::hud::replace_extension` builds `format!("{stem}{extension}")`,
    // so every 2048 HUD texture lookup silently resolved to a name like
    // `missile_reticulegxt` and never found the file. Composing was never run
    // against real data until now, which is how a bug that broke every
    // texture reference on the title went unnoticed.
    texture_extension: Some(TEXTURE_EXTENSION),
    always_on: ALWAYS_ON,
    raster: false,
    // **Concentric sprites, read off this title's own composed layouts, not
    // copied from HD.** Every one of the 26 layouts that authors a reticle at
    // all authors it as `<Image>` widgets named `MissileSightBG/Outer/Inner/
    // Middle` (seeking) and `MissileSightLockedOnLines/Middle` (locked) -
    // zero `<Mode3D><Model>` widgets appear anywhere in the composed set. The
    // names are identical to [`oag_hd::hud::ART`]'s because this is the same
    // HD-lineage HUD dialect authoring the same widget vocabulary, not an
    // assumption carried over from HD.
    //
    // The placeholder-centre idiom corroborates it independently: the root/
    // `wo3_hud`/`2097_hud` skins centre the reticle at `(-960, 540)` - HD's
    // own 1920x1080 half - while `2048_hud` centres it at `(-480, 272)`,
    // exactly the negated half of the Vita's native 960x544 screen. That
    // second number is itself part of why [`skins::PLAYED`] is read as the
    // played skin: the other three are still authored for HD's screen and
    // were never rescaled for this console.
    //
    // A `LeachBeamSight*` family sits alongside every `MissileSight*` one,
    // same shape, and **is now modelled here too, as data off the disc
    // alone** - as of 2026-09-15, `LeachBeamSightBG`/`Outer`/`Middle`/`Inner`
    // off `Data\XML\2048_hud\Texture\leach_reticule.gxt`, 88/88/64/40 pixels
    // square, centred on the same `(-480, 272)` placeholder every other sight
    // widget on this skin uses, and no `LeachBeamSight*LockedOn*` widget
    // anywhere in the composed set - the identical four-name, no-`LockedOn`
    // shape [`oag_hd::hud::ART`] carries. Confidence 90 on the widget names
    // and sizes, the same literal-XML-read terms as the row above.
    //
    // **The sight itself is not runtime-checked.** The 2026-09-16 Vita3K
    // captures (`docs/reverse-engineering/vita3k-capture.md`) held a Missile
    // and a Rocket and never had an opponent in the sight's range at a
    // sampled instant, so no frame shows any of these widgets; the row is
    // still read off `data/extracted/vita/PCSF00007` alone. Which of the
    // four draws when runs the same measured, ported reveal
    // `oag_hd::hud::ART`'s own `leach` field does - 2048 was never
    // independently disassembled for it, so the law is corroborated by the
    // shared HD/2048 lineage rather than by a second reading of this binary.
    sights: &oag_title::hud::Sights::Concentric {
        seeking: &[
            "MissileSightBG",
            "MissileSightOuter",
            "MissileSightInner",
            "MissileSightMiddle",
        ],
        locked: &["MissileSightLockedOnLines", "MissileSightLockedOnMiddle"],
        leach: Some([
            "LeachBeamSightBG",
            "LeachBeamSightOuter",
            "LeachBeamSightMiddle",
            "LeachBeamSightInner",
        ]),
    },
    // `None` is "draw what the layout authors", which is the conservative
    // answer for a title whose pickup backdrop colour nothing has measured -
    // Pulse's colour substitution is the one that would need evidence.
    pickup_backdrop_colour: None,
    // Not just unmeasured: `HUD_pickups.xml`'s `PickupBackground` authors no
    // `<Values>` at all, and the icon is one widget (`PickupIcon`) whose UV
    // rect the runtime rewrites per weapon (see `pickup_icon_uv` below)
    // rather than thirteen names `pickup_icon_name` can select between the
    // way Pulse's and HD's own dialects allow. A colour table keyed by name
    // has nothing to attach to yet - see `docs/formats/2048-hud.md`'s "What
    // is not done".
    pickup_colours: None,
    // 2048's `PickupIcon` is a single `<Image>` widget whose UV rect the
    // runtime rewrites, the same widget kind `pickup_colours`' comment above
    // names - not `<Mode3D><Model>`s, so this field is moot here too.
    pickup_icon_models: None,
    pickup_icon_backdrop_model: None,
    pickup_icon_uv: Some(PICKUP_ICON_UV),
    // `None`: 2048's Zone HUD shows a class per *band* of zones off
    // `oag_2048::race::ZONE_STAGES`, which is a different shape from the
    // per-zone ladder this row carries. See `oag_title::ZoneSpeedClasses`.
    zone_speed_classes: None,
    // **Measured `true`, 2026-09-16.** Every Vita3K race frame reads the
    // shield with a `%` - `100%` on the grid, `99%`/`95%`/`93%` after wall
    // hits, `27%` in Zone - under the silhouette at `EnergyText`'s authored
    // `(51, 509)`. See `oag_title::HudArt::shield_percent` and
    // `docs/formats/2048-hud.md`.
    shield_percent: true,
    // **Measured, and the correction this axis exists for.** 2048's language
    // plugins name no `HUD` role at all - `english/Definition.xml` carries
    // `<Font><Values name="2048HUD" ...Src="Data\XML\2048_hud\font\2048_hud.fnt">`
    // instead, confirmed across the seventeen loaded plugins. Confidence 90.
    // See `oag_title::HudArt::hud_font_role` and
    // `docs/formats/2048-frontend.md`'s "The language plugins carry a HUD
    // font role too" section.
    hud_font_role: "2048HUD",
    // **Real gap, not a default omitted.** None of the seventeen plugins
    // names a second `<Font>` slot for the caption face; `2048HUD` is the
    // whole of this title's own HUD font vocabulary. `hud_font` falls back
    // to `hud_font_role` above rather than the 5x7 glyphs - chosen, not
    // measured. See `oag_title::HudArt::hud_small_font_role`.
    hud_small_font_role: None,
    total_time_timed_modes_only: false,
    kill_column: false,
    message_slots: false,
    runtime: None,
};

/// What a layout's texture reference becomes on this title.
///
/// See [`ART`]; [`oag_title::hud::replace_extension`] is the rule that
/// applies it.
#[must_use]
pub fn texture_entry(reference: &str) -> String {
    oag_title::hud::replace_extension(reference, TEXTURE_EXTENSION)
}

/// What [`texture_entry`] replaces a reference's extension with.
pub const TEXTURE_EXTENSION: &str = ".gxt";

/// The sprite widgets 2048 draws whenever its HUD is up.
///
/// # Read off frames of the running original
///
/// **Captured 2026-09-16 on Vita3K** (`PCSF00007` v1.04 - the same EU build
/// the layouts and [`ART`] were read from, and the same one the Ghidra
/// program `/vita-2048-eu-v104` is), through the harness
/// `docs/reverse-engineering/vita3k-capture.md` records. Frames are game
/// content and stay out of the tree, under `~/.cache/oag/2048-hud/frames/`;
/// `docs/formats/2048-hud.md` names each one and what it shows. The states
/// captured: the arcade grid before the craft moves (no pickup), mid-race
/// with no pickup, mid-race holding a Missile and later a Rocket, the
/// pickup's announcement frame, a damaged shield (99% down to 92%, and 27% in
/// Zone), the time-trial grid and mid-lap, and a Zone run at zones 2-5.
///
/// The list is the **intersection** across those states, not the contents
/// of one good frame - the same rule `oag_hd::hud::ALWAYS_ON`'s "what is
/// deliberately left out" section applies. A widget is identified by
/// matching its authored source rectangle (cut out of the decoded `.gxt` by
/// `crates/game/examples/vita_2048_hud_atlas.rs`) against the frame at the
/// authored `x`/`y`:
///
/// | Widget | In the frames |
/// | --- | --- |
/// | `ThrustBarBG` | the dim two-bar speed readout, bottom right, in every arcade and time-trial frame from the grid onward. Its lit copies (`ThrustBar`, `SpeedBar0`-`4`) fill in as speed rises. |
/// | `EnergyBgFrame` | the ship-silhouette outline, bottom left, in every arcade and Zone frame. Time trial does not author it and the time-trial frames show none. |
/// | `EnergyBg` | the silhouette *inside* that outline, tinted light grey normally and red at or under a critical shield - see "`EnergyBg` tints" below. |
/// | `ZoneCounterBG` | the dim arc of ten dashes round the zone number, bottom left - authored at alpha 0.31 and read at that in the Zone frames; `ZoneLight0`-`9` are the bright copies lit one per zone. |
///
/// Confidence **90** on the first two (dozens of frames, both modes, both
/// clearly the authored art at the authored rectangle), **85** on `EnergyBg`
/// (the same rect and source as `EnergyBar`/`EnergyBarDelay`; the name is
/// assigned by elimination - the white fill is the one that shrinks with the
/// shield and the red one flashes on a hit, which leaves this one for the
/// translucent constant), **85** on `ZoneCounterBG` (one Zone run, four
/// frames, authored alpha matches what is seen).
///
/// # `EnergyBar`: wired, in its authored colour - 2026-09-20
///
/// `EnergyBar` shares `EnergyBg`'s rect and source rectangle exactly
/// (`(27, 372)` 50x110 off `(270, 16)`), differing only in colour, so it is
/// now in this list too and cropped **vertically from the bottom** by
/// `oag_hud::draw::crop_vertically`, keyed to
/// `Readout::shield_fraction()` - the vertical counterpart to
/// `crop_horizontally`, Pulse's `ShieldBar` model. `36-w-5.png` (95%) and
/// `68-zone-5.png` (27%) both show the fill's top edge tracking the
/// percentage with the bottom edge fixed, which is the crop direction this
/// implements; see `crop_vertically`'s own doc comment for the confidence
/// (80, same terms as `crop_horizontally`'s).
///
/// **Chosen, not measured: `EnergyBar`'s own runtime colour.** The layout
/// authors it green at half alpha (`[0.19, 1.0, 0.19, 0.50]`, read straight
/// off the composed layout), but the captured frames show the *live* fill as
/// white (`36-w-5.png`). Nothing decompiled writes a colour onto `EnergyBar`
/// itself (see "`EnergyBg` tints" below for the widget that *is* decompiled),
/// so a tint recalled from the frames rather than sourced from the disc or
/// the executable would be an invented colour - the fill draws at its
/// authored green-at-half-alpha, cropped correctly and coloured wrong. See
/// `docs/formats/2048-hud.md`'s dated section.
///
/// # `EnergyBg` tints, and `EnergyBarDelay` is wired - 2026-09-25
///
/// **Decompiled `Hud_UpdateEnergyBar` in full**
/// (`docs/ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md`,
/// confidence 85) rather than reading only its "confirms the crop" summary,
/// and it settled two things the frame-based pass above could not:
///
/// - **`EnergyBg`, not `EnergyBar`, is what turns red.** The function calls
///   a fixed-colour draw on `EnergyBg` every tick: opaque light grey
///   (`0xffa7a5a7`) normally, opaque red (`0xffff0000`) at or under 20%
///   shield or during the shared post-hit flash window
///   (`Readout::shield_flashing`) - wired in
///   `oag_hud::dialect_2048::energy_bg_tint`, resolving the earlier
///   "our silhouette is opaque white" gap against the running original.
/// - **`EnergyBarDelay` is a lagging trail, not a flash.** It receives the
///   same vertical crop as `EnergyBar`, fed an exponentially-smoothed
///   fraction (`lagging += (target - lagging) * 0.1` every tick, `target`
///   the current shield fraction on every path) rather than the raw one -
///   wired in `oag_hud::dialect_2048::vertical_bar_fraction` and
///   `Race::advance_energy_bar_delay`. Now in this list too.
///
/// `EnergyBarDelay`'s own runtime colour is the one open gap this pass did
/// not close: nothing in the decompiled function writes a colour onto it
/// either, so it draws at its authored opaque white - the "red cap... for a
/// moment after a hit" `docs/formats/2048-hud.md` recorded off `36-w-15.png`
/// is not reproduced.
///
/// # What is still deliberately left out
///
/// - `PickupBgFrame`: the arc round the held pickup, right of the shield.
///   Absent on the grid and in every no-pickup frame; up the whole time a
///   Missile or Rocket is held. State-gated, with the pickup.
/// - `PickupIcon` and `DenyPickup`: the icon. Seen once at the authored
///   top-centre rect with the `PickupText` caption `MISSILE` (the grant
///   announcement) and then inside `PickupBgFrame` at the bottom left for as
///   long as it is held - so the runtime moves the one widget, which the
///   layout's placeholder-rect idiom already implied.
/// - `ThrustBar`, `SpeedBar0`-`4`, `PilotAssist`, `ZoneLight0`-`9`: drawn off
///   race state by `oag_hud::dialect_2048::state_sprites` since 2026-10-05.
/// - `PilotAssist` is the icon left of the speed bar, up only once Pilot
///   Assist was switched to Extreme in the options; absent at Normal.
/// - `ZoneLight0`-`9` and `ZoneSpeedLogo`: lit per zone, and never seen (the
///   class was drawn as text, `VENOM`/`FLASH`, not as the badge).
/// - `Radar`, `PlayerDot`, `RadarDot*`: never seen in any state reached.
/// - `ManualShield`, `GiftCannon*`, `TargetShip*`, `TargetReticule*`,
///   `ReverseShipPos*`, `VoiceCom*`, `ObjectivePoint*`, `GloryMoment*`,
///   `ProtoTypeShipLogo`, `SpeedPad*`, `RaceMedal`, `MissileSight*`,
///   `LeachBeamSight*`: never seen in any state reached.
///
/// The countdown, the `SCORE` readout in Zone and the `ZONE`/number/class
/// text are not sprites and are recorded on `docs/formats/2048-hud.md`.
pub const ALWAYS_ON: &[&str] = &[
    "ThrustBarBG",
    "EnergyBgFrame",
    "EnergyBg",
    "EnergyBar",
    "EnergyBarDelay",
    "ZoneCounterBG",
];

/// `PickupIcon`'s per-weapon source rectangle, as [`oag_title::HudArt::pickup_icon_uv`]
/// carries it - `[U, V, W, H]` in `hud_2048.gxt` atlas pixels, indexed in
/// `oag_tables::weapons::Weapon::ALL`'s order.
///
/// **Read off `eboot.elf`, not inferred from a frame.** `Hud_UpdatePickupIcon`
/// (`0x81194f9c`) indexes a 12-slot table at `g_pickup_icon_uv_table`
/// (`0x81489070`, confidence 90) by the held weapon's internal id and writes
/// the four values straight into `PickupIcon`'s own fields; the internal ids
/// (1 Rockets .. 11 LeachBeam) are reordered here onto `Weapon::ALL`'s own
/// axis. Full evidence, including the two entries checked pixel-for-pixel
/// against a live frame (Missile and Rocket) and the disc-level sanity check
/// (all eleven crop to a distinct, semantically correct icon), is
/// [`pickup-icon-uv-table.md`].
///
/// `Repulser` and `Shuriken` are `None`: the table has no entry past index 11,
/// matching `docs/gameplay/pickups.md` recording neither as implemented on
/// this title, not an unmeasured gap. `Disruptor` is Pure's own weapon and no
/// title but Pure authors it at all.
///
/// Weapon id 1 (Rockets) is special-cased in the executable onto a *second*
/// table selected by an unresolved "tier" read, one entry of which duplicates
/// this row's own value exactly - see the doc page's own caveat. Not modelled
/// here: this row is the table's fallback/default entry, which is what the
/// cross-checked frame actually showed.
///
/// [`pickup-icon-uv-table.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md
pub const PICKUP_ICON_UV: [Option<[u16; 4]>; 14] = [
    Some([203, 201, 85, 86]), // Rocket
    Some([301, 201, 85, 86]), // Missile
    Some([692, 201, 85, 86]), // Quake
    Some([7, 201, 85, 86]),   // Cannon
    Some([7, 301, 85, 86]),   // Turbo
    Some([105, 301, 85, 86]), // Shield
    Some([790, 201, 85, 86]), // Autopilot
    Some([399, 201, 85, 86]), // Plasma
    Some([105, 201, 85, 86]), // Bomb
    Some([594, 201, 85, 86]), // Mine
    Some([496, 201, 85, 86]), // LeachBeam
    None,                     // Repulser - unimplemented on 2048
    None,                     // Shuriken - unimplemented on 2048
    None,                     // Disruptor - Pure-only weapon
];

/// The three HUD skins, as directory prefixes under `Data\XML\`.
///
/// On the same terms as [`oag_hd::hud::skins`], with one addition: this title
/// authors a *fourth* set, the bare root, that is not itself a selectable
/// skin - see [`DEMO`] and the module docs above.
pub mod skins {
    /// No prefix: the root set. **Not the played skin** - see [`DEMO`].
    pub const ROOT: &str = "";
    /// Wipeout 3's skin. Present, composes cleanly, unreferenced by any code
    /// path found so far.
    pub const WO3: &str = "wo3_hud/";
    /// Wipeout 2097's skin, on the same terms as [`WO3`].
    pub const RETRO_2097: &str = "2097_hud/";
    /// This title's own skin - the one `SpArcadeRaceManager_Construct` and
    /// its Elimination/SpeedLap/TimeTrial/Zone siblings actually construct a
    /// race's HUD from. See the module docs for the evidence.
    pub const PLAYED: &str = "2048_hud/";

    /// The bare root set's paths, read by [`DemoRaceManager_Construct`] -
    /// the attract-mode demo, not a mode a player reaches by starting a race.
    ///
    /// Kept distinct from [`played`] because the two skins are not shaped the
    /// same: the root carries Detonator and Duel, which 2048 never runs, and
    /// gives time trial and speed lap the same file where `2048_hud` gives
    /// them different ones.
    ///
    /// [`DemoRaceManager_Construct`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/vita-2048-eu-v104/race-hud-selection.md
    pub mod demo {
        /// Single race.
        pub const ARCADE: &str = r"Data\XML\Arcade_HUD.xml";
        /// Time trial and speed lap both - unlike [`super::played`], this
        /// skin gives the two modes one shared file, the same shape both PSP
        /// discs use.
        pub const TIME_TRIAL: &str = r"Data\XML\TimeTrial_HUD.xml";
        /// See [`TIME_TRIAL`].
        pub const SPEED_LAP: &str = TIME_TRIAL;
        /// Zone.
        pub const ZONE: &str = r"Data\XML\Zone_HUD.xml";
    }

    /// [`PLAYED`]'s paths - what [`LAYOUTS`](super::LAYOUTS) reads.
    pub mod played {
        /// Single race, constructed directly by `SpArcadeRaceManager_Construct`.
        pub const ARCADE: &str = r"Data\XML\2048_hud\Arcade_HUD.xml";
        /// Eliminator.
        pub const ELIMINATION: &str = r"Data\XML\2048_hud\Elimination_HUD.xml";
        /// Time trial - its own file, distinct from [`SPEED_LAP`].
        pub const TIME_TRIAL: &str = r"Data\XML\2048_hud\SpeedLap_TimeTrial_HUD.xml";
        /// Speed lap - its own file, distinct from [`TIME_TRIAL`].
        pub const SPEED_LAP: &str = r"Data\XML\2048_hud\SpeedLap_HUD.xml";
        /// Zone.
        pub const ZONE: &str = r"Data\XML\2048_hud\Zone_HUD.xml";
        /// This title's own bonus mode. `oag_race::Mode` has no rules for it
        /// yet, so nothing reads this today; kept so the skin's own file set
        /// is complete against the archive.
        pub const ZOMBIE: &str = r"Data\XML\2048_hud\Zombie_HUD.xml";
        /// Multiplayer tag, on the same terms as [`ZOMBIE`].
        pub const MP_TAG: &str = r"Data\XML\2048_hud\MPTag_HUD.xml";
    }
}

/// The layout entries themselves - the bare root set. Kept for
/// [`skins::demo`] and to mirror [`oag_hd::hud::layouts`]'s shape; **not**
/// what [`LAYOUTS`] reads. See the module docs.
pub mod layouts {
    /// Single race.
    pub const ARCADE: &str = super::skins::demo::ARCADE;
    /// Time trial.
    pub const TIME_TRIAL: &str = super::skins::demo::TIME_TRIAL;
    /// Speed lap - the time trial's, on this skin. See
    /// [`skins::demo::SPEED_LAP`](super::skins::demo::SPEED_LAP).
    pub const SPEED_LAP: &str = super::skins::demo::SPEED_LAP;
    /// Zone.
    pub const ZONE: &str = super::skins::demo::ZONE;
}

/// Every root layout the archive ships, played skin first.
///
/// 25: the eight bare-root files (shared with [`oag_hd::hud::ROOTS`]'s own
/// eight, Detonator and Duel included even though this build never reads
/// them), the seven under [`skins::PLAYED`], and five each under
/// [`skins::WO3`] and [`skins::RETRO_2097`]. The archive ships a 26th
/// `*_HUD.xml` - `Data\XML\SplitScreenZone_hud\Zone_HUD.xml` - excluded on the
/// same terms [`oag_hd::hud::ROOTS`] excludes its own split-screen family:
/// nothing in this project draws two viewports.
/// `crates/game/tests/vita_2048_hud_ground_truth.rs` re-derives this from the
/// archive's own manifest rather than trusting it.
pub const ROOTS: &[&str] = &[
    skins::played::ARCADE,
    skins::played::ELIMINATION,
    skins::played::MP_TAG,
    skins::played::SPEED_LAP,
    skins::played::TIME_TRIAL,
    skins::played::ZOMBIE,
    skins::played::ZONE,
    r"Data\XML\Arcade_HUD.xml",
    r"Data\XML\Detonator_HUD.xml",
    r"Data\XML\Duel_HUD\Duel_HUD.xml",
    r"Data\XML\Elimination_HUD.xml",
    r"Data\XML\MPTag_HUD.xml",
    r"Data\XML\SpeedLap_HUD.xml",
    r"Data\XML\TimeTrial_HUD.xml",
    r"Data\XML\Zone_HUD.xml",
    r"Data\XML\2097_hud\Arcade_HUD.xml",
    r"Data\XML\2097_hud\Elimination_HUD.xml",
    r"Data\XML\2097_hud\SpeedLap_HUD.xml",
    r"Data\XML\2097_hud\TimeTrial_HUD.xml",
    r"Data\XML\2097_hud\Zone_HUD.xml",
    r"Data\XML\wo3_hud\Arcade_HUD.xml",
    r"Data\XML\wo3_hud\Elimination_HUD.xml",
    r"Data\XML\wo3_hud\SpeedLap_HUD.xml",
    r"Data\XML\wo3_hud\TimeTrial_HUD.xml",
    r"Data\XML\wo3_hud\Zone_HUD.xml",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_root_is_named_once() {
        let mut seen: Vec<&str> = ROOTS.to_vec();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), before);
        assert_eq!(before, 25);
    }

    #[test]
    fn the_extension_rule_carries_a_leading_dot() {
        // The bug this test exists for: a bare `"gxt"` builds
        // `format!("{stem}{extension}")` into `missile_reticulegxt`, silently
        // dangling every reference. See the doc comment on `ART` above.
        assert_eq!(
            texture_entry(r"Data\HUD\Textures\missile_reticule.gtf"),
            r"Data\HUD\Textures\missile_reticule.gxt"
        );
        assert_eq!(
            texture_entry(r"Data\XML\2048_hud\Texture\missile_reticule.gxt"),
            r"Data\XML\2048_hud\Texture\missile_reticule.gxt"
        );
    }
}
