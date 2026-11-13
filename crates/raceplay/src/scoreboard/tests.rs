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

fn with_kills(points: u32, kills: u32) -> Option<Points> {
    Some(Points {
        points,
        kills,
        deaths: 0,
    })
}

/// Equal points and kills: the person (slot 0) is above the opponent, not the later slot as on
/// the HUD's rows. `place` is the sort index, so two rows never share one.
#[test]
fn equal_points_and_kills_put_the_person_above_an_opponent() {
    let crafts = [
        craft(0, 1, with_kills(40, 2)),
        craft(1, 2, with_kills(40, 2)),
    ];
    let board = build(&crafts, None, 0);
    let order: Vec<(u8, u8)> = board.rows.iter().map(|row| (row.slot, row.place)).collect();
    assert_eq!(order, [(0, 1), (1, 2)]);
}

/// The original's bubble sort puts the craft with fewer kills first on equal points
/// (`oag_race::score::final_places`): the person who tied an opponent with more kills is 2nd.
#[test]
fn equal_points_put_fewer_kills_first() {
    let crafts = [
        craft(0, 1, with_kills(177, 4)),
        craft(1, 2, with_kills(177, 1)),
    ];
    let board = build(&crafts, None, 0);
    assert_eq!(board.player().map(|row| row.place), Some(2));
    assert_eq!(board.rows[0].slot, 1);
}

#[test]
fn an_unscored_race_still_orders_by_race_position() {
    let crafts = [craft(0, 2, None), craft(1, 1, None)];
    let board = build(&crafts, None, 0);
    assert!(!board.scored());
    assert_eq!(board.rows[0].slot, 1);
}
