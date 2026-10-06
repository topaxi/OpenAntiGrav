//! What the AI driver in [`super`] is asserted to do, and the fixtures every
//! theme is built from.
//!
//! Split out of `driver.rs` under the 200-line cap on inline `#[cfg(test)]`
//! modules, and by subject under the 1,000-line file cap
//! (`scripts/check-file-size.py`). Themes are named `<theme>_tests.rs` because
//! `scripts/check-transcendentals.py` decides what is test code from the file
//! stem alone, and a fixture corner built with `cos` is only legal under such a
//! stem.

use super::*;
use crate::field::Rival;
use crate::line::Frame;
use oag_core::math::{Quat, Vec3};
use oag_physics::{Body, Sideshift};

mod avoidance_tests;
mod line_following_tests;
mod personality_tests;
mod provocation_tests;
mod ramming_tests;
mod reaction_tests;
mod yielding_tests;

fn placed(place: u8) -> Field {
    Field {
        place,
        ..Field::EMPTY
    }
}

fn stewing(driver: &mut Driver, line: &Line, field: &Field, personality: &Personality) {
    let tuning = Tuning::default();
    driver.stew(
        &Context {
            line,
            tuning: &tuning,
            pilot: &Pilot::BALANCED,
            field,
            yaw_ceiling: None,
            plan: None,
        },
        personality,
    );
}

/// A craft level with this one, in the grid slot given.
///
/// **The slot is a parameter because the ram reads it**: only the player's is a
/// target - see `driver::ram`'s `PLAYER_SLOT`.
fn alongside_slot(slot: u8, offset: f32) -> Field {
    Field {
        alongside: Some(Rival {
            slot,
            gap: 1.0,
            offset,
            closing: 0.0,
            range: offset.abs(),
            cos_bearing: 0.0,
        }),
        ..Field::EMPTY
    }
}

/// The player, level with this craft.
fn alongside(offset: f32) -> Field {
    alongside_slot(0, offset)
}

fn shove(driver: &Driver, state: &ShipState, line: &Line, field: &Field, ram: f32) -> Sideshift {
    let tuning = Tuning::default();
    driver.ram(
        state,
        &Context {
            line,
            tuning: &tuning,
            pilot: &Pilot::BALANCED,
            field,
            yaw_ceiling: None,
            plan: None,
        },
        &Personality {
            ram,
            ..Personality::NEUTRAL
        },
    )
}

fn rival_behind(offset: f32, gap: f32, closing: f32) -> Field {
    Field {
        behind: Some(Rival {
            slot: 2,
            gap: -gap,
            offset,
            closing,
            range: gap,
            cos_bearing: -1.0,
        }),
        ..Field::EMPTY
    }
}

fn across(personality: &Personality, line: &Line, field: &Field) -> f32 {
    let tuning = Tuning::default();
    let aim = line.aim(0, 40.0);
    let lateral = aim.corridor.expect("fixture has a corridor").lateral;
    Driver::seeded(11)
        .drift(
            &aim,
            40.0,
            &Context {
                line,
                tuning: &tuning,
                pilot: &Pilot::BALANCED,
                field,
                yaw_ceiling: None,
                plan: None,
            },
            personality,
        )
        .dot(lateral)
}

fn straight() -> Line {
    Line::new(
        (0..64)
            .map(|step| Vec3::new(0.0, 0.0, -10.0 * step as f32))
            .collect(),
    )
}

/// A craft at the origin pointing down `-Z`, which is [`Body::forward`].
fn craft(position: Vec3, speed: f32) -> ShipState {
    ShipState {
        body: Body {
            position,
            orientation: Quat::IDENTITY,
            linear_velocity: Vec3::new(0.0, 0.0, -speed),
            ..Body::default()
        },
        ..ShipState::default()
    }
}

/// A straight with a corridor 8 units either side of the line.
fn straight_with_corridor() -> Line {
    let points: Vec<Vec3> = (0..64)
        .map(|step| Vec3::new(0.0, 0.0, -10.0 * step as f32))
        .collect();
    let corridor = points
        .iter()
        .map(|_| Frame {
            // The line runs along `-Z` and `Body::right` is `orientation *
            // X`, so the driver's right is `+X`. The disc's own
            // `sample.lateral` points the same way.
            lateral: Vec3::X,
            left: -8.0,
            right: 8.0,
        })
        .collect();
    Line::with_corridor(points, corridor)
}

/// An arc bending to the driver's right, with an even corridor.
///
/// An arc bending to the driver's right, with an even corridor. Built as
/// `tests/closed_loop.rs` builds the oval's (chord direction crossed with world
/// up), legitimate because the fixture is flat.
fn right_hand_corner(radius: f32) -> Line {
    let points: Vec<Vec3> = (0..96)
        .map(|step| {
            let angle = step as f32 * 0.02;
            // Toward `+X`, which is the driver's right on a line running
            // along `-Z`.
            Vec3::new(radius - radius * angle.cos(), 0.0, -radius * angle.sin())
        })
        .collect();
    let corridor = points
        .iter()
        .enumerate()
        .map(|(at, _)| {
            let next = points[(at + 1).min(points.len() - 1)];
            let here = points[at.min(points.len() - 2)];
            let along = (next - here).normalize_or_zero();
            Frame {
                lateral: along.cross(Vec3::Y).normalize_or_zero(),
                left: -8.0,
                right: 8.0,
            }
        })
        .collect();
    Line::with_corridor(points, corridor)
}
