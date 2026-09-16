//! Measures how far an opponent strays from the line after firing a Turbo, on
//! top of the `allows_speed` gate that already exists, across a range of
//! `Tuning::look_max` settings and, separately, across circuits.
//!
//! Written while chasing why the field drove in single file - see
//! `docs/gameplay/ai.md`'s Turbo section for the measurement
//! this produced and the conclusion drawn from it. Kept in the tree, unlike
//! the sweeps `driver/tuning.rs` cites, so the next person who wants to
//! re-check a driver constant against a real circuit has a harness rather
//! than a number with no way back to it. Needs a disc image:
//!
//! ```sh
//! cargo run --release -p oag-game --example turbo_lookahead_sweep
//! ```
//!
//! # Why more than one circuit
//!
//! The first `look_max` sweep ran on `16_Track` alone and came back flat -
//! that rules out `look_max` there, but `16_Track` is one geometry among
//! twelve, and a negative result on one circuit is evidence the escape is
//! rare, not proof it is gone. [`TRACKS`] widens the same excursion
//! measurement to two more circuits, at the shipped `look_max` (90) only -
//! the first sweep already answered whether `look_max` itself is the lever,
//! so this does not re-sweep it. See that constant for which circuits and
//! why.
//!
//! # Why this measures excursion, not respawn count
//!
//! A first pass counted `Race::respawns_of` inside a window after a Turbo.
//! Two things made that number worthless: `respawns_of` fires on the stall
//! rescue too, which has nothing to do with a Turbo, and a wide window (960
//! ticks) meant "inside a window" was true almost always with seven craft
//! each firing every few seconds - the count degenerated to "any respawn at
//! all". This instead tracks, for every Turbo a craft fires, how far it gets
//! from the spline in the ticks the boost plus a short coast covers - the
//! same "worst excursion" shape `driver/tuning.rs`'s own sweep tables already
//! use - and reports the distribution, not a single lucky/unlucky count.

use oag_game::race;
use oag_gameplay::PlayerInputs;

/// How many ticks past the boost's own duration to keep watching, for the
/// coast into whatever corner the boost carried the craft towards. Ours -
/// enough to cover the "arrives at 270 having looked 160 units ahead" shape
/// `ai.md` describes without folding in the craft's next, unrelated Turbo.
const COAST_TICKS: u32 = 120;

/// Circuits the widened sweep runs on, beyond the original `16_Track`.
///
/// **Chosen, not measured** - both are picked from `docs/gameplay/ai.md`'s
/// own prior findings about circuit geometry, not drawn at random:
///
/// - `07_Track` has the disc's tightest measured arc: curvature 0.047,
///   radius 21, admitting 33 units/s, where the grip term alone already
///   predicted 74 and a craft arrived at 94 (see "The second was missing and
///   it cost a circuit" in `ai.md`'s speed-model section). A corner that
///   already overspeeds an *unboosted* craft this badly is the most likely
///   place left for a boosted one to leave the road.
/// - `09_Track` is independently flagged twice in `ai.md` as the worst case
///   for a craft finishing a lap with the least shield left, across both the
///   roll-arming and shield-economics sweeps - a circuit that already
///   punishes a driving mistake hardest is a reasonable place to look for a
///   Turbo one.
///
/// `13_Track` is deliberately excluded even though it is on the twelve-
/// circuit list: it carries its own, separately chased and unrelated
/// pathology (`ai.md`'s "The `13_Track` Novice pathology, chased" section,
/// the authored jump at `grip_believed` 0.30) that would confound a Turbo
/// excursion count with a different failure entirely.
const TRACKS: [&str; 3] = ["16_Track", "07_Track", "09_Track"];

struct RunStats {
    /// Forward speed, across every craft and every tick - confirms whether a
    /// given `look_max` can ever actually bind the lookahead clamp at all.
    max_speed: f32,
    /// One entry per Turbo fired, its peak distance from the spline over the
    /// boost-plus-coast window.
    excursions: Vec<f32>,
    /// How many of those exceeded the real rescue threshold - the same one
    /// `Race::lost_off_the_circuit` (opponents) checks.
    escapes: u32,
    /// [`oag_race::recovery::RESCUE_HALF_WIDTHS`] resolved against this track, for scale -
    /// same value on every seed, since it depends on the circuit alone.
    rescue_distance: f32,
}

fn run(seeds: &[u64], track: &str, look_max: f32) -> RunStats {
    let image = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");
    let mut stats = RunStats {
        max_speed: 0.0,
        excursions: Vec::new(),
        escapes: 0,
        rescue_distance: 0.0,
    };
    for &seed in seeds {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            difficulty: oag_ai::Difficulty::Ace,
            track: Some(format!(r"Data\Environments\{track}\track.vex")),
            seed: Some(seed),
            ..race::Options::default()
        })
        .expect("loading the race");
        let mut world_race = race::Race::start(loaded.setup);
        let tuning = oag_ai::Tuning {
            look_max,
            ..oag_ai::Tuning::default()
        };
        world_race.set_ai_tuning(oag_ai::Difficulty::Ace.tune(&tuning));

        let rescue_distance =
            world_race.spline().max_half_width() * oag_race::recovery::RESCUE_HALF_WIDTHS;
        stats.rescue_distance = rescue_distance;

        // Per-slot: ticks left to watch, and the peak distance seen so far in
        // this watch.
        let mut watch = [0u32; 8];
        let mut peak = [0f32; 8];
        let mut prev_turbo = [0.0f32; 8];
        for _ in 0..3_600u32 {
            world_race.tick(&PlayerInputs::none());
            for slot in 1..8 {
                let ship = &world_race.sim.world.ships[slot];
                let turbo = ship.physics.turbo_timer;
                let speed = ship
                    .physics
                    .body
                    .linear_velocity
                    .dot(ship.physics.body.forward())
                    .max(0.0);
                stats.max_speed = stats.max_speed.max(speed);

                if prev_turbo[slot] <= 0.0 && turbo > 0.0 {
                    if watch[slot] > 0 {
                        // A prior watch was still running - close it out first
                        // so no excursion is dropped.
                        stats.excursions.push(peak[slot]);
                        if peak[slot] > rescue_distance {
                            stats.escapes += 1;
                        }
                    }
                    watch[slot] = (turbo * 60.0).ceil() as u32 + COAST_TICKS;
                    peak[slot] = 0.0;
                }
                prev_turbo[slot] = turbo;

                if watch[slot] > 0 {
                    if let Some(distance) =
                        world_race.spline().distance_to(ship.physics.body.position)
                    {
                        peak[slot] = peak[slot].max(distance);
                    }
                    watch[slot] -= 1;
                    if watch[slot] == 0 {
                        stats.excursions.push(peak[slot]);
                        if peak[slot] > rescue_distance {
                            stats.escapes += 1;
                        }
                    }
                }
            }
        }
    }
    stats
}

fn report(label: &str, stats: &RunStats) {
    let n = stats.excursions.len().max(1) as f32;
    let mean = stats.excursions.iter().sum::<f32>() / n;
    let worst = stats.excursions.iter().cloned().fold(0.0f32, f32::max);
    println!(
        "{label}: {} Turbo(s) fired, mean excursion {mean:.1}, \
         worst {worst:.1} (rescue at {:.0}), {} escape(s), max speed seen {:.1}",
        stats.excursions.len(),
        stats.rescue_distance,
        stats.escapes,
        stats.max_speed,
    );
}

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");
    if !path.exists() {
        eprintln!("no disc image at {}, nothing to measure", path.display());
        return;
    }

    // 32 seeds, seven opponents, a minute each - 224 craft-minutes, eight
    // times the 28 craft-minutes `docs/gameplay/ai.md` measured the residual
    // escape rate against.
    let seeds: Vec<u64> = (1..=32).collect();

    // 90 is today's default. 100/110/120 probe just above and below where the
    // arithmetic (`look_min + look_speed * speed > look_max`) says the clamp
    // starts to matter for a boosted craft. 200 is the "clamp essentially
    // never binds" control. This part of the sweep stays on `16_Track` alone -
    // it already answered the `look_max` question there.
    println!("-- look_max, 16_Track --");
    for look_max in [90.0f32, 100.0, 110.0, 120.0, 200.0] {
        let stats = run(&seeds, "16_Track", look_max);
        report(&format!("look_max {look_max:>5.0}"), &stats);
    }

    // Widened part: shipped look_max only, across TRACKS.
    println!("-- circuits, look_max 90 (shipped) --");
    for track in TRACKS {
        let stats = run(&seeds, track, 90.0);
        report(track, &stats);
    }
}
