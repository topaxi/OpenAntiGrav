//! What the value marquee is asserted to do.
//!
//! Its own file under the inline-test rule in `scripts/check-file-size.py`;
//! a move, with no change to a single assertion. `use super::*` still reaches
//! every private item, so every test keeps the path it had.

use super::*;

fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 6.0
}

fn fixture(value: &str) -> Menu {
    let definition = oag_ui::menu::Definition::parse(
        &format!(
            r#"
        version = 1
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "choice"
        label = "RENDERER"
        setting = "graphics.renderer"
        values = ["{value}"]
        "#
        ),
        &oag_ui::language::StringTable::default(),
    )
    .expect("parse");
    Menu::new(definition)
}

fn skin() -> Skin {
    Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

/// Two rows, cursor on the first: `RENDERER` (the second) overflows
/// while nothing has it focused - the case the machine this was written
/// on hit, where the cursor sits on a nearer row while the adapter
/// name's own row stays broken.
fn unfocused_fixture(value: &str) -> Menu {
    let definition = oag_ui::menu::Definition::parse(
        &format!(
            r#"
        version = 1
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "choice"
        label = "FILTERING"
        setting = "graphics.filtering"
        values = ["off"]
        [[page.entry]]
        kind = "choice"
        label = "RENDERER"
        setting = "graphics.renderer"
        values = ["{value}"]
        "#
        ),
        &oag_ui::language::StringTable::default(),
    )
    .expect("parse");
    Menu::new(definition)
}

#[test]
fn a_row_that_fits_is_left_exactly_as_drawn() {
    let menu = fixture("off");
    let before = oag_ui::menu::draw_list(
        &menu,
        &skin(),
        &|_| Vec::new(),
        &measure,
        None,
        &oag_ui::menu::Frame::default(),
        false,
    )
    .flatten();
    let (after, clip) = apply(before.clone(), &menu, &skin(), &measure, 5.0);
    assert_eq!(before, after);
    assert_eq!(clip, None, "nothing to scroll, so nothing to clip either");
}

/// The row this test names is exactly the one the machine's own real
/// adapter name broke: 200 pixels does not fit `vulkan: llvmpipe (LLVM
/// 22.1.8, 256 bits) (cpu)`-length strings, which is the whole reason
/// this module exists.
#[test]
fn an_overflowing_row_slides_and_reports_where_to_clip() {
    let long = "vulkan: a very generationally long laptop gpu name";
    let menu = fixture(long);
    let before = oag_ui::menu::draw_list(
        &menu,
        &skin(),
        &|_| Vec::new(),
        &measure,
        None,
        &oag_ui::menu::Frame::default(),
        false,
    )
    .flatten();
    let (after, clip) = apply(before.clone(), &menu, &skin(), &measure, 5.0);
    assert_ne!(before, after, "an overflowing value has to move");
    let Some((_, left, right)) = clip else {
        panic!("an overflowing value needs a clip window");
    };
    assert!((right - left - MAX_WIDTH).abs() < f32::EPSILON);
}

/// The clip names a draw by index, so a page drawn inside a **frame** has
/// to name the same one.
///
/// The frame's clear and marks go in front of everything in the flattened
/// list, which shifts every index in it. That is safe because the index is
/// computed over the list this returns rather than over some earlier one -
/// and "safe because of how it happens to be written" is exactly the kind of
/// thing that stops being true silently, so it is pinned. Wipeout HD is the
/// only title with a frame today, and every other test in this module passes
/// an empty one.
#[test]
fn a_framed_page_still_clips_the_value_and_not_the_frame() {
    let long = "vulkan: a very generationally long laptop gpu name";
    let menu = fixture(long);
    let framed = oag_ui::menu::Frame {
        clear: Some(Draw::Fill {
            rect: [0.0, 0.0, 480.0, 272.0],
            color: [0.0, 0.0, 0.0, 1.0],
        }),
        marks: vec![Draw::Sprite {
            rect: [10.0, 10.0, 100.0, 8.0],
            uv: [0.0, 0.0, 8.0, 8.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }],
        ink: Some([1.0, 1.0, 1.0, 1.0]),
        blocks: None,
        tab_selected: None,
        settings: None,
    };
    let before = oag_ui::menu::draw_list(
        &menu,
        &skin(),
        &|_| Vec::new(),
        &measure,
        None,
        &framed,
        false,
    )
    .flatten();
    let (after, clip) = apply(before, &menu, &skin(), &measure, 5.0);
    let Some((index, _, _)) = clip else {
        panic!("an overflowing value needs a clip window");
    };
    assert!(
        matches!(&after[index], Draw::Text { text, .. } if text.contains("vulkan")),
        "the clip names the value, not a mark: {:?}",
        after[index]
    );
}

/// The bug this whole module exists to fix: a long adapter name buries
/// its own label whether or not the cursor happens to be sitting on that
/// row. An unfocused overflowing row gets no motion, but it must still
/// stop covering its label.
#[test]
fn an_unfocused_overflowing_row_is_still_clipped_though_not_scrolled() {
    let long = "vulkan: a very generationally long laptop gpu name";
    let menu = unfocused_fixture(long);
    assert_eq!(menu.selected(), 0, "FILTERING, not RENDERER, is focused");
    let before = oag_ui::menu::draw_list(
        &menu,
        &skin(),
        &|_| Vec::new(),
        &measure,
        None,
        &oag_ui::menu::Frame::default(),
        false,
    )
    .flatten();
    let (after, clip) = apply(before.clone(), &menu, &skin(), &measure, 5.0);
    assert_ne!(
        before, after,
        "RENDERER's value still has to stop overflowing"
    );
    assert_eq!(
        clip, None,
        "nothing is scrolling, so there is nothing to clip in the renderer"
    );
    let width: f32 = after
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, scale, .. } if long.ends_with(text.as_str()) => {
                Some(measure(text) * scale)
            }
            _ => None,
        })
        .next()
        .expect("the shortened RENDERER value is still on screen somewhere");
    assert!(width <= MAX_WIDTH, "{width} still overflows {MAX_WIDTH}");
}

/// [`Timer`] starts at zero and only resets when the identity it is
/// handed actually changes.
#[test]
fn the_timer_keeps_running_while_focus_is_unchanged() {
    let mut timer = Timer::default();
    let key = Some(("graphics".to_string(), 2, "vulkan: a gpu".to_string()));
    timer.tick(0.5, key.clone());
    timer.tick(0.5, key);
    assert!((timer.elapsed() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn changing_the_value_under_an_unmoved_cursor_resets_the_timer() {
    let mut timer = Timer::default();
    timer.tick(
        0.5,
        Some(("graphics".to_string(), 2, "vulkan: a gpu".to_string())),
    );
    timer.tick(
        0.5,
        Some(("graphics".to_string(), 2, "vulkan: another gpu".to_string())),
    );
    assert_eq!(
        timer.elapsed(),
        0.5,
        "a new value is a fresh row to read, timed from this tick rather than \
         carrying the old row's clock forward"
    );
}
