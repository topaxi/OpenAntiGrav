//! What a reconciliation costs on a real circuit: section 3a's M2 (a replay
//! tick, normal and sim-only) and M3 (a snapshot).
//!
//! **A scratch sweep, not a test.** It prints a table and asserts nothing about
//! the numbers, because wall clock on a shared machine is not a verdict; it is
//! `#[ignore]`d and returns at once without `OAG_SWEEP`, so `just test-data`
//! never pays for it. Run it at an optimised profile and read the load average
//! printed beside each repeat:
//!
//! ```sh
//! OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay \
//!     --run-ignored all --no-capture -E 'test(/reconcile_cost::/)'
//! ```
//!
//! The table is recorded in `docs/networking/README.md`, "Reconciliation cost".

use super::*;
use crate::HeldButtons;
use oag_gameplay::PlayerInputs;
use std::time::Instant;

/// This process's resident set, in KiB.
fn rss_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("VmRSS:"))
                .and_then(|l| l.split_whitespace().next()?.parse().ok())
        })
        .unwrap_or(0)
}

/// Heap one value of `make` holds, in KiB: [`HELD`] of them at once, read off
/// the resident set. **Only meaningful on a fresh process** - a heap that an
/// earlier circuit grew and freed serves the copies from pages it already
/// holds and the delta reads zero. Run one circuit per process
/// (`OAG_SWEEP_TRACKS`), as nextest does for a single test.
fn held_kib<T>(make: impl Fn() -> T) -> f64 {
    let before = rss_kib();
    let held: Vec<T> = (0..HELD).map(|_| make()).collect();
    let delta = rss_kib().saturating_sub(before);
    drop(held);
    delta as f64 / HELD as f64
}

/// Ticks timed per repeat when the figure wanted is nanoseconds per tick.
const TICKS: u32 = 240;
/// Repeats of each timing; the median is reported, the range beside it.
const REPEATS: usize = 7;
/// Copies held at once by [`held_kib`].
const HELD: usize = 64;

fn loadavg() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .map(|s| s.split_whitespace().take(3).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

/// Median, minimum and maximum of a set of samples.
fn spread(samples: &mut [f64]) -> (f64, f64, f64) {
    samples.sort_by(f64::total_cmp);
    (
        samples[samples.len() / 2],
        samples[0],
        samples[samples.len() - 1],
    )
}

fn circuits() -> Vec<(String, String)> {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    crate::catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
        .map(|track| (track.id.clone(), track.entry_name()))
        .collect()
}

/// The immutable half of a [`RaceSim`], cloned the way a snapshot would.
fn track_half(sim: &RaceSim) -> impl Sized {
    (
        sim.collision.clone(),
        sim.spline.clone(),
        sim.racing_line.clone(),
        sim.ai_order.clone(),
        sim.routes.clone(),
        sim.speedup_pads.clone(),
        sim.weapon_pads.clone(),
        sim.weapons.clone(),
    )
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn reconcile_cost_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let only = std::env::var("OAG_SWEEP_TRACKS").ok();
    println!("load at start: {}", loadavg());
    println!(
        "size_of RaceSim: {} bytes (heap not included)",
        std::mem::size_of::<RaceSim>()
    );
    for (id, entry) in circuits() {
        if only.as_ref().is_some_and(|only| !only.contains(&id)) {
            continue;
        }
        let loaded = crate::load(&crate::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: Mode::SingleRace,
            track: Some(entry),
            ..crate::Options::default()
        })
        .expect("loading the race");
        let mut race = Race::start(loaded.setup);
        let inputs = PlayerInputs::single(
            HeldButtons::new(oag_gameplay::input::Button::Cross.bit()).snapshot(),
        );
        for _ in 0..oag_race::COUNTDOWN_TICKS + 600 {
            race.tick(&inputs);
            race.drain_cues();
        }
        let ships = race.ship_count();
        let s0 = race.snapshot_sim();

        // M3: a snapshot, whole and track half, in time and in resident bytes.
        let mut whole = Vec::new();
        let mut half = Vec::new();
        for _ in 0..REPEATS {
            let t = Instant::now();
            for _ in 0..50 {
                std::hint::black_box(race.snapshot_sim());
            }
            whole.push(t.elapsed().as_secs_f64() * 1e6 / 50.0);
            let t = Instant::now();
            for _ in 0..50 {
                std::hint::black_box(track_half(&race.sim));
            }
            half.push(t.elapsed().as_secs_f64() * 1e6 / 50.0);
        }
        let (w, wmin, wmax) = spread(&mut whole);
        let (h, hmin, hmax) = spread(&mut half);
        let whole_kib = held_kib(|| race.snapshot_sim());
        let half_kib = held_kib(|| track_half(&race.sim));
        println!(
            "{id} ships {ships} M3 snapshot: whole {w:.0} us ({wmin:.0}-{wmax:.0}) {whole_kib:.0} KiB; \
             track half {h:.0} us ({hmin:.0}-{hmax:.0}) {half_kib:.0} KiB; load {}",
            loadavg()
        );

        // M2: nanoseconds per tick, normal against sim-only, alternating which
        // goes first so a drifting machine does not favour either.
        let (mut normal, mut quiet) = (Vec::new(), Vec::new());
        let mut loads = Vec::new();
        for repeat in 0..REPEATS {
            for first_quiet in [repeat % 2 == 1, repeat % 2 == 0] {
                race.restore_sim(&s0);
                let t = Instant::now();
                for _ in 0..TICKS {
                    if first_quiet {
                        race.tick_sim_only(&inputs);
                    } else {
                        race.tick(&inputs);
                    }
                    race.drain_cues();
                }
                let us = t.elapsed().as_secs_f64() * 1e6 / f64::from(TICKS);
                if first_quiet {
                    quiet.push(us);
                } else {
                    normal.push(us);
                }
            }
            loads.push(loadavg());
        }
        let (n, nmin, nmax) = spread(&mut normal);
        let (q, qmin, qmax) = spread(&mut quiet);
        println!(
            "{id} M2 per tick: normal {n:.0} us ({nmin:.0}-{nmax:.0}), sim-only {q:.0} us \
             ({qmin:.0}-{qmax:.0}), saved {:.0}%; loads {}",
            (1.0 - q / n) * 100.0,
            loads.join(" | ")
        );

        // A whole rollback: restore plus the replay window.
        for window in [6u32, 12, 24] {
            let (mut normal, mut quiet) = (Vec::new(), Vec::new());
            for _ in 0..REPEATS * 3 {
                let t = Instant::now();
                race.restore_sim(&s0);
                for _ in 0..window {
                    race.tick_sim_only(&inputs);
                }
                quiet.push(t.elapsed().as_secs_f64() * 1e3);
                race.drain_cues();
                let t = Instant::now();
                race.restore_sim(&s0);
                for _ in 0..window {
                    race.tick(&inputs);
                }
                normal.push(t.elapsed().as_secs_f64() * 1e3);
                race.drain_cues();
            }
            let (n, nmin, nmax) = spread(&mut normal);
            let (q, qmin, qmax) = spread(&mut quiet);
            println!(
                "{id} rollback of {window} ticks: sim-only {q:.3} ms ({qmin:.3}-{qmax:.3}), \
                 normal {n:.3} ms ({nmin:.3}-{nmax:.3}); load {}",
                loadavg()
            );
        }
    }
}
