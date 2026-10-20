//! A scratch sweep: HD Chenghou Project, Time Trial, Piranha, the player on the
//! autopilot, per tick height above the spline, vertical velocity and the cage's push.
//!
//! Prints only. `#[ignore]`d, and returns early unless `OAG_SWEEP` is set:
//!
//! ```text
//! OAG_SWEEP=1 OAG_CAGE=off OAG_TICKS=900 \
//!     cargo nextest run -p oag-raceplay hd_chenghou_jumps_sweep --run-ignored all --no-capture
//! ```

use oag_core::math::{Mat3, Quat, Vec3};

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

#[test]
#[ignore = "scratch sweep: OAG_SWEEP=1, needs data/images/"]
fn chenghou_jumps_per_tick() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let image = oag_testdata::image(&env_or("OAG_HD_IMAGE", "hdfury-ps3-eu-dec.iso"))
        .expect("the HD image");
    let mut loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: env_or("OAG_HD_CLASS", "VENOM"),
        mode: oag_race::Mode::TimeTrial,
        track: Some(env_or(
            "OAG_HD_TRACK",
            "/data/environments/04_chenghou_project/track.vex",
        )),
        seed: Some(1),
        team: Some(env_or("OAG_HD_TEAM", "Piranha")),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        if line.contains("cage") {
            println!("report: {line}");
        }
    }
    if env_or("OAG_CAGE", "on") == "off" {
        loaded.setup.spline = loaded.setup.spline.clone().with_cage(None);
    }
    let mut race = oag_raceplay::Race::start(loaded.setup);
    // `OAG_PLACE=x,y,z,fx,fy,fz,ux,uy,uz,speed`: after the idle ticks (past GO), put the
    // player there, velocity along forward, no spin, then hold thrust alone, as
    // `scripts/rpcs3_place.py` does on the original.
    let place: Option<Vec<f32>> = std::env::var("OAG_PLACE")
        .ok()
        .map(|v| v.split(',').map(|x| x.parse().unwrap()).collect());
    let mut thrust = oag_gameplay::input::Input::new();
    if let Some(p) = &place {
        // `OAG_IDLE=<ticks>` idle ticks before the placement. The default, 1500, is past the
        // hover cap's release (`hover-target.md`): at 300 the cap still read `3.52`, so the
        // placed craft found no floor on the ramp and took off about 2 units low, which read as
        // a takeoff residual (`docs/ghidra/functions/ps3-hdfury-eu/collision-cage.md`).
        let idle: u64 = env_or("OAG_IDLE", "1500").parse().unwrap();
        for _ in 0..idle {
            race.tick_sim_only(&oag_gameplay::PlayerInputs::none());
        }
        println!(
            "placed after {idle} idle ticks: hover cap {:?}, on grid {}",
            race.sim.world.ships[0].physics.hover_cap, race.sim.world.ships[0].physics.on_grid
        );
        let forward = Vec3::new(p[3], p[4], p[5]).normalize();
        let up0 = Vec3::new(p[6], p[7], p[8]);
        let up = (up0 - forward * up0.dot(forward)).normalize();
        let right = forward.cross(up);
        let body = &mut race.sim.world.ships[0].physics.body;
        body.position = Vec3::new(p[0], p[1], p[2]);
        body.orientation = Quat::from_mat3(&Mat3::from_cols(right, up, -forward));
        body.linear_velocity = forward * p[9];
        body.angular_velocity = Vec3::ZERO;
    } else {
        race.set_autopilot(true);
    }
    let ticks: u64 = env_or("OAG_TICKS", "1200").parse().unwrap();
    println!("tick x y z vy height grounded speed fx fy fz ux uy uz");
    for _ in 0..ticks {
        let inputs = if place.is_some() {
            thrust.begin_frame(oag_gameplay::input::Button::Cross.bit());
            oag_gameplay::PlayerInputs::single(oag_gameplay::InputSnapshot {
                buttons: thrust,
                ..oag_gameplay::InputSnapshot::new()
            })
        } else {
            oag_gameplay::PlayerInputs::none()
        };
        race.tick_sim_only(&inputs);
        let t = race.telemetry();
        let body = &race.sim.world.ships[0].physics.body;
        let v = body.linear_velocity;
        let f = body.orientation * Vec3::NEG_Z;
        let u = body.orientation * Vec3::Y;
        println!(
            "t {} {:.2} {:.2} {:.2} {:.3} {:.3} {:.2} {:.1} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            t.tick,
            t.position.x,
            t.position.y,
            t.position.z,
            v.y,
            t.height_above_spline,
            t.grounded,
            t.speed,
            f.x,
            f.y,
            f.z,
            u.x,
            u.y,
            u.z
        );
    }
}
