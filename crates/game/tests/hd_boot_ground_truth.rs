//! Validates Wipeout HD / Fury's front end against its real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! HD's front end was the first one wired from a **declared** boot chain rather
//! than a measured one - see
//! [ADR-0025](../../../docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md).
//! **That ended on 2026-09-05**: three cold boots on RPCS3 walked the chain and
//! `oag_hd::frontend::BOOT` is `Provenance::Measured`.
//!
//! **The distinction this file was built around still holds, and it is the
//! thing to keep straight.** Everything here is a check that the *mechanism*
//! reaches HD's own data - that the skin parses, that the roster is the disc's,
//! that the paths resolve. **None of it is evidence about what a PS3 does**, and
//! the capture that upgraded the provenance did not happen in this file or in
//! any test; it happened in an emulator, and its artefacts are under
//! `data/reference/hd-boot-chain/`. A green run here would look identical if the
//! chain were still only declared, which is exactly why the label lives on the
//! value rather than being inferred from a passing test.
//!
//! The three axes that moved into the title package for this title are each
//! pinned against the disc, because each was a constant in `oag-pulse` that
//! every title reached for and each is now wrong for one of three: the
//! front-end root, the language plugins, and the boot chain itself.

use std::path::{Path, PathBuf};

use oag_game::boot;
use oag_ui::frontend;

/// The decrypted HD/Fury image, if it is there.
fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// Boots the shell alone - no movies, which is the slow half.
fn shell(image: &Path) -> (boot::Shell, oag_assets::Archives) {
    let options = boot::Options {
        // No saved language: this boots a fresh install every time, so the
        // picker is walked rather than skipped.
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-hd-boot-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        // The shell is the half with no movies in it, and every assertion here
        // is about the chain rather than the picture - so nothing below pays for
        // a 1080p transcode.
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (shell, archives, _title) =
        boot::load_shell(&options).expect("HD's front end is wired; see ADR-0025");
    (shell, archives)
}

/// The front end opens at all, which it did not before ADR-0025.
///
/// The whole of what changed is in the first line of this test: `load_shell`
/// used to refuse this source by name.
///
/// **The provenance assertion has flipped since**, on 2026-09-05: three cold boots
/// on RPCS3 walked all eight steps of `oag_hd::frontend::BOOT_CHAIN` in order,
/// savedata moved aside so `FirstPlay` was not skipped. It is asserted here as
/// well as in `oag-hd`'s own test because this is the value that reaches a
/// running boot - a `Shell` built from the disc, not a constant read in place.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_end_opens_on_a_measured_chain() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    assert_eq!(
        shell.profile.provenance,
        oag_title::Provenance::Measured,
        "three cold boots on RPCS3, 2026-09-05; see data/reference/hd-boot-chain/"
    );
    assert!(
        !shell.screens.screens.is_empty(),
        "the skin parsed into screens"
    );
}

/// **The boot no longer says the order is a declaration**, because it is not one.
///
/// This was ADR-0025's load-bearing test and it asserted the opposite: that
/// `load_shell`'s report carried the caveat line, since what replaced the old
/// `front_end: None` guarantee is a label and a label nobody prints is worth
/// nothing. HD was the only title that label ever applied to, and the 2026-09-05
/// capture retired it. So this now guards the other way - **a chain somebody
/// watched must not be presented as one nobody did**, which is the failure mode
/// a stale caveat would leave behind.
///
/// The caveat's *own* correctness moved to a unit test in `oag-game`'s
/// `boot::tests`, which reaches both branches with no disc at all. That is
/// deliberate: with no `Declared` title left on any disc, a disc-backed test
/// cannot cover ADR-0025's mechanism any more.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_boot_report_does_not_call_a_watched_order_a_declaration() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    assert!(
        !shell
            .report
            .iter()
            .any(|line| line.contains("not a boot anyone has watched")),
        "the chain is measured, so no line may call it a declaration: {:#?}",
        shell.report
    );
}

/// The menus offer the roster and the circuits the **disc** declares.
///
/// The fourth axis on the list in this file's own docs, and the one that hid
/// longest: `boot::definitions` asked for `Data\Plugins\PI001\Definition.xml`
/// whatever title it had opened, so on this disc it found nothing, the roster
/// came back empty, and `load_teams` fell through to its eight-team stand-in -
/// a list this crate holds rather than one the disc declares. Eight teams in a
/// menu is what a working front end looks like, which is why nothing failed.
///
/// Twelve is the count `DATA00`'s copy of the definition carries: the eight
/// base teams and Fury's four. See `oag_hd::names::FRONT_END_PLUGIN_DEFINITION`
/// for the five copies and why this one is served.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_menus_offer_the_twelve_teams_this_disc_declares() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    let teams: Vec<&str> = shell.teams.iter().map(|team| team.id.as_str()).collect();
    assert_eq!(
        teams.len(),
        12,
        "the roster should be the disc's twelve, not the eight-team stand-in: {teams:?}"
    );
    // The stand-in was `oag_tables::handling::TEAMS` (now `oag_pulse::race::TEAMS`),
    // the PSP roster, which has no Fury team in it. It is gone rather than fixed - see
    // `roster_declared_ground_truth` - and naming a Fury team directly is still
    // what tells the two cases apart when the count is right for the wrong
    // reason.
    assert!(
        teams.contains(&"Icaras"),
        "a Fury team is missing, so this is a PSP-shaped roster: {teams:?}"
    );
    // The circuits come off the same file, and their count is what tells the
    // five copies apart where the team count cannot: `DATA00` declares 28,
    // `DATA03`/`DATA05`/`DATA06` declare 16 and `DATA02` declares 8, while four
    // of the five agree on twelve teams. All 28 resolve to geometry on this
    // source, so `load_tracks` drops none of them.
    assert_eq!(
        shell.tracks.len(),
        28,
        "28 raceable circuits is `DATA00`'s copy of the definition; 16 or 8          would mean the archive ordering now serves a different one"
    );
}

/// HD's front-end root is a **named** plugin, and it resolves.
///
/// `Data\Plugins\PI001\GUI\Skin.xml` - the constant both PSP titles use, and
/// which every title reached for until this one - is not on this disc at all.
/// Asserted from both ends so that a change to either the constant or the
/// lookup fails here rather than in a boot report nobody reads.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_end_root_is_a_named_plugin_and_the_psp_path_is_absent() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");

    assert_eq!(
        oag_hd::frontend::names::FRONTEND_ROOT,
        r"Data\Plugins\Frontend\Gui\Skin.xml"
    );
    let blob = archives
        .read_name(oag_hd::frontend::names::FRONTEND_ROOT)
        .expect("HD's own front-end root reads");
    assert!(!blob.is_empty());
    assert!(
        archives.read_name(oag_pulse::names::FRONTEND_ROOT).is_err(),
        "the PSP titles' path is not on this disc, which is why root became an axis"
    );
}

/// All sixteen language plugins resolve, and they are named rather than
/// numbered.
///
/// The count is asserted against the *disc* rather than against the constant:
/// `load_languages` drops a plugin whose definition will not read, which is
/// right and is also exactly how a renamed directory would go unnoticed.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_declared_language_plugin_resolves() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let mut report = Vec::new();

    let plugins = oag_hd::frontend::LANGUAGE_PLUGINS;
    assert_eq!(plugins.len(), 16);
    for plugin in plugins {
        assert!(
            plugin.starts_with(r"Languages\"),
            "{plugin} is a named plugin, not one of the PSP titles' PI0xx"
        );
        let name = format!(r"Data\Plugins\{plugin}\Definition.xml");
        assert!(
            archives.read_name(&name).is_ok(),
            "{name} is not on the disc"
        );
    }

    // **All sixteen, and this used to be fifteen.** An inequality here would
    // pass just as well at twelve, which is the "silent truncation reads as
    // covered" shape this project keeps writing rules about - so this is the
    // number that was measured, and a regression fails it.
    //
    // The sixteenth is Portuguese, and what was dropping it was one byte:
    // `Portugu\xeas` is Latin-1, `from_utf8` refused the whole file and the
    // plugin went with it, silently. `oag_ui::xml::expand` falls back to Latin-1
    // now; `all_sixteen_languages_load_including_the_latin_1_one` asserts the
    // name comes out right rather than merely coming out.
    let languages =
        oag_ui::language::load::load_languages(&mut archives, plugins, None, &mut report);
    // Sixteen plugins, then the project's own `PortugueseBR` appended after them
    // (`oag_ui::strings::PROJECT_LANGUAGES`): a deliberate 2026-10-06 change.
    assert_eq!(languages.len(), 16 + 1, "every declared plugin parses");
    assert_eq!(
        languages.last().map(|l| l.name.as_str()),
        Some("PortugueseBR")
    );
    assert!(
        languages
            .iter()
            .any(|language| language.name == "Portuguese"),
        "the Latin-1 plugin is the one this count used to be missing"
    );
    assert!(languages.iter().any(|language| language.name == "English"));

    // **Three native names are wrong on the disc itself**, which is worth an
    // assertion because the obvious reading of `Japanese (Svenska)` in a boot
    // report is that this build mixed two plugins up. It did not:
    // `japanese/definition.xml` literally contains
    // `<Entry ID="Japanese" String="Svenska">`, and Korean and
    // TraditionalChinese carry the same copy-paste. Pinned so that a future
    // encoding fix which *changes* these is noticed rather than welcomed.
    for wrong in ["Japanese", "Korean", "TraditionalChinese"] {
        let language = languages
            .iter()
            .find(|language| language.name == wrong)
            .unwrap_or_else(|| panic!("{wrong} parses"));
        assert_eq!(
            language.native_name, "Svenska",
            "{wrong}'s native name is Swedish on the disc; see hd-frontend.md"
        );
    }
}

/// The chain reaches `Studio Logo`, and the six screens this build cannot drive
/// are stepped over rather than stalled on.
///
/// **The skip is the behaviour under test, not a shortcoming being tolerated.**
/// HD's declared chain has a connection check, three dialogs, an EULA and a save
/// warning in it; none is implemented, and a boot that stopped at the first
/// would look like a hang. `boot::load_shell` filters them and reports each,
/// which is the mechanism ADR-0023 built and this is the first title to lean on
/// it for more steps than it keeps.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_walked_chain_is_the_picker_and_the_logo_and_says_what_it_dropped() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    let walked: Vec<&str> = shell.walked.iter().map(|step| step.state).collect();
    assert_eq!(
        walked,
        vec![
            oag_hd::frontend::states::LANGUAGE_SELECTION,
            oag_hd::frontend::states::STUDIO_LOGO,
        ]
    );
    for skipped in [
        oag_hd::frontend::states::PRE_FMV_CONNECT,
        oag_hd::frontend::states::EPILEPSY_WARNING,
        oag_hd::frontend::states::FIRST_PLAY,
        oag_hd::frontend::states::SAVE_WARNING,
        oag_hd::frontend::states::EULA,
        oag_hd::frontend::states::UPDATE_ANNOUNCEMENT,
    ] {
        assert!(
            shell
                .report
                .iter()
                .any(|line| line.contains(skipped) && line.contains("skipped")),
            "{skipped} was dropped without saying so"
        );
    }
}

/// The one movie step names a `.bik`, and the widget on the screen names one
/// too.
///
/// **They are not the same file, and that is the disc's.** `DATA00`'s
/// `skin.xml` - the copy this build's archive order serves - names
/// `StudioLiverpool_fury.bik`, while the chain constant is `DATA06`'s spelling
/// of the plain reel. Both exist and both decode; which one a PS3 plays folds
/// into which of the six skins is live, which is unread. Pinned so the
/// divergence is visible rather than discovered again.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_logo_step_and_the_logo_widget_both_name_a_bik() {
    let Some(image) = image() else { return };
    let (shell, mut archives) = shell(&image);

    let step = shell
        .profile
        .step(oag_hd::frontend::states::STUDIO_LOGO)
        .expect("the chain carries the logo step");
    let named = step.movie.expect("it plays something");
    assert_eq!(named, oag_hd::frontend::names::STUDIO_LOGO_MOVIE);
    assert!(named.to_ascii_lowercase().ends_with(".bik"));

    let screen = shell
        .screens
        .by_name(oag_hd::frontend::states::STUDIO_LOGO)
        .expect("the skin carries the screen");
    let widget = screen.movies.first().expect("with a movie widget on it");
    // `entry_name` leaves a `src` that already names its container alone, which
    // is what `MOVIE_EXTENSIONS` gaining `.bik` bought: appending `.PMF` here
    // would ask for `StudioLiverpool_fury.bik.PMF`, which is nothing at all.
    // HD's widget already names its own container, so the region argument is
    // never consulted - `oag_ui::screen::DEFAULT_REGION` stands in for it.
    assert!(
        widget
            .entry_name(oag_ui::screen::DEFAULT_REGION)
            .to_ascii_lowercase()
            .ends_with(".bik")
    );

    for name in [named, &widget.entry_name(oag_ui::screen::DEFAULT_REGION)] {
        let blob = archives.read_name(name).expect("the reel reads");
        let header = oag_video::bik::parse(&blob).expect("and parses");
        assert_eq!((header.width, header.height), (1920, 1080));
    }
}

/// HD's widgets are placed in a 1920x1080 grid, not the PSP's 480x272.
///
/// `oag_hd::frontend::MENU_SKIN` carries `menu_x: 800`, which in a 480-wide
/// space is off the right-hand edge - the hazard that module's own doc comment
/// recorded and left for whoever drew these first.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_end_grid_is_1920_by_1080() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    assert_eq!(shell.space.size, (1920.0, 1080.0));
    assert!((shell.space.display_aspect - 16.0 / 9.0).abs() < 1e-6);
    assert_eq!(shell.space, oag_display::space::Space::HD);
    assert!(
        oag_hd::frontend::MENU_SKIN.menu_x < shell.space.size.0,
        "the menu column lands on screen in this grid"
    );
}

/// The skin says which grid it was read in, and it is this disc's own.
///
/// The pair the menus are reconciled from: `oag_ui::menu::Skin::new` uses the
/// *source's* grid to draw in and the *title's* to convert from, and on this
/// title they are the same 1920x1080. A table whose `space` drifted from the
/// grid it was read in would put the label column somewhere plausible and wrong,
/// which is the failure mode the field exists to make impossible.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_menu_skins_own_grid_is_the_one_this_disc_authors_in() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    assert_eq!(oag_hd::frontend::MENU_SKIN.space, (1920.0, 1080.0));
    assert_eq!(shell.menu_skin.space, shell.space.size);
}

/// Wipeout HD's fonts read, and they read **big-endian**.
///
/// Every one of them fell back to the built-in 5x7 face until `fnt::byte_order`
/// existed: the magic at `+0x00` is one 32-bit word, so the PS3 exporter writes
/// it reversed and `\x01FNT` becomes `TNF\x01`. The whole front end drew in
/// debug glyphs, which is legible enough that nothing failed.
///
/// Asserted from both ends. The `.fnt` itself has to sniff big-endian and give
/// back the figures its header states; and `load_font` has to have put that face
/// on the atlas rather than the fallback, which is what the line height pins -
/// the built-in face is 7 pixels tall and `helv.fnt` declares 33.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_end_font_is_a_big_endian_fnt_and_it_is_the_one_loaded() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");

    let name = r"Data\FE\Fonts\helv.fnt";
    let blob = archives.read_name(name).expect("HD's body face reads");
    assert_eq!(
        oag_texture::fnt::byte_order(&blob),
        Some(oag_formats::ByteOrder::Big),
        "a PS3 .fnt is the PSP layout with its words the other way round"
    );
    assert_eq!(&blob[..4], b"TNF\x01", "the magic is a swapped word");

    let font = oag_texture::fnt::Font::parse(&blob).expect("and it parses");
    assert_eq!((font.width, font.height), (1024, 512));
    assert_eq!(font.line_height, 33);
    assert_eq!(font.glyphs.len(), 243);
    // Read from the flag, not from the console: HD's atlases ship linear where
    // the PSP's ship swizzled, so a reader that unswizzled on platform would
    // comb every glyph.
    assert_eq!(font.indices.len(), 1024 * 512);

    let (shell, _) = shell(&image);
    assert!(
        (shell.font.line_height - 33.0).abs() < f32::EPSILON,
        "the boot put the disc's face on the atlas, not the 5x7 fallback: {}",
        shell.font.line_height
    );
}

/// The front end's three `.gtf` images decode, where all three used to fail.
///
/// `sprite::Image::decode` tried the PSP `.mip` parser and then the PS2 one, so
/// a `.gtf` was reported as `zero-sized texture 1281x0` - a complaint about a
/// format the file is not - and the sheet came out 1x1. `oag_texture::gtf` had
/// decoded these since long before the front end asked for one.
///
/// The sizes are the disc's and are what tells a decoded sheet from a plausible
/// one: `line.gtf` is an 8x8 tile stretched to a 1600-pixel rule, so a sheet
/// that merely has *some* width would pass a weaker assertion.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_front_end_image_this_disc_names_decodes() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    let named: Vec<&str> = shell
        .screens
        .screens
        .iter()
        .flat_map(|screen| screen.images.iter())
        .map(|image| image.src.as_str())
        .collect();
    assert!(!named.is_empty(), "the root names images at all");
    for src in &named {
        assert!(
            shell.sprites.get(src).is_some(),
            "{src} is named by a screen and not in the sheet"
        );
        assert!(
            src.to_ascii_lowercase().ends_with(".gtf"),
            "this disc's front-end images are all .gtf: {src}"
        );
    }

    let line = shell
        .sprites
        .get(r"Data\FE\Images\line.gtf")
        .expect("the rule above and below every screen");
    assert_eq!((line.width, line.height), (8, 8));
    let arrow = shell
        .sprites
        .get(r"Data\FE\Images\Title_Arrow_HD.gtf")
        .expect("the title arrow");
    assert_eq!((arrow.width, arrow.height), (32, 32));
}

/// All sixteen languages reach the picker, not fifteen.
///
/// **One Latin-1 byte used to drop one of them silently.** `Portuguese` writes
/// its own name `Portugu\xeas`, which is not valid UTF-8, so `oag_ui::xml::expand`
/// failed, `load_languages` skipped the plugin and the boot reported fifteen
/// with no line saying which had gone. `Spanish` on the same disc is genuinely
/// UTF-8, so the release is mixed-encoding rather than Latin-1.
///
/// The native names are asserted for two of them because the count alone would
/// not notice a plugin that loaded and lost its name.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn all_sixteen_languages_load_including_the_latin_1_one() {
    // (Seventeen entries since 2026-10-06: the disc's sixteen and PortugueseBR.)
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    // Sixteen on the disc and the project's own `PortugueseBR` after them.
    assert_eq!(shell.languages.len(), 16 + 1);
    let native = |name: &str| {
        shell
            .languages
            .iter()
            .find(|language| language.name == name)
            .map(|language| language.native_name.clone())
    };
    assert_eq!(native("Portuguese").as_deref(), Some("Português"));
    assert_eq!(native("Spanish").as_deref(), Some("Español"));

    // **Not a bug on this side of the disc.** Four of the sixteen declare their
    // own name as `Svenska` and `Russian` declares `P??????`; those are the
    // shipped files, checked byte for byte, and a reader that "corrected" them
    // would be inventing. See `docs/formats/hd-frontend.md`.
    assert_eq!(native("Japanese").as_deref(), Some("Svenska"));
}

/// A circuit is named off the copy of the string table that agrees with the
/// circuit list, not off the copy that happens to be mounted first.
///
/// **This test used to assert the opposite**, and the fact it pinned is still
/// true: `Data\Plugins\Frontend\definition.xml` in `DATA00` declares 28
/// circuits keyed `NN_Track`, and the *served* `entries.xml` - `DATA02`'s -
/// keys their names `NN_TRACK` in a different numbering, so folding the case
/// against that copy puts `SEBENCO CLIMB REVERSE` on the circuit that loads
/// `Talons_Junction`. What was missing was the count: HD ships this file five
/// times, four copies carry 24 `NN_TRACK` keys and `DATA06`'s carries 28, and
/// the four the others lack are exactly the Zone circuits `DATA00` declares.
/// So one copy names the whole list, and it is chosen on that.
///
/// Three assertions, because the selection is only right if all three hold:
///
/// 1. **The pairing.** `17_Track` loads `Talons_Junction` and now reads
///    `TALON'S JUNCTION` - the answer the served copy got wrong.
/// 2. **The coverage.** Every one of the 28 resolves, which is the property the
///    copy was chosen for; a drift back to `DATA02`'s copy fails here even if
///    `17_Track` happened to survive.
/// 3. **The corroboration.** Every circuit sharing an environment with another
///    reads the same name as it. That is what makes the chosen copy the *right*
///    one rather than merely the fullest, and it is not the selector: it is
///    false by design on Pulse, where `16_Track` and `32_Track` share an
///    environment and are named separately.
///
/// The served table is left alone and still holds the old numbering, asserted
/// below so this fails rather than passes if the whole table is ever swapped:
/// that would be a much larger change than this one, and it should not happen
/// quietly. See `oag_ui::language::CircuitNames`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_circuit_is_named_off_the_copy_that_agrees_with_the_circuit_list() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    let seventeen = shell
        .tracks
        .iter()
        .find(|track| track.id == "17_Track")
        .expect("this disc declares a seventeenth circuit");
    assert!(
        seventeen.location.contains("Talons_Junction"),
        "the id and the geometry agree: {}",
        seventeen.location
    );
    assert_eq!(
        shell.circuit_names.get(&seventeen.id),
        Some("TALON'S JUNCTION"),
        "the chosen copy names the circuit its own geometry is"
    );

    let unnamed: Vec<&str> = shell
        .tracks
        .iter()
        .filter(|track| shell.circuit_names.get(&track.id).is_none())
        .map(|track| track.id.as_str())
        .collect();
    assert!(
        unnamed.is_empty(),
        "the copy was chosen for naming every circuit; these have no name: {unnamed:?}"
    );

    // Corroboration: one environment, one name. Twelve of HD's environments
    // are declared twice, once each way round.
    let mut pairs = 0;
    for track in &shell.tracks {
        for other in &shell.tracks {
            if other.id <= track.id || other.location != track.location {
                continue;
            }
            pairs += 1;
            assert_eq!(
                shell.circuit_names.get(&track.id),
                shell.circuit_names.get(&other.id),
                "{} and {} are one piece of track and must read as one name",
                track.id,
                other.id
            );
        }
    }
    assert_eq!(pairs, 12, "twelve of HD's circuits are driven both ways");

    // And the served table still holds the numbering that made this necessary,
    // so this test fails rather than quietly passing if that copy is swapped.
    assert_eq!(
        shell.strings.get("17_TRACK"),
        Some("SEBENCO CLIMB REVERSE"),
        "the served copy is untouched and still disagrees"
    );
}

/// The chrome title resolves the `Title` role to `helvb.fnt`, not `helv.fnt`.
///
/// `oag_hd::frontend::MENU_SKIN::title_font` names `"Title"`, which every
/// language plugin on this disc resolves to `helvb.fnt` (see
/// `docs/formats/hd-frontend.md`'s `TitleColor` section) - a distinct,
/// bolder-looking file from `Default`'s `helv.fnt`, both 1024x512 but 44px
/// against 33px. Asserted the same way the body face already is: the `.fnt`
/// itself decodes to the disc's own numbers, and `load_shell` has to have put
/// that atlas on `shell.title_font` rather than leaving it `None`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_chrome_title_resolves_to_the_bold_face_not_the_body_one() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");

    let name = r"Data\FE\Fonts\helvb.fnt";
    let blob = archives.read_name(name).expect("HD's title face reads");
    let font = oag_texture::fnt::Font::parse(&blob).expect("and it parses");
    assert_eq!((font.width, font.height), (1024, 512));
    assert_eq!(font.line_height, 44, "bolder and taller than helv.fnt's 33");

    let (shell, _) = shell(&image);
    let title_font = shell
        .title_font
        .as_ref()
        .expect("MENU_SKIN::title_font names \"Title\" and it reads");
    assert!(
        (title_font.line_height - 44.0).abs() < f32::EPSILON,
        "the chrome title's own atlas, not the 33px body one: {}",
        title_font.line_height
    );
    assert!(
        (title_font.line_height - shell.font.line_height).abs() > f32::EPSILON,
        "helvb and helv must not be the same atlas"
    );
}
