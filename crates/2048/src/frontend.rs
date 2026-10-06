//! Wipeout 2048's front end: its boot chain, its language plugins and its
//! touch-icon layout.
//!
//! Full reading in `docs/formats/2048-frontend.md`; the type this fills is
//! [`oag_title::FrontEnd`], widened for this title by [ADR-0054]. Read
//! directly off `PSP2/data.psarc` in
//! `data/extracted/vita/PCSF00007/base` and corroborated by one cold
//! Vita3K boot, 2026-09-20 - see [`BOOT_PROFILE`] for what that boot did and
//! did not confirm.
//!
//! # No `MenuSkin` here, and that is the point
//!
//! This module has no `MENU_SKIN` constant the way [`oag_pulse::frontend`],
//! [`oag_pure::frontend`] and [`oag_hd::frontend`] each do. 2048 authors no
//! `<FEGlobals>` block and no `<Menu>`/`<HorizMenu>` widget anywhere across
//! all 25 `NEWGUI` documents - `oag_title::FrontEnd::menu` stays `None` on
//! [`super::FRONT_END`], and [`TOUCH`] is what fills the axis instead. See
//! [ADR-0054] and `oag_title::touch`'s own module docs.
//!
//! [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md
//! [`oag_pulse::frontend`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/pulse/src/frontend.rs
//! [`oag_pure::frontend`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/pure/src/frontend.rs
//! [`oag_hd::frontend`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/frontend.rs

use oag_title::{TouchButton, TouchFrontEnd};

/// Screen names, spelled exactly as `NEWGUI/Definition.xml` and
/// `NEWGUI/Bootup_Definition_EU.xml` spell them.
pub mod states {
    /// Leaves via `<UnityConBasic>`'s `MoveTo`, success and failure both to
    /// [`BOOT_STUDIO_LOGO`] - a network check the boot passes through
    /// unconditionally, not a fork.
    pub const BOOT_CONNECT: &str = "Boot Connect";
    /// The Studio Liverpool card. `<Redirect delay="4.0">` - matched to the
    /// second by five identical 0.75s Vita3K samples (~3.75s held).
    pub const BOOT_STUDIO_LOGO: &str = "Boot Studio Logo";
    /// Plays [`super::names::INTRO_MOVIE`]. Confirmed on Vita3K as a real,
    /// changing picture across ~90s of samples - a ship-history reel, not a
    /// logo card.
    pub const BOOT_INTRO_MOVIE: &str = "Boot Intro Movie";
    /// `type="HDDBoot"` - the save-data check. Not caught as a distinct
    /// Vita3K frame (plausibly sub-0.75s with a save already present), which
    /// does not contradict the declared order.
    pub const LOAD_SAVE_BOOTUP: &str = "Load Save Bootup";
    /// `WIPEOUT 2048` and `PRESS ANY BUTTON TO START` - confirmed byte for
    /// byte against `Legal_Line_Definition_EU.xml`'s own footer string on
    /// Vita3K.
    pub const TITLE_SCREEN: &str = "TitleScreen";
    /// Where [`TITLE_SCREEN`] goes on a press - the touch-icon grid
    /// [`super::TOUCH`] describes. **Not the next [`oag_title::BootStep`]
    /// in [`super::BOOT_PROFILE`]'s own chain**: a percentage-driven
    /// `LOADING...` screen and an attract-mode demo race sit in between on
    /// Vita3K, both undeclared in any `NEWGUI` file and neither walked by
    /// this build's own boot mechanism yet. See
    /// `docs/formats/2048-frontend.md`'s "Corroborated on Vita3K" section.
    pub const GAME_MODE_CHOICE: &str = "GameModeChoice";
    /// The five-icon row `newFEshell`'s `<TouchHomeButton>` redirects to -
    /// team, community, profile, options, extras. See [`super::TOUCH`]'s
    /// `home`.
    pub const HOME: &str = "Home";
    /// The touch shell itself: `<Screen name="newFEshell">`, whose
    /// `<TouchScroll>` holds the persistent `<FE3DCanvas>` map and the
    /// `<TouchCampaign>` widget - the screen the campaign is picked on, and
    /// the one `GameModeChoice`'s confirm tick redirects to.
    pub const NEW_FE_SHELL: &str = "newFEshell";
    /// Where a tap on a campaign event goes: `<TouchCampaign>`'s and
    /// `<FE3DCanvas>`'s shared `redirect="Launch 2048"`, and one of the five
    /// boot-mode strings the executable knows by name (`game-boot.md`).
    /// **Not a screen in any `NEWGUI` file** - it is the disc's own name for
    /// leaving the front end for a race, the way Pulse's `Launch Game` is.
    pub const LAUNCH_2048: &str = "Launch 2048";

    /// `Team_Definition.xml`'s `teamshell->team`, the screen
    /// [`HOME`]'s `ER_TEAM` tile redirects to (`redirect="team"`). Fired and
    /// looked up by this bare name, not the nested path its own standalone
    /// parse computes - see [`super::includes`]'s module doc for why that is
    /// safe: `Screens::by_name` matches on the flattened `name` field
    /// regardless, and [`super::super::frontend`]'s `STATES` list is what
    /// registers the bare string the redirect actually names.
    pub const TEAM: &str = "team";
    /// `Profile_Definition.xml`'s `profileshell->profile`, `HOME`'s
    /// `FE_PROFILE` tile's `redirect`.
    pub const PROFILE: &str = "profile";
    /// `profile`'s own `profile_stats` sibling, one tap away either
    /// direction.
    pub const PROFILE_STATS: &str = "profile_stats";
    /// `Options_Definition.xml`'s `optionsshell->options`, the settings hub -
    /// reached from [`OPTIONS_CAMERA`]'s own tick-less back gesture, never
    /// directly from `Home` (see that constant).
    pub const OPTIONS: &str = "options";
    /// **What `HOME`'s `FE_OPT_PLUS` tile actually redirects to** - not
    /// [`OPTIONS`] itself. The disc jumps straight into the camera panel;
    /// `options`'s own four buttons are reached from there by an unauthored
    /// back gesture, not the other way around.
    pub const OPTIONS_CAMERA: &str = "OptionsCamera";
    pub const OPTIONS_AUDIO: &str = "OptionsAudio";
    pub const OPTIONS_CONTROLS: &str = "OptionsControls";
    pub const OPTIONS_PILOT: &str = "OptionsPilot";
    /// `options`'s own tick: `<ProfileController task="Auto Save">` then an
    /// unconditional `<Redirect delay="0.2">` to `Home` - a save this build
    /// has nothing to write, so it is a timed pass-through like
    /// [`BOOT_STUDIO_LOGO`]'s own delay.
    pub const SAVE_2048_OPTIONS: &str = "save_2048_options";
    /// `Community_Definition.xml`'s network-refusal screen -
    /// `HOME`'s `FE_COMMUNITY` tile's actual `redirect`, never `community`
    /// itself (which needs a live session this build never has and so is
    /// never entered, the same standing `GameModeChoice`'s three network
    /// modes carry).
    pub const COMMUNITY_ADHOC_CHECK: &str = "communityAdhocCheck";
    /// `Extras_Definition_EU.xml`'s `2048extrasshell->2048extras`, `HOME`'s
    /// `FE_EXTRAS` tile's `redirect`.
    pub const EXTRAS: &str = "2048extras";
    /// `2048extras`'s manual viewer - one tap away, plain `Text`/`Image`.
    pub const EXTRAS_MANUAL: &str = "manual3D";
    /// `2048extras`'s credits reel - one tap away, plain `Text` widgets.
    pub const EXTRAS_CREDITS: &str = "extrasCredits";
}

/// The screens the boot walks and the grids it lands on live in the root's
/// `<LoadXML>` includes, not in the root - and that is this title's shape,
/// not a gap in the reading.
///
/// `NEWGUI/Skin.xml` declares eight colour globals, the inherited `HD_Colours`
/// block and six `<LoadXML>` lines, and **no screen of its own**
/// (`docs/formats/2048-frontend.md`, the file quoted in full). Its first
/// include, `Definition.xml`, declares `overshell` -> `newFEshell` ->
/// `GameModeChoice`/`Home` and pulls in two more: `Bootup_Definition.xml`
/// (`localised="true"`, so really `Bootup_Definition_EU.xml`, holding
/// `Boot Connect`) and `Intro_Definition.xml` (the other four boot screens,
/// `Boot Studio Logo` through `TitleScreen`).
pub mod includes {
    /// The includes the boot follows, spelled as their `SrcRel` attributes
    /// are before any region suffix, in the order the files list them.
    ///
    /// **Nine files: the boot chain, its footer, the two grids and `Home`'s
    /// five destinations.** `Skin.xml`'s five remaining includes are the
    /// in-race, end-race and demo overlays, and `Definition.xml`'s own
    /// `Unlocks_Definition.xml` is a sub-screen this build still does not
    /// draw - no save to read an unlock graph from. Following every include
    /// was measured once: 46 textures into a 1024x6766 sprite sheet, for
    /// screens nothing samples; five more (`Team_`, `Community_`, `Profile_`,
    /// `Options_`, `Extras_`) is the deliberate cost of drawing `Home`'s own
    /// tiles rather than leaving every one of them a "screen not loaded"
    /// note. A list, the same way [`oag_title::FrontEnd::race_box`] names
    /// Pulse's one selection include by file rather than following its
    /// root's whole `LoadXML` list.
    pub const FOLLOWED: &[&str] = &[
        "Definition.xml",
        "Bootup_Definition.xml",
        "Intro_Definition.xml",
        // `TitleScreen`'s legal footer, `DirectEmbed="true"` and
        // `localised="true"`: one bare `<Text>` per territory, embedded
        // into the screen that names it.
        "Legal_Line_Definition.xml",
        "Team_Definition.xml",
        "Community_Definition.xml",
        "Profile_Definition.xml",
        "Options_Definition.xml",
        // `localised="true"`: `Extras_Definition_EU.xml` on this package,
        // the same territory switch `Bootup_Definition.xml` goes through.
        "Extras_Definition.xml",
    ];
    /// The region suffix a `localised="true"` include resolves to.
    ///
    /// **A package fact, not a choice.** `PCSF00007` - the serial under
    /// `data/extracted/vita/PCSF00007` and the only 2048 package this
    /// project has unpacked - is Sony's EU title id, and the EU boot file
    /// is the one the Vita3K capture matched byte for byte
    /// (`Legal_Line_Definition_EU.xml`'s `BOOT_LEGAL_TRADEMARK_FULL_EU`,
    /// `2048-frontend.md`'s "Corroborated on Vita3K" section). A USA or
    /// JP package would need its own serial mapped here; none is extracted.
    pub const LOCALISED_SUFFIX: &str = "_EU";
}

/// Entry names this module reads.
pub mod names {
    /// The front-end root: the colour globals, `HD_Colours`, and the
    /// `LoadXML` list pulling in the rest of `NEWGUI`. 2048's own name for
    /// what [`oag_pulse::names::FRONTEND_ROOT`]/[`oag_hd::names::FRONTEND_ROOT`]
    /// each are on their titles - no `<FEGlobals>` block, unlike either, see
    /// the module docs.
    ///
    /// [`oag_pulse::names::FRONTEND_ROOT`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/pulse/src/lib.rs
    /// [`oag_hd::names::FRONTEND_ROOT`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/frontend.rs
    pub const FRONTEND_ROOT: &str = r"Data\Plugins\Frontend\NEWGUI\Skin.xml";
    /// The boot movie `<Movie name="LogoMovie">` names directly - a real
    /// `ftyp`/`mp42` MP4 container, which `oag-video` does not demux yet
    /// (`docs/formats/2048-frontend.md`'s "Movies are MP4" section). Wiring
    /// this into [`super::BOOT_PROFILE`] still gives a caller the right
    /// entry name to refuse by, rather than skipping the step outright.
    pub const INTRO_MOVIE: &str = r"Data\Videos\intro.mp4";
}

/// The seventeen plugins that carry a language, named exactly as
/// `PSP2/data.psarc`'s own manifest spells their directories - **all
/// lowercase**, unlike Wipeout HD's `Languages\American`-style PascalCase.
/// Read directly off the archive, 2026-09-20 (`cargo run -p oag-assets
/// --example psarc_list -- ... plugins/languages`), 17 present and 17
/// checked - see `docs/formats/2048-frontend.md`'s language-plugin section,
/// which corroborates the count without spelling every entry.
///
/// Listed in the order the archive lists them, alphabetical and therefore
/// not a menu order, the same caveat [`oag_hd::frontend::LANGUAGE_PLUGINS`]
/// carries for the same reason.
///
/// [`oag_hd::frontend::LANGUAGE_PLUGINS`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/frontend.rs
pub const LANGUAGE_PLUGINS: &[&str] = &[
    r"languages\american",
    r"languages\danish",
    r"languages\dutch",
    r"languages\english",
    r"languages\finnish",
    r"languages\french",
    r"languages\german",
    r"languages\italian",
    r"languages\japanese",
    r"languages\korean",
    r"languages\norwegian",
    r"languages\polish",
    r"languages\portuguese",
    r"languages\russian",
    r"languages\spanish",
    r"languages\swedish",
    r"languages\traditionalchinese",
];

/// 2048's boot sequence, as far as this pass read and watched it.
///
/// **`Declared`, not `Measured` - one boot is not [ADR-0025]'s bar.** Wipeout
/// HD needed three independent cold boots on RPCS3 to move from `Declared` to
/// `Measured`; this title has exactly one, on Vita3K, 2026-09-17, and even
/// that one did not catch [`states::BOOT_CONNECT`] or
/// [`states::LOAD_SAVE_BOOTUP`] as distinct frames (both plausibly
/// sub-0.75s). What the one boot did do is corroborate the other three
/// screens in the declared order with content matching the XML exactly - the
/// Studio Liverpool card's own `delay="4.0"`, the real MP4 playing, and
/// `TitleScreen`'s footer text byte for byte - which is why this chain is
/// carried at all rather than withheld the way Wipeout HD's was before its
/// own captures. See `docs/formats/2048-frontend.md`'s "Corroborated on
/// Vita3K, 2026-09-17" section, confidence 80.
///
/// **Ends at [`states::TITLE_SCREEN`], not [`states::GAME_MODE_CHOICE`].**
/// The five rows `2048-frontend.md`'s own boot-chain table names are exactly
/// this chain; `GameModeChoice` is that table's *destination* column on row
/// 5, reached through an undeclared `LOADING...` screen and attract-mode demo
/// this build does not walk yet - see [`states::GAME_MODE_CHOICE`]'s own doc.
///
/// [ADR-0025]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md
pub const BOOT_PROFILE: &oag_title::BootProfile = &oag_title::BootProfile {
    provenance: oag_title::Provenance::Declared,
    chain: &[
        oag_title::BootStep::screen(states::BOOT_CONNECT),
        oag_title::BootStep::screen(states::BOOT_STUDIO_LOGO),
        oag_title::BootStep::playing(states::BOOT_INTRO_MOVIE, names::INTRO_MOVIE),
        oag_title::BootStep::screen(states::LOAD_SAVE_BOOTUP),
        oag_title::BootStep::screen(states::TITLE_SCREEN),
    ],
    // No off-path reel state has been found for this title - the boot-mode
    // strings `game-boot.md` documents (`RaceBox`, `MPStress`, ...) are debug
    // entry points selected off a command-line flag, not a `--reel`-shaped
    // state this build's own mechanism would walk.
    reel: None,
    // 2048's persistent background is `<FE3DCanvas>` (see `TOUCH`), not a
    // looping video - unlike Pulse's `Backdrop.PMF`.
    menu_backdrop: None,
    menu_scene: false,
    // No language-picker screen sits in this chain at all; the 17 plugins in
    // `LANGUAGE_PLUGINS` are read for their `<Font>`/string-table roles, not
    // for a picker screen this boot walks.
    picker_backdrop_parent: None,
    picker_from_loaded_faces: false,
    fallback_globals: &[],
    fallback_images: &[],
};

/// 2048's touch-icon front end. See [`oag_title::TouchFrontEnd`] and the
/// module docs above for why this fills [`oag_title::FrontEnd::touch`]
/// rather than `menu`.
///
/// Every coordinate below was read directly off `NEWGUI/Definition.xml` and
/// `NEWGUI/Team_Definition.xml` in `PSP2/data.psarc`
/// (`data/extracted/vita/PCSF00007/base`), 2026-09-20 - **not** transcribed
/// from `docs/formats/2048-frontend.md`'s own prose, which quotes the shared
/// `140x140` icon size and the button names but not their individual `x`/`y`.
/// Confidence 92, the same cap that page puts on a claim read straight out of
/// the disc's own XML.
pub const TOUCH: &TouchFrontEnd = &TouchFrontEnd {
    // `<Screen name="GameModeChoice" type="GameModeChoice_Screen">`'s four
    // `<TouchButton>`s, in the file's own order - `offline`/`multiplayer`/
    // `adhoc`/`crossplay` by `name`, one row at `y="190"`.
    game_mode_choice: &[
        TouchButton {
            id: "FE_SP_CAMPAIGN",
            x: 167.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        },
        TouchButton {
            id: "FE_MP_CAMPAIGN",
            x: 329.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        },
        TouchButton {
            id: "FE_ADHOC",
            x: 491.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        },
        TouchButton {
            id: "FE_CROSSPLAY",
            x: 653.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        },
    ],
    // `<Screen name="Home" type="Home">`'s five `<TouchButton>`s, in the
    // file's own order. The third and fourth (`FE_PROFILE`, `FE_OPT_PLUS`)
    // are the two of the nine buttons this type carries that author no
    // `name` attribute at all - `idstring` is what `oag_title::TouchButton::id`
    // holds for exactly that reason.
    home: &[
        TouchButton {
            id: "ER_TEAM",
            x: 86.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        },
        TouchButton {
            id: "FE_COMMUNITY",
            x: 248.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        },
        TouchButton {
            id: "FE_PROFILE",
            x: 410.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        },
        TouchButton {
            id: "FE_OPT_PLUS",
            x: 572.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        },
        TouchButton {
            id: "FE_EXTRAS",
            x: 734.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        },
    ],
    // `Team_Definition.xml`'s `teamshell->team` screen, `type="team2048"`.
    team_screen: "team",
    // That screen's `<Model name="ShipModel">`: `<Values OriginX="1400"
    // OriginY="264" ...>`.
    team_model_origin: (1400.0, 264.0),
    // `newFEshell`'s own `<TouchScroll name="FeShellTouchScroll">` nests
    // `<FE3DCanvas>` - see `oag_title::TouchFrontEnd::canvas_screen`'s own
    // doc for why no `<CanvasLabel>` coordinate is carried here.
    canvas_screen: "newFEshell",
    // Recovered, not authored on the widget - see
    // `oag_title::TouchFrontEnd::canvas_texture`'s own doc. Verified present
    // in the base package this session: `cargo run -p oag-assets --example
    // psarc_list -- ... canvasTexture` -> `data/FE/NewImages/canvasTexture.gxt`.
    canvas_texture: r"Data\FE\NewImages\canvasTexture.gxt",
};

/// 2048's front end, as [`oag_title::Title::front_end`] carries it.
///
/// **`menu: None`, `touch: Some(TOUCH)` - the one title in the corpus that
/// fills the second axis and not the first.** See the module docs for why:
/// this title's front end is real and read (its boot chain, its language
/// plugins), but it draws no `FEGlobals`/`<Menu>`/`<HorizMenu>` vocabulary
/// anywhere for [`oag_title::MenuSkin`] to hold.
pub const FRONT_END: &oag_title::FrontEnd = &oag_title::FrontEnd {
    root: names::FRONTEND_ROOT,
    language_plugins: LANGUAGE_PLUGINS,
    menu: None,
    menu_ps2: None,
    touch: Some(TOUCH),
    boot: BOOT_PROFILE,
    // No frame screen in this idiom - `newFEshell` is the touch shell itself,
    // not a `FE Screen`-style rule-and-corner-marks wrapper the way Pulse,
    // Pure and HD each author one.
    menu_frame: None,
    // 2048's team/track pickers are the touch idiom `TOUCH` describes, not
    // the `oag_ui_screens::picker::Layout` dialect this field names a definition
    // file for.
    race_box: None,
    team_select: None,
    track_select: None,
    race_setup: None,
    // Inert either way: `race_box` is `None`, so no picker opens to read
    // this. The real evidence that 2048 previews a mesh on its team screen
    // lives in `TOUCH::team_model_origin` instead - see that field's own doc
    // for why it is not routed through this flag.
    preview_meshes: false,
    // Plain XML (not dictionary-shortened) in the base package's `data.psarc`;
    // `patch-v104/data2.psarc` re-ships only the `Legacy` sibling, so the base
    // copy is the live one. See `docs/ui/endrace-2048.md`.
    endrace_entry: Some(r"Data\Plugins\Frontend\NEWGUI\EndRace_Definition.xml"),
    endrace_style: Some(oag_title::EndRaceStyle::measured(
        oag_title::EndRaceDialect::Touch,
    )),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_boot_chain_ends_at_title_screen() {
        assert_eq!(BOOT_PROFILE.chain.len(), 5);
        assert_eq!(BOOT_PROFILE.start().state, states::BOOT_CONNECT);
        assert_eq!(
            BOOT_PROFILE.chain.last().unwrap().state,
            states::TITLE_SCREEN
        );
        assert_eq!(BOOT_PROFILE.provenance, oag_title::Provenance::Declared);
    }

    #[test]
    fn the_intro_movie_is_the_only_step_that_plays_one() {
        let playing: Vec<_> = BOOT_PROFILE
            .chain
            .iter()
            .filter(|step| step.movie.is_some())
            .collect();
        assert_eq!(playing.len(), 1);
        assert_eq!(playing[0].state, states::BOOT_INTRO_MOVIE);
        assert_eq!(playing[0].movie, Some(names::INTRO_MOVIE));
    }

    #[test]
    fn every_language_plugin_is_lowercase_and_named_once() {
        let mut sorted = LANGUAGE_PLUGINS.to_vec();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "no plugin listed twice");
        assert_eq!(before, 17, "2048-frontend.md counted 17");
        for plugin in LANGUAGE_PLUGINS {
            assert_eq!(
                *plugin,
                plugin.to_lowercase(),
                "2048's own plugin directories are lowercase, unlike HD's"
            );
        }
    }

    #[test]
    fn both_touch_grids_share_one_icon_size() {
        for button in TOUCH.game_mode_choice.iter().chain(TOUCH.home) {
            assert_eq!(button.width, 140.0);
            assert_eq!(button.height, 140.0);
        }
    }

    #[test]
    fn game_mode_choice_has_four_buttons_and_home_has_five() {
        assert_eq!(TOUCH.game_mode_choice.len(), 4);
        assert_eq!(TOUCH.home.len(), 5);
    }
}
