use super::*;

fn craft(slot: u8, place: u8, points: Option<Points>) -> Craft {
    Craft {
        slot,
        person: slot == 0,
        place,
        lap: 3,
        finish_tick: None,
        best_lap_ticks: None,
        points,
    }
}

fn points(points: u32) -> Option<Points> {
    Some(Points {
        points,
        kills: points / 20,
        deaths: 0,
    })
}

#[test]
fn a_scored_race_is_ordered_by_points_not_by_race_position() {
    // Slot 0 leads the race on the road but is second on points; slot 2 is last on the road.
    let crafts = [
        craft(0, 1, points(177)),
        craft(1, 2, points(129)),
        craft(2, 3, points(202)),
    ];
    let board = build(&crafts, None, 0);
    assert!(board.scored());
    let order: Vec<(u8, u8, u32)> = board
        .rows
        .iter()
        .map(|row| (row.slot, row.place, row.points.map_or(0, |p| p.points)))
        .collect();
    assert_eq!(order, [(2, 1, 202), (0, 2, 177), (1, 3, 129)]);
}

#[test]
fn equal_points_stack_with_the_later_slot_on_top_as_the_hud_rows_do() {
    let crafts = [craft(0, 2, points(40)), craft(1, 1, points(40))];
    let board = build(&crafts, None, 0);
    let slots: Vec<u8> = board.rows.iter().map(|row| row.slot).collect();
    assert_eq!(slots, [1, 0]);
}

#[test]
fn an_unscored_race_still_orders_by_race_position() {
    let crafts = [craft(0, 2, None), craft(1, 1, None)];
    let board = build(&crafts, None, 0);
    assert!(!board.scored());
    assert_eq!(board.rows[0].slot, 1);
}
