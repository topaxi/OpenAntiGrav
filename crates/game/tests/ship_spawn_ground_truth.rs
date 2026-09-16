//! A ship spawns on a real track and flies along it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all
//! ```
//!
//! Split out of `race_ground_truth.rs` on 2026-09-02, because that file sits at
//! its `BASELINE` ceiling in `scripts/check-file-size.py` and the countdown fix
//! below could not be written inside it. The ratchet only ever lowers, so the
//! move claws its row back rather than raising it. A move, with no behaviour
//! change beyond that fix. The helpers below are the same deliberate
//! near-duplicates `mine_ground_truth.rs` and `plasma_ground_truth.rs` carry.
//!
//! # What only real data can say here
//!
//! Every layer under a race has its own tests against synthetic data. What none
//! of them can check is whether the *composition* is right: whether the
//! collision geometry a ship hovers on is the same geometry the spline runs
//! along, whether the handling parameters arrive scaled exactly once, whether a
//! spawn pose built from a `.vex` frame puts a ship the right way up. Those only
//! fail on real data, so this is the only place they can be asserted.

use std::path::PathBuf;

use oag_game::race;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;

/// Ticks the per-tick suspension assertions cover: two seconds at 60 Hz, driven
/// *after* the start-line countdown rather than from tick zero.
///
/// Short on purpose, because this is the test that checks *every* tick against a
/// hover target and a probe count.
const TICKS: u32 = 120;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn load() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// The track's own scale for "still roughly on the track".
///
/// Two of the widest half-width the track has anywhere. Derived from the data
/// rather than picked, and generous on purpose: this catches a ship that has
/// left, it does not grade the driving.
fn envelope(loaded: &race::Loaded) -> f32 {
    loaded.setup.spline.max_half_width() * 2.0
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ship_spawns_on_the_track_and_flies_along_it() {
    let Some(loaded) = load() else { return };
    let bound = envelope(&loaded);
    let handling = loaded.setup.handling;
    let mut race = race::Race::start(loaded.setup);

    // The spawn itself: on the track's authored `Start Position`, the suspension's
    // resting height above the surface, the right way up, and with the mass the force
    // law will read also in the body the integrator divides by.
    //
    // The height assertion below is worth more than it looks now that the spawn takes
    // its height off the *collision mesh* under the slot while this reads it off the
    // *spline*: agreeing to a hundredth means those two surfaces agree there, which
    // nothing made them do.
    let start = race.telemetry();
    assert!(start.position.is_finite(), "{:?}", start.position);
    assert_eq!(race.ship().physics.body.mass, handling.physical.mass);
    assert!(
        (start.height_above_spline - race::spawn_height(&handling)).abs() < 0.01,
        "spawned at {} above the spline, wanted {}",
        start.height_above_spline,
        race::spawn_height(&handling)
    );
    // `up` from a `.vex` frame is the surface normal negated, and getting that
    // backwards buries the ship. On this track's first sample the surface is roughly
    // level, so the ship's own up must have a positive world y.
    assert!(
        race.ship().physics.body.up().y > 0.5,
        "the ship is not the right way up: {}",
        race.ship().physics.body.up()
    );

    // Out of the countdown first - it gates thrust, so the window below used to
    // sit wholly inside it and the ship travelled 0.03 units. The spawn
    // assertions above are before this on purpose: they are about tick zero.
    let mut held = race::HeldButtons::new(Button::Cross.bit());
    for _ in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::single(held.snapshot()));
    }
    let moving_from = race.telemetry().position;
    let mut grounded_ticks = 0u32;
    // Counted apart, because one probe in contact and two are different situations:
    // a single probe is a pitch torque applied every tick.
    let mut both_probes = 0u32;
    let mut worst_distance = 0.0f32;
    let mut peak_speed = 0.0f32;

    for tick in 0..TICKS {
        let snapshot = held.snapshot();
        race.tick(&PlayerInputs::single(snapshot));
        let telemetry = race.telemetry();

        assert!(
            telemetry.position.is_finite(),
            "tick {tick}: {}",
            race::describe(&telemetry)
        );
        assert!(
            telemetry.spline_distance < bound,
            "tick {tick}: {bound:.1} units off the spline is off the track: {}",
            race::describe(&telemetry)
        );

        if race.ship().physics.grounded > 0.0 {
            grounded_ticks += 1;
        }
        if race.ship().physics.grounded == 1.0 {
            both_probes += 1;
        }
        worst_distance = worst_distance.max(telemetry.spline_distance);
        peak_speed = peak_speed.max(telemetry.speed);
    }

    let end = race.telemetry();
    println!("{}", race::describe(&end));
    println!(
        "grounded on {grounded_ticks}/{TICKS} tick(s), both probes on {both_probes}, \
         worst spline distance {worst_distance:.2} of {bound:.1}, peak speed {peak_speed:.2}"
    );

    // It hovered: a probe reached the collision geometry on some tick. Without this
    // the ship could be in free fall and everything above would still hold.
    assert!(
        grounded_ticks > 0,
        "no hover probe ever reached the track's collision geometry"
    );
    // And it hovered on *both* probes, every tick. This is the pin on
    // `race::DEFAULT_TRACK`: against the track this scenario was previously flown on,
    // the same run held both probes on 6 of these 120 ticks and one on 83, which reads
    // as a suspension that cannot hold a ship and was actually a ship being flown over
    // the wrong track's collision geometry. Strict on purpose - a single dropped probe
    // here is a regression worth looking at, and this test never runs in CI.
    assert_eq!(
        both_probes, TICKS,
        "both hover probes were in contact on only {both_probes} of {TICKS} tick(s); \
         see race::DEFAULT_TRACK"
    );
    // And it went somewhere. Thrust held for two seconds against an engine that
    // reaches tens of units per second cannot leave it where it started.
    let travelled = (end.position - moving_from).length();
    assert!(
        travelled > 10.0,
        "the ship travelled {travelled:.2} units in {TICKS} ticks: {}",
        race::describe(&end)
    );
    // And it went *forwards*. Three separate conventions have to agree for this to
    // hold - the spline's tangent, the rotation `Pose::from_sample` builds from it,
    // and the body axis the engine pushes along - and any one of them inverted would
    // still satisfy every assertion above.
    let forward = race.ship().physics.body.forward();
    let along = race.ship().physics.body.linear_velocity.dot(forward);
    assert!(
        along > 0.0,
        "thrust drove the ship backwards along its own forward axis: {along:.2}"
    );
    // The countdown drive plus the measured window: a check that nothing above
    // ticked the world an extra time, not a claim about the window's length.
    assert_eq!(
        race.sim.world.tick,
        oag_race::COUNTDOWN_TICKS + u64::from(TICKS)
    );
}
