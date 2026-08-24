//! The ram, and the `Copy + Eq` property that keeps a [`Driver`] in the world
//! snapshot.
//!
//! One theme of `driver.rs`'s tests. They were an inline `#[cfg(test)]`
//! module of 1,185 lines, which is over both caps in
//! `scripts/check-file-size.py` at once - 200 inline, 1,000 in a file - so
//! they are split by subject, and the fixtures they share stay in
//! [`super`].
//!
//! **These build their own corridor rather than taking `super`'s.** A shift
//! reaches further than the eight units either side that
//! `straight_with_corridor` allows - see [`RAM_CLEARANCE`] for the measurement.
//! On that fixture every one of these tests would pass by never firing at all,
//! which is the shape of a suite that has stopped asking anything.

use super::*;

/// A straight whose corridor is wide enough that the clearance gate is not the
/// thing under test, unless a test says it is.
fn straight_with_room(left: f32, right: f32) -> Line {
    let points: Vec<Vec3> = (0..64)
        .map(|step| Vec3::new(0.0, 0.0, -10.0 * step as f32))
        .collect();
    let corridor = points
        .iter()
        .map(|_| Frame {
            // The line runs along `-Z` and `Body::right` is `orientation * X`,
            // so the driver's right is `+X`, as in `straight_with_corridor`.
            lateral: Vec3::X,
            left,
            right,
        })
        .collect();
    Line::with_corridor(points, corridor)
}

/// A corridor with room to spare either side of the line.
fn wide() -> Line {
    straight_with_room(-20.0, 20.0)
}

/// The first shift a driver takes in six hundred ticks, or `None`.
fn first_shift(state: &ShipState, line: &Line, field: &Field) -> Option<Sideshift> {
    for phase in 0..600u32 {
        let mut driver = Driver::seeded(5);
        driver.phase = phase;
        let shift = shove(&driver, state, line, field, 1.0);
        if shift != Sideshift::None {
            return Some(shift);
        }
    }
    None
}

/// Over a second of ticks a keen rammer takes its shot, and it goes toward
/// the craft it is level with.
#[test]
fn a_ram_goes_toward_the_craft_alongside() {
    let line = wide();
    let state = craft(Vec3::ZERO, 40.0);
    for (offset, wanted) in [(4.0, Sideshift::Right), (-4.0, Sideshift::Left)] {
        assert_eq!(
            first_shift(&state, &line, &alongside(offset)),
            Some(wanted),
            "offset {offset}"
        );
    }
}

#[test]
fn a_ram_waits_for_the_physics_own_lockout() {
    let line = wide();
    let mut state = craft(Vec3::ZERO, 40.0);
    state.shift_lockout = 0.5;
    assert_eq!(
        first_shift(&state, &line, &alongside(4.0)),
        None,
        "shifted while locked out"
    );
}

/// A ram that puts the rammer into the wall is a bug with a personality.
#[test]
fn a_ram_never_goes_toward_a_corridor_edge_it_has_no_room_for() {
    // A corridor with nothing to the right at all.
    let pinned = straight_with_room(-20.0, 0.0);
    let state = craft(Vec3::ZERO, 40.0);
    assert_eq!(
        first_shift(&state, &pinned, &alongside(4.0)),
        None,
        "shifted into the wall"
    );
}

/// **The room that decides is the craft's own, not the line's.**
///
/// The corridor here is twenty units wide either side, so a gate that asks how
/// far the *line* may go this way sees twenty and fires. What the craft has is
/// two, because it is already eighteen units out - which is where drift, a
/// corner or the last shove it took routinely puts it. Measured on `16_Track`
/// before this: a shove fired with 3.13 units of corridor to its left while the
/// craft was already 11.67 units past that edge. See [`RAM_CLEARANCE`].
#[test]
fn a_ram_measures_its_room_from_the_craft_and_not_from_the_line() {
    let line = wide();
    // Eighteen units right of the line, with a rival further right still.
    let pinned = craft(Vec3::new(18.0, 0.0, 0.0), 40.0);
    assert_eq!(
        first_shift(&pinned, &line, &alongside(4.0)),
        None,
        "shifted toward an edge two units away, on a corridor twenty units wide"
    );
    // The same craft, the same corridor, a rival on the side it has room for.
    assert_eq!(
        first_shift(&pinned, &line, &alongside(-4.0)),
        Some(Sideshift::Left),
        "refused a shift with the whole corridor to move into"
    );
}

#[test]
fn a_driver_with_no_appetite_never_rams() {
    let line = wide();
    let state = craft(Vec3::ZERO, 40.0);
    for phase in 0..600u32 {
        let mut driver = Driver::seeded(5);
        driver.phase = phase;
        assert_eq!(
            shove(&driver, &state, &line, &alongside(4.0), 0.0),
            Sideshift::None
        );
    }
}

#[test]
fn a_driver_alongside_nobody_never_rams() {
    let line = wide();
    let state = craft(Vec3::ZERO, 40.0);
    assert_eq!(first_shift(&state, &line, &Field::EMPTY), None);
}

/// `Driver` lives in the world snapshot, and that is what keeps it there.
#[test]
fn a_driver_stays_copy_and_eq() {
    fn requires<T: Copy + Eq + Default>() {}
    requires::<Driver>();
}
