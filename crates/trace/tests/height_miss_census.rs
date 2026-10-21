//! A scratch census: every row of a set of Pulse captures stepped once (physics plus the
//! collision cage) against the next row, and the height miss along the next row's up axis
//! split into its two parts, so a miss can be classified rather than only counted:
//!
//! - `rec_jump`: the original's own `pos(t+1) - pos(t) - v(t+1) dt` along up, a move of
//!   the body its velocity does not carry (an escape, the cage, a rail write);
//! - `our_jump`: the same for ours;
//! - `dv_up`: ours' velocity minus the record's along up, times `dt`.
//!
//! `miss = our_jump - rec_jump + dv_up` to rounding. Per row it also prints what each
//! probe saw (the nearest surface straight down its ray, its height, contact, escape),
//! the wall and cage contacts and the speed.
//!
//! Prints only. `#[ignore]`d, and returns early unless `OAG_SWEEP` is set:
//!
//! ```text
//! OAG_SWEEP=1 OAG_CENSUS_OUT=/path/rows.csv cargo nextest run -p oag-trace \
//!     height_miss_census --run-ignored all --no-capture
//! ```
//!
//! `OAG_CENSUS` replaces the capture list: `path|track|team|class;...`, `track` the
//! circuit directory (`16_Track`). `OAG_CENSUS_SUM=1` sums the two probes' escapes instead
//! of taking the larger.

use oag_core::math::Vec3;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::collide::{Ray, Raycaster, Surface};
use oag_physics::controls::CONTROL_RANGE;
use oag_physics::{Environment, ShipControls, TrackSample, forces, step};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::AngularReading;
use oag_vex::collision;
use std::fmt::Write as _;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";

/// The player-input captures on the two circuits whose collision this crate loads.
const CAPTURES: &str = "data/traces/talons-junction-clean-lap.csv|16_Track|Assegai|VENOM;\
data/traces/talons-junction-time-trial-lap.csv|16_Track|Assegai|VENOM;\
data/traces/talons-junction-lap-replay.csv|16_Track|Assegai|VENOM;\
data/traces/talons-junction-standing-start.csv|16_Track|Assegai|VENOM;\
data/traces/fortgale-player-ramp-1170.csv|14_Track|Assegai|PHANTOM;\
data/traces/fortgale-player-ramp-1180.csv|14_Track|Assegai|PHANTOM;\
data/traces/fortgale-player-ramp-1180nb.csv|14_Track|Assegai|PHANTOM;\
data/traces/fortgale-player-ramp-1190.csv|14_Track|Assegai|PHANTOM";

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
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

/// Named columns a capture may or may not carry, per row.
fn column(text: &str, names: &[&str]) -> Option<Vec<Vec<f32>>> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next()?.split(',').collect();
    let at: Option<Vec<usize>> = names
        .iter()
        .map(|n| header.iter().position(|h| h == n))
        .collect();
    let at = at?;
    Some(
        lines
            .map(|line| {
                let f: Vec<&str> = line.split(',').collect();
                at.iter().map(|&i| f[i].parse().unwrap()).collect()
            })
            .collect(),
    )
}

fn surface_name(surface: Surface) -> &'static str {
    match surface {
        Surface::Floor => "floor",
        Surface::Wall => "wall",
        Surface::MagFloor => "mag",
        Surface::Reset => "reset",
    }
}

#[test]
#[ignore = "scratch sweep: OAG_SWEEP=1, needs data/images/"]
fn every_row_stepped_once_and_its_height_miss_split() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let list = std::env::var("OAG_CENSUS").unwrap_or_else(|_| CAPTURES.to_string());
    let sum = std::env::var_os("OAG_CENSUS_SUM").is_some();
    let mut out = String::from(
        "capture,tick,g,g_next,g_ours,speed,miss,rec_jump,our_jump,dv_up,escape,\
         p0_surf,p0_h,p0_c,p0_e,p1_surf,p1_h,p1_c,p1_e,walls,cage,stun\n",
    );
    let mut archives = pulse::open(workspace(IMAGE).to_str().unwrap()).expect("image");
    let global = handling::global_from_blob(&archives.read_name(handling::GLOBAL_ENTRY).unwrap())
        .unwrap()
        .expect("<Global>");
    for entry in list.split(';').filter(|e| !e.trim().is_empty()) {
        let parts: Vec<&str> = entry.trim().split('|').collect();
        let [path, track, team, class] = parts[..] else {
            panic!("OAG_CENSUS entry is path|track|team|class: {entry}");
        };
        let name = std::path::Path::new(path)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let Ok(text) = std::fs::read_to_string(workspace(path)) else {
            println!("SKIP {path}: not present");
            continue;
        };
        let trace = Trace::parse(&text).expect("parsing the capture");
        let track_blob = archives
            .read_name(&format!(r"Data\Environments\{track}\track.vex"))
            .unwrap();
        let world = collision_world(&collision::from_vex(&track_blob).unwrap());
        let cage = oag_trace::replay::cage_of(&track_blob);
        let stats =
            handling::from_blob(&archives.read_name(&handling::entry_name(team)).unwrap()).unwrap();
        let (pads, gravity_mul, _) = global.class_named(class).expect("class");
        let handling = handling_for(&stats, class, pads, global.special).expect("class handling");
        let timers = ground_timers(&trace.frames, handling.antigrav.rebound_jump_time);
        let stick = column(&text, &["stick_pitch"]);
        let downs = column(&text, &["spline_down_x", "spline_down_y", "spline_down_z"]);
        let mut rows = 0usize;
        for (i, frame) in trace.frames.iter().enumerate() {
            let Some(next) = trace.frames.get(i + 1) else {
                continue;
            };
            let env = Environment {
                class_gravity_scale: gravity_mul.airborne,
                track_sample: downs.as_ref().map(|d| TrackSample {
                    position: frame.position,
                    down: Vec3::new(d[i][0], d[i][1], d[i][2]),
                }),
                ..Environment::default()
            };
            let mut state = initial_state(
                frame,
                &handling,
                Basis::LeftUpForward,
                AngularReading::NegatedLocal,
            );
            state.stun_timer = frame.stun_timer.unwrap_or(0.0);
            state.slowdown_timer = frame.timer_2e0.unwrap_or(0.0);
            (state.time_since_landing, state.time_airborne) = timers[i];
            if sum {
                state.hover_rig.sum_escapes = true;
            }
            let controls = ShipControls {
                steer_x: frame.steer / CONTROL_RANGE,
                thrust: frame.throttle / CONTROL_RANGE,
                airbrake_left: frame.airbrake_left / CONTROL_RANGE,
                airbrake_right: frame.airbrake_right / CONTROL_RANGE,
                steer_y: stick.as_ref().map_or(0.0, |s| s[i][0] / CONTROL_RANGE),
                ..ShipControls::default()
            };
            let up = state.body.up();
            let mut probe = state;
            probe.body.clear_accumulators();
            let terms = forces::evaluate(&mut probe, &controls, &handling, &env, &world, frame.dt);
            // The nearest surface straight down each probe's ray, whatever its class: the
            // probe record keeps the height and escape but not what it hit.
            let mut probes = String::new();
            for p in terms.hover.active().iter().take(2) {
                let hit = world.raycast(Ray::new(p.point, -up, 12.0), None, false);
                let surf = hit.map_or("none", |h| surface_name(h.surface));
                let _ = write!(
                    probes,
                    "{surf},{:.4},{},{:.4},",
                    p.height,
                    u8::from(p.contact),
                    p.escape.length()
                );
            }
            let evaluated = step(&mut state, &controls, &handling, &env, &world, frame.dt);
            let caged = oag_gameplay::cage::pass(
                cage.as_ref(),
                &mut state,
                &handling,
                &evaluated.wall,
                false,
            );
            let n_up = next.up;
            let miss = (state.body.position - next.position).dot(n_up);
            let rec_jump = (next.position - frame.position - next.velocity * frame.dt).dot(n_up);
            let our_jump =
                (state.body.position - frame.position - state.body.linear_velocity * frame.dt)
                    .dot(n_up);
            let dv_up = (state.body.linear_velocity - next.velocity).dot(n_up) * frame.dt;
            let _ = writeln!(
                out,
                "{name},{},{},{},{},{:.3},{:.4},{:.4},{:.4},{:.4},{:.4},{probes}{},{},{:.3}",
                frame.tick,
                frame.grounded,
                next.grounded,
                state.grounded,
                frame.velocity.length(),
                miss,
                rec_jump,
                our_jump,
                dv_up,
                terms.hover.escape.length(),
                evaluated.wall.contacts,
                caged.contacts,
                state.stun_timer,
            );
            rows += 1;
        }
        println!("{name}: {rows} rows");
    }
    match std::env::var("OAG_CENSUS_OUT") {
        Ok(path) => std::fs::write(&path, out).expect("writing the census"),
        Err(_) => print!("{out}"),
    }
}
