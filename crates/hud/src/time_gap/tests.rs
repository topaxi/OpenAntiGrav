use super::*;

const DT: f32 = 1.0 / 60.0;

fn rule_of(art: &oag_title::HudArt) -> SplitGapRule {
    art.lap_hud.expect("the title's lap routine is read").gap
}

fn pulse() -> TimeGap {
    TimeGap::new(rule_of(oag_pulse::hud::ART))
}

fn table() -> oag_ui::language::StringTable {
    let mut strings = oag_ui::language::StringTable::default();
    strings.merge(
        [
            (
                "HUD_You are %.1f secs ahead".to_string(),
                "You are %.1f secs ahead".to_string(),
            ),
            (
                "HUD_You are %.1f secs behind".to_string(),
                "You are %.1f secs behind".to_string(),
            ),
        ]
        .into(),
    );
    strings
}

fn text(gap: &TimeGap) -> Option<String> {
    gap.line().and_then(|line| line.text(&table()))
}

fn made(gap: f32, ahead: bool) -> GapEvents {
    GapEvents {
        led_at: None,
        gap: Some((gap, ahead)),
    }
}

#[test]
fn a_lead_counts_up_until_the_next_craft_makes_the_gap() {
    let mut gap = pulse();
    let led = GapEvents {
        led_at: Some(10.0),
        gap: None,
    };
    gap.advance(DT, 10.4, led, 8, 1, false);
    assert_eq!(text(&gap).as_deref(), Some("0.4"));
    assert!(!gap.line().unwrap().icon);
    gap.advance(DT, 12.5, GapEvents::default(), 8, 1, false);
    assert_eq!(text(&gap).as_deref(), Some("2.5"));
    gap.advance(DT, 12.9, made(2.9, true), 8, 1, false);
    assert_eq!(text(&gap).as_deref(), Some("- 2.9"));
    assert!(gap.line().unwrap().icon);
}

#[test]
fn a_gap_behind_is_red_with_a_plus_and_flashes_at_five_hertz() {
    let mut gap = pulse();
    gap.advance(DT, 30.0, made(1.2, false), 8, 4, false);
    let first = gap.line().unwrap().clone();
    assert_eq!(text(&gap).as_deref(), Some("+ 1.2"));
    let mut shades = vec![first.color];
    for _ in 0..30 {
        gap.advance(DT, 30.0, GapEvents::default(), 8, 4, false);
        shades.push(gap.line().unwrap().color);
    }
    shades.sort_by(|a, b| a.partial_cmp(b).unwrap());
    shades.dedup();
    assert_eq!(shades.len(), 2, "two shades alternate");
}

#[test]
fn a_gap_is_held_three_and_a_half_seconds() {
    let mut gap = pulse();
    gap.advance(DT, 30.0, made(1.2, false), 8, 4, false);
    let mut ticks = 1;
    while gap.line().is_some() {
        gap.advance(DT, 30.0, GapEvents::default(), 8, 4, false);
        ticks += 1;
        assert!(ticks < 400);
    }
    assert!((209..=213).contains(&ticks), "{ticks}");
}

/// Pulse's `Hud_UpdateTimeDiff` re-reads the player's place into the hold flag
/// of a won gap; HD's and Pure's routines have no such test.
#[test]
fn only_pulses_green_gap_ends_when_the_player_is_no_longer_first() {
    let mut gap = pulse();
    gap.advance(DT, 30.0, made(1.2, true), 8, 1, false);
    assert!(gap.line().is_some());
    gap.advance(DT, 30.0, GapEvents::default(), 8, 2, false);
    gap.advance(DT, 30.0, GapEvents::default(), 8, 2, false);
    assert!(gap.line().is_none());

    for art in [oag_hd::hud::ART, oag_pure::hud::ART] {
        let mut gap = TimeGap::new(rule_of(art));
        gap.advance(DT, 30.0, made(1.2, true), 8, 1, false);
        gap.advance(DT, 30.0, GapEvents::default(), 8, 2, false);
        gap.advance(DT, 30.0, GapEvents::default(), 8, 2, false);
        assert!(gap.line().is_some(), "held through a lost place");
    }
}

#[test]
fn a_lone_craft_and_a_far_gap_show_nothing() {
    let mut gap = pulse();
    let led = GapEvents {
        led_at: Some(10.0),
        gap: None,
    };
    gap.advance(DT, 10.4, led, 1, 1, false);
    assert!(gap.line().is_none());
    gap.advance(DT, 41.0, GapEvents::default(), 8, 1, false);
    assert!(gap.line().is_none(), "thirty seconds on, the count is gone");
}

/// HD's `0x907b0` skips the live-craft count in Time Trial and Speed Lap, so a
/// lone craft sees the count-up from its own crossing; in any other mode it
/// needs a second craft, as on Pulse and Pure.
#[test]
fn hd_counts_up_alone_in_the_timed_modes_only() {
    let led = GapEvents {
        led_at: Some(10.0),
        gap: None,
    };
    let mut hd = TimeGap::new(rule_of(oag_hd::hud::ART));
    hd.advance(DT, 10.4, led, 1, 1, true);
    assert_eq!(text(&hd).as_deref(), Some("0.4"));
    let mut elsewhere = TimeGap::new(rule_of(oag_hd::hud::ART));
    elsewhere.advance(DT, 10.4, led, 1, 1, false);
    assert!(elsewhere.line().is_none());
    let mut pulse = pulse();
    pulse.advance(DT, 10.4, led, 1, 1, true);
    assert!(pulse.line().is_none(), "Pulse needs a second craft");
}

/// Pure words the held gap as a sentence from its own table and counts up bare.
#[test]
fn pure_words_the_gap_as_a_sentence() {
    let mut gap = TimeGap::new(rule_of(oag_pure::hud::ART));
    gap.advance(DT, 30.0, made(2.8, true), 8, 1, false);
    assert_eq!(text(&gap).as_deref(), Some("You are 2.8 secs ahead"));
    let mut gap = TimeGap::new(rule_of(oag_pure::hud::ART));
    gap.advance(DT, 30.0, made(1.2, false), 8, 4, false);
    assert_eq!(text(&gap).as_deref(), Some("You are 1.2 secs behind"));
    let empty = oag_ui::language::StringTable::default();
    assert_eq!(
        gap.line().unwrap().text(&empty),
        None,
        "a table without the phrase draws nothing, not the raw id"
    );
}
