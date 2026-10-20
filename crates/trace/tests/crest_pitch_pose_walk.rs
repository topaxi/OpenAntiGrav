//! A scratch sweep: the airborne pitch over Talon's Junction's 1595 crest, one
//! `oag_physics::step` (plus the collision cage) from each recorded row of the
//! clean lap against the next row, so nothing compounds.
//!
//! Prints the pitch-axis angular-acceleration residual (ours minus the
//! original's, about the craft's lateral axis) beside every angular term that
//! can reach pitch: the pitch input and the in-air bias, angular damping, the
//! weathervane, the hover probes' own spring torques, the surface alignment.
//!
//! Prints only. `#[ignore]`d, and returns early unless `OAG_SWEEP` is set:
//!
//! ```text
//! OAG_SWEEP=1 OAG_CREST_TICKS=1585..1615 cargo nextest run -p oag-trace \
//!     crest_pitch_pose_walk --run-ignored all --no-capture
//! ```
//!
//! `OAG_CREST_TRACE` (default the clean lap), `OAG_CREST_TEAM` (`Assegai`) and
//! `OAG_CREST_CLASS` (`VENOM`) choose the capture.

use oag_core::math::Vec3;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::controls::CONTROL_RANGE;
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

/// `craft+0x284`/`+0x288` replayed over the recording, as the ramp walk does.
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

/// The driver's raw pitch output (`ai_pitch`), on the `-1..=1` scale and with
/// `Ship_TestPitchAlignment`'s gate applied from the capture's own `hull_up`
/// and `spline_down` columns, where the capture carries them.
fn pitch_column(text: &str) -> Option<Vec<f32>> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next()?.split(',').collect();
    let at = |name: &str| header.iter().position(|h| *h == name);
    let pitch = at("ai_pitch")?;
    let hull = [at("hull_up_x")?, at("hull_up_y")?, at("hull_up_z")?];
    let down = [
        at("spline_down_x")?,
        at("spline_down_y")?,
        at("spline_down_z")?,
    ];
    Some(
        lines
            .map(|line| {
                let f: Vec<f32> = line.split(',').map(|v| v.parse().unwrap()).collect();
                let alignment =
                    -(f[hull[0]] * f[down[0]] + f[hull[1]] * f[down[1]] + f[hull[2]] * f[down[2]]);
                if alignment > 0.9 {
                    f[pitch] / CONTROL_RANGE
                } else {
                    0.0
                }
            })
            .collect(),
    )
}

#[test]
#[ignore = "scratch sweep: OAG_SWEEP=1, needs data/images/"]
fn the_crest_stepped_once_per_row() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let path = workspace(&env_or(
        "OAG_CREST_TRACE",
        "data/traces/talons-junction-clean-lap.csv",
    ));
    let text = std::fs::read_to_string(&path).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");
    let ticks = env_or("OAG_CREST_TICKS", "1585..1615");
    let (lo, hi) = ticks.split_once("..").expect("OAG_CREST_TICKS is lo..hi");
    let (lo, hi): (u64, u64) = (lo.parse().unwrap(), hi.parse().unwrap());
    let team = env_or("OAG_CREST_TEAM", "Assegai");
    let class = env_or("OAG_CREST_CLASS", "VENOM");
    let track = env_or("OAG_CREST_TRACK", r"Data\Environments\16_Track\track.vex");
    let pitch = pitch_column(&text);

    let mut archives = pulse::open(workspace(IMAGE).to_str().unwrap()).expect("image");
    let track_blob = archives.read_name(&track).unwrap();
    let nodes = collision::from_vex(&track_blob).unwrap();
    let world = collision_world(&nodes);
    let cage = oag_trace::replay::cage_of(&track_blob);
    let stats =
        handling::from_blob(&archives.read_name(&handling::entry_name(&team)).unwrap()).unwrap();
    let handling = handling_for(
        &stats,
        &class,
        handling::SpeedupPads::default(),
        handling::Special::default(),
    )
    .expect("class handling");
    let env = Environment::default();
    let timers = ground_timers(&trace.frames, handling.antigrav.rebound_jump_time);
    println!(
        "team {team} class {class} weight_distribution {} pitch_air {} pitch_damping {}",
        handling.dimensions.weight_distribution,
        handling.pitch.pitch_air,
        handling.pitch.pitch_damping
    );
    println!(
        "tick  g_rec g_next ours | dw_pitch_res | pitch_in damp vane probes align | contacts | w_pitch_next ours"
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
        let controls = ShipControls {
            steer_x: frame.steer / CONTROL_RANGE,
            thrust: frame.throttle / CONTROL_RANGE,
            airbrake_left: frame.airbrake_left / CONTROL_RANGE,
            airbrake_right: frame.airbrake_right / CONTROL_RANGE,
            steer_y: pitch.as_ref().map_or(0.0, |p| p[i]),
            ..ShipControls::default()
        };
        let lat0 = state.body.up().cross(state.body.forward());
        let inertia = state.body.inertia;

        let mut probe = state;
        probe.body.clear_accumulators();
        let terms = forces::evaluate(&mut probe, &controls, &handling, &env, &world, frame.dt);
        // Each term as a world angular acceleration about lat0. Local terms are
        // `x` (pitch) over `I_xx` in the body frame; world terms divide by the
        // world-axis diagonal (Pulse's frame), projected on lat0.
        let orientation = probe.body.orientation;
        let local = |v: Vec3| (orientation * (v / inertia)).dot(lat0);
        let world_term = |v: Vec3| (v / inertia).dot(lat0);
        let probe_torque = terms.hover.active().iter().fold(Vec3::ZERO, |a, p| {
            a + (p.point - state.body.position).cross(p.force)
        });
        let contacts: String = terms
            .hover
            .active()
            .iter()
            .map(|p| if p.contact { '1' } else { '0' })
            .collect();

        if std::env::var_os("OAG_CREST_PROBES").is_some() {
            let heights: Vec<String> = terms
                .hover
                .active()
                .iter()
                .map(|p| format!("h{:.3} c{} e{:.3}", p.height, p.contact, p.escape.length()))
                .collect();
            println!(
                "PROBES {} {heights:?} escape {:.3}",
                frame.tick,
                terms.hover.escape.length()
            );
            let up = state.body.up();
            for p in terms.hover.active() {
                for back in [0.0f32, 3.0] {
                    let ray = oag_physics::collide::Ray::new(p.point + up * back, -up, 12.0);
                    let hit = oag_physics::collide::Raycaster::raycast(&world, ray, None, true);
                    println!(
                        "   cast from {back} above: {:?}",
                        hit.map(|h| (
                            h.distance - back,
                            h.surface,
                            h.normal.dot(up),
                            h.normal.dot(state.body.forward()),
                            2.0 * (h.distance - back)
                                - (h.distance - back)
                                - 6.0 * h.normal.dot(state.body.forward())
                        ))
                    );
                }
            }
        }
        let evaluated = step(&mut state, &controls, &handling, &env, &world, frame.dt);
        let caged =
            oag_gameplay::cage::pass(cage.as_ref(), &mut state, &handling, &evaluated.wall, false);
        if std::env::var_os("OAG_CREST_PROBES").is_some() {
            let rise = (state.body.position - next.position).dot(next.up);
            println!(
                "STEP {} wall {:?} cage contacts {} escape {:?} dv {:?} | rise vs record {rise:.3}",
                frame.tick,
                evaluated.wall.contacts,
                caged.contacts,
                caged.escape,
                caged.velocity_delta
            );
        }
        let omega_next = AngularReading::NegatedLocal
            .to_world(next.angular_rate.expect("omega columns"), next.rows());
        let dw = (state.body.angular_velocity - omega_next) / frame.dt;
        println!(
            "{:>4} {:.1} {:.1} {:.1} | {:7.3} | {:6.3} {:6.3} {:6.3} {:6.3} {:6.3} | {} | {:7.3} {:7.3}",
            frame.tick,
            frame.grounded,
            next.grounded,
            state.grounded,
            dw.dot(lat0),
            local(Vec3::new(terms.pitch, 0.0, 0.0)),
            local(terms.angular_damping),
            world_term(terms.weathervane),
            world_term(probe_torque),
            world_term(terms.hover.alignment_torque),
            contacts,
            omega_next.dot(lat0),
            state.body.angular_velocity.dot(lat0),
        );
        println!(
            "ROW,{},{},{},{},{},{},{},{},{},{},{},{},{}",
            frame.tick,
            frame.grounded,
            next.grounded,
            state.grounded,
            dw.dot(lat0),
            local(Vec3::new(terms.pitch, 0.0, 0.0)),
            local(terms.angular_damping),
            world_term(terms.weathervane),
            world_term(probe_torque),
            world_term(terms.hover.alignment_torque),
            omega_next.dot(lat0),
            frame.velocity.length(),
            controls.steer_y,
        );
    }
}

/// From `OAG_ROLL_FROM`, our physics rolled forward on its own state with the
/// recording's controls, once per variant; each variant puts one state component
/// back on the recording every tick. Whichever component removes the growth of
/// the nose error is the one the error grows through.
#[test]
#[ignore = "scratch sweep: OAG_SWEEP=1, needs data/images/"]
fn the_crest_rolled_forward_with_one_component_restored() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let path = workspace(&env_or(
        "OAG_CREST_TRACE",
        "data/traces/talons-junction-clean-lap.csv",
    ));
    let text = std::fs::read_to_string(&path).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");
    let from: u64 = env_or("OAG_ROLL_FROM", "1594").parse().unwrap();
    let to: u64 = env_or("OAG_ROLL_TO", "1610").parse().unwrap();
    let team = env_or("OAG_CREST_TEAM", "Assegai");
    let class = env_or("OAG_CREST_CLASS", "VENOM");
    let track = env_or("OAG_CREST_TRACK", r"Data\Environments\16_Track\track.vex");

    let mut archives = pulse::open(workspace(IMAGE).to_str().unwrap()).expect("image");
    let track_blob = archives.read_name(&track).unwrap();
    let world = collision_world(&collision::from_vex(&track_blob).unwrap());
    let cage = oag_trace::replay::cage_of(&track_blob);
    let stats =
        handling::from_blob(&archives.read_name(&handling::entry_name(&team)).unwrap()).unwrap();
    let handling = handling_for(
        &stats,
        &class,
        handling::SpeedupPads::default(),
        handling::Special::default(),
    )
    .expect("class handling");
    let env = Environment::default();
    let timers = ground_timers(&trace.frames, handling.antigrav.rebound_jump_time);
    let start = trace
        .frames
        .iter()
        .position(|f| f.tick == from)
        .expect("OAG_ROLL_FROM is in the capture");
    let seed = |i: usize| {
        initial_state(
            &trace.frames[i],
            &handling,
            Basis::LeftUpForward,
            AngularReading::NegatedLocal,
        )
    };
    for variant in ["none", "omega", "attitude", "position", "velocity"] {
        let mut state = seed(start);
        (state.time_since_landing, state.time_airborne) = timers[start];
        let mut line = format!("{variant:>9}:");
        for i in start..trace.frames.len() - 1 {
            let frame = &trace.frames[i];
            if frame.tick >= to {
                break;
            }
            if i > start {
                let record = seed(i);
                match variant {
                    "omega" => state.body.angular_velocity = record.body.angular_velocity,
                    "attitude" => state.body.orientation = record.body.orientation,
                    "position" => state.body.position = record.body.position,
                    "velocity" => state.body.linear_velocity = record.body.linear_velocity,
                    _ => {}
                }
            }
            let controls = ShipControls {
                steer_x: frame.steer / CONTROL_RANGE,
                thrust: frame.throttle / CONTROL_RANGE,
                airbrake_left: frame.airbrake_left / CONTROL_RANGE,
                airbrake_right: frame.airbrake_right / CONTROL_RANGE,
                ..ShipControls::default()
            };
            let evaluated = step(&mut state, &controls, &handling, &env, &world, frame.dt);
            oag_gameplay::cage::pass(cage.as_ref(), &mut state, &handling, &evaluated.wall, false);
            let next = &trace.frames[i + 1];
            let nose = (state.body.forward() - next.forward).dot(next.up) * 1000.0;
            let rise = (state.body.position - next.position).dot(next.up) * 1000.0;
            let along = (state.body.position - next.position).dot(next.forward) * 1000.0;
            line += &format!(
                " {}:{nose:.1}/{:.1}/up{rise:.0}/fwd{along:.0}",
                next.tick, state.grounded
            );
        }
        println!("{line}");
    }
}
