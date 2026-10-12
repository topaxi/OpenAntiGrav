use super::*;

const DT: f32 = 1.0 / 60.0;

fn text(gap: &TimeGap) -> Option<String> {
    gap.line().map(|line| line.text.clone())
}

#[test]
fn a_lead_counts_up_until_the_next_craft_makes_the_gap() {
    let mut gap = TimeGap::default();
    let led = GapEvents {
        led_at: Some(10.0),
        gap: None,
    };
    gap.advance(DT, 10.4, led, 8, 1);
    assert_eq!(text(&gap).as_deref(), Some("0.4"));
    assert!(!gap.line().unwrap().icon);
    gap.advance(DT, 12.5, GapEvents::default(), 8, 1);
    assert_eq!(text(&gap).as_deref(), Some("2.5"));
    let made = GapEvents {
        led_at: None,
        gap: Some((2.9, true)),
    };
    gap.advance(DT, 12.9, made, 8, 1);
    let line = gap.line().unwrap();
    assert_eq!(line.text, "- 2.9");
    assert!(line.icon);
}

#[test]
fn a_gap_behind_is_red_with_a_plus_and_flashes_at_five_hertz() {
    let mut gap = TimeGap::default();
    let made = GapEvents {
        led_at: None,
        gap: Some((1.2, false)),
    };
    gap.advance(DT, 30.0, made, 8, 4);
    let first = gap.line().unwrap().clone();
    assert_eq!(first.text, "+ 1.2");
    let mut shades = vec![first.color];
    for _ in 0..30 {
        gap.advance(DT, 30.0, GapEvents::default(), 8, 4);
        shades.push(gap.line().unwrap().color);
    }
    shades.sort_by(|a, b| a.partial_cmp(b).unwrap());
    shades.dedup();
    assert_eq!(shades.len(), 2, "two shades alternate");
}

#[test]
fn a_gap_is_held_three_and_a_half_seconds() {
    let mut gap = TimeGap::default();
    let made = GapEvents {
        led_at: None,
        gap: Some((1.2, false)),
    };
    gap.advance(DT, 30.0, made, 8, 4);
    let mut ticks = 1;
    while gap.line().is_some() {
        gap.advance(DT, 30.0, GapEvents::default(), 8, 4);
        ticks += 1;
        assert!(ticks < 400);
    }
    assert!((209..=213).contains(&ticks), "{ticks}");
}

#[test]
fn a_green_gap_ends_when_the_player_is_no_longer_first() {
    let mut gap = TimeGap::default();
    let made = GapEvents {
        led_at: None,
        gap: Some((1.2, true)),
    };
    gap.advance(DT, 30.0, made, 8, 1);
    assert!(gap.line().is_some());
    gap.advance(DT, 30.0, GapEvents::default(), 8, 2);
    gap.advance(DT, 30.0, GapEvents::default(), 8, 2);
    assert!(gap.line().is_none());
}

#[test]
fn a_lone_craft_and_a_far_gap_show_nothing() {
    let mut gap = TimeGap::default();
    let led = GapEvents {
        led_at: Some(10.0),
        gap: None,
    };
    gap.advance(DT, 10.4, led, 1, 1);
    assert!(gap.line().is_none());
    gap.advance(DT, 41.0, GapEvents::default(), 8, 1);
    assert!(gap.line().is_none(), "thirty seconds on, the count is gone");
}
