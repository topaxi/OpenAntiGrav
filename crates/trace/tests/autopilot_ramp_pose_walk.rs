//! A scratch sweep: one `oag_physics::step` from each recorded row of the
//! original's autopilot over a stretch of track, against the next row.
//!
//! The capture is `scripts/psp-autopilot-trace.py`'s: `psp-trace.py`'s columns
//! plus the rail's blend and the own sample's lateral, so the rail
//! (`Ai_SnapToLane`), which writes velocity and position between the AI update
//! and the world step, can be undone on the next row before it is compared.
//! Each row is seeded with `oag_trace::replay::initial_state`, stepped once
//! with the row's own ramped controls, the autopilot's engine flags and the
//! class's gravity scale, and the stepped velocity and up axis are compared
//! with the next row's. Every tick reseeds, so nothing compounds.
//!
//! Prints only. `#[ignore]`d, and returns early unless `OAG_SWEEP` is set:
//!
//! ```text
//! OAG_SWEEP=1 OAG_RAMP_TRACE=data/traces/<capture>.csv OAG_RAMP_TICKS=1170..1230 \
//!     cargo nextest run -p oag-trace autopilot_ramp_pose_walk --run-ignored all --no-capture
//! ```
//!
//! `OAG_RAMP_TEAM` (default `Assegai`), `OAG_RAMP_CLASS` (default `PHANTOM`) and
//! `OAG_RAMP_TRACK` (default Fort Gale, `14_Track`) choose the rest.

use oag_core::math::Vec3;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::controls::CONTROL_RANGE;
use oag_physics::engine::{ENGINE_AUTOPILOT_SPEEDUP, EngineFlags};
use oag_physics::{Environment, ShipControls, forces, step};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::AngularReading;
use oag_vex::collision;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

/// The driver's raw output (`ai_steer`, `ai_thrust`, `ai_left`, `ai_right`,
/// `ai_pitch`) where the capture carries it, as the controls our step takes.
fn raw_controls(text: &str) -> Option<Vec<ShipControls>> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next()?.split(',').collect();
    let at = |name: &str| header.iter().position(|h| *h == name);
    let sign: f32 = std::env::var("OAG_RAMP_PITCH_SIGN").map_or(1.0, |v| v.parse().unwrap());
    let bias: f32 = std::env::var("OAG_RAMP_BIAS").map_or(0.0, |v| v.parse().unwrap());
    let gated = std::env::var_os("OAG_RAMP_GATE").is_some();
    let hull = [at("hull_up_x")?, at("hull_up_y")?, at("hull_up_z")?];
    let down = [
        at("spline_down_x")?,
        at("spline_down_y")?,
        at("spline_down_z")?,
    ];
    let flags = at("flags1c0")?;
    let cols = [
        at("ai_steer")?,
        at("ai_thrust")?,
        at("ai_left")?,
        at("ai_right")?,
        at("ai_pitch")?,
    ];
    Some(
        lines
            .map(|line| {
                let f: Vec<f32> = line.split(',').map(|v| v.parse().unwrap()).collect();
                ShipControls {
                    steer_x: f[cols[0]] / CONTROL_RANGE,
                    thrust: f[cols[1]] / CONTROL_RANGE,
                    airbrake_left: f[cols[2]] / CONTROL_RANGE,
                    airbrake_right: f[cols[3]] / CONTROL_RANGE,
                    steer_y: {
                        let alignment = -(f[hull[0]] * f[down[0]]
                            + f[hull[1]] * f[down[1]]
                            + f[hull[2]] * f[down[2]]);
                        let open = !gated || alignment > 0.9;
                        let airborne = (f[flags] as u32) & 1 == 0;
                        // In steer_y units at the air gain: `bias / (100 * 0.3)`.
                        let extra = if airborne { bias / 30.0 } else { 0.0 };
                        sign * (if open { f[cols[4]] } else { 0.0 }) / CONTROL_RANGE + extra
                    },
                    ..ShipControls::default()
                }
            })
            .collect(),
    )
}

/// The extra columns `Trace` does not know: rail blend and own lateral, per tick.
fn rail_columns(text: &str) -> Vec<(f32, Vec3)> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().expect("header").split(',').collect();
    let at = |name: &str| header.iter().position(|h| *h == name).expect(name);
    let (b, x, y, z) = (
        at("rail_blend"),
        at("own_lat_x"),
        at("own_lat_y"),
        at("own_lat_z"),
    );
    lines
        .map(|line| {
            let f: Vec<f32> = line.split(',').map(|v| v.parse().unwrap()).collect();
            (f[b], Vec3::new(f[x], f[y], f[z]))
        })
        .collect()
}

/// Undoes `Ai_SnapToLane`'s velocity turn: it scales the lateral component by
/// `1 - k`, `k = blend^2 * 0.7 / 4`, then restores the speed.
fn undo_rail(velocity: Vec3, blend: f32, lateral: Vec3) -> Vec3 {
    let k = blend * blend * 0.7 * 0.25;
    if k < 0.25e-6 {
        return velocity;
    }
    let pre = velocity + lateral * (velocity.dot(lateral) * k / (1.0 - k));
    pre.normalize_or_zero() * velocity.length()
}

/// `craft+0x284`/`+0x288` replayed over the recording, as the force-term walk does.
fn ground_timers(frames: &[oag_trace::trace::Frame], rebound: f32) -> Vec<(f32, f32)> {
    let (mut landing, mut airborne) = (0.0f32, 0.0f32);
    let mut out = vec![(landing, airborne)];
    for pair in frames.windows(2) {
        if pair[1].grounded > 0.0 {
            airborne = 0.0;
            landing += pair[0].dt;
        } else {
            if rebound < airborne {
                landing = 0.0;
            }
            airborne += pair[0].dt;
        }
        out.push((landing, airborne));
    }
    out
}

#[test]
#[ignore = "scratch sweep: OAG_SWEEP=1 with OAG_RAMP_TRACE, needs data/images/"]
fn the_original_s_autopilot_stepped_once_per_row() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let path = workspace(&env_or(
        "OAG_RAMP_TRACE",
        "data/traces/fortgale-autopilot-ramp-1.csv",
    ));
    let text = std::fs::read_to_string(&path).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");
    let rail = rail_columns(&text);
    let raw = raw_controls(&text);
    let raw = raw.as_deref();
    let ticks = env_or("OAG_RAMP_TICKS", "1170..1230");
    let (lo, hi) = ticks.split_once("..").expect("OAG_RAMP_TICKS is lo..hi");
    let (lo, hi): (u64, u64) = (lo.parse().unwrap(), hi.parse().unwrap());
    let team = env_or("OAG_RAMP_TEAM", "Assegai");
    let class = env_or("OAG_RAMP_CLASS", "PHANTOM");
    let track = env_or("OAG_RAMP_TRACK", r"Data\Environments\14_Track\track.vex");

    let mut archives = pulse::open(workspace(IMAGE).to_str().unwrap()).expect("image");
    let nodes = collision::from_vex(&archives.read_name(&track).unwrap()).unwrap();
    let world = collision_world(&nodes);
    let stats =
        handling::from_blob(&archives.read_name(&handling::entry_name(&team)).unwrap()).unwrap();
    let global = handling::global_from_blob(&archives.read_name(handling::GLOBAL_ENTRY).unwrap())
        .unwrap()
        .expect("<Global>");
    let (pads, gravity_mul, _) = global.class_named(&class).expect("class");
    let handling = handling_for(&stats, &class, pads, global.special).expect("class handling");
    let env = Environment {
        class_gravity_scale: gravity_mul.airborne,
        engine_flags: EngineFlags {
            uncapped: true,
            speedup: Some(ENGINE_AUTOPILOT_SPEEDUP),
        },
        ..Environment::default()
    };
    let timers = ground_timers(&trace.frames, handling.antigrav.rebound_jump_time);
    println!(
        "team {team} class {class} gravity_mul {} mass {}",
        gravity_mul.airborne, handling.physical.mass
    );
    println!(
        "tick  g    dt     | dw_yaw dw_pitch dw_roll | w_yaw w_pitch | v_meas.y  v_ours.y  dvy   | dv_up  dv_fwd dv_lat | up_err_deg | hover_y grav_y eng_y vdamp_y | rail_dvy"
    );
    for (i, frame) in trace.frames.iter().enumerate() {
        if frame.tick < lo || frame.tick >= hi || i + 1 >= trace.frames.len() {
            continue;
        }
        let next = &trace.frames[i + 1];
        let mut state = initial_state(
            frame,
            &handling,
            Basis::LeftUpForward,
            AngularReading::NegatedLocal,
        );
        state.stun_timer = frame.stun_timer.unwrap_or(0.0);
        state.slowdown_timer = frame.timer_2e0.unwrap_or(0.0);
        (state.time_since_landing, state.time_airborne) = timers[i];
        let controls = match raw {
            Some(raw) => raw[i],
            None => ShipControls {
                steer_x: frame.steer / CONTROL_RANGE,
                thrust: frame.throttle / CONTROL_RANGE,
                airbrake_left: frame.airbrake_left / CONTROL_RANGE,
                airbrake_right: frame.airbrake_right / CONTROL_RANGE,
                ..ShipControls::default()
            },
        };
        let (up0, fwd0) = (state.body.up(), state.body.forward());
        let lat0 = up0.cross(fwd0);

        let mut probe = state;
        probe.body.clear_accumulators();
        let terms = forces::evaluate(&mut probe, &controls, &handling, &env, &world, frame.dt);
        let mass = handling.physical.mass;
        let hover_y = terms
            .hover
            .probes
            .iter()
            .filter(|p| p.contact)
            .fold(Vec3::ZERO, |a, p| a + p.force)
            .y
            / mass;
        let engine_y = (probe.body.orientation * terms.engine.as_local_force()).y / mass;

        step(&mut state, &controls, &handling, &env, &world, frame.dt);
        let (blend, lateral) = rail[i + 1];
        let measured = undo_rail(next.velocity, blend, lateral);
        let rail_dvy = next.velocity.y - measured.y;
        let dv = state.body.linear_velocity - measured;
        let omega_next = AngularReading::NegatedLocal
            .to_world(next.angular_rate.expect("omega columns"), next.rows());
        let dw = (state.body.angular_velocity - omega_next) / frame.dt;
        let up_err = state
            .body
            .up()
            .dot(next.up)
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        println!(
            "{:>4} {:.1} {:.4} | {:6.3} {:6.3} {:6.3} | {:6.3} {:6.3} | {:8.2} {:8.2} {:6.2} | {:6.2} {:6.2} {:6.2} | {:6.3} | {:7.1} {:6.1} {:5.1} {:6.1} | {:6.2}",
            frame.tick,
            next.grounded,
            frame.dt,
            dw.dot(up0),
            dw.dot(lat0),
            dw.dot(fwd0),
            omega_next.dot(up0),
            omega_next.dot(lat0),
            measured.y,
            state.body.linear_velocity.y,
            dv.y,
            dv.dot(up0),
            dv.dot(fwd0),
            dv.dot(lat0),
            up_err,
            hover_y,
            terms.gravity.y / mass,
            engine_y,
            terms.vertical_damping.y / mass,
            rail_dvy,
        );
    }
    rollout(&trace, &rail, raw, lo, hi, &handling, &env, &world, &timers);
}

/// From `lo`, our physics stepped on its own state with the original's recorded
/// controls each tick, and the original's rail (its blend, its lateral, the lane
/// point's lateral coordinate solved from the single-step prediction) applied to
/// ours. Isolates physics drift from the controller and the rail's own state.
#[allow(clippy::too_many_arguments)]
fn rollout(
    trace: &Trace,
    rail: &[(f32, Vec3)],
    raw: Option<&[ShipControls]>,
    lo: u64,
    hi: u64,
    handling: &oag_physics::Handling,
    env: &Environment,
    world: &oag_physics::CollisionWorld,
    timers: &[(f32, f32)],
) {
    let start = trace.frames.iter().position(|f| f.tick == lo).expect("lo");
    let seed = &trace.frames[start];
    let mut ours = initial_state(
        seed,
        handling,
        Basis::LeftUpForward,
        AngularReading::NegatedLocal,
    );
    (ours.time_since_landing, ours.time_airborne) = timers[start];
    println!(
        "\nrollout from {lo}: tick | orig pos.y vel.y | ours pos.y vel.y | dpos dvel | up_deg | g o/u"
    );
    for i in start..trace.frames.len() - 1 {
        let frame = &trace.frames[i];
        if frame.tick >= hi {
            break;
        }
        let next = &trace.frames[i + 1];
        let controls = match raw {
            Some(raw) => raw[i],
            None => ShipControls {
                steer_x: frame.steer / CONTROL_RANGE,
                thrust: frame.throttle / CONTROL_RANGE,
                airbrake_left: frame.airbrake_left / CONTROL_RANGE,
                airbrake_right: frame.airbrake_right / CONTROL_RANGE,
                ..ShipControls::default()
            },
        };
        // The lane point's lateral coordinate, solved from a reseeded single step.
        let mut probe = initial_state(
            frame,
            handling,
            Basis::LeftUpForward,
            AngularReading::NegatedLocal,
        );
        (probe.time_since_landing, probe.time_airborne) = timers[i];
        step(&mut probe, &controls, handling, env, world, frame.dt);
        let (blend, lateral) = rail[i + 1];
        let f = blend * blend * 0.7;
        let lane = if f >= 1.0e-6 {
            let pre = probe.body.position.dot(lateral);
            Some(pre - (probe.body.position - next.position).dot(lateral) / f)
        } else {
            None
        };
        step(&mut ours, &controls, handling, env, world, frame.dt);
        if let Some(lane) = lane {
            let v = ours.body.linear_velocity;
            let turned = v - lateral * (v.dot(lateral) * f * 0.25);
            ours.body.linear_velocity = turned.normalize_or_zero() * v.length();
            let d = ours.body.position.dot(lateral) - lane;
            ours.body.position -= lateral * (d * f);
        }
        let up = ours
            .body
            .up()
            .dot(next.up)
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        println!(
            "{:>4} | {:8.2} {:7.2} | {:8.2} {:7.2} | {:6.2} {:6.2} | {:6.2} | {:.1}/{:.1}",
            next.tick,
            next.position.y,
            next.velocity.y,
            ours.body.position.y,
            ours.body.linear_velocity.y,
            (ours.body.position - next.position).length(),
            (ours.body.linear_velocity - next.velocity).length(),
            up,
            next.grounded,
            ours.grounded,
        );
    }
}
