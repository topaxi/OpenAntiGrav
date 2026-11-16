//! Flies a ship on a real track, out of a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all
//! ```
//!
//! # What this is for
//!
//! Every layer under a race has its own tests against synthetic data. What none of
//! them can check is whether the *composition* is right: whether the collision
//! geometry a ship hovers on is the same geometry the spline runs along, whether the
//! handling parameters arrive scaled exactly once, whether a spawn pose built from a
//! `.vex` frame puts a ship the right way up. Those only fail on real data, so this
//! is the only place they can be asserted. The same goes for the two halves of the
//! binary: [`the_front_end_hands_off_into_a_driveable_race`] boots the front end off
//! the disc, picks a language and flies what `Launch Game` starts, which is the
//! whole of what a player does and needs no GPU to check.
//!
//! Everything asserted is structural: something is finite, something is bounded by a
//! number the *track itself* supplies, something changed. Nothing here asserts a
//! speed, a height or a settling point as though it were known - the force law is
//! transcribed static analysis that has never been run against the original, so a
//! test that pinned a speed would be pinning this project's own arithmetic and
//! calling it a measurement. See `crates/physics/tests/ship_dynamics.rs`, which says
//! the same thing at its own level.
//!
//! # The ship now stays on the track
//!
//! [`a_ship_stays_on_the_track_for_ten_seconds`] is the assertion, and it used to be
//! the opposite one: `the_ship_does_not_stay_on_the_track_yet` recorded the ship
//! leaving the envelope at about tick **257** and said to delete itself when that
//! stopped happening. It stopped. The property is now pinned the right way round, so
//! the same measurement catches a regression instead of an improvement.
//!
//! **Two wrong diagnoses on the way there, both recorded rather than forgotten.** The
//! first was a suspension that dropped a probe: on the *previous* default track the
//! ship was grounded on 83 of 120 ticks with both probes on only 6. The default track
//! was the bug - see [`oag_raceplay::DEFAULT_TRACK`] - and the right one is grounded
//! on 120 of 120 with both probes throughout. The second was a missing speed
//! equilibrium, inferred from captures that held 23.6-25.1 units/s; the standing-start
//! capture inverted it. The force law is right, those captures were in sustained wall
//! contact the whole way, and what was missing was the contact response that reads it.
//!
//! What is left is **not** on the track-holding axis: the speed is still far above any
//! capture's. Contact generation is no longer the gap it was - the ten box sample
//! points, the angular half of the response, per-triangle contacts and the original's
//! single-sided rejection are all in `oag_physics::wall` now, each measured against
//! the whole-lap scenario on its own commit. What the lap run shows instead is a ship
//! that gets **wedged**: it reaches about 40 units/s by tick 400, stops at one place
//! on the circuit for a thousand ticks and then reverses, and its angular velocity
//! averages 21 rad/s over the lap. See `docs/physics/force-balance-ground-truth.md`,
//! and nothing here is tuned to hide it.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_game::{boot, movie};
use oag_gameplay::{PlayerInputs, input::Button, input::Input};
use oag_physics::Raycaster;
use oag_raceplay::{self as race, catalogue};
use oag_ui::frontend::states;

pub(crate) fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

pub(crate) fn load() -> Option<race::Loaded> {
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

/// The grid measurement's own scenario: [`Options::opponents`] forced on.
///
/// `Mode::SingleRace` now fields a grid on its own, so the override is no longer
/// the only way to get eight craft - but it is still the right one *here*,
/// because it puts a full grid on the default mode and so keeps the grid
/// measurements independent of which mode is being raced. The tests that want
/// the AI driving use [`load_single_race`] instead. Every other test in this
/// file wants a solo ship, which is what [`load`] gives.
pub(crate) fn load_with_opponents() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// A single race: the one mode that fields a grid because the mode says so,
/// rather than because a test override forced one.
pub(crate) fn load_single_race() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
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
/// Two of the widest half-width the track has anywhere, which on the default track
/// is tens of units. Derived from the data rather than picked, so it means the same
/// thing on a wider track, and generous on purpose: this is meant to catch a ship
/// that has left, not to grade the driving.
pub(crate) fn envelope(loaded: &race::Loaded) -> f32 {
    loaded.setup.spline.max_half_width() * 2.0
}

/// Ticks the handoff test flies: the 272-tick countdown, where nothing is driven
/// (`grid-state.md`: a craft held on the grid does not move), then 60 more, half
/// the two seconds `ship_spawn_ground_truth.rs` drives. The question is whether
/// `Launch Game` produces a driveable ship, not how long one stays driveable.
const HANDOFF_TICKS: u32 = oag_race::state::COUNTDOWN_TICKS as u32 + 60;

/// `Launch Game` is what puts a player in a ship, so the two halves have to
/// compose: the boot sequence has to reach that state off a real disc, and what it
/// hands off to has to be a race that spawns, hovers and drives.
///
/// The window does exactly this, in this order, with a GPU in the middle. Nothing
/// here needs one, which is the point: if this passes and the window does not, the
/// fault is in the window.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_end_hands_off_into_a_driveable_race() {
    let Some(image) = image() else { return };

    // The same load the binary does, without the transcode: what is under test is
    // the sequencing and the handoff, not the picture.
    let boot = boot::load(&boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: Some(boot::DEFAULT_BOOT_MOVIE.to_string()),
        cache: std::env::temp_dir().join("oag-race-handoff"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    })
    .expect("loading the boot sequence");

    let mut frontend = boot.frontend;
    let mut input = Input::new();
    let dt = 1.0 / 60.0;

    // START skips the intro through `LogoFMVRedirectScreen`, cross picks a
    // language, and START again presses through `Show Logo`.
    input.begin_frame(Button::Start.bit());
    frontend.update(dt, &mut input, None);
    input.begin_frame(0);
    frontend.update(dt, &mut input, None);
    assert!(
        frontend.machine().is(states::LANGUAGE_SELECTION),
        "got as far as {:?}",
        frontend.machine().current()
    );

    input.begin_frame(Button::Cross.bit());
    frontend.update(dt, &mut input, None);
    assert!(frontend.machine().is(states::SHOW_LOGO));

    // `Show Logo` is the disc's PRESS START screen and it sits between the
    // picker and everything after it, so the front end is not finished until
    // START goes through it.
    input.begin_frame(Button::Start.bit());
    frontend.update(dt, &mut input, None);
    assert!(frontend.machine().is(states::LAUNCH_GAME));
    assert!(
        frontend.is_finished(),
        "reaching Launch Game is what starts a race"
    );

    // And this is what the window does next.
    let Some(loaded) = load() else { return };
    let bound = envelope(&loaded);
    let mut race = race::Race::start(loaded.setup);
    let mut held = race::HeldButtons::new(Button::Cross.bit());
    let mut grounded_ticks = 0u32;

    for tick in 0..HANDOFF_TICKS {
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
    }

    println!("{}", race::describe(&race.telemetry()));
    assert!(
        grounded_ticks > 0,
        "no hover probe reached the track in {HANDOFF_TICKS} ticks"
    );
    let forward = race.ship().physics.body.forward();
    let along = race.ship().physics.body.linear_velocity.dot(forward);
    assert!(
        along > 0.0,
        "the ship the front end handed off to does not drive forwards: {along:.2}"
    );
}

/// Ten seconds at full throttle, on the track and finite the whole way.
///
/// **This replaces a recorded negative result.** `the_ship_does_not_stay_on_the_track_yet`
/// asserted the opposite - that the ship *left* the envelope inside 600 ticks - and
/// carried an instruction to delete it when it started failing. It did, so the
/// property is pinned the right way round here rather than dropped: a regression that
/// threw the ship off the track again would otherwise go unnoticed.
///
/// What changed is the wall contact response in `oag_physics::wall`, matched to the
/// `3.67 %`-per-frame speed loss measured off a real capture. Before it, the ship left
/// the envelope at about tick **257**, not by falling off the surface but by carrying
/// far more speed into a corner than the corner would take.
///
/// Measured on the reference scenario - Assegai, Venom, [`race::DEFAULT_TRACK`], thrust
/// held from the first tick - over the full 600:
///
/// | | |
/// | --- | --- |
/// | Worst distance from the spline | **27.2** of a 114.0 envelope, at tick 520 |
/// | Grounded | **600 of 600** ticks |
/// | Respawns | **0** - it stays on by driving, not by being put back |
/// | Non-finite | never |
///
/// The assertion is the envelope and finiteness only. **The speed is still not the
/// original's** - this run peaks at 122 against a capture's 23.6-25.1 - and nothing
/// here asserts it, because a clean straight has no equilibrium to assert and the
/// captures that looked like one were in sustained wall contact. See
/// `docs/physics/force-balance-ground-truth.md`.
///
/// **The "contact generation and the missing `cross(r, impulse)` angular response"
/// this used to name as the remaining gaps are both closed**, and were already
/// closed when that sentence was written - `oag_physics::wall` has the ten box
/// sample points, the angular share of the denominator, the `0.1`-scaled angular
/// application, per-triangle contacts and the original's single-sided rejection.
/// What a whole-lap comparison diverges on now is an open question rather than a
/// known omission.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ship_stays_on_the_track_for_ten_seconds() {
    let Some(loaded) = load() else { return };
    let bound = envelope(&loaded);
    let mut race = race::Race::start(loaded.setup);
    let mut held = race::HeldButtons::new(Button::Cross.bit());

    let mut worst_distance: f32 = 0.0;
    let mut worst_at = 0u32;
    let mut top_speed: f32 = 0.0;
    let mut grounded_ticks = 0u32;
    for tick in 0..600u32 {
        let snapshot = held.snapshot();
        race.tick(&PlayerInputs::single(snapshot));
        let telemetry = race.telemetry();
        assert!(
            telemetry.position.is_finite(),
            "the ship went non-finite at tick {tick}"
        );
        if telemetry.spline_distance > worst_distance {
            worst_distance = telemetry.spline_distance;
            worst_at = tick;
        }
        top_speed = top_speed.max(telemetry.speed);
        if telemetry.grounded > 0.0 {
            grounded_ticks += 1;
        }
    }

    println!(
        "worst spline distance {worst_distance:.1} of {bound:.1} at tick {worst_at}, \
         top speed {top_speed:.1}, grounded on {grounded_ticks} of 600, \
         {} respawn(s)",
        race.respawns()
    );
    assert!(
        worst_distance < bound,
        "the ship left the {bound:.1}-unit envelope at tick {worst_at}"
    );
}

/// The spline and the collision geometry have to be the same track.
///
/// Nothing under this test can catch a mismatch: `oag-formats` decodes each of them
/// correctly in isolation, and a ship hovering over geometry a hundred units from
/// the spline it spawned on would look like a physics bug rather than a loading one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_collision_geometry_is_under_the_spline() {
    let Some(loaded) = load() else { return };
    let handling = loaded.setup.handling;
    let spline = &loaded.setup.spline;
    let collision = &loaded.setup.collision;

    assert!(!spline.is_empty(), "the track produced no spline samples");
    assert!(
        !collision.colliders().is_empty(),
        "the track produced no colliders"
    );

    // Cast down the surface normal at a spread of samples along the track, from one
    // probe reach above the surface line to one below it. `ride_height` is the length
    // the ship's own probes use, so a miss here is a place a ship on the spline would
    // find nothing to hover on.
    let reach = handling.antigrav.ride_height;
    let mut hits = 0;
    let mut probes = 0;
    let step = (spline.len() / 200).max(1);
    for index in (0..spline.len()).step_by(step) {
        let Some(sample) = spline.sample(index) else {
            continue;
        };
        let up = (-Vec3::from_array(sample.down)).normalize_or_zero();
        let from = Vec3::from_array(sample.pos) + up * reach;
        probes += 1;
        if collision
            .raycast(oag_physics::Ray::new(from, -up, reach * 2.0), None, false)
            .is_some()
        {
            hits += 1;
        }
    }

    println!("{hits}/{probes} downward probes found collision geometry within {reach} units");
    // Not all of them: a junction's samples overlap, the spline runs through
    // scenery-only stretches, and the two decoders were never promised to agree
    // everywhere. Most of them is the claim.
    assert!(
        hits * 2 > probes,
        "only {hits} of {probes} samples have anything under them within \
         {reach} units; the spline and the collision geometry are not the same track"
    );
}

/// Which way the shipped `Wall` triangles are wound, measured rather than assumed.
///
/// **This is the measurement that gates single-sided wall contacts.**
/// `Collision_BoxAgainstMesh` (`0x08815cd4`) does not flip a triangle normal; it
/// *rejects* the sample point when `dot(boxCentre - sample, n) <= 0`, so the raw
/// winding normal is what reaches the contact - see
/// `docs/ghidra/functions/psp-pulse-usa/collision.md`. `oag_physics::wall` flips
/// instead, which is safe under either winding and is why nobody has had to know
/// this. Reproducing the original's rejection is only correct if the shipped
/// walls actually face the circuit, and nothing in `docs/` says they do.
///
/// So: for every `Wall` triangle on the track, take its raw winding normal
/// `(b - a) x (c - a)` and ask whether it points toward the nearest point of the
/// track's own spline. A wall that faces the circuit answers yes.
///
/// The assertion is deliberately loose. Some walls genuinely face away - a
/// barrier with a driveable side and a scenery side, a tunnel mouth, geometry
/// past a junction where "nearest spline point" is the wrong reference - and a
/// track is not obliged to be tidy. What would sink single-siding is a *mixed*
/// result, so the claim is that one answer dominates, and the printed fraction
/// is the number a reader should take away rather than the pass.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_track_s_walls_are_wound_toward_the_circuit() {
    let Some(loaded) = load() else { return };
    let spline = &loaded.setup.spline;
    assert!(!spline.is_empty(), "the track produced no spline samples");

    let centres: Vec<Vec3> = (0..spline.len())
        .filter_map(|index| spline.sample(index))
        .map(|sample| Vec3::from_array(sample.pos))
        .collect();

    let mut inward = 0usize;
    let mut outward = 0usize;
    for collider in loaded.setup.collision.colliders() {
        if collider.surface() != oag_physics::Surface::Wall {
            continue;
        }
        for index in 0..collider.triangle_count() {
            let Some([a, b, c]) = collider.triangle(index) else {
                continue;
            };
            let normal = (b - a).cross(c - a);
            if normal.length_squared() <= 0.0 {
                continue;
            }
            let centroid = (a + b + c) / 3.0;
            let Some(nearest) = centres.iter().copied().min_by(|p, q| {
                (*p - centroid)
                    .length_squared()
                    .total_cmp(&(*q - centroid).length_squared())
            }) else {
                continue;
            };
            if normal.dot(nearest - centroid) > 0.0 {
                inward += 1;
            } else {
                outward += 1;
            }
        }
    }

    let total = inward + outward;
    assert!(total > 0, "the track has no wall triangles to measure");
    let fraction = inward as f32 / total as f32;
    println!(
        "{inward}/{total} wall triangles ({:.1}%) are wound toward the nearest spline point",
        fraction * 100.0
    );
    assert!(
        !(0.3..=0.7).contains(&fraction),
        "the shipped wall winding is mixed ({:.1}% inward), so a single-sided \
         contact gate cannot be implemented from it without more evidence",
        fraction * 100.0
    );
}

/// One craft, alone, lapping.
///
/// **The benchmark the pace tuning should be judged against**, and the reason
/// it exists apart from the field tests: with seven craft on the circuit every
/// number is a mixture of how well a craft drives and how much the traffic cost
/// it, and the two move in opposite directions when aggression changes. A solo
/// lap is the driving alone. Tune here first, then check the field.
///
/// Returns the best lap the craft managed, in ticks.
fn solo_lap_ticks(level: oag_ai::Difficulty) -> Option<u64> {
    solo_lap_on(level, oag_pulse::race::DEFAULT_TRACK).best
}

/// What a lone craft managed on one circuit.
#[derive(Debug, Default, Clone)]
struct Solo {
    /// The quickest **clean** lap, in ticks.
    ///
    /// A lap the craft had to be recovered during is not counted, and that is
    /// the point of carrying [`Self::respawns`] alongside it. Respawning is a
    /// safety net for a race with seven other craft shoving; on an empty
    /// circuit a competent driver should never need it, so a benchmark that let
    /// recovery paper over a craft flying into the scenery would report the
    /// driving as fixed when only the symptom was.
    best: Option<u64>,
    /// How many times the craft had to be put back on the track.
    respawns: u32,
    /// How far round it got, in laps.
    laps: u32,
    /// The driver index the craft was on when each rescue fired, and the line's
    /// length, so a reader can tell whether the losses cluster anywhere.
    lost_at: Vec<u32>,
    line_len: u32,
}

/// The same, on a named circuit.
fn solo_lap_on(level: oag_ai::Difficulty, track: &str) -> Solo {
    solo_lap_tuned(level, track, None)
}

/// The same, with the drivers' tuning overridden - what a sweep calls.
fn solo_lap_tuned(level: oag_ai::Difficulty, track: &str, tuning: Option<oag_ai::Tuning>) -> Solo {
    let Some(image) = image() else {
        return Solo::default();
    };
    let Ok(loaded) = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(track.to_string()),
        ..race::Options::default()
    }) else {
        return Solo::default();
    };
    let mut race = race::Race::start(loaded.setup);
    if let Some(tuning) = tuning {
        race.set_ai_tuning(tuning);
    }
    // Everyone but one opponent off the track, so nothing it does is about
    // anybody else.
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;

    let mut best: Option<u64> = None;
    let mut lap = race.sim.world.ships[1].standing.lap;
    let mut started = 0u64;
    let mut recovered_this_lap = false;
    let mut lost_at = Vec::new();
    for tick in 0..18_000u64 {
        let before = race.respawns_of(1);
        let was_at = race.sim.world.ships[1].driver.index;
        race.tick(&PlayerInputs::none());
        if race.respawns_of(1) != before {
            recovered_this_lap = true;
            lost_at.push(was_at);
        }
        let now = race.sim.world.ships[1].standing.lap;
        if now != lap {
            // The first lap is the standing start, and a lap the craft had to
            // be recovered during is not a lap it drove.
            if lap > 1 && !recovered_this_lap {
                let taken = tick - started;
                best = Some(best.map_or(taken, |held: u64| held.min(taken)));
            }
            started = tick;
            lap = now;
            recovered_this_lap = false;
        }
    }
    Solo {
        best,
        respawns: race.respawns_of(1),
        laps: lap,
        lost_at,
        line_len: race.racing_line().len() as u32,
    }
}

/// A solo craft laps, and laps faster at a harder setting.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_solo_craft_laps_faster_at_a_harder_setting() {
    let Some(novice) = solo_lap_ticks(oag_ai::Difficulty::Novice) else {
        return;
    };
    let ace = solo_lap_ticks(oag_ai::Difficulty::Ace).expect("the image is there");
    println!(
        "solo best lap: novice {novice} ticks ({:.1}s), ace {ace} ticks ({:.1}s)",
        novice as f32 / 60.0,
        ace as f32 / 60.0
    );
    assert!(
        ace < novice,
        "an ace should lap quicker alone: {ace} against {novice}"
    );
}

/// Every circuit on the disc, one craft, best lap.
///
/// **The benchmark that stops a tuning being fitted to one track.** Talon's
/// Junction is the default and therefore the one every other measurement here
/// happens to use; a knob that helps there and hurts everywhere else would look
/// like an improvement right up until someone played a different circuit.
fn solo_laps_everywhere(level: oag_ai::Difficulty) -> Vec<(String, Solo)> {
    solo_laps_tuned(level, None)
}

/// The same, with the drivers' tuning overridden.
fn solo_laps_tuned(
    level: oag_ai::Difficulty,
    tuning: Option<oag_ai::Tuning>,
) -> Vec<(String, Solo)> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
        .map(|track| {
            let entry = track.entry_name();
            (track.id.clone(), solo_lap_tuned(level, &entry, tuning))
        })
        .collect()
}

/// What a grip figure is worth across the whole disc rather than on one
/// circuit.
///
/// A scratch sweep, kept `#[ignore]`d and printing rather than asserting: the
/// number it produces goes into `Tuning::lateral_accel` by hand, with the table
/// written into `docs/gameplay/ai.md` so the choice has its measurement next to
/// it. It exists because a solo lap on Talon's Junction said 260 was quicker
/// than 180, and Talon's Junction is one of the five circuits a craft gets
/// round cleanly - exactly the subset that cannot see the failure.
/// Twelve circuits times five grip figures times five minutes of simulation is
/// minutes of wall clock in a debug build, so it is off unless asked for:
///
/// ```sh
/// OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 \
///   cargo nextest run --release -p oag-game --run-ignored all sweep_grip --no-capture
/// ```
///
/// `#[ignore]` alone would not do it - `just test-data` runs ignored tests, and
/// a five-minute entry in that suite is how a suite stops being run.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table, choose"]
fn sweep_grip() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let mut report = String::from("\ngrip   clean  round  respawns  mean clean lap\n");
    for grip in [120.0f32, 180.0, 260.0, 340.0, 440.0, 560.0, 700.0] {
        let tuning = oag_ai::Tuning {
            lateral_accel: grip,
            ..oag_ai::Tuning::default()
        };
        let laps = solo_laps_tuned(oag_ai::Difficulty::Ace, Some(tuning));
        if laps.is_empty() {
            return;
        }
        let clean: Vec<u64> = laps.iter().filter_map(|(_, solo)| solo.best).collect();
        let mean = if clean.is_empty() {
            f32::NAN
        } else {
            clean.iter().sum::<u64>() as f32 / clean.len() as f32 / 60.0
        };
        report.push_str(&format!(
            "{grip:<6} {:<6} {:<6} {:<9} {mean:.1}s\n",
            clean.len(),
            laps.iter().filter(|(_, solo)| solo.laps >= 2).count(),
            laps.iter().map(|(_, solo)| solo.respawns).sum::<u32>(),
        ));
    }
    println!("{report}");
}

/// Every craft starts on its own racing line, on **every** circuit.
///
/// **The test that was missing, and the bug it would have caught was severe.**
/// `Driver::drive` locates a craft with a 48-sample *window* around its last
/// index - deliberately, so a circuit that passes over itself cannot make a
/// craft latch onto a stacked section. `Driver::index` therefore has to start
/// somewhere near the truth, and it did not: it started at zero, so a grid
/// sitting at sample 2,500 never found itself and steered at whatever piece of
/// track it believed it was on.
///
/// It went unnoticed because every other test here runs the default circuit,
/// and the default circuit's start line happens to sit near sample zero. On the
/// disc's twelve circuits the field began between 280 and 10,862 units from its
/// own racing line on eight of them, and on six it never got going at all -
/// 300 units of travel in a minute, and on one the craft ground itself to
/// destruction.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_craft_starts_on_its_line_on_every_circuit() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    let mut checked = 0;
    for track in catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
    {
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
        checked += 1;

        for slot in 1..race.ship_count() as usize {
            let ship = &race.sim.world.ships[slot];
            let sample = race
                .ai_sample_for(slot, ship.driver.index as usize)
                .expect("the driver stands on a sample");
            let lateral = Vec3::from_array(sample.lateral).normalize_or_zero();
            let line = Vec3::from_array(sample.pos)
                - oag_vex::track::HOVER_LIFT * Vec3::from_array(sample.down)
                + sample.racing_line * lateral;
            let off = (ship.physics.body.position - line).length();
            // A grid is staggered across and along the track, so this is
            // "somewhere on the start line" rather than "on the racing line".
            assert!(
                off < 120.0,
                "{}: slot {slot} starts {off:.0} units from the sample its \
                 driver thinks it is on",
                track.id
            );
        }
    }
    assert!(checked >= 10, "only {checked} circuits were loadable");
}

/// How many of the disc's circuits a lone craft can actually get round.
///
/// **A record of a known-incomplete state, pinned so it cannot quietly get
/// worse**, and it holds two separate ratchets because there are two separate
/// failures behind the one symptom.
///
/// # What the numbers were, and what moved them
///
/// Two of twelve until `Driver::index` was seeded from the spawn (see
/// [`every_craft_starts_on_its_line_on_every_circuit`]), then five. Adding a
/// rescue for a craft that leaves the circuit ([`oag_race::recovery::RESCUE_HALF_WIDTHS`])
/// took *laps completed* to nine of twelve without moving *clean* laps at all,
/// and that gap is the finding: the craft were not failing to drive round, they
/// were driving off and never coming back, because a `Reset` volume cannot
/// catch something receding into open space.
///
/// # The two ratchets, and why a clean lap is the one that matters
///
/// A lap the craft had to be recovered during is not a lap it drove. On an
/// empty circuit at the top difficulty a competent driver should never need
/// recovering, so the respawn count *is* the driving-quality metric here and
/// the rescue must not be allowed to launder a craft that flies into the
/// scenery. Four circuits complete every lap and never manage a clean one.
///
/// and that gap is the finding. Dropping the paths a lap never drives out of
/// the AI line ([`race::Race::ai_sample`]) then took *round* to eleven and
/// *clean* to six, and the nine circuits it did not touch reproduced their lap
/// times, respawn counts and loss indices exactly - which is what said the
/// mapping was right.
///
/// # What is left
///
/// **Nothing, as of 2026-08-12**: all twelve circuits complete every lap and
/// all twelve manage a clean one, eleven of them without being recovered once.
/// 01 keeps two recoveries, one of them on the standing-start lap.
///
/// It took four fixes and none of them were the AI. Dropping the paths a lap
/// never drives out of the racing line ([`race::Race::ai_sample`]) took *round*
/// from nine to eleven. Gating the landing response on `rebound_jump_time` took
/// 01 from eight recoveries to three. Deriving the rear hover probe above
/// `oag_physics::hover::FAST_PROBE_SPEED` instead of casting it took clean laps
/// from seven to nine and 05 from never lapping to lapping without a scratch.
/// And a swept contact test (`oag_physics::hover::sweep`) - the one mechanism in
/// that crate that is ours rather than recovered - took the last three, because
/// a downward-only probe cannot find a surface the hull has already crossed.
///
/// Both bounds are "no worse than today" rather than targets: a number that
/// asserted twelve would be a test failing for a reason already written down,
/// which is noise rather than signal.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round() {
    let laps = solo_laps_everywhere(oag_ai::Difficulty::Ace);
    if laps.is_empty() {
        return;
    }
    let lapped: Vec<&String> = laps
        .iter()
        .filter(|(_, solo)| solo.best.is_some())
        .map(|(id, _)| id)
        .collect();
    let missed: Vec<&String> = laps
        .iter()
        .filter(|(_, solo)| solo.best.is_none())
        .map(|(id, _)| id)
        .collect();
    for (id, solo) in &laps {
        println!(
            "{id:<12} clean lap {:>6}  laps {:<3} respawns {:<3} of {} lost at {:?}",
            solo.best.map_or("none".to_string(), |ticks| format!(
                "{:.1}s",
                ticks as f32 / 60.0
            )),
            solo.laps,
            solo.respawns,
            solo.line_len,
            solo.lost_at
        );
    }
    println!("clean laps: {lapped:?}\nno clean lap: {missed:?}");
    // The full set, so this is now a plain regression bound rather than a
    // record of an incomplete state: any circuit that stops managing a clean
    // lap is a regression, and the message names it.
    assert_eq!(
        lapped.len(),
        laps.len(),
        "only {} of {} circuits saw a clean lap, and all of them used to: \
         {missed:?}",
        lapped.len(),
        laps.len()
    );

    // The second ratchet: got round at all, recovered or not. It is the one the
    // rescue moved, and keeping it separate is what stops a driving regression
    // hiding behind a recovery that still gets the craft home.
    let round: Vec<&String> = laps
        .iter()
        .filter(|(_, solo)| solo.laps >= 2)
        .map(|(id, _)| id)
        .collect();
    assert!(
        round.len() >= 12,
        "only {} of {} circuits were completed at all, and all twelve have been",
        round.len(),
        laps.len()
    );
}

/// Whether there is anything under the line an opponent is told to drive.
///
/// **The measurement that turned "the AI flies off at one corner" into
/// something that is not an AI question at all.** Craft were leaving six
/// circuits at a repeatable place every lap, falling five to eight hundred
/// units while only a few units laterally off their own line - straight down,
/// not wide. So the question stopped being how the driver steers and became
/// whether the surface it is steering along is there.
///
/// Casting down the surface normal at **every** sample of the racing line, from
/// one probe reach above to one below, which is exactly what a craft's own
/// antigravity probes reach:
///
/// | circuit | line samples with nothing under them | where |
/// | --- | --- | --- |
/// | 16, 03, 04 | **0** | - |
/// | 06 | 5 | 1196-1200 |
/// | 01 | 12 | 31-42 |
/// | 07 | 13 | 2364-2376 |
/// | 13 | 31 | 19-49 |
/// | 09 | 55 | 967-1021 |
/// | 02 | 70 | 1167-1236 |
/// | 10 | 72 | 135-206 |
/// | 14 | 82 | 684-765 |
/// | 05 | 134 | 161-211, 811-893 |
///
/// Each is a single contiguous run, and **the three circuits with none are
/// three of the circuits that never lose a craft**. A second, deeper cast
/// separates two different things wearing the same result: on 13 and most of 10
/// there is nothing within sixty reaches, which is an authored *jump* - 13 flies
/// it, lands, and laps cleanly. On 09, 02, 05, 14, 01 and 07 there is a surface,
/// one to sixty ride-heights **below** the line. That is not a gap in the track;
/// it is a racing line running above the track.
///
/// Which of those two this is per circuit is not settled, and neither is whether
/// the deeper cases are the disc's data or this repository's collision loading.
/// See `docs/gameplay/ai.md`.
///
/// The assertion is only that the three clean circuits stay clean, because that
/// is the part that is understood. The table prints so a reader gets the whole
/// picture rather than the one bound.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_racing_line_has_track_under_it_where_it_is_known_to() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    let mut measured: Vec<(String, usize)> = Vec::new();
    for track in catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
    {
        let Ok(loaded) = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            track: Some(track.entry_name()),
            ..race::Options::default()
        }) else {
            continue;
        };
        let reach = loaded.setup.handling.antigrav.ride_height;
        let collision = loaded.setup.collision.clone();
        let race = race::Race::start(loaded.setup);
        let line = race.racing_line();

        // Two casts per unsupported sample: one probe reach, which is what a
        // craft holds onto, then sixty, which says whether there is a surface
        // down there at all.
        let (mut gaps, mut below) = (Vec::new(), 0u32);
        for index in 0..line.len() {
            let Some(sample) = race.ai_sample(index) else {
                continue;
            };
            let up = (-Vec3::from_array(sample.down)).normalize_or_zero();
            let from = line.point(index) + up * reach;
            let cast = |length: f32| {
                collision
                    .raycast(oag_physics::Ray::new(from, -up, length), None, false)
                    .is_some()
            };
            if cast(reach * 2.0) {
                continue;
            }
            gaps.push(index);
            if cast(reach * 60.0) {
                below += 1;
            }
        }
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for index in &gaps {
            match runs.last_mut() {
                Some(last) if *index == last.1 + 1 => last.1 = *index,
                _ => runs.push((*index, *index)),
            }
        }
        println!(
            "{:<12} {:>4} of {} line samples unsupported ({below} have a surface further \
             below) in runs {:?}",
            track.id,
            gaps.len(),
            line.len(),
            runs.iter()
                .filter(|(from, to)| to - from >= 4)
                .map(|(from, to)| format!("{from}..{to}"))
                .collect::<Vec<_>>(),
        );
        measured.push((track.id.clone(), gaps.len()));
    }
    if measured.is_empty() {
        return;
    }

    for id in ["16_Track", "03_Track", "04_Track"] {
        let Some((_, gaps)) = measured.iter().find(|(name, _)| name == id) else {
            continue;
        };
        assert_eq!(
            *gaps, 0,
            "{id} used to have track under every sample of its racing line and now has \
             {gaps} without"
        );
    }
}

/// The Rocket's model is longest along the axis `Race::rocket_model_matrices`
/// aims at its velocity.
///
/// **This is the assertion that catches a rocket drawn flying sideways.**
/// `rocket_model_matrices` builds `Mat4::from_cols(side, up, forward, position)`,
/// which maps the model's own **+Z** onto the direction of travel. Nothing in the
/// unit tests can check that +Z is where the dart's nose actually points: they
/// assert the matrix contains what was written into it, which is true whatever
/// the mesh looks like. Only the real file can settle it.
///
/// It also bears on something left unresolved in
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`. `Rocket_Update`
/// builds the same forward/normal/cross basis and then rotates it by a further
/// `-pi/2` about an axis this project has not identified - and a quarter turn is
/// exactly the shape of a model-space axis convention. If this test ever fails
/// because the long axis is X, that deferred rotation is the reason, and the fix
/// is a fixed pre-rotation here rather than a new reading of the original.
#[test]
#[ignore = "needs a real disc image under data/images/"]
fn the_rocket_model_is_longest_along_the_axis_it_is_flown_down() {
    let Some(loaded) = load() else { return };
    let model = loaded
        .rocket_model
        .expect("the disc carries Data\\Weapons\\Rocket.vex");

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for vertex in &model.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex.position[axis]);
            max[axis] = max[axis].max(vertex.position[axis]);
        }
    }
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    println!(
        "Rocket.vex spans x {:.3}, y {:.3}, z {:.3}",
        span[0], span[1], span[2]
    );

    let longest = (0..3).max_by(|a, b| span[*a].total_cmp(&span[*b])).unwrap();
    assert_eq!(
        longest,
        2,
        "the dart is longest along {} but `rocket_model_matrices` aims +Z down the \
         velocity, so the model would be drawn broadside; spans are {span:?}",
        ["x", "y", "z"][longest]
    );
    // A dart, not a disc: the long axis should dominate, or "longest" is noise.
    let second = (0..3)
        .filter(|axis| *axis != longest)
        .map(|axis| span[axis])
        .fold(0.0f32, f32::max);
    assert!(
        span[longest] > second * 1.5,
        "expected one clearly dominant axis, got {span:?}"
    );
}

#[test]
#[ignore = "needs a real disc image under data/images/"]
fn pulses_default_track_reports_its_own_trackstartup_xml() {
    let Some(loaded) = load() else { return };
    let report = loaded.report.join("\n");
    let want = report.contains("8 billboard slot(s), 7 naming a model and 1 a colour")
        && report.contains("sound bank TALONS_JUNCTION_ENV.bnk");
    assert!(want, "no matching trackstartup.xml line: {report}");
}
