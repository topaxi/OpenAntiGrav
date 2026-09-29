//! Does our hull find the wall where the original's craft was stopped by one?
//!
//! Walks a capture of a **shove** - a craft teleported (`psp-drive.py place`)
//! toward a wall at an angle and speed - through `oag_physics::wall::resolve`
//! at every recorded pose, and prints beside our contact count the original's
//! own speed and per-tick speed change. A wall hit in the original is a
//! sudden loss of speed; if our hull reports no contact on those ticks, our
//! wall is missing, or lower, or thinner than the original's at that place.
//!
//! Scratch and env-driven: `OAG_SHOVE_CAPTURE=<csv>` and `OAG_SHOVE_TRACK=<n>_Track`.
//! Not a gate; prints only. `#[ignore]`d, needs a disc image.

use oag_core::math::Vec3;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::controls::CONTROL_RANGE;
use oag_physics::{Environment, Ray, Raycaster, ShipControls, clamp_dt, forces, hover, wall};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::AngularReading;
use oag_vex::collision;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

#[test]
#[ignore = "needs data/images/ and a scratch capture"]
fn wall_contacts_along_a_shove_capture() {
    let (Ok(capture), Ok(track)) = (
        std::env::var("OAG_SHOVE_CAPTURE"),
        std::env::var("OAG_SHOVE_TRACK"),
    ) else {
        return;
    };
    let mut archives = pulse::open(workspace(IMAGE).to_str().expect("utf-8")).expect("disc");
    let track_blob = archives
        .read_name(&format!(r"Data\Environments\{track}\track.vex"))
        .expect("reading the track");
    let nodes = collision::from_vex(&track_blob).expect("decoding the collision nodes");
    let collision = collision_world(&nodes);
    let stats_blob = archives
        .read_name(&handling::entry_name(TEAM))
        .expect("handling stats");
    let stats = handling::from_blob(&stats_blob).expect("parsing");
    let handling = handling_for(
        &stats,
        CLASS,
        handling::SpeedupPads::default(),
        handling::Special::default(),
    )
    .expect("class");
    let text = std::fs::read_to_string(capture).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");
    let environment = Environment::default();
    println!("tick  grounded  o_speed  o_dspeed  our_contacts  our_depth  our_g  pred_dspeed  pos");
    for pair in trace.frames.windows(2) {
        let mut state = initial_state(
            &pair[1],
            &handling,
            Basis::LeftUpForward,
            AngularReading::NegatedLocal,
        );
        let response = wall::resolve(
            &mut state,
            &handling,
            &environment,
            &collision,
            pair[0].position,
        );
        // The force law at the same pose: what speed change does it predict
        // for the step that follows, along the recorded velocity?
        let mut fstate = initial_state(
            &pair[0],
            &handling,
            Basis::LeftUpForward,
            AngularReading::NegatedLocal,
        );
        let dt = clamp_dt(pair[0].dt);
        let controls = ShipControls {
            steer_x: pair[0].steer / CONTROL_RANGE,
            thrust: pair[0].throttle / CONTROL_RANGE,
            airbrake_left: pair[0].airbrake_left / CONTROL_RANGE,
            airbrake_right: pair[0].airbrake_right / CONTROL_RANGE,
            ..ShipControls::default()
        };
        let mass = fstate.body.mass;
        let evaluated = forces::evaluate(
            &mut fstate,
            &controls,
            &handling,
            &environment,
            &collision,
            dt,
        );
        let predicted = fstate.body.force / mass * dt;
        let heading = pair[0].velocity.normalize_or_zero();
        let reach = hover::target_height(&handling, 0.0, 0.0);
        let _ = (reach, Vec3::ZERO);
        println!(
            "{:>4}  {:>4}  {:>8.1}  {:>+8.2}  {:>5}  {:>8.3}  {:>4}  {:>+8.2}  {:.0},{:.1},{:.0}",
            pair[1].tick,
            pair[1].grounded,
            pair[1].speed,
            pair[1].speed - pair[0].speed,
            response.contacts,
            response.resolved.map_or(0.0, |c| c.depth),
            oag_physics::ShipState::quantise_grounded(evaluated.hover.contacts),
            predicted.dot(heading),
            pair[1].position.x,
            pair[1].position.y,
            pair[1].position.z,
        );
    }

    // What lies ahead of the recorded path: every surface, near each tick, along
    // the velocity, both a hull-length cast and casts to each side.
    let from: u64 = std::env::var("OAG_PROBE_FROM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);
    let to: u64 = std::env::var("OAG_PROBE_TO")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(12);
    for frame in trace
        .frames
        .iter()
        .filter(|f| f.tick >= from && f.tick <= to)
    {
        let heading = frame.velocity.normalize_or_zero();
        for (name, direction) in [
            ("ahead", heading),
            ("left", frame.row0),
            ("right", -frame.row0),
            ("down", -frame.up),
            ("up", frame.up),
        ] {
            match collision.raycast(
                Ray::new(frame.position, direction.normalize_or_zero(), 14.0),
                None,
                false,
            ) {
                Some(hit) => println!(
                    "PROBE tick {} {name:>5} d {:>5.2} {:?} normal {:.2},{:.2},{:.2}",
                    frame.tick, hit.distance, hit.surface, hit.normal.x, hit.normal.y, hit.normal.z
                ),
                None => println!("PROBE tick {} {name:>5} nothing within 14", frame.tick),
            }
        }
    }
}
