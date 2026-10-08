//! A page drawn the way the original's settings screen draws it: the rules,
//! the columns, the step arrows and the pointer regions that follow them.
//!
//! The fixture is the shape of Pulse's `Single Player` screen, three rows of
//! it; the disc-backed numbers are `crates/game/tests/pulse_settings_ground_truth.rs`.

use super::*;

use super::blocks::sprites;
use crate::frontend::Placed;
use crate::menu::frame::settings::Layout;
use crate::menu::pointer::{self, Part};

const SCREEN: &str = r#"
<Screen name="Single Player">
<LeftLayer>
<item OffsetX="50" OffsetY="33">
<Image><Values x="0" y="0" width="190" height="1" U="505" V="1" TxtrWidth="1" TxtrHeight="1" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff" src="sheet.mip"></Values></Image>
<Image><Values x="190" y="0" width="190" height="1" U="505" V="1" TxtrWidth="1" TxtrHeight="1" Color1="0xffffffff" Color2="0xffffffff" Color3="0x00ffffff" Color4="0x00ffffff" src="sheet.mip"></Values></Image>
<Image><Values x="0" y="23" width="190" height="1" U="505" V="1" TxtrWidth="1" TxtrHeight="1" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff" src="sheet.mip"></Values></Image>
</item>
<Text><Values idstring="A" font="small" x="50" y="35"></Values></Text>
<List name="A"><Values font="small" x="250" y="35"></Values><Entry idstring="X"></Entry></List>
<List name="B"><Values font="small" x="250" y="58"></Values><Entry idstring="X"></Entry></List>
</LeftLayer>
</Screen>"#;

fn spec() -> oag_title::MenuSettings {
    oag_pulse::FRONT_END
        .menu
        .and_then(|skin| skin.settings)
        .expect("Pulse's skin names a settings screen")
}

fn layout() -> Layout {
    let sheet = (
        spec().sheet.to_string(),
        Placed {
            x: 0,
            y: 0,
            width: 512,
            height: 128,
            quad_extent: None,
            blend: None,
        },
    );
    Layout::read(SCREEN, &Default::default(), spec(), &[sheet]).expect("the fixture reads")
}

fn race_menu() -> Menu {
    Menu::new(
        Definition::parse(
            r#"
            version = 1
            root = "race"
            [[page]]
            id = "race"
            title = "RACE"
            [[page.entry]]
            kind = "choice"
            label = "MODE"
            setting = "race.mode"
            values = ["a", "b"]
            [[page.entry]]
            kind = "choice"
            label = "CLASS"
            setting = "race.class"
            values = ["a", "b"]
            [[page.entry]]
            kind = "action"
            label = "START"
            action = "launch_race"
            "#,
            &crate::language::StringTable::default(),
        )
        .expect("the fixture parses"),
    )
}

fn frame() -> Frame {
    Frame {
        settings: Some(layout()),
        ..Frame::default()
    }
}

fn draw(menu: &Menu, frame: &Frame) -> Vec<Draw> {
    draw_list(menu, &skin(), &no_bindings, &measure, None, frame, false).flatten()
}

#[test]
fn the_numbers_are_the_screens_own() {
    let layout = layout();
    assert_eq!(
        (layout.label_x, layout.value_x, layout.first_y, layout.pitch),
        (50.0, 250.0, 35.0, 23.0)
    );
    assert_eq!(layout.rules.len(), 3, "every <Image> in the <item>");
    assert!(
        matches!(layout.rules[0], Draw::GradientFill { rect, .. } if rect == [50.0, 33.0, 190.0, 1.0]),
        "the item's offset is folded in: {:?}",
        layout.rules[0]
    );
    assert!((layout.band_offset() + 2.0).abs() < f32::EPSILON);
}

#[test]
fn a_page_in_the_list_draws_rules_columns_and_arrows() {
    let menu = race_menu();
    let list = draw(&menu, &frame());

    assert_eq!(
        list.iter()
            .filter(|d| matches!(d, Draw::GradientFill { .. }))
            .count(),
        3,
        "the screen's rules"
    );
    let label = list.iter().find_map(|d| match d {
        Draw::FacedText {
            role, x, y, text, ..
        } if text == "MODE" => Some((*role, *x, *y)),
        _ => None,
    });
    assert_eq!(
        label,
        Some(("Title", 50.0, 35.0)),
        "the label column, in the title's face"
    );
    let value = list.iter().find_map(|d| match d {
        Draw::FacedText { x, y, text, .. } if text.eq_ignore_ascii_case("a") => Some((*x, *y)),
        _ => None,
    });
    assert_eq!(value, Some((250.0, 35.0)), "the value column, left aligned");

    // Two plain arrows on each stepping row, and the halo pair under the
    // focused one's; the action row has none.
    let sprites = sprites(&list);
    let plain = |u: f32| {
        sprites
            .iter()
            .filter(|(_, uv)| uv[0] == u && uv[2] == 9.0)
            .count()
    };
    assert_eq!((plain(289.0), plain(314.0)), (2, 2));
    let halo = sprites.iter().filter(|(_, uv)| uv[2] == 20.0).count();
    assert_eq!(halo, 2, "the focused row's arrows glow, no other row's");
}

#[test]
fn dropping_the_layout_gives_back_the_plain_column() {
    let menu = race_menu();
    let list = draw(&menu, &Frame::default());
    assert!(list.iter().all(|d| !matches!(d, Draw::GradientFill { .. })));
    assert!(sprites(&list).is_empty());
}

#[test]
fn a_page_outside_the_list_is_untouched() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Cross]);
    let list = draw(&menu, &frame());
    assert!(list.iter().all(|d| !matches!(d, Draw::GradientFill { .. })));
}

#[test]
fn the_pointer_follows_the_new_rows_and_the_arrows_step() {
    let menu = race_menu();
    let regions = pointer::regions(&menu, &skin(), &frame(), &measure);
    let layout = layout();
    // Row 1's band: from its rule to the next, 23 high.
    let second = regions
        .iter()
        .find(|r| r.row == 1 && r.part == Part::Row)
        .expect("a region per row");
    assert_eq!(second.rect[1], 58.0 - 2.0);
    assert_eq!(second.rect[3], 23.0);
    // The arrows are regions of their own, on the rows that step.
    let [left, right] = layout.arrow_rects(35.0);
    let at = |rect: [f32; 4]| (rect[0] + 1.0, 40.0);
    assert_eq!(
        hit(&regions, at(left)).map(|r| (r.row, r.part)),
        Some((0, Part::StepBack))
    );
    assert_eq!(
        hit(&regions, at(right)).map(|r| (r.row, r.part)),
        Some((0, Part::StepForward))
    );
    assert!(
        !regions.iter().any(|r| r.row == 2 && r.part != Part::Row),
        "START has nothing to step"
    );
}

use crate::menu::pointer::hit;
