# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); this file
covers what a fresh reader would otherwise have to rediscover.

Written 2026-07-26. The previous pass covered the interactive orbit camera and
the `vex::textures` PS2 fix. This pass is much larger: it opened M4, added five
crates, decoded two formats, and recovered the control force law that M4 was
actually blocked on.

## Read this first: the disc images are in place

`data/images/` now holds all three, copied from `~/Downloads` this session:

| File | SHA-256 matches [`source-images.md`](docs/reverse-engineering/source-images.md) |
| --- | --- |
| `pulse-psp-usa.chd` | yes |
| `pulse-ps2-eu.chd` | yes |
| `pure-psp-usa.chd` | yes |

All three hashes were recomputed and match the recorded values, so **every
finding in `docs/` is reproducible against exactly this data**, and
`just test-data` works. Earlier sessions had them and a later one did not, which
cost that session the ability to verify anything; check they are still there
before concluding a ground-truth test "cannot run". The originals in
`~/Downloads` were copied, not moved, and are untouched.

## In flight

Nothing blocked. `just` passes: fmt-check, clippy `-D warnings`, **546 tests**
(306 at the start of this session), check-docs. `just test-data` runs **568, all
passing**, which is those plus the 22 `#[ignore]`d ground-truth tests, exercised
against both Pulse discs and Pure. `just audit-leakage` is clean.

M4 is open and partly built. What exists now that did not this morning:

| Crate | What it is |
| --- | --- |
| `oag-physics` | Ship dynamics and collision queries. Depends on `oag-core` **only**. |
| `oag-gameplay` | `World`, `Ship`, `InputSnapshot`, spline spawning, and the two bridges from the on-disc schemas onto the simulation's types. |
| `oag-render` | The wgpu renderer, extracted out of `oag-view` so `oag-game` can use it. Owns no window. |
| `oag-input` | Devices onto the abstract button layer and the input snapshot. |

Plus `oag_formats::collision` and `oag_formats::handling`, and
[`docs/ghidra/functions/psp-pulse/engine.md`](docs/ghidra/functions/psp-pulse/engine.md),
which is the single most load-bearing new document in the tree.

Two modules **moved**, and the reason is worth keeping: `input.rs` went from
`oag-game` to `oag-gameplay` because the simulation owns the snapshot type, and
`keys.rs` went to `oag-input` because a device mapping is not a game concern.
Both are re-exported from `oag_game` so the front end's call sites read as they
did.

## There is a ship on a track now, and it cannot finish a lap

```sh
just play --race                       # arrows steer, X/Return thrust, Q/E airbrakes
just play --race --art --screenshot /tmp/race.png --ticks 45 --hold cross
just play --race --dry-run             # report what came off the disc, then exit
```

`oag-game --race` loads a track and a ship off the disc, spawns the ship on the
racing line, steps the simulation at a fixed 60 Hz from the keyboard and draws it
from the chase camera the ship's own data describes. `--art` draws the track's art
meshes instead of the diagnostic ribbon; the ship sits in the same place in both,
which demonstrates spline, collision and render geometry share one world space.

**What works.** The ship spawns the right way up on the racing line, the probes
find the collision soup, and thrust drives it forward along its own forward axis.
Over 120 ticks it is grounded on 83, peaks at 79.9 speed, and stays within 18.7 of
the spline against a 66.4 envelope. All 203 sampled downward probes along the
whole spline find geometry.

**What does not.** At about 100 ticks the ship loses its last probe contact,
pitches, is thrown clear, and is outside the envelope by tick 166. This is the
force law, not the composition, and three diagnostics say so: it reproduces on a
**flat infinite floor** with no track, spline or camera involved; it is **stable at
1/240 s**; and it is **stable at 60 Hz given a textbook box inertia** from
`<Misc>`. That is the `h <= c/k` arithmetic below, showing up exactly where it
predicted. **Nothing was tuned to hide it**, and
`the_ship_does_not_stay_on_the_track_yet` records it as a test whose failure
message says to delete it.

Three things it found that live in crates the race work did not own, all still
open:

1. **`ChaseParams::pos_length` is stored negative** in every
   `<ExternalCameraFar/Close>` on the disc, while `oag_render::camera::chase`
   documents it as a positive distance behind and negates it again - which puts
   the eye in front of the ship. `race.rs` negates at the adapter boundary,
   confidence 80. The convention probably belongs in `oag-render`. This is exactly
   the "cheap to check once there is a ship to look at" test that module wrote
   down for itself.
2. **`hover::target_height` exceeds `<Antigrav ride_height>`, which is also the
   probe cast length.** So there is no height at which the spring rests *and* the
   probes still see ground, and the oscillation is structural under the current
   reading. The culprit is the additive `antigrav_height_adjust`, which
   `oag_physics::hover` already flags as a confidence-50 guess with nothing found
   that writes it. Zeroing it makes target and reach equal. One line, but it is a
   physics decision and wants evidence, not a patch.
3. **Single-probe contact is the norm**: both probes touch on only 6 of 120 ticks,
   one on 83. An off-centre force every tick is a pitch torque every tick, which
   the missing inertia tensor (`Body::inertia` is still `(1,1,1)`) then amplifies.

Findings 2 and 3 together are probably the whole of why it cannot fly a lap, and
both are cheap to test. Start there before touching the integrator.

## The engine force law is recovered, and it corrected four things

M4 was blocked on something nobody had written down:
[`docs/physics/README.md`](docs/physics/README.md) specifies the air cushion, the
airbrakes, grip, drag and magstrips in detail, and says **nothing** about how
`<Engine>`, `<Brakes>`, `<Turning>` and `<pitch>` become force. A ship built from
that page hovers and slides but cannot drive.

That is now recovered from `BOOT.BIN` into
[engine.md](docs/ghidra/functions/psp-pulse/engine.md): the craft update frame
(`Ship_UpdateCraft`, `0x08849618`), every control term, the passive terms, and
**all 32 handling parameters placed** with the block's byte accounting closing
exactly - `6+3+5+3+7+4+4 = 32` floats = `0x80` = the observed per-class stride,
no gap and no overlap. Read from the XML loader itself, so name-to-offset is a
direct read rather than inference. 26 renames, nothing below confidence 70.

Four of its findings corrected work that had already been written:

1. **Four parameters are pre-scaled by the loader** (`Engine.amount` x0.001,
   `Brakes.amount` x-0.01, `Airbrake.amount` and `slidegrip` x1e-4). This answers
   the Units question [handling-stats.md](docs/formats/handling-stats.md) could
   not settle from the XML alone. The scaling is applied in
   `oag_gameplay::handling`, at the same point in the pipeline the original does
   it; `oag_formats::handling` deliberately returns the document's raw values and
   `oag_physics::Handling` holds the scaled form. **Applying it twice is the most
   likely bug in this area and it would be silent** - the ship would just be
   sluggish. `oag_gameplay::handling::SCALED_FIELDS` exists so the set is
   greppable, with a test that it has not grown.
2. **`Engine.gain`/`falloff` are dead.** The throttle state is ramped, stored,
   then overwritten with the raw input two instructions later. Verified in
   disassembly, not in the decompiler. `oag-physics` had faithfully implemented
   that ramp, in good faith, because the page said the idiom was documented.
3. **`ride_height` IS in the force law**, contradicting that page's "it never
   appears": it reaches the hover spring's target through `craft+0x2f0`.
4. **Groundedness is one frame stale for the control terms but not for grip.**
   Hover is step 8 of 15 and clears the contact flag on entry, so engine, brakes,
   pitch, drag and gravity read *last* frame's value while lateral grip and the
   weathervane read this frame's. The lag was known for the hover spring; it
   actually splits the force law in two. **A reimplementation that resolves
   contacts first diverges on every takeoff and landing frame.**

It also found real angular damping at `(-pitch_damping, -5, -2)`, which retires
an earlier "there is no angular damping at all" finding - that was true of the
page, not of the game.

## The best method lesson this session: two contradictions, and what they turned out to be

`docs/physics/README.md` recorded the surface-alignment torque as
`-400 * cross(up, avgNormal)` **and**, in the same sentence, as one "which levels
roll and yaw". Those cannot both hold. For unit `up`,
`(up x n) x up = n - up(n.up)`, the component of `n` perpendicular to `up`, so
`+k * cross(up, n)` aligns and `-k` diverges - and no magnitude, inertia tensor or
lever arm can change that. Simulation agreed: the literal sign tumbles a ship in
under a second.

**The interesting part is what happened next.** The weathervane torque recovered
from `engine.md` has the identical shape - `cross(forward, velocity) * -0.1`,
described two lines later as "what turns the nose toward the direction of
travel". Two independent transcription errors of one shape is a poor explanation
where a **systematic handedness difference** is a good one, and handedness is
listed as not determined on both pages. If that is what this is, both expressions
are correct in the original's own frame, nothing is a typo, and a reimplementation
in a right-handed frame must flip every cross product inherited from this path.

**That is a falsifiable prediction: the next cross-product term recovered from
the craft path should need the same flip.** Checking it is cheap and it would
settle the handedness question as a side effect. This is the most valuable open
thread in the repository right now.

Scored 84, not 85, and the reasoning is worth copying: the arithmetic admits no
third reading, but neither leg of the rubric's 85-94 band applies, because the
decompilation as recorded is self-contradictory, no call site was re-read, and
there is no multi-file data invariant.

### A number that turns "the ship wobbles" into a measurement

Roll is a damped oscillator with stiffness `k = 400` (alignment) and damping
`c = 2.0` (`Ship_ApplyAngularDamping`). Explicit Euler is stable exactly while
`h <= c/k = 0.005`, and the specified sub-step is `h = 1/180 = 0.00556` - about
**11 % too large**. So a ship sitting still on flat ground with a small roll grows
its oscillation by roughly a third per period and tumbles in a few seconds, which
`oag-physics` reproduces.

It is independent of the inertia tensor and of whether the angular accumulators
hold torque or angular acceleration, so neither open question tunes it away. One
of three things is true, and a trace distinguishes them in minutes: a magnitude is
wrong, the sub-step count is not three, or the original is marginally unstable in
that state and gets away with it because a racing ship is never in it. **This is
the first concrete thing M3 should measure.**

## Two formats decoded, both validated on both discs

**Collision geometry** ([collision.md](docs/formats/collision.md)) is an indexed
triangle soup in `.vex` nodes under five collision class IDs, separate from the
render mesh. The premise that it lives in the track `.vex` was unverified when the
work started and is now confirmed two ways: the class table, and the decoder used
as a detector firing on exactly those five IDs and no others across ~132,000
nodes. Validated over **319 nodes, 18,745 objects, 602,086 vertices, 594,615
triangles**, no exceptions. New findings:

- The "unknown `u16`" at chunk `+0x04` **is the element stride** (56,235/56,235).
- Chunk 3 is per-vertex, as predicted at confidence 88 (18,745/18,745).
- Every object stores exactly 3 chunks in order **1, 3, 2** - scalars before
  indices, which keeps the `f32` array 4-byte aligned when an odd triangle count
  ends the index array on a half-word.
- **All 602,086 vertex scalars are exactly `1.0`.** The field is authored and
  unused, so "what does the per-vertex scalar mean" is **not answerable from the
  assets** - only its consumer can say. The roadmap's open question is re-scoped.
- **Cage Collision is not dead content, it is the other SKU**: 6 nodes on PS2, 0
  on PSP.
- **Vertices are already world space** (identity transform, depth 1, 319/319). A
  consumer that composes the transform chain moves them twice.
- **The same format is in Wipeout Pure**, class IDs `0x36b`/`0x36c`/`0x37f`, 48/48
  closing exactly. Which is which is *not* determined (confidence 45), so nothing
  was named.
- **A live contradiction**: world extents reach 1,335.8 (PSP) and 1,554.1 (PS2),
  but the documented sweep-and-prune packing `((int)coord + 0x400) * 2` covers only
  about +/-1024. A straight transcription would silently drop geometry at the far
  end of a large track.

**Handling stats** are parsed and validated: 8 teams x 4 speed classes on **both**
discs, 174 attributes per file, every one required - a missing attribute is a
typed error, never a zero, because a `mass` that quietly arrived as `0.0` reads as
a tuning problem and gets looked for in the wrong place for a long time. `"nan"`
and `"inf"` are rejected too, since Rust's `f32` parser accepts them.

### Where the two readings disagreed, and which won

`engine.md` and `docs/physics/README.md` overlap and conflict in five places.
`engine.md` won each time, because it was read from the loader and the craft path
with addresses rather than summarised. Recorded so nobody re-litigates it:

| Question | Taken from | Note |
| --- | --- | --- |
| Bank-to-yaw: `+30 * right.y`, or that times `(1 - magLockBlend)` | `engine.md` | |
| Which speed the airbrake terms use | `engine.md` | The craft caches `abs(dot(v, forward))`; the brake uses `length(velocity)`, per its own section |
| Whether `ride_height` is in the force law | `engine.md` | It is. Ships now settle at a data-set height rather than an emergent one, which is the largest behavioural change of the session |
| Whether the engine ramp exists | `engine.md` | It does not |
| Two masses: spring reads `Physical::mass`, gravity reads `Body::mass` | both | Equal in practice; **keeping them equal is the integration layer's job** and nothing does it for you |

One interface note, because it looks like a 100x bug and is not:
`oag_physics::controls` works on the original's `0..=100` control scale, but
`ShipControls` remains normalised (`-1..=1` axes, `0..=1` triggers) and the
multiply by `CONTROL_RANGE` happens inside `oag-physics`. Callers must **not**
pre-scale.

## Traps and near-misses from this session

- **The PS2 `handlingstats.xml` files have a malformed prolog**:
  `<?xml version="1.0" encoding=utf-81"?>` - unquoted value, stray digit, **odd
  number of quotes**. Tracking quotes across the closing `>` put the scanner
  permanently out of phase and returned an *empty document with no error*. Two-line
  fix (end a processing instruction at its own `?>`), eight otherwise unreadable
  files. Wrong, silent, and it would have looked like a physics bug.
- **The collision walk came up 10 bytes short**, and the answer was 16-byte node
  padding - **not** the `u16`-chunk-count hypothesis the error message itself
  suggested. The arithmetic invariant beat a careful re-read again, and it beat a
  plausible hypothesis that the code was already nudging toward.
- **`docs/formats/handling-stats.md` was reproducing two real tuning values** in
  its Units note, an ADR-0006 violation that predated this session. Rewritten to
  describe the ratio. Worth a periodic sweep; the rule is easy to breach while
  making a legitimate point.
- **Do not run `just` or `cargo fmt --all` while background agents are editing.**
  A gate run taken mid-edit reported a physics test failure that no longer
  existed, and `cargo fmt --all` rewrites files an agent holds open, breaking its
  next edit. Scope to `-p <crate>` while work is in flight.
- **`ls` and `rg` intermittently fail on relative paths in this shell**
  (`permission denied`, `No such file or directory`) on directories that plainly
  exist; absolute paths work. Suspected `ecryptfs`. Use absolute paths in scripts.
- `data/` is on `ecryptfs`, so **`cp --reflink` does not work** - copying the
  three images is a real 4 GB copy, and there is no cheap clone.

### How to verify a renderer refactor with no "before" to compare against

This is reusable and it closed the one gap the `oag-render` extraction otherwise
had. `git worktree add <tmp> HEAD` gives a clean pre-change checkout; symlink
`data/images` into it (note the worktree already contains a tracked
`data/README.md`, so link `data/images`, not `data`), build `oag-view --release`
there, and capture screenshots. Then capture the same three from the working tree
and `cmp` them.

Result this session: `--mesh` on the Feisar ship, `--track` on `01_Track` and a
texture from `FE.wad` all came out **byte-identical** across the extraction. That
is what makes the refactor verified rather than only "behaviour-preserving by
construction".

## Verification status: what to lean on

**Verified against real data this session:** the collision layout (319/319 closure
on both discs), the handling schema (8 teams x 4 classes on both discs), the
`oag-render` extraction (byte-identical screenshots), and the three image hashes.

**Verified earlier and still good:** the WAD name hash and size-field ordering,
LZSS, texture decoding, `.vex` geometry and textures, the `section` count, the
whole `WO Track` layout, the junction slot ordering, the import stub names, the
PSP image base.

**Static reading, never executed:** *all* of the physics. `oag-physics` transcribes
`docs/physics/README.md` and `engine.md`, asserts structure, and **tunes nothing**.
The one exception is the two cross-product signs, settled by arithmetic - and even
there only the signs, not the magnitudes. Nothing in the force law has been
compared against the original running, which is what M3 is for. Do not raise a
confidence score on the strength of the simulation agreeing with itself; that was
exactly the RNG mistake this file already records.

## What is deliberately not done

| Not done | Why |
| --- | --- |
| Engine/brake/steer/pitch **verification** | The force law is recovered and implemented; nothing has been traced. M3. |
| The magstrip magnetic hold | Its force law was not decoded. Consequence: **an inverted ship falls off.** Nothing recovered pulls a ship toward a ceiling - gravity writes world `.y` only and the hover spring always pushes *away* from the surface it found. Expected, not a bug. |
| The four-corner hover variant | The selector is now known (`DAT_08ab07e3 == 0 && DAT_08b31048 == 6`, which also disables the brakes and swaps in an auto-speed law). The variant itself is not implemented. |
| Sweep and prune | A linear scan behind an AABB reject. Correct, and not the bottleneck. See the +/-1024 contradiction above before implementing it. |
| Quadratic drag and hover downforce magnitudes | Implemented as shape with the coefficient at `0.0`, because the pages record that the terms exist and not how large they are. |
| **The ship is rotationally unstable at rest** | Not a bug to patch. It is the `h <= c/k` arithmetic above showing through: a stationary ship with a small roll grows its oscillation and tumbles in a few seconds. Fixing it by changing a magnitude would destroy the measurement, which is currently M3's sharpest question. |
| **Quadratic drag adds energy while reversing** | `velocity * forwardSpeed * k` is dissipative only while `forwardSpeed > 0`. No prose contradicts the expression, unlike the cross-product terms, so it is transcribed literally and no test lets a ship reverse. A real hazard the moment anything can. |
| Sideshift | Recovered as a mechanism, but **which buttons fire it is not known**, so `oag_gameplay::controls` returns `Sideshift::None` always and a test pins the gap. |
| The game's own font | Unchanged: `.fnt` metrics decode, the atlas's pixel layout does not. |
| Front-end widget offsets, audio, batch list B, transparency sorting, mip levels, `section` payloads, the 29 unresolved import stubs | Unchanged from the previous handover. |

## Method that is not obvious

### Symbol names are reproducible

```sh
just apply-names        # 426 symbols into whichever program the bridge has open
```

120 come from [`names.tsv`](docs/ghidra/functions/psp-pulse/names.tsv) (up from 94)
and 306 are import stubs re-derived from the binary.
`scripts/apply-ghidra-names.py` refuses any row whose address and name are not
both still on its evidence page, so ADR-0005 is enforced rather than trusted.

**Two traps in the Ghidra bridge.** `dry_run` is *not* honoured by the rename
endpoints: it reports what it would do and does it anyway. And it rejects global
names lacking a Hungarian type prefix, which is not this project's convention, so
data addresses go through `create_label`.

`just check-docs` validates that links **resolve**, not that a page is
**reachable**. `engine.md` was an orphan when it landed. If you add a page, add
its row to the relevant `README.md` too.

### Naming imports is a check, not a guess

A PSP import NID is the first four bytes of `SHA-1(name)`, little-endian. Hash
candidates and keep the matches; a wrong name cannot produce the right NID. 306 of
335 resolve. See [imports.md](docs/ghidra/functions/psp-pulse/imports.md).

### Finding entries by name

WADs store only a CRC-32 of each name and most names are built at runtime, so
`scripts/mine-names.py` rebuilds the candidate list: `just mine-names <image>`,
292 of 1,142 in `Data.wad`. The step that matters is that **track directories are
numbered** (`01_Track`), so `Data\Plugins\PI001\Definition.xml`'s `location`
attributes plus the `%s\%strack%s.vex` template are what make track files
reachable at all.

### Validating a decoder

Every format decoded here had a self-check available, and using it has now caught
real errors nine times and confirmed a format outright three more. **Look for the
arithmetic that must hold before writing the parser, then put it in a test rather
than in prose.**

- **WAD**: the offset chain. 193/193 against 2/193 resolved a transposition.
- **`.vex` geometry**: each batch's declared bounding box, over 342,115 vertices.
- **`.vex` node tree**: child counts sum to one less than the node count. Exact as
  a `u16`; 111 million as a `u32`, which settled the width.
- **Embedded textures**: sizes sum to the declared block length.
- **Track**: exactly 64 `section` nodes, matching the 64-bit PVS mask.
- **`WO Track`**: the payload closes on 40 of 40 files.
- **LZSS**: where the *reader* stops - every one of 6,053 streams consumed to
  within one or two bytes of its end.
- **PMF**: header arithmetic, zero stray bytes, access-unit counts against
  duration.
- **Collision**: the walk closes on **319 of 319** nodes once 16-byte node padding
  is accounted for, which also pinned the chunk count at 32 bits and turned the
  unknown `u16` into the stride.
- **Handling stats**: completeness. 8 teams x 4 classes x every attribute, with a
  test that removes each of 174 attributes in turn and requires every removal to
  error.
- **The handling parameter block**: 32 floats = `0x80` = the observed per-class
  stride, closing with no gap or overlap, which is what placed the 13 previously
  unknown offsets.

### Running the game headlessly

`PPSSPPHeadless` boots straight from a raw ISO with no display:

```sh
chdman extractdvd -i data/images/pulse-psp-usa.chd -o /tmp/pulse-psp-usa.iso
PPSSPPHeadless /tmp/pulse-psp-usa.iso -l --timeout=8 > log.txt
```

`-l` logs the full HLE trace. `--debugger=PORT` opens a websocket debugger that
breaks at start - not used yet, and it is the way in for M3's scripted input.

### Ghidra

`BOOT.BIN` must be loaded with the **Allegrex** processor module, not stock
`MIPS:LE:32`, or the decompiler builds confident fictional C from VFPU
instructions. `just build-allegrex`, needs JDK 21.

Image base `0x08804000`, confirmed against PPSSPP's own loader log and against
`BOOT.BIN`'s ELF program headers. The Ghidra project still lives in the repository
root (`OpenAntiGrav.gpr`) rather than `data/ghidra/`; moving it is safe once
closed.

## Where I would go next

1. **Make the ship fly a lap, and try the two cheap explanations first.** The
   hover target exceeding the probe reach, and the missing inertia tensor, are
   findings 2 and 3 above; both are small, both are testable against
   `just play --race`, and neither needs an emulator. Only if those do not settle
   it is the integrator itself the suspect.
2. **M3, and start with the two numbers this session produced.** The
   integrator-stability arithmetic above (`h <= 0.005` vs `1/180`) and the
   handedness prediction are both falsifiable in minutes with a trace, and both
   currently cap confidence across the whole physics tree.
   `PPSSPPHeadless --debugger=PORT` is the way in.
3. **Test the handedness prediction.** Recover one more cross-product term from
   the craft path. If it needs the same flip, the systematic-handedness
   explanation is confirmed and the coordinate-conventions open question falls out
   with it. Cheapest high-value RE task available.
4. **Find the call site of `Ship_UpdateCraft`.** It is vtable-dispatched, and its
   absence caps that page's confidence at 82. It would also settle the frame
   ordering that item 4 of the corrections above depends on.
5. **Decode PS2 vertex type `0x1b9`**, still M1's exit criterion. `--track` now
   renders on both discs; `--mesh` gets past textures on PS2 and fails on this.
   Comparing the PSP and PS2 batch headers of the same ship is the obvious first
   move and needs no emulator.
6. **The `.fnt` atlas layout.** Unchanged and still the last thing between the
   front end and Pulse's own type. Four more transform families were ruled out two
   sessions ago; see [fnt.md](docs/formats/fnt.md) for what has been tried.
7. **LZSS is still tested only by its own inverse.** The stream-consumption check
   is the only independent oracle and it constrains the layout without pinning it.
   Untouched again this session.
8. **Check whether the determinism gate actually covers the new simulation.**
   `crates/core/tests/determinism.rs` compares a hash against a committed
   reference, and `CLAUDE.md` calls that gate load-bearing. It was written before
   `oag-physics` and `oag-gameplay` existed, and nobody checked this session
   whether any of their state feeds it. If it does not, the two crates that most
   need the guarantee are outside the gate. Worth an hour, and note the standing
   rule: **when that test fails, find the bug - never update the reference
   constants to make it pass.**
