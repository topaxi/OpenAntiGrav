# ADR-0087: A replay and photo mode share one `Timeline` over a copy of the race

## Status

**Accepted** (2026-11-07, lane `replay-timeline`). The decisions the maintainer made on
2026-11-07 are marked **(M)** and are not reopened here. Everything else, the sizes, the
intervals, the budgets and the screen rows, is measured where a number exists and labelled
**chosen, not measured** where it does not.

**Supersedes [ADR-0055](0055-replays-are-inputs-and-a-ghost-is-poses.md) in three places only:**

1. Its "full-race replay viewer (not built yet)" paragraph under "When the simulation changes":
   the viewer is now designed here, it does not stop at a first desync with nothing to show,
   and its whole-race pose track is the format in decision 4, not the 28-byte-per-tick sketch.
2. "Recording is optional state ... off by default": a race that can be recorded is recorded
   (decision 1).
3. "Anything that changes `RaceSim` outside `Race::tick` is invisible to the input stream":
   the rule stands, and decision 7 gives such a writer a place in the file (an event list).

The file format, the ghost, `ghosts/` and its key, "inputs are the truth, poses are a cache of
what the truth looked like", and "validity is decided by stored hashes, never by version
strings" all stand unchanged. Builds on [ADR-0018](0018-audio-mixer-architecture.md) (cues are
a per-tick output), [ADR-0003](0003-no-ecs.md) (the world is plain data),
[ADR-0052](0052-world-and-race-tick-widen-to-n-players.md) (one input per slot) and
[ADR-0064](0064-project-toml-files-declare-a-toml-schema.md) (every TOML declares a schema).

## Context

Two features want the same machine and neither has it.

- **Photo mode** (`docs/ui/photo-mode.md`) pauses the live race and draws it from a camera. It
  cannot go back, and its motion blur and accumulated depth of field are listed as left out
  because "how a craft's motion gets into either on a frozen race was never read and the
  simulation must not tick".
- **A replay viewer** was deferred by ADR-0055: watch a whole race again, jump around in it,
  and (the maintainer's addition) take the pad and race on from any moment.

Both are "put a copy of the race at tick T, step it, draw it". The pieces already exist:

- `Race::snapshot_sim` and `restore_sim` clone and put back a whole `RaceSim`, and the clone is
  structurally complete (`crates/raceplay/src/sim.rs`, "Why it is `Clone`").
- `Race::tick_sim_only` advances the simulation to the same state as `Race::tick` and touches
  no picture or sound (`tests::reconcile::a_quiet_replay_reaches_the_hash_of_a_normal_one`).
- `oag_replay::Recorder` and `Verifier` hold the inputs and hashes of a run.
- `oag_sound::sfx::RaceFrame` is a plain-data description of one tick, built from a race.
- A second `Race` is already built from `Setup::clone()` (the web load worker's
  `Loaded::started`).

What was missing is numbers: what a snapshot weighs, what a seek costs, what a whole grid's
pose track costs, and how long a craft lives when a pad takes it over. They are measured
below, on all five titles, by one scratch sweep
(`crates/raceplay/src/tests/replay_timeline_cost.rs`, `#[ignore]`d).

### The measurements this ADR rests on

Machine: AMD Ryzen 9 7900, 24 threads, shared, so **every time is inflated by contention** and
every byte count is exact (a counting allocator tallies the live heap of a clone). The
1-minute load average read 24 to 44 for the dev-profile runs and 7 to 28 for the release-profile
runs (in order: Pulse 7, Pure 7, HD 13, 2048 17 to 19, Omega 28); the table in decision 3 gives
each title's own. Each title: Single Race, VENOM, 8 craft, the player's craft flown by the
operator's Autopilot (a means to get a race run, not a physics reference), 18,000 ticks (five
minutes; 2048's race ends at tick 12,990, the others' at 6,195 to 6,966, and the run goes on
past it), one title per process. Reproduce: the command in the sweep file's header.

**A snapshot is almost all track.** Heap bytes of one `RaceSim` clone, in KiB. "Constant" is the
collision soup, spline, racing line, pads, route lines, weapon tables and the speed plans.

| Title | Whole clone today | Constant half | Mutable heap | `size_of RaceSim` (inline) |
| --- | ---: | ---: | ---: | ---: |
| Pulse | 1,985 | 1,983 | 2 | 28,424 B |
| Pure | 1,317 | 1,315 | 3 | 28,424 B |
| HD | 3,140 | 3,137 | 4 | 28,424 B |
| 2048 | 5,210 | 5,202 | 8 | 28,424 B |
| Omega | 3,597 | 3,594 | 4 | 28,424 B |

The mutable part is `World` (20,136 B, no heap: fixed arrays, [ADR-0003](0003-no-ecs.md)), the
other 8,288 inline bytes of `RaceSim`, and 2 to 8 KiB of `Vec`s (cue queues, pad distances).
**The constant half is 97.8 to 99.3 percent of a snapshot's bytes (99.8 to 99.9 percent of its
heap), and no tick writes it.** Checked by
fingerprinting the `Debug` text of every field of it: unchanged from the first tick to tick
18,000 on all five titles. Two things are filled on the first tick, not at `Race::start`: the
speed plans (`speed_plan`, `hull_plans`: 355 to 1,088 KiB) and, on 2048, the route lines
(`routes`, +218 KiB). So "constant" means **constant after the first tick**.

**The 2048 and Omega census** (CLAUDE.md, "A 2048 finding is checked against Omega"), each
title having been measured on its own data: the snapshot split holds on both, **ported**
(constant half 5.1 MiB and 3.5 MiB, mutable 8 and 4 KiB). The route lines are the one
difference: 2048 fills `routes` on the first tick, Omega has it already at `Race::start`
(only `speed_plan` and `hull_plans` change there), **checked, differs**. A circuit that
authors a fork is what `routes` is for, so this may be a property of the circuit sampled, not
of the title; not checked further. Seek and the pose track were run on both, **ported**, with
their own numbers. The take-over table was run on both, **ported**. HD's front end is not
involved: the end-screen rows of decision 9 are per title and say so.

**What these runs prove, and what they do not.** Every run flew the player on the Autopilot
with the pad idle, so the input stream was constant. They prove that a restore plus
resimulation is deterministic and complete (hash and text) under constant input, and they
price it. They do **not** prove that a varying recorded stream re-drives correctly; that half
is `crates/raceplay/tests/replay_ground_truth.rs` (7,200 analog ticks through `Verifier` on
Pulse, Pure, HD and 2048), and stage 3's acceptance repeats the seek test over a pad-flown
recording on all five titles. The "18,000-tick pass" figures include a whole-clone snapshot
every 60 ticks (300 of them: about 0.34 s of 6.38 s on 2048) and eight pose reads a tick, so
they overstate the cost of the shared design.
`World::clone` alone, which is what a snapshot sharing the constant half would cost, is 0 to
1 us (release; 1 to 2 us dev); the whole `snapshot_sim` is 57 to 1,132 us (release: Pulse 111,
Pure 57, HD 484, 2048 1,132, Omega 381; the first networking sweep measured Pulse at 63 us on an
idle-er machine).

## Decision

### 1. Whole-race recording and saving

- **Record every race that can be recorded** (a single-player or split-screen race, not a
  modded one, not a network one: decision 7). The recorder already exists
  (`Race::start_recording`); the session that starts a player's race now arms it for every such
  race, not the ghost path alone. A race built any other way (a test, a trace, a capture, a
  server) records nothing unless it asks, so every existing golden and trace runs the code it
  ran before, which was ADR-0055's reason for "off by default". Its cost is one 32-byte `InputSnapshot` per human slot per tick in memory
  and one `state_hash` a second (12 to 53 us, release, loaded: 12 Pulse, 26 Pure, 27 HD, 53 2048,
  45 Omega).
- **Where.** `<data dir>/oag/replays/`, `dirs::data_dir()`'s, which is where `photos/` already
  is. Not the config directory: a replay is the player's content, as a photo is, and
  `ghosts/` stays where it is, beside `records.toml`.
  - `auto/oag-<title>-<unix seconds>-<n>.oagr`: written when the race ends (finish, quit to
    the menus, restart). A **ring**: the newest 20 files and at most 32 MiB, whichever is hit
    first, oldest deleted first.
  - `saved/`: the player's own. Never pruned. A replay enters it by KEEP on the REPLAYS page
    (a copy from `auto/`) or by SAVE REPLAY after a take-over (decision 9).
- **What is kept.** The whole `.oagr`: header, inputs, hashes, the pose track of every craft
  (decision 4). **No keyframes**: they are built in memory on load and never written
  **(M)**.
- **Size, measured.** Inputs are 6.3 to 6.9 bytes a tick on an analog pad that moves every tick
  (ADR-0055), so 124 KB a human seat for five minutes. The pose track of eight craft over five
  minutes is 358 to 404 KB (decision 4). Hashes are 8 bytes a second. A five-minute eight-craft
  race with one person is therefore about 0.5 MB, and a two-lap race of two minutes about a
  fifth of that. **The 20-file, 32 MiB budget is chosen, not measured**: 20 races of the
  largest kind measured is about 11 MB, so the count is what normally bites and the byte cap
  guards a pathological file (a long race, or four people).
- **How a player picks one.** A REPLAYS row on the main page, attached by the built-in mod
  `oag-extras` as PHOTO MODE is (`docs/ui/photo-mode.md`, "Entry"), opens a page listing
  `saved/` then `auto/`, newest first: circuit, class, mode, date, length, the player's finish,
  size, and flags (SAVED, TAKEN OVER). The list reads `HEAD` only (the chunk is first), a few
  hundred bytes a file. Actions: WATCH, KEEP, DELETE. The end-of-race menu gains a WATCH REPLAY
  row after VIEW RESULTS AGAIN for the race just driven. Chosen, not measured, as is the
  setting `[replay] record` (default on) that turns recording off; it needs a `settings.tosd`
  key.
- **The ghost is unchanged.** `ghosts/` holds the best lap as poses, written by the path
  ADR-0055 describes. A whole-race replay is not a ghost and a ghost is not a replay.

### 2. Keyframes in memory

- **A `Timeline` keeps a keyframe every 60 ticks (one second)**, chosen, not measured. The
  bound it buys is measured (decision 3): a seek replays at most 59 ticks.
- **A keyframe holds the mutable half of `RaceSim` only.** The constant half (table above) is
  held once, behind an `Arc`, shared by the live race, every keyframe and the timeline's copy.
  Today `snapshot_sim` clones the whole struct; that is a bad bargain at one a second (table
  below), and **splitting the constant half out is the first stage of the plan**, not an
  optimisation to find later. It is a change to a simulation crate, so the stage's proof is
  that no golden hash moves and that a restored `RaceSim` equals the original as text
  (`a_quiet_replay_reaches_the_hash_of_a_normal_one` already compares the text).
- **Per title, five minutes, a keyframe a second (300 keyframes):**

| Title | One keyframe, shared | 300 shared | One keyframe, whole clone | 300 whole clones |
| --- | ---: | ---: | ---: | ---: |
| Pulse | 29.8 KiB | 8.7 MiB | 2.0 MiB | 590 MiB |
| Pure | 30.5 KiB | 8.9 MiB | 1.3 MiB | 394 MiB |
| HD | 30.9 KiB | 9.0 MiB | 3.1 MiB | 928 MiB |
| 2048 | 35.0 KiB | 10.2 MiB | 5.1 MiB | 1,536 MiB |
| Omega | 30.9 KiB | 9.0 MiB | 3.5 MiB | 1,062 MiB |

  (A shared keyframe is 27.8 KiB inline plus the mutable heap, averaged over the 300 keyframes
  of the run.) The whole-clone
  column is what the current `snapshot_sim` would hold: not a design option.
- **Memory ceiling: 16 MiB of keyframes plus one constant half (1.3 to 5.1 MiB).** Past it the
  interval doubles and every other keyframe is dropped, so a long race costs seek time, not
  memory. Chosen, not measured. The Steam Deck, with 16 GB shared, is not the case that sets
  it: the web build (a 4 GiB address space) and a phone are. Five minutes on the heaviest title
  (2048) is 10.2 MiB, under the ceiling at the base interval; the ceiling thins a race past
  about eight minutes on that title.
- **The live race keeps the same keyframes** while it is played, for photo rewind (decision 5):
  one `World::clone` and a few `Vec` clones a second, a microsecond or two once the constant half
  is shared, against a tick of 30 to 222 us. With no recorder (a modded race) rewind snaps
  to whole keyframes: there are no inputs to resimulate between them.
- **What a keyframe does not hold:** the view (camera spring, particles, exhaust trails, the
  six view RNGs) and the presentation journal. They are outside `RaceSim` by design
  (`tick_sim_only` documents the list), cannot move a hash, and are rebuilt at a seek
  (decision 6).

### 3. Resimulation speed and seek time, per title

`tick_sim_only`, one tick, microseconds, median of 7 repeats of 240 ticks (range in the logs).
Dev profile is this workspace's test profile (`opt-level 2` for our crates); release is
`--cargo-profile release`. A fat-LTO `dist` test binary was tried and hit the 8 GiB memory cap
the lane runs under, so `dist` is not measured; the one `dist` datum in the docs is Pulse's
19.8 us a tick (`docs/tools/packaging.md`, 100,000 ticks, 2026-10-09).

| Title | Dev (load 24-30) | Release (load) | First tick, release (learns the plans) | `Race::start(setup.clone())`, release | 18,000-tick pass, release | Seek, 60-tick keyframes, release, median (max) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Pulse | 64 | 30 (7) | 123 ms | 4 ms | 1.03 s | 1.25 (2.24) ms |
| Pure | 124 | 48 (7) | 342 ms | 3 ms | 1.36 s | 1.64 (4.06) ms |
| HD | 255 | 139 (13) | 844 ms | 6 ms | 3.87 s | 5.76 (14.77) ms |
| 2048 | 200 | 222 (19) | 1,585 ms | 24 ms | 6.38 s | 10.89 (25.25) ms |
| Omega | 124 | 90 (28) | 770 ms | 25 ms | 3.20 s | 4.44 (8.07) ms |

- **A seek is a restore plus at most 59 ticks.** Measured with 60-tick keyframes (table):
  median 1 to 11 ms, worst 2 to 25 ms, release, loaded; with 300-tick keyframes the median is
  6 to 36 ms and the worst 14 to 95 ms, and with 1,800-tick keyframes 30 to 181 ms and up to
  470 ms. 2048 can take more than a 16.7 ms frame at its worst on this loaded machine; a scrub
  there holds the last frame for a frame or two, which is fine. **Every seek reproduced the
  original run's state hash at every checked target (41 to 45 per title and interval, 15
  title-by-interval runs, 0 misses)**, and the `RaceSim` after a seek equals the original's as
  text at three targets per title, five titles (tick 617, 5,117 and 9,017: all 15 true).
- **Opening a five-minute replay is a full pass**: `Verifier` plays every input through
  `tick_sim_only`, checks every stored hash, and takes a keyframe a second. That is 1.0 to 6.4 s
  release, loaded (47 to 290 times real time; 3.1 to 4.9 s on the dev profile), plus the first
  tick's plan learning (0.12 to 1.6 s on a copy built from a file; none on a copy cut from a
  live race, which already has the plans). The
  pass runs on a worker thread (`oag-thread`) ahead of the playhead: the viewer plays from
  tick 0 at once, and a seek is allowed up to the built prefix, as a video player's buffered
  bar is. A hash that disagrees ends the prefix there (decision 4).
- **Playback is cheap**: one drawn tick is 38 to 382 us (release, loaded), a few hundredths of a
  frame.
- **The Steam Deck is not measured, and is owed.** No Deck tick figure exists anywhere in the
  docs (`docs/tools/render-bench.md` lists Deck commands "not run: the Deck is the
  maintainer's"). The owed command, on the Deck, from a checkout with Rust and the discs:
  `OAG_SWEEP=1 OAG_SWEEP_TITLE=<pulse|pure|hd|2048|omega> OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --cargo-profile release --run-ignored all --no-capture -E 'test(/replay_timeline_cost::/)'`.
  What it must show for the design to hold: a seek under one frame (16.7 ms) at its
  worst needs `tick_sim_only` at or under 280 us on the Deck (59 ticks), and an open pass of
  18,000 ticks at or under 100 times real time (3 s) needs 170 us. This desktop's release
  figures are inside the first for every title and inside the second for Pulse and Pure; **how
  far the Deck is from them is the unknown**, and the pass is the figure most likely to miss on
  HD, 2048 and Omega (the viewer plays from tick 0 while it runs, so a miss is a shorter
  buffered bar, not a wait).

### 4. A build mismatch, and the pose track as the fallback

- **How a player sees it.** Opening a replay starts the pass of decision 3. The first stored
  hash that disagrees (`Desync { tick, expected, found }`, once a second's resolution) ends the
  verified prefix. The viewer then shows, over the picture, **THIS REPLAY WAS RECORDED ON AN
  EARLIER VERSION AND STOPS MATCHING AT 1:30**, with the tick of the last agreeing hash, and:
  - the verified part plays and seeks exactly as any replay;
  - past it, if the file has a pose track, the playhead continues in **POSE VIEW**, labelled
    on screen: the hulls at their recorded poses over the circuit, with no HUD readouts, no
    weapons, pickups or effects (a pose track has none of them);
  - with no pose track (a file written before this ADR) it stops at the last agreeing second.

  A tick-zero mismatch (`Verifier::new`) means the race cannot be rebuilt at all (another
  circuit, a mod or a DLC missing, a changed roster): the list says so by name and the pose
  track, if present, still plays as a POSE VIEW from tick 0. The `Verifier` stays the judge
  ([ADR-0055](0055-replays-are-inputs-and-a-ghost-is-poses.md)); this only says what the
  player sees.
- **The pose track** is a `POSE` chunk of the `.oagr` (an unknown chunk is skipped by an older
  reader), every craft on the grid, recorded while the race is driven:
  - position quantised to 1/64 unit and delta-coded per axis as zigzag varints;
  - rotation (the drawn orientation, barrel roll and lean folded in as `Race::player_pose`
    does) as a unit quaternion in 4 bytes: the largest component dropped, the other three in
    10 bits each, 2 bits to say which;
  - **keys at 10 Hz, refined**: a key every 6th tick, and a key added at the tick of the
    worst interpolation error of any segment until no tick between two keys is more than
    0.05 unit or 0.5 degree from the truth. Playback is a lerp and a slerp between keys.
  - Chosen, not measured: the 1/64 step, the 10-bit rotation, the 10 Hz base, the tolerances.
    Measured: what they cost and what they lose, in the next table. The refinement is what
    bounds the error, and a refined 10 Hz track is *smaller* than a refined 20 Hz one, because
    a coarse base only gains keys where the line bends.
- **Measured, 8 craft, 18,000 ticks, all five titles** (the Autopilot race, so no respawn in
  it; the largest per-tick step was 4.0 to 6.6 units and none exceeded 8). Bytes a craft a
  second; position error in units and rotation error in degrees against the unquantised pose,
  p50 / p99 / p99.9 / max:

| Encoding | B per craft per s | 8 craft, 5 min | Position error (p50 / p99.9 / max) | Rotation error (p50 / p99.9 / max) |
| --- | ---: | ---: | --- | --- |
| Raw `f32` x7 every tick (ADR-0055) | 1,680 | 3.9 MB | 0 | 0 |
| 60 Hz, 1/64 unit | 484 to 500 | 1.16 to 1.20 MB | 0.0077 / 0.0127 / 0.0135 | 0.079 / 0.15 / 0.22 |
| 20 Hz keys, unrefined | 175 to 185 | 0.42 to 0.44 MB | 0.010 / 0.13 to 0.23 / 0.5 to 3.8 | 0.07 / 0.4 to 0.8 / 1.0 to 14.8 |
| 10 Hz keys, unrefined | 91 to 95 | 0.22 to 0.23 MB | 0.04 / 0.43 to 0.66 / 0.8 to 5.2 | 0.09 / 1.1 to 2.3 / 3.5 to 28.9 |
| 20 Hz refined to 0.05 unit, 0.5 degree | 186 to 199 | 0.45 to 0.48 MB | 0.010 / 0.050 to 0.053 / 0.057 to 0.058 | 0.07 / 0.29 to 0.51 / 0.52 to 0.62 |
| **10 Hz refined to 0.05 unit, 0.5 degree (chosen)** | **149 to 168** | **0.36 to 0.40 MB** | 0.014 to 0.017 / 0.053 / 0.057 to 0.059 | 0.07 to 0.08 / 0.38 to 0.51 / 0.54 to 0.62 |
| 10 Hz refined to 0.02 unit, 0.2 degree | 219 to 269 | 0.52 to 0.65 MB | 0.009 / 0.026 / 0.028 to 0.029 | 0.07 / 0.19 to 0.23 / 0.27 to 0.29 |

  Ranges are across the five titles; the 60 Hz row is the same on all of them because it is a
  property of the encoding and of eight craft on a circuit, not of a title.

  Pulse's own ghost spends about 9 bytes a key at 20 Hz, 180 a second for one craft: the
  unrefined 20 Hz row matches it, and the chosen row spends less per craft with an error bound. Without refinement the worst cases are impacts (a pair of keys straddling a
  wall hit), 3.8 units and 14.8 degrees at 20 Hz on 2048 and 5.2 units and 28.9 degrees at
  10 Hz; the refinement exists to bound that. A general-purpose compressor gains little
  (on the chosen track: gzip -9 9 percent, xz -9 28 percent on Pulse; on the 60 Hz stream gzip -9
  29 percent, zstd -19 42 percent, xz -9 55 percent), so **no compression** is applied, as
  ADR-0055 said. The ADR-0055 estimate for a two-minute eight-craft race was 1.6 MB; the
  chosen track is about 0.15 MB.
- **A pose track is not a verified state.** It is a picture of the craft on the build that
  recorded it, the same status a ghost has. It can never start a take-over (decision 9).

### 5. A timeline over a copy: the live race stays untouched

- **`oag_raceplay::timeline::Timeline`** owns: the shared constant half, the keyframe list, the
  replay's inputs (the recorder's, for a live race; the file's, for a replay), and **a second
  `Race`** (the probe) built from `Setup::clone()` and moved with `restore_sim` and
  `tick_sim_only`. It never holds `&mut` to the live `Race`. It is in the simulation half of
  `oag-raceplay` (no `view` feature needed to keep keyframes and seek), so a server can verify
  a replay without linking a renderer (CLAUDE.md, dependency rule 6).
  The pose-track codec and the file's new chunks live in `oag-replay`, which knows no race.
- **Photo mode** takes `Race::snapshot_sim()` of the live race on entry and moves the probe.
  The live race does not tick while the mode is open (as now) and is not written. **Rewind** is
  a scrub control: seek the probe to any keyframe-or-resimulated tick from the race's start to
  the live tick; leaving drops the probe and the live race resumes where it stood.
- **Proof the live race is untouched:** the existing
  `a_race_through_photo_mode_hashes_the_same_at_every_tick_as_one_without` is extended to
  rewind (same state hash at every live tick with and without a visit), and the live race's
  `RaceSim` is compared as text before and after.
- **Accumulated depth of field** (HD, 7 frames; 2048 likewise) needs no resimulation: the race
  is frozen in the original and the frames differ in the lens, so it is the same tick drawn 7
  times with the camera jittered over the aperture.
- **Motion blur** (HD 120 frames, 2048 130, shutter `(v * 0.01)^2 * 0.25` s, so at most 0.25 s
  or 15 ticks) is where a timeline is needed. **Chosen, not measured**, since the original's
  mapping of frames to time was never read: the frames are spread evenly over the shutter
  ending at the shot's tick; the probe is sought to 15 ticks before it and stepped through
  them, and sub-tick frames are drawn from the poses of the two ticks around each
  (`crates/raceplay/src/between_ticks.rs` already blends a draw between ticks). Track blur
  follows the camera, which rides the craft, so the scenery smears and the craft stays sharp;
  ship blur holds the camera at the shot's pose and the craft smears. Cost: 120 frames of the
  scene. One drawn frame's CPU record and submit was 7.3 ms on Pulse and 6.6 ms on HD (debug
  build, 1280x800, `docs/ui/photo-mode.md`, "Frame time with the mode open"): about 0.9 s of
  CPU for a shot, GPU not measured. It runs behind a progress state and abandons on any input,
  as 2048's accumulation does (`docs/ghidra/functions/vita-2048-eu-v104/photo-mode.md`).

### 6. Audio

- **Photo mode is silent while the probe resimulates** **(M)**. At its target the mode's
  existing pause audio applies (`docs/ui/photo-mode.md`, "While it is open"): nothing new plays.
- **A full replay plays audio.** Forward playback at 1x ticks the probe through `Race::tick`,
  and its cues feed the mixer exactly as a live race's do (ADR-0018: cues are a per-tick
  output, never `World` state). At other speeds the effects are muted and the music is
  untouched.
- **A seek is silent up to the target, then recovers** **(M)**: the ticks before the target run
  through `tick_sim_only`, which drops the cues, announcements and class announcements they
  raise; at the target the mixer is told to stop every effect voice, and the continuous voices
  are rebuilt **from state** by feeding the target tick's `RaceFrame` to a new
  `Sfx::rebuild(&RaceFrame)`:
  - the engine note of every slot, with its pitch lag set to its target for the craft's speed
    (`Engine`'s law is `lag += (target - lag) * 0.01` a substep, which would glide up from rest);
  - the shield, Autopilot and exploding loops (`shielded`, `autopilot_active`, `exploding`);
  - the lock-on beeps (`sight`), the quake wave (`quake_point`), live LeachBeams and every
    in-flight projectile's loop.
  One-shots whose tick is already past are **dropped**, not replayed late: a gun that fired two
  seconds before the target is not heard at the target. Ticks after the target play their cues
  normally.
- **Music is not part of a replay** **(M)**: the viewer's own soundtrack plays as for a new
  race, and a seek does not touch it (`docs/architecture/race-music-resume.md`).
- **Acceptance** is by WAV, never by ear: render the mixer to a WAV for a seek to tick T and
  for continuous playback through T, and compare the engine voices' pitch and gain at T.
- **The same rebuild serves a take-over** (decision 9), where it matters most.

### 7. What records nothing today, and what does not need a tiebreak

- **Modded races** record nothing and stay that way (`Race::start_recording` returns early with
  a log line when `is_modded`): a simulation mod's module bytes would have to be named and
  matched by the file, which is a feature of its own. Open.
- **Network play** (LAN and online) records nothing on the client: a client keeps the inputs it
  *predicted*, not the corrected ones (`Race::tick_sim_only`, "The recording does not hear a
  replayed tick"), so its file would not reproduce. Only `oag-server`, which holds the
  authoritative per-seat inputs, can write a true replay. Not staged; it needs the timeline's
  simulation half, which decision 5 keeps free of a renderer.
- **Split-screen** records: `Header::slots` lists every human slot and the recorder keeps one
  stream each. A split replay is watched in one view of a chosen craft (L and R cycle, as in
  photo mode). Whether a split recording reproduces is not measured by this lane; stage 3
  proves it with `head_to_head_people_ground_truth`'s setup.
- **Pilot Assist** is a per-seat level in `RaceSim` (`pilot_assist_levels`) written outside
  `Race::tick`. Today the level at the start is a header option nothing reads back
  (`oag_game::ghosts::race_options`), and switching it mid-race **stops the recording**
  (`pause_menu.rs`). Decision: the header's `options` become the race's rebuild request and
  are read back (stage 2), and every out-of-tick writer becomes an entry in a new **`EVNT`
  chunk**: `(tick, seat, kind, value)`, applied by the recorder's caller and by the timeline
  between the same two ticks. First kinds: `pilot_assist`, `give` (the capture flag that tops
  up a pickup), `controller` (decision 9). With it, a mid-race Pilot Assist change no longer
  ends the recording. ADR-0055's rule ("a new out-of-tick writer must become an input or be
  recorded") now has a place to be recorded.
- **Rebuilding the race.** `Setup` has many fields (laps, weapons, difficulty, roster, AI
  pilot files, mode flags). The header gains a `[setup]` table: the resolved launch request.
  **The tick-zero hash is the safety net**: a rebuild that is missing something disagrees at
  tick 0 and is refused by name, never played wrong. Stage 2 must show agreement for every
  title and mode it claims.
- **Random.** Every draw in the simulation is from `World::rng`, which is in the state hash
  (`rng.snapshot()`) and in the snapshot. The six view RNGs are outside both and are not
  restored by a seek, so a seek's sparks and shake differ from the original run's: pictures,
  not state.
- **There is no tiebreak because there is no gap**: the snapshot is a whole-struct clone, so a
  field cannot be forgotten, and the fields outside the state hash (`rolls_armed`,
  `wall_contact_ticks`, the cue backlog and the rest) travel with it. The proof is not the
  hash, which is not an oracle for completeness (`sim.rs`), but the `Debug` text of the
  restored `RaceSim` against the original, which the sweep compared at three targets per title
  (all 15 equal). The two pieces of state a tick reads that live on `Race`, not `RaceSim`
  (`capture_results`, `wreck_ended_tick`), are why a take-over is not offered after the
  finish or while the seat is wrecked (decision 9).

### 8. A `[toml-schema]` header for `HEAD`

- The `HEAD` chunk's TOML text starts with the ADR-0064 one-line header:
  `toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/replay-header.tosd", version = "1.0.0" }`.
  Kind `replay-header`, major 1; the schema is `schemas/v1/replay-header.tosd`, deployed with
  the others (`scripts/stage-schemas.sh`), and a Rust drift test runs accepted and rejected
  headers through the loader and the schema, as ADR-0064 point 7 requires.
- **A file with no header loads.** Ghost files already sit in players' config directories, as
  pilot files did when ADR-0064 was made; the loader logs it at `debug` (nobody edits a
  machine-written chunk, so a warning would be noise) and the next write adds it.
- **A header naming another kind or a later major is refused by name** through
  `oag_schema::check`, and the REPLAYS list shows it as RECORDED BY A NEWER VERSION, not as a
  damaged file.
- The reader tolerates keys it does not know, as the track reader does and `Header` already
  does (no `deny_unknown_fields`), so a later compatible addition stays in `v1`; the schema
  itself is closed. `options` is an open table of strings; stage 2 checks that `tosd`
  1.0.0-rc.2 can say so and, if it cannot, declares the table without keys.
- New in the header, all `#[serde(default)]`: `[setup]` (decision 7), `[fork]` (decision 9).
  The binary container's `u16` version does not change: unknown chunks are skipped.

### 9. Take over

A replay has **TAKE OVER AND PLAY FROM HERE** **(M)**: at a tick the player picks, the recorded
seat's input stream runs on through a hold of at most a second after GO, then stops; the live pad takes the
seat and the race goes on live.

- **On the real race, not the copy.** The viewer's probe never becomes the race the player
  drives. The session builds the ordinary race (`Race::start`, with its HUD, recorder, results
  capture and end-of-race flow, as for any race), `restore_sim`s the nearest keyframe at or
  before the take-over tick, and runs it forward: `tick_sim_only` to 60 ticks before the tick,
  then 60 drawn but silent ticks (`Race::tick`) so the camera spring, exhaust trails and
  particles settle (decision 6's cold-start rule). The recorder is seeded with the replay's
  inputs up to the tick, so the new file contains the whole race. The probe is dropped.
- **Only from a verified state.** Refused past the last agreeing hash, never from a pose track,
  never from a build whose tick-zero hash disagrees. A fork is a *new recording by this
  build*: its hashes are this build's, so it verifies here even if the parent file does not.
- **Only while the seat is racing**: not on the countdown's grid hold, not once the race has
  finished, not while the seat's craft is wrecked. Those states are where `capture_results` and
  `wreck_ended_tick` (view-side, read back by the caller) would already have fired in the
  parent and be missing from a fresh `Race`. Stage 7 either proves the fresh race derives them
  or keeps the refusal.
- **A fork.** Taking over **forks the replay at that tick** **(M)**. The saved file is the
  parent's inputs up to the fork tick `F`, then the live pad's, with its own hashes, and a
  header table that **marks it**:

  ```toml
  [fork]
  tick = 4312            # the first tick the live pad flew: the tick TAKE OVER was pressed on
  seats = [0]
  parent_build = "..."   # the parent's header.build, informational
  parent_hash = "0x0123456789abcdef"   # the parent's stored hash nearest before the tick
  ```

  It carries **no `ghost` chunk**. It is held in memory until the end-of-race menu offers
  **SAVE REPLAY**; it is never written to `auto/` by the ring.
- **Never record eligible: no records and no ghost** **(M)**. A race with a `fork` is
  `Race::taken_over()`. Every write the end-of-race flow makes is skipped when it is set:
  `records.toml` (best lap, best total, medals, `last_*`), `take_new_best_ghost` returns
  `None`, and `ghosts::auto_load_from` refuses a file with a `[fork]` table or no ghost chunk.
  The results table still shows, without a NEW RECORD or NEW GHOST marker, and a TAKEN OVER
  line. The REPLAYS list labels the file TAKEN OVER AT 1:12.
- **Which row is SAVE REPLAY, per title.** What decides the screen is
  `oag_title::FrontEnd::endrace_style` (`crates/title/src/endrace.rs`), read by
  `oag_game::endrace`. It is the last row of the end-of-race menu, offered only for a fork:
  - **Pulse (`EndRaceDialect::Pulse`):** after `ER_VIEW_AGAIN`, in the place the original puts
    `ER_SAVE_GHOST` (the last step of `EndRaceMenu_PopulateOptions`, `docs/formats/
    endrace-screens.md`, "The option list is built per mode"; the fourth row shown in a
    non-Tournament race). The original gates that row to Time Trial and Speed Lap; ours is
    offered for any mode with a fork. This build models no `SaveGhost` row, so a new
    `MenuOption::SaveReplay` follows `ViewResultsAgain` in `menu_options`.
  - **HD (`EndRaceDialect::Field`):** `Endrace Options` has four `<Block>` rows (`next_race`,
    `race_again`, return, `view_again`) and no ghost row (`docs/formats/hd-endrace-screens.md`),
    so SAVE REPLAY is a fifth block under `view_again`, at the spacing of the others.
  - **2048 (`EndRaceDialect::Touch`):** the `EndRace` tree has `RestartRaceTouchButton` and
    `QuitTouchButton` only (`docs/ui/endrace-2048.md`); SAVE REPLAY is a third box beside them
    in the same `Blue2048` idiom.
  - **Pure and Omega: not investigated.** Both carry `endrace_style: None`
    (`crates/pure/src/lib.rs`, whose comment calls it "a gap, not a measurement that Pure ships
    no such screen"; `crates/omega/src/frontend.rs`; `docs/formats/omega-status.md` records that
    the end screens "still dispatch `title == HD` alone"). Where a title draws no end-of-race
    screen of the three dialects, the session shows a **SAVE THIS REPLAY? YES / NO** prompt
    when a fork ends. Chosen.
  The layouts of HD and 2048 are chosen, not read off an original (neither has the row);
  stage 7 shows a frame of each. The string is a new `OAG_SAVE_REPLAY` id in `english.toml`
  (`just check-strings`) with French under `[untranslated]`, and every new row has pointer
  support in the same change.
- **A build mismatch and a take-over.** A replay that desyncs at 1:30 can still be taken over
  at 0:40: the state there is exactly the recorded one. Nothing past the last agreeing hash can.
- **Multi-seat.** The rule, for a later stage: each pad that presses TAKE OVER takes the seat of
  the same index in `Header::slots` before the countdown ends. A recorded human seat nobody
  took is **handed to the opponent driver from the fork tick** (a `controller` entry in `EVNT`,
  the driver located on the line as `Race::locate_player_driver` does for the Autopilot),
  because a recorded stream is right only for the state it was recorded in and the field has
  already moved. That the driver's state is all hashed or snapshotted is to be proved by the
  fork verifying. **v1 offers take-over on single-seat replays only.**
- **What the hand-off looks like.** The pad takes the seat **at the tick, exactly**: the
  recorded stream stops at `tick` and the first tick the pad flies is `tick` (the fork's
  `tick` and the tick TAKE OVER was pressed on are the same), as the maintainer set it. The
  craft is moving at speed, so what the player sees first is a countdown, not a cut:
  1. The replay **pauses on the chosen frame** and TAKE OVER opens a **3-2-1 countdown with
     the race frozen** (not ticking: the fork's state is the verified state at `tick`, so
     nothing is simulated while the player gets their hands on the pad).
  2. During it the screen draws **the recorded stick and throttle at `tick`** (the deflection
     the replay's input stream holds there) beside the live pad's, so a player can already
     be holding what the craft was doing when GO arrives. A replay recorded by the Autopilot
     or a pad-less seat has none and shows the live pad alone.
  3. At GO the race ticks live with the pad.

  The 3-2-1, its length (three seconds, the engine's own start countdown) and the stick
  display are chosen, not measured; the countdown is one constant.

  **This does not remove the hazard, and the measurement says how large it is.** From 60
  keyframes along each title's five-minute race, take the Autopilot off the player's craft and
  fly it with a pad that holds nothing, or only the throttle:

| Title | Nothing held: wall contact within 1 s / 2 s | Throttle held: within 1 s / 2 s | Earliest contact (ticks) |
| --- | ---: | ---: | ---: |
| Pulse | 1 / 11 of 60 | 6 / 18 | 46 |
| Pure | 2 / 13 | 2 / 16 | 46 |
| HD | 1 / 12 | 7 / 15 | 41 |
| 2048 | 0 / 7 | 2 / 27 | 57 |
| Omega | 0 / 10 | 7 / 16 | 51 |

  Between 7 and 27 of the 60 points (12 to 45 percent) put the craft into a wall inside two
  seconds, the earliest in 41 ticks (0.7 s), if the pad is not already steering. (Release,
  loaded. The points are every 300th tick from tick 120; the Autopilot is taken off at each
  and the craft flown for 600 ticks. The earlier dev-profile run, 58 points from tick 600, read
  the same: 3 to 23 within two seconds.) A frozen countdown with the recorded stick on screen
  is the answer inside the maintainer's rule, and **whether it is enough is owed**: stage 7
  measures it with a pad-flown run (the stick held at the recorded deflection at GO), not with
  the Autopilot, which is neither a pad nor a physics reference.

  **A hold of at most one second (M).** The table is why: after GO the seat keeps its recorded
  input until the stick moves past a 0.1 dead zone or an airbrake is pressed, 60 ticks at most, then
  the stream stops and the pad flies the seat. The fork `tick` is the tick the pad first flies, so the
  file's inputs stay the truth, and the header keeps `offered_at` for the tick the player picked.
- **Audio and camera hand-off.** Audio: the same `Sfx::rebuild` as a seek, then the live race's
  ordinary cues. Camera: the viewer may have been on any craft or a cinematic view; the live
  race starts on the taken seat's chase camera (the one a new race opens on), cold-started by
  the 60-tick pre-roll above. The HUD is the race's own and is built from state.

### 10. A staged plan

Each stage is a lane, one claim, and its acceptance evidence. Order: 1 and 2 first (they can run
in parallel), then 3 and 5, then 4, 6 and 7.

1. **`shared-track-half`.** *Claim:* a snapshot is the mutable half only. Move the constant half
   of `RaceSim` (collision, spline, racing line, ai order, routes, pads, weapon tables, speed
   and hull plans, the title laws) behind an `Arc`. *Evidence:* no golden hash moves (the
   determinism job's constants untouched); `tests::reconcile` and the `quiet_replay_ground_truth`
   tests pass; the sweep reports `snapshot_sim` at or under 40 KiB and 10 us on all five titles;
   the `RaceSim` text after a restore equals the original's. A simulation-crate change, so the
   full gate.
2. **`replay-header-v1`.** *Claim:* a file rebuilds its race and names its schema. `HEAD`
   `toml-schema`, `schemas/v1/replay-header.tosd` and its drift test, `[setup]`, options read
   back, the `EVNT` chunk with `pilot_assist` and `give`, the `[fork]` table (read, not yet
   written). *Evidence:* a tick-zero hash that agrees on every title in every mode the lane
   claims; an old ghost file still loads; a header of another kind is refused by name.
3. **`timeline-core`.** *Claim:* a seek reproduces the original run on every title. `Timeline`
   with the probe, 60-tick keyframes, the ceiling and thinning, the opening pass on a worker
   thread with `Verifier`, the desync state. *Evidence:* the sweep's seek table as a ground-truth
   test (hash at 40 targets and the `RaceSim` text at 3, five titles); the live race's hash and
   text unchanged by a visit; the open pass time per title on this desktop; a split-screen
   recording reproduces.
4. **`pose-track`.** *Claim:* a full grid's pose track costs the bytes measured here and loses no
   more than the stated error. The `POSE` chunk in `oag-replay`, recorded for every craft.
   *Evidence:* bytes a craft a second and the error percentiles from a real race on five
   titles, against this ADR's table; a file with the chunk read by a build that does not know it.
5. **`replay-viewer`.** *Claim:* a player can find, open, watch and scrub a recorded race, and
   sees a build mismatch. The REPLAYS page, the `auto/` ring and its budget, WATCH, KEEP,
   DELETE, the end-of-race WATCH REPLAY row, the mismatch banner and POSE VIEW, audio rebuild
   (`Sfx::rebuild`), the 60-tick pre-roll. *Evidence:* frames of the page, a playing replay and
   a seek on each title (read with the Read tool, more than one frame each); the WAV comparison
   of decision 6; a replay with a deliberately perturbed input shows the banner and then POSE
   VIEW; the ring deletes the oldest at 20 files and at 32 MiB.
6. **`photo-rewind`.** *Claim:* photo mode can rewind and shoots motion blur and accumulated
   depth of field. Photo mode over a probe, the scrub control, the 15-tick seek and sub-tick
   frames. *Evidence:* `a_race_through_photo_mode_hashes_the_same_at_every_tick_as_one_without`
   extended to rewind; shots of a track blur, a ship blur and a depth of field, each next to
   the same shot unblurred; the measured time of a 120-frame shot.
7. **`take-over`.** *Claim:* a player can take the pad from any verified moment of a replay and
   save the result without it ever counting as a record. The countdown and stick display, the fork, the
   `[fork]` table, `SaveReplay` per title, `taken_over`. *Evidence:* a saved fork verifies on this
   build; after a fork `records.toml` and `ghosts/` are byte-identical to before (a test);
   a frame of the SAVE REPLAY row on each of the five titles' end screens; the hand-off table
   re-run with a pad-flown run that holds the recorded stick at GO (contacts within two seconds
   of GO), which says whether one second is enough.

Not staged: modded-race replays, network replays from `oag-server`, multi-seat take-over,
the Steam Deck measurement (owed, decision 3).

## Alternatives considered

**Keyframes in the file.** Rejected **(M)**: `World`'s layout changes with commits, so a
keyframe is valid on one build only, which is the objection ADR-0055 made to a whole-world
snapshot. Inputs and hashes stay the file; keyframes are a cache built on load.

**Clone the whole `RaceSim` for every keyframe.** What `snapshot_sim` does today. Rejected by
the table in decision 2: 390 MiB to 1.5 GiB for five minutes at a keyframe a second.

**Resimulate from tick 0 on every seek.** No keyframes at all: 3.1 to 4.9 s for a late tick,
dev, loaded. Rejected for scrubbing; kept as the opening pass.

**Keyframes every 300 ticks (five seconds).** Measured: a median seek of 6 to 36 ms and a worst
of 14 to 95 ms (release, loaded), over a frame for the heavier titles. A keyframe a second costs
about 7 to 8 MiB more than one every five seconds (8.7 to 10.2 MiB against 1.7 to 2.1 MiB for
five minutes) and keeps a seek to a fraction of a frame on four titles and about a frame and a
half at its worst on the fifth.

**A pose track at 60 Hz.** 484 to 500 bytes a craft a second, 1.2 MB for five minutes of eight
craft, against 0.4 MB at 10 Hz refined. The refined 10 Hz track bounds its error instead.

**A pose track only, no inputs, for replays.** Rejected in ADR-0055 and still: a take-over needs
state, and state needs inputs.

**Promote the probe to the live race at a take-over.** The probe is a viewer's instance with no
recorder, HUD, results capture or end-of-race flow; building the ordinary race and moving it to
the tick keeps one path for how a race is driven and ended **(M)**.

**A hard cut at the take-over tick, with no countdown.** Measured above: 12 to 45 percent of the
points hit a wall inside two seconds with nothing held or only the throttle.

**A two-second hold.** Rejected **(M)** for at most one second, ended by the stick or an airbrake (decision 9).

**Auto-save every taken-over run.** Rejected **(M)**: a fork is saved from SAVE REPLAY only.

**Refuse a build-mismatched replay outright.** Rejected: the verified prefix is exact and a
pose track can carry the rest; a player should see the race, labelled.

## Consequences

**Good.**

- One mechanism, tested once, serves photo rewind, the replay viewer and the take-over. Each
  stage adds a screen, not a second way to rebuild a tick.
- The cost of a keyframe falls from megabytes to about 30 KiB, which also helps network
  reconciliation, whose `snapshot_sim` was priced at 63 us and 1.7 MiB
  (`docs/networking/README.md`, "Reconciliation cost").
- A mismatched replay is still a watchable race, honestly labelled, and a take-over cannot start
  from a state the build does not reproduce.
- Golden hashes are untouched: the timeline, the keyframes, the recorder and the pose track sit
  beside the simulation, and sharing the constant half changes no hashed byte.

**Bad.**

- **Stage 1 edits a simulation crate** and touches every place that reads the track through
  `RaceSim`. It is the risk of the plan, and why it goes first and alone.
- A second `Race` (the probe) costs memory and, from a file, a first tick of 0.5 to 2.1 s to
  learn the speed plans, unless the plans are carried in the shared half.
- A pose track is a second copy of what the inputs already imply, 0.4 MB for five minutes of
  eight craft, written for the case that the simulation changes. On a quiet build it is never
  read.
- Every race writes a file. The ring bounds it at 32 MiB, but a player who does not want it
  must find `[replay] record`.
- The countdown makes a take-over slower than a plain cut, and does not by itself remove the
  hand-off hazard (decision 9). The 3-2-1 is chosen, not measured, and is one constant.
- A view rebuilt at a seek or take-over is not the view the original run had: the particles and
  shake draw from fresh view RNGs. The race is identical; the sparks are not.
- **Not measured:** the Steam Deck (owed); a race with a respawn in the pose track (the sweep's
  Autopilot races have none, so the refinement's handling of a teleport is argued, not shown:
  a respawn is a segment no interpolation fits, and refinement inserts keys until it does);
  whether a split-screen recording reproduces; the cost of drawing a motion blur shot on a GPU;
  whether a fresh `Race` derives `capture_results` and `wreck_ended_tick` at a take-over after
  a finish (the plan refuses those moments instead).

**Revisit if:** the Deck's tick is slower than 280 us (decision 3), or the 16 MiB ceiling
thins a race a player scrubs often, or a network replay from `oag-server` is wanted, or a
modded race should record.
