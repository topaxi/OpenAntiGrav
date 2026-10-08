use super::*;

fn layout() -> Layout {
    Layout {
        screen: oag_ui::screen::Screen::default(),
        scale: [1.0, 1.0],
        faces: crate::picker::FaceScales::default(),
    }
}

fn authored(name: &str) -> Text {
    Text {
        name: Some(name.to_string()),
        color: TITLE_COLOR,
        ..Text::default()
    }
}

fn parts(draw: Draw) -> (String, [f32; 4]) {
    match draw {
        Draw::Text { text, color, .. } | Draw::FacedText { text, color, .. } => (text, color),
        other => panic!("not text: {other:?}"),
    }
}

#[test]
fn a_heading_draws_grey_and_a_value_white() {
    let (_, heading) = parts(draw(&authored("Event Title"), "Event Type", &layout()));
    let (_, value) = parts(draw(&authored("RC Laps"), "3", &layout()));
    assert_eq!(heading, [150.0 / 255.0, 150.0 / 255.0, 150.0 / 255.0, 1.0]);
    assert_eq!(value, [1.0; 4]);
}

#[test]
fn the_four_emblem_values_and_the_counter_draw_in_capitals() {
    for name in ["Event", "Track", "Speed Class", "Weapons", "GridNum"] {
        let (text, _) = parts(draw(&authored(name), "Single Race", &layout()));
        assert_eq!(text, "SINGLE RACE", "{name}");
    }
    let (text, _) = parts(draw(&authored("RC Laps"), "None", &layout()));
    assert_eq!(text, "None", "a value outside the four keeps its case");
}

#[test]
fn a_colour_the_screen_authors_itself_is_left_alone() {
    let mut text = authored("chooserace");
    text.color = 0xFF12_3456;
    let (_, colour) = parts(draw(&text, "CHOOSE RACE", &layout()));
    assert_eq!(colour[0], 0x12 as f32 / 255.0);
}

#[test]
fn a_title_font_text_draws_in_the_title_face() {
    let mut text = authored("GridNum");
    text.font = "Title".to_string();
    text.scale = 1.0;
    match draw(&text, "Event 01/08", &layout()) {
        Draw::FacedText {
            role, text: shown, ..
        } => {
            assert_eq!(role, "Title");
            assert_eq!(shown, "EVENT 01/08");
        }
        other => panic!("not the title face: {other:?}"),
    }
}
