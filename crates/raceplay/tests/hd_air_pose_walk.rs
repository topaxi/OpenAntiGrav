//! A scratch sweep: one `oag_physics::step` from each row of a Wipeout HD capture, against the
//! next row. The capture is player input (`scripts/rpcs3-hd-air-hold.py`: placed, throttle held);
//! rows flown by the original's Autopilot (`scripts/rpcs3-hd-autopilot.py`) are refused, see below.
//!
//! The race is loaded (`oag_raceplay::load`, Ubermall PHANTOM by default) only for what it seats
//! on the player's craft: HD's craft laws, hover rig and handling, and the collision world. Each
//! row then seeds that state's body (position, the basis from the up and forward rows, velocity,
//! and `body+0x1a0`, which is the negated body-local rate as on Pulse) and steps once with the
//! row's own controls (the record `craft+0x84` points at: steer, thrust, left, right, pitch) and
//! the row's own engine flags (`craft+0x264`). The frame's `dt` is not recorded, so it is read off the position change along
//! the velocity. Rows are one per new body position, so a missed frame shows as a long `dt`
//! and is skipped.
//!
//! Prints only. `#[ignore]`d, and returns early unless `OAG_SWEEP` is set:
//!
//! ```text
//! OAG_SWEEP=1 OAG_HD_AIR=data/.../samples.csv OAG_HD_ROWS=1260..1420 \
//!     cargo nextest run -p oag-raceplay hd_air_pose_walk --run-ignored all --no-capture
//! ```
//!
//! `OAG_HD_GRAV=ground` puts the class gravity scale on Pulse's grounded term instead of HD's
//! airborne one. `OAG_HD_TEAM` (default `Feisar`, HD's default team) picks the handling. `OAG_HD_BIAS=0` steps with the in-air `weight_distribution` torque off, `1` with it on
//! (default: the title's own craft law). `OAG_HD_MODE=tt` loads a Time Trial, `OAG_HD_CLASS` the
//! class (default `PHANTOM`), `OAG_HD_AUTOPILOT=1` admits Autopilot rows, `OAG_HD_SUM=0` takes the
//! largest hover escape alone, `OAG_HD_GATE=0` leaves the pitch gate open.

use oag_core::math::{Mat3, Quat, Vec3};
use oag_physics::engine::EngineFlags;
use oag_physics::{Environment, ShipControls, step};

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

#[test]
#[ignore = "scratch sweep: OAG_SWEEP=1 with OAG_HD_AIR, needs data/images/"]
fn the_original_hd_capture_stepped_once_per_row() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let path = env_or("OAG_HD_AIR", "");
    let text = std::fs::read_to_string(&path).expect("reading the capture");
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().expect("header").split(',').collect();
    let rows: Vec<Vec<f32>> = lines
        .map(|l| {
            l.split(',')
                .map(|v| v.parse().unwrap_or(f32::NAN))
                .collect()
        })
        .collect();
    let at = |name: &str| header.iter().position(|h| *h == name).expect(name);
    let v3 = |r: &[f32], a: &str, b: &str, c: &str| Vec3::new(r[at(a)], r[at(b)], r[at(c)]);
    let (lo, hi) = env_or("OAG_HD_ROWS", "1260..1420")
        .split_once("..")
        .map(|(a, b)| (a.parse::<usize>().unwrap(), b.parse::<usize>().unwrap()))
        .unwrap();

    let image = oag_testdata::image(&env_or("OAG_HD_IMAGE", "hdfury-ps3-eu-dec.iso"))
        .expect("the HD image");
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: env_or("OAG_HD_CLASS", "PHANTOM"),
        mode: if env_or("OAG_HD_MODE", "") == "tt" {
            oag_race::Mode::TimeTrial
        } else {
            oag_race::Mode::SingleRace
        },
        track: Some(env_or(
            "OAG_HD_TRACK",
            "/data/environments/05_ubermall/track.vex",
        )),
        seed: Some(1),
        weapons_override: Some(false),
        team: Some(env_or("OAG_HD_TEAM", "Feisar")),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    let gravity = loaded.setup.class_gravity_scale;
    let mut race = oag_raceplay::Race::start(loaded.setup);
    // `OAG_HD_OPENLOOP=<ticks>`: fly our own race with no input and print the player's clamp and
    // body height per tick, to set against a `--ramp` capture's `timer` and `py`.
    if let Ok(ticks) = std::env::var("OAG_HD_OPENLOOP") {
        for _ in 0..ticks.parse::<u64>().unwrap_or(600) {
            race.tick_sim_only(&oag_gameplay::PlayerInputs::none());
            let ship = &race.sim.world.ships[0].physics;
            println!(
                "ol {} {:?} {:.4}",
                race.sim.world.tick, ship.hover_cap, ship.body.position.y
            );
        }
        return;
    }
    // One tick so the per-tick craft laws (hover rig, steering clamp) are written.
    race.tick_sim_only(&oag_gameplay::PlayerInputs::none());
    let mut template = race.sim.world.ships[0].physics;
    // The tick above ran on the grid, where HD caps the hover target low (`hover_cap`); the
    // capture starts after GO and fits no cap from its row 20, when the cap has released (`hover-target.md`).
    if std::env::var_os("OAG_HD_GRID_CAP").is_none() {
        template.hover_cap = None;
    }
    // `OAG_HD_SUM=0`: the hover escapes take the largest alone instead of HD's sum.
    if std::env::var("OAG_HD_SUM").as_deref() == Ok("0") {
        template.hover_rig.sum_escapes = false;
    }
    let mut handling = race.sim.world.ships[0].handling;
    // `OAG_HD_RIDE=<delta>`: the handling's ride height moved by `delta`, a sensitivity probe.
    if let Ok(delta) = std::env::var("OAG_HD_RIDE") {
        handling.antigrav.ride_height += delta.parse::<f32>().unwrap_or(0.0);
    }
    let world = race.collision();
    let term = match std::env::var("OAG_HD_GRAV").as_deref() {
        Ok("ground") => oag_physics::passive::ClassGravityTerm::Grounded,
        _ => oag_physics::passive::ClassGravityTerm::Airborne,
    };
    let base_env = Environment {
        class_gravity_scale: gravity,
        class_gravity_term: term,
        ..Environment::default()
    };
    // A row from the original's Autopilot (slot bit `0x800`) carries its rail, which writes the
    // craft's velocity, so a law cannot be read off it: such rows are refused unless
    // `OAG_HD_AUTOPILOT=1` asks for them (the 2026-10-19 capture predates this rule).
    let autopilot_ok = std::env::var_os("OAG_HD_AUTOPILOT").is_some();
    let bias = std::env::var("OAG_HD_BIAS").ok().map(|v| v == "1");
    println!(
        "gravity_mul {gravity} mass {} body mass {} normal_gravity {} flight_gravity {} wd {} bias {:?}",
        handling.physical.mass,
        template.body.mass,
        handling.physical.normal_gravity,
        handling.physical.flight_gravity,
        handling.dimensions.weight_distribution,
        bias
    );
    println!(
        "row  g  dt     | vy_meas  vy_ours  dvy  | dv_up  dv_fwd | dw_pitch dw_yaw dw_roll | w_pitch | dv_side dp_side"
    );
    for i in lo..hi.min(rows.len() - 1) {
        let (r, n) = (&rows[i], &rows[i + 1]);
        let (p0, p1) = (v3(r, "px", "py", "pz"), v3(n, "px", "py", "pz"));
        let v = v3(r, "vx", "vy", "vz");
        // A `--ramp` capture carries the hover clamp `craft+0x348`, a game-seconds timer
        // once the grid state ends: its step is the frame's own `dt`.
        let timer = header.iter().position(|h| *h == "timer");
        let raced = |row: &[f32]| row[at("state")] as u32 != 0;
        let dt = match timer {
            Some(t) if raced(r) && raced(n) => n[t] - r[t],
            _ => (p1 - p0).dot(v) / v.dot(v),
        };
        if !(0.005..0.05).contains(&dt) {
            continue;
        }
        let up = v3(r, "upx", "upy", "upz").normalize();
        let fw = v3(r, "fwx", "fwy", "fwz");
        let z = (-fw - up * (-fw).dot(up)).normalize();
        let x = up.cross(z);
        let orientation = Quat::from_mat3(&Mat3::from_cols(x, up, z)).normalize();
        let left = v3(r, "r0x", "r0y", "r0z");
        let w = v3(r, "wx", "wy", "wz");
        let omega = -(left * w.x + up * w.y + fw * w.z);
        if (r[at("slot_flags")] as u32) & 0x800 != 0 && !autopilot_ok {
            continue;
        }
        // The engine bits off the row's own `craft+0x264`: `0x8` uncapped thrust, `0x4` the
        // multiplier `craft+0x2f0` (both set only by the Autopilot's branch, `autopilot.md`).
        let row_flags = r[at("flags")] as u32;
        let env = Environment {
            engine_flags: EngineFlags {
                uncapped: row_flags & 0x8 != 0,
                speedup: (row_flags & 0x4 != 0).then_some(r[at("mult")]),
            },
            ..base_env
        };
        let grounded = if row_flags & 1 != 0 { 1.0 } else { 0.0 };
        let mut state = template;
        // `OAG_HD_RAMP=<lag>`: the release ramp `3.03825 + game seconds since GO`, taking the
        // capture's `clock` (`craft+0x308`, which runs at game time while grounded) from row 0
        // plus `lag` seconds as the time since GO.
        // `OAG_HD_RAMP=row`: the row's own clamp `+0x348` and grid state; `next`, the clamp
        // the next row reads (the timer steps before the target is built).
        let ramp = std::env::var("OAG_HD_RAMP");
        if let (Ok("row" | "next"), Some(t)) = (ramp.as_deref(), timer) {
            let source = if ramp.as_deref() == Ok("next") { n } else { r };
            state.hover_cap = Some(source[t]);
            state.on_grid = !raced(r);
        } else if let Ok(lag) = ramp {
            let lag: f32 = lag.parse().unwrap_or(0.0);
            state.hover_cap = Some(3.038_25 + lag + (r[at("clock")] - rows[0][at("clock")]));
        }
        state.body.position = p0;
        state.body.orientation = orientation;
        state.body.linear_velocity = v;
        state.body.angular_velocity = omega;
        state.grounded = grounded;
        state.grounded_prev = grounded;
        state.thrust = r[at("throttle")];
        if let Some(bias) = bias {
            state.air_pitch_bias = bias;
        }
        // The walk carries no track sample, so `step`'s pitch gate reads open; the gate is taken
        // here off the row's own located down (`ship+0x7850`) instead: a closed gate drops the
        // whole pitch input and keeps the bias (`oag_physics::engine::pitch`).
        // `OAG_HD_GATE=0` leaves it open.
        let located_down = v3(r, "sdx", "sdy", "sdz");
        let gate_closed = state.pitch_gate
            && std::env::var("OAG_HD_GATE").as_deref() != Ok("0")
            && up.dot(-located_down) <= oag_physics::engine::PITCH_ALIGNMENT;
        let controls = ShipControls {
            steer_x: r[at("c0")] / 100.0,
            thrust: r[at("c1")] / 100.0,
            airbrake_left: r[at("c2")] / 100.0,
            airbrake_right: r[at("c3")] / 100.0,
            steer_y: if gate_closed {
                0.0
            } else {
                r[at("c4")] / 100.0
            },
            ..ShipControls::default()
        };
        let ev = step(&mut state, &controls, &handling, &env, world, dt);
        if std::env::var_os("OAG_HD_DIAG").is_some() {
            let m = state.body.mass;
            let spring: Vec3 = ev
                .hover
                .active()
                .iter()
                .map(|p| p.force)
                .fold(Vec3::ZERO, |a, b| a + b);
            let hs: Vec<String> = ev
                .hover
                .active()
                .iter()
                .map(|p| {
                    if p.contact {
                        format!("{:.2}", p.height)
                    } else {
                        "-".into()
                    }
                })
                .collect();
            println!(
                "diag {i} dp_up {:.4} c{} h[{}] spring {:.2} down {:.2} grav {:.2} vdamp {:.2} wall {} floor {} wdv {:.3} wesc {:.3} hesc {:.3} tsl {:.3} gp {:.2} cap {:?}",
                (state.body.position - p1).dot(up),
                ev.hover.contacts,
                hs.join(" "),
                spring.dot(up) / m,
                ev.hover.downforce.dot(up) / m,
                ev.gravity.dot(up) / m,
                ev.vertical_damping.dot(up) / m,
                ev.wall.contacts,
                ev.wall.floor_contacts,
                ev.wall.velocity_delta.dot(up),
                ev.wall.escape.dot(up),
                ev.hover.escape.dot(up),
                template.time_since_landing,
                template.grounded_prev,
                r[at("grounded")],
            );
        }
        let v_next = v3(n, "vx", "vy", "vz");
        let (n_up, n_fw) = (v3(n, "upx", "upy", "upz"), v3(n, "fwx", "fwy", "fwz"));
        let n_left = v3(n, "r0x", "r0y", "r0z");
        let n_w = v3(n, "wx", "wy", "wz");
        let omega_next = -(n_left * n_w.x + n_up * n_w.y + n_fw * n_w.z);
        let dv = state.body.linear_velocity - v_next;
        let dw = (state.body.angular_velocity - omega_next) / dt;
        let right = -left;
        println!(
            "{i:>4} {grounded:.0} {dt:.4} | {:7.2} {:7.2} {:6.2} | {:6.2} {:6.2} | {:7.3} {:7.3} {:7.3} | {:6.3} | {:6.3} {:7.4}",
            v_next.y,
            state.body.linear_velocity.y,
            dv.y,
            dv.dot(up),
            dv.dot(fw),
            dw.dot(right),
            dw.dot(up),
            dw.dot(fw),
            omega_next.dot(right),
            // Sideways: the velocity and position residual along the craft's left, where a rail
            // (`Ai_SnapToLane`) would show.
            dv.dot(left),
            (state.body.position - p1).dot(left),
        );
    }
}
