//! A scratch census: on the magstrip rows of a Talon's Junction capture, the original's own
//! position jump along up (`pos(t+1) - pos(t) - v(t+1) dt`, a move its velocity does not
//! carry) against the move `oag_physics::maglock`'s hold makes from the same recorded pose.
//!
//! The one-step pose walk in `oag-trace` (`height_miss_census.rs`) cannot run the hold: it
//! needs the race spline's sections, which only this crate loads. This is that half.
//!
//! The blend is ramped along the record from `0` (`maglock::ramp`, `0.2` a frame on contact),
//! so the rows entering and leaving a strip carry the ramp the original had there, as far as
//! our contact probe agrees with its.
//!
//! Prints only. `#[ignore]`d, and returns early unless `OAG_SWEEP` is set:
//!
//! ```text
//! OAG_SWEEP=1 cargo nextest run -p oag-raceplay magstrip_jump_census --run-ignored all --no-capture
//! ```
//!
//! `OAG_MAG_TRACE` chooses the capture (default the clean lap).

use std::path::Path;

use oag_core::math::{Mat3, Quat, Vec3};
use oag_physics::{Body, Environment, ShipState, maglock};
use oag_raceplay as race;

/// The recorded basis as this crate's orientation: row 0 of the capture is the ship's left
/// (`maglock_ground_truth.rs`, `Row::orientation`).
fn orientation(left: Vec3, up: Vec3, forward: Vec3) -> Quat {
    let z = -forward.normalize_or_zero();
    let x = {
        let right = -left.normalize_or_zero();
        (right - z * right.dot(z)).normalize_or_zero()
    };
    let y = z.cross(x);
    assert!((y - up.normalize_or_zero()).length() < 1e-2);
    Quat::from_mat3(&Mat3::from_cols(x, y, z)).normalize()
}

#[test]
#[ignore = "scratch sweep: OAG_SWEEP=1, needs data/images/"]
fn the_hold_against_the_originals_own_jump_on_a_strip() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let trace = std::env::var("OAG_MAG_TRACE")
        .unwrap_or_else(|_| "data/traces/talons-junction-clean-lap.csv".to_string());
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(&trace),
    )
    .expect("the capture");
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().expect("a header").split(',').collect();
    let at = |name: &str| header.iter().position(|c| *c == name).expect(name);
    let (tick, dt) = (at("tick"), at("dt"));
    let (left_x, up_x, fwd_x, pos_x, vel_x) = (
        at("right_x"),
        at("up_x"),
        at("fwd_x"),
        at("pos_x"),
        at("vel_x"),
    );
    let rows: Vec<Vec<f32>> = lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            l.split(',')
                .map(|f| f.parse().unwrap_or(f32::NAN))
                .collect()
        })
        .collect();
    let v3 = |r: &[f32], b: usize| Vec3::new(r[b], r[b + 1], r[b + 2]);

    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        ..race::Options::default()
    })
    .expect("the default track loads");
    let setup = loaded.setup;
    let handling = &setup.handling;

    let mut blend = 0.0f32;
    let mut contact_prev = None;
    println!("tick blend contact rec_jump hold_up hold_height");
    for pair in rows.windows(2) {
        let (row, next) = (&pair[0], &pair[1]);
        let up = v3(row, up_x);
        let mut state = ShipState {
            body: Body {
                position: v3(row, pos_x),
                orientation: orientation(v3(row, left_x), up, v3(row, fwd_x)),
                linear_velocity: v3(row, vel_x),
                mass: handling.physical.mass,
                ..Body::default()
            },
            mag_lock_blend: blend,
            mag_contact: contact_prev,
            ..ShipState::default()
        };
        let nearest = setup.spline.nearest(state.body.position);
        let env = Environment {
            track_sample: nearest.map(|(_, sample, _)| race::Spline::track_sample(sample)),
            track_sample_next: nearest
                .and_then(|(index, _, _)| setup.spline.sample(index + 1))
                .map(race::Spline::track_sample),
            ..Environment::default()
        };
        let contact = maglock::probe(&state, &env, &setup.collision);
        let target = oag_physics::hover::target_height(handling, blend, 0.0);
        let hold = maglock::update(&mut state, &env, contact, target);
        blend = state.mag_lock_blend;
        contact_prev = state.mag_contact;
        let next_up = v3(next, up_x);
        let rec_jump = (v3(next, pos_x) - v3(row, pos_x) - v3(next, vel_x) * row[dt]).dot(next_up);
        if hold.is_some() || rec_jump.abs() > 0.02 {
            println!(
                "MAG,{},{blend:.2},{},{rec_jump:.4},{:.4},{:.4}",
                row[tick],
                u8::from(contact.is_some()),
                hold.map_or(0.0, |h| h.translation.dot(next_up)),
                hold.map_or(0.0, |h| h.height),
            );
        }
    }
}
