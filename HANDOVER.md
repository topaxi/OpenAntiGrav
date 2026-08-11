# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); this file
covers what a fresh reader would otherwise have to rediscover.

**Pruned 2026-08-09**, from 1,794 lines and 267 KB. The previous prune
(2026-07-29) had shrunk an append-only session log from 4,393 lines; it grew
back, because six passes on the same subject each appended their own narrative
instead of replacing the last. Nothing is lost: `git show
73e8567:HANDOVER.md` is the pre-prune edition and `git log -p HANDOVER.md` is
every intermediate one. What changed this time, besides the length:

- **The measurement and reverse-engineering rules moved into `docs/`**, where
  they belong, and this file points at them. See
  [methodology](docs/reverse-engineering/methodology.md#rules-learned-the-expensive-way),
  [verification protocol](docs/reverse-engineering/verification-protocol.md#reading-a-capture-rules-that-each-cost-a-session),
  [rendering](docs/rendering/README.md#measuring-a-renderer-change),
  [WAD](docs/formats/wad.md#traps-when-surveying-a-disc),
  [Ghidra workflow](docs/ghidra/workflow.md#the-database-can-disagree-with-namestsv-and-the-docs-win)
  and [the PPSSPP page](docs/reverse-engineering/ppsspp-debugger.md#session-hygiene).
- **Open threads are only what is still open.** A row whose work landed is
  deleted, not annotated; its evidence is on the docs page it always cited.

**Keep it that way.** A finding belongs on a docs page with its evidence and its
confidence score, and this file points at it. Rewrite a section rather than
appending a dated pass to it.

## Read this first

- **The disc images are in place** - all six, in `data/images/`:
  `pulse-psp-usa.chd`, `pulse-psp-eu.chd`, `pulse-ps2-eu.chd`,
  `pure-psp-usa.chd`, `pure-psp-eu.chd` and `hdfury-ps3-eu.iso`. Hashes match
  [`source-images.md`](docs/reverse-engineering/source-images.md), so
  `just test-data` works. Earlier sessions have believed them lost and spent the
  session unable to verify anything - check before concluding a ground-truth
  test "cannot run".
- **`rg` and `fd` cannot tell you that.** Both respect `.gitignore` and `data/`
  is gitignored, so both return nothing with exit code 0 whether the directory
  is full or empty - no error, no warning, and it reads exactly like "there is
  nothing here". Use `--no-ignore`, or `/bin/ls` / `find` / `grep`. Note plain
  `ls` may be shell-aliased (`ls -la` failing with `ls:1: command not found:
  -la` reads like a broken argument and is not), and a spawned agent has a
  second, independent reason to see `data/` as empty: being gitignored, it does
  not travel into a worktree or a restricted sandbox.
- **Derived evidence under `data/` is not durable; the scenarios and scripts
  that regenerate it are.** A 2026-07-29 pass found three reference traces gone,
  leaving `force-balance-ground-truth.md`, `contact-response.md` and three
  citations inside `wall.rs` pointing at files that were not there. Anything
  under `data/traces/`, `data/shots/` or `data/cache/` can be absent; the
  committed `verification/scenarios/*.inputs` are what reproduce it.
- **Check `git status` before assuming the tree is clean.** A whole milestone's
  work once sat uncommitted for a day.
- **Gate status:** last measured green at **1,629 tests (2026-08-11)**, with
  `fmt`, `clippy`, `check-docs`, `check-deps` and `audit-leakage` all clean.
  Re-measure rather than trusting the number here - `git stash && just test` is
  how the drift was caught last time. **`just test-data` is not clean**: four
  `oag-trace` tests fail on an untouched tree, which has its own row under Open
  threads - do not read them as a regression from whatever you are landing. The
  disc-backed suites that gate the front end (`oag-game`, `oag-pure`,
  `oag-pulse`, `oag-title`) were 541 of 541 the same day. Separately, and still
  true, **one failure is the machine rather than the code**:
  `oag-disc::ground_truth listing_matches_chdman_and_the_reference_iso` shells
  out to `chdman extractdvd` into `/tmp/oag-ground-truth.iso`, `/tmp` here is a
  6.8 G tmpfs, and the extracted PSP ISO does not fit in what is free. It fails
  at "Extracting, 29.4% complete... Error writing to file; check disk space",
  which `nextest` reports as `chdman extractdvd failed` two frames up and reads
  like a broken invocation. Free `/tmp` or point the test elsewhere; do not go
  looking for a disc bug.
- **Rendering fidelity gaps are mostly not bugs.** Bloom, colour grading and
  every authored light class (`AmbientLight`/`DirectionalLight`/etc.) are still
  unchecked M6 roadmap items - the flat, dark look that produces is a known,
  documented gap, not something to guess-fix from a screenshot.

## Where the project stands

**M4, with a run of M5 items landed alongside it on 2026-08-10.** In the order
they were done, because each unblocked the next:

| Landed | State |
| --- | --- |
| Shield and energy | pool, maximum, contact damage, all measured against the original |
| Zone's ending | the bit nobody could find is set three states after the pool empties |
| The starting grid | order *and* geometry; the authored node is **slot 8** |
| Eight craft on the grid | placed and drawn; **nothing drives the seven opponents** |
| Weapon pads | drawn, and checked against their own trigger volumes |
| The weapon table | `WeaponStats_*.xml` decodes - the weapons are authored data |
| **Pickups** (2026-08-11) | a pad hands one out, `SQUARE` fires, `CIRCLE` absorbs; **Turbo, Shield and Rocket** have effects |
| **Projectiles** (2026-08-11) | `oag_gameplay::projectile`: rockets fly, sweep against track and hulls, and spend `blastradius`/`damage`/`blastforce` |
| **The race-level determinism hash** (2026-08-11) | `hash_world` + `Race::state_hash` close the hole the pickups opened - inventory, projectiles, pad timers and the RNG position |
| **`Mode::SingleRace`** (2026-08-11) | the fourth mode, and the only one with weapons on - pickups had nowhere else to happen |

**What blocks M5 now is the AI, and it is the only thing** - and the weapon pass
sharpened that rather than changing it. Seven craft sit on the grid and nothing
drives them, which blocks race positions, the opponent-bearing half of a single
race, the Autopilot pickup, and now **a rocket having anything to shoot at**:
`Mode::has_opponents` is unconditionally `false`, so a fired rocket's whole
target list is the track and whatever the `--opponents` verification flag
parked - `Ai_Construct` names the local player's input source the literal
`"autopilot input"`, so whoever builds the AI builds most of the autopilot too.
Nothing else in M5 depends on work that has not started.

**Everything the pickup pass left open is on
[pickups.md](docs/gameplay/pickups.md)**, and two of its rows are worth knowing
before touching anything nearby: `craft+0x1c0` is still thirteen unread bits out
of fourteen, and **no pickup-grant call site exists anywhere in the
executable** - so the grant, the weighted draw and the one-slot inventory are
this project's rather than a port, and are labelled so where they live.

The rest of this section is M4.

---

The energy-pool detail, kept because the offsets are easy to get wrong: as of
2026-08-10 it depletes. `entity+0x88` (that is `*(craft + 0x1c4) + 0x88` - **not** the
craft the trace harness breaks on), its skill-indexed `<Misc>` maximum,
`Ship_Damage` (`0x088439ac`) and the contact law `|p| * 0.05 * 0.7` are
recovered ([shield.md](docs/ghidra/functions/psp-pulse-usa/shield.md)) and
ported (`oag_physics::damage`), the pool is hashed by the determinism gate, and
the HUD's `ShieldBar` moves. **Measured against the running original** for the
pool's location, the two race-option globals, the regeneration branch and the
weapons-off halving (1.98 against a predicted 2.00), and for the coefficient
itself - `amount / |p|` is `0.035000` on 25 of 25 `Ship_Damage` calls. Zone's ending was
built on it the same day; weapon damage still needs weapons.

M4 itself: The engine/title split that ran alongside it is finished (see the next
section).

The force law, the angular-momentum model, the contact response and the
mag-lock attitude hold are all recovered at instruction level and implemented,
and **every fitted constant that once stood in for a recovered one has been
retired** - `YAW_DRIVE_CALIBRATION`, `ALIGNMENT_INERTIA` and the
`<Misc>`-derived box inertia each had a real value found for it. The evidence
lives in [`docs/physics/`](docs/physics/README.md) (the two ground-truth pages
and [`angular-velocity-column.md`](docs/physics/angular-velocity-column.md)) and
under [`docs/ghidra/functions/`](docs/ghidra/functions/README.md) - `engine.md`,
`rigid-body.md`, `contact-response.md`, `craft-update.md`.

**Pass `--script-lead 2` against everything in `data/traces/`**, and on both
sides of any new capture. `psp-trace.py --script-lead 2` cancels the emulator's
three-frame input latency for every transition after the start and, in doing so,
**never sends the script's first two states at all**. Our replay applied them,
and the resulting two-tick head start was diagnosed as a physics wedge for a
session and a half. The whole chain is measured in
[`oag-trace.md`](docs/tools/oag-trace.md#the-first-two-ticks-of-a-script-never-reach-the-emulator);
`oag-trace run --script-lead N` and `drive --script-lead N` exist to release the
same ticks on our side.

**Read lap fidelity off the *seeded* comparison** (`oag-trace run --reseed N`),
never the open-loop one, and note the roadmap's exit criterion is stated as two
decidable numbers for exactly this reason. The original integrates the frame
duration it measured
([ADR-0007](docs/architecture/adr/0007-fixed-timestep-vs-original.md)), those
durations follow host load, and **two runs of the original itself, from a start
pose pinned to 0.0000 degrees, are 100 units apart by tick 495** with `dt`
agreeing on about 1 % of ticks. A 3,146-tick single-seeded trajectory comparison
measures the emulator's scheduler, not the force law, and a byte-exact
implementation would fail it.

**The reference lap is `data/traces/talons-junction-clean-lap.csv`** (regenerate
with `just drive menu` + `just drive restart` + `just autopilot --spline ...
--laps 1`; script and trace come from the same run, which matters - a
closed-loop recording replayed open-loop drifts into walls). It is wall-free on
**96.3 %** of its ticks against the previous capture's 32.7 %, and first touches
a wall at tick 256. Two attempts at pushing that further gained nothing, which
was the agreed stopping point.

**The real finding is that the wall was never the whole story.** Inside that
capture's own clean window, single-seeded position error is already 17.5 units
by tick 180 and 44.1 by tick 240 - **47.76 max / 13.28 mean over ticks 0-255**.
So raising the wall-contact ceiling from 171 to 256 did not buy 85 more ticks of
"the physics agrees"; it exposed that **the force law, not contact geometry, is
what limits comparison length**. That is the harder problem left blocking M4's
single-seeded number, not another capture. Under `--reseed 60` this capture
reads 28.86 at tick 1,739 (bounded) and worst orientation axis 0.4858 rad at
tick 1,129 (bounded). Checked before attributing that to a regression: re-run
under today's physics, the *old* capture's worst reseeded orientation axis is
0.0541 rad against its recorded 0.0824 - a modest shift, nowhere near the new
capture's jump - so the gap is this lap holding racing speed and cornering hard
throughout rather than spending most of its length wedged against a wall.

**Do not read `grounded` as the headline it looks like.** The original's column
is `1.0` on all 3,146 ticks, so agreement means only that our ship also never
leaves the ground. A constant column cannot test whether the hover model
quantises contact the way the original does, and under `--reseed` the field is
one of the ones the seed restores anyway.

**The standing start is the best-conditioned scenario.**
`verification/scenarios/standing-start.inputs` - 300 ticks of held thrust from
the start line - reproduces *both* legs of the force-balance conclusion on data
that did not exist when they were derived: **launch acceleration 32.69** on its
third tick against the recorded 32.52 and the predicted 31.8, and **the `0.035`
contact-friction floor never crossed** (minimum loss 3.534 % over 114 contact
ticks, 0 of 114 ticks above 5 units/s below it). "Third tick" is not a quirk of
the capture - it is the first tick the emulator was sent thrust, the first two
having been eaten by the lead.

**One older hypothesis stays dead.** The run report's angular velocity averages
21.2 rad/s over the lap, which reads like a craft spinning three and a half
times a second. The original reads **22.96 rad/s on the same scenario**,
slightly *higher* than ours. Whatever that column is, both sides do the same
thing with it.

**A save-state fixed start was investigated and not adopted.** `--state=FILE`
exists and `wtype` can drive PPSSPP's own overlay to create one - both now on
[ppsspp-debugger.md](docs/reverse-engineering/ppsspp-debugger.md#save-states-and-the-input-recording-api-that-may-replace-them)
- but two loads of the same state disagreed by 0.031 units / 1.41 degrees
against `--start-heading`'s 0.0022/0.0001. The roadmap checkbox is closed via
heading-pinning; the state-loading mechanism is kept documented rather than
wired in, for a future need a heading pin cannot reach (mid-race state, a
different track's start). `.ppst`/`.p2s`/`.state` are in `just audit-leakage`'s
extension list.

## The exhaust and the boost visuals

Six passes between 2026-08-02 and 2026-08-09 worked this subject, each partly
withdrawing the last. **The evidence is on
[`exhaust.md`](docs/ghidra/functions/psp-pulse-usa/exhaust.md),
[`mesh-draw.md`](docs/ghidra/functions/psp-pulse-usa/mesh-draw.md) and
[`camera.md`](docs/ghidra/functions/psp-pulse-usa/camera.md)** - this is the net
state and the traps, not the history.

**The boost has exactly three elements and no others.** `boost_timer`
(`flare+0xb8`) reaches the flare's half-size, the reveal of the
`<Team>boost.vex` plume, and the `engine_on` flag - nothing else in
`Exhaust_Update` reads it (confidence 90). So **no other mesh on the ship can
change when a pad fires**; a shortlist of `glowingShape` /
`self_illuminatedShape` / `underbrake_flash*` was drawn up and is a dead end,
and every future version of it is too. Those are the blink lights, flashing on
their own cycle - authored, since 2026-08-11, as the same per-material keyframe
block every other animated surface uses: `v` from 0 to one whole tile over
frames 1..60, on a 1.0 s loop.

**There are no boost particles.** None of the 35 authored `.pob` effects is a
boost or speed-pad effect ([pob.md](docs/formats/pob.md)), and the flare and
trail are a hand-authored code path with no `.pob` at all. What reads as
particles is a texture. Do not go looking for an emitter to port.

**The racing craft builds exactly one trail, settled at instruction level.**
`Trail_InitPreset` has two callers: `ExhaustFlare_Init`, which makes one `0x210`
allocation and one `Trail_InitPreset(obj, 2)` with no loop, and the **missile**
constructor, which makes two at preset 0. Twin trails belong to missiles. Our
one-ribbon-at-the-nozzle topology already matches the original, and no
measurement disagreement should be read as a missing second ribbon again.

**Every recovered ribbon constant matches, and so does the state**: ring
capacity 10, taper rate `-4.5`, head taper `1.0`, three layers, per-layer
half-widths `[1.0, 0.7, 0.5]`, the `intensity * 0.35 + 0.2` width scale, the
four-fin cross and the `~3.5` direction stretch; and a
`--pose-intensity 0.1292277 --pose-boost 0.5` render reaches `0.2542` against
the capture's recorded `0.2544947`.

**Overturned 2026-08-10: the plume is replayed under `TEXMAPMODE` 0 and the
original DOES read the authored UVs. `oag_render::texgen` is the wrong
mechanism for the plume.** Settled live at confidence 92 - the recorded frame
stream was walked in GE execution order to the plume's own `VADDR`/`PRIM`
words, six independently frozen frames, all mode 0, with 73 mode-2 `PRIM`s
per frame as the positive control that the walker does see the env-map
brackets. Full method and evidence: mesh-draw.md, "The plume is replayed
under `TEXMAPMODE` 0". The earlier text here claimed
`Mesh_BeginTransparentPass` sets uvgen 2 and "nothing in the batch loop
undoes it" - the bracket is real but the plume draws outside it, downstream
of its restore. What replaces texgen as the recovered mechanism: **authored
UVs sampled through an animated per-material `TexOffset` u-scroll** - the
plume's texture-transform list (compiled lazily on first draw; `0xfeadfead`
heap canaries until then) reads `TexScale(1,1)` + `TexOffset(u, 0)` with `u`
animating (`0.956`, `0.504` at two freezes). That is the same per-material
`Gu_TexScale`/`Gu_TexOffset` transform the open-threads table below already
carries as real-and-unported - **and it is ported now, for the plume**:
`oag_formats::vex::mesh_tex_transform` parses the block,
`boost_plume_ground_truth.rs` pins the identical track on all eight teams,
and `race::Scene` samples it per frame onto the authored UVs
(`Drawable::apply_uv_transform`), replacing `texgen` there. **The scroll's
clock is the flare's own life timer (`flare+0x88`), reset at each reveal** -
measured at the updater's entry, t equals the timer on every hit - so each
boost plays the bright-to-dark sweep exactly once, which is what makes the
capture's phase derivable. At the derived phase the matched-pose mask row
beats texgen's on nearly every column and closes the "~11 % dimmer"
residual entirely (exhaust.md, "Sixth measurement"); what remains open is
smaller: b 16 counts low, orange rim 3.6x, extent 0.72. `texgen`'s
empirical win was real but for the wrong reason. The texgen code itself and its two recovered light vectors
remain correct *for the batches inside the bracket* (73 draws per frame use
mode 2); only its application to the plume is wrong.

**`exhaust::BLEND` (`SrcAlpha`) is kept because it measures better, not because
it is understood, and this is a negative result about something the project
believes it has recovered.** Putting the plume back on the recovered
`GU_FIX`/`GU_FIX` blend restores the authored `(255, 98, 5)` rim at full
strength: **29x the original's orange pixel count**, red-dominant where the
original is blue-dominant, at 21 % of its extent. So a term in the recovered
blend chain is still missing - the GE state was read carefully and does not
reproduce the picture; an unrecovered source-alpha weight does. **Do not "fix"
it back without re-running exhaust.md's table.**

**Colour space is settled: [ADR-0020](docs/architecture/adr/0020-gamma-authoritative-colour-space.md)
made gamma authoritative.** The GE is the specification and every blend equation
this project has recovered is defined on framebuffer bytes. That ADR also
absorbs the `mesh_render.rs` sRGB fork, the `race.rs` load-time plume re-encode
(deleted), the 54.6/255 measurement and the capture-path question this file used
to carry as an open row.

**Withdrawn, and stated here because the earlier text is still quoted.**
`BoostFovKick` is a **deliberate invention of this project**, not a
reimplementation of anything: a boost reads better with a fov kick, its
`DEFAULT` is a taste decision, and it is not being fitted to the original. **Do
not retune it against the "90-95 degrees" figure.** The fov chain *was* recovered
end to end ([camera.md](docs/ghidra/functions/psp-pulse-usa/camera.md)), and two
of the facts it establishes are ones `Race::projection` already assumes: the fov
unit is **vertical degrees** and the aspect is a hardcoded `480.0/272.0`.
`FLARE_ASPECT` is likewise retired - the flare quad is square, confirmed three
independent ways.

**Corrected 2026-08-09: "what widens the fov" *is* a live gap after all**, just
not a boost one. The original widens with **speed**, additively in degrees, with
no boost involved - see the craft-render-scale section below and the open-thread
row. This does not license retuning `BoostFovKick`, which remains an invention;
it means a second, separate term is missing.

**`HALF_SIZE_TO_WORLD` is `1.0`, recovered, and *not* the next thing to
measure - this paragraph said the opposite for two days.** The fitted `2.15` was
retired on 2026-08-07 by a live matrix read at `ExhaustFlare_Draw`
(`0x08904a30`); `crates/render/src/exhaust.rs` has carried the recovered value
since. What follows is kept because the *size law* below it is still the useful
part. The size law is confirmed exactly
(`(intensity * 0.6 + 0.4) * 2.5 + boost_timer * 8.0`, giving 2.5 at rest and 8.9
at a pad's armed 0.8), so the *ratio* is the original's but the absolute world
size is ours - and a constant fitted at 2.5 has never been checked at 3.96 or
8.9. The trace harness makes checking it cheap: `psp-trace.py --flare` records
`half_size` per tick, so a matched-pose render can be compared against a frame
whose flare size is *known*.

**But the pixel argument that pointed at the flare has to be re-taken before it
is leaned on.** "The gap is worst at the bright core - 4.6x at luma > 200,
narrowing to 2.1x at luma > 70, which is the signature of a too-small bright
source rather than a missing halo" was measured between frames now known to be
misregistered by **1.19-1.25x at those very ticks**, and the misregistration is
speed-dependent rather than a constant that divides out. The same applies to the
bloom shoulder (the original holding a flat luminance profile near 150 out to
~100 px where ours peaks at 201 and falls monotonically) and to the plume mask
reading `(143, 110, 178)` against the original's `(193, 147, 218)`, about 26 %
dimmer.

**Two of the three were re-taken the same day; here is where each stands.**

| reading | status |
| --- | --- |
| the plume mask, "26 % dimmer" | **Re-taken: 11 %, not 26.** And it is *mechanism* rather than a better measurement - a wider fov packs the plume's overlapping quads into fewer pixels, and an additive blend stacks more fragments per pixel. The corrected frame is a different and correct picture. |
| the bloom shoulder / annuli table | **Re-taken and found unmeasurable**, and not because of the fov. The original's saturated core has no stable centre: widening the analysis box walks its centroid from `(469,422)` to `(602,452)` and flips the headline number's sign from -16 % to +21 %. **The M6 bright-pass item loses this as its supporting measurement** - though not the bright pass itself, which is directly visible anywhere the original's track is blown to white and ours is not. |
| the flare's "4.6x at luma > 200" | **Still unmeasured.** Same family as the annuli reading and probably the same defect, since it is also a luma threshold centred on a saturated core. |

Both re-takes used `--camera-fov` from the tick's own **forward velocity**
(`60 + 0.075 * dot(fwd, vel)`), not the `speed` column, and the instruments are
committed now: [`scripts/plume-mask.py`](scripts/plume-mask.py) and
[`scripts/frame-register.py`](scripts/frame-register.py). See
[projection-vs-the-original.md](docs/rendering/projection-vs-the-original.md).

**Still not matched, and unaffected by that:** the bright pass itself is
unimplemented, and **the PSP has no programmable shaders at all**, so "find the
shader" is the wrong search.

### Closed: the craft is the right size, and the zoom was our projection (2026-08-09)

**This section used to say our craft renders too large and that the mechanism was
`0.75` applied twice. That is refuted.** Six days and five failed detectors went
into measuring a craft-specific error that does not exist. The evidence, the
instrument and its three validations are on
[projection-vs-the-original.md](docs/rendering/projection-vs-the-original.md);
what belongs here is only what a fresh reader needs to not restart the hunt.

**The craft mesh is correct to 0.15 %.** With the background zeroed, craft-only
boxes register at 0.9989/0.9985/0.9987. Across 16 frames, `craft / far scenery`
is **0.992 +/- 0.007** - and far scenery is at effectively infinite distance,
so no mesh scale can move it. The whole frame was zoomed 1.12-1.26x and the
craft merely inherited it. `1/0.75^2 = 1.7778` is dead, and **the planned
red-hulled-team capture is no longer needed** - do not spend the emulator time.

**What was actually missing is a speed-dependent fov widen**, and it is
recovered rather than fitted: the original adds `0.075 * dot(fwd, vel)` additive
**degrees** to both tripod fovs every frame. Fitting exactly that form to the 16
frames gives `fov = 60.181 + 0.07685 * dot(fwd, vel)`, with the recovered
`0.075` and the authored `60` both inside the fit's 95 % intervals. It is now
**implemented** in `Race::projection` as `SPEED_FOV_GAIN_DEG`.

**Settled on the running game, and the law is exact.** `g_camera_fov_degrees`
(`0x08b34310`) read live against the body's own `fwd` and `vel` gives
`fov(N) = 60 + 0.075 * dot(fwd, vel)(N-1)` with a **worst residual of 0.0007
degrees over 19 samples** - float32 precision, not a fit. `craft+0x7c` is
exactly zero, the one-frame offset is publish order (`Camera_PublishTripod` runs
after `Ship_UpdateCraft`), and the driver is settled by a sample at *negative*
forward velocity that drove the fov to **54.26**, below the authored 60, which
no speed-magnitude driver can do. Confidence **94**. The pixel measurement was
right to 0.6 %. **Ported the same day**: `SPEED_FOV_GAIN_DEG` in `Race::projection`, with two
tests, one of which pins the backwards case. `just` green at 1477.

**The pixel fit that briefly appeared to reject the driver was a confound, and
the resolution is still worth reading.** Nesting both candidate drivers rejected a
*pure* forward-velocity driver at about 95 % - but only on 3 of 16 ticks, and
those three are the hardest-yawing ticks in the capture, where a registration
metric that fits **no rotation term** degrades. `|residual|` correlates with yaw
rate at +0.78, and the craft slips *because* it yaws, so "off-axis" and
"the instrument is unreliable here" are the same three frames and cannot be
separated. Meanwhile `shipNode+0x140` is now **identified** as the rigid body's
own velocity - no smoothing, refuting the reconciliation this file previously
named - so the store is exactly `dot(fwd, vel)`. Magnitude **88**, driver
**85**. Both are on
[camera.md](docs/ghidra/functions/psp-pulse-usa/camera.md) and
[projection-vs-the-original.md](docs/rendering/projection-vs-the-original.md).
**Do not quote either from this file** - point at those, which is what this file
is for.

**The general lesson outlived the number.** The metric does not measure a
hard-turning frame, and nothing said so until a column of the capture that
nobody had looked at was plotted against the residuals. Before a pixel result
disagrees with a decompile, check what the craft was *doing* in the frames where
it disagrees.

**Matched-pose pixel comparisons are still invalid**, for this reason rather
than the craft's size, and with a fix: re-take with `--camera-fov` computed from
the capture's own forward velocity, or at a low-speed tick. The three readings
that depend on it are listed in the exhaust section above. **One boundary is
worth stating**, because it decides which past results survive: a comparison
between two of *our own* renders at the same pose is safe, since a shared
registration error cancels - but only while it is a ratio. Anything in
*absolute* pixels (a width, a saturated-run length, a count above a luma cut) is
still unsafe between two of our own frames if their poses differ in speed.

**The instrument is committed now**, as
[`scripts/frame-register.py`](scripts/frame-register.py), because it is the
metric for every future comparison against the original and its scratch original
lived in gitignored `data/`. It **errors out when the correlation peak lands on
an end of the sweep** rather than returning it - the failure that produced the
`1.2500` readings, which read exactly like measurements.

**Two traps worth keeping out of all that failed work.** (1) **Measure the
background before believing anything about a foreground object's size** - the
clause that hid this for six days was "with the track and buildings behind
aligning", written into the measurement's own record and never checked; the far
scenery read 1.1195. (2) **The camera node stores its rotation transposed**, so
a projection using the trace's `cam_*` rows directly lands on empty track; sample
our own render at the predicted pixels against a ship-free render of the same
pose, and if neither pixel differs, the projection is wrong.

**State comparison never inherited any of this, which is why the trace harness
is worth more than another screenshot.** `boost_timer`, `plume_timer`,
`intensity` and `half_size` are numbers on both sides and care nothing about
render scale. Prefer the flare column group and `oag-game --trace-out` over any
pixel argument about the exhaust.

### Capturing a boost, and what silently goes wrong

- **`OAG_SHOT_DISPLAY` is not optional.** Without it `niri_shot.find_window()`
  looks for a niri window, does not find PPSSPP on the Xvfb display, returns
  `None`, and `psp-trace.py` `parser.error`s out before capturing anything. The
  recipe that removes the off-canvas trap structurally is a `960x544` Xvfb
  (exactly 2x PSP, aspect-exact) with PPSSPP fullscreen on it, so `import
  -window root` *is* the game window.
- **Placing a craft on a pad has two silent failure modes.** `just pads`'
  `--before N` approach points are on the **course ring** and a pad can sit ~8
  units to the side of it (pad 0 on Talon's Junction does), so an on-ring
  approach drives *past* the pad - walk back along the **pad's own push axis**
  instead. And `--settle 8` is not enough: the hover shoves a freshly-placed
  craft several units up and it flies over the plate with `grounded = 0` for the
  whole run. `--settle 30` at 110 units/s settles it. **Check `grounded` and
  `boost_timer` in the capture before believing it** - two attempts produced 140
  clean-looking rows with `max boost_timer = 0.0`.
- **A posed frame lies unless every input is pinned.** `--pose-intensity`,
  `--pose-speed` and `--pose-boost` exist because intensity ramps at only
  `0.25`/s, so a teleported capture enters a pad at `0.1292` while
  `force_boost_state` saturates ours. Posed frames also had **no trail at all**
  until `--ticks` pushed the ring's ten samples, so the element carrying most of
  the exhaust's colour was simply absent from every comparison before that.
- **Check for strobing before believing a still frame.** A texture whose `v`
  axis alternates hard between two colours is a high-frequency response to small
  normal changes, and the UVs regenerate every frame from a pose that jitters
  under the hover spring. Measured: consecutive-tick RMSE in the plume region is
  within 0.003 of the pre-texgen build on every pair.
- **A two-heading A/B is not a heading A/B unless the poses match.** The first
  attempt compared frames at 440 and 85 km/h, so the hover spring had the craft
  at different pitch and roll.
- **The plume measurement has been retracted five times and the instrument was
  the cause every time.** A `luma > 60` gate makes every row hostage to a global
  brightness change; its replacement normalises chroma by the pixel's own
  luminance and is immune to one by construction - but the **fifth** cause was
  outside the mask entirely: every one of those revisions normalised by craft
  span, which the projection finding makes 12-25 % wrong and *tick-dependent*,
  so it does not divide out. **The blend table needs a sixth taking**, at a
  `--camera-fov` matched to the capture's speed column. **What survives the sixth
  taking, and what does not, splits on one line: does the column compare two of
  *our* renders, or one of ours against the original's frame?** All three builds
  in that table render at the same `--pose-tick 62`, so they share one
  whole-frame zoom and any ratio between them cancels it exactly. That keeps the
  ordinal results - generated coordinates beat authored ones on extent and
  orange count, and `One`/`One` is the worst build on every column. It does
  **not** keep the `b - r`-closest-to-the-original leg, or the 26 %-dimmer
  reading, or anything else stated as a distance to the original: a zoom shifts
  which of the original's pixels fall inside the mask, and no ours-vs-ours
  cancellation reaches that. Note the cancellation is also only good while the
  error is a pure whole-frame zoom and the poses match in speed - an absolute
  pixel count compared across two of our own frames at *different* speeds moves
  too.
- **A matched-pose *reference* frame from the emulator is still unobtainable
  here**, after three routes failed: `import -window root` on Xvfb returns a
  blank frame (PPSSPP's GL window never composites into the root X sees), the
  debugger's `gpu.buffer.screenshot` answers `Could not download output` at a
  CPU breakpoint on the GL backend, and `--shot-every` inherits the first
  failure. **The untried route is `PPSSPPHeadless --graphics=software`**, whose
  framebuffer lives in RAM.
- `*batch & 0x0020` is **set** on every plume batch, so the original culls all
  32 across the 8 teams, while **our renderer sets `cull_mode: None`
  everywhere** deliberately (strip winding is reconstructed rather than read).
  We draw back faces the original discards, and under a normal-driven texgen
  those shade differently rather than merely overdrawing. Not acted on: the
  no-cull decision is renderer-wide, and we are already *under* the original's
  extent, so removing geometry moves the wrong way.

## The engine/title split (2026-08-09, all seven stages landed)

**Finished.** The per-stage working handover has been deleted, as it said it
should be; what follows is the part that outlives the effort. The durable record
is [ADR-0022](docs/architecture/adr/0022-title-packages.md),
[workspace-layout](docs/architecture/workspace-layout.md), the module docs named
below and this section.

Done on a user directive to make Pure workable in parallel with Pulse. Governed by [ADR-0022](docs/architecture/adr/0022-title-packages.md),
which supersedes **ADR-0009 item 3 only** - items 1, 2 and 4 stand, and item 2
in particular still gates second-title *simulation* work behind M4's exit.

**The rule that decides every question this split raises.** Two questions were
being conflated, and they have different answers:

- *"How does this byte stream decode?"* - answered by the **file**, inside
  `oag-formats`, from its own version word. Class-ID tables, header shapes and
  schema variants stay there. `oag-formats` must never depend on a title crate:
  the reverse edge exists, so that one is a cycle.
- *"What does this title ship?"* - answered by a **title package**. Archive
  names, entry names, name hashes: `oag-pulse`, and later `oag-pure`.

Landed:

- **`oag-title`** - `Title`, `ArchiveCandidates`, `ForeignSerial`. Types only.
  Scoped deliberately to the three axes with two measured corpora; presentation
  vocabulary stays as plain constants in `oag-pulse` until Pure forces its
  shape, because `pure-status.md` measured none of it.
- **`oag-pulse`** - what used to be `oag_assets::pulse`'s tables.
- **`oag-assets::source`** (was `assets/src/pulse.rs`) - the mechanism, now
  taking a `&Title`. Its unit tests run against a *fixture* title, so the
  crate's own tests no longer depend on Pulse's constants.
- Both new crates are in `scripts/check-dependency-rules.py`'s
  `GAMEPLAY_CRATES`. **That script validates every name against
  `cargo metadata`**, so a crate cannot be listed ahead of its own creation the
  way `oag-audio` once was - `oag-pure` joins in the change that creates it.

Two behaviours changed, both deliberate:

- **`Archives::read_image` is gone**; it is `oag_pulse::read_image(&mut
  archives, name)`. The PS2 name-to-hash substitution is a fact about Pulse's
  PS2 pressing, not about archives.
- **The Pure deny-list is per-title.** `OTHER_TITLES` was a hardcoded "reject
  Pure" inside the engine, which becomes actively wrong the moment `oag-pure`
  exists. It is now `Title::foreign_serials`: each title names the *others* it
  rules out. The one-directional policy is unchanged - it rules out, never in.

Stage **(3)** is substantially done: `vex::classes` keys class ids on the file's
own version word, `fog`/`pvs`/`collision`/`track` read their ids off the file,
and `handling.rs` no longer refuses a five-rung ladder or the three attributes
Pulse added. What is left of it: ten test call sites still spelling
`CLASS_WO_TRACK` by hand (`track::find_node` exists for them), and the
pre-swizzle read, which is blocked on the finding above rather than on effort.

Stages **(4)**, **(5)**, **(6)** and **(7)** are done, plus an unplanned item
**(0)** that turned out to matter more than any of them:

- **(0) The handling schema, enumerated in one pass.** Four differences between
  Pure's `handlingstats.xml` and Pulse's had been found *one at a time*, each
  visible only once the one before it was handled, because a **survey cannot see
  an element that is not there** - absence needs a comparison.
  `crates/pure/tests/handling_schema_ground_truth.rs` walks every shipped file on
  both discs and asserts the whole symmetric difference at once. `<pitch>` is the
  last of it. It also found two things poking one file never would: a Pure file
  with **no `<Class>` blocks at all** whose six parameter blocks the decoder
  silently drops, and `Data\XML\HandlingStats.xml`, which is on Pure and parses
  unchanged. Both on [`pure-status.md`](docs/formats/pure-status.md).
- **(4) Presentation tables.** Six modules in `oag-pulse`: `textures`, `loading`,
  `hud`, `frontend`, `movies`, `race`. `oag-render` gains `oag-pulse` as a normal
  dependency, which `check-dependency-rules.py` already anticipated - rule 1
  forbids gameplay -> render, and the reverse edge is how a title package is read.
  **`SCREEN` did not become per-source and the plan was wrong to ask**: `Space`
  already carries a grid *and* the aspect it is shown as, two numbers that
  disagree by 7% on the PS2 and that one `(f32, f32)` cannot hold.
- **(6) The physics seam, stated, with nothing moved.** Four module docs -
  `oag_physics::params`, `oag_gameplay::handling`, `oag_race::zone`,
  `Course::START_LINE_OFFSET`. Every simulation number now falls in a named
  place: off the disc per team and class, off the disc engine-wide, or out of
  Pulse's compiled code, and only the third moves when ADR-0009 item 2's gate
  opens. `START_LINE_OFFSET` **could not be filed**, which is the finding: at
  confidence 65 it stands in for a computation nobody has read.
- **(7)** A `Titles` column on both tables in
  [`docs/formats/README.md`](docs/formats/README.md), and the last version-locked
  `CLASS_WO_TRACK` lookups routed through `track::find_node`. One of those was in
  `oag-trace`'s own `ai_of`, not a test: it would have found no spline in a Pure
  track.

**Stage 3's pre-swizzle precondition was checked, and it failed.** The claim was
that `Texture` payload `+0x06` bit 0 is set only on font atlases in Pulse, so
`vex::textures` could start reading it for free. It is set on **88 of 5,375**
`Texture` nodes on the Pulse PSP pressing and **120 of 8,972** on the PS2 one,
and they are ship liveries, glass, engine and environment maps, not font
atlases. `crates/formats/tests/texture_swizzle_flag_ground_truth.rs` measures
and pins it; [`pure-status.md`](docs/formats/pure-status.md) carries the
correction.

So **do not make `vex::textures` read `+0x06` unconditionally.** Gate it on
version word <= 4, or settle what bit 0 means first. The open question the
histogram cannot answer: whether the Pulse claim is wrong, or whether bit 0 is
not the bit that claim means. Decoding one flagged Pulse texture both ways and
looking at it would separate them; that is unstarted.

**Stage 6 is deliberately half-done and must stay that way until M4 closes.**
The physics, Zone and `START_LINE_OFFSET` constants got a seam and did not move.
The force law is M4's live blocker and relocating its constants mid-investigation
is the one part of this refactor with real downside. `oag_race::zone`'s four
numbers are the ones to move first when it does: they are Pulse literals, and
Pure ships zone mode too.

**Decisions not to silently reverse.** No `trait Game`, no `enum Title`
dispatch, no plugin registry - ADR-0022 answers the n=1 objection for the
*format layer only*. **One axis has since been added to `oag-title` and it is
still not a trait**:
[ADR-0023](docs/architecture/adr/0023-boot-sequence-as-title-data.md) makes the
boot sequence a measured table per title, superseding ADR-0022 item 4 for that
axis alone, because both titles' sequences were cold-booted and a front-end
XML's declared entry point turned out not to be its runtime's - it holds on Pure
and fails on Pulse, so the order cannot be derived from the data at all. It is a
struct of tables selected once by serial, and item 3's refusal of `trait Game` is
upheld by it rather than weakened. No `oag-psp`/`oag-ps2` crates; ADR-0004 stands, and turning
the console axis into a code axis forfeits the property that made the PS2 fan-out
cost days. `classes_of` errors on an unknown version rather than defaulting to
V6, because the numberings share no id and a fallback looks exactly like an empty
file. The foreign-serial lists rule a source *out*, never in, or a real player's
own legitimate pressing gets hard-rejected. A `None` in a class table means "not
recovered", never "absent from the format".

**Two traps this refactor paid for.** A table read from data collapses "nothing
matched" and "I could not read the file" into one empty result unless you are
deliberate - it bit `collision::from_vex`, the `pvs` fixtures and `Stats::class`,
and it is live again in the decoder's handling of Pure's classless zone file. And
**do not `git add -A` while another session shares the worktree**: a file written
concurrently landed in two commits unreviewed; `2628a80` is the correction.

## Open threads

Each is a real, named next step. Task numbers are the ones the agent passes
used, kept because commits and docs cite them.

| Thread | What is known, and the next step |
| --- | --- |
| **`just apply-names` reported success for renames the bridge did not make** | 2026-08-10, found while landing the shield rows. `scripts/apply-ghidra-names.py docs/ghidra/functions/psp-pulse-usa/names.tsv --program /psp-pulse-usa/BOOT.BIN` printed **"383 applied, 0 skipped"**, and every one of the eight new symbols was still `FUN_`/`DAT_` afterwards - proved by the manual MCP call, which reported renaming *from* `FUN_0883dd24`. There is only one Ghidra instance (`list_instances`, TCP 8089), so it is not a wrong-target problem. **Data rows have a second, separate failure**: `rename_data` is rejected outright by the server's Hungarian-prefix validator (`g_skill_level` -> `missing_hungarian_prefix`), which conflicts with ADR-0005's own `g_snake_case` convention that all 45 existing data rows use; `create_label` works and is what landed them. Root cause not chased - the eight renames were made directly through MCP and the program saved. **This matters because `just apply-names` is the project's stated mechanism for making the Ghidra database reproducible from the repository**, and a silent no-op means a fresh import stays at `FUN_` while the tool says otherwise. Check a sample symbol after running it. |
| **DLC packs are mounted; two things inside them are not read** | 2026-08-09. The four Pulse packs load, region-independently: `just play --race --team Mantis` off `pulse-psp-usa.chd` draws a European pack's ship, and the catalogue goes from 8 teams / 24 circuits to **12 / 32**. Format, evidence and per-claim confidences on [`docs/formats/dlc-pack.md`](docs/formats/dlc-pack.md); the divergence from the original's region lock is [ADR-0021](docs/architecture/adr/0021-region-independent-dlc.md). **Left unread, both deliberate.** (1) `downloadNN.xml`, entry 1 of each `PACKn.edat`: a `PI_Grid` championship ladder, which is progression work rather than asset work and belongs with the campaign, not here. (2) `PI_TeamModel`/`PI_ModelSkin` - the concept, zone and unlockable liveries with their `loyalty` thresholds - are declared in every team's manifest, disc and pack alike, and `catalogue::Team` deliberately does not collect them while nothing draws a second hull. That is the same list the roadmap's "livery and team variants" item wants, so whoever takes that item gets the parse for free. **The trap worth knowing**: a pack's ship is in `PACKn.edat` and its `handlingstats.xml` is in `PACKn_UI1.edat`, so mounting only the main archive gives a ship that loads and cannot race. |
| **Two recovered chase-camera behaviours are ported; `headtilt` is not** | `crates/render/src/camera/chase.rs` reproduces `Ship_UpdateCameraRigs` whole - rigid radius, `pos_height` after the spring, craft scale applied about the craft after the spring - measured against `data/traces/pad0-boost.csv` at **RMS 0.008 / max 0.060** world units against the shipped model's 4.938 / 6.824. Two things to carry forward. **Neither half was worth landing alone**: `pos_height` after the spring, on its own, is *worse* than what it replaced, which is the shape of a change anyone trying half of it would have reverted off a screenshot. And **a settled camera cannot see any of it** - all the models agree exactly at rest, so every static probe and every `--pose-from --ticks 0` screenshot this project ever took was blind to a 4.9-unit error. **A camera can only be judged while it is lagging.** `crates/game/tests/chase_camera_ground_truth.rs` pins it (`#[ignore]`d; needs a disc image *and* the capture). **Still open:** `<InternalCamera headtilt>` is parsed and not applied - the original rolls the view's up vector by `side * craft[0x844] * headtilt`. **`craft+0x844` is identified as of 2026-08-09 and this item is now a port, not research**: it is a smoothed steering lean, `steer * 0.01` through a rate-limited follower at `craft+0x848` and then a `4/s` first-order filter, saturating at 0.6 - so the craft leans *into* the turn and the arithmetic needs no new capture. Confidence 80 for the arithmetic, 0 for what the input's units are. **A wrong name was live in the Ghidra database on the producing function** - `0x0883fab4` carried `Ship_UpdateStartBoost`, which is really `0x0883fdec`, so two functions shared one name and a search for the start boost landed on a steering filter; reverted to `FUN_0883fab4` rather than renamed, since its two outputs feed two different subsystems. `names.tsv` was never wrong. Also `craft+0x790`'s **second** write site is now fully settled - see the fov section above; it runs in an external view and the law is exact. |
| **The original's fov widens with speed, and `Race::projection` does not** | **2026-08-09, and it replaces the "craft model is too large" thread, which is closed - see the section above.** There is no second scale site to find; there never was one. What is real and unported is `craft[0x790] = dot(fwd, vel) * 0.075 + craft[0x7c]` (`FUN_088455ec`, `0.075` at `0x08a7b6a0`), added to both tripod fovs every frame in `Ship_UpdateCameraRigs`' tail. Both constants are now confirmed against 16 frames of the original's own pixels: the fit is `60.181 + 0.07685 * dot(fwd, vel)` degrees, bracketing `0.075` and the authored `60`. **Do the one-breakpoint read first**: `g_camera_fov_degrees` (`0x08b34310`) at the exact tick `data/shots/pad0-boost/tick00000.png` was captured, speed 71.84. `~65.4` confirms the whole finding from the executable's own state rather than from pixels; `~60` refutes the interpretation while leaving the registration intact. Two things in the repo still numerically contradict the finding - `exhaust.md`'s 136-edge world fit (slope 0.4 % where a 1.1195 zoom needs 12 %) and the B2 tower's "90-95 degrees" - and that one read disposes of both; the docs page carries the arithmetic and the pairing hypothesis for the first. **Done, both halves.** The live read confirmed the law exactly (0.0007 deg worst residual over 19 samples), and it is ported: `SPEED_FOV_GAIN_DEG` in `Race::projection`, additive degrees before the player's fov setting, composing with - not replacing - `BoostFovKick`. The one predicted test failure was exactly the predicted one and its intent survived the fix: it compared the kick against a tick-0 reference, which the speed term breaks for reasons unrelated to the kick, and now compares two races at the same tick. A second test pins the backwards case. **Captures are unaffected**: `place_at` resets the body, so a posed craft has zero velocity and the term is exactly `0` in the `--pose-from` path - a matched-pose comparison still needs `--camera-fov` computed from the tick's own forward velocity. There are **no golden or screenshot frame-hash tests anywhere in `crates/`**, and `projection` feeds no simulation state, so **determinism is untouched**. The additive form needs no `speed == 0.0` short-circuit either, unlike the `boost_kick` zero case sitting right beside it: `authored + 0.0` is bit-exact where `2.0 * atan(tan(x) * 1.0)` is not. Two loose ends worth an emulator session each: an exec breakpoint on `0x088455ec` under an external view would turn "runs in external view" from inference into measurement, and `craft+0x7c` is only shown quiescent on a *clean* run, so a capture with an impact in it could still show it carrying something. |
| **The per-material `Gu_TexScale`/`Gu_TexOffset` transform is real and unported** | **The values gap closed 2026-08-10**: the keyframes are authored in the `.vex` file itself, in a 0x40-byte block after each mesh's material array - `u16` key times in 60 Hz frames, `s16` values in 1/256 units, clamp-and-lerp, evaluated per frame by `TexAnim_EvalKeyframes` (`0x08927034`) via `TexAnim_UpdateTransform` (`0x08927204`), both now named at 90. The plume's track: `u` ramps `2/256 -> 253/256` over frames 1..90, loops at ~1.5 s, `v = 0`. Talon's Junction scenery carries real tracks too, so the earlier "every track material simply has no data" inference is withdrawn - the compiled-list path bypasses `Gu_TexOffset`, which is why the live negative never saw it. See texture-animation.md and vex.md. **Ported for the plume 2026-08-10** (parser `oag_formats::vex::mesh_tex_transform`, applied in `race::Scene`, pinned by `boost_plume_ground_truth.rs`); track scenery still draws static, which is now a pure render-side port with the data source known. |
| **The menus draw the disc's layout; the chrome around them is unbuilt** | **2026-08-10, and it closes the standing "only `pulse_text.fnt` is wired to the menus" item below.** `Data.wad` carries `MainMenu_Definition.xml` and 16 sibling GUI files - `menu.rs`'s comment that "the disc's own menu layout has not been read" was simply out of date. Read, then measured against a PPSSPP capture: rows at x=50, pitch 28, seven of them, in the `menu` font role, `TextColor` for an unselected row and a brightening toward white for the selected one. Our capture now measures identical to the original's - glyph tops 40.0, heights 11.5, pitch 28.0, left edge 50.0. Everything is on [menus-original.md](docs/ui/menus-original.md) and [fe-menu-definitions.md](docs/formats/fe-menu-definitions.md). **Three things are known and unbuilt.** (1) The light angled **top bar** (`topbarleft`/`center`/`right`) and the **footer** (tag block, scrolling ticker, button prompts) - until the bar exists the authored black `TitleColor` cannot be used, so this build substitutes its own title colour, the same substitution the picker already makes. (2) The selected row's highlight **pulses** - two frames of the same still menu measured different peaks - and its period and depth were not measured, so it is drawn flat; the same discipline as task #7 below, and the same warning, **do not measure it off our own build**. (3) The **easing curve is invented** (confidence 30): the capture shows only that the motion accelerates. `Tween::eased` is the one function to change if anyone reads the real curve out of the executable. **Two traps cost time here**: `just drive menu` walks straight past `Main Menu` into a live race, and the original enters an attract demo after ~120 s idle, which silently invalidates a capture left sitting. |
| **Task #7: PRESS START does not fade in or throb** | `BOOT_PRESS_START` carries `pulse="true"` and `delay="1"`; ours appears at once, at full opacity, and stands still. Neither attribute is decoded. The two numbers needed are the period between peaks and the min/max alpha, both readable off a real display: `just launch-pulse-psp data/images/pulse-psp-eu.chd`, reach PRESS START, record, step frames over ~4 seconds. **Do not measure it off our own build** - ours is static, so a capture would confirm nothing. Full context on [frontend-boot.md](docs/architecture/frontend-boot.md). |
| **Task #8: `escape` -> menus shows 1-3 black frames** | The fourth instance of the backdrop-seeding bug; the other three are fixed and written up on [menus.md](docs/architecture/menus.md#a-continuous-playhead-is-not-a-continuous-picture). The fix is the same one that closed the boot path: stash a picture beside the playhead when a race starts. Neither `--screenshot` nor `--menu-page` can see this path. The way in: run the windowed build under Xvfb with a temporary probe printing `Backdrop::shown` and what `take_upto` returned per frame, force a race and then `escape` - before the fix the first menu frames read `take=None shown=None video=TRIMMED`. Dump frames to `data/cache`, never the session scratchpad, and arm the dump **on** the defect rather than over a frame range: a range-gated readback slows the loop enough to hide the timing it is meant to catch. |
| **PS2 front-end layout is hardcoded to 480x272** | **Closed 2026-08-09, and not the way this row proposed.** `SCREEN` did *not* become per-source: `frontend::Space` carries a grid **and** the display aspect that grid is shown as - two numbers that disagree by 7% on the PS2 and that one `(f32, f32)` cannot hold - so a per-source `SCREEN` would have been a second, weaker `Space`. The rects and pillarboxes were routed through `Space` instead; stage 4 of the title split found the last three that were not (the menu backdrop's rect in `main` and `capture`, and the no-movie aspect fallback in `boot`). `SCREEN`'s remaining readers are the ones for which 480x272 is right whatever disc is mounted: our own loading screen, the HUD bounds check, and `race::AUTHORED_ASPECT`. Task #37's remainder is **mostly closed as of 2026-08-10**: the menus now draw in the title's own menu role, which is `Pulse_20.fnt` on Pulse, resolved through the language plugin's `<Font>` slots rather than a hard-coded filename. Two faces are now in use where there was one. **Still open**: the original draws a screen *title* in the `title` role (`Pulse_14.fnt`, 17px) while its rows use `menu` (22px), and this build draws both in the row face at the authored title scale - two faces on one screen needs the atlas keyed by face, which `hud::Font`'s three unhonoured variants also want. |
| **The HUD layout ground truth is PSP-only, and the PS2 layouts are unchecked** | Measured 2026-08-09 while settling `frontend::SCREEN`, and re-measured coordinate by coordinate after the first phrasing over-claimed: the PS2 release authors the same five HUD layouts in its own **640x448** grid. `Data\XML\Arcade_HUD.xml` carries 141 coordinate values on both discs in the same order, and **121 of them are the PSP's own value scaled by 640/480 or 448/272 and rounded to an integer**. `Skin.xml` was measured the same way on 2026-08-10, after a review caught `frontend::Space` making the same over-claim this row had already been corrected for: **30 of its 43 shared coordinates scale, 13 do not**, and the exceptions are pinned in `crates/game/tests/frontend_grid_ground_truth.rs`. Three of those thirteen are `<Animation><Key>` travels rather than positions - the same mixed meaning of `x` as `TimeDiffIcon` below, in a second file, which makes it a fact about the dialect rather than about one layout. That file also pins **which PSP pressing the PS2 layout came from**: `y=220` USA, `y=230` EU, and the PS2's 362 scales from 220, so a EU-to-EU comparison of that widget looks 4% wrong for a reason that is not the console. The 20 that are not are 18 `<Mode3D><Model>` placements, unchanged at the PSP's `x="-240" y="136"` in an orthographic 3D mode nothing here places, and `TimeDiffIcon`'s two small negative nudges inside an `<Item>`. **It is not a flat scaling, and that matters for the fix**: a PS2 sweep needs more than `Space::PS2.size` in `inside_screen`, because it also has to agree with the `RUNTIME_ANCHORED` skip about which coordinates are screen positions at all. Consequence: `hud::inside_screen` and `hud_layout_ground_truth::every_widget_lands_on_screen` are PSP-only by construction, so **no PS2 HUD layout has ever been checked for a dropped `<Item>` offset**, which is the defect that test exists to catch. The runtime draw path is unaffected - it takes its `screen` uniform from the source's own space. The sweep needs `Space::PS2.size` in `inside_screen`, a second `open()` in that test, and a decision about the unscaled offsets above. |
| **PS2 sky and pads: 30 circuits unswept** | `build_sky`/`build_pads` never took the external texture set, which is fixed and documented on [`skycube.md`](docs/formats/skycube.md#open) and [`ps2-texture.md`](docs/formats/ps2-texture.md#how-a-model-finds-its-texture-set-directory-position-not-a-name). **Not done**: the other 30 PS2 circuits were reasoned about from the same shared texture-set resolution rather than screenshotted - worth a visual sweep if anyone doubts it generalises. |
| **Audio: race SFX is blocked on research, not on code** | Both discs make sound and menu sound is reachable, because `frontend.bnk` degenerates usefully. **The blocker is per-sound boundaries inside a race `.bnk`**, which the other banks do not give up. Two untried angles, cheapest first: name the cue-string lookup that the 37 `FUN_089392b0` callers feed, or back-walk from `__sceSasSetVoice`, where a waveform address and length finally reach the hardware. Plan (outside the repo): `~/.claude/plans/investigate-plan-to-add-dreamy-toast.md`. Movie audio is separate and equally open - ATRAC3+ is demuxed and discarded, and whenever it arrives the playhead becomes something two consumers pace against rather than one; `movie::Player` is the place that changes, not `Feed`. |
| **`oag-trace plan` has never been replayed into the emulator** | The subcommand works in our simulation and `scripts/psp-autopilot.py` gained `--gate/--gate-dir/--gate-after` for the same target driven live. **The only thing that settles it**: `psp-trace.py --script verification/scenarios/talons-junction-pad-0.inputs --script-lead 2`, compared against `plan --trace-out`. Design and the full untested list: [`autopilot-planning.md`](docs/tools/autopilot-planning.md). Watch the start-pose trap - `drive`'s default `Start Position` on `16_Track` is ~138 units behind the time-trial line. |
| **M6 authored lighting: no hardware light slot found enabled** | `AmbientLight`/`DirectionalLight` are registered, `PointLight` genuinely is not, and none of the mesh-draw functions read so far enables a GE light slot. The runtime direction vector recovered along the way is useful evidence for [`lighting.md`](docs/formats/lighting.md)'s open rotation-versus-translation question. Recorded so a future pass does not reopen them: `FUN_08a6b5bc`/`FUN_08a6b5c8` are World/Scene container class stubs, not lighting. Full account on [`psp-pulse-eu/lighting.md`](docs/ghidra/functions/psp-pulse-eu/lighting.md). |
| **Ghidra target of record is `psp-pulse-eu`; EU cross-verification is owed** | Search and decompile in `/psp-pulse-eu/BOOT.BIN` first, cross-verify against `/psp-pulse-usa/BOOT.BIN` with `find_similar_functions_fuzzy`/`diff_functions`, document under `docs/ghidra/functions/psp-pulse-eu/`. **The camera pass got two of six**: `Camera_SubmitScene` (EU `0x088786d0`) and `Camera_PublishTripod` (EU `0x08885bd4`) are body-identical to their USA counterparts including the `1.7647059`, the `65.0 / fov` near-plane term and the literal `480.0 / 272.0` (confidence 90); the other four are listed as not-checked rather than guessed. Data rows stay out until the import is rebased - see [the workflow page](docs/ghidra/workflow.md#psp-pulse-eus-data-addresses-do-not-share-usas-base). |
| **`/psp-pure-usa`, `/psp-pure-eu` and `/psp-pure-eu-reimport` were missing from the project on 2026-08-10, despite the 2026-08-09 note below describing them as already imported and saved** | **Most likely just a different workstation's copy of the `OpenAntiGrav` Ghidra project**, not data loss - the project is per-machine (`data/ghidra/` is gitignored, per-checkout), so two workstations naturally hold independent Ghidra databases under the same project name. Not confirmed which machine wrote the 2026-08-09 note. Whatever the cause, the two rows below are a full, independent redo done on 2026-08-10, not a continuation of the earlier one - and it landed the same swap the earlier note asked for. **If this happens again on a workstation that expects the earlier state, check which machine before assuming loss.** |
| **The EU/Pure imports have never been diffed** | Both Pulse builds and both Pure builds are imported and folder-organised per binary. **The payoff has not been cashed in**: run `diff_functions`/`compare_programs_documentation` between `/psp-pulse-usa/BOOT.BIN` and `/psp-pulse-eu/BOOT.BIN` to test the networking-code hypothesis. Same opportunity for the two Pure builds, with no hypothesis yet to test. When cleaning up a botched import, do the delete/rename in the Ghidra GUI - the bridge does not reliably release a program. |
| **`psp-pure-usa`/`psp-pure-eu` re-imported and verified, 2026-08-10** | Redo of the 2026-08-09 work on this workstation (see the row above). `/psp-pure-usa/BOOT.BIN` imported clean the right way round (default base, full auto-analyze, then `set_image_base 08804000`): **8,958 functions**, string xrefs resolve post-rebase (`0x08a50450` -> `FUN_088990f4`). `/psp-pure-eu/BOOT.BIN`'s first attempt never completed auto-analysis - stuck at 0 functions for a whole working session, `reanalyze` timing out every time it was polled; abandoned rather than diagnosed. Reimported to `/psp-pure-eu-reimport/BOOT.BIN`, which completed cleanly: **8,969 functions** (identical to the figure the 2026-08-09 note recorded for the same procedure - the number is reproducible, not evidence of a shared database), string xrefs resolve (`0x08a4c538` -> `FUN_08898594`, the `HandlingStats.xml` loader), and **all 9 rows of `names.tsv` applied clean** (`apply-ghidra-names.py --program /psp-pure-eu-reimport/BOOT.BIN docs/ghidra/functions/psp-pure-eu/names.tsv`, saved). **Landed**: `/psp-pure-eu` (the stuck-at-0 program) deleted and `/psp-pure-eu-reimport` renamed to `/psp-pure-eu` in the GUI - confirmed after, both `/psp-pure-usa/BOOT.BIN` and `/psp-pure-eu/BOOT.BIN` are the verified imports, no `-reimport` folder left over. The delete/rename needed the GUI: `delete_file` through the bridge fails with `"BOOT.BIN is in use"` even after `close_program` reports success and `list_open_programs` no longer lists it - confirmed twice, a few minutes apart, same session. Procedure and verification checks: [workflow.md](docs/ghidra/workflow.md#importing-a-binary-analyse-then-rebase). Residual wart, documented and not blocking: this order leaves instruction *bytes* unrelocated, so decompiled constants read `0x248538` where the string is at `0x08a4c538` (add `0x08804000`). |
| **Ghidra names owed from the camera/matrix corroboration** | `FUN_08901dc4` (the scene projection builder), `FUN_08900884`/`FUN_08900a9c` (the view-stack writers) and the `Gu_SetMatrix` index mapping are all read and written up on [`exhaust.md`](docs/ghidra/functions/psp-pulse-usa/exhaust.md), but **none is renamed in Ghidra or in `names.tsv`**. Same for the two plugin-manifest functions [`frontend-boot.md`](docs/architecture/frontend-boot.md) reads at confidence 80 (`FUN_0888b7dc` and its caller). A future pass should land the rows properly. |
| **The ghost-ship renderer is read and written down nowhere else** | Ghidra by-catch from the magstrip detour, kept here because it exists in no docs page and nothing is renamed (confidence ~55). Class `0x3d4` `MeshNode_Ghost` registers a vtable at `0x08ad171c` from `FUN_08911480` (the generic `Mesh` class `0x125` registers `0x08ad1694` in `FUN_089100a0`; vtable entries at base `+0xc`, 8-byte stride). It overrides submit - `FUN_08910320`, which enqueues the same mesh up to three times with sort keys `0x4d000000 \| 0..2` - and draw, `FUN_08910fe0`, a three-phase state machine: phase 0 untextured depth-lay, phase 1 per-material texture with `Gu_DepthFunc(EQUAL)` and a fixed-colour blend whose brightness ramps with distance to the player ship over 5..25 units, phase 2 `Data\Tex\staticglow.mip` projected through a **texture matrix** (`Gu_SetMatrix(3, world x view x scale/offset)`, `TexMapMode(matrix)`, scale globals `4.0`/`1.8` at `0x08abf4c4`/`c8`, offsets re-randomised per frame at `0x08abf4e4`/`e8` by `FUN_0891055c`), with stencil and alpha-test `GEQUAL 0x80`. Reads as the **time-trial ghost renderer** - proximity fade plus sparkle. [roadmap M6](docs/overview/roadmap.md) has the checkbox; it needs replay data, so it lands late. |
| **Magstrip leftovers, none load-bearing** | The overlay question is closed ([track.md](docs/formats/track.md)) and these four are what was left unexamined. (1) The magstrip's own animation, if any - `_magsurface3_1verb`'s `verb` suffix is unexplained, and material `+0x10`'s flag drives `FUN_0892733c` texture-matrix uploads in the batch walker `FUN_0890d0cc`. (2) Batch `+0x14`, and pass bit `0x1000` - present on every magstrip batch except the far-LOD copy's, meaning still open ([mesh-draw.md](docs/ghidra/functions/psp-pulse-usa/mesh-draw.md) also has `& 0x1000` undecoded). (3) Reproducing the corner tick-frames, only if anyone still cares - the magnification-artefact explanation stands. |
| **Frame comparison: three residuals** | The pipeline landed and settled the fov unit. Left: (1) the fov reading rests on one ship and one view - a second team's authored value plus the internal view closes it fully, about an hour of emulator time; (2) the shot-versus-row phase is bounded at two ticks but not pinned, which only a fast-moving per-tick capture would do; (3) `place` always writes the basis rows and the `+0xc0` transpose together, so whether the integrator's post-loop rebuild covers the transpose alone was never isolated - academic while the pair works. |
| **Weapons: three of thirteen are built; what is left needs the AI or an unread mechanic** | 2026-08-11, [pickups.md](docs/gameplay/pickups.md) - which carries the recovered-versus-ours split and is the page to read, not this row. A `Weapon Pad` hands out a pickup, `SQUARE` fires it, `CIRCLE` absorbs it, a time trial gets its free turbo a lap, and **Turbo, Shield and Rocket do something**. **This needed a fourth race mode**: a weapons-off race in the original hides every pad and empties the trigger list, so none of the three modes here could exercise a pickup at all - `Mode::SingleRace` is the container. **What is still unread is the same word it always was**: `craft+0x1c0`'s bits, and the pickup word at `*(entity+0x4c)+0x1b8` behind it. Bit `0x0004` is identified (the `1.2` engine multiplier) and thirteen are not. **No grant, fire or projectile-flight call site exists anywhere in the executable**, so the grant, the draw, "one slot", the shield refusing damage and the whole of rocket flight are this project's and are labelled so in code and on the page. **The ten still out are blocked on something specific, not on effort**: Missile needs a lock *and a target*, which needs the AI; Quake needs track deformation; LeachBeam a beam; most of the rest the slowdown mechanic behind `<Global slowdown_limit>`, which has no consumer. `oag_gameplay::pickup::IMPLEMENTED` is still the whole list and growing it is still the whole change. **And `spread` is an open question, not a closed one** - it is on the Rocket and not on the Missile, which argues for a volley; this engine fires one as a stated choice. |
| **Three traps the weapon pass hit, all of the "silently nothing happens" kind** | 2026-08-11. (1) **`oag_render::exhaust::MAX_VERTICES` was 6** - the flare's one quad - and `Pipeline::upload` clamps with `min`, so projectile sprites appended to that buffer were dropped with nothing in the logs and nothing on screen. It is now `MAX_SPRITES * 6` with a `const _: () = assert!(MAX_SPRITES > 1)` behind it and a budget test on both sides. Anything appending to a shared vertex buffer in this tree should check its ceiling first. (2) **`race::key_for_button` bound no key to `SQUARE` or `CIRCLE`**, so `--press square` was a silent no-op and a weapon capture was impossible - the function documents skipping unbound buttons, which was harmless until a weapon needed one. The real keyboard map (`oag_input::keys::map_key`) had `c` and `z` all along; the capture helper is a second, smaller table and they had drifted. (3) **A ground-truth test that pins a specific weapon breaks when `IMPLEMENTED` grows.** `a_weapon_pad_on_the_disc_hands_out_a_pickup_in_a_single_race` asserts a fired Turbo's *magnitude*, which stopped being automatic the moment a pad could hand out something else. `Options::seed` exists for that, and the test **scans** seeds for the draw it wants rather than pinning one, so the next weapon does not quietly invalidate it. |
| **Two findings the pickup work produced, both cheap and both easy to lose** | 2026-08-11. (1) **The HUD pickup icon is found by *name*.** [hud.md](docs/ui/hud.md) had these down as "14 `*Icon` widgets" with "numeric" ids and no known mapping; `Arcade_HUD.xml` authors **thirteen**, named after each weapon's own `type` string, so `format!("{}Icon", weapon.as_type())` is the entire lookup and the fourteenth was `TimeDiffIcon` being counted. Pinned for all thirteen against the shipped file. (2) **A time trial's free turbo has a second shipped record.** `MSC_EVENT_TT` said *"a free turbo pickup once per lap"* and that was the only source; `TimeTrial_HUD.xml` authors a `PickupBackground` and **`TurboIcon` alone**, where `Zone_HUD.xml` authors none and `Arcade_HUD.xml` authors all thirteen. A layout carrying one pickup widget for a mode whose pads are hidden is otherwise inexplicable, so the two files agreeing puts the rule at 85 - and it is implemented. **The trap that hid both**: HUD layouts are shortened XML with a *per-file* dictionary, so `name` is `b=` in one and `c=` in another, and grepping the first file's key against the second returns nothing with exit code 0. It reads exactly like "this layout has no icons", which is what it was believed to say for half a session. Expand before grepping, or use the parser. |
| **The shield path's last unmeasured field: `entity + 0x368`** | 2026-08-10. Everything else about the pool now has a runtime leg, including the coefficient (`amount / |p| = 0.035000` on 25 of 25 calls) - see [shield.md](docs/ghidra/functions/psp-pulse-usa/shield.md). What is left: `entity + 0x368` gates the whole damage path, which touches the pool only when it is 0 or 2, and several sites test `< 0` explicitly so `-1` is a real value. "Local human player slot" fits every site read and would imply AI craft take no damage through this function, which is almost certainly wrong. Confidence 60; the field keeps its offset rather than a name until someone finds the writer. **Two probe facts worth keeping** for whoever works this path: the contact ring is intact at a `Ship_Damage` breakpoint and drained at a `Ship_UpdateCraft` one, and `Ship_Damage` takes its entity in **a0** with the float amount in **f12** - the leading float does not reserve `a0` the way o32 would. |
| **The hull sparks are approximated in three named ways** | `ShipCollisionFx_Trigger` (`0x089246b4`) is found, renamed and documented, and `sparks.rs` implements the blend-split pipeline pair, the streak geometry and the locator anchoring. Still approximated, each with a doc note: colour endpoints instead of the 256-entry tables (ADR-0006; runtime disc-load is the recorded follow-up), a cone aimed along the contact normal instead of the authored emitter-node frame, and ~~the streak end-cap stretch (untraced resource field)~~ - **that one is recovered**: `ParticleSystem_InitParticleFields` (`0x088f79b4`) writes a literal `1.0f` and there is no resource field behind it (confidence 88). **A side-by-side `just play` wall hit against the original is the open visual check.** |
| **The original's camera/HUD shake on impact is untraced** | A hard wall hit in the original visibly shakes the camera/HUD; `oag_render` has no shake at all. The once-suspected function calls no camera API (contact-response.md carries the retraction) and `FUN_088418e0`'s reactions - sound, hull damage, shield flash - do not shake anything either, so the consumer is elsewhere: likely the camera update reading impact state off the craft, or a screen-space offset in the HUD draw. The severity plumbing recovered for the sparks (`min(|impulse| * 0.0125, 1)`) is the obvious input to look for. |
| **PVS culling: three ceilings, one of them invented** | (1) A craft's bounding sphere reaches 6-14 of the 64 sections, and **splitting batches by section at load** is the only lever that moves this - it trades draw-call count, so measure before building. (2) Which section a *mesh* belongs to is still unrecovered: `oag_render::pvs` invents it by sphere-versus-box overlap and says so, and recovering the original's association would retire the only invented part of the pipeline. (3) Moving entities have no section and are never culled by tier one. |
| **The HUD's four remaining items** | The layout was never an RE problem - Pulse ships five layouts as `Data\XML\*_HUD.xml`. What is left is on [hud.md](docs/ui/hud.md), and the one worth naming here: **a mode's code substitutes string keys into widgets the layout positioned** - a time trial's top right reads `IG_HUD_RECORD` where `TimeTrial_HUD.xml` says `IG_HUD_TOTAL`, and 26 of the binary's 38 `IG_HUD_*` keys appear in no layout at all. The substitution rule is unread and deliberately **not** worked around by hardcoding. Zone, Eliminator, `<Mode3D>` and text outlines are scoped out and listed on the page. |
| **Race modes: the start line is measured on one circuit only** | Time trial, speed lap and Zone run. `Course::START_LINE_OFFSET` is fitted on `16_Track`; the authored `Start Position` frame would replace it at confidence 88 across all 40. Two guards exist because both failed in practice: `RaceState::lap_gate` requires the near half *then* the far half of a lap (the ship spawns behind the line, so its first crossing is a wrap, and rocking over the line would otherwise record an unbeatable best lap), and lap 1's clock starts at the line rather than at the standing start. |
| **Zone ends now; what is missing is the explosion, not the rule** | 2026-08-10. The chain is closed end to end and implemented: pool empties -> `Ship_Damage` sets craft state 4 -> the explosion runs `0.5 s` (`0x088404c8`, four lines) -> `Ship_SetState` case 5 sets **bit 12 of `entity+0x860`**, which is what `Zone_UpdateRacing` ends on. [zone-mode.md](docs/ghidra/functions/psp-pulse-usa/zone-mode.md) had that bit down as set by nothing findable; the search had been in the Zone code and on the contact path, and it is in the craft's own state machine three states later. `oag_physics::damage::CraftState` and `oag_race::RaceState::eliminate` are the port, confidence 88, no runtime leg. **What is deliberately absent**: the original plays `_BLOWUP`, hides the HUD and swings the camera into mode 5 through state 4, and this engine has none of those, so a wrecked run just stops - `race.rs` carries the note at the point where they would go. Also unmodelled: states 6 and 8, the respawn and the Eliminator's kill bookkeeping, which state 5 leads into after a further `1.5 s`. |
| **The grid is recovered and ported; the opponents do not move** | 2026-08-10, [grid.md](docs/ghidra/functions/psp-pulse-usa/grid.md). Order: `Race_SpawnGrid` (`0x088247e0`) assigns slots from `g_grid_orders` (`0x08ab0a90`), a table of eight-`int` permutations compiled into the executable, row picked by the unread `FUN_0894d440()`; **a short field packs to the back**, so three racers take slots 6-8. Geometry: **the authored `Start Position` node is slot 8**, measured - our spawn from it lands 1.84 units from the original's own eighth craft against 139.7 from the time-trial line - and the other seven run forward in two staggered columns, `19.79` per slot along forward and alternating `20.0` along row 0, all sharing one heading. `oag_gameplay::spawn::grid_pose` is the port, with the two constants flagged **measured rather than read**: `20.0` is almost certainly authored, `19.79` is a mean of steps between `19.763` and `19.862`. **Reproduce with** `psp-drive.py menu --single-race` (new, and the only reachable race type with a full field), then sample the racer table at `0x08b34420`, stride `0xdc * 4`, count at `0x08b35fa0`, AI object's `+0x50` is the slot. **The one run cannot separate two things**: the local player was the last racer, and both the identity permutation row and `Ai_Construct`'s forcing put that racer in slot 8. | **Ported 2026-08-10**: eight craft take the grid on any track with an authored slot, every one within **2.40 units** of where the original puts its own, the residual near-constant and dominated by our anchor being 1.68 out. `our_grid_is_the_originals_grid` pins it against the original's own eight positions. **The sign trap cost a pass**: the lateral offset goes along the craft's *left*, because the original's row 0 is left where `Body::right` is right - measured `(-0.006, -0.008, -0.9999)` against our `(0, 0, 1)` on the same grid - and taking it the other way put the front of the field off the outside of the first corner. No unit test on an identity node can catch that; `the_whole_grid_lands_on_the_track` did, by finding a slot with no collision surface under it. Two sub-unit departures are stated on the page: the original's grid follows the track's curve and ours is straight offsets from one node, and each of our slots is re-dropped onto its own footprint. **Nothing drives the opponents and they all wear the player's hull.**
| **The AI is authored XML, and its units are the only thing blocking a port of the original's numbers** | 2026-08-11, [ai.md](docs/gameplay/ai.md) - which carries the schema, the recovered-versus-ours split and the design this project adopts instead, and is the page to read rather than this row. Both games ship their AI as tuning data: `Data\XML\AIControlStats.xml` is a lookahead steering controller with a cross-track term, per speed class, and **the same shape as `oag-trace plan`'s pure-pursuit steerer**, so the lateral half is close to built. `AIRaceStats_<class>.xml` (Pulse, four files) and `AIRaceStats.xml` (Pure, one) are a speed *schedule*, not a driver. **The finding that matters for the config**: Pulse spreads the player coupling across **two** blocks, `RubberBanding` (gap-keyed) and `PosBalancing` (keyed on the player's race position), and only the first is named after it - a switch gating the named one leaves the other running. PS2 corroborates the Pulse files field for field. **The parser is now read end to end and named** - nine functions at confidence 88, `AiStats_LoadAll` (`0x08835830`) down to `AiStats_ParseController` (`0x08836548`), evidence on [ai-stats.md](docs/ghidra/functions/psp-pulse-usa/ai-stats.md), rows in `names.tsv`, program saved. **The per-class record is `0x10c` bytes and its layout closes exactly**: four abutting eight-entry arrays (`BaseThrust` `+0x28`, `StartBoost` `+0x48`, `AIThrust` `+0x68`, `SpreadDist` `+0x88`), six rubber-banding scalars at `+0xa8`, and three `0x18`-byte difficulty records from `+0xc4` ending precisely on the `0x10c` stride the file parser independently multiplies by - two numbers from different functions meeting on the nose. **Two findings that fall out of it.** (1) `SkillScalePoint<N>` and `AIPackSwapping<N>` are **one struct** at the same index and stride, so pack swapping is part of the difficulty setting rather than a system beside it. (2) **`BaseStartThrust` is authored in every shipped file and never read** - the parser compares against the literal `BaseThrust`, exactly, while the parser's names match the shipped names character for character on all fourteen *other* attributes (confidence 85). So only `StartBoost` actually varies per grid slot. Checkable by reading `+0x28` after load under PPSSPP. Also: the `VectorStats` branch discards its own comparison result, so a file containing that element would write below the array - unreachable with shipped data, and consistent with Vector having been dropped from Pulse's AI data. **Still unread**: the *units* (the parser stores every attribute and uses none, so this needs the consumer, which is **not identified**), the sign convention on Pure's `Position`, which of Pure's **two** per-class blocks (legacy versus `<NewStats>`) is live, and `ControllerPO`'s consumer - hypothesis is the Autopilot pickup, confidence 40, deliberately not acted on. Also unlooked-at: `Data\XML\WeaponAIstats.xml`, which `AiStats_LoadAll` does *not* load, so it has its own loader. **Pure is the richer document** - zone curves of speed against distance-to-player, and a fatigue rule that caps how long a craft may hold station near the player - and Pulse reads as a cut-down of it, with the player-position gradient reduced several-fold. The fatigue rule is the one piece worth taking forward, because it is not player-coupled in the objectionable way. **The nine renames are saved in the shared Ghidra database and their `names.tsv` rows exist only on the `ai-recovery` branch**, so if that branch is abandoned a fresh import reproduces nothing - land it or revert the renames. |
| **The AI drives, and what it does not do is the list that matters** | 2026-08-11, [ai.md](docs/gameplay/ai.md) - which carries the whole account and is the page to read. `oag-ai` exists: `Line` (world-space points, closed, built by the caller), `Tuning` (ours, no shipped value anywhere in the tree) and `Driver` (one `u32`, `Copy`, **on `Ship` and therefore inside the world snapshot**, which is what makes a driven race replay). The law is a speed-scaled lookahead plus a cross-track term, damped on the craft's **own yaw rate about its own up axis** - not a finite difference of the error, which divides by `dt` and amplifies a one-tick wobble sixtyfold, and not world up, which is wrong the moment the track rolls. Throttle is bang-bang against `sqrt(lateral_accel / curvature)`. **`Mode::SingleRace::has_opponents()` now returns `true`**, so a grid is reachable from the menu rather than only through `Options::opponents`. **The verification split is the thing to know before trusting any of it**: `race::tests`' synthetic craft is built on `Handling::ZERO` with an empty collision world, so *no force law runs at all* and those tests can only assert that the chosen controls reached the physics - they say nothing about whether anyone drives. The two tests that do are `#[ignore]`d ground truth needing a disc image: `the_ai_drives_the_field_along_the_track` and `a_driven_field_replays_identically`. **Neither has ever been run** - this work was done in a worktree, where `data/` does not exist, so run `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all` from a checkout that has the images before believing the field gets round. **The first version drove wall to wall, reported from the running game.** Cause, and it is structural rather than tuning: `engine::steering` feeds the **torque** accumulator, so steering commands yaw *acceleration*, and `ramp_steering` lags the input on top - proportional feedback on line offset around a double integrator is a limit cycle at any gain that corners usefully. Fixed by closing the loop one derivative in: pure-pursuit curvature -> target turn rate -> error against the craft's actual turn rate. Three things went with it - the separate cross-track term (the aim point already carries the offset, so it double-counted), the snapped aim point (interpolated now; ~2.5-unit sample steps were injecting steps into a high-gain loop) and the steering-triggered inside airbrake (a single airbrake yaws the craft, closing a *second* loop around an oscillator). `crates/ai/tests/closed_loop.rs` runs the real force law on a flat plane and was **written first and watched failing at 84.1 units of peak error from the line; it is 7.3 now**. Counting line crossings was tried as the metric and discarded - 19 against 16, because a craft tracking tightly still crosses at every corner entry - so the assertion is on amplitude. **Two traps paid for.** (1) A three-point curvature estimate divides by the distance between the chords' *midpoints*, which is half their total length; dividing by the whole reads a circle as half as curved and lets every craft into every corner 1.41 times too fast. Pinned by `curvature_approximates_one_over_the_radius`. (2) Each opponent needs **its own** `Environment` - sharing the player's flies seven craft against the player's piece of track, which is what the old "they do not move" comment in `race.rs` was warning about. **Pads landed 2026-08-11 after a play report**: both classes now cross every craft, each racer keeping its own broadphase row and its own pad edge, while a weapon pad's *refresh timer* stays shared because it belongs to the pad. An opponent draws from the shipped table's `ai` column rather than `human`. What it then *does* with a pickup is pure invention and says so - Turbo fired where the driver is not braking, everything else absorbed - because the original decides it in `Data\XML\WeaponAIstats.xml`, a sixth AI file `AiStats_LoadAll` does not load and whose loader has not been looked for. So **an opponent never shoots at anybody**: nothing picks a target. **Lap counting per craft landed 2026-08-11**: `oag_race::Standing` (lap, distance round the circuit, finish tick) sits on every `Ship` and so inside the world snapshot, `Race::places()` returns all eight positions and `player_place()` the player's. **The lap rule is not written twice** - `Standing` reaches `wrapped_forward` and `LapGate::advanced` in `oag_race::state`, and the player's displayed lap is *assigned from* their standing rather than counted again; two counters that disagreed would put a craft in a position it is not in. `RaceState` stays the player's clock, best lap, Zone counters and finish condition. Ordering is finishers by finish tick, then racers by distance, then **by slot index** - that last rule matters because eight craft on the grid are at identical distance before anyone has a fix. **Unbuilt and deliberate**: the skill vector, mistake injection, difficulty selection and the `[ai]` config; a lap *time* per craft (only the player has a clock); a HUD that shows a position, so `places()` has no reader outside the tests; an exhaust plume per craft (there is one `Exhaust` and it is slot 0's, so an opponent gets a pad's force and no visual); per-craft liveries. **Craft-to-craft collision is not an AI gap and does not exist for anybody** - an opponent is stepped against the same `CollisionWorld` as the player, so walls, floors and reset zones all work; what is missing is the two-body path, and the pair resolver `0x0884ef30` plus the writer of the pending impulse at `entity->0x4c + 0x110` are both unread. |
| **`psp-pulse-usa`'s import carries unrelocated address constants, at least around `0x08835830`** | 2026-08-11, found while reading the AI parser. The decompiler prints `0x276c70` where the string is at `0x08a7ac70` - **add `0x08804000`**. This is the same wart [the Pure re-import row](#open-threads) records as a consequence of analyse-then-rebase, now confirmed on the USA Pulse import as well. **The consequence is worse than cosmetic: the call graph does not resolve in that region.** Every call decompiles as `func_0x000318a4` rather than as a named function, `get_xrefs_to` returns nothing for any string or data address there, and `get_function_callers` returns nothing for functions that certainly have callers - all three read exactly like "this is dead code", which is what stopped the AI consumer hunt. **The technique that does work**: take the rebased address, subtract `0x08804000`, and `search_instructions` for the low half as an operand substring - `Data\XML\AIControlStats.xml` at `0x08a7ac70` becomes `0x276c70`, a single `addiu` hit that anchored the whole parser chain. Whether the whole program is affected or only some regions was **not** established, and it matters: prior pages cite call sites freely, so either they were read another way or the damage is local. Worth ten minutes before the next person concludes a function has no callers. |
| **Nothing airborne has ever been captured** | `grip_air`, the airborne pitch gain and the `-0.3` airborne weathervane have no runtime leg at all. One capture with a real jump in it closes several at once. |
| **Task #33: `oag-trace` cannot exercise the mag-lock hold** | `replay`/`drive` take one `Environment` for the whole run and the track samples change per tick, so a replay's blend is 0 by construction. Needs per-tick locator plumbing. It carries the locator-fidelity lead too: the hold explains 49.5 % of the inverted-section residual with nothing fitted, and the remaining magnitude points at the *locator* - our 4-per-segment resampled spline may not be the original's evaluated curve. |
| **The hover probe takes only the deepest hit** | Eight probes, one contact, and no `cross(r, impulse)` angular response. [`oag-trace.md`](docs/tools/oag-trace.md) dates when it bites: the two sides agree to a unit for the first 75 ticks, and what fails afterwards is *staying on the track* - `grounded` dropping to `0.5` and then `0` while the original never leaves `1.0`. That is the named next piece of contact work. |
| **Is there a fifth handling class?** | [`handling-stats.md`](docs/formats/handling-stats.md) resolves `VECTOR`/`PHANTOM` ordering but explicitly does not answer whether a fifth class exists anywhere in Pulse - Wipeout HD's ladder does begin at Vector. Open question, not chased. |
| **Three places still call `talons-junction-time-trial-lap.csv` a lap** | It is not one: measured off the capture, it stalls and reverses. `crates/physics/src/wall.rs:225` and [`oag-trace.md`](docs/tools/oag-trace.md) both describe it as a whole lap and were deliberately not swept when that was settled. The one thing the file *is* good for is the start line - its first frame resolves to the same ring point the lap counter starts a lap at, which is the strongest check `Course::START_LINE_OFFSET` has. No exit-criterion argument may lean on it. |
| **The 13 % roll-stiffness gap** | The recovered tensor runs the roll oscillator 13 % stiff against measurement. Untouched by the downforce fix by construction (the probes are on the centreline), so it is now the only gap of its kind. |
| **Craft-to-craft collision is implemented; the stun is still not armed** | 2026-08-11, after a play report that craft passed through each other. `Body_ResolveContactPair` (`0x0884ef30`) - the function this row and [contact-response.md](docs/ghidra/functions/psp-pulse-usa/contact-response.md) both listed as unread - **is read, named at 80 and reimplemented** in `oag_physics::pair`, resolved once a tick over every pair after every craft has been stepped. **The finding to carry: a craft bounces off another craft with `e = 0.1` and off the track with `e = 0.4`, in the same build.** The pair resolver hardcodes `-1.1` where the one-body path reads the per-body `0.4` the ship constructor writes; it also has a separating-velocity gate (`vn - 0.5 <= 0`) the one-body path does not, applies **no friction**, and corrects position by a mass-independent quarter of the overlap each way. The PS2's pair resolver carries the same `-1.1`, so the two builds agree about pairs while the PSP disagrees with itself across its two paths - a third instance of the `0.4`-versus-`0.1` trap that page already warns about. **The detection is ours and there is nothing to recover**: `0x08815ccc`, the box-against-box narrowphase a pair of craft dispatches to, is **`jr ra; nop`** - two instructions, reports nothing, Ghidra does not even claim it as a function (confidence 90). Craft are box proxies, so **no pair of craft can produce a contact through `Collision_StepNarrowphase`**, and what actually feeds `Body_ResolveContactPair` two craft is unfound - which positively excludes the narrowphase from the hunt for the pending-impulse writer rather than leaving it merely unexamined. `pair::overlap` therefore tests an oriented box against an oriented box over the hull's own `<Misc width height length>`. It started as a sphere of half the hull's diagonal and that was far too big - 4.58 units on a 4x2x8 hull where the flank is 2 away, so craft shoved each other while visibly apart, reported from play within an hour. **The box was still too big, reported again**, and the reason is worth keeping: **`<Misc width height length>` is a bounding box, not the hull.** A real Pulse craft measures `5.5 x 3.5 x 13` and half that length, `6.5`, lands within a whisker of the `6.45` bounding radius `oag-view` reports for the shipped Feisar mesh - so the authored box bounds the model rather than tracing it, and a hull that tapers toward the nose collides along its whole length at the width of its widest point. `pair::HULL_SCALE` is the knob; it is ours with nothing behind it, and only play sets it. Tests are written against the constant, not against literals. **Also worth knowing: `data/` can be symlinked into a worktree** (`ln -s <checkout>/data/images data/images`, same for `extracted`) - it is gitignored, so nothing is committed, and it turns every `#[ignore]`d ground-truth test from unrunnable into a one-command check. Symlink the *subdirectories*, not `data/` itself, which holds a tracked `README.md`. **The stun is still not armed**, and the reason is now narrower: the two calls at the tail of the pair resolver, at unrelocated `0x499f8` and `0x49c60`, are the obvious candidates for posting the pending impulse at `entity->0x4c + 0x110`, but rebased they land *inside* `Ship_UpdateCraft` rather than on a function start - so either Ghidra's boundaries are wrong there or the rebasing rule that works everywhere else on that page does not hold for them. Not guessed at. |
| **Ship-to-ship collision stun** | `stun_timer`, its constant and the engine gate all exist and nothing arms them, deliberately - track contact does not arm it in the original. Ship-to-ship needs the writer of the pending impulse at `entity->0x4c + 0x110`, plus `0x0884ef30` (the two-body resolver) and the `+0xb8 == 6` zero-friction entity class, all unread. |
| **The barrel roll is identified and unimplemented** | A headline Pulse mechanic we do not have. The tap-history chain is read link by link at confidence 80 in [`input-bindings.md`](docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-tap-history-path-is-the-barrel-roll). A follow-up needs two unread things: `FUN_08840770`'s energy cost, and what consumes `craft+0x1c0 & 0x400` - presumably the visual roll, which would decide whether the phase is an angle. `DAT_08b36bf0`, the ramp rate, is `.bss` and needs a live read. |
| **Sideshift has no runtime leg** | Both control schemes are implemented and `sideshift-flick.inputs` covers the novice gesture. **No capture of either gesture off the original exists**, so every timing is static analysis; the recipe is on the page. Note the scheme resolves once at startup and applies to the next race, not the running one. |
| **Task #38: the penetration-escape surface gate** | The original gates escape on surface type 1 only; the crate uses 1-or-3. Noticed, not acted on. |
| **Task #31 residual: `race.rs`'s unguarded `slice(..)`** | `oag-view --collision`'s panic on an empty vertex buffer is fixed. `crates/game/src/race.rs:2764` has the same unguarded slice, left alone deliberately: a track always has collision geometry, so guarding it would be speculative. The `orbit` guard is the one with no coverage - it needs a window. |
| **Menus: rebinding is the one thing that does not work** | The shell navigates and every other row is live ([menus.md](docs/architecture/menus.md)). Separately, **escape backs out of a race into the menus, and that is deliberately *not* a pause menu**: the `World` is dropped rather than suspended, so re-entering loads a fresh race. Suspending it is the remaining work, and the menu stage already builds its own renderer so it can be opened from somewhere that is not the front end. |
| **The ESC rewiring has no live-keypress check** | The unit-testable half is tested; `main.rs`'s wiring needs a window and a real keypress, and no key-injection tool (`wtype`, `ydotool`) was installed on the machine it was written on. **Press it once by hand**: `just play`, reach a race, escape; then `just play --race`, escape. Two things it would catch that the compiler will not - a held escape walking out through key repeat (guarded by `!event.repeat`, unverified), and re-entering the menus mid-race leaving a stale title or cursor. |
| **Nobody has *watched* the menu backdrop** | It is verified as a loop, including the wrap and the reopen, but menu -> race -> escape -> menu was verified by forcing a second `open_menus()` rather than by pressing escape, for the same missing-key-injection reason. `just play` and look at the background for twenty seconds. |
| **MONITOR has only ever run on a one-screen machine** | The *miss* path is covered; the working path is not. A wrong scale factor offsets the window by the difference on any screen not at 100 %, and it centres on the *inner* extent while setting the *outer* position, so a decorated window sits high by about a title bar (known, documented, not corrected). **Try it on two screens**: `just play`, OPTIONS -> DISPLAY -> MONITOR, windowed and borderless. A tiling compositor may refuse all of it. |
| **Do the game's capture paths need encode-on-write?** | Answered "no" by [ADR-0020](docs/architecture/adr/0020-gamma-authoritative-colour-space.md) for the colour-space question, but the front end's **authored text and fill colours** were tuned against a window and there is no measurement of the text case the way `render.rs` measured the sprite case (sprites are unlit, so that answer was clean). Settle it with a window-versus-capture comparison of a text-heavy screen before changing anything. |
| **FSR 1's default is open** | `oag_render::post::fsr1` transliterates AMD's MIT `ffx_fsr1.h`; the route for FSR 3.1 is settled in [ADR-0012](docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md). It cannot reach the front end because `capture::run`'s front-end path has no `Framebuffer`, and giving it one is the same work as the UI-compositing restructure ([modern features](docs/overview/modern-features.md) has the prerequisite table). **Do not** read a menu capture taken with `--upscaler` as evidence either way: the flag changes the UPSCALER row's own text, so the two images differ for an unrelated reason - that nearly produced a false conclusion. |
| **Two anti-aliasing residuals** | MSAA and FXAA/SMAA are in; TAA is deliberately not. (1) `Entry::warning` is a single `Option<Warning>` and cannot express two independently-triggered warnings without restructuring that type, so the 200 % render-scale case has no live warning. (2) There is no `--anti-aliasing` CLI flag, unlike `--upscaler`/`--render-scale`; add one the same way if an AA comparison capture needs it. Note **MSAA 2x fails wgpu validation here** - `sample_count 2` needs a device feature this project never requests. |
| **Front-end gaps behind `Image`** | `screen.rs` reads only a screen's direct children, so `Show Logo`'s `BOOT_LEGAL` line is absent - now a *guarded* absence. The fix is not a scissor rect: the `Viewport` around it clips nothing vertically, and what the line needs is `widthlimited="true"`, i.e. wrapping a 118-character string, and `Draw::Text` has no width or wrap. That is text layout. USA disc only - EU drops both. An `Image` with no `x` is centred on a guess. |
| **Pure's dev/pub hold duration, and how the original picks a regional cut** | The two things still unread after `Developer Publisher Screen` was traced to the dev/pub reel (see [pure-boot.md](docs/architecture/pure-boot.md)). **(1) The hold duration.** `144`, `231` and `260` are three `li` immediates in Pure's own `BOOT.BIN` - the only such site in 3.6 MB, at VA `0894b70c` under PPSSPP's `0x08804000` base - each starting an identical pause-and-reload block. The duration itself is *not* an immediate: the block loads a float from `*(base+0x53d68) + 0x44`, so `oag_pulse::frontend::HOLD_SECONDS = 2.0` remains Pulse's measurement, imported. Two ways to settle it - read that global, or re-film the screen at **0.1 s** sampling and count consecutive identical frames on the first plateau (about 22 for a 2 s hold, at most 2 for none). Also unconfirmed: that `0894b70c` is reached *from* the `Developer Publisher` screen-type handler - it is the only candidate, which is strong, but it is inference rather than a traced call path. **(2) Region selection.** All four cuts are now named (below); what the original selects between them on has not been read out of any binary. |
| **`Movie::entry_name` hardcodes `_US`, so a European Pure disc shows the American card** | `crates/game/src/screen.rs`'s `entry_name` appends `_US.PMF` to **every** `localised="true"` widget on every source. That was defensible while `_US` was the only known name; all four cuts of both Pure movies are now recovered and sit in `oag_pure::names::INTRO_MOVIE_CUTS` / `FMV_INTRO_MOVIE_CUTS` - `_EU`, `_US`, `_JAP`, `_KO`, each verified by resolving it against a real WAD entry. Both Pure pressings carry the first three; the Korean cut is on the EU disc only. Frame 144 of the `_EU` cut reads EUROPE where `_US` reads AMERICA, so booting `pure-psp-eu.chd` through this build currently shows the wrong regional card. The fix wants a region to resolve against; the mechanism the original uses is the open half above. **Side effect worth banking**: these names close the question at `docs/ghidra/functions/psp-pulse-usa/frontend-video.md`'s "no name produces those" for all three Pulse-side reel hashes, and `oag_pulse::names::DEVPUB_REEL` is now the name rather than `hash:b1ba72c3`. They are owed to a `names.tsv` - but that file is `psp-pulse-usa` only, and these are entry names rather than symbols, so decide where they belong before adding rows. |
| **`--until` cannot reach a late movie frame on a machine with an audio device** | The movie is clocked by its own sound (ADR-0019) and a headless capture runs far faster than real time, so the playhead only ever advances by the wall-clock seconds the process is alive - a few - while `--until` spends its 3600-tick ceiling and fails with "never reached X". **Every `--screenshot` of a movie leg is therefore an early frame**, which cost this session two wrong readings: a near-white reel frame and a black intro frame were both taken as evidence that a movie was not rendering. `crates/game/src/audio.rs`'s `movie_playhead` already half-documents the cause ("the mixer has to be **moving** as well as sounding"). It would work on CI, which has no device. A `--no-audio` flag, or letting `--dump-audio` drive the mixer deterministically when a device is attached, would make movie legs screenshot-able; until then use a windowed run to check anything that plays. |
| **Four `oag-trace` ground-truth tests fail on a clean tree** | `wall_contact_ground_truth` (three) and `lap_capture_ground_truth` (one), under `cargo nextest --run-ignored all`. Not a code regression: they fail identically on `main` at `442ebf8` with nothing checked out on top, so **`just test-data` is not a clean gate right now**. The cause is `data/traces/` fixture drift - `the_capture_stalls_and_reverses_rather_than_completing_a_lap` asserts the capture does *not* contain a complete lap and it now reads one at tick 3087, which is exactly the "if it has been recaptured as a real lap, this test and everything citing the file need rereading" case the test's own message names. Either the capture was replaced or the reader changed; decide which, then re-read the pages citing it. |
| **Where Pulse's language picker belongs is unevidenced** | A cold boot of `pulse-psp-eu.chd` (2026-08-10, frames under `data/shots/pulse-cold-boot-2026-08-10/`) opens straight into `LogoFMV` playing `Data\Movies\Intro.PMF` and runs on to `Show Logo` **with no picker in between**. This build puts the picker between those two. The movie-first half is now confirmed correct - it had been recorded as merely "the order asked for" - but the picker's own position is not: what the disc does with no language saved was not observed, and the run had a savedata profile in place. Re-check with `UCES00465P0000` moved aside, the way [pure-boot.md](docs/architecture/pure-boot.md) records for Pure. Note Pulse's own `Skin.xml` *declares* the picker first and its runtime does not, which is the measurement ADR-0023 turns on. |
| **Pure's string tables are not read, and its front-end font is absent** | Two separate gaps a Pure boot prints and nothing chases. `German names no string table` - `load_strings` finds no entries for Pure's language plugins, so every `idstring` falls back to its own id. And `Data\FE\Fonts\pulse_text.fnt` is not in Pure's `Data.wad` (hash `f55e014c`), so the whole front end draws in the built-in 5x7 glyphs rather than the disc's own font - which is why Pure's picker looks blocky beside the PPSSPP capture. Neither blocks the boot sequence; both make every Pure screenshot a poor likeness. |
| **`boot.rs` still applies five Pulse literals to every source** | Scoped out of the ADR-0023 pass deliberately, and listed so the next one does not have to re-derive them: the front-end root (`Data\Plugins\PI001\GUI\Skin.xml`), the font set, the language-plugin ids, `Data\Plugins\PI001\Definition.xml`, and `load_teams`' fallback to `oag_formats::handling::TEAMS` (Pulse's eight, on a source with nine). The first four are *identical* on both discs - `oag_pure::names::GAME_PLUGIN_DEFINITION` already restates one verbatim and is unused - so routing them through the boot profile would add indirection with no behavioural difference today. The `TEAMS` fallback is a real divergence but unreachable, Pure never reaching a race. Real ADR-0022 debt; move them when a third title or a Pure race forces it. |
| **Pure's `Title Screen` is missing its own logo wordmark and most of its frame-line decoration - not chased further** | Comparing `oag-game`'s own render against the real PPSSPP capture: the big "WipEout"/"pure" logo (`TitleFrame`, `x=0 y=76 width=480 height=128`, `StartEnabled="false"`) never gets a `src` in `Skin.xml` at all - it is set programmatically on the original, the same way Pulse's own dangling globals are, and finding it needs either Ghidra (where `TitleFrame`'s texture gets assigned) or a full image-content scan of `Data.wad`/`FE.wad`, not a name guess - one guess (`Data\FE\Images\Logo.mip`, hash `be900df4`) did resolve to a real entry but at 2064 bytes is far too small to be a 480x128 texture, so it was not pursued. Separately, `Title Screen` authors five `Animation`-driven corner-line `Image`s sharing one source (`Data\FE\Images\FETextures_startscreen.mip`) at different `U`/`V` sub-rects - `Image` (`crates/game/src/screen.rs`) does not capture `U`/`V` at all, only `x`/`y`/`width`/`height`, so all five collapse onto whichever one `draw_backdrops` happens to place, drawn at the wrong position. Confirmed this is not new: Pulse's own `ArrowSelect` widget authors `U="53" V="0"` and would have the same problem, unconfirmed whether it is visibly wrong there too. |
| **The `.pob` record's own field layout** | The container, the `SYSP` slot table and the names are decoded and corroborated on a second binary; a resolved record's fields are not. The preload path-string addresses `0x08a886f8`-`0x08a8877c` are the untried xref target. |
| **The `.vex` class-ID table's extent** | All ~55 game classes are transcribed into [vex.md](docs/formats/vex.md) and `vex::CLASS_NAMES`, self-validating at 95. The walk stopped at `0x08ab26a0` without reaching the `id == -1` terminator, so generic Maya classes past `0x3eb` are only partly covered. Cheap to finish; nothing depends on it. |
| **One team's `Ship.vex` does not resolve by name** | Was five. Four of them - `Auricom`, `Harimau`, `Icaras` and the team sold as `Mirage` - are **downloadable content**, not missing names and not PS2-only: their ships are in the four Pulse DLC packs, and the fourth's folder is `Mantis`, which is why no spelling of `Mirage` ever hashed. See [`docs/formats/dlc-pack.md`](docs/formats/dlc-pack.md). `Van_Uber` remains, and is a *Pure* team rather than a Pulse one. The trap that cost a wrong conclusion once still stands: the path templates resolve with the **FE team-model name**, whose default is the literal `"ship"` - hence `shipwreck.vex`, *not* `Assegaiwreck.vex`. |
| **The airbrake flap rotation axis is chosen, not recovered** | The flaps deploy, and the reason to know this before retrying: the node's payload is zero bytes, its class descriptor carries no handler pointer, no `0x3c5` immediate exists in the binary, and `Vex_FindClassDescriptor`'s only two callers are load-time. **There is no per-class function to decompile** - the way in is a live read under PPSSPP, watching which node matrix moves when the airbrake goes down. |
| **A model built from several small pieces sharing one atlas** | The delta-(-1) directory rule now covers ships (11 of 11 exact) and track art (27 of 32 exact, the rest short 1-2 slots), and `oag_game::race::load` tries it for both. The shared-atlas case is the separate, harder problem the disc-wide 535/975 figure is really about - see [`ps2-texture.md`](docs/formats/ps2-texture.md#how-a-model-finds-its-texture-set-directory-position-not-a-name). |
| **Which movie cut plays, and what plays the three 260-frame reels** | The reel set is region-invariant, so the disc's region cannot be the picker, and nothing in the boot path plays them at all. [`pmf.md`](docs/formats/pmf.md). |
| **The PS2 PAL/NTSC selector's ultimate trigger** | `Movie_ResolveSourcePath` picks on global `0x0027a85c`; what decides that value (a numbered case in dispatcher `FUN_00186ed8`) was time-boxed away deliberately and nothing needs it yet. |
| **Two PS2 identifications, one of which is wrong** | `0x0015d058` zeroes the same four accumulators as the already-named `Body_ClearAccumulators` (`0x0015ca48`). Left unnamed per ADR-0005; cheap to resolve. |
| **The SAP clamp globals** | Both are all-zero in shipped `.data` with no writer anywhere in the image (xrefs, operand scans over all 635,898 instructions, `.ctors` - all checked), so taken literally the clamp is degenerate. Confidence 45, and nothing depends on which way it resolves. |
| **SteamOS's own glibc version** | Unestablished. `just appimage-portable`'s floor is `GLIBC_2.34` (`objdump -T` and `readelf -V` agree, and the determinism test passes inside that container, so the two builds are interchangeable for the simulation). If `GLIBC_... not found` ever appears on a Deck, the version it names is the missing datum. [`packaging.md`](docs/tools/packaging.md). |
| **The chase camera's 3/4 factor** | The eye sits at exactly 0.75 of the authored external offset, at rest and at speed alike, and where that factor comes from was not found - do not hardcode it. [`camera.md`](docs/ghidra/functions/psp-pulse-usa/camera.md) has the samples. |
| **The loading screen has no caller** | The rippling band is procedural and the `loading` plugin holds no video; the decode is pinned against the real blob (which is **swizzled**, bit 0 of `+0x07`, and **fully opaque**, so both plausible wrong readings are excluded by construction). **Nothing is runtime-verified against the original, and the game does not draw a loading screen yet.** |
| **`engine_fire` and `exitglow` are authored but never registered** | Neither has one of `Vex_RegisterClass`'s 46 call sites, yet `exitglow` is authored 13 times on `16_Track` and `engine_fire` on zero of the disc's 340 VEXX files. Same shape as `gate`. Closed as an exhaust question - unregistered *and*, for `engine_fire`, uninstanced. |
| **Trackside texture animation draws from the authored data and is on by default** | *(Closed 2026-08-11; was "implemented and switched off".)* Each animated material carries its own keyframed `TEXSCALE`/`TEXOFFSET` block and the renderer replays it, so `[graphics] animated_textures` defaults on and off now means "freeze at the first key" rather than "do not depart from the original". The global scroll clock that was unfound never existed - there is no global clock, each track wraps on its own authored period. `oag_pulse::textures::ANIMATED_TEXTURES` is kept as the record of the geometric survey and **no longer draws anything**; where it and the authored blocks disagree, it is the wrong one (it scrolls `col_display7_GLOW` in V, the disc authors U). Two traps the port paid for, both now in [vex.md](docs/formats/vex.md): the block's `+0x2c` is the loop period and **not** the last key time (`16_Track`'s flicker families end at frames 12/18/24 and all loop at 50), and the blocks are **per material** with a later block's key offsets relative to the array base rather than to itself. [texture-animation.md](docs/ghidra/functions/psp-pulse-usa/texture-animation.md) still records that **`Gfx_BindTexture`'s `0.2` threshold is a VRAM re-upload heuristic, not an animation clock**. The reader gates on the material's `& 0x10` flag, which is the engine's own gate and is **load-bearing rather than belt-and-braces**: a `Skycube` payload is a `Mesh` payload, arbitrary bytes read as a plausible block often enough to matter, and a sky that picks one up slides the whole horizon. |
| **Vertica's sky really does drift, and it is the only one that does** | Found 2026-08-11 while asserting the opposite. `06_Track`'s `Skycube` (both layouts) moves its texture one whole tile **diagonally** on a **33.3-second** loop, over 71 of its 494 vertices - a cloud layer, and the slowest authored track on the disc by a factor of three. The other 39 shipped skies author nothing. Recorded because "the sky is static" is the natural assumption and this project's own test asserted it before the data said otherwise. [skycube.md](docs/formats/skycube.md). |
| **`--race --screenshot` renders a white scene, and it predates this work** | Noticed 2026-08-11 while trying to verify the animation port end to end; reproduced on `HEAD` (`3a947e0`) in a clean worktree, so it is not the port's doing. The HUD draws, the craft state advances normally (`spline 10.06`, `speed 0.01`), and the scene behind it is blank white at ticks 200, 400 and 900 alike. The mesh pipeline itself is fine - `oag-view --mesh` renders `16_Track` correctly through the same shader. Unattributed; **it makes headless race screenshots useless as visual evidence** until someone looks. The working substitute is `oag-view --mesh ... --anim-seconds`, which is why that flag exists. |

## Pending maintainer decision: shipped design data in tracked docs

[`handling-stats.md`](docs/formats/handling-stats.md)'s own rule, per
[ADR-0006](docs/architecture/adr/0006-no-copyrighted-content.md) - "a field name
is a description of the format, a tuning table is the content itself" - is
breached in places by earlier passes. `angular-velocity-column.md`'s shipped
`Turning.amount` has been redacted, but it still quotes the Assegai `Misc` hull
dimensions in its box-tensor refutation, and `force-balance-ground-truth.md`
carries XML-side values from the force-law derivations (`accelcap`, an
`amount`-derived figure, `ride_height`, `Physical.mass`), some of them
load-bearing in the arithmetic.

**Whether to sweep those historically is a maintainer decision, not an
agent's**, which is why it is flagged rather than done. Git history retains
everything regardless, so a sweep would clean the distributable snapshot, not
the past.

## Scope decisions that look like gaps

| Decision | Why |
| --- | --- |
| **The Memory Stick and profile system** | One automatic save slot, the way a modern PC game does it. Most of the disc's boot chain between `Language Selection` and the menus is deliberately out of scope. |
| **PSP over PS2 where they diverge** | User directive, now policy in [goals.md](docs/overview/goals.md#scope). PS2 stays the corroboration leg for confidence, not the target. |
| **The collision stun is not armed by track contact** | The original's is gated on a pending impulse the contact path never writes. |
| **Sweep and prune is not reimplemented** | The original's packing clamps world space rather than rebasing it, so it never drops a genuinely overlapping pair; ours does the same job without the packing. |
| **The four-corner hover variant** | Not implemented. Its selector is known (`DAT_08ab07e3 == 0 && DAT_08b31048 == 6`, which also disables the brakes and the weapons) and it is not the racing configuration. |
| **Pure asset work** | No longer deferred as a whole. [ADR-0022](docs/architecture/adr/0022-title-packages.md) opened the format and asset layers on the strength of [`pure-status.md`](docs/formats/pure-status.md)'s measurements; ADR-0009 item 2 still defers Pure *simulation* work behind M4's exit. |
| **PS3 and Vita content** | Paused 2026-08-09, on the toolchain rather than on the research. Both are encrypted - see [`data/README.md`](data/README.md#the-ps3-and-vita-images-are-encrypted-and-nothing-here-decrypts-them-yet) for the two different mechanisms and the tools each needs. None of `PS3Dec`/`scetool`/`pkg2zip`/`psvpfsparser` is installed and there is no RPCS3 install to borrow a decrypted copy from. The one architecturally relevant fact was free: **HD/Fury and 2048 both ship PSARC, not WAD**, and 2048 arrives as a PKG rather than a disc filesystem. |
| **A "CPU renderer" switch** | There is nothing to switch. wgpu ships no software rasteriser (`Backend::Noop` draws nothing), so CPU rendering exists only via a system Vulkan implementation such as lavapipe, selected outside the process. |
| **RENDERER applies on the next launch** | The device is made once at boot and everything hangs off it. Live switching means tearing down the surface, the pipelines and every GPU resource. |
| **PS2 `.PSS`/`.IPF` through GStreamer** | [ADR-0017](docs/architecture/adr/0017-gstreamer-native-video.md). |
| **`just play`'s default is not `oag-game`'s default** | `native-video`'s Cargo feature is off by default, so `cargo build`/`cargo test`/CI stay GStreamer-free; `just play` turns it on. |
| **GStreamer for ATRAC3+** | **Do not retry this.** Extending the `native-video` path to audio was the intended route and it cannot work - see [ADR-0019](docs/architecture/adr/0019-atrac3plus-out-of-process.md). |
| **Motion blur is designed but not built** | [`docs/rendering/motion-blur.md`](docs/rendering/motion-blur.md) specifies it fully and costs it at about a week and a half; no code exists. An invented modern feature, not a recovered one. Written down because the per-object velocity buffer it needs is also two of the five things FSR 3.1 and TAA are waiting on, so the design is worth more than the effect. **Two open questions could still change it**: whether `Rg16Float` is multisample-renderable at 4x on the adapters here (probe before writing any shader - `sample_count: 2` already failed everywhere, which is why there is no `Msaa2x`), and whether bloom, which the design inherits its placement and colour-space handling from, is actually correct today - it defaults off and has no menu row, so nobody has looked. ADR-0024 is owed by whoever implements it, since ADR-0013 currently forbids the row. |
| Everything else milestone-scoped | Weapons, AI, netcode, the shell: [roadmap](docs/overview/roadmap.md), not this file. |

## Working rules that were learned expensively

**Most of them now live in `docs/`**, because they are properties of the method
rather than of any pass:

- [Reverse engineering](docs/reverse-engineering/methodology.md#rules-learned-the-expensive-way) -
  decode before reporting, xref counts bound authored instances only, sibling
  pages go stale, never write a fitted constant in as a recovered one, and the
  rest.
- [Reading a capture](docs/reverse-engineering/verification-protocol.md#reading-a-capture-rules-that-each-cost-a-session) -
  control columns first, unclamped ramps, clean-capture detection, ask the
  tangent not the index, Left-Up-Forward, and threshold-versus-frame stability.
- [Measuring a renderer change](docs/rendering/README.md#measuring-a-renderer-change) -
  aggregates rank broken changes higher, brightness thresholds measure the
  circuit, measure a screenshot before citing it.

What stays here is what is about working in *this* repository, with other
writers in it at the same time:

- **`git-commit` does not isolate a pathspec - it commits everything staged.**
  `git add <paths> && git-commit` silently sweeps whatever another writer staged
  in between; it has happened twice. Check `git diff --cached --name-only`
  immediately before every commit.
- **Do not run `just` or `cargo fmt --all` while anything else is editing.** A
  gate run taken mid-edit reports failures that do not exist, and `cargo fmt
  --all` rewrites files another writer holds open. Scope to `-p <crate>`.
- **A doc two parties are editing wants committing promptly**, not left dirty:
  concurrent uncommitted edits are indistinguishable from data loss to whoever
  looks second.
- **If two sessions do run at once, the value is in their *disagreement*, not
  their throughput - and one thing destroys it.** On 2026-08-09 two sessions
  worked this tree simultaneously; every number in that day's four commits was
  reproduced by whoever did **not** produce it, and each caught refuted
  citations the other's sweep had missed, in both directions. What made that
  work is that neither could see the other's intermediate state, so **a wrong
  premise had to survive contact with someone who had not formed it** - which
  is precisely what six days of single-session work could not provide. Two
  sessions that agree on method up front and split strictly by file reproduce
  each other's blind spots at twice the cost. So: split by *claim* and re-derive
  each other's numbers, do not split by directory and trust the reports. And
  note the same day's counter-example - a spawned subagent went idle four times
  without ever producing a report, and its one decisive check had to be run by
  hand. **Parallelism bought nothing that day; independent re-derivation bought
  everything.** Do not read that session as an argument for more agents.

## Traps that are live

**Subsystem traps have moved to their pages**:
[surveying a disc](docs/formats/wad.md#traps-when-surveying-a-disc),
[the Ghidra database versus `names.tsv`](docs/ghidra/workflow.md#the-database-can-disagree-with-namestsv-and-the-docs-win),
[PPSSPP session hygiene](docs/reverse-engineering/ppsspp-debugger.md#session-hygiene),
and the `ffmpeg`/`ffprobe`/GStreamer traps on
[frontend-boot.md](docs/architecture/frontend-boot.md). What is left is this
machine and this checkout.

- `data/` is on `ecryptfs`. **`cp --reflink` does not work** (copying the images
  is a real multi-GB copy), and `ls`/`rg` intermittently fail with "permission
  denied" or "no such file" on directories that plainly exist. `fd`, `find` and
  absolute paths work.
- **Backticks in a justfile are command substitution at parse time**, even
  inside an `@echo` string - an echoed example command once ran a full
  trace-compare on every `just` invocation, `just --list` included.
- **`XDG_CONFIG_HOME` is how you point `oag-game` at a scratch `settings.toml`**
  for A/B config captures without touching `~/.config/oag/`.
- **A niri screenshot piped through `wl-paste` can return the PREVIOUS clipboard
  image.** One lap "verification" shot came out as a menu from an earlier
  session. `psp-trace.py` now detects this and writes `.stale.png` rather than
  citing it, but nothing else does. `niri` screenshots land in
  `~/Pictures/Screenshots/`.
- **`/tmp` here is quota-limited well below the project's own disk.** Extracting
  a large WAD into it once broke the shell entirely - every command failed,
  `echo` included, until the partial extraction was deleted. Scan through
  `oag_assets::Archive` in memory instead.
- **Shipped `section` data is not tidy, and a strict parser refuses real
  tracks.** ~2 % of PVS mask bits name a section the file does not declare, one
  id is authored three times over on four tracks, and 116 PS2 control points
  name a section their file lacks. All benign; see
  [`track.md`](docs/formats/track.md).
- **No capture will ever show the BRIGHTNESS or GAMMA settings.** They apply at
  the blit, which `--screenshot` and the race capture never reach, so a picture
  that looks ungraded is not a bug. The real check is
  `upscale::tests::the_grade_moves_the_picture_in_the_direction_the_setting_names`,
  which builds the actual pipeline on a headless target and reads the pixel back
  (mutation-checked by swapping brightness and the exponent in the shader). It
  **skips with a note where there is no adapter**, so a green CI run does not
  mean it ran.
- **Judge line width at native 480x272 only.** Anything upscaled makes a
  one-pixel authored line a magnification artefact argument rather than a
  fidelity one.
- **`--features native-video` makes the boot fifty times slower, and `just play`
  turns it on** (Linux only, `justfile:25`). Measured on `pulse-psp-eu.chd`,
  caches warm: **4.9 s** of boot with it (intro 2.7, backdrop 2.2) against
  **0.08 s** without. `GstDecoder::open` decodes every frame into memory before
  it returns - ~288 MiB for the two reels - where the AV1 cache path decodes on
  demand. [ADR-0017](docs/architecture/adr/0017-gstreamer-native-video.md) chose
  that deliberately but costed it at 270 frames and 53 MiB, which is the
  *backdrop*; the intro is 1200. **The boot now prints its own timings** (`the
  boot's first half took ...`, `the boot's movies took ...`), so this is
  re-measurable rather than folklore, and it is covered by the loading screen
  rather than by an empty desktop. Making the GStreamer path lazy would remove
  the wait rather than hide it, and is not done.
- **Two things convert into `data/cache/movies` and they must never overlap.**
  The boot's own two reels and the `--prefetch` worker both shell out to
  `ffmpeg`, and two processes writing one cache file is a corrupt file rather
  than a race that resolves. The ordering used to come free from `boot::load`
  being blocking; now that the movies run on `boot::MediaWorker`,
  `Session::start_prefetch` is the only thing holding it, and it is one `if` in
  `Session::frame`.

## Verification status: what to lean on

**Measured against the original running.** The forward force law end to end (a
standing start fits from `fs = 0.56` to `47.67` at rms 0.127 on a one-parameter
fit); lateral grip at 0.9985 of the disc value, 99.95 % explained; the yaw
accumulator term by term at racing speed, 99.94 %; the inertia tensor on all
three axes, pitch to 0.13 % and the `I * omega` identity to 0.01 %; the pitch
step response at 1.006x; the contact friction coefficient as a one-sided bound
approached from above; the mag-lock probe re-finding the magstrip from geometry
that knows nothing about the capture (91 % of inverted poses against 0.3 % of
upright ones); the authored `Start Position` frame, whose forward lands within
**1.12 degrees** of the original's craft at the start line and whose left lands
within 1.54; and the chase camera at **RMS 0.008 world units** over 150 recorded
ticks. Method and numbers in the two ground-truth pages under
[`docs/physics/`](docs/physics/README.md) and in
`crates/game/tests/chase_camera_ground_truth.rs`.

Add the whole-lap result, with its limits stated in the same breath: over the
clean window position tracks to **6.54 units at the worst and 2.41 on average**
on a craft doing 100 units/s, with speed inside 6.01 - but that window is the
first ~170 ticks, nothing after tick 256 measures our physics on the current
capture, and `grounded` is a constant column so agreeing with it says only that
our ship also never left the ground.

**Instruction-level reading with no runtime leg yet.** Mag-lock's blend weights
and its `|h - d| > 5.0` fallback; the swept collision path; everything airborne;
both sideshift gestures. Single-sided wall rejection is measured against the
*data* rather than against the original running: **2,076 of 2,084 wall triangles
on `16_Track` (99.6 %)** are wound toward the nearest point of the track's own
spline, which is what makes reproducing the original's
`dot(boxCentre - sample, n) > 0` rejection faithful rather than merely literal.

**The determinism gate reaches the simulation** (`dd53c1f`).
`oag_physics::probe` steps the same `oag_physics::step` the race loop steps, over
a scripted input and a synthetic corridor built in the file, so it runs in CI on
all three OSes rather than needing a disc image. Two mechanisms keep the hashed
field list from rotting and **both are needed**: exhaustive destructuring makes a
new `ShipState`/`Body` field a compile error, and a perturbation test proves each
field actually reaches the hash rather than merely being bound - that second test
caught a real hole on its first run, since `time_since_landing` defaults to `1.0`
and perturbing it *to* `1.0` was a vacuous no-op. What it does not cover is
stated in its module docs: maglock, reset, and the swept tunnelling path.
`probe::environment` now crosses a speed pad twice, once on each side of the
`<Special speedpad_jump>` tilt threshold, because until it did, step 15 of 15 was
the one force term the gate did not cover at all - and a new branch inside an
uncovered term lands with the hashes *not* moving, which is the worst outcome
available.

**Two standing rules.** When the determinism test fails, find the bug - never
update the reference constants to make it pass. And never raise a confidence
score on the strength of the simulation agreeing with itself.

## Reference

The user pointed at <https://www.youtube.com/watch?v=lGAWYHmgo7o> as a
reasonably high-quality capture of Wipeout Pulse, useful for checking a reading
against the real thing without setting up an emulator capture. It is not an
authority over the disc images and Ghidra: it is a recording of someone else's
playthrough, subject to its own encode and possibly the wrong regional cut.
Whether Pure has an equivalent is not checked.
