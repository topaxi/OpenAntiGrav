//! The ram, and the `Copy + Eq` property that keeps a [`Driver`] in the world
//! snapshot.
//!
//! One theme of `driver.rs`'s tests. They were an inline `#[cfg(test)]`
//! module of 1,185 lines, which is over both caps in
//! `scripts/check-file-size.py` at once - 200 inline, 1,000 in a file - so
//! they are split by subject, and the fixtures they share stay in
//! [`super`].

use super::*;

/// Over a second of ticks a keen rammer takes its shot, and it goes toward
/// the craft it is level with.
#[test]
fn a_ram_goes_toward_the_craft_alongside() {
    let line = straight_with_corridor();
    let state = craft(Vec3::ZERO, 40.0);
    for (offset, wanted) in [(4.0, Sideshift::Right), (-4.0, Sideshift::Left)] {
        let mut fired = None;
        for phase in 0..600u32 {
            let mut driver = Driver::seeded(5);
            driver.phase = phase;
            let shift = shove(&driver, &state, &line, &alongside(offset), 1.0);
            if shift != Sideshift::None {
                fired = Some(shift);
                break;
            }
        }
        assert_eq!(fired, Some(wanted), "offset {offset}");
    }
}

#[test]
fn a_ram_waits_for_the_physics_own_lockout() {
    let line = straight_with_corridor();
    let mut state = craft(Vec3::ZERO, 40.0);
    state.shift_lockout = 0.5;
    for phase in 0..600u32 {
        let mut driver = Driver::seeded(5);
        driver.phase = phase;
        assert_eq!(
            shove(&driver, &state, &line, &alongside(4.0), 1.0),
            Sideshift::None,
            "shifted while locked out at phase {phase}"
        );
    }
}

/// A ram that puts the rammer into the wall is a bug with a personality.
#[test]
fn a_ram_never_goes_toward_a_corridor_edge_it_has_no_room_for() {
    // A corridor with nothing to the right at all.
    let points: Vec<Vec3> = (0..64)
        .map(|step| Vec3::new(0.0, 0.0, -10.0 * step as f32))
        .collect();
    let corridor = points
        .iter()
        .map(|_| Frame {
            lateral: Vec3::X,
            left: -8.0,
            right: 0.0,
        })
        .collect();
    let pinned = Line::with_corridor(points, corridor);
    let state = craft(Vec3::ZERO, 40.0);
    for phase in 0..600u32 {
        let mut driver = Driver::seeded(5);
        driver.phase = phase;
        assert_eq!(
            shove(&driver, &state, &pinned, &alongside(4.0), 1.0),
            Sideshift::None,
            "shifted into the wall at phase {phase}"
        );
    }
}

#[test]
fn a_driver_with_no_appetite_never_rams() {
    let line = straight_with_corridor();
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
    let line = straight_with_corridor();
    let state = craft(Vec3::ZERO, 40.0);
    for phase in 0..600u32 {
        let mut driver = Driver::seeded(5);
        driver.phase = phase;
        assert_eq!(
            shove(&driver, &state, &line, &Field::EMPTY, 1.0),
            Sideshift::None
        );
    }
}

/// `Driver` lives in the world snapshot, and that is what keeps it there.
#[test]
fn a_driver_stays_copy_and_eq() {
    fn requires<T: Copy + Eq + Default>() {}
    requires::<Driver>();
}
