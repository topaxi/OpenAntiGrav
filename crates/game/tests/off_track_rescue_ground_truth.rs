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
//! because out there is nothing of any class to touch - `race::RESCUE_HALF_WIDTHS`
//! records the measurement on the opponents' side, and `docs/gameplay/ai.md` has
//! the same drop taken by a human at the controls with no AI involved.
//!
//! The failure is not reachable synthetically: it is a property of shipped track
//! geometry that a step down bigger than the hover probes' reach takes the surface
//! out of view, after which the craft travels forward faster than gravity brings
//! it back. Measured here before the rescue existed, one autopiloted craft alone
//! at ace: on `05_Track` it passed 20 units from the sample table at tick 591,
//! never came back, and was **7,983 units** away by tick 6,000.
//!
//! So the four tests are: the bound on the circuit that produces the failure
//! unprompted, the reported failure reproduced on the circuit it was reported on,
//! the count of `Reset` colliders that says why those volumes cannot be the
//! answer, and a control on the circuits that never leave at all.

use std::path::{Path, PathBuf};

use oag_game::race;

/// The disc, or `None` on a checkout without one.
fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    None
}

/// What one lone craft did over a run.
#[derive(Debug, Default)]
struct Solo {
    /// The furthest it ever got from the nearest spline sample.
    peak: f32,
    /// The circuit's widest half-width, which is the scale
    /// [`race::PLAYER_RESCUE_HALF_WIDTHS`] is a multiple of.
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
        race.world.ships[slot].active = false;
    }
    race.set_autopilot(true);

    let mut solo = Solo {
        half_width: race.spline().max_half_width(),
        ..Solo::default()
    };
    let mut stall = 0u32;
    for _ in 0..ticks {
        race.tick(&oag_gameplay::InputSnapshot::default());
        let ship = &race.world.ships[0];
        let position = ship.physics.body.position;
        if let Some(distance) = race.spline().distance_to(position) {
            solo.peak = solo.peak.max(distance);
        }
        let stopped = ship.physics.thrust > 0.0
            && !ship.standing.finished()
            && ship.physics.body.linear_velocity.length() < race::STALL_SPEED;
        stall = if stopped { stall + 1 } else { 0 };
        solo.longest_stall = solo.longest_stall.max(stall);
    }
    solo.laps = race.world.ships[0].standing.lap;
    solo.respawns = race.respawns();
    Some(solo)
}

/// **A player who comes off a real circuit stops receding.**
///
/// `05_Track` is the circuit that produces the failure with no input from a
/// driver: the craft leaves early in the first lap and, before this, never came
/// back. The bound asserted is the one the mechanism guarantees rather than a
/// fitted number - a craft cannot recede past the threshold for longer than
/// [`race::PLAYER_RESCUE_TICKS`], so its distance is bounded by how far a falling
/// craft travels in three quarters of a second.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_player_that_leaves_a_real_circuit_stops_receding() {
    let Some(solo) = solo_player(
        oag_ai::Difficulty::Ace,
        "Data\\Environments\\05_Track\\track.vex",
        6_000,
    ) else {
        return;
    };
    println!(
        "ace 05_Track: peak {:.1} ({:.1} half-widths), laps {}, respawns {}, longest stall {}",
        solo.peak,
        solo.peak / solo.half_width,
        solo.laps,
        solo.respawns,
        solo.longest_stall
    );

    assert!(
        solo.respawns > 0,
        "nothing recovered the player, so this circuit no longer measures anything"
    );
    // Before the rescue: 7,983 units, and receding for the rest of the race.
    assert!(
        solo.peak < 1_000.0,
        "the player got {:.1} units from the circuit, so the rescue is not bounding it",
        solo.peak
    );
    // **Not a lap assertion, and that is a finding rather than a gap.** This
    // circuit's authored racing line runs above its own collision surface for 134
    // samples (`docs/gameplay/ai.md`), so a craft put back there comes off again
    // and eventually wedges. An opponent is picked up by the stall rescue;
    // the player is not, deliberately - see `Race::lost_off_the_track`. What this
    // test owns is that the craft stops *receding*, which is what was broken.
    println!(
        "ace 05_Track: the craft is bounded but still not lapping - the stall is \
         the separate open thread on this circuit"
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
        race.world.ships[slot].active = false;
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
        race.tick(&oag_gameplay::InputSnapshot::default());
    }
    let from = race.world.ships[0].physics.body.position;
    // Off the side and up, at the speed the log recorded. Sideways rather than
    // down, deliberately: straight down is the case the authored `Reset` volumes
    // already answer, and this test is about the one they do not.
    let velocity = &mut race.world.ships[0].physics.body.linear_velocity;
    *velocity = oag_core::math::Vec3::new(velocity.x, 60.0, 0.0).normalize() * 240.0;

    let mut peak: f32 = 0.0;
    for _ in 0..600 {
        race.tick(&oag_gameplay::InputSnapshot::default());
        let position = race.world.ships[0].physics.body.position;
        if let Some(distance) = race.spline().distance_to(position) {
            peak = peak.max(distance);
        }
    }
    let position = race.world.ships[0].physics.body.position;
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
/// The margin [`race::PLAYER_RESCUE_HALF_WIDTHS`] rests on is that no healthy
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
        let threshold = solo.half_width * race::PLAYER_RESCUE_HALF_WIDTHS;
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
