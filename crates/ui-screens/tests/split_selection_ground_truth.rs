//! The PS2 pressing's `Team SelectionSplit` against the disc it is read off:
//! both players' Ship Selects on one authored page, split back into one layout
//! each by [`oag_ui_screens::picker::Layout::read_split`].
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! # What this pins that the unit tests cannot
//!
//! The widget tree is flattened when the XML is parsed, so which half a widget
//! is in is read back from the player index its name carries and from its height
//! for the ones that have none. A hand-written miniature cannot tell whether that
//! rule survives the real file's unnamed rules, bars and the shared title. This
//! checks the two halves hold the same widgets in the same shape, and that the
//! second panel sits where the XML puts it (`Item OffsetY` 181 against the
//! first's -15).

use oag_tables::fexml;
use oag_ui_screens::picker::{FaceScales, Layout};

fn screens() -> Option<oag_ui::screen::Screens> {
    let path = oag_testdata::image("data/images/pulse-ps2-eu.chd")?;
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let raw = archives
        .read_name(r"Data\Plugins\PI001\GUI\Selection_Definition.xml")
        .expect("the PS2 pressing carries the race box's selection screens");
    let xml = fexml::text(&raw).expect("it is text");
    Some(oag_ui::screen::Screens::from_xml(&xml))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_two_halves_are_the_same_screen_twice() {
    let Some(screens) = screens() else {
        return;
    };
    let strings = oag_ui::language::StringTable::default();
    let [first, second] =
        Layout::read_split(&screens, &strings, FaceScales::default(), [640.0, 448.0])
            .expect("the PS2 pressing authors Team SelectionSplit");

    // The XML's own panels: the first set at y 9, the second 195 lower.
    assert_eq!(first.panel[1], 9.0);
    assert_eq!(second.panel[1], 204.0);
    assert_eq!(first.panel[2..], second.panel[2..], "the same panel twice");

    // Each half is the single-player screen's names, with the index gone.
    let names = |layout: &Layout| -> Vec<String> {
        let mut names: Vec<String> = layout
            .screen
            .images
            .iter()
            .filter_map(|image| image.name.clone())
            .chain(
                layout
                    .screen
                    .texts
                    .iter()
                    .filter_map(|text| text.name.clone()),
            )
            .collect();
        names.sort();
        names
    };
    let (a, mut b) = (names(&first), names(&second));
    // The one `Suggest` line (the forced-craft help) is authored once, below the
    // second set, and no screen of this build draws it.
    b.retain(|name| name != "Suggest");
    assert_eq!(a, b, "both halves hold the same named widgets");
    for want in [
        "honey",
        "up arrow",
        "down arrow",
        "Speed Bar",
        "Shield Bar",
        "Infohexgrid",
    ] {
        assert!(a.iter().any(|name| name == want), "{want} in {a:?}");
    }
    assert!(
        a.iter()
            .all(|name| !name.ends_with('0') && !name.ends_with('1')),
        "no player index left: {a:?}"
    );
    assert_eq!(
        first.screen.fills.len(),
        second.screen.fills.len(),
        "the unnamed rules split evenly"
    );

    // One title, on the first.
    assert!(!first.title.is_empty());
    assert!(second.title.is_empty());

    // Each craft goes between its own arrows, clear of the other half's.
    assert!(first.preview[1] + first.preview[3] <= second.preview[1]);
    assert!(first.preview[3] > 50.0 && second.preview[3] > 50.0);
}

/// A layout's widgets as comparable text, sorted: what a half draws, name and rect.
fn widgets(layout: &Layout) -> Vec<String> {
    let screen = &layout.screen;
    let mut lines: Vec<String> = screen
        .fills
        .iter()
        .map(|f| {
            format!(
                "fill {:?} {} {} {:?} {:?} {:#x}",
                f.name, f.x, f.y, f.width, f.height, f.color
            )
        })
        .chain(screen.images.iter().map(|i| {
            format!(
                "image {:?} {} {} {:?} {:?} {:?} {:?} {:?}",
                i.name, i.x, i.y, i.width, i.height, i.src, i.texture_width, i.texture_height
            )
        }))
        .chain(screen.texts.iter().map(|t| {
            format!(
                "text {:?} {} {} {:?} {:?}",
                t.name, t.x, t.y, t.string, t.idstring
            )
        }))
        .collect();
    lines.sort();
    lines
}

/// The PSP borrows this page, built from the numbers in `oag_ui_screens::picker::borrow`.
/// Expressed in the PS2's own grid, they read back as the same two halves the file does:
/// widget for widget, and the same panel and craft rect. Dropping a number from the table,
/// or a widget from its template, fails this.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_borrowed_page_is_the_ps2_authored_one() {
    let Some(screens) = screens() else {
        return;
    };
    let strings = oag_ui::language::StringTable::default();
    let authored = Layout::read_split(&screens, &strings, FaceScales::default(), [640.0, 448.0])
        .expect("the PS2 pressing authors Team SelectionSplit");
    let borrowed = oag_ui_screens::picker::borrow::read_borrowed(
        &screens.globals,
        &strings,
        FaceScales::default(),
        [640.0, 448.0],
    )
    .expect("the numbers read back as a split page");
    for half in 0..2 {
        assert_eq!(
            authored[half].panel, borrowed[half].panel,
            "half {half}'s panel"
        );
        assert_eq!(
            authored[half].preview, borrowed[half].preview,
            "half {half}'s craft"
        );
        let (a, b) = (widgets(&authored[half]), widgets(&borrowed[half]));
        assert!(a.len() > 40, "a real half has widgets: {}", a.len());
        let missing: Vec<_> = a.iter().filter(|line| !b.contains(line)).collect();
        let extra: Vec<_> = b.iter().filter(|line| !a.contains(line)).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "half {half}: the file has and the borrowed lacks {missing:#?}; the borrowed has and the file lacks {extra:#?}"
        );
    }
}
