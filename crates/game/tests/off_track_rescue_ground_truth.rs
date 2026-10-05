//! A player who leaves a real circuit is put back on it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! cargo nextest run -p oag-game --run-ignored all -E 'binary(off_track_rescue_ground_truth)'
//! ```
//!
//! # Why this needs a disc
//!
//! The authored `Reset` volumes recover a craft that falls *through* a floor.
//! They do not recover one that leaves the circuit sideways into open space,
//! because out there is nothing of any class to touch - `oag_race::recovery::RESCUE_HALF_WIDTHS`
//! records the measurement on the opponents' side, and `docs/gameplay/ai.md` has
//! the same drop taken by a human at the controls with no AI involved.
//!
//! The failure is not reachable synthetically: it is a property of shipped track
//! geometry that a step down bigger than the hover probes' reach takes the surface
//! out of view, after which the craft travels forward faster than gravity brings
//! it back. Measured here before the rescue existed, one autopiloted craft alone
//! at ace (from the pre-2026-09-29 start; the test now uses Elite, see it): on `05_Track` it passed 20 units from the sample table at tick 591,
//! never came back, and was **7,983 units** away by tick 6,000.
//!
//! So the four tests are: the bound on a player shoved off an open edge (the
//! autopilot produced the failure unprompted until 2026-09-29, and no longer
//! does anywhere), the reported failure reproduced on the circuit it was reported on,
//! the count of `Reset` colliders that says why those volumes cannot be the
//! answer, and a control on the circuits that never leave at all.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// The disc, or `None` on a checkout without one.
fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// What one lone craft did over a run.
#[derive(Debug, Default)]
struct Solo {
    /// The furthest it ever got from the nearest spline sample.
    peak: f32,
    /// The circuit's widest half-width, which is the scale
    /// [`oag_race::recovery::PLAYER_RESCUE_HALF_WIDTHS`] is a multiple of.
    half_width: f32,
    /// Laps reached.
    laps: u32,
    /// Times it was put back, by either of the player's two triggers.
    respawns: u32,
    /// The longest unbroken run of ticks spent below one unit per second while
    /// asking to move - the stall condition the *opponents'* rescue answers, which
    /// the player deliberately does not have.
    longest_stall: u32,
}

/// Drives the **player's** craft alone round `track` under autopilot.
///
/// Autopilot rather than a scripted input, because it is slot 0 going through
/// `Race::tick`: the same code path, the same triggers and the same recovery a
/// human gets - see `Race::set_autopilot`. Alone, so nothing that happens to it
/// is about anybody else.
fn solo_player(level: oag_ai::Difficulty, track: &str, ticks: u64) -> Option<Solo> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for slot in 1..oag_gameplay::MAX_SHIPS {
        race.sim.world.ships[slot].active = false;
    }
    race.set_autopilot(true);

    let mut solo = Solo {
        half_width: race.spline().max_half_width(),
        ..Solo::default()
    };
    let mut stall = 0u32;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[0];
        let position = ship.physics.body.position;
        if let Some(distance) = race.spline().distance_to(position) {
            solo.peak = solo.peak.max(distance);
        }
        let stopped = ship.physics.thrust > 0.0
            && !ship.standing.finished()
            && ship.physics.body.linear_velocity.length() < oag_race::recovery::STALL_SPEED;
        stall = if stopped { stall + 1 } else { 0 };
        solo.longest_stall = solo.longest_stall.max(stall);
    }
    solo.laps = race.sim.world.ships[0].standing.lap;
    solo.respawns = race.respawns();
    Some(solo)
}

/// The player's craft alone on `track`, placed on the racing line at spline
/// sample `index`, turned `degrees` to the left off the tangent, moving at
/// `speed` along that heading with the throttle held for `ticks` - a player who
/// steers off an open edge, without an input model. The same placement as
/// `falloff_survey_ground_truth.rs`'s shove sweep.
fn shoved_player(track: &str, index: usize, degrees: f32, speed: f32, ticks: u64) -> Option<Solo> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for slot in 1..oag_gameplay::MAX_SHIPS {
        race.sim.world.ships[slot].active = false;
    }
    let mut buttons = oag_gameplay::input::Input::new();
    buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    let throttle = oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    };
    // Past the countdown, so the craft is allowed to move.
    for _ in 0..600 {
        race.tick(&PlayerInputs::single(throttle));
    }
    let height = oag_gameplay::spawn::spawn_height(&race.sim.world.ships[0].handling);
    let sample = race.spline().sample(index).copied()?;
    let mut pose = oag_gameplay::Pose::from_sample(&sample, sample.racing_line, height);
    let up = pose.orientation * oag_core::math::Vec3::Y;
    pose.orientation =
        oag_core::math::Quat::from_axis_angle(up, -degrees.to_radians()) * pose.orientation;
    let forward = pose.orientation * oag_core::math::Vec3::NEG_Z;
    race.sim.world.ships[0].place_at(pose);
    race.sim.world.ships[0].physics.body.linear_velocity = forward * speed;

    let mut solo = Solo {
        half_width: race.spline().max_half_width(),
        ..Solo::default()
    };
    let before = race.respawns();
    for _ in 0..ticks {
        race.tick(&PlayerInputs::single(throttle));
        let position = race.sim.world.ships[0].physics.body.position;
        if let Some(distance) = race.spline().distance_to(position) {
            solo.peak = solo.peak.max(distance);
        }
    }
    solo.laps = race.sim.world.ships[0].standing.lap;
    solo.respawns = race.respawns() - before;
    Some(solo)
}

/// **A player who comes off a real circuit stops receding.**
///
/// **A placed shove off `04_Track`'s open edge since 2026-09-29.** The subject
/// used to be the player's autopilot leaving `05_Track` on its own, first at
/// Ace and then at Elite; with the hull port and the AI's gap-aware braking
/// (`docs/gameplay/leaving-the-track.md`) no tier on any forward circuit sends
/// the autopilot off through `OffTrack` any more, measured over all twelve and
/// all four tiers. The failure this test owns is a property of the geometry -
/// an edge with no wall and a drop beyond it - so it is now staged directly:
/// `04_Track` sample 0, turned 45 degrees left at 110 u/s, throttle held, one of
/// the shove sweep's leaks. Measured: it passes 200 units from the circuit
/// before `OffTrack` puts it back.
///
/// The bound asserted is the one the mechanism guarantees rather than a fitted
/// number - a craft cannot recede past the threshold for longer than
/// [`oag_race::recovery::PLAYER_RESCUE_TICKS`], so its distance is bounded by
/// how far a falling craft travels in three quarters of a second.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_player_that_leaves_a_real_circuit_stops_receding() {
    let Some(solo) = shoved_player(
        "Data\\Environments\\04_Track\\track.vex",
        0,
        45.0,
        110.0,
        600,
    ) else {
        return;
    };
    println!(
        "04_Track shove: peak {:.1} ({:.1} half-widths), respawns {}",
        solo.peak,
        solo.peak / solo.half_width,
        solo.respawns
    );
    assert!(
        solo.respawns > 0,
        "nothing recovered the player, so this placement no longer measures anything"
    );
    // Before the rescue existed, a craft that left this way receded for the
    // rest of the race: 7,983 units on `05_Track` by tick 6,000.
    assert!(
        solo.peak < 1_000.0,
        "the player got {:.1} units from the circuit, so the rescue is not bounding it",
        solo.peak
    );
}

/// **The reported failure, on the circuit it was reported on.**
///
/// A hand-driven lap of Vertica reversed (`14_Track`) on 2026-08-17: the craft
/// took a Turbo onto a crest, left the circuit at about 240 units per second and
/// fell for the remaining eight seconds of the log - `spline` 59, 147, 227, 355,
/// 521, 666, 802, 954, `height` down to -866, with no recovery of any kind. This
/// reproduces the departure with an impulse rather than a Turbo, because what the
/// rescue answers is a craft leaving at speed and not what launched it.
///
/// The count of `Reset` colliders is printed alongside, and it settles why the
/// authored volumes cannot be the answer on this circuit: **there are none at
/// all.** See [`not_every_circuit_authors_reset_geometry`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_player_thrown_off_a_real_circuit_at_speed_comes_back() {
    let Some(image) = image() else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some("Data\\Environments\\14_Track\\track_reversed.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let reset_colliders = loaded
        .setup
        .collision
        .colliders()
        .iter()
        .filter(|collider| collider.surface() == oag_physics::Surface::Reset)
        .count();
    println!("14_Track reversed authors {reset_colliders} Reset collider(s)");

    let mut race = race::Race::start(loaded.setup);
    for slot in 1..oag_gameplay::MAX_SHIPS {
        race.sim.world.ships[slot].active = false;
    }
    race.set_autopilot(true);

    // A lap's worth of driving first, so the craft leaves from the circuit rather
    // than from the grid - and **past the start-line countdown before that
    // starts counting**. `RaceState` gates thrust for the measured
    // `oag_race::COUNTDOWN_TICKS` (272), so 600 ticks was 328 of driving from a
    // standing start and the craft was thrown from near the grid rather than
    // from the circuit. Red since `b4bb23ee` landed the countdown, and
    // invisible to `just` because this file never runs in CI.
    for _ in 0..oag_race::COUNTDOWN_TICKS + 600 {
        race.tick(&PlayerInputs::none());
    }
    let from = race.sim.world.ships[0].physics.body.position;
    // Off the side and up, at the speed the log recorded. Sideways rather than
    // down, deliberately: straight down is the case the authored `Reset` volumes
    // already answer, and this test is about the one they do not.
    let velocity = &mut race.sim.world.ships[0].physics.body.linear_velocity;
    *velocity = oag_core::math::Vec3::new(velocity.x, 60.0, 0.0).normalize() * 240.0;

    let mut peak: f32 = 0.0;
    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
        let position = race.sim.world.ships[0].physics.body.position;
        if let Some(distance) = race.spline().distance_to(position) {
            peak = peak.max(distance);
        }
    }
    let position = race.sim.world.ships[0].physics.body.position;
    let distance = race.spline().distance_to(position).expect("a sample");
    println!(
        "thrown from {from:?}: peak {peak:.1}, back within {distance:.1}, \
         respawns {}",
        race.respawns()
    );

    assert!(race.respawns() > 0, "nothing put the craft back");
    assert!(
        distance < 50.0,
        "the craft is {distance:.1} from the circuit ten seconds later"
    );
}

/// **Why a second trigger has to exist at all: some circuits author no `Reset`
/// geometry whatsoever.**
///
/// `docs/ghidra/functions/psp-pulse-usa/collision.md` records that touching the
/// `Reset` class respawns the craft, at confidence 86, and `oag_physics::reset`
/// implements it. What was never checked is whether every circuit *has* any -
/// and the circuit this rescue was reported on, Vertica reversed, has none. On
/// one of those, a craft that leaves has nothing to touch by construction, so no
/// amount of work on the reset path could recover it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn not_every_circuit_authors_reset_geometry() {
    let Some(image) = image() else {
        return;
    };
    let mut without = 0;
    let tracks = [
        "01_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "09_Track",
        "13_Track", "14_Track", "16_Track",
    ];
    for track in tracks {
        for variant in ["track.vex", "track_reversed.vex"] {
            let entry = format!("Data\\Environments\\{track}\\{variant}");
            let Ok(loaded) = race::load(&race::Options {
                source: image.display().to_string(),
                class: "VENOM".to_string(),
                mode: oag_race::Mode::SingleRace,
                track: Some(entry.clone()),
                ..race::Options::default()
            }) else {
                continue;
            };
            let count = loaded
                .setup
                .collision
                .colliders()
                .iter()
                .filter(|collider| collider.surface() == oag_physics::Surface::Reset)
                .count();
            println!("{track} {variant}: {count} Reset collider(s)");
            if count == 0 {
                without += 1;
            }
        }
    }
    assert!(
        without > 0,
        "every circuit authors Reset geometry, which is not what the reported \
         failure on 14_Track reversed measured - re-read that before trusting this"
    );
}

/// **The control: the rescue fires nowhere it is not needed.**
///
/// The margin [`oag_race::recovery::PLAYER_RESCUE_HALF_WIDTHS`] rests on is that no healthy
/// craft on the disc gets three quarters of one half-width from the sample table,
/// jumps included - `13_Track`, the circuit with the authored jump, peaks at 13.7
/// units against a threshold of 140. This is that margin as a gate: a rescue that
/// fired on a craft merely cornering wide would launder bad driving into completed
/// laps, and would teleport a player who was doing nothing wrong.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_healthy_player_is_never_recovered() {
    for track in ["16_Track", "13_Track", "06_Track", "07_Track"] {
        let entry = format!("Data\\Environments\\{track}\\track.vex");
        let Some(solo) = solo_player(oag_ai::Difficulty::Ace, &entry, 6_000) else {
            return;
        };
        let threshold = solo.half_width * oag_race::recovery::PLAYER_RESCUE_HALF_WIDTHS;
        println!(
            "ace {track}: peak {:.1} against a threshold of {threshold:.1}, laps {}, respawns {}",
            solo.peak, solo.laps, solo.respawns
        );
        assert_eq!(
            solo.respawns, 0,
            "{track} recovered a craft that did not need it"
        );
        assert!(
            solo.peak < threshold,
            "a healthy craft on {track} reached {:.1} units from the spline, \
             against a threshold of {threshold:.1} - the margin this constant rests on",
            solo.peak
        );
    }
}
