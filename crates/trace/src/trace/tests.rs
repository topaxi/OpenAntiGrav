//! What the trace reader in [`super`] is asserted to do: the capture script's
//! own column list, parsing and round-tripping every optional column group,
//! the errors a malformed row raises, and the summaries taken off a trace.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `trace.rs`: the tests are 639 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

/// Two ticks in exactly the shape `scripts/psp-trace.py` writes: the same
/// header, in the same order, with `%.7g`-shaped values. Hand-authored, not
/// captured - a real trace is derived game data and cannot be committed.
pub(crate) const FIXTURE: &str = "\
tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
stun_timer,timer_2e0,shield,fov_intercept,fov_additive,\
ss_tap_window_l,ss_tap_window_r,ss_shift_l,ss_shift_r,ss_lockout,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,avel_x,avel_y,avel_z,omega_x,omega_y,omega_z,\
cam_right_x,cam_right_y,cam_right_z,cam_up_x,cam_up_y,cam_up_z,\
cam_fwd_x,cam_fwd_y,cam_fwd_z,cam_pos_x,cam_pos_y,cam_pos_z,\
boost_timer,plume_timer,intensity,half_size,engine_on,flare_speed_kmh,speed_ramp,boost_accum
0,0.016683,1,100,0,0,0,0,21.98,0,0,300,0,1.6485,0.25,0,0.2,0,1,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,10,8,-45,0.8,0.25,0.5,3.5,1,79.2,0.3125,0.1875
1,0.016683,1,100,0,0,0,0,22,0,0,296.5,0,1.6575,0.233317,0,0.183317,0,0.983317,1,0,0,0,1,0,0,0,1,10,2.5,-29.63301,0,0,22.1,22.1,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,10,8,-44.6,0.78,0.2666,0.52,3.55,1,79.56,0.3187,0.19
";

/// The same two ticks as a capture taken **before** the angular-velocity and
/// timer columns existed - which is every trace under `data/traces/` today.
///
/// This fixture is the backwards-compatibility contract in one constant. It
/// is not a stale copy of [`FIXTURE`] to be updated alongside it: it is a
/// frozen record of a file format that real, unreproducible captures are
/// already written in, and it must keep parsing unchanged forever.
pub(crate) const LEGACY_FIXTURE: &str = "\
tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed
0,0.016683,1,100,0,0,0,0,21.98,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22
1,0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,10,2.5,-29.63301,0,0,22.1,22.1
";

#[test]
fn the_fixture_header_is_the_capture_script_s_header() {
    // If `scripts/psp_trace_fields.py` grows a column, this is what fails first.
    let header = FIXTURE.lines().next().unwrap();
    assert_eq!(header, COLUMNS.join(","));
}

/// The header this crate expects, read out of the capture script itself.
///
/// The doc comment on [`COLUMNS`] says the list is a transcription of
/// `scripts/psp_trace_fields.py`'s `CRAFT_FIELDS` and `BODY_FIELDS`. A
/// transcription that nothing checks is a copy waiting to drift, and the
/// failure mode is quiet: a column added on one side only makes every trace
/// unreadable, or worse, shifts what a name means. So the script is the
/// source of truth and this reads it.
///
/// The tables moved out of `psp-trace.py` itself when `psp-autopilot.py`
/// started writing the same columns; this test is what noticed.
fn header_the_capture_script_writes() -> Vec<String> {
    const SCRIPT: &str = include_str!("../../../../scripts/psp_trace_fields.py");
    let mut columns = vec!["tick".to_owned()];
    // In the order the capture writes them, which is **not** the order they
    // are declared in: `ENTITY_FIELDS` sits between the craft and the body
    // because that is where both writers splice it in. This list caught the
    // drift when `shield` moved from `CRAFT_FIELDS` to `ENTITY_FIELDS`,
    // which is the whole reason it reads the script rather than a copy.
    for list in [
        "CRAFT_FIELDS = [",
        "ENTITY_FIELDS = [",
        "BODY_FIELDS = [",
        "CAMERA_FIELDS = [",
        "FLARE_FIELDS = [",
    ] {
        let start = SCRIPT
            .find(list)
            .unwrap_or_else(|| panic!("{list} is not in scripts/psp_trace_fields.py"))
            + list.len();
        let body = &SCRIPT[start..];
        let end = body.find("\n]").expect("an unterminated field list");
        // Every entry is `("name", 0x...)`, so the names are exactly the
        // quoted strings inside the list.
        columns.extend(body[..end].split('"').skip(1).step_by(2).map(str::to_owned));
    }
    columns
}

/// The one test that can catch the capture script and this crate drifting
/// apart, which no fixture can: a fixture is written by hand to match
/// whatever this file already says.
#[test]
fn the_column_list_is_the_capture_script_s_own() {
    assert_eq!(header_the_capture_script_writes(), COLUMNS.to_vec());
}

#[test]
fn the_legacy_fixture_is_exactly_the_required_columns() {
    let header = LEGACY_FIXTURE.lines().next().unwrap();
    assert_eq!(header, REQUIRED_COLUMNS.join(","));
}

/// The optional columns are the difference between the two, and every one of
/// them is in the full header. A column added to `COLUMNS` and forgotten in
/// `OPTIONAL_COLUMNS` would be silently unwritable by [`Trace::to_csv`].
#[test]
fn every_column_is_either_required_or_optional() {
    for column in COLUMNS {
        assert!(
            REQUIRED_COLUMNS.contains(&column) ^ OPTIONAL_COLUMNS.contains(&column),
            "{column} is in neither list, or in both"
        );
    }
    assert_eq!(
        COLUMNS.len(),
        REQUIRED_COLUMNS.len() + OPTIONAL_COLUMNS.len()
    );
    for column in ANGULAR_COLUMNS {
        assert!(OPTIONAL_COLUMNS.contains(&column));
    }
    for column in CAMERA_COLUMNS {
        assert!(OPTIONAL_COLUMNS.contains(&column));
    }
}

#[test]
fn a_capture_shaped_csv_parses() {
    let trace = Trace::parse(FIXTURE).unwrap();
    assert_eq!(trace.len(), 2);
    let first = trace.frames[0];
    assert_eq!(first.tick, 0);
    assert_eq!(first.dt, 0.016_683);
    assert_eq!(first.grounded, 1.0);
    assert_eq!(first.throttle, 100.0);
    assert_eq!(first.speed_cached, 21.98);
    assert_eq!(first.row0, Vec3::X);
    assert_eq!(first.up, Vec3::Y);
    assert_eq!(first.forward, Vec3::Z);
    assert_eq!(first.position, Vec3::new(10.0, 2.5, -30.0));
    assert_eq!(first.velocity, Vec3::new(0.0, 0.0, 22.0));
    assert_eq!(trace.frames[1].tick, 1);
}

#[test]
fn columns_are_located_by_name_not_by_position() {
    let reordered = "\
speed,vel_z,vel_y,vel_x,pos_z,pos_y,pos_x,fwd_z,fwd_y,fwd_x,up_z,up_y,up_x,\
right_z,right_y,right_x,speed_cached,airbrake_r,airbrake_l,steer,brake,throttle,\
grounded,dt,tick
22,22,0,0,-30,2.5,10,1,0,0,0,1,0,0,0,1,21.98,0,0,0,0,100,1,0.016683,0
";
    let trace = Trace::parse(reordered).unwrap();
    assert_eq!(
        trace.frames[0],
        Trace::parse(LEGACY_FIXTURE).unwrap().frames[0]
    );
}

/// The whole backwards-compatibility contract: every capture under
/// `data/traces/` predates these columns, none of them can be re-taken
/// without a hand-driven PPSSPP session, and all of them must keep working.
#[test]
fn a_capture_without_the_new_columns_still_parses() {
    let trace = Trace::parse(LEGACY_FIXTURE).unwrap();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace.frames[0].speed_cached, 21.98);
    assert_eq!(trace.frames[0].position, Vec3::new(10.0, 2.5, -30.0));
}

/// Absent, not zero. A zero angular velocity is a claim that the ship was not
/// rotating; a capture that never measured it must not be able to make that
/// claim, because that is exactly how a comparison passes without looking.
#[test]
fn an_absent_column_is_absent_rather_than_zero() {
    let frame = Trace::parse(LEGACY_FIXTURE).unwrap().frames[0];
    assert_eq!(frame.angular_velocity, None);
    assert_eq!(frame.stun_timer, None);
    assert_eq!(frame.timer_2e0, None);
    assert_eq!(frame.camera_position, None);
    assert_ne!(frame.angular_velocity, Some(Vec3::ZERO));
}

/// Writing a legacy trace back out must not grow it a column, or the absence
/// would be laundered into a zero by one round trip through this crate.
#[test]
fn a_legacy_trace_round_trips_without_growing_a_column() {
    let trace = Trace::parse(LEGACY_FIXTURE).unwrap();
    let csv = trace.to_csv();
    assert_eq!(csv.lines().next().unwrap(), REQUIRED_COLUMNS.join(","));
    assert_eq!(Trace::parse(&csv).unwrap(), trace);
    assert_eq!(trace.columns(), REQUIRED_COLUMNS.to_vec());
}

#[test]
fn a_current_trace_round_trips_with_every_column() {
    let trace = Trace::parse(FIXTURE).unwrap();
    let csv = trace.to_csv();
    assert_eq!(csv.lines().next().unwrap(), COLUMNS.join(","));
    assert_eq!(Trace::parse(&csv).unwrap(), trace);
}

#[test]
fn the_new_columns_are_read_when_they_are_there() {
    let text = FIXTURE
        .replacen(
            ",22,22,0,0,0,0,0,0,",
            ",22,22,0.25,-1.5,0.125,0.0625,-0.5,0.03125,",
            1,
        )
        .replacen("21.98,0,0,", "21.98,0.5,0.75,", 1);
    let frame = Trace::parse(&text).unwrap().frames[0];
    assert_eq!(frame.angular_velocity, Some(Vec3::new(0.25, -1.5, 0.125)));
    assert_eq!(frame.angular_rate, Some(Vec3::new(0.0625, -0.5, 0.031_25)));
    assert_eq!(frame.stun_timer, Some(0.5));
    assert_eq!(frame.timer_2e0, Some(0.75));
}

#[test]
fn the_sideshift_timer_columns_are_read_when_they_are_there() {
    let frame = Trace::parse(FIXTURE).unwrap().frames[0];
    assert_eq!(frame.ss_tap_window_l, Some(0.25));
    assert_eq!(frame.ss_tap_window_r, Some(0.0));
    assert_eq!(frame.ss_shift_l, Some(0.2));
    assert_eq!(frame.ss_shift_r, Some(0.0));
    assert_eq!(frame.ss_lockout, Some(1.0));
}

/// The mirror of [`a_trace_without_the_flare_columns_has_no_flare_rather_than_zeroes`]
/// for the sideshift timers: a capture taken before the column existed - every
/// one in `data/traces/` before 2026-08-27 - keeps parsing, with the five
/// columns **absent** rather than zero. A zero `ss_lockout` would be a claim
/// that the original was never locked out, and no capture before this one
/// measured that at all.
#[test]
fn a_trace_without_the_sideshift_columns_has_them_absent_rather_than_zero() {
    let frame = Trace::parse(LEGACY_FIXTURE).unwrap().frames[0];
    assert_eq!(frame.ss_tap_window_l, None);
    assert_eq!(frame.ss_tap_window_r, None);
    assert_eq!(frame.ss_shift_l, None);
    assert_eq!(frame.ss_shift_r, None);
    assert_eq!(frame.ss_lockout, None);
    for column in [
        "ss_tap_window_l",
        "ss_tap_window_r",
        "ss_shift_l",
        "ss_shift_r",
        "ss_lockout",
    ] {
        assert!(!frame.has(column), "{column} reported present");
    }
}

#[test]
fn the_camera_columns_are_read_when_they_are_there() {
    let frame = Trace::parse(FIXTURE).unwrap().frames[0];
    assert_eq!(frame.camera_row0, Some(Vec3::X));
    assert_eq!(frame.camera_up, Some(Vec3::Y));
    assert_eq!(frame.camera_forward, Some(Vec3::Z));
    assert_eq!(frame.camera_position, Some(Vec3::new(10.0, 8.0, -45.0)));
}

#[test]
fn the_flare_columns_are_read_when_they_are_there() {
    let frame = Trace::parse(FIXTURE).unwrap().frames[0];
    let flare = frame.flare.expect("the fixture carries the flare group");
    assert_eq!(flare.boost_timer, 0.8);
    assert_eq!(flare.plume_timer, 0.25);
    assert_eq!(flare.intensity, 0.5);
    assert_eq!(flare.half_size, 3.5);
    assert_eq!(flare.speed_kmh, 79.2);
    assert_eq!(flare.speed_ramp, 0.3125);
    assert_eq!(flare.boost_accumulator, 0.1875);
    assert!(flare.engine_on_is_set());
}

/// The mirror of [`the_legacy_fixture_is_exactly_the_required_columns`]: a
/// capture taken before `--flare` existed must keep parsing, with the group
/// **absent** rather than zeroed - a zero `boost_timer` would be a claim
/// that the ship was not boosting.
#[test]
fn a_trace_without_the_flare_columns_has_no_flare_rather_than_zeroes() {
    let frame = Trace::parse(LEGACY_FIXTURE).unwrap().frames[0];
    assert_eq!(frame.flare, None);
    for column in FLARE_COLUMNS {
        assert!(!frame.has(column), "{column} reported present");
    }
    assert!(
        !Trace::parse(LEGACY_FIXTURE)
            .unwrap()
            .columns()
            .iter()
            .any(|c| FLARE_COLUMNS.contains(c))
    );
}

/// The mirror of [`a_partial_angular_velocity_is_an_error`]: half a flare is
/// an edited file, not an older capture.
#[test]
fn a_partial_flare_group_is_an_error() {
    let text = FIXTURE.replace(",plume_timer", "");
    assert_eq!(
        Trace::parse(&text),
        Err(Error::MissingColumn("plume_timer".to_owned()))
    );
}

/// `engine_on` is an integer field the capture reads as a float, so the
/// value that actually appears in `data/traces/pad0-boost.csv` is the
/// denormal whose bit pattern is `257`. It must read as *set*, and it must
/// survive a round trip - a reader that treated it as noise and zeroed it
/// would silently flip the only bit the column carries.
#[test]
fn the_engine_on_denormal_reads_as_set_and_round_trips() {
    let raw = f32::from_bits(257);
    let text = FIXTURE.replacen(",3.5,1,79.2,", &format!(",3.5,{raw:e},79.2,"), 1);
    let trace = Trace::parse(&text).expect("the edited fixture");
    let flare = trace.frames[0].flare.expect("the flare group");
    assert!(flare.engine_on_is_set(), "{raw:e} must read as engine-on");
    assert_eq!(flare.engine_on.to_bits(), 257);
    assert_eq!(Trace::parse(&trace.to_csv()).unwrap(), trace);
}

/// Two thirds of a vector is a truncated or edited file, not an older
/// capture, and reading it as a rotation about one axis would be an
/// invention.
#[test]
fn a_partial_angular_velocity_is_an_error() {
    // The header check fires before any row is read, so only the header
    // needs editing; the rows' widths never come into it.
    let text = FIXTURE.replace(",avel_z", "");
    assert_eq!(
        Trace::parse(&text),
        Err(Error::MissingColumn("avel_z".to_owned()))
    );
}

/// A basis without its eye is not a pose, and reading one as a camera
/// would silently frame every comparison shot from the wrong place.
#[test]
fn a_partial_camera_pose_is_an_error() {
    let text = FIXTURE.replace(",cam_pos_z", "");
    assert_eq!(
        Trace::parse(&text),
        Err(Error::MissingColumn("cam_pos_z".to_owned()))
    );
}

#[test]
fn a_missing_column_is_an_error_not_a_default() {
    let text = FIXTURE.replacen("speed_cached", "not_the_column", 1);
    assert_eq!(
        Trace::parse(&text),
        Err(Error::MissingColumn("speed_cached".to_owned()))
    );
}

#[test]
fn a_short_row_is_an_error() {
    let text = format!("{FIXTURE}2,0.016683\n");
    assert_eq!(
        Trace::parse(&text),
        Err(Error::Width {
            line: 4,
            expected: COLUMNS.len(),
            found: 2
        })
    );
}

#[test]
fn a_non_numeric_field_names_itself() {
    let text = FIXTURE.replacen("21.98", "nan-ish", 1);
    assert_eq!(
        Trace::parse(&text),
        Err(Error::NotANumber {
            line: 2,
            column: "speed_cached".to_owned(),
            value: "nan-ish".to_owned()
        })
    );
}

#[test]
fn comments_and_blank_lines_are_skipped() {
    let text = format!("# craft at 0x08c00000\n\n{FIXTURE}");
    assert_eq!(Trace::parse(&text).unwrap().len(), 2);
}

#[test]
fn an_empty_file_is_an_error() {
    assert_eq!(Trace::parse(""), Err(Error::Empty));
}

#[test]
fn a_trace_round_trips_through_csv() {
    let trace = Trace::parse(FIXTURE).unwrap();
    assert_eq!(Trace::parse(&trace.to_csv()).unwrap(), trace);
}

#[test]
fn the_summary_measures_what_the_ship_did() {
    let summary = Trace::parse(FIXTURE).unwrap().summary();
    assert_eq!(summary.ticks, 2);
    assert_eq!(summary.positively_oriented, 2, "cross(x, y) = z");
    assert!((summary.displacement - 0.366_99).abs() < 1e-4);
    assert!((summary.dt_mean - 0.016_683).abs() < 1e-6);
}

/// How far the ship is rolled in [`turning`], in radians.
///
/// **Not zero, and not a right angle.** A ship rotating about world `+y` with
/// its own up axis along `+y` records the same three numbers under the local
/// and the world reading, so a level fixture can separate the *sign* question
/// and is blind to the *frame* one. Rolling it puts the rotation axis across
/// all three body axes and separates all four readings.
const ROLL: f32 = 0.5;

/// The recorded yaw rate the captures actually show, in rad/s: holding left
/// gives `+1.51` about row 1 on 199 of 199 ticks, per
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`. Used as the fixture's signal
/// so the numbers in these tests are the size of the real ones.
const YAW_RATE: f32 = 1.51;

/// A ship turning steadily about world `+y` at [`YAW_RATE`], rolled by
/// [`ROLL`], with the angular-velocity column written under `reading`.
///
/// The basis is `Ry(theta) * B` for a fixed rolled triad `B`, so the rotation
/// carrying one tick onto the next is `Ry(omega * dt)` in world space on every
/// tick: the world angular velocity is exactly `(0, YAW_RATE, 0)` and the
/// recorded column is whatever `reading` says that is.
fn turning(ticks: usize, dt: f32, reading: AngularReading) -> Trace {
    let (roll_sin, roll_cos) = ROLL.sin_cos();
    let base = (
        Vec3::new(roll_cos, roll_sin, 0.0),
        Vec3::new(-roll_sin, roll_cos, 0.0),
        Vec3::Z,
    );
    let world = Vec3::new(0.0, YAW_RATE, 0.0);
    Trace {
        frames: (0..ticks)
            .map(|tick| {
                let (sin, cos) = (YAW_RATE * tick as f32 * dt).sin_cos();
                let yaw = |v: Vec3| Vec3::new(v.x * cos + v.z * sin, v.y, -v.x * sin + v.z * cos);
                let rows = (yaw(base.0), yaw(base.1), yaw(base.2));
                Frame {
                    tick: tick as u64,
                    dt,
                    row0: rows.0,
                    up: rows.1,
                    forward: rows.2,
                    angular_velocity: Some(reading.to_recorded(world, rows)),
                    ..Frame::default()
                }
            })
            .collect(),
    }
}

#[test]
fn the_turning_fixture_is_a_positively_oriented_basis_like_a_real_one() {
    for frame in turning(4, 1.0 / 60.0, AngularReading::default()).frames {
        assert!(frame.basis_is_positively_oriented(1e-5));
    }
}

/// The recorded rows differentiate into the rotation that carries one onto
/// the next, which is what makes the new column checkable against the file it
/// arrived in rather than against a simulation.
#[test]
fn the_basis_derivative_recovers_the_rate_the_fixture_turns_at() {
    let trace = turning(4, 1.0 / 60.0, AngularReading::default());
    let measured = trace.frames[0]
        .basis_rotation_rate(&trace.frames[1])
        .expect("a nonzero dt");
    assert!(
        (measured - Vec3::new(0.0, YAW_RATE, 0.0)).length() < 1e-3,
        "{measured}"
    );
}

#[test]
fn a_zero_delta_has_no_rotation_rate_rather_than_an_infinite_one() {
    let frame = Frame::default();
    assert_eq!(frame.basis_rotation_rate(&Frame::default()), None);
}

/// [`AngularReading::ALL`] is in report order rather than declaration order,
/// and [`Trace::angular_readings`] indexes its accumulators against it. A
/// variant added to the enum and forgotten in `ALL` would simply never be
/// scored, and the report would look complete while missing a candidate.
#[test]
fn every_reading_is_in_the_list_exactly_once() {
    for reading in [
        AngularReading::World,
        AngularReading::NegatedWorld,
        AngularReading::Local,
        AngularReading::NegatedLocal,
    ] {
        assert_eq!(
            AngularReading::ALL
                .iter()
                .filter(|r| **r == reading)
                .count(),
            1,
            "{reading:?}"
        );
    }
    assert_eq!(AngularReading::ALL.len(), 4);
}

/// Every reading must be invertible, or seeding a run from a recorded column
/// and writing our own back out would not be the same transformation twice.
/// The same property [`crate::replay::Basis`] is pinned on.
#[test]
fn every_reading_round_trips() {
    let frame = turning(1, 1.0 / 60.0, AngularReading::default()).frames[0];
    let rows = frame.rows();
    let w = Vec3::new(0.3, -1.51, 0.07);
    for reading in AngularReading::ALL {
        let there_and_back = reading.to_world(reading.to_recorded(w, rows), rows);
        assert!(
            (there_and_back - w).length() < 1e-5,
            "{reading:?}: {there_and_back} is not {w}"
        );
    }
}

/// **The measurement the column exists to make.** A capture written under one
/// reading must name that reading and no other, with the three wrong ones an
/// order of magnitude worse - and this is decided by the recording alone, so
/// the first real capture settles both open questions without a run.
#[test]
fn a_capture_names_the_reading_it_was_written_under() {
    for reading in AngularReading::ALL {
        let summary = turning(40, 1.0 / 60.0, reading).summary();
        let fits = summary.angular_readings.expect("40 ticks of it");
        assert_eq!(summary.angular_ticks, 40);
        assert_eq!(fits[0].reading, reading, "{reading:?}: {summary}");
        assert!(
            fits[0].rms_error < 1e-2,
            "{reading:?} should fit itself: {}",
            fits[0].rms_error
        );
        assert!(
            fits[1].rms_error > 10.0 * fits[0].rms_error.max(1e-3),
            "{reading:?} is not distinguished from {:?}: {} vs {}",
            fits[1].reading,
            fits[0].rms_error,
            fits[1].rms_error
        );
        assert!((summary.angular_rms - YAW_RATE).abs() < 1e-3);
    }
}

#[test]
fn a_capture_without_the_column_scores_no_readings_and_says_so() {
    let summary = Trace::parse(LEGACY_FIXTURE).unwrap().summary();
    assert_eq!(summary.angular_ticks, 0);
    assert_eq!(summary.angular_readings, None);
    assert_eq!(summary.best_angular_reading(), None);
    assert_eq!(summary.stunned_ticks, None);
    assert!(summary.to_string().contains("not in this capture"));
}

/// The stun timer is the capture's wall-contact indicator, and a straight-line
/// capture that spent ticks inside the window is a *stunned* ship coasting -
/// not an equilibrium of the engine force law. So the count is in the summary
/// rather than only in a comparison.
#[test]
fn the_summary_counts_the_ticks_the_original_spent_stunned() {
    let mut trace = Trace::parse(FIXTURE).unwrap();
    assert_eq!(trace.summary().stunned_ticks, Some(0));
    trace.frames[1].stun_timer = Some(0.5);
    let summary = trace.summary();
    assert_eq!(summary.stunned_ticks, Some(1));
    assert_eq!(summary.timer_2e0_ticks, Some(0));
    assert!(summary.to_string().contains("armed on 1/2"), "{summary}");
}

/// A synthetic contact tick and a synthetic unmeasurably-slow tick, checked
/// against the mechanical version of the `speed / |velocity|` check that
/// used to be computed ad hoc - see [`Summary::clean_ticks`], which this
/// pins by construction rather than by reproducing a real capture (that
/// reproduction lives in `HANDOVER.md`'s own numbers: 1029/3146 and first
/// contact 171 on the reference lap capture, 186/300 and first contact 186
/// on the standing start, both reproduced exactly once this existed).
#[test]
fn the_cleanliness_report_dates_the_first_contact_and_tallies_the_unmeasurable() {
    let clean = Frame {
        tick: 0,
        velocity: Vec3::new(0.0, 0.0, 10.0),
        speed: 10.0,
        ..Frame::default()
    };
    let contact = Frame {
        tick: 1,
        velocity: Vec3::new(0.0, 0.0, 10.0),
        speed: 8.0,
        ..Frame::default()
    };
    // Below `UNMEASURABLE_SPEED`, and would also fail the raw ratio test -
    // both counts see it, neither excludes it.
    let unmeasurable = Frame {
        tick: 2,
        velocity: Vec3::new(0.0, 0.0, 0.5),
        speed: 100.0,
        ..Frame::default()
    };
    let trace = Trace {
        frames: vec![clean, contact, unmeasurable],
    };

    let summary = trace.summary();
    assert_eq!(summary.first_contact, Some(1), "{summary:?}");
    assert_eq!(summary.clean_ticks, 1, "{summary:?}");
    assert_eq!(summary.unmeasurable_ticks, 1, "{summary:?}");
    assert!(
        summary.to_string().contains("1/3 tick(s), first contact 1"),
        "{summary}"
    );
}

/// A trace with nothing under [`UNMEASURABLE_SPEED`] and nothing that ever
/// diverges is clean start to finish, with no contact to date.
#[test]
fn a_fully_clean_trace_reports_no_first_contact() {
    let frame = Frame {
        velocity: Vec3::new(0.0, 0.0, 10.0),
        speed: 10.0,
        ..Frame::default()
    };
    let trace = Trace {
        frames: vec![frame, frame],
    };

    let summary = trace.summary();
    assert_eq!(summary.first_contact, None, "{summary:?}");
    assert_eq!(summary.clean_ticks, 2, "{summary:?}");
    assert_eq!(summary.unmeasurable_ticks, 0, "{summary:?}");
}

/// A dead stop at tick 0 - `velocity == Vec3::ZERO`, so the ratio is `0/0`
/// (`NaN`, which never compares clean) - must not become `first_contact`.
/// This is exactly the shape a fresh capture's grid start takes, and is
/// why `first_contact` skips ticks below `UNMEASURABLE_SPEED` rather than
/// trusting whatever a near-zero-velocity ratio happens to read.
#[test]
fn a_dead_stop_at_the_start_does_not_poison_the_first_contact_date() {
    let rest = Frame {
        tick: 0,
        velocity: Vec3::ZERO,
        speed: 0.0,
        ..Frame::default()
    };
    let clean = Frame {
        tick: 1,
        velocity: Vec3::new(0.0, 0.0, 10.0),
        speed: 10.0,
        ..Frame::default()
    };
    let contact = Frame {
        tick: 2,
        velocity: Vec3::new(0.0, 0.0, 10.0),
        speed: 8.0,
        ..Frame::default()
    };
    let trace = Trace {
        frames: vec![rest, clean, contact],
    };

    let summary = trace.summary();
    assert_eq!(summary.first_contact, Some(2), "{summary:?}");
    assert_eq!(summary.unmeasurable_ticks, 1, "{summary:?}");
}

#[test]
fn a_left_handed_basis_is_reported_as_such() {
    let frame = Frame {
        forward: -Vec3::Z,
        ..Frame::default()
    };
    assert!(!frame.basis_is_positively_oriented(BASIS_TOLERANCE));
}
