//! `oag_hd::frontend::MENU_SKIN`'s strip, against the disc it was read off.
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
//! `menu_skin.rs` checks the title packages against each other and runs
//! everywhere; `menu_layout_ground_truth.rs` checks the two PSP titles' numbers
//! against their own `Skin.xml`. Neither reaches this one, because HD's strip is
//! not a `FEGlobals` value at all - it is a widget inside
//! `MainMenu_Definition.xml`, which no title package reads at runtime.
//!
//! So the constant and the file it was transcribed from are joined here, and
//! nowhere else. Every number `oag_title::MenuStrip` carries is asserted against
//! **every copy of that file on the disc**, because five copies agreeing to the
//! digit is what the confidence 92 in that comment rests on - an assertion
//! against one copy would still pass the day the archives diverge.

use std::path::{Path, PathBuf};

use oag_formats::fexml;

/// The decrypted HD/Fury image, if it is there.
fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// One `<Image>` widget, flattened for comparison: source, position, the size it
/// states, and its tint.
///
/// A tuple rather than a struct because the assertion reads as the file does -
/// and named, because six columns of it is what clippy calls a very complex
/// type.
type Widget<'a> = (&'a str, f32, f32, Option<f32>, Option<f32>, u32);

/// [`Widget`] from a parsed image.
fn widget(image: &oag_game::screen::Image) -> Widget<'_> {
    (
        image.src.as_str(),
        image.x,
        image.y,
        image.width,
        image.height,
        image.color,
    )
}

/// The main menu's definition file, as spelled on the disc.
const MAIN_MENU: &str = r"Data\Plugins\Frontend\Gui\MainMenu_Definition.xml";

/// Every archive's copy of `MAIN_MENU`, as `(archive label, parsed root)`.
///
/// Read per container rather than through `Archives::read_name`, which serves
/// the first archive holding a name and so could only ever see one of them.
/// Which copy the runtime serves is still open - see `hd-frontend.md` - and that
/// is exactly why this reads them all.
///
/// All three fields, in `read_name`'s own search order: `DATA02` is HD's `fe`
/// rather than one of its `extra`, and a sweep that walked `data` and `extra`
/// alone would miss a copy and still look like it had read every archive.
fn copies(archives: &mut oag_assets::Archives) -> Vec<(String, fexml::Node)> {
    let mut out = Vec::new();
    let containers = std::iter::once(&mut archives.data)
        .chain(archives.fe.iter_mut())
        .chain(archives.extra.iter_mut());
    for container in containers {
        if !container.contains(MAIN_MENU) {
            continue;
        }
        let label = container.label().to_string();
        let blob = container.read_entry(MAIN_MENU).unwrap_or_else(|error| {
            panic!("{label} holds {MAIN_MENU} but will not read it: {error}")
        });
        let xml = fexml::text(&blob).unwrap_or_else(|error| panic!("{label}: {error}"));
        out.push((label, fexml::parse(&xml)));
    }
    out
}

/// The `<HorizMenu>` widgets under `node`, deepest last.
fn horiz_menus<'a>(node: &'a fexml::Node, out: &mut Vec<&'a fexml::Node>) {
    if node.name.eq_ignore_ascii_case("HorizMenu") {
        out.push(node);
    }
    for child in &node.children {
        horiz_menus(child, out);
    }
}

/// Every element with this name under `node`, at any depth.
fn count(node: &fexml::Node, name: &str) -> usize {
    let here = usize::from(node.name.eq_ignore_ascii_case(name));
    here + node
        .children
        .iter()
        .map(|child| count(child, name))
        .sum::<usize>()
}

/// The strip in the title package is the widget on the disc, in every copy.
///
/// The three numbers are asserted as the literals the file writes, not as
/// whatever the constant happens to hold, so a transcription slip in either
/// direction fails rather than agreeing with itself.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hds_main_menu_authors_the_strip_the_title_package_carries() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let copies = copies(&mut archives);

    // Five of the seven archives carry this file; the sound-only `DATA01` and
    // `DATA04` carry none. Asserted as a count rather than as "at least one",
    // for the reason the module docs give.
    assert_eq!(
        copies.len(),
        5,
        "archives carrying {MAIN_MENU}: {:?}",
        copies.iter().map(|(label, _)| label).collect::<Vec<_>>()
    );

    let strip = oag_hd::frontend::MENU_SKIN
        .strip
        .expect("HD's table carries a strip");
    for (label, root) in &copies {
        let mut found = Vec::new();
        horiz_menus(root, &mut found);
        assert_eq!(found.len(), 1, "{label}: one strip on the main menu");
        let widget = found[0];
        assert_eq!(widget.attr("name"), Some("Mode"), "{label}");

        let number = |key: &str| -> f32 {
            widget
                .value(key)
                .unwrap_or_else(|| panic!("{label}: the widget states no {key}"))
                .parse()
                .unwrap_or_else(|error| panic!("{label}: {key} is not a number: {error}"))
        };
        assert!((number("x") - 160.0).abs() < f32::EPSILON, "{label}: x");
        assert!((number("y") - 125.0).abs() < f32::EPSILON, "{label}: y");
        assert_eq!(
            widget.value("color"),
            Some("0xff705070"),
            "{label}: the widget's own colour"
        );
        assert_eq!(
            widget.value("align"),
            Some("left"),
            "{label}: no alignment field exists on MenuStrip because of this"
        );

        // What the constant carries, against the same file.
        assert!((strip.x - number("x")).abs() < f32::EPSILON, "{label}");
        assert!((strip.y - number("y")).abs() < f32::EPSILON, "{label}");
        assert_eq!(strip.color, 0xFF70_5070, "{label}");
    }
}

/// The screen has no vertical `<Menu>` at all, which is why `first_row_y` is
/// `None`.
///
/// The other half of the same reading, and the one that would fail quietly:
/// filling `first_row_y` in from some other screen's `y` would leave every test
/// above passing.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hds_main_menu_has_no_vertical_menu_to_read_a_first_row_from() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");

    for (label, root) in copies(&mut archives) {
        assert_eq!(count(&root, "Menu"), 0, "{label}: no <Menu> widget");
        assert_eq!(count(&root, "HorizMenu"), 1, "{label}: exactly one strip");
    }
    assert!(
        oag_hd::frontend::MENU_SKIN.first_row_y.is_none(),
        "a screen with no rows states no first row"
    );
}

/// The strip's colour is not `FEGlobals->TextColor`, on the disc as in the
/// table.
///
/// Pinned because the two are easy to conflate and the wrong one is invisible in
/// a headless test: a strip drawn in `TextColor` is white where the disc says
/// mauve, and nothing but a capture or this assertion would say so.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_strips_colour_is_the_widgets_own_and_not_textcolor() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");

    let root = archives
        .read_name(oag_hd::frontend::names::FRONTEND_ROOT)
        .expect("the front-end root reads");
    let xml = fexml::text(&root).expect("it is text");
    let globals = fexml::parse(&xml);
    let mut text_color = None;
    fn walk(node: &fexml::Node, out: &mut Option<String>) {
        if node.name.eq_ignore_ascii_case("Variable")
            && node.attr("global") == Some("TextColor")
            && let Some(value) = node.value("String")
        {
            *out = Some(value.to_string());
        }
        for child in &node.children {
            walk(child, out);
        }
    }
    walk(&globals, &mut text_color);
    assert_eq!(
        text_color.as_deref(),
        Some("0xFFFFFFFF"),
        "the global this must not be confused with"
    );

    let strip = oag_hd::frontend::MENU_SKIN
        .strip
        .expect("HD's table carries a strip");
    assert_ne!(
        Some(strip.color),
        oag_hd::frontend::MENU_SKIN.text,
        "the widget overrides TextColor, and the table has to keep both"
    );
}

/// Boots the shell alone - no movies, which is the slow half.
///
/// The frame tests need it: what is on screen behind a menu is the parsed
/// screens *and* the sprite sheet built from them, and only a boot has both.
fn shell(image: &Path) -> oag_game::boot::Shell {
    let options = oag_game::boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_game::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-hd-menu-ground-truth"),
        audio_cache: oag_game::boot::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    oag_game::boot::load_shell(&options)
        .expect("HD's front end is wired; see ADR-0025")
        .0
}

/// HD's menus are framed by its own `FE Screen`, read off the disc.
///
/// The numbers asserted here are the file's, and the point of asserting them is
/// that **nothing in this repository holds them**: the title package names the
/// screen and `menu::read_frame` reads the widgets. So this is the test that
/// fails if the reading stops working, and there is no constant it could agree
/// with instead.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hds_menus_are_framed_by_its_own_screen() {
    let Some(image) = image() else { return };
    let shell = shell(&image);

    assert_eq!(
        oag_hd::frontend::FRONT_END.menu_frame,
        Some("FE Screen"),
        "the title package names the screen and nothing more"
    );
    let screen = shell
        .screens
        .by_name("FE Screen")
        .expect("the front-end root carries it");

    // `FEGlobals->HD_BG`, resolved against the archive that actually served the
    // root - which is `DATA00`, where that global is **black**. `DATA06` says
    // white. See `the_hd_palette_is_the_fe_style_and_the_archives_disagree`,
    // which is why this asserts the resolved value rather than a colour.
    assert_eq!(screen.clear, Some(0xFF00_0000), "<ScreenClear> = HD_BG");

    // Two rules and one arrow, all tinted HD_Grey - again the served archive's,
    // `0xff969696`. Ordered as the file writes them, which is also the order
    // they are drawn in.
    let images: Vec<Widget<'_>> = screen.images.iter().map(widget).collect();
    assert_eq!(
        images,
        vec![
            (
                r"Data\FE\Images\line.gtf",
                160.0,
                110.0,
                Some(1600.0),
                Some(8.0),
                0xFF96_9696
            ),
            (
                r"Data\FE\Images\Title_Arrow_HD.gtf",
                160.0,
                73.0,
                Some(32.0),
                Some(32.0),
                0xFF96_9696
            ),
            (
                r"Data\FE\Images\line.gtf",
                160.0,
                975.0,
                Some(1600.0),
                Some(8.0),
                0xFF96_9696
            ),
        ],
        "FE Screen's own widgets"
    );

    // And they reach the screen: the frame is a fill plus three sprites, which
    // needs both textures to have decoded into the sheet. `line.gtf` is an 8x8
    // tile stretched to 1600 wide, so a frame built from the texture's own size
    // instead of the widget's would be 8 pixels of rule and look like nothing.
    let frame = &shell.frame;
    assert!(frame.clear.is_some(), "the clear reaches the frame");
    assert_eq!(frame.marks.len(), 3, "both rules and the arrow are placed");
    let widths: Vec<f32> = frame
        .marks
        .iter()
        .map(|draw| match draw {
            oag_game::frontend::Draw::Sprite { rect, .. } => rect[2],
            other => panic!("a mark is a sprite: {other:?}"),
        })
        .collect();
    assert_eq!(widths, vec![1600.0, 32.0, 1600.0]);
}

/// The `HD_*` palette is the FE style, and the archives declare it differently.
///
/// **The finding this test exists for, and it is the reason no colour in it is
/// a constant anywhere in this repository.** `OPT_FE_STYLE` offers `HD` and
/// `FURY`; the two archives that carry the palette declare the same six globals
/// with different values, and the difference is exactly those two looks - white
/// with teal against black with red. Which one a boot draws is decided by which
/// archive serves the front-end root, and this build serves `DATA00`.
///
/// So a table of HD's colours transcribed out of `DATA06` would be one style
/// hard-coded, and would not even be the style this build shows. Reading them
/// off the served copy at runtime is the whole of the fix, and this asserts the
/// two sides of it: that they really do differ, and that what reaches the frame
/// is the served one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_hd_palette_is_the_fe_style_and_the_archives_disagree() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");

    let palette = |archives: &mut oag_assets::Archives, index: usize| {
        let mut containers = std::iter::once(&mut archives.data)
            .chain(archives.fe.iter_mut())
            .chain(archives.extra.iter_mut());
        let container = containers
            .find(|container| container.label().contains(&format!("DATA{index:02}")))
            .unwrap_or_else(|| panic!("DATA{index:02} is mounted"));
        let blob = container
            .read_entry(oag_hd::frontend::names::FRONTEND_ROOT)
            .unwrap_or_else(|error| panic!("DATA{index:02}: {error}"));
        let xml = fexml::text(&blob).expect("it is text");
        let screens = oag_game::screen::Screens::from_xml(&xml);
        ["HD_BG", "HD_Grey", "HD_Blue"].map(|name| {
            screens
                .globals
                .get(name)
                .map(|value| value.trim().to_string())
        })
    };

    let fury = palette(&mut archives, 0);
    let hd = palette(&mut archives, 6);
    assert_eq!(
        fury,
        [
            Some("0xff000000".to_string()),
            Some("0xff969696".to_string()),
            Some("0xffac0717".to_string()),
        ],
        "DATA00: black, light grey, red - the Fury style"
    );
    assert_eq!(
        hd,
        [
            Some("0xffffffff".to_string()),
            Some("0xFF646464".to_string()),
            Some("0xff8ac0ca".to_string()),
        ],
        "DATA06: white, dark grey, teal - the HD style"
    );
    assert_ne!(fury, hd, "if these ever agree, the finding is gone");

    // And the two globals that are *not* the style, which is why the title
    // package can hold those as values: both archives agree on them.
    let shell = shell(&image);
    assert_eq!(
        shell.screens.globals.get("TextColor").map(String::as_str),
        Some("0xFFFFFFFF")
    );
    assert_eq!(oag_hd::frontend::MENU_SKIN.text, Some(0xFFFF_FFFF));
    assert!(
        oag_hd::frontend::MENU_SKIN.selected.is_none(),
        "the highlight colour is HD_Blue, whose value is the style"
    );

    // What actually reaches the frame is the served archive's, resolved rather
    // than transcribed.
    let frame = &shell.frame;
    let ink = frame.ink.expect("the marks agree, so there is an ink");
    let grey = 150.0 / 255.0;
    assert!(
        (ink[0] - grey).abs() < 0.001
            && (ink[1] - grey).abs() < 0.001
            && (ink[2] - grey).abs() < 0.001
            && (ink[3] - 1.0).abs() < 0.001,
        "the frame's ink is DATA00's HD_Grey, 0x969696: {ink:?}"
    );
}

/// The disc's own five entries, recorded rather than implemented.
///
/// **This build's menu tree is its own** - `docs/architecture/menus.md` - so
/// nothing here asserts that the shipped `menu.toml` has these rows, and it must
/// not: the strip is presentation and the tree is ours. What this pins is the
/// reading, so that "HD's main menu offers five modes" stays a checkable claim
/// about the disc rather than a sentence in a document.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_disc_offers_five_modes_and_says_which() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");

    for (label, root) in copies(&mut archives) {
        let mut found = Vec::new();
        horiz_menus(&root, &mut found);
        let entries: Vec<&str> = found[0]
            .children
            .iter()
            .filter(|child| child.name.eq_ignore_ascii_case("Entry"))
            .filter_map(|child| child.attr("IDString"))
            .collect();
        assert_eq!(
            entries,
            [
                "FE_RC",
                "FE_RACEBOX",
                "FE_ONLINE",
                "FE_OPT_PLUS",
                "FE_RECORDS"
            ],
            "{label}"
        );
    }
}
