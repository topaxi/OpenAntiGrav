use super::*;

fn pulse() -> SplitGap {
    SplitGap::new(oag_pulse::hud::ART.lap_hud.expect("read").gap)
}

fn over(index: usize, player: bool) -> Crossing {
    Crossing {
        index,
        player,
        finished: false,
    }
}

#[test]
fn the_player_over_first_then_an_opponent_makes_a_green_gap() {
    let mut table = pulse();
    let led = table.cross(65.7, over(2, true));
    assert_eq!(led.led_at, Some(65.0));
    let (secs, ahead) = table.cross(68.2, over(2, false)).gap.expect("a gap");
    assert!(ahead);
    assert!(
        (secs - 3.2).abs() < 1e-4,
        "against the whole second: {secs}"
    );
    assert_eq!(table.cross(70.0, over(2, false)).gap, None);
}

#[test]
fn an_opponent_over_first_then_the_player_makes_a_red_gap() {
    let mut table = pulse();
    assert_eq!(table.cross(60.5, over(3, false)), GapEvents::default());
    assert_eq!(table.cross(63.0, over(3, true)).gap, Some((3.0, false)));
}

#[test]
fn a_finishing_crossing_makes_no_gap() {
    let mut table = pulse();
    table.cross(60.0, over(4, false));
    let mut done = over(4, true);
    done.finished = true;
    assert_eq!(table.cross(63.0, done), GapEvents::default());
}

/// HD stamps a lap in thirds: a craft a third of the way round on its first
/// lap already reaches an index, the line itself only from its second
/// crossing, and a title with one split per lap never sees a mid-lap index.
#[test]
fn hd_splits_a_lap_in_thirds_and_pulse_does_not() {
    let hd = SplitGap::new(oag_hd::hud::ART.lap_hud.expect("read").gap);
    assert_eq!(hd.index_of(1, 0.10), None, "the first line is not stamped");
    assert_eq!(hd.index_of(1, 0.40), Some(4), "a third of lap one");
    assert_eq!(hd.index_of(1, 0.70), Some(5), "two thirds of lap one");
    assert_eq!(hd.index_of(2, 0.05), Some(6), "the second crossing");
    assert_eq!(hd.index_of(0, 0.5), None, "not yet on a lap");
    let pulse = pulse();
    assert_eq!(pulse.index_of(1, 0.40), None);
    assert_eq!(
        pulse.index_of(2, 0.40),
        Some(2),
        "one stamp a lap, at the line"
    );
    assert_eq!(pulse.index_of(21, 0.0), None, "past the 21-crossing table");
    let pure = SplitGap::new(oag_pure::hud::ART.lap_hud.expect("read").gap);
    assert_eq!(pure.index_of(5, 0.0), Some(5));
    assert_eq!(pure.index_of(6, 0.0), None, "Pure's table holds six");
}

/// HD's `Craft_RecordSplit` returns first while one craft is live; Pulse's and
/// Pure's lap functions stamp whoever is on the track.
#[test]
fn only_hd_leaves_the_table_unwritten_while_one_craft_is_live() {
    let table = |art: &oag_title::HudArt| SplitGap::new(art.lap_hud.expect("read").gap);
    let hd = table(oag_hd::hud::ART);
    assert!(!hd.stamps_with(1));
    assert!(hd.stamps_with(2));
    for art in [oag_pulse::hud::ART, oag_pure::hud::ART] {
        assert!(table(art).stamps_with(1));
    }
}
