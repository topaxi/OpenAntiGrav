# ADR-0055: Replays are inputs and hashes, and a ghost is drawn from poses

## Status

Accepted. Opens roadmap M7 (Replay) and creates `oag-replay`, which
[workspace-layout.md](../workspace-layout.md) reserved for it. Builds on
[ADR-0002](0002-determinism-model.md) (determinism),
[ADR-0003](0003-no-ecs.md) (the world as plain data) and
[ADR-0052](0052-world-and-race-tick-widen-to-n-players.md) (one input snapshot
per grid slot).

## Context

The first thing that needs a replay is a **ghost**: in Time Trial and Speed
Lap the player races a translucent copy of their own best lap. After that
comes watching a whole race back. Both need a file format, and the maintainer
set the terms for it directly: **the format does not have to be compatible
with any original's.** Pulse is not the authority here. The format should
serve every title this engine races (Pulse, Pure, HD, 2048), and a ghost
should work at least as well as the originals' do.

What the originals do was read for reference only, on
[ghost.md](../../ghidra/functions/psp-pulse-usa/ghost.md) (a static read of
Pulse's PSP executable, confidence 80-90 per row). Pulse:

- **records poses, not inputs**: the craft's matrix, sampled at 20 Hz into
  the engine's own `AnimTransform` keyframe format, up to 90 seconds a lap;
- keeps **the best lap**, restarting its recorder and its ghost's clock at
  the line every lap, and swaps a lap that beats the ghost in as the new
  ghost at once. The end-of-race menu's `NEW GHOST RECORD` is the run's best
  lap in Time Trial as well
  ([endrace-screens.md](../../formats/endrace-screens.md));
- keys its save file (`SAVE.GHO`) on **track and class only**, so Time Trial,
  Speed Lap and Free Play share one ghost, and stores the ghost's own team
  for the hull;
- draws it as the team's ordinary ship model in three passes: a depth lay, a
  flat cross-fade of the unlit hull whose weight rises with distance from the
  player (nothing inside 5 units), and a screen-space static texture stamped
  into the bloom's glow mask.

Four facts about this project shaped the decision:

1. **The simulation is deterministic by construction**
   ([determinism.md](../determinism.md)). The same build, handed the same
   inputs, reaches the same state on every platform. A replay stored as
   inputs is therefore complete: nothing else needs storing.
2. **The simulation changes all the time.** The force law, the lap gate, the
   AI and the pad broadphase have each moved in the last month, as reading
   the original sharpened. A replay stored as inputs alone replays a
   *different run* on the next build that changes any of them.
3. **`oag_core::StateHasher` already reduces a race to 64 bits**
   (`RaceSim::state_hash`), and golden tests are built on it.
4. **`World::race` and `PlayerInputs` are already per slot** (ADR-0052). A
   format that records slot 0 alone would be single-player-shaped for no
   reason.

## Decision

### A replay is the inputs, per human slot and per tick, plus periodic state hashes

A replay file (`oag_replay::Replay`) stores:

- a **header**: format version, the build that recorded it (informational),
  the key (title, track, mode, class, team), the seed, the tick rate, which
  slots were recorded, the hash interval, and a free-form `options` table the
  composition root fills with whatever else it needs to rebuild the same race
  (control scheme, AI difficulty, a lap override, the operator's autopilot);
- **one input stream per human slot**: every `InputSnapshot` exactly as
  `Race::tick` received it, all four button masks and all four axes, bit for
  bit. The edges are stored, not rebuilt from the held mask on playback,
  because a press consumed before the snapshot reached the tick is part of
  what the simulation saw. AI slots store nothing, because they reproduce from
  the seed;
- **the state hash before the first tick, then one every 60 ticks** (one a
  second). The first hash catches a replay handed to the wrong track, class or
  seed before anything is stepped. The rest catch a build that no longer
  reproduces the run, and say roughly when (see below);
- optionally, **a ghost lap** (next section).

The encoding is chunked (`OAGR`, a `u16` version, then tagged chunks). The
header is TOML text, so a person can read it when a replay will not play, and
it can grow a `#[serde(default)]` field without a version bump. The input
stream is binary and run-length coded: each run is a varint length, a one-byte
mask of which of the eight words changed, and those words. A trailing FNV-1a
checksum catches the one failure a state hash cannot: a file cut short while
it was being written. A chunk a reader does not know is skipped, so adding a
section does not break older readers.

**Sizes, measured.** A held throttle for 3,000 ticks encodes in under 40
bytes (`codec::tests`): that is the keyboard end. The analog end is measured
by `crates/game/tests/replay_ground_truth.rs`, which flies a Time Trial
through a pad whose stick moves nearly every tick. Over 7,200 ticks (two
minutes) the input stream came to **6.3 to 6.9 bytes a tick** on all four
titles: 48,818 bytes on Pulse, 45,070 on Pure, 49,594 on HD and 46,018 on
2048. That is about 16 KB for a 40-second lap. The hash track adds 8 bytes a
second. The whole file, a 2,427-tick Pulse ghost lap included, was 118 KB.
Nothing here needs general-purpose compression.

### A ghost is drawn from poses sampled while the lap was driven, never from a resimulation

A ghost lap (`oag_replay::GhostLap`) is one pose per tick of the lap's own
clock: the craft's world position and its drawn orientation, barrel roll
included, seven `f32`s. It samples every tick rather than at Pulse's
20 Hz, and stores values unquantised, so the track is exact and needs no
interpolation. That costs 84 KB for a 50-second lap, where Pulse spends about
9 bytes a key. `poses[k]` is where the craft was `k` ticks after it crossed
the line to start the lap. To draw the ghost, read the entry at the
**player's** lap clock. Both clocks start on the same edge (the forward line
crossing that `oag_race::Standing` times laps from), so the ghost sits
exactly where the recorded craft was at the same point in its lap.

**Why poses, when the inputs could resimulate the lap** (Pulse made the
same choice, for its own reasons):

- **A personal best must keep showing the lap that set it.** Fact 2 above
  means a resimulation on a later build is a *different* lap: slower,
  quicker, or into a wall. The time on the records table was set on the build
  it was driven on, and the ghost is the picture of that time. Poses make the
  ghost immune to simulation changes by construction.
- **A lap ghost starts mid-run.** In Speed Lap the best lap may be the
  eleventh. Resimulating it means replaying ten laps first, or storing a
  whole-world snapshot at the lap's start: kilobytes of engine-internal state
  whose layout changes every time a field is added to `World`.
- **It costs a second simulation in a live race.** A resimulated ghost is
  another `Race` stepping beside the player's, doubling the CPU cost of the
  tick for a hull that collides with nothing.

**The inputs still travel with the ghost.** A ghost file is a replay:
header, inputs and hashes from tick 0 to the end of the ghost lap (truncated
there, since a Speed Lap session can run on long after it), plus the pose
track. On a build that reproduces it, the lap can be re-driven and checked
(`oag_replay::Verifier`). A pose track can be regenerated from the inputs if
the pose encoding ever changes. And a later full-race viewer reads the same
file. Inputs are the truth; the poses are a cache of what the truth looked
like on the build that produced it. Neither replaces the other.

### When the simulation changes

**Validity is decided by the stored hashes, never by comparing version
strings.** A commit hash changes on a docs-only commit, and a hand-bumped
simulation version gets forgotten. The question that matters is "does this
build reproduce this run", and re-driving the run answers it:
`Verifier::run` plays the inputs and compares every stored hash. It reports
the first one that disagrees as `Desync { tick, expected, found }`. The
header's `build` field is only there so a person can see where a file came
from.

So a simulation change affects the two uses differently:

- **A ghost is kept and keeps working.** It is drawn from poses and never
  resimulated. Its inputs may no longer reproduce, and that only matters to a
  tool that tries to re-drive them.
- **A full-race replay viewer** (not built yet) resimulates, and stops at the
  first desync. It says so, rather than drawing a race that never happened.
  The ghost's hybrid scheme is the fallback it can grow into: a whole-race
  pose track for every craft is 28 bytes per craft per tick, about 1.6 MB for
  a two-minute eight-craft race. Record it, and the viewer can keep drawing
  poses past a desync, labelled as such.

### Which lap, and when it is saved

**Best lap in both modes**, following Pulse (`NEW GHOST RECORD` is the best
lap in Time Trial as well as Speed Lap). A lap ghost restarts at the line
with every lap, so it is on screen for the whole run. A total-time ghost
would be on screen for only one pass of a Time Trial.

**It improves within a run.** When a lap beats the ghost being raced, that
lap becomes the ghost from the next lap on, drawn as the player's own hull.
This is Pulse's own behaviour (its two lap buffers swap at the line; see
ghost.md, "best lap, restarting at the line").

**Saved automatically when it beats the stored file**, at the two places
`records.toml` is already written (the finish transition and leaving the
race; see [persistence.md](../persistence.md)). Pulse asks first
(`SAVE GHOST`). This engine has no end-race menu row for that yet, and a best
lap nobody saved is the worse failure. Chosen, not measured.

### Where files live

Beside `records.toml`, under `<config dir>/oag/ghosts/`. There is one file per
`(title, track, mode, class)`, the same key `records.toml` uses
(`oag_game::records::Key`), so a ghost and its lap time share a row. Each part
of the key becomes a filesystem-safe name segment, and the header repeats the
key verbatim. A file whose header does not match the key it was found under
is ignored.

**The key includes the mode, where Pulse's does not.** Pulse keeps one ghost
per track and class across Time Trial, Speed Lap and Free Play. This engine
keeps one per mode, so a ghost's time is always the `best_lap_ticks` of the
`records.toml` row it sits beside. With a shared ghost, a Speed Lap lap
could be raced in Time Trial under a best lap the Time Trial row never
recorded. Chosen, not measured, and a one-line change to `ghost_path` if
sharing turns out to be what players want.

The ghost's hull is the **recorded team's**, which may differ from the
player's. It is loaded alongside the grid when the two differ.

### The ghost is not in the world

The ghost is not a ninth ship, and it does not occupy a grid slot. It lives
in `oag_game::race`, beside `RaceView`, never in `RaceSim` or `World`. It
collides with nothing, and it cannot change a state hash. A race with a ghost
hashes exactly like the same race without one, tick for tick.
`crates/game/tests/replay_ground_truth.rs` asserts that on a real circuit.
Recording is optional state on the same side, off by default, so every
existing test, trace and golden hash runs the same code path as before.

## Alternatives considered

**Inputs only, resimulated for the ghost as well.** The smallest file and
one playback path. Rejected for the three reasons above. The first is
decisive: this project's simulation changes weekly, and a ghost that drives
into a wall after a physics fix is worse than none.

**Poses only, no inputs.** The simplest thing that draws a ghost. Rejected
because it gives up the one property that makes this engine's replays cheap:
a full-race replay would need a pose track for every craft plus every
projectile, pickup and effect, where inputs plus determinism need none of
that. It would also mean a separate file format to design when the viewer
lands.

**A whole-world snapshot at the ghost lap's start**, then resimulating from
it. Rejected: `World`'s layout is engine-internal and changes with nearly
every commit that touches the simulation, and the snapshot would still
resimulate differently on a changed build.

**A simulation version number in the header, compared on load.** Rejected as
the validity test (see "when the simulation changes"). It is kept as the
informational `build` field.

**Pulse's own ghost file format** (`SAVE.GHO`: a team name, a lap time,
20 Hz keys with byte quaternions and `s16` positions). Not a goal (see the
maintainer's terms above), it would tie every title to one title's layout,
and it carries no inputs, so it could never serve a full-race replay.

**A ghost as a ninth `World::ships` entry marked inactive.** Rejected:
`MAX_SHIPS` is a real grid bound. A ninth entry would change the size of the
snapshot, and so every golden hash. Every per-slot loop would also need
teaching to skip it.

## Consequences

**Good.** A ghost survives every simulation change. The file is small, and
it is self-checking: bad magic, a damaged byte, the wrong race and a
simulation that has drifted are each reported, and none of them plays back
silently. The same file serves a future full-race viewer. The format is
title-agnostic, so the composition root adds the per-title key and nothing
in `oag-replay` changes per title. Golden hashes are untouched, because the
ghost and the recorder are outside the simulation.

**Bad.**

- A ghost can show a lap the current build could not drive. That is the
  point, but it means a ghost is no evidence about today's simulation.
- The inputs in a ghost file may stop reproducing and nobody will notice,
  because nothing re-drives them in normal play. A desync is only found by a
  tool or test that runs `Verifier`.
- Pose playback is at tick resolution. A renderer that interpolates between
  ticks sees the ghost step at 60 Hz. The pose track is exact, so
  interpolating it later is additive.
- The key includes the mode, so a Speed Lap ghost is never raced in Time
  Trial, even when it is the quicker lap. Pulse shares them.
- A pose every tick is about three times the bytes of Pulse's 20 Hz
  quantised keys. It is still well under a megabyte per ghost.
- Anything that changes `RaceSim` outside `Race::tick` is invisible to the
  input stream. Today that is the `--give` capture flag, which tops up the
  pickup slot. A recording made with it is marked in `options` and will not
  reproduce. That is acceptable for a debug flag, but it is a rule to
  remember: **a new out-of-tick writer to the simulation must either become
  an input or be recorded as an option.**

**Revisit if:** the full-race viewer lands (it will want the whole-race pose
track sketched above), or network play's reconciliation wants the same input
stream on the wire, where a shared encoding would be worth a superseding ADR.
