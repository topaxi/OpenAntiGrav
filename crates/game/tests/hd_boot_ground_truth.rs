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
//! HD's front end is the first one wired from a **declared** boot chain rather
//! than a measured one - see
//! [ADR-0025](../../../docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md).
//! Everything here is therefore a check that the *mechanism* reaches HD's own
//! data, and nothing here is evidence about what a PS3 does. The two are easy
//! to confuse, which is why the boot itself prints a line saying which it is and
//! why [`the_boot_report_says_the_order_is_only_declared`] asserts that line
//! exists.
//!
//! The three axes that moved into the title package for this title are each
//! pinned against the disc, because each was a constant in `oag-pulse` that
//! every title reached for and each is now wrong for one of three: the
//! front-end root, the language plugins, and the boot chain itself.

use std::path::{Path, PathBuf};

use oag_game::{boot, frontend};

/// The decrypted HD/Fury image, if it is there.
fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
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
        audio_cache: boot::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        // The shell is the half with no movies in it, and every assertion here
        // is about the chain rather than the picture - so nothing below pays for
        // a 1080p transcode.
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    boot::load_shell(&options).expect("HD's front end is wired; see ADR-0025")
}

/// The front end opens at all, which it did not before ADR-0025.
///
/// The whole of what changed is in the first line of this test: `load_shell`
/// used to refuse this source by name.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_end_opens_on_a_declared_chain() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    assert_eq!(
        shell.profile.provenance,
        oag_title::Provenance::Declared,
        "no capture of a PS3 running this title exists in this project"
    );
    assert!(
        !shell.screens.screens.is_empty(),
        "the skin parsed into screens"
    );
}

/// **The boot says the order is a declaration**, in the report, before anything
/// is drawn.
///
/// The load-bearing test of ADR-0025. What replaced the old `front_end: None`
/// guarantee is a label, and a label nobody prints is worth nothing - so this
/// asserts the print rather than the field, which
/// [`the_front_end_opens_on_a_declared_chain`] already covers.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_boot_report_says_the_order_is_only_declared() {
    let Some(image) = image() else { return };
    let (shell, _) = shell(&image);

    assert!(
        shell
            .report
            .iter()
            .any(|line| line.contains("declares") && line.contains("not a boot anyone has watched")),
        "no line of the report says the order is declared rather than measured: {:#?}",
        shell.report
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

    let languages = boot::load_languages(&mut archives, plugins, &mut report);
    assert!(
        languages.len() >= 15,
        "at least fifteen of the sixteen parse into a language: {}",
        languages.len()
    );
    assert!(languages.iter().any(|language| language.name == "English"));
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
    assert!(widget.entry_name().to_ascii_lowercase().ends_with(".bik"));

    for name in [named, &widget.entry_name()] {
        let blob = archives.read_name(name).expect("the reel reads");
        let header = oag_formats::bik::parse(&blob).expect("and parses");
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
    assert_eq!(shell.space, frontend::Space::HD);
    assert!(
        oag_hd::frontend::MENU_SKIN.menu_x < shell.space.size.0,
        "the menu column lands on screen in this grid"
    );
}
