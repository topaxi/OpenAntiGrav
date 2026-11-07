use super::*;

const GRID: (f32, f32) = (480.0, 272.0);

fn say(id: &str) -> String {
    id.to_string()
}

fn view() -> View {
    View {
        at: 1_800,
        ticks: 18_000,
        built: Some(9_000),
        playing: true,
        speed: 16,
        mismatch: None,
        at_end: false,
    }
}

fn texts(list: &[Draw]) -> Vec<String> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_click_on_the_bar_seeks_to_where_it_lands() {
    let [x, y, w, h] = bar_rect(GRID);
    assert_eq!(
        control_at(GRID, (x + w / 2.0, y + h / 2.0)),
        Some(Control::Bar)
    );
    assert_eq!(tick_at(GRID, x, 18_000), 0);
    assert_eq!(tick_at(GRID, x + w / 2.0, 18_000), 9_000);
    assert_eq!(
        tick_at(GRID, x + w + 50.0, 18_000),
        18_000,
        "past the end is the end"
    );
    assert_eq!(tick_at(GRID, -10.0, 18_000), 0);
    // Taller to a finger than it is drawn.
    assert_eq!(control_at(GRID, (x + 10.0, y - 3.0)), Some(Control::Bar));
}

#[test]
fn every_chip_is_found_where_it_is_drawn_and_none_overlap() {
    for chip in CHIPS {
        let [x, y, w, h] = chip_rect(chip, GRID);
        assert_eq!(control_at(GRID, (x + w / 2.0, y + h / 2.0)), Some(chip));
        assert!(
            x >= 0.0 && x + w <= GRID.0 && y + h <= GRID.1,
            "{chip:?} is on screen"
        );
    }
    for (a, b) in CHIPS.iter().zip(CHIPS.iter().skip(1)) {
        let (a, b) = (chip_rect(*a, GRID), chip_rect(*b, GRID));
        assert!(a[0] + a[2] < b[0]);
    }
    assert_eq!(
        control_at(GRID, (240.0, 100.0)),
        None,
        "the picture is not a control"
    );
}

#[test]
fn the_screen_shows_the_time_the_chips_and_the_built_part() {
    let list = draw_list(&view(), GRID, &say, "");
    let words = texts(&list);
    assert!(words.contains(&"0:30 / 5:00".to_string()), "{words:?}");
    assert!(words.contains(&"OAG_REPLAY_PAUSE".to_string()));
    assert!(words.contains(&"1X".to_string()));
    assert!(!words.iter().any(|w| w == "OAG_REPLAY_OPENING"));
    let mut paused = view();
    paused.playing = false;
    assert!(texts(&draw_list(&paused, GRID, &say, "")).contains(&"OAG_REPLAY_PLAY".to_string()));
    // The built part is its own fill, wider than the played part and narrower than the bar.
    let bar = bar_rect(GRID);
    let widths: Vec<f32> = draw_list(&view(), GRID, &say, "")
        .iter()
        .filter_map(|d| match d {
            Draw::Fill { rect, .. } if rect[1] == bar[1] && rect[3] == bar[3] => Some(rect[2]),
            _ => None,
        })
        .collect();
    assert_eq!(widths.len(), 3, "track, built, played");
    assert!(widths[2] < widths[1] && widths[1] < widths[0], "{widths:?}");
}

#[test]
fn opening_says_so_and_a_mismatch_names_the_second_it_stops_matching() {
    let mut opening = view();
    opening.built = None;
    assert!(
        texts(&draw_list(&opening, GRID, &say, "")).contains(&"OAG_REPLAY_OPENING".to_string())
    );
    let mut stopped = view();
    stopped.mismatch = Some(5_400);
    stopped.at_end = true;
    let words = texts(&draw_list(&stopped, GRID, &say, ""));
    assert!(
        words.contains(&"OAG_REPLAY_MISMATCH".to_string()),
        "{words:?}"
    );
    assert!(
        words.contains(&"OAG_REPLAY_STOPPED".to_string()),
        "a stopped replay says it stops"
    );
    let fixed = |id: &str| {
        if id == strings::MISMATCH {
            "STOPS MATCHING AT %s".into()
        } else {
            id.to_string()
        }
    };
    assert!(
        texts(&draw_list(&stopped, GRID, &fixed, ""))
            .contains(&"STOPS MATCHING AT 1:30".to_string())
    );
}

#[test]
fn the_clock_and_the_speeds() {
    assert_eq!(clock(0), "0:00");
    assert_eq!(clock(5_400), "1:30");
    assert_eq!(clock(60 * 600), "10:00");
    assert_eq!([8, 16, 32, 64].map(speed_label), ["0.5X", "1X", "2X", "4X"]);
}
