use super::*;

fn over(count: usize, player: bool) -> Crossing {
    Crossing {
        count,
        player,
        finished: false,
    }
}

#[test]
fn the_player_over_first_then_an_opponent_makes_a_green_gap() {
    let mut table = SplitGap::default();
    let led = table.cross(65.7, over(2, true));
    assert_eq!(led.led_at, Some(65.0));
    let (secs, ahead) = table.cross(68.2, over(2, false)).gap.expect("a gap");
    assert!(ahead);
    assert!((secs - 3.2).abs() < 1e-4, "against the whole second: {secs}");
    assert_eq!(table.cross(70.0, over(2, false)).gap, None);
}

#[test]
fn an_opponent_over_first_then_the_player_makes_a_red_gap() {
    let mut table = SplitGap::default();
    assert_eq!(table.cross(60.5, over(3, false)), GapEvents::default());
    assert_eq!(table.cross(63.0, over(3, true)).gap, Some((3.0, false)));
}

#[test]
fn a_finishing_crossing_makes_no_gap() {
    let mut table = SplitGap::default();
    table.cross(60.0, over(4, false));
    let mut done = over(4, true);
    done.finished = true;
    assert_eq!(table.cross(63.0, done), GapEvents::default());
}
