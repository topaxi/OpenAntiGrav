//! Follow-up to `novice_respawn_ground_truth.rs`: **why** the grip sweep's
//! cliff between 0.40 and 0.48 exists, not just that it does - and, once that
//! is measured, whether anything in `oag-ai` closes it without moving
//! `Difficulty::grip_believed` itself. See
//! `docs/gameplay/ai.md`, "The jump-clearing failure" for the full writeup;
//! the short version is a real corner (not the gap) throttles the approach,
//! the threshold is `0.42..0.43` rather than the coarse sweep's `0.40..0.48`,
//! and the corner is geometrically capable of the needed speed at Novice's
//! own turn-rate degradation - so the shortfall is entirely
//! `grip_believed(Novice) = 0.30` being lower than this one jump needs, not a
//! controller bug.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-raceplay \
//!   --run-ignored all novice_jump_ground_truth --no-capture
//! ```
//!
//! Diagnostic only: every test here prints and does not assert, because the
//! finding is a shape, not a pass/fail. So each is a scratch sweep gated on
//! `OAG_SWEEP` and `just test-data` skips it. This file only reads
//! `crates/ai`'s public surface and `crates/game`'s own `race` module - it
//! does not touch `crates/raceplay/src/`.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;

const TRACK_ID: &str = "13_Track";
const LONE: usize = 1;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn entry_for(id: &str) -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
        .find(|track| track.id == id)
        .map(|track| track.entry_name())
}

/// Question 2: is the jump's approach gated by a corner, and does
/// `grip_believed` change the speed the craft carries into it?
///
/// Dumps `Line::curvature` for every index `0..30` (span 4.0, well inside a
/// hull length, just to see the shape), then runs a lone Novice craft at two
/// `grip_believed` values either side of the measured cliff and logs its
/// actual forward speed at every driver-line index `0..26` on the **first**
/// pass - before any respawn has had a chance to re-teleport it - so the
/// speeds compared are the ones flown by an undisturbed approach.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn what_the_jumps_approach_looks_like() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    let Some(image) = image() else { return };

    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Novice,
        track: Some(entry.clone()),
        ..race::Options::default()
    })
    .expect("loading 13_Track");
    let probe = race::Race::start(loaded.setup.clone());
    let line = probe.racing_line();
    println!("\ncurvature (span 4.0) and corridor by index, 0..55 on {TRACK_ID}:");
    for index in 0..55 {
        let aim = line.aim(index, 0.0);
        let corridor = aim
            .corridor
            .map(|frame| format!("left {:.2} right {:.2}", frame.left, frame.right))
            .unwrap_or_else(|| "none".to_string());
        let point = line.point(index);
        println!(
            "  {index:>3}: curvature {:.6}  point {point:?}  corridor {corridor}",
            line.curvature(index, 4.0)
        );
    }

    for grip in [0.30f32, 0.48] {
        let novice = oag_ai::Difficulty::Novice.tune(&oag_ai::Tuning::default());
        let tuning = oag_ai::Tuning {
            lateral_accel: oag_ai::Tuning::default().lateral_accel * grip,
            ..novice
        };
        let mut race = race::Race::start(loaded.setup.clone());
        race.set_ai_tuning(tuning);
        for slot in 2..8 {
            race.sim.world.ships[slot].active = false;
        }
        race.sim.world.ships[0].active = false;

        let mut seen = [false; 30];
        let mut logged = 0usize;
        // The grid start is not index 0 - it can be anywhere on a 3,084-point
        // line - so "started" only latches once the driver has actually been
        // seen inside the window this pass is about, and everything before
        // that (including the initial large index while still approaching
        // from the grid) is not logged as though it were part of it.
        let mut started = false;
        println!("\ngrip_believed {grip}: speed by index, first pass through 0..26:");
        for _tick in 0..18_000 {
            race.tick(&PlayerInputs::none());
            let ship = &race.sim.world.ships[LONE];
            let index = ship.driver.index as usize;
            if index < 5 {
                started = true;
            }
            if started && index < 30 && !seen[index] {
                seen[index] = true;
                let forward = ship.physics.body.forward();
                let speed = ship.physics.body.linear_velocity.dot(forward);
                println!(
                    "  index {index:>3}: speed {speed:.2} airborne {}",
                    ship.physics.time_airborne > 0.0
                );
                logged += 1;
            }
            if (started && index >= 26) || race.respawns_of(LONE) > 0 {
                break;
            }
        }
        println!("  ({logged} indices logged before landing or a respawn)");
    }
}

/// One lone Novice-tuned craft on `13_Track`, with `lateral_accel` scaled by
/// `grip` (everything else - `max_turn_rate`, `mistake_rate`,
/// `reaction_ticks` - held at Novice's own value), for 18,000 ticks. Returns
/// `(respawns, laps, liftoff_speed, first_landing_index)`, where
/// `liftoff_speed` is the forward speed at the first tick the driver is found
/// airborne and `first_landing_index` is the driver-line index where the
/// first airborne window ends.
fn run_grip(setup: &race::Setup, grip: f32) -> (u32, u32, f32, u32) {
    let novice = oag_ai::Difficulty::Novice.tune(&oag_ai::Tuning::default());
    let tuning = oag_ai::Tuning {
        lateral_accel: oag_ai::Tuning::default().lateral_accel * grip,
        ..novice
    };
    let mut race = race::Race::start(setup.clone());
    race.set_ai_tuning(tuning);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;

    let mut liftoff_speed = -1.0f32;
    let mut landing_index = 0u32;
    let mut was_airborne = false;
    for _tick in 0..18_000u32 {
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[LONE];
        let airborne_now = ship.physics.time_airborne > 0.0;
        if airborne_now && !was_airborne && liftoff_speed < 0.0 {
            let forward = ship.physics.body.forward();
            liftoff_speed = ship.physics.body.linear_velocity.dot(forward);
        }
        if !airborne_now && was_airborne && landing_index == 0 {
            landing_index = ship.driver.index;
        }
        was_airborne = airborne_now;
        if race.respawns_of(LONE) > 0 {
            break;
        }
    }
    (
        race.respawns_of(LONE),
        race.sim.world.ships[LONE].standing.lap,
        liftoff_speed,
        landing_index,
    )
}

/// Question 2, the discriminating case the grip sweep alone cannot answer:
/// **is the jump reachable by raising grip at all**, or does
/// `Tuning::max_turn_rate` (scaled by `turn_allowed`, untouched by this
/// sweep) put a lower ceiling under the corner regardless of grip? If the
/// craft still lands short of index 50 once `lateral_accel` is scaled so far
/// past the sweep's clean rows that the grip term cannot possibly bind, the
/// yaw term is what is actually limiting the approach, and no amount of
/// throttle-side change in `oag-ai` reaches this jump.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn novice_13_track_grip_ceiling() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Novice,
        track: Some(entry.clone()),
        ..race::Options::default()
    })
    .expect("loading 13_Track");
    println!("\ngrip  respawns  laps  liftoff_speed  first_landing_index");
    for grip in [0.48f32, 1.0, 5.0, 20.0, 100.0] {
        let (respawns, laps, liftoff_speed, landing_index) = run_grip(&loaded.setup, grip);
        println!("{grip:<6}{respawns:<10}{laps:<6}{liftoff_speed:<15.2}{landing_index}");
    }
}

/// Question 2, the fine sweep the prior pass left unswept: `0.41..=0.47`, at
/// `0.01` steps, between the two points the coarse sweep already measured
/// (`0.40` at 350 respawns, `0.48` at 0). Turns "somewhere in that interval"
/// into an actual number.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn novice_13_track_grip_fine_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Novice,
        track: Some(entry.clone()),
        ..race::Options::default()
    })
    .expect("loading 13_Track");
    println!("\ngrip  respawns  laps  liftoff_speed  first_landing_index");
    let mut grip = 0.41f32;
    while grip <= 0.470_001 {
        let (respawns, laps, liftoff_speed, landing_index) = run_grip(&loaded.setup, grip);
        println!("{grip:<6.2}{respawns:<10}{laps:<6}{liftoff_speed:<15.2}{landing_index}");
        grip += 0.01;
    }
}

/// Arc length along the racing line to every sample, so a position projects
/// to a distance travelled rather than to an index (samples are unevenly
/// spaced).
fn arc_lengths(line: &oag_ai::Line) -> Vec<f32> {
    let mut out = Vec::with_capacity(line.len());
    let mut total = 0.0f32;
    for index in 0..line.len() {
        out.push(total);
        total += (line.point(index + 1) - line.point(index)).length();
    }
    out
}

/// One tick of one craft near the jump, as a CSV row: where it is along the
/// line, its velocity split along the line's own tangent, sideways and
/// vertically, the forward-axis speed the earlier passes logged, its
/// attitude and its ground contact.
fn flight_row(
    label: &str,
    race: &race::Race,
    arcs: &[f32],
    slot: usize,
    previous: Option<oag_core::math::Vec3>,
) -> String {
    let line = race.racing_line();
    let ship = &race.sim.world.ships[slot];
    let index = ship.driver.index as usize;
    let here = line.point(index);
    let tangent = (line.point(index + 1) - here).normalize();
    let flat = oag_core::math::Vec3::new(tangent.x, 0.0, tangent.z).normalize();
    let side = oag_core::math::Vec3::new(-flat.z, 0.0, flat.x);
    let body = &ship.physics.body;
    let velocity = body.linear_velocity;
    let forward = body.forward();
    let up = body.up();
    let along = (body.position - here).dot(tangent);
    let accel = previous.map_or(oag_core::math::Vec3::ZERO, |was| (velocity - was) * 60.0);
    format!(
        "{label},{},{slot},{index},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.3},{:.3},{:.3},{:.3},{:.2},{:.3},{:.2},{:.2},{:.2},{:.2},{},{}",
        race.sim.world.tick,
        arcs[index] + along,
        body.position.y,
        body.position.y - here.y,
        velocity.dot(tangent),
        velocity.dot(side),
        velocity.y,
        velocity.dot(forward),
        velocity.length(),
        (velocity.x * velocity.x + velocity.z * velocity.z).sqrt(),
        forward.y,
        up.x,
        up.y,
        up.z,
        ship.physics.grounded,
        ship.physics.time_airborne,
        ship.physics.thrust,
        accel.dot(flat),
        accel.y,
        ship.physics.brake,
        ship.physics.wall_contact_prev,
        race.respawns_of(slot),
    )
}

const FLIGHT_HEADER: &str = "run,tick,slot,idx,arc,y,y_above_line,v_along,v_lat,v_vert,speed_fwd,v_len,v_horiz,fwd_y,up_x,up_y,up_z,grounded,t_air,thrust,a_flat,a_vert,brake,wall,respawns";

/// Every tick of a craft's first pass over the jump (`idx` 10..70) until it
/// is past it or respawns, as CSV: the per-tick table `docs/gameplay/ai.md`,
/// "The 13_Track jump, flown tick by tick", is read from. Two runs: a lone
/// Novice craft on this project's driver, and the original driver's field
/// of seven with the player parked on the grid (seed 1, VENOM, single race).
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn the_jump_flown_tick_by_tick() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    let Some(image) = image() else { return };
    let options = |difficulty| race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty,
        track: Some(entry.clone()),
        seed: Some(1),
        ..race::Options::default()
    };

    println!("{FLIGHT_HEADER}");
    let novice = race::load(&options(oag_ai::Difficulty::Novice)).expect("loading 13_Track");
    let mut lone = race::Race::start(novice.setup);
    for slot in (0..8).filter(|&slot| slot != LONE) {
        lone.sim.world.ships[slot].active = false;
    }
    let arcs = arc_lengths(lone.racing_line());
    log_first_passes("novice", &mut lone, &arcs, &[LONE], 18_000);

    let field = race::load(&options(race::Options::default().difficulty)).expect("loading");
    let mut parked = race::Race::start(field.setup);
    assert!(parked.use_original_ai(None, 1), "VENOM's AI tables read");
    let slots: Vec<usize> = (1..8).collect();
    log_first_passes("original", &mut parked, &arcs, &slots, 9_000);
}

/// Ticks `race` with no player input and prints [`flight_row`] for each of
/// `slots` while it is inside `idx` 10..70 on its first pass, stopping that
/// slot's log once it is past index 70 or 60 ticks after its first respawn.
fn log_first_passes(label: &str, race: &mut race::Race, arcs: &[f32], slots: &[usize], ticks: u32) {
    let mut entered = [false; 8];
    let mut done = [false; 8];
    let mut after_respawn = [0u32; 8];
    let mut previous = [None; 8];
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
        for &slot in slots {
            if done[slot] {
                continue;
            }
            let index = race.sim.world.ships[slot].driver.index as usize;
            let inside = (10..70).contains(&index);
            if inside {
                entered[slot] = true;
            }
            if entered[slot] && (inside || race.respawns_of(slot) > 0) {
                println!("{}", flight_row(label, race, arcs, slot, previous[slot]));
            }
            previous[slot] = Some(race.sim.world.ships[slot].physics.body.linear_velocity);
            if race.respawns_of(slot) > 0 {
                after_respawn[slot] += 1;
            }
            if entered[slot] && (index >= 70 || after_respawn[slot] > 60) {
                done[slot] = true;
            }
        }
        if slots.iter().all(|&slot| done[slot]) {
            break;
        }
    }
}

/// Loads the parked-player scenario `the_jump_flown_tick_by_tick` runs: the
/// original driver's field of seven, VENOM, seed 1, single race.
fn parked_field() -> Option<(race::Setup, race::Race)> {
    let entry = entry_for(TRACK_ID)?;
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(entry),
        seed: Some(1),
        ..race::Options::default()
    })
    .expect("loading 13_Track");
    let mut race = race::Race::start(loaded.setup.clone());
    assert!(race.use_original_ai(None, 1), "VENOM's AI tables read");
    Some((loaded.setup, race))
}

/// The original's reset puts a craft that falls into `13_Track`'s jump back on
/// the road behind it, moving (`oag_raceplay`'s `breadcrumbs` module), and so
/// does ours: the parked-player field comes back where the original's does,
/// at the speed its law gives, and falls a handful of times rather than in a
/// loop.
///
/// The window is the original's, read live on PPSSPP (`docs/gameplay/ai.md`,
/// "What the original does after a fall"): AI resets at line rows 2938 to 2953,
/// natural Time Trial falls all at row 2949, widened by two samples a side for
/// this project's nearest-sample locate. The speed is `50 + 0.25 * latch`, the
/// latch being the craft's speed on its last fully grounded tick; the original
/// held it to 0.1 u/s on five of five falls. Under the old pose (at rest, on
/// the sample nearest the fall, over the gap) the same 9,000 ticks gave 389
/// respawns; 2 measured on 2026-10-13 after the port.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_parked_field_is_put_back_where_the_original_puts_it() {
    let Some((_, mut race)) = parked_field() else {
        return;
    };
    let mut seen = [0u32; 8];
    let mut latch = [0.0f32; 8];
    for _ in 0..9_000 {
        race.tick(&PlayerInputs::none());
        for (slot, seen) in seen.iter_mut().enumerate().skip(1) {
            let ship = &race.sim.world.ships[slot];
            if race.respawns_of(slot) == *seen {
                if ship.physics.grounded == 1.0 {
                    latch[slot] = ship.physics.body.linear_velocity.length();
                }
                continue;
            }
            *seen = race.respawns_of(slot);
            let index = ship.driver.index as usize;
            assert!(
                (2936..=2955).contains(&index),
                "slot {slot} was put back at line row {index}, the original at 2938-2953"
            );
            let line = race.racing_line();
            let velocity = ship.physics.body.linear_velocity;
            let along = (line.point(index + 1) - line.point(index)).normalize();
            let want = 50.0 + 0.25 * latch[slot];
            assert!(
                (velocity.length() - want).abs() < 1e-3
                    && velocity.dot(along) > 0.9 * velocity.length(),
                "slot {slot} was put back moving {velocity:?}, want {want} along {along:?}"
            );
        }
    }
    let total: u32 = seen.iter().sum();
    assert!(
        total < 20,
        "{total} respawns: the parked field is falling in a loop"
    );
}

/// Every respawn of the parked-player field over 9,000 ticks: which slot,
/// which trigger, the line index it was lost at and the one it was put back
/// on, and its speed the tick before. `OAG_FLIGHT_GRAVITY_SCALE` multiplies
/// every craft's `flight_gravity` first, a counterfactual for which gravity
/// lane the class scale belongs on.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn the_parked_field_at_the_jump() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some((_, mut race)) = parked_field() else {
        return;
    };
    let scale: f32 = std::env::var("OAG_FLIGHT_GRAVITY_SCALE")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1.0);
    for ship in &mut race.sim.world.ships {
        ship.handling.physical.flight_gravity *= scale;
    }
    println!("flight_gravity scale {scale}");
    println!("tick,slot,cause,lost_idx,put_idx,speed_before,y_before,put_y");
    let mut counts = [0u32; 8];
    let mut before = [(0u32, 0.0f32, 0.0f32); 8];
    for _ in 0..9_000 {
        race.tick(&PlayerInputs::none());
        for slot in 1..8 {
            let ship = &race.sim.world.ships[slot];
            let respawns = race.respawns_of(slot);
            if respawns > counts[slot] {
                counts[slot] = respawns;
                let cause = race
                    .last_respawn_cause_of(slot)
                    .map_or("?", |cause| cause.label());
                let (lost, speed, y) = before[slot];
                println!(
                    "{},{slot},{cause},{lost},{},{speed:.2},{y:.2},{:.2}",
                    race.sim.world.tick, ship.driver.index, ship.physics.body.position.y,
                );
            }
            before[slot] = (
                ship.driver.index,
                ship.physics.body.linear_velocity.length(),
                ship.physics.body.position.y,
            );
        }
    }
    println!(
        "flight_gravity scale {scale}: respawns per slot 1..8 {:?}, total {}, \
         flight_gravity now {}",
        &counts[1..],
        counts.iter().sum::<u32>(),
        race.sim.world.ships[3].handling.physical.flight_gravity,
    );
}

/// What the far lip is made of: rays along the line's tangent at a ladder
/// of heights relative to the line, from a point inside the gap, and rays
/// straight down along the line either side of it. Answers whether what a
/// low craft hits is the far road's own front face or something standing
/// proud of it.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn the_far_lip_face() {
    use oag_physics::collide::{Ray, Raycaster};
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some((setup, race)) = parked_field() else {
        return;
    };
    let line = race.racing_line();
    let arcs = arc_lengths(line);
    println!("down casts: idx,arc,line_y,surface_y,normal,surface");
    for index in (10..60).step_by(2) {
        let point = line.point(index);
        let ray = Ray::new(
            point + oag_core::math::Vec3::new(0.0, 50.0, 0.0),
            oag_core::math::Vec3::new(0.0, -1.0, 0.0),
            100.0,
        );
        let hit = setup.collision.raycast(ray, None, true);
        println!(
            "{index},{:.2},{:.2},{},{}",
            arcs[index],
            point.y,
            hit.map_or("none".to_string(), |hit| format!("{:.2}", hit.point.y)),
            hit.map_or("-".to_string(), |hit| format!(
                "{:.2} {:.2} {:.2},{:?}",
                hit.normal.x, hit.normal.y, hit.normal.z, hit.surface
            )),
        );
    }
    let flagged: Vec<usize> = (0..line.len())
        .filter(|&index| {
            race.ai_sample(index)
                .is_some_and(|sample| sample.flags & 1 != 0)
        })
        .collect();
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &index in &flagged {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == index => *end = index,
            _ => runs.push((index, index)),
        }
    }
    println!(
        "\nflags bit 0 set on {} of {} line indices, runs {runs:?}",
        flagged.len(),
        line.len(),
    );
    for index in (0..70).step_by(5) {
        if let Some(sample) = race.ai_sample(index) {
            println!(
                "  idx {index}: flags {:#04x} half_width {:.2}/{:.2} ai_bound {:.2}/{:.2} line {:.2}",
                sample.flags,
                sample.half_width_left,
                sample.half_width_right,
                sample.ai_bound_left,
                sample.ai_bound_right,
                sample.racing_line
            );
        }
    }
    let from = 40;
    let start = line.point(from);
    let tangent = (line.point(from + 4) - start).normalize();
    let flat = oag_core::math::Vec3::new(tangent.x, 0.0, tangent.z).normalize();
    println!(
        "\nforward casts from idx {from} (arc {:.2}), along flat tangent:",
        arcs[from]
    );
    println!("dy_vs_line,hit_arc,hit_y,normal,surface");
    let mut dy = -6.0f32;
    while dy <= 2.01 {
        let origin = start + oag_core::math::Vec3::new(0.0, dy, 0.0);
        let hit = setup
            .collision
            .raycast(Ray::new(origin, flat, 30.0), None, true);
        match hit {
            Some(hit) => println!(
                "{dy:.1},{:.2},{:.2},{:.2} {:.2} {:.2},{:?}",
                arcs[from] + hit.distance,
                hit.point.y,
                hit.normal.x,
                hit.normal.y,
                hit.normal.z,
                hit.surface
            ),
            None => println!("{dy:.1},none"),
        }
        dy += 0.5;
    }
}

/// Contiguous runs of `true` in a ring of `len` entries, as inclusive
/// `(first, last)` pairs.
fn runs_of(len: usize, set: impl Fn(usize) -> bool) -> Vec<(usize, usize)> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for index in (0..len).filter(|&index| set(index)) {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == index => *end = index,
            _ => runs.push((index, index)),
        }
    }
    runs
}

/// Every forward circuit: where its spline authors `flags` bit 0, against
/// where its racing line has no surface under it.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn the_flag_bit_zero_census() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    for track in catalogue::tracks(&definition) {
        let Ok(loaded) = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            track: Some(track.entry_name()),
            ..race::Options::default()
        }) else {
            continue;
        };
        let race = race::Race::start(loaded.setup);
        let line = race.racing_line();
        let flagged = runs_of(line.len(), |index| {
            race.ai_sample(index)
                .is_some_and(|sample| sample.flags & 1 != 0)
        });
        let other = runs_of(line.len(), |index| {
            race.ai_sample(index)
                .is_some_and(|sample| sample.flags & !1 != 0)
        });
        let gaps = runs_of(line.len(), |index| line.is_unsupported(index));
        println!(
            "{} rev {} len {}: bit0 {flagged:?} other_bits {other:?} unsupported {gaps:?}",
            track.id,
            track.reversed,
            line.len()
        );
    }
}

/// The speed pad before the jump (`oag-trace pads` pad 6, on line row 3073,
/// about 45 units before the gap): each pad near that row, its signed lateral
/// offset from the line along the spline's `lateral`, and then every crossing
/// of row 3073 by the parked-player field over 9,000 ticks, with the craft's
/// signed lateral, its speed either side, and whether the pad armed
/// (`ShipState::pad_timer`). The original's boosted crossings sit at -9.8 to
/// -1.9 (`docs/gameplay/ai.md`).
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn the_pad_before_the_jump() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some((_, mut race)) = parked_field() else {
        return;
    };
    const ROW: usize = 3073;
    let lateral_at = |race: &race::Race, index: usize, position: oag_core::math::Vec3| {
        race.ai_sample(index).map_or(f32::NAN, |sample| {
            let axis = oag_core::math::Vec3::from_array(sample.lateral).normalize();
            (position - oag_core::math::Vec3::from_array(sample.pos)).dot(axis)
        })
    };
    let line = race.racing_line();
    for (n, pad) in race.speedup_pads().iter().enumerate() {
        let centre =
            oag_core::math::Vec3::new(pad.to_world[12], pad.to_world[13], pad.to_world[14]);
        let index = line.nearest(centre, 0, line.len());
        if (3040..3100).contains(&index) {
            println!(
                "pad {n}: centre {centre:?} line row {index} lateral {:.2} box {:?}..{:?}",
                lateral_at(&race, index, centre),
                pad.min,
                pad.max
            );
        }
    }
    println!("tick,slot,lap,lateral,speed_before,speed_after,pad_timer,place");
    let mut before = [(0u32, 0.0f32); 8];
    let mut pending = [None::<(u64, f32)>; 8];
    for _ in 0..9_000 {
        race.tick(&PlayerInputs::none());
        let places = race.places();
        for slot in 1..8 {
            let ship = &race.sim.world.ships[slot];
            let index = ship.driver.index;
            let speed = ship.physics.body.linear_velocity.length();
            if let Some((at, was)) = pending[slot]
                && race.sim.world.tick == at + 20
            {
                println!("gain,{slot},{at},{:.1}", speed - was);
                pending[slot] = None;
            }
            if before[slot].0 < ROW as u32 && index >= ROW as u32 && index < ROW as u32 + 30 {
                pending[slot] = Some((race.sim.world.tick, before[slot].1));
                println!(
                    "{},{slot},{},{:.2},{:.1},{:.1},{:.3},{}",
                    race.sim.world.tick,
                    ship.standing.lap,
                    lateral_at(&race, index as usize, ship.physics.body.position),
                    before[slot].1,
                    speed,
                    ship.physics.pad_timer,
                    places[slot]
                );
            }
            before[slot] = (index, speed);
        }
    }
}
