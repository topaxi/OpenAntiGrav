//! `Race::ship_model_matrix_of` and `Race::ship_active`: the slot-indexed
//! accessors `Scene::render`'s per-craft loops use instead of `zip`ping onto
//! [`Race::ship_model_matrices`]' filtered, gap-closed list.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the
//! 200-line rule in `scripts/check-file-size.py`. Shared fixtures live in the
//! parent `tests.rs`.

use super::*;

/// Nothing deactivates a middle slot mid-race today - `Race::start` sets
/// every filled slot active and nothing clears one afterward - so this is set
/// by hand, the shape a future elimination takes.
fn deactivate(race: &mut Race, slot: usize) {
    race.sim.world.ships[slot].active = false;
}

/// `ship_active` reports exactly the slot flipped, and nothing else - the
/// underlying [`oag_gameplay::world::Ship::active`] field it reads is a
/// per-slot flag, not derived from position or count.
#[test]
fn ship_active_reports_only_the_slot_that_went_inactive() {
    let mut race = race_with_a_grid();
    assert_eq!(race.ship_count(), 8);
    for slot in 0..8 {
        assert!(race.ship_active(slot), "slot {slot} starts active");
    }

    deactivate(&mut race, 3);

    assert!(!race.ship_active(3));
    for slot in [0, 1, 2, 4, 5, 6, 7] {
        assert!(race.ship_active(slot), "slot {slot} is unaffected");
    }
}

/// `ship_model_matrix_of` is exactly that - **by slot** - so one slot going
/// inactive must not move any other slot's matrix. This is the property the
/// fix in `Scene::render` leans on: every per-craft loop there now walks
/// `0..ship_count()` and reads this rather than `ship_model_matrices()`.
#[test]
fn ship_model_matrix_of_does_not_move_when_another_slot_goes_inactive() {
    let mut race = race_with_a_grid();
    let before: Vec<Mat4> = (0..8).map(|slot| race.ship_model_matrix_of(slot)).collect();

    deactivate(&mut race, 3);

    for (slot, matrix) in before.into_iter().enumerate() {
        assert_eq!(
            race.ship_model_matrix_of(slot),
            matrix,
            "slot {slot}'s own matrix must not change because slot 3 went inactive"
        );
    }
}

/// The bug `Scene::render` used to carry, reproduced directly on the data
/// rather than on a GPU-backed `Scene`: [`Race::ship_model_matrices`] filters
/// out an inactive slot and **closes the gap**, so the position that used to
/// hold slot 3's matrix now holds slot 4's. `zip`ping that list onto
/// slot-indexed drawables (`self.ships[3]`, `self.ships[4]`, ...) would have
/// written slot 4's pose into drawable 3 and shifted every craft after it by
/// one - which is exactly why the fix reads [`Race::ship_model_matrix_of`] by
/// slot instead. `Race::ship_model_matrices` itself is unchanged; it still
/// filters, and callers that do not need slot alignment - none left in
/// `Scene::render`, but the accessor is still public - can still use it.
#[test]
fn the_filtered_matrix_list_closes_the_gap_left_by_an_inactive_slot() {
    let mut race = race_with_a_grid();
    deactivate(&mut race, 3);

    let filtered = race.ship_model_matrices();
    assert_eq!(filtered.len(), 7, "one of eight slots is now inactive");

    // Positions before the gap still line up with their own slot.
    for (slot, matrix) in filtered.iter().enumerate().take(3) {
        assert_eq!(*matrix, race.ship_model_matrix_of(slot));
    }
    // Position 3 has closed over the gap: it is slot 4's matrix now, not
    // slot 3's - the misalignment a positional `zip` would draw.
    assert_eq!(
        filtered[3],
        race.ship_model_matrix_of(4),
        "position 3 of the filtered list now holds slot 4's matrix"
    );
    assert_ne!(
        filtered[3],
        race.ship_model_matrix_of(3),
        "and it is not slot 3's own matrix, which was dropped by the filter"
    );
}
