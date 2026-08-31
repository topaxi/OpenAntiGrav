//! Wipeout 2048's in-race HUD layout entries.
//!
//! Entry names in the [ADR-0022] sense - *which* files this package ships -
//! the same division [`oag_hd::hud`] draws. Everything that reads them is
//! `oag_game::hud`.
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
    // same shape, and is not modelled here - `oag_title::hud::Sights` has one
    // reticle axis, and HD's own `ART` leaves its `LeachBeamSight*` widgets
    // out on the same terms (see its `ALWAYS_ON` doc comment). Confidence 85.
    sights: &oag_title::hud::Sights::Concentric {
        seeking: &[
            "MissileSightBG",
            "MissileSightOuter",
            "MissileSightInner",
            "MissileSightMiddle",
        ],
        locked: &["MissileSightLockedOnLines", "MissileSightLockedOnMiddle"],
    },
    // `None` is "draw what the layout authors", which is the conservative
    // answer for a title whose pickup backdrop colour nothing has measured -
    // Pulse's colour substitution is the one that would need evidence.
    pickup_backdrop_colour: None,
    // `None`: 2048's Zone HUD shows a class per *band* of zones off
    // `oag_2048::race::ZONE_STAGES`, which is a different shape from the
    // per-zone ladder this row carries. See `oag_title::ZoneSpeedClasses`.
    zone_speed_classes: None,
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

/// The sprite widgets 2048 draws whenever its HUD is up: **unread**.
///
/// Layout composition and the reticle axis are both measured now (see
/// [`ART`]), but which widgets a running race actually shows is a different
/// question, and this title has no equivalent of the rpcs3 frame capture
/// [`oag_hd::hud::ALWAYS_ON`] rests on - nothing here runs the Vita title.
/// Empty because nothing has been captured, *not* because a capture came back
/// with nothing.
pub const ALWAYS_ON: &[&str] = &[];

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
