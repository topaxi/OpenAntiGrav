//! The in-race HUD's entries: eighteen root layouts, three skins, twelve
//! textures.
//!
//! Entry names in the [ADR-0022] sense, the same division [`oag_pulse::hud`]
//! draws: *which* files HD ships and what each is. Everything that **reads**
//! them - the `<LoadXML>` splice, the widget model, the offset composition - is
//! `oag_hud` and stays there, because it is one dialect across three
//! titles.
//!
//! Measured on `hdfury-ps3-eu-dec.iso` (`BCES-00664`), 2026-08-17. Written up
//! with its evidence in [`hd-hud.md`].
//!
//! # HD's HUD is not one file per mode
//!
//! Pulse ships five self-contained layouts. HD ships a **shell** per mode that
//! pulls in a dozen fragments by `<LoadXML SrcRel=...>`, so a mode's HUD is 5 to
//! 17 files rather than one, and reading only the root gets two empty
//! rectangles. `oag_hud::compose` is what assembles one - this crate
//! cannot link to it, being the wrong side of the dependency arrow.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`hd-hud.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/hd-hud.md
//! [`oag_pulse::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/pulse/src/hud.rs

/// The three HUD skins, as directory prefixes under `/data/xml/`.
///
/// `wo3` and `2097` are Wipeout 3 and Wipeout 2097 - the retro HUD skins the
/// game offers as an option - with the bare directory as the default. The same
/// three-way split appears in the fonts: `docs/formats/hd-frontend.md` records
/// `skin.xml` declaring `HUD`, `wo3HUD` and `2097HUD` font slots.
///
/// **How a skin is chosen is not read here.** The executable's race-manager
/// constructors pass whole paths as literals, so the choice is made by which
/// call runs rather than by composing a directory at runtime - see
/// `docs/ghidra/functions/ps3-hdfury-eu/race-hud.md`.
pub mod skins {
    /// No prefix: the default, modern HUD.
    pub const DEFAULT: &str = "";
    /// Wipeout 3's skin.
    pub const WO3: &str = "wo3_hud/";
    /// Wipeout 2097's skin.
    pub const RETRO_2097: &str = "2097_hud/";
}

/// The layout each mode this engine runs reads its HUD from, as
/// [`oag_title::Title::hud`] carries it.
///
/// The constants below are the same values, kept as named items because that is
/// where their evidence is written down; this is the table the engine reads.
///
/// **The default skin, because how a skin is chosen is not read** - see
/// [`skins`]. The `wo3` and `2097` roots stay in [`ROOTS`] until something can
/// say which one a race is in.
///
/// `speed_lap` is this title's own file. Both PSP discs ship none and draw the
/// time trial's instead, which is the divergence
/// [`oag_title::hud`](oag_title::hud) exists for.
pub const LAYOUTS: &oag_title::HudLayouts = &oag_title::HudLayouts {
    arcade: layouts::ARCADE,
    time_trial: layouts::TIME_TRIAL,
    speed_lap: layouts::SPEED_LAP,
    zone: layouts::ZONE,
    elimination: layouts::ELIMINATION,
};

/// How HD's HUD sprites reach the screen, as [`oag_title::Title::hud_art`]
/// carries it.
///
/// All three rows disagree with both PSP titles', which is why
/// [`oag_title::HudArt`] is an axis rather than a constant in the engine.
pub const ART: &oag_title::HudArt = &oag_title::HudArt {
    // The layouts name the exporter's input; the disc ships the conversion.
    // See `texture_entry`, which is this row spelled out with its evidence.
    texture_extension: Some(TEXTURE_EXTENSION),
    always_on: ALWAYS_ON,
    // **Concentric sprites, not brackets.** HD's arcade HUD composes to zero
    // `<Mode3D>` models; its reticle is six `<Image>` widgets off
    // `Data\HUD\Textures\missile_reticule.gtf`, authored at 128, 108, 80 and
    // 64 pixels square and all centred on `(-960, 540)` - the negated centre of
    // its own 1920x1080 screen, which is the same placeholder idiom the PSP
    // titles use at `(-240, 136)`.
    //
    // The four here are the seeking set and the two on the other row are the
    // locked one, read off the names. `MissileSightBG` is the backdrop and is
    // drawn first, so the order of this list is paint order.
    //
    // **The LeachBeam's own four are in too, as of 2026-09-15.** HD's arcade
    // layout also authors `LeachBeamSightBG`/`Outer`/`Middle`/`Inner`, all off
    // `Data\HUD\Textures\HUD_Components_01.gtf` at 176/176/128/80 pixels
    // square and centred on the same `(-960, 540)` placeholder - measured
    // directly off the composed layout, confidence 90. There is no
    // `LeachBeamSight*LockedOn*` widget anywhere on any of the eighteen
    // composed layouts (checked, not assumed -
    // `crates/game/tests/lock_sight_ground_truth.rs`), so unlike the Missile's
    // pair above there is no second widget for a lock to add. **Which of the
    // four draws when is measured and ported, at reduced confidence** - see
    // `Sights::Concentric`'s own `leach` field doc for the reveal law and
    // what this engine's own hold constant means for it.
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
    // **Not Pulse's substitution.** HD's `PickupBackground` is a hexagon
    // *outline* rather than a filled white one, so the icon on it is visible in
    // the colour the layout authors. See [`ALWAYS_ON`] for the frame.
    //
    // Confirmed directly, 2026-09-04: `HUD_pickup_background.xml`'s
    // `PickupBackground` widget carries no `Color=` attribute at all, unlike
    // every `<Type>Icon` beside it (all thirteen author `Color="FEConst->
    // HudColour1"`) and unlike Pulse's own `PickupBackground` - it draws
    // whatever `HUD_Components.gtf` bakes at that UV rect, with nothing for a
    // runtime tint to override.
    pickup_backdrop_colour: None,
    // Same reasoning: an outline needs no colour substituted into it, per
    // weapon or otherwise.
    pickup_colours: None,
    // HD's icons are `<Image>` sprites (the `<Type>Icon` widgets the comment
    // above names), the same kind as Pulse's - not `<Mode3D><Model>`s, so this
    // field is moot on the same terms `pickup_colours` is above it.
    pickup_icon_models: None,
    pickup_icon_backdrop_model: None,
    // Same reasoning again: HD names a widget per weapon, same as Pulse -
    // 2048's own dialect is the one with a single rewritten-UV widget.
    pickup_icon_uv: None,
    // The fifteen rungs' names. Which rung a zone is on is a different
    // question and an open one - see [`ZONE_SPEED_CLASSES`].
    zone_speed_classes: Some(ZONE_SPEED_CLASSES),
    // **Measured `false`, not left at Pulse's `true`.** All three of
    // `talons-matched/{00,01,03}.png` read a bare number - `100`, `100`, `98` -
    // with no `%` anywhere. See `oag_title::HudArt::shield_percent`.
    shield_percent: false,
    // lane/2048-hud-font: measured. Every language plugin's own
    // `definition.xml` (`DATA02`/`DATA03`) names
    // `<Font><Values name="HUD" ...Src="Data\FE\Fonts\PulseHud.fnt">` and
    // `HUDSmall` -> `Data\FE\Fonts\small.fnt` - `docs/formats/hd-frontend.md`'s
    // "`menu_font` is `None`, and that is a measurement" section quotes the
    // XML directly (lines 1555-1568). See `oag_title::HudArt::hud_font_role`.
    // Single-field edit; the rest of this file is lane 3's.
    hud_font_role: "HUD",
    hud_small_font_role: Some("HUDSmall"),
    total_time_timed_modes_only: false,
    kill_column: false,
    message_slots: false,
    runtime: Some(RUNTIME),
};

/// What HD's per-tick HUD update writes over the layout, read off the
/// executable: `Hud_UpdateShieldReadout` (`0x000866c8`),
/// `Hud_UpdateLapCounter` (`0x00096ef0`) and `Hud_UpdatePositionCounter`
/// (`0x00096088`), all three called every tick from `Hud_Update`. Every value
/// below is a direct read; the evidence, the branch table and the addresses
/// are on [`hud-readouts.md`].
///
/// - `0x1664FF` is built branch-free from `0xFF166500 + 0x00FFFFFF` when a
///   mode byte is clear - the same "HD blue" `zone_hud.xml` authors on its
///   own `DamageBar`. The fill and the number both take it, opaque, which is
///   why the number reads blue and not the translucent red
///   `HUD_damage_indicator.xml` authors for it. Confidence 82.
/// - `20` and `8` are the floats at `0x008a7678` and `0x008a7704`: the
///   readout flashes at or under 20 % (or through a post-hit second), and a
///   flash phase is on while `floor(t * 8)` is even.
/// - The two arcs carry no colour write at all: their yellow is baked into
///   `HUD_Components.gtf`, and the runtime only shows and hides segments.
///   `8` is the position function's own literal, not the field size.
///
/// [`hud-readouts.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md
pub const RUNTIME: &oag_title::hud::RuntimeHud = &oag_title::hud::RuntimeHud {
    shield: oag_title::hud::ShieldReadout {
        fill: "DamageBar",
        background: "DamageBarBg",
        text: "ShieldBarText",
        rgb: 0x0016_64FF,
        warning_rgb: 0x00FF_0000,
        critical_percent: 20,
        phases_per_second: 8,
    },
    lap_arc: oag_title::hud::SegmentArc {
        prefix: "LapBar",
        segments: 7,
    },
    place_arc: oag_title::hud::SegmentArc {
        prefix: "PosBar",
        segments: 8,
    },
};

/// The sprite widgets HD draws whenever its HUD is up.
///
/// # Read off a frame of the running original
///
/// The fifteen below are the widgets visible in a race capture taken through
/// [`rpcs3-debugger.md`]'s harness (`just rpcs3-race`, 2026-08-24,
/// `hdfury-ps3-eu-dec.iso`, speed lap on Talon's Junction) with the craft
/// stationary on the grid, no pickup held, no assist and no warning up. Each
/// one is identified by matching its authored source rectangle against the
/// decoded atlas and the shape against the frame:
///
/// | Widget | In the frame |
/// | --- | --- |
/// | `LapPanel`, `PositionPanel` | the hexagonal panel top left |
/// | `TimeIcon`, `TimeIconVertBar`, `ClockIcon`, `BestLapImage` | the lap-time cluster bottom left |
/// | `SpeedBarBG`, `ThrustBarBG`, `SpeedBarOverlay` | the speed bar bottom right |
/// | `PickupFarBackground`, `PickupAbsorbBG`, `PickupDamageBG`, `PickupBackground` | the pickup bar and its hexagon, top centre |
/// | `DamageBarBg` | the striped hexagon around it |
/// | `TotalTimeBG` | the panel at the right edge |
///
/// `PositionPanel` is the one not in that frame: the capture is speed lap,
/// which has no place, and the arcade layout authors `PositionPanel` where this
/// one authors `TotalTimeBG` - a background of the same kind at the same
/// corner. Confidence 70 against 90 for the rest, and it is here rather than
/// left out because the alternative is an arcade HUD whose position readout
/// floats on nothing.
///
/// # What is deliberately left out
///
/// **A single frame cannot tell "always on" from "on in this state"**, and the
/// widgets below are all in a state this capture does not vary:
///
/// - `LapBar0`-`LapBar6` and `PosBar0`-`PosBar7`, the progress arcs around the
///   two panels, and `DamageBar`, the shield hexagon's fill. All three are
///   shown, cropped or recoloured every tick by the executable, so they are
///   drawn by [`RUNTIME`]'s rules rather than from this list. The arcs' yellow
///   is the atlas's own, not a tint: an earlier reading here called the
///   segments white in the layout, which sampled the atlas upside down.
/// - `Lap1Image`-`Lap4Image`, the per-lap time rows, which appear as laps are
///   set - their labels are authored as empty strings.
/// - `FrameLeft`/`Middle`/`Right` and their four numbered copies, the info-text
///   frames; `WrongWayFrame*`; the forward and rear warning icons; `VoiceCom0`-
///   `VoiceCom7`; `Prox0`-`Prox4`; `AssistIndicator*`; `MissileSight*` and
///   `LeachBeamSight*`; and `ReverseShipPos*`.
///
/// # Zone's ladder is the second frame, and it is a different mode
///
/// **2026-08-31.** The three names this list used to leave out for want of a
/// Zone capture - `ZoneBG`, `CurrentZonePanel` and
/// `ZonePlusLight0`-`ZonePlusLight10` - are here now, off a Zone frame of the
/// running original the maintainer supplied
/// (`data/shots/hd_zone_hud_original.png`, gitignored). All thirteen are up in
/// it with the craft mid-race: the vertical column down the left, the wider
/// panel across the current zone's row, and the small tick beside each row.
///
/// **They cost this list nothing on the other seventeen layouts.** All thirteen
/// are authored by the Zone roots and by nothing else across the whole disc -
/// checked over every `*_hud.xml` including the split-screen family - so a
/// title-wide allow-list carrying them changes no other mode's frame.
/// `every_zone_ladder_widget_is_authored_by_the_zone_layouts_alone` pins that.
///
/// **`NextSpeedClassBG` is authored beside them and is still not on this list**,
/// but for a reason rather than a gap: it is up only when there *is* a next
/// class with a row on screen, which an allow-list cannot express.
/// `oag_hud::draw::zone_next_row` draws it and reconstructs its position,
/// the layout authoring it at the placeholder `x=0 y=0`.
///
/// [`rpcs3-debugger.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/reverse-engineering/rpcs3-debugger.md
pub const ALWAYS_ON: &[&str] = &[
    "LapPanel",
    "PositionPanel",
    "TotalTimeBG",
    "PickupFarBackground",
    "PickupAbsorbBG",
    "PickupDamageBG",
    "DamageBarBg",
    "PickupBackground",
    "TimeIcon",
    "TimeIconVertBar",
    "ClockIcon",
    "BestLapImage",
    "SpeedBarBG",
    "ThrustBarBG",
    "SpeedBarOverlay",
    // Zone's ladder. Authored by the Zone roots alone; see the section above.
    "ZoneBG",
    "CurrentZonePanel",
    "ZonePlusLight0",
    "ZonePlusLight1",
    "ZonePlusLight2",
    "ZonePlusLight3",
    "ZonePlusLight4",
    "ZonePlusLight5",
    "ZonePlusLight6",
    "ZonePlusLight7",
    "ZonePlusLight8",
    "ZonePlusLight9",
    "ZonePlusLight10",
];

/// The fifteen rungs of Wipeout HD's Zone escalation ladder, by name.
///
/// # The names, and the three readings that agree on them
///
/// 1. **`/data/environments/zonemode.effectsettings`** keys its palettes by
///    `0 Start`, `1 Sub Venom`, `2 Venom`, `3 Sub Flash` .. `14 Supersonic` -
///    fifteen rungs whose names *are* speed classes, numbered from zero.
/// 2. **The language plugin carries every one of those names as a HUD string**,
///    one for one: `MSC_SVENOM` = `SUB-VENOM` against `1 Sub Venom`, through to
///    `IG_HUD_SUPSON` = `SUPERSONIC` against `14 Supersonic`. `IG_HUD_MACH1` =
///    `MACH 1` fills `13 Mach 1`, the one rung with no obvious class name of its
///    own - which is the row that says this is the right table rather than a
///    plausible one.
/// 3. **A Zone frame of the running original reads `SUB-VENOM`** in the
///    `SpeedClass` widget (`data/shots/hd_zone_hud_original.png`), spelled with
///    the hyphen the string table has and the effectsettings key does not - so
///    the HUD is drawing *these strings*, not the palette keys.
///
/// `0 Start` is `None`: the string table has no class name for it, and every
/// other rung has one.
///
/// **Confidence 84** on the fifteen names in this order. It is a table read off
/// the disc twice and confirmed on one rung in a frame; no disassembly says the
/// HUD indexes it, and nothing at all has been read about rungs 3 upward.
///
/// # This is the rung's name; which zone is on which rung is [`crate::race::ZONE_STAGES`]
///
/// The two are separate tables in the original too, and they were recovered
/// hours apart. An earlier revision of this constant read the ladder as **one
/// rung per zone** off the reference frame's first two rows; the maintainer's
/// play corrected it - not every zone is a class bump - and the executable then
/// settled it outright: `g_ZoneSpeedClassTable` (`0x00860d44`) is fourteen
/// records of `{ zoneThreshold, stringIdPointer }` with bands `0`-`1`, `2`,
/// `3`-`4`, `5`-`6`, `7`-`11` and so on. See
/// [zone-speed-class-table.md].
///
/// **That table carries these same ids**, which is why this constant is a
/// cross-check rather than a duplicate: it is indexed by *rung* where
/// `ZONE_STAGES` is indexed by *zone*, and
/// `the_two_zone_tables_name_the_same_class_at_every_zone` asserts the two
/// cannot drift. `oag_hud` reads this one, because what the HUD has in
/// hand is the rung the colour grade is showing.
///
/// [zone-speed-class-table.md]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/ps3-hdfury-eu/zone-speed-class-table.md
pub const ZONE_SPEED_CLASSES: &oag_title::ZoneSpeedClasses = &oag_title::ZoneSpeedClasses {
    ids: &[
        None,                  // 0 Start - no class name in the string table
        Some("MSC_SVENOM"),    // 1 Sub Venom
        Some("Venom"),         // 2 Venom
        Some("MSC_SFLASH"),    // 3 Sub Flash
        Some("Flash"),         // 4 Flash
        Some("MSC_SRAPIER"),   // 5 Sub Rapier
        Some("Rapier"),        // 6 Rapier
        Some("MSC_SPHANTOM"),  // 7 Sub Phantom
        Some("Phantom"),       // 8 Phantom
        Some("MSC_SPPHANTOM"), // 9 Super Phantom
        Some("MSC_ZEN"),       // 10 Zen
        Some("IG_HUD_SUPZEN"), // 11 Super Zen
        Some("IG_HUD_SUBSON"), // 12 Subsonic
        Some("IG_HUD_MACH1"),  // 13 Mach 1
        Some("IG_HUD_SUPSON"), // 14 Supersonic
    ],
};

/// The eighteen root layouts, by mode and skin.
///
/// **Read off the manifests rather than off the executable**, and the two do not
/// agree: `.rodata` carries 31 `*_HUD.xml` paths, which include the split-screen
/// and vertical-split variants this list leaves out. What is here is every root
/// that ships as `/data/xml/[<skin>/]<mode>_hud.xml`.
///
/// Two absences are the shape of the thing rather than gaps in the reading:
/// **Detonator, Duel and MPTag have no skinned variant at all**, and Duel's root
/// is one directory further down.
pub mod layouts {
    /// Single race and tournament.
    pub const ARCADE: &str = "/data/xml/arcade_hud.xml";
    /// Eliminator.
    pub const ELIMINATION: &str = "/data/xml/elimination_hud.xml";
    /// Time trial.
    pub const TIME_TRIAL: &str = "/data/xml/timetrial_hud.xml";
    /// Speed lap.
    pub const SPEED_LAP: &str = "/data/xml/speedlap_hud.xml";
    /// Zone and Zone Battle.
    pub const ZONE: &str = "/data/xml/zone_hud.xml";
    /// Detonator, the Fury mode. **No skinned variant.**
    pub const DETONATOR: &str = "/data/xml/detonator_hud.xml";
    /// Multiplayer tag. **No skinned variant.**
    pub const MP_TAG: &str = "/data/xml/mptag_hud.xml";
    /// Duel, the Fury head-to-head mode. **No skinned variant**, and the only
    /// root that is not directly under `/data/xml/`.
    pub const DUEL: &str = "/data/xml/duel_hud/duel_hud.xml";
}

/// Every root layout the disc ships, default skin first.
///
/// Eighteen: the eight in [`layouts`], plus five each for [`skins::WO3`] and
/// [`skins::RETRO_2097`]. `crates/game/tests/hd_hud_ground_truth.rs` re-derives
/// this from the manifests rather than trusting it.
///
/// **Single-screen only.** The disc also ships a split-screen family -
/// `splitscreen_hud/`, `splitscreenzone_hud/`, `duel_hud/duel_splitscreen_hud/`
/// and a `vert_` variant of most of it - which is not enumerated here because
/// nothing in this project draws two viewports. The one that would slip past a
/// looser rule is `/data/xml/splitscreenzone_hud/zone_hud.xml`: the same
/// basename as [`layouts::ZONE`], one directory across.
pub const ROOTS: &[&str] = &[
    layouts::ARCADE,
    layouts::ELIMINATION,
    layouts::TIME_TRIAL,
    layouts::SPEED_LAP,
    layouts::ZONE,
    layouts::DETONATOR,
    layouts::MP_TAG,
    layouts::DUEL,
    "/data/xml/wo3_hud/arcade_hud.xml",
    "/data/xml/wo3_hud/elimination_hud.xml",
    "/data/xml/wo3_hud/timetrial_hud.xml",
    "/data/xml/wo3_hud/speedlap_hud.xml",
    "/data/xml/wo3_hud/zone_hud.xml",
    "/data/xml/2097_hud/arcade_hud.xml",
    "/data/xml/2097_hud/elimination_hud.xml",
    "/data/xml/2097_hud/timetrial_hud.xml",
    "/data/xml/2097_hud/speedlap_hud.xml",
    "/data/xml/2097_hud/zone_hud.xml",
];

/// The archive entry a layout's `src=` reference names.
///
/// # The layouts name source art, and the disc ships the conversion
///
/// A HUD sprite's `src` is the **exporter's input**, not the shipped file:
/// across the eighteen composed layouts the twelve distinct references spell
/// themselves `.gtf` ten times, `.mip` once and `.tga` once, and the two odd
/// ones are the two with the most uses between them. `Data\HUD\Textures\
/// hdHUD.mip` has **148** references and there is **no `.mip` anywhere on this
/// disc**; `Data\HUD\Textures\detonator_hud2.tga` has 44 and the disc's only
/// `.tga` is a smoke ramp in `data/ribboneffects`. Both resolve when the
/// extension is replaced with `.gtf`: `/data/hud/textures/hdhud.gtf` is in
/// `DATA02` and `DATA03`, `/data/hud/textures/detonator_hud2.gtf` in `DATA00`.
///
/// So the rule is **replace the extension**, and with it all twelve references
/// resolve to a shipped entry rather than ten of twelve. Read as literal names
/// they are dangling references and 192 of the HUD's 1,029 sprites have no
/// texture - which is exactly how it looked before this was measured.
///
/// # Confidence 90
///
/// Twelve of twelve resolve, and the two that need the rule need it for 192
/// sprites, so this is not a coincidence of one file. 2026-08-25: widened past
/// the HUD's own twelve references to **every** `Src`/`ImageSrc`-shaped image
/// reference across all of HD's XML (133 distinct values, all eighteen HUD
/// layouts plus front end, ships, skins) - **129 of 133 (97%) resolve under
/// this exact rule**, `hdHUD.mip` and `detonator_hud2.tga` both among them. The
/// four that do not (`teaser_firedup.mip`, `teaser_medievil.mip`,
/// `teaser_wipeout.mip`, `default_texture.mip`) do not exist on the disc under
/// *any* name or extension - cut content and an unbacked fallback name, not
/// counter-examples. Not 94 (the arithmetic-invariant ceiling) because those
/// four are absences rather than confirmations, and because **the literal HUD
/// code path is still unread** - the ceiling below still applies. What moved
/// it off 85: a hardcoded instance of the *identical* convention was found in
/// `EBOOT.elf` while chasing this rule through the executable - a ship-thumbnail
/// loader builds `%s\fe\miniBW.gtf` from a bare directory, ignoring entirely
/// that the XML authors the same asset as `Data\Ships\<Ship>\fe\miniBW.tga`.
/// The engine hardcoding `.gtf` and discarding the source extension, observed
/// directly in code for one asset kind, is what a blind string-replace with no
/// fallback attempt looks like from the executable side - not proof this
/// exact function does it for HUD sprites too, but no longer inferred from the
/// file set alone either.
///
/// A second spelling exists on this disc and is **not** what the HUD uses:
/// `/data/environments/02_track/hd_textures/and_thinsteps.tga.gtf` *appends*
/// rather than replaces. Nothing in the HUD reaches a name of that shape.
///
/// Separators and case are left alone - [`oag_assets::psarc::Archive::read_path`]
/// folds both.
///
/// The rewriting itself is [`oag_title::hud::replace_extension`] and the
/// extension is [`ART`]'s own row, so the engine applies this rule to HD
/// without knowing it is HD. This function is that pair spelled out, and is
/// where the evidence above lives.
#[must_use]
pub fn texture_entry(reference: &str) -> String {
    oag_title::hud::replace_extension(reference, TEXTURE_EXTENSION)
}

/// What [`texture_entry`] replaces a reference's extension with.
pub const TEXTURE_EXTENSION: &str = ".gtf";

/// The twelve textures the eighteen composed layouts sample, as the layouts
/// spell them.
///
/// Kept as the **reference** spelling rather than the resolved one, because the
/// reference is what is authored and [`texture_entry`] is the claim about it -
/// storing the resolved name would hide the rule inside a constant. Counts are
/// sprites across all eighteen layouts, so a texture shared by three skins is
/// counted three times.
///
/// | Uses | Reference |
/// | ---: | --- |
/// | 338 | `HUD_Components.gtf` |
/// | 148 | `hdHUD.mip` |
/// | 105 | `fury_hud.gtf` |
/// | 104 | `wo3_hud.gtf` |
/// | 97 | `wo2097_hud.gtf` |
/// | 63 | `HUD_Components_01.gtf` |
/// | 56 | `voiceCom.gtf` |
/// | 44 | `detonator_hud2.tga` |
/// | 36 | `missile_reticule.gtf` |
/// | 24 | `HUD_Components_02.gtf` |
/// | 12 | `nitro_hud.gtf` |
/// | 2 | `ZoneDamage.gtf` |
///
/// **One of them is not a HUD texture at all**: `Data\FE\Images\voiceCom.gtf` is
/// a front-end image, reached from the position-list fragment for the
/// voice-chat indicator. So a HUD's texture set is not confined to
/// `data/hud/textures/`, which a loader that assumed a directory would get
/// wrong for 56 sprites.
pub const TEXTURES: &[&str] = &[
    r"Data\HUD\Textures\HUD_Components.gtf",
    r"Data\HUD\Textures\HUD_Components_01.gtf",
    r"Data\HUD\Textures\HUD_Components_02.gtf",
    r"Data\HUD\Textures\ZoneDamage.gtf",
    r"Data\HUD\Textures\detonator_hud2.tga",
    r"Data\HUD\Textures\fury_hud.gtf",
    r"Data\HUD\Textures\hdHUD.mip",
    r"Data\HUD\Textures\missile_reticule.gtf",
    r"Data\HUD\Textures\nitro_hud.gtf",
    r"Data\FE\Images\voiceCom.gtf",
    r"Data\XML\wo3_hud\texture\wo3_hud.gtf",
    r"Data\XML\2097_hud\texture\wo2097_hud.gtf",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_source_art_references_resolve_to_a_gtf() {
        assert_eq!(
            texture_entry(r"Data\HUD\Textures\hdHUD.mip"),
            r"Data\HUD\Textures\hdHUD.gtf"
        );
        assert_eq!(
            texture_entry(r"Data\HUD\Textures\detonator_hud2.tga"),
            r"Data\HUD\Textures\detonator_hud2.gtf"
        );
    }

    #[test]
    fn a_reference_that_is_already_a_gtf_is_unchanged() {
        for reference in TEXTURES.iter().filter(|r| r.ends_with(".gtf")) {
            assert_eq!(&texture_entry(reference), reference);
        }
    }

    /// The **append** spelling exists elsewhere on this disc, so replacing must
    /// not turn one of those back into its source name.
    #[test]
    fn the_appended_spelling_survives_the_rule() {
        assert_eq!(
            texture_entry("/data/environments/02_track/hd_textures/and_thinsteps.tga.gtf"),
            "/data/environments/02_track/hd_textures/and_thinsteps.tga.gtf"
        );
    }

    /// A dot in a directory name is not a file extension, and appending `.gtf`
    /// to the whole path is the only right answer for an extensionless name.
    #[test]
    fn a_dot_in_a_directory_is_not_an_extension() {
        assert_eq!(texture_entry(r"Data\v1.2\atlas"), r"Data\v1.2\atlas.gtf");
        assert_eq!(texture_entry("data/v1.2/atlas"), "data/v1.2/atlas.gtf");
    }

    #[test]
    fn every_root_is_named_once() {
        let mut seen: Vec<&str> = ROOTS.to_vec();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), before);
        assert_eq!(before, 18);
    }
}
