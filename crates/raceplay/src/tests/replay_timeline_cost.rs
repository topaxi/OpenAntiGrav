//! What a whole-race timeline costs, per title: the snapshot's bytes split into the part
//! that changes and the part that never does, a tick, a seek, a full pass, the pose track
//! of a full grid under three encodings, and how long a craft lives when a pad takes it
//! over with nothing held.
//!
//! **A scratch sweep, not a test.** It prints tables and asserts nothing about the numbers
//! (a wall clock on a shared machine is not a verdict); it is `#[ignore]`d and returns at
//! once without `OAG_SWEEP`. One title per process, so the heap figures start from a clean
//! heap:
//!
//! ```sh
//! OAG_SWEEP=1 OAG_SWEEP_TITLE=pulse OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay \
//!     --run-ignored all --no-capture -E 'test(/replay_timeline_cost::/)'
//! ```
//!
//! `OAG_SWEEP_CLASS` is the speed class (default `VENOM`); `OAG_SWEEP_TITLE` is `pulse`, `pure`, `hd`, `2048` or `omega`; `OAG_SWEEP_TICKS` the run's
//! length (default 18,000, five minutes); `OAG_SWEEP_OUT` a directory the encoded pose tracks
//! are written to, for a general-purpose compressor to be run over. The tables are recorded
//! in `docs/architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md`.
//!
//! Bytes are counted by an allocator that tallies the calling thread's live heap, so a
//! clone's size is exact rather than read off the resident set.

#![allow(unsafe_code)]

use super::*;
use oag_gameplay::PlayerInputs;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::time::Instant;

struct Counting;

thread_local! {
    static LIVE: Cell<i64> = const { Cell::new(0) };
}

fn add(bytes: i64) {
    // `try_with`: an allocation made while this thread's locals are torn down is not counted.
    let _ = LIVE.try_with(|n| n.set(n.get() + bytes));
}

// SAFETY: every call forwards to `System` unchanged; the counter is a thread-local `Cell`
// that never allocates.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        add(layout.size() as i64);
        // SAFETY: the caller's contract, passed through.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        add(layout.size() as i64);
        // SAFETY: as above.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        add(-(layout.size() as i64));
        // SAFETY: as above.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        add(new_size as i64 - layout.size() as i64);
        // SAFETY: as above.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn live() -> i64 {
    LIVE.with(Cell::get)
}

/// Heap bytes `make`'s value holds, struct body excluded (the bytes it points to).
fn heap_bytes<T>(make: impl FnOnce() -> T) -> (T, i64) {
    let before = live();
    let value = make();
    let held = live() - before;
    (value, held)
}

fn mib(bytes: i64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn loadavg() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .map(|s| s.split_whitespace().take(3).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

fn median(samples: &mut [f64]) -> (f64, f64, f64) {
    samples.sort_by(f64::total_cmp);
    (
        samples[samples.len() / 2],
        samples[0],
        samples[samples.len() - 1],
    )
}

fn fingerprint(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A title's source and class, or `None` without its data.
fn source(title: &str) -> Option<(String, String)> {
    let path = match title {
        "pulse" => oag_testdata::image("data/images/pulse-psp-usa.chd")?,
        "pure" => oag_testdata::image("data/images/pure-psp-eu.chd")?,
        "hd" => oag_testdata::exact("data/images/hdfury-ps3-eu-dec.iso")?,
        "2048" => oag_testdata::exact("data/extracted/vita/PCSF00007")?,
        "omega" => oag_testdata::exact("data/extracted/ps4")?,
        other => panic!("OAG_SWEEP_TITLE {other}: pulse, pure, hd, 2048 or omega"),
    };
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "VENOM".to_string());
    Some((path.display().to_string(), class))
}

/// What one snapshot holds of its own, and the bytes of the shared half it points at.
struct Split {
    /// The heap of one [`RaceSim`] clone: the mutable half.
    whole: i64,
    world: i64,
    track: i64,
    tables: i64,
    plans: i64,
}

impl Split {
    fn of(sim: &RaceSim) -> Self {
        let (_, whole) = heap_bytes(|| sim.clone());
        let (_, world) = heap_bytes(|| sim.world.clone());
        let (_, track) = heap_bytes(|| {
            (
                (*sim.collision).clone(),
                (*sim.spline).clone(),
                (*sim.racing_line).clone(),
                sim.ai_order.to_vec(),
                sim.routes.to_vec(),
                sim.speedup_pads.to_vec(),
                sim.weapon_pads.to_vec(),
                sim.course.as_deref().cloned(),
            )
        });
        let (_, tables) = heap_bytes(|| {
            (
                sim.weapons.as_deref().cloned(),
                sim.weapon_ai.as_deref().cloned(),
                sim.allowed_weapons.to_vec(),
                sim.original_tables.as_deref().copied(),
                sim.class.to_string(),
            )
        });
        let (_, plans) = heap_bytes(|| {
            (
                sim.speed_plan.as_deref().cloned(),
                sim.hull_plans.to_vec(),
                sim.plan_reference,
            )
        });
        Self {
            whole,
            world,
            track,
            tables,
            plans,
        }
    }

    fn constant(&self) -> i64 {
        self.track + self.tables + self.plans
    }
}

/// A text fingerprint of every part [`Split`] counts as constant, one per field.
fn constant_part(sim: &RaceSim) -> Vec<(&'static str, u64)> {
    let f = |name, text: String| (name, fingerprint(&text));
    vec![
        f("collision", format!("{:?}", sim.collision)),
        f("spline", format!("{:?}", sim.spline)),
        f("racing_line", format!("{:?}", sim.racing_line)),
        f("ai_order", format!("{:?}", sim.ai_order)),
        f("routes", format!("{:?}", sim.routes)),
        f("speedup_pads", format!("{:?}", sim.speedup_pads)),
        f("weapon_pads", format!("{:?}", sim.weapon_pads)),
        f("course", format!("{:?}", sim.course)),
        f("weapons", format!("{:?}", sim.weapons)),
        f("weapon_ai", format!("{:?}", sim.weapon_ai)),
        f("allowed_weapons", format!("{:?}", sim.allowed_weapons)),
        f("class", format!("{:?}", sim.class)),
        f("speed_plan", format!("{:?}", sim.speed_plan)),
        f("hull_plans", format!("{:?}", sim.hull_plans)),
    ]
}

/// The fields of `a` whose fingerprint is not `b`'s.
fn changed(a: &[(&'static str, u64)], b: &[(&'static str, u64)]) -> Vec<&'static str> {
    a.iter()
        .zip(b)
        .filter(|(x, y)| x.1 != y.1)
        .map(|(x, _)| x.0)
        .collect()
}

/// One craft's drawn pose: where `Race::player_pose` puts the player's, for any slot.
fn pose(race: &Race, slot: usize) -> (Vec3, Quat) {
    let ship = &race.sim.world.ships[slot];
    let roll = oag_render::roll::display_rotation(
        Vec3::NEG_Z,
        ship.physics.roll_phase,
        ship.physics.steer_lean,
    );
    (
        ship.physics.body.position,
        ship.physics.body.orientation * roll,
    )
}

fn zigzag(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}

fn varint(out: &mut Vec<u8>, mut v: u32) {
    while v >= 0x80 {
        out.push((v & 0x7f) as u8 | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

/// A unit quaternion in four bytes: the largest component dropped, the other three in ten
/// bits each over `[-1/sqrt 2, 1/sqrt 2]`, and two bits to say which was dropped.
fn pack_rotation(q: Quat) -> u32 {
    let q = q.normalize();
    let c = [q.x, q.y, q.z, q.w];
    let largest = (0..4)
        .max_by(|&a, &b| c[a].abs().total_cmp(&c[b].abs()))
        .unwrap_or(3);
    let sign = if c[largest] < 0.0 { -1.0 } else { 1.0 };
    let scale = 511.0 * std::f32::consts::SQRT_2;
    let mut word = largest as u32;
    for v in c
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != largest)
        .map(|(_, v)| v)
    {
        let n = ((v * sign * scale).round() as i32 + 512).clamp(0, 1023) as u32;
        word = (word << 10) | n;
    }
    word
}

fn unpack_rotation(word: u32) -> Quat {
    let largest = (word >> 30) as usize;
    let scale = 511.0 * std::f32::consts::SQRT_2;
    let parts = [
        ((word >> 20) & 1023) as f32,
        ((word >> 10) & 1023) as f32,
        (word & 1023) as f32,
    ]
    .map(|n| (n - 512.0) / scale);
    let dropped = (1.0 - parts.iter().map(|p| p * p).sum::<f32>())
        .max(0.0)
        .sqrt();
    let mut c = [0.0; 4];
    let mut next = 0;
    for (i, slot) in c.iter_mut().enumerate() {
        if i == largest {
            *slot = dropped;
        } else {
            *slot = parts[next];
            next += 1;
        }
    }
    Quat::from_xyzw(c[0], c[1], c[2], c[3]).normalize()
}

fn degrees_between(a: Quat, b: Quat) -> f32 {
    (2.0 * a.dot(b).abs().min(1.0).acos()).to_degrees()
}

/// A pose track under test: every slot's pose at every tick.
struct Track {
    slots: usize,
    poses: Vec<Vec<(Vec3, Quat)>>,
}

/// The ticks `slot` gets a key on: every `stride`-th, the last, and then - when a tolerance
/// (position units, degrees) is given - the tick of the worst interpolation error in any
/// segment over it, until no tick between two keys is further from the truth than that.
fn key_ticks(
    track: &Track,
    slot: usize,
    stride: usize,
    tolerance: Option<(f32, f32)>,
) -> Vec<usize> {
    let ticks = track.poses.len();
    let mut keys: std::collections::BTreeSet<usize> = (0..ticks).step_by(stride).collect();
    keys.insert(ticks - 1);
    let Some((max_pos, max_rot)) = tolerance else {
        return keys.into_iter().collect();
    };
    loop {
        let list: Vec<usize> = keys.iter().copied().collect();
        let mut added = false;
        for pair in list.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (pa, qa) = track.poses[a][slot];
            let (pb, qb) = track.poses[b][slot];
            let worst = (a + 1..b)
                .map(|t| {
                    let f = (t - a) as f32 / (b - a) as f32;
                    let (p, q) = track.poses[t][slot];
                    let dp = (pa.lerp(pb, f) - p).length() / max_pos;
                    let dq = degrees_between(qa.slerp(qb, f), q) / max_rot;
                    (t, dp.max(dq))
                })
                .max_by(|x, y| x.1.total_cmp(&y.1));
            if let Some((t, over)) = worst
                && over > 1.0
            {
                keys.insert(t);
                added = true;
            }
        }
        if !added {
            return keys.into_iter().collect();
        }
    }
}

/// The track as one slot-major delta stream: the first key of a slot in full, every later
/// key as the position step in `1/steps_per_unit` units (zigzag varint per axis) and the
/// four-byte rotation. `stride` keeps every n-th tick. Returns the bytes and the decoded
/// track, ticks between keys linearly interpolated.
#[allow(clippy::needless_range_loop)]
fn encode(
    track: &Track,
    stride: usize,
    steps_per_unit: f32,
    tolerance: Option<(f32, f32)>,
) -> (Vec<u8>, Vec<Vec<(Vec3, Quat)>>) {
    let mut out = Vec::new();
    let ticks = track.poses.len();
    let mut decoded = vec![vec![(Vec3::ZERO, Quat::IDENTITY); track.slots]; ticks];
    for slot in 0..track.slots {
        let mut previous = [0i32; 3];
        let mut keys: Vec<(usize, Vec3, Quat)> = Vec::new();
        let key_ticks = key_ticks(track, slot, stride, tolerance);
        for tick in key_ticks {
            let (position, rotation) = track.poses[tick][slot];
            let q = (position * steps_per_unit).round();
            let q = [q.x as i32, q.y as i32, q.z as i32];
            for axis in 0..3 {
                varint(&mut out, zigzag(q[axis] - previous[axis]));
            }
            previous = q;
            let word = pack_rotation(rotation);
            out.extend_from_slice(&word.to_le_bytes());
            keys.push((
                tick,
                Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32) / steps_per_unit,
                unpack_rotation(word),
            ));
        }
        for pair in keys.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            for tick in a.0..b.0 {
                let t = (tick - a.0) as f32 / (b.0 - a.0) as f32;
                decoded[tick][slot] = (a.1.lerp(b.1, t), a.2.slerp(b.2, t));
            }
        }
        if let Some(last) = keys.last() {
            for tick in last.0..ticks {
                decoded[tick][slot] = (last.1, last.2);
            }
        }
    }
    (out, decoded)
}

/// Position error (units) and rotation error (degrees): 50th, 99th, 99.9th percentile and max.
fn error_of(track: &Track, decoded: &[Vec<(Vec3, Quat)>]) -> ([f32; 4], [f32; 4]) {
    let mut pos = Vec::new();
    let mut rot = Vec::new();
    for (truth, got) in track.poses.iter().zip(decoded) {
        for (t, g) in truth.iter().zip(got) {
            pos.push((t.0 - g.0).length());
            rot.push(degrees_between(t.1, g.1));
        }
    }
    let pick = |v: &mut Vec<f32>| {
        v.sort_by(f32::total_cmp);
        let at = |p: f64| v[((v.len() as f64 * p) as usize).min(v.len() - 1)];
        [at(0.5), at(0.99), at(0.999), v[v.len() - 1]]
    };
    (pick(&mut pos), pick(&mut rot))
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1 and OAG_SWEEP_TITLE, read the tables"]
fn replay_timeline_cost_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let title = std::env::var("OAG_SWEEP_TITLE").unwrap_or_else(|_| "pulse".to_string());
    let ticks: u64 = std::env::var("OAG_SWEEP_TICKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(18_000);
    let Some((source, class)) = source(&title) else {
        return;
    };
    println!("== {title}: load at start {}", loadavg());
    let loaded = crate::load(&crate::Options {
        source,
        class,
        mode: Mode::SingleRace,
        ..crate::Options::default()
    })
    .expect("loading the race");
    let setup = loaded.setup.clone();

    let started = Instant::now();
    let mut race = Race::start(setup.clone());
    let start_ms = started.elapsed().as_secs_f64() * 1e3;
    let started = Instant::now();
    let copy = Race::start(setup.clone());
    let copy_ms = started.elapsed().as_secs_f64() * 1e3;
    drop(copy);
    println!(
        "start: Race::start(setup.clone()) {start_ms:.0} ms first, {copy_ms:.0} ms second; \
         ships {}; size_of RaceSim {} bytes, Race {} bytes",
        race.ship_count(),
        std::mem::size_of::<RaceSim>(),
        std::mem::size_of::<Race>(),
    );

    race.set_autopilot(true);
    let idle = PlayerInputs::single(oag_gameplay::InputSnapshot::default());
    let slots = usize::from(race.ship_count());

    let before_run = Split::of(&race.sim);
    let constants_at_start = constant_part(&race.sim);
    let mut keyframes: Vec<(u64, RaceSim)> = Vec::new();
    let mut hashes: Vec<(u64, u64)> = Vec::new();
    let mut probes: Vec<(u64, RaceSim)> = Vec::new();
    let mut track = Track {
        slots,
        poses: Vec::new(),
    };
    let run = Instant::now();
    let mut finished_at = None;
    let mut first_tick_ms = 0.0;
    let mut constants_after_first = Vec::new();
    for _ in 0..ticks {
        let t = Instant::now();
        race.tick_sim_only(&idle);
        if track.poses.is_empty() {
            first_tick_ms = t.elapsed().as_secs_f64() * 1e3;
            constants_after_first = constant_part(&race.sim);
        }
        let now = race.sim.world.tick;
        if now.is_multiple_of(60) {
            keyframes.push((now, race.snapshot_sim()));
        }
        if now % 397 == 1 {
            hashes.push((now, race.sim.state_hash()));
        }
        if [617, 5117, 9017].contains(&now) {
            probes.push((now, race.snapshot_sim()));
        }
        if finished_at.is_none() && race.finished() {
            finished_at = Some(now);
        }
        track
            .poses
            .push((0..slots).map(|slot| pose(&race, slot)).collect());
    }
    let full_pass = run.elapsed().as_secs_f64();
    println!(
        "full pass: {ticks} sim-only ticks with poses read and a snapshot a 60 in {full_pass:.2} s \
         ({:.0} us a tick); race finished at tick {finished_at:?}; load {}",
        full_pass * 1e6 / ticks as f64,
        loadavg()
    );
    let constants_at_end = constant_part(&race.sim);
    println!(
        "first tick {first_tick_ms:.1} ms (it learns the speed plans); fields that differ \
         between Race::start and the first tick: {:?}; between the first tick and the end: {:?}",
        changed(&constants_at_start, &constants_after_first),
        changed(&constants_after_first, &constants_at_end),
    );
    let after_run = Split::of(&race.sim);
    println!("heap, KiB: a snapshot's own / world / shared: track / tables / plans");
    for (label, s) in [("tick 0", &before_run), ("end", &after_run)] {
        println!(
            "  {label:>6}: {:.1} / {:.0} / {:.0} / {:.0} / {:.0}   shared half {:.0} KiB",
            s.whole as f64 / 1024.0,
            s.world as f64 / 1024.0,
            s.track as f64 / 1024.0,
            s.tables as f64 / 1024.0,
            s.plans as f64 / 1024.0,
            s.constant() as f64 / 1024.0,
        );
    }
    let inline = std::mem::size_of::<RaceSim>() as i64;
    let at_end = race.snapshot_sim();
    let largest = keyframes
        .iter()
        .map(|(_, s)| heap_bytes(|| s.clone()).1)
        .max()
        .unwrap_or(0);
    println!(
        "snapshot bytes: {} at the end, {} at the largest keyframe ({inline} inline plus heap), \
         limit 40960",
        inline + after_run.whole,
        inline + largest
    );
    println!(
        "snapshot_bytes, the budget's estimate: {} at the end, {} at the largest keyframe",
        at_end.snapshot_bytes(),
        keyframes
            .iter()
            .map(|(_, s)| s.snapshot_bytes())
            .max()
            .unwrap_or(0)
    );
    println!(
        "keyframes holding the race's own shared half: {} of {}",
        keyframes
            .iter()
            .filter(|(_, s)| s.shares_constant_half_with(&at_end))
            .count(),
        keyframes.len()
    );
    let mutable_total: i64 = keyframes
        .iter()
        .map(|(_, s)| heap_bytes(|| s.clone()).1)
        .sum();
    println!(
        "keyframes every 60 ticks: {} held, mutable bytes {:.1} MiB in all, {:.0} KiB each on average",
        keyframes.len(),
        mib(mutable_total),
        mutable_total as f64 / 1024.0 / keyframes.len().max(1) as f64
    );

    let mut clone_us = Vec::new();
    for _ in 0..7 {
        let t = Instant::now();
        for _ in 0..50 {
            std::hint::black_box(race.snapshot_sim());
        }
        clone_us.push(t.elapsed().as_secs_f64() * 1e6 / 50.0);
    }
    let (m, lo, hi) = median(&mut clone_us);
    println!(
        "snapshot_sim: {m:.0} us ({lo:.0}-{hi:.0}); load {}",
        loadavg()
    );

    let mut world_us = Vec::new();
    for _ in 0..7 {
        let t = Instant::now();
        for _ in 0..50 {
            std::hint::black_box(race.sim.world.clone());
        }
        world_us.push(t.elapsed().as_secs_f64() * 1e6 / 50.0);
    }
    let (m, lo, hi) = median(&mut world_us);
    println!(
        "World::clone alone (the floor of a snapshot that shares the track): {m:.0} us ({lo:.0}-{hi:.0}); \
         size_of World {} bytes",
        std::mem::size_of::<oag_gameplay::World>()
    );

    let s0 = keyframes[keyframes.len() / 2].1.clone();
    let (mut normal, mut quiet) = (Vec::new(), Vec::new());
    for repeat in 0..7 {
        for first_quiet in [repeat % 2 == 1, repeat % 2 == 0] {
            race.restore_sim(&s0);
            let t = Instant::now();
            for _ in 0..240 {
                if first_quiet {
                    race.tick_sim_only(&idle);
                } else {
                    race.tick(&idle);
                    race.drain_cues();
                }
            }
            let us = t.elapsed().as_secs_f64() * 1e6 / 240.0;
            if first_quiet {
                quiet.push(us);
            } else {
                normal.push(us);
            }
        }
    }
    let (n, nlo, nhi) = median(&mut normal);
    let (q, qlo, qhi) = median(&mut quiet);
    println!(
        "tick: Race::tick {n:.0} us ({nlo:.0}-{nhi:.0}), tick_sim_only {q:.0} us ({qlo:.0}-{qhi:.0}); load {}",
        loadavg()
    );

    println!("seek (restore the keyframe at or before the target, resimulate to it):");
    for interval in [60u64, 300, 1800] {
        let frames: Vec<&(u64, RaceSim)> = keyframes
            .iter()
            .filter(|(tick, _)| tick % interval == 0)
            .collect();
        let mut ms = Vec::new();
        let mut agree = (0, 0);
        for target in (1..ticks).filter(|t| t % 397 == 1) {
            let Some(&&(from, ref sim)) = frames.iter().rev().find(|(t, _)| *t <= target) else {
                continue;
            };
            let t = Instant::now();
            race.restore_sim(sim);
            for _ in from..target {
                race.tick_sim_only(&idle);
            }
            ms.push(t.elapsed().as_secs_f64() * 1e3);
            if let Some(&(_, hash)) = hashes.iter().find(|(t, _)| *t == target) {
                agree.1 += 1;
                agree.0 += usize::from(race.sim.state_hash() == hash);
            }
        }
        let (m, lo, hi) = median(&mut ms);
        println!(
            "  interval {interval:>5} ticks ({} keyframes): median {m:.2} ms ({lo:.2}-{hi:.2}); \
             hash agrees at {}/{} checked targets",
            frames.len(),
            agree.0,
            agree.1
        );
    }

    for (target, truth) in &probes {
        let Some((from, sim)) = keyframes.iter().rev().find(|(t, _)| t <= target) else {
            continue;
        };
        race.restore_sim(sim);
        for _ in *from..*target {
            race.tick_sim_only(&idle);
        }
        println!(
            "  RaceSim as text after a seek to tick {target} equals the original run's: {}",
            format!("{:?}", race.sim) == format!("{truth:?}")
        );
    }
    let mut hash_us = Vec::new();
    for _ in 0..7 {
        let t = Instant::now();
        for _ in 0..50 {
            std::hint::black_box(race.sim.state_hash());
        }
        hash_us.push(t.elapsed().as_secs_f64() * 1e6 / 50.0);
    }
    let (m, lo, hi) = median(&mut hash_us);
    println!("state_hash: {m:.0} us ({lo:.0}-{hi:.0})");

    println!("take over with a pad that holds nothing / only the throttle:");
    let mut dead = Vec::new();
    let mut thrust = Vec::new();
    let mut shield_after = Vec::new();
    let press = oag_gameplay::input::Button::Cross.bit();
    let held = PlayerInputs::single(crate::HeldButtons::new(press).snapshot());
    for (_, sim) in keyframes.iter().skip(2).step_by(5) {
        for (inputs, out) in [(&idle, &mut dead), (&held, &mut thrust)] {
            race.restore_sim(sim);
            race.set_autopilot(false);
            let base = race.sim.wall_contact_ticks[0];
            let shield = race.ship().physics.shield;
            let mut first = None;
            for k in 0..600u32 {
                race.tick_sim_only(inputs);
                if first.is_none() && race.sim.wall_contact_ticks[0] > base {
                    first = Some(k);
                }
            }
            out.push(first.map_or(600.0, f64::from));
            if std::ptr::eq(inputs, &held) {
                shield_after.push(f64::from(shield - race.ship().physics.shield));
            }
        }
        race.set_autopilot(true);
    }
    let report = |name: &str, v: &mut Vec<f64>| {
        let (m, lo, hi) = median(v);
        let under = |n: f64| v.iter().filter(|&&x| x < n).count();
        println!(
            "  {name}: ticks to first wall contact, median {m:.0}, min {lo:.0}, max {hi:.0} \
             (600 = none in 10 s); under 30 ticks {}, under 60 {}, under 120 {}, of {}",
            under(30.0),
            under(60.0),
            under(120.0),
            v.len()
        );
    };
    report("nothing held", &mut dead);
    report("throttle held", &mut thrust);
    let (m, lo, hi) = median(&mut shield_after);
    println!("  shield lost in 10 s, throttle held: median {m:.1} ({lo:.1}-{hi:.1})");

    println!(
        "pose track, {slots} craft, {ticks} ticks ({:.0} s):",
        ticks as f64 / 60.0
    );
    println!(
        "  raw f32 x7 every tick (ADR-0055): {:.1} B per craft per s",
        28.0 * 60.0
    );
    let mut steps: Vec<f32> = (1..track.poses.len())
        .flat_map(|t| {
            let track = &track;
            (0..slots).map(move |s| (track.poses[t][s].0 - track.poses[t - 1][s].0).length())
        })
        .collect();
    steps.sort_by(f32::total_cmp);
    let at = |p: f64| steps[((steps.len() as f64 * p) as usize).min(steps.len() - 1)];
    println!(
        "  position step per tick, units: p50 {:.3} p99 {:.3} p99.9 {:.3} p99.99 {:.3} max {:.3}",
        at(0.5),
        at(0.99),
        at(0.999),
        at(0.9999),
        steps[steps.len() - 1]
    );
    let out_dir = std::env::var_os("OAG_SWEEP_OUT").map(std::path::PathBuf::from);
    for (name, stride, steps, tolerance) in [
        ("60 Hz, 1/64 unit", 1usize, 64.0f32, None),
        ("60 Hz, 1/16 unit", 1, 16.0, None),
        ("20 Hz keys, 1/64 unit", 3, 64.0, None),
        ("10 Hz keys, 1/64 unit", 6, 64.0, None),
        (
            "20 Hz keys refined to 0.05 unit and 0.5 degree, 1/64 unit",
            3,
            64.0,
            Some((0.05, 0.5)),
        ),
        (
            "10 Hz keys refined to 0.05 unit and 0.5 degree, 1/64 unit",
            6,
            64.0,
            Some((0.05, 0.5)),
        ),
        (
            "10 Hz keys refined to 0.02 unit and 0.2 degree, 1/64 unit",
            6,
            64.0,
            Some((0.02, 0.2)),
        ),
    ] {
        let (bytes, decoded) = encode(&track, stride, steps, tolerance);
        let (pos, rot) = error_of(&track, &decoded);
        let per = bytes.len() as f64 / slots as f64 / (ticks as f64 / 60.0);
        println!(
            "  {name}: {} bytes, {per:.1} B per craft per s, whole grid {:.1} KiB per s; error vs truth: \
             position p50/p99/p99.9/max {:.4}/{:.4}/{:.4}/{:.4} units, rotation {:.3}/{:.3}/{:.3}/{:.3} degrees",
            bytes.len(),
            per * slots as f64 / 1024.0,
            pos[0],
            pos[1],
            pos[2],
            pos[3],
            rot[0],
            rot[1],
            rot[2],
            rot[3]
        );
        if let Some(dir) = &out_dir {
            let file = dir.join(format!(
                "{title}-{}.bin",
                name.replace([' ', ',', '/'], "_")
            ));
            std::fs::write(file, &bytes).expect("writing the encoded track");
        }
    }
    println!("== {title}: load at end {}", loadavg());
    let _ = mib(0);
}
