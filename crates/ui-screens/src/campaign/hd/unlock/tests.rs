use super::*;
use crate::campaign::{GridSelection, GridSummary};

fn grid(name: &str, flyer: &str, required: u32, earned: u32, locked: bool) -> GridSummary {
    GridSummary {
        name: name.to_string(),
        cell_count: 6,
        max_points: 18,
        required_points: required,
        gold_medals: 0,
        points_earned: earned,
        locked,
        flyer_name: Some(flyer.to_string()),
    }
}

/// `grid0`, `grid1`, `grid2` as a fresh profile reads them: only `grid0`
/// unlocked, and each `RequiredPoints` what it takes to open the next.
fn fresh(at: usize) -> GridSelection {
    let mut model = GridSelection::new(vec![
        grid("grid0", "01_uplift", 10, 0, false),
        grid("grid1", "02_warped", 13, 0, true),
        grid("grid2", "03_frenzy", 16, 0, true),
    ]);
    model.set_index(at);
    model
}

/// RPCS3, `grid0`: `10 MORE POINTS NEEDED TO UNLOCK:` over `warped`.
#[test]
fn an_unlocked_tier_asks_for_its_own_points_and_shows_the_next_grids_logo() {
    let model = fresh(0);
    let note = unlock_box(&model).expect("a box");
    assert_eq!(note.kind, UnlockKind::ToUnlock);
    assert_eq!(note.points, 10);
    assert_eq!(note.logo, Some("02_warped"));
}

/// RPCS3, `grid1` and `grid2`: `10 MORE POINTS NEEDED IN:` over `uplift`,
/// `13 ...` over `warped` - the **previous** tier's requirement and logo.
#[test]
fn a_locked_tier_asks_for_the_previous_tiers_points_and_shows_its_logo() {
    let (one, two) = (fresh(1), fresh(2));
    let note = unlock_box(&one).expect("a box");
    assert_eq!(note.kind, UnlockKind::NeededIn);
    assert_eq!((note.points, note.logo), (10, Some("01_uplift")));
    let note = unlock_box(&two).expect("a box");
    assert_eq!((note.points, note.logo), (13, Some("02_warped")));
}

/// Points earned come off the figure, which is how the Fury frame on a
/// profile with three points reads 9 where a fresh one reads 12.
#[test]
fn points_already_earned_reduce_the_figure() {
    let mut model = GridSelection::new(vec![
        grid("grid8", "09_Blitzed", 12, 3, false),
        grid("grid9", "10_Impact", 15, 0, true),
    ]);
    model.set_index(0);
    assert_eq!(unlock_box(&model).map(|note| note.points), Some(9));
}

/// **Chosen, not measured**: nothing is left to unlock once the requirement is
/// met, and the last tier has no next grid.
#[test]
fn no_box_when_nothing_is_left_to_unlock() {
    let mut met = GridSelection::new(vec![
        grid("grid0", "01_uplift", 10, 10, false),
        grid("grid1", "02_warped", 13, 0, false),
    ]);
    met.set_index(0);
    assert!(unlock_box(&met).is_none());
    let mut last = GridSelection::new(vec![grid("grid0", "01_uplift", 10, 0, false)]);
    last.set_index(0);
    assert!(unlock_box(&last).is_none());
}

/// `RC_POINT_NEEDED` is what the table ships beside the plural for exactly one
/// point; an empty table answers with the id itself, which says which was
/// asked for.
#[test]
fn one_point_asks_for_the_singular_string() {
    let strings = StringTable::default();
    let one = UnlockBox {
        kind: UnlockKind::NeededIn,
        points: 1,
        logo: None,
    };
    assert_eq!(one.needed_in(&strings), "RC_POINT_NEEDED");
    let many = UnlockBox { points: 4, ..one };
    assert_eq!(many.needed_in(&strings), "RC_POINTS_NEEDED");
}
