# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); this file
covers what a fresh reader would otherwise have to rediscover.

Written 2026-07-27. The pass before this one opened M4: five crates, two formats
decoded, and the control force law recovered. **It was never committed**, and
this pass starts by committing it as three commits (`be5da47`, `9d14acd`,
`ce3a958`) before doing anything else. If you are reading this after a session
that ended abruptly, check `git status` first - a whole milestone's work sat
uncommitted in the working tree for a day.

This pass then opened **M3**, and that is the news: the original can now be
driven and read from a script. Four things that were static readings are now
runtime-verified, and one open question the roadmap tracked is answered.

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

## 2026-07-28, second pass: the M4 resistance blocker is resolved - there was never a missing force

A second three-agent team ran the "next steps" the morning pass left behind:
`hover-re` (the VFPU hover/damping/grip reading, now that the Ghidra bridge is
back up), `captures` (the standing-start and steering captures), and
`mechanical` (collision survey, docs debt, trace migration). `hover-re` and
`mechanical` are done and folded in below; `captures` is still in flight on the
steering side, though its committed groundwork (`7738bf4`) already carries a
finding. `just` passes at **788 tests** after all of it; `audit-leakage` clean.

**The headline (`d7fea3a`, `ef110d8`, `2f26453`, `07411ad`): the force law was
right all along, and both old reference captures were recorded scraping a
wall.** `docs/physics/force-balance-ground-truth.md` now opens with the
resolution; the short version, because it inverts the standing diagnosis:

- All three VFPU candidates for the missing linear resistance are **refuted at
  instruction level** with the Allegrex module: `Ship_UpdateHover`
  (`0x0884870c`) is a twenty-instruction *dispatcher*, and the real spring in
  `Ship_HoverTwoPoint` damps via a multiplier on the spring magnitude along the
  ship's **up** axis, with the normal projection present at `0x0884aa28`
  (conf 90); the inline vertical damping is `up * dot(up, v) * -0.25 *
  (1 - grounded)` - normal-projected AND zero while grounded (conf 92); and
  `Ship_ApplyLateralGrip` writes only body-local `X` (conf 90). With those
  gone, the force-path enumeration is complete and contains **no
  linear-in-velocity term at all**.
- **The deficit is a velocity-space scale, not a force.** `speed` is
  `body+0x398`, which `Body_Integrate` writes as `sqrt(dot(v,v))` from the same
  register it stores as velocity - so the trace's unexplained `1.0367x`
  `speed`-vs-`|velocity|` ratio means something shrinks `body+0x140` *after*
  the integrator, invisible to any force accumulator. A multiplicative
  `3.67 %`-per-frame loss is `2.22 * |v|` of equivalent force; the fitted
  constant was `2.28`. **Right shape, right size, wrong kind of thing.** Both
  old captures sit in that regime for their entire length (`speed/|v|` never
  reads `1.0000`); they were recordings of the collision response, not the
  force law. Conf 90 on three agreeing legs.
- **The standing-start capture proves the law end to end**
  (`data/traces/talons-junction-standing-start.csv`, 300 ticks; produced by
  `captures`, consumed by `hover-re`): `net = fs + 2*accelcap - 2.0 -
  0.005*fs^2` fits the launch from `fs = 0.56` to `47.67` with **rms 0.127**
  on a one-parameter fit, and launch acceleration reads `32.52` against the
  predicted `31.8`. `stun_timer` and `timer_2e0` are `0.0` on every tick, so
  the engine-gate escape hatch is closed by columns, not argument.
- **Standing, not retracted**: `mass = 1`, the tick-8 speed pad, the stun-gate
  negative, every thrust-side confirmation. **Retracted**: implementing
  `2.28 * fs` as a force, ever.
- Crate corrections that fell out (`ef110d8`): vertical damping now scales by
  `1 - grounded` (was wrongly `1 - magLockBlend`); `craft+0x2ec` is
  `|dot(v, fwd)|` - the `vabs.s` at `0x08849930` is its only writer (conf
  90/88) - so quadratic drag is dissipative in both directions and the
  reversing branch (`-0.1`) is unreachable in the original; the airbrake drag
  scale is `1e-5`, not `0.01` - two literals, the capstone pass had stopped at
  the first (conf 88); rolling resistance magnitude is exactly `2` via a
  `vadd.t` self-add, conf 80 -> 92.
- New evidence page `docs/ghidra/functions/psp-pulse/rigid-body.md`
  (`Body_AddForceAtPoint`, `Body_Init`, `Body_Integrate`; linear damping
  confirmed `0.01`; `body+0x398` identified).
- **Where this sends the next session**: compare `crates/physics/src/wall.rs`
  against the now-concrete target - sustained contact loses ~`3.6 %` of speed
  per frame, decaying from `4.2 %` post-impact toward `3.67 %`. The live lead
  for the writer is `FUN_0884e968` (tangential relative velocity scaled by a
  friction field at `contact+0x34`, applied through `FUN_0884d64c`);
  `Ship_UpdateMagLock` is ruled out (renormalises to the original magnitude).
  Any future reference capture should first pass the cheap cleanliness test
  `speed/|velocity| == 1.0000`. `the_ship_does_not_stay_on_the_track_yet`
  still fails and is now expected to hinge on the wall response.

**`captures` (in flight): the angular column is not angular velocity in
matching units, and that un-ties `oag-trace show`'s four-way scoring.**
`scripts/trace-angular-fit.py` (`7738bf4`) exists because the four
`AngularReading`s come back *undecided* on real data - all four residuals land
within a few percent of the column's own rms. The script's fit says why: the
recorded `body+0x160` column is proportional to the **body-local** angular
velocity with three distinct negative per-axis constants near `-15, -21, -15` -
i.e. it reads like a body-frame angular *momentum* `I * w` with a diagonal
inertia tensor, and `1/|k_y| ~= 1/21` is suspiciously close to the `1/22.1`
that `YAW_DRIVE_CALIBRATION` stands in for. If that holds it would revise this
file's earlier "the yaw equilibrium is provably inertia-independent" note -
treat as preliminary until the agent's final report and steering captures land.
It also committed scripted scenarios (`verification/scenarios/steer-left`,
`steer-right`, `airbrake-asymmetric.inputs`) and a division-by-zero fix in
`trace-force-balance.py` for stationary data.

**`mechanical` (done): the SAP +/-1024 contradiction is dissolved into a
narrower question, the docs debt is paid, and `oag-trace` reads both discs.**

- **Collision survey (`ead464a`)**: walking every `.vex` entry on both discs
  (all 130 PSP + 189 PS2 collision nodes), **seven of sixteen environments
  reach past +/-1024, but nothing spans more than 2,048** - the widest
  collidable span is `2026.6057` (`10_Track`, 98.96 % of the window). So every
  shipped track fits the packing window and none fits it centred on the world
  origin: the packing must subtract a per-track base (prediction conf 75).
  The survey table lives in `docs/formats/collision.md`; the next step is
  reading `Sap_Init` (`0x0882f8f4`) / `Sap_QueryAabb` (`0x088304d4`) for that
  base. A straight transcription packing raw world coordinates would silently
  drop geometry on seven of sixteen tracks.
- **Docs debt (`52f40ed`, `aa5a7db`)**: `wad.md` records the second-binary
  confirmation; `handling-stats.md` gains the `<Misc>` element (offsets
  `0x78..0x90`, `weight_distribution` at `0x90`). One deliberate demotion: the
  "PSP build has an equivalent `<Misc>` parser, conf 80" claim is now an open
  question with **no confidence score** - it was a hypothesis from the
  consumer (`Ship_UpdatePitch` reads `0x90`), not an observation of a parser,
  and no PSP parser was ever searched for.
- **Trace migration (`b19bcee`)**: `crates/trace` now resolves archives via
  `oag_assets::pulse::Archives` name-based lookup, so **PS2 discs are valid
  `--source` arguments to `oag-trace`** - the PCSX2 leg of M3 just got its
  asset path for free.

Both finished agents went idle without delivering final reports (their
commits and doc edits carried the substance instead); everything above is
folded from the diffs directly. Task #2 (PS2 force-term corroboration) is
still unclaimed.

## 2026-07-28: a three-agent pass is in flight, and the capture harness got its angular-velocity column

A new session spawned three concurrent background agents in this working tree,
per the previous pass's "Where I would go next" list: `resistance-gap` (the
~12x missing-resistance investigation, the M4 blocker), `ps2-game-path`
(platform-generic `oag-game`), and `angvel-capture` (the trace harness's
angular-velocity column). Findings are folded in here centrally as each
reports; sections below this one predate the pass.

**`angvel-capture` is done and verified.** Commits `79deaa5`, `bb312c5`,
`40c7d3b`, `96815f2`, `b4c5a01` - plus its first chunk (`scripts/psp-trace.py`
and `crates/trace/src/{trace,replay,main}.rs`) which landed inside `5f2ad20`,
a commit whose message says "game: support PS2 source..." because a concurrent
agent's commit swept up these staged files. **Do not conclude from `git log`
that the first chunk of the trace work is missing - it is in `5f2ad20`.**

What exists now:

- `scripts/psp-trace.py` records three new per-tick fields with no extra
  read round trip: `avel_x/y/z` from `body+0x160` (justified two ways from
  already-documented pages: PSP `Body_ClearVelocity` `0x0884da5c` zeroes
  `+0x140`/`+0x160` together, and PS2 `Ship_ApplyAngularDamping` `0x0015c1b0`
  reads `+0x160` as the angular velocity it damps), `stun_timer`
  (`craft+0x290`, doubling as the wall-contact indicator the resistance
  investigation wants), and `timer_2e0` (`craft+0x2e0`, kept offset-named per
  ADR-0005 since its arming condition has never been read). Values recorded
  raw - nothing negated or rotated on the way out of memory.
- `oag_trace::Frame` carries them as `Option`s. Backwards compatibility is
  hard-tested: a frozen 25-column `LEGACY_FIXTURE`, absent columns are `None`
  never `Some(ZERO)`, `compare` reports missing fields as `not compared` with
  `compared_ticks: 0` rather than a perfect-looking zero error, and `COLUMNS`
  is cross-checked against `psp-trace.py` via `include_str!` so the two
  cannot drift. `replay` seeds `body.angular_velocity` from the first row
  when present.
- **The first real capture with the new column settles two open questions at
  once, with no simulation involved**: `AngularReading` enumerates the four
  readings of the recorded column (sign x local/world frame - both open:
  `craft-update.md`'s `w_game = -w_physics` leaves the frame unresolved, and
  `engine.md` caps local-vs-world at 74), and `oag-trace show` scores all
  four against the recording's own basis derivative
  (`sum cross(row_i, row_i') = 2 n sin(t)`), printing them best first.
- 86/86 tests pass (re-verified centrally); `compare` and a disc-backed `run`
  are byte-identical to the pre-change baseline on all five real captures in
  `data/traces/`. **Not yet done: the live capture itself** - needs an
  interactive PPSSPP session. The exact commands (scripted steer-both-ways
  form to separate all four readings; straight-line form to check whether the
  stun gate fired during the reference capture) are in the agent's report and
  in the updated capture docs (`40c7d3b`); the steering scenario is the
  discriminating one, since a level ship yawing about world `+y` reads the
  same in local and world frames.
- Two `#[ignore]`d ground-truth tests fail on *missing input*, pre-existing:
  they want `data/traces/talons-junction-steer-left.csv`/`-right.csv`, which
  are not in `data/traces/`. Not a regression from this work.
- A lead deliberately not followed (would need a new-address hunt, out of the
  harness task's scope): `craft-update.md` says the matrix at `body+0x80`
  maps `+0x160` onto `+0x150`, so a `0x150` twin column would settle
  local-vs-world directly if the basis-derivative check comes back ambiguous.

**`ps2-game-path` is done and verified on both real discs, centrally
re-checked.** Commits `fde5297`, `7e6bc71`, `b652e3a`, `5f2ad20` (which also
carries the trace agent's first chunk, see above), `00dd670`, `5f32c30`.
`just play data/images/pulse-ps2-eu.chd --race` loads a driveable race off
the PS2 disc: same 16_Track spline (2 paths, 862 control points), 196
colliders/6,227 triangles, Assegai Venom handling - and the PS2 front end
boots too (11 screens, 1,724 English strings from `WADS2.WAD`'s
`Data\Plugins\PI001\GUI\Skin.xml`), which had been written off as probably
unreachable. What to know:

- **Archives are found by name, not derived from the platform.**
  `oag_assets::pulse::Layout::resolve`/`Archives::open` match trailing path
  components case-insensitively (`PSP_GAME/USRDIR/Data.wad` then
  `WADS2.WAD`; `FE.wad` then `WADSP.WAD`) against the source's own file
  list, because the PS2 archives sit in a directory named after the disc
  serial (`54748/` on SCES-54748) - a path constant would be right for one
  pressing and wrong for the next. Nothing branches on
  `oag_disc::Platform` in any decode path (the PS2 disc carries PSP-format
  batches, so per-disc discrimination would be wrong). `Boot.movie` is now
  `Option` - the PS2 ships no PMF in any archive.
- **Telemetry after 60 ticks is identical on both discs** (speed 49.60,
  grounded 1.0, spline 3.90, height 3.89, same position). Deliberately kept
  out of `docs/comparisons/pulse-psp-vs-ps2.md`, which holds the
  same-values question open on purpose - but it is a strong data point for
  ADR-0004's "gameplay is identical across asset sets".
- **Two real-data corrections**: the untextured-model report had been keyed
  on an empty texture list and wrongly fired on the PSP disc (the ribbon
  declares no slots on either disc); now keyed on *unfilled* slots with
  counts, plus a regression test. And PS2 movie names carry their own
  extensions (`Intro.pss`, `Backdrop.ipf`), so the Movie widget's
  `src + ".PMF"` rule yields `Intro.pss.PMF` on PS2 - documented, nothing
  acts on it yet since neither container is decoded.
- **Degradations are explicit, not silent**: no PMF intro, a zero-byte
  `.fnt` entry (5x7 fallback), and front-end images under unrecovered names
  (empty sprite sheet) each get a report line. The PS2 ship renders white -
  the model-to-texture-set lookup is still unrecovered (the decoder itself
  exists, `oag_render::mesh::ps2_texture_set`).
- **The one failing test in its full run
  (`the_ship_does_not_stay_on_the_track_yet`) is pre-existing and stays.**
  The agent verified it fails identically at `27b2587` (before any of this
  pass's work) and recommended deletion per the test's own comment - but
  the wall-collision section elsewhere in this file already deliberated
  exactly this: the ship satisfies the formal check by wedging into a wall
  at 4x the real speed, which is not what "the physics improved" means. The
  test stays until the resistance gap is resolved; its recommendation is
  declined, not missed.
- Optional follow-ups it named: recover the PS2 model-to-texture-set
  lookup; find the PS2 front-end image/font entry names; migrate
  `crates/trace` off the old `pulse::archive_spec` PSP path onto `Layout`
  (left alone because another agent owned that crate at the time).
- Its sandbox note, worth keeping: its first `ls` of `data/images/`
  returned nothing because **the sandbox silently blocks `ls` on some
  paths** ("permission denied" even on `crates/disc/src`) - `find` works.
  A third distinct way real data can look absent when it isn't.

**`resistance-gap` is done, and the blocker's diagnosis was wrong on both
sides.** Commits `695178a` (scripts/trace-force-balance.py), `aa060b9` +
`80d1924` + `f436950` (docs/physics/force-balance-ground-truth.md and
in-place engine.md corrections), `e050e0a` (passive.rs airbrake-drag
comment correction). Centrally re-verified: the force-balance script
reproduces every number below against the real traces, and
`cargo nextest run -p oag-physics` is 168/168. Its method was measurement
first: reconstruct the original's along-forward force from the traces
themselves - projecting onto `forward` cancels everything normal-axis
(hover spring, the disputed downforce, vertical damping) and lateral grip,
and gravity is removed analytically because **the reference capture climbs
a sustained ~4.25% grade for its whole 200 ticks - it was never a
flat-ground equilibrium**.

- **`mass = 1`, not ~23. Confidence 92.** `Ship_InitCraft` calls
  `Body_SetMass` (`0x0884d850`, four instructions, stores unscaled) at
  `0x0884985c` with the XML `mass` in the delay slot. This determines the
  `<Physical mass>`-to-body relation engine.md called "not determined",
  and retires this file's earlier "craft mass near 23" - that number was
  derived from the very balance it was offered as corroboration for.
  Load-bearing for everything below, which is why it was verified first.
- **The thrust side is fully confirmed at instruction level (92-95).**
  The `0.001` load scale, `cap = 0.5*speed + accelcap` (the `add.s` at
  `0x0884c7b0` refutes a sign-error hypothesis), the `min`, and the `*2.0`
  (whose store always lands - `0x0884c938` is `bc1f`, not branch-likely).
  `T ~= 58` is right. The `gain`/`falloff` ramp being dead is confirmed in
  the same pass: `0x0884c728` overwrites the ramped state with raw input.
- **The missing resistance is LINEAR in speed, not quadratic. Confidence
  85.** Missing force is `52.5` at `fs = 23.03` and `45.1` at `fs = 20.04`
  - scaling exponent 1.09; linear predicts the second point to +1.3%,
  quadratic to -11.9%. This refutes engine.md's earlier `0.095*v^2` target
  shape, which came from a single operating point. Size roughly
  `2.28 * fs`, putting equilibrium at 22.94 against the measured
  zero-crossing 23.07. **The constant is fitted and deliberately NOT
  implemented** - no recovered source yet.
- **The stun gate never fires in the reference capture** - answered from
  the trace itself, no collision geometry needed: median implied thrust is
  4.15-4.66 across all tick windows (a fired stun forces zero), and the
  deficit is speed-proportional where a gate predicts constant. Closes
  `21798c6`'s open question.
- **The downforce cannot be this gap at any magnitude** - it acts along
  the surface normal and cancels identically in the projection. The
  resting-height contradiction stays open but is not load-bearing here.
- Also found: **`passive.rs`'s "no dedicated airbrake drag term" was
  false** - `Ship_UpdateAirbrakes` forms
  `|airbrake_l - airbrake_r| * Airbrake.drag * |steer| * 0.01 * fs` gated
  on `fs > 0` (confidence 85; identically zero in every existing capture,
  so explicitly not this discrepancy). `Body_Integrate`'s linear damping
  confirmed `0.01` (right shape, 228x too small). Quadratic drag
  coefficients raised to 95, read as immediates, including the `-0.9`
  branch value engine.md had only estimated. Rolling resistance really
  does normalise (constant magnitude, cannot hide a linear term; 80,
  hand-decoded VFPU). The trace's `speed` column is a consistent 1.0367x
  multiple of `|velocity|` (still unexplained); `speed_cached` is
  `dot(velocity, forward)`. **Tick 8 of the reference capture is a genuine
  speed-pad impulse - the capture is not pad-free**, worth knowing before
  reusing it as a clean straight-line reference.
- Trap: `jal` targets in `BOOT.BIN` encode the ELF vaddr (base 0), not
  the `0x08804000` image base - scan for `0x0C012614`, not `0x0E213614`,
  to find calls to `0x0884d850`.
- **The Ghidra bridge was down for this agent** (`list_instances` empty),
  so all disassembly came from capstone over the extracted `BOOT.BIN`
  directly. Consequence: the VFPU-heavy function bodies could not be read
  - and that is exactly where the mechanism must be. **Next RE step, with
  Ghidra and the Allegrex module up: read the VFPU bodies of
  `Ship_UpdateHover` (`0x0884870c`), the inline vertical damping in
  `Ship_UpdateCraft`, and `Ship_ApplyLateralGrip` (`0x08848b78`). Top
  hypothesis, the only candidate whose shape matches: a hover
  spring-damper written `-c * velocity` instead of `-c * (v.n) * n`,
  which is exactly a linear drag.**
- **The capture it wants next is a STANDING START** on the reference
  configuration (200+ ticks, the new `stun_timer`/`timer_2e0` columns
  included): at `fs ~= 5` linear predicts ~11.4 missing force against
  quadratic's ~2.5 (4x discrimination, versus the current 12% margin),
  and at `fs ~= 0` the measured forward acceleration IS the thrust with
  zero model assumptions - the confirmed law predicts exactly
  `2 * min(41.8, 17) = 34.0`, so a launch showing ~4 instead would invert
  the whole diagnosis. Avoid the start-line boost window (`craft+0x294`)
  and the false-start trap. Second priority: a capture with asymmetric
  airbrakes plus steer, the only way to exercise the new airbrake drag
  term.

## The movie cache is lossless AV1 now, and two of its traps fail silently

The cache holds lossless AV1 in IVF instead of raw `yuv420p`, decoded in process
by `re_rav1d`; see [ADR-0008](docs/architecture/adr/0008-av1-movie-cache.md) for
why and for the measurements. The intro's default extent went from 48.8 MiB to
1.17 MiB, and the decoded picture is **bit-identical** to what the raw cache held
- verified by decoding the shipped code's own cache file and comparing md5 with
the `.raw` it replaced.

Two traps cost a cycle each here, and both produce plausible wrong output rather
than an error:

1. **`ffmpeg -lossless 1` on its own is silently ignored.** It yields a
   ~200 kbit/s *lossy* encode - 141 KB for 261 frames, which looks like a
   spectacular compression win and is not lossless at all. The quantiser has to
   be pinned and rate control disabled too:
   `-b:v 0 -crf 0 -qmin 0 -qmax 0 -aom-params lossless=1`. Worse, with lossless
   correctly forced, **`-cpu-used 4` and `-cpu-used 2` are still not bit-exact**
   (PSNR y 116 dB - a handful of samples differ), reproducibly. `-cpu-used 6`
   and `8` are exact, and faster. No cause established; just don't go below 6,
   and verify the round-trip rather than trusting the flag.
2. **`re_rav1d`'s `Picture::plane_data_geometry` returns `(stride, height)`, not
   `(width, height)`** - its doc comment says so, the name does not. Copying
   stride-wide rows pads every frame invisibly: 480x272 came out as 512 wide,
   and the only symptom was a byte count that was 6.7% too large. Use `width()`
   and `height()` for the extent and `stride()` only as the row pitch.

Also worth knowing: `re_rav1d`'s **default features need `nasm`**, which is not
installed here. The dependency sets `default-features = false`, and decode still
runs at ~900 fps for a 480x272 clip - thirty times playback. Don't "fix" a build
by adding the `asm` feature.

The 4 KB fixture at `crates/formats/tests/data/testsrc-64x64.ivf` is generated
from `ffmpeg`'s `testsrc`, carries no game content, and is what lets the decoder
and the rewind path be tested in CI with no disc image.

## The intro was playing the wrong reel - the counters were always right

Reported from playing it: the intro's pauses looked correct but the video looked
wrong, and it did - the pauses were landing on the FX400 Racing League card and
on "ENGINE CORE ENABLED", mid-motion inside the 40-second team showcase.

`Data\Movies\Intro.PMF` was the default because it was the only intro-shaped
movie whose *name* had been resolved, and that was recorded as a known divergence
rather than chased. Chasing it took an afternoon and settled three things:

- **The reel is identified.** `Data.wad` has exactly three 260-frame movies, at
  indices 440/441/442 right after `Intro.PMF` at 439. All three are static at
  frames 144 and 231 and moving either side; decoded, 144 is
  `SONY COMPUTER ENTERTAINMENT <region> PRESENTS` and 231 is
  `A STUDIO LIVERPOOL GAME`. They are regional cuts - `b1ba72c3` Europe,
  `41fbd22f` Inc., `3d2c85f8` America - and `b1ba72c3` is now the `--movie`
  default. `Intro.PMF` at those same frames is moving (mean interframe delta 5.57
  against 0.15), which is the negative control.
- **The cut is European, not American, and the image's `UCUS-98712` serial is a
  red herring.** All three cuts ship on every disc - *Pure*'s USA disc carries the
  same three hashes at the same three sizes - so the disc's region cannot be what
  picks one, and the pick is at runtime. The executable here is the EU build
  throughout: 18 `UCES00465` strings in `BOOT.BIN` and **zero** `UCUS`, an ISO
  volume id and publisher of `SCEE`, and a `PSP_GAME/USRDIR/UCES00465/` tree with
  its own `SYSDIR/BOOT.BIN` on the disc. Confidence 75; the selection mechanism
  itself is unread.
- **`frontend-video.md`'s 82-confidence "these are logo cards" inference is now
  95**, from the picture rather than from the constants.
- **The name is still not recovered, and the cheap paths are exhausted.** No
  `Data\Movies\` string in `BOOT.BIN` beyond the four save-data icons; every
  fexml blob in all four WADs expanded, and `Movie` widgets exist in exactly one
  file naming only `Intro` and `Backdrop`; `Intro Screen->IntroMovie1` is in no
  XML at all, so that state is code-side; 27,328 assembled candidates hash to
  none of the three. Entries are addressed by hash instead - `--movie
  hash:3d2c85f8` - which is what the WAD directory stores anyway.

**Two things this got wrong on the way, both corrected in the docs.** Do not
re-derive them from the old wording:

- *The American cut was the first pick and it was wrong.* It rested on the
  `UCUS-98712` serial alone; everything the running code touches says EU. See
  above. If a future session sees `hash:3d2c85f8` referenced anywhere, that is
  stale.
- *The disc does not show a dev/pub leg at boot.* Cold boot under PPSSPP with
  `MoviePlayer_Open`, `0x088d7d80` and `0x088e3938` armed from reset, over ten
  minutes: only `Intro.PMF` (at `Language Selection`) and `Backdrop.PMF` (at
  `LogoFMV`) are ever opened, and `IntroMovie1`'s `OnEnter` never fires. So the
  260-frame reels are never played during boot, the old "logos, then picker, then
  intro" guess is **dead**, and the reel is matched to the state by its contents
  fitting the constants rather than by observation. Where those reels *are*
  played is now an open question worth someone's time.

**Traps worth carrying forward:**

- Breaking on `Wad_HashName` (`0x08940d0c`) under PPSSPP does recover real entry
  names live - 32 of them, correct - but arming it from reset runs the emulator
  so far below real time that 25 minutes of wall clock did not reach the movie
  load. Arm it *late*, after a rare breakpoint has got you where you want to be.
- Piping a probe's stdout through `tail` buffers the whole run, so a background
  capture looks like it produced nothing until it exits. Redirect to a file.

## The physics/gravity Ghidra pass: gravity confirmed, two leads, nothing landed

A deliberate sweep of the force law against `BOOT.BIN`, prompted by "the racing
feel must match as closely as possible". **Read the last section first if you are
about to implement the downforce - it is the one that stops you.**

### Confirmed, no change needed

**Gravity is correct as implemented.** `passive::gravity` matches
`Ship_UpdateCraft`'s `vmul_p` chain term for term: `normal_gravity` grounded,
`flight_gravity` airborne, blended by the *previous* frame's fraction via
`vocp_s`, mass from `body+0x374`, world `.y` only, per-class scale on
`normal_gravity` **alone**.

**`g_class_gravity_scale` is `1.0, 1.0, 1.0, 1.0`** (`0x08ab0dcc`, four floats).
`Environment::class_gravity_scale` documented the identity as a default because
the table "was not read". It is now. Confidence 90.

**`BANK_TO_YAW_GAIN` is `30.0`, and the PS2's `50` was never a contradiction** -
that one belongs to `Ship_HoverFourCorner`, a *different function* selected by
`DAT_08b31048 == 6`. The two-point path this crate models reads 30.

**The alignment torque** is `cross(up, avgNormal) * -400` with the right-axis
component projected out, into the **world** angular accumulator. As transcribed.

**The weathervane** is `cross(forward, velocity) * -0.1` grounded, `-0.3`
airborne. As transcribed.

### The yaw-resistance hunt is now a closed negative

`engine.md`'s suggested lead was `Ship_ApplyLateralGrip` (`0x08848b78`). **It is
not the answer**: it writes a body-local *force* along `X` into `body+0x110`, at
the centre of mass, so it produces no torque at all.

With that read, **every** writer to `craft+0x340`/`+0x350` on the grounded racing
path is now enumerated and verified: airbrake yaw, steering, pitch, alignment,
bank-to-yaw, weathervane, angular damping. This crate implements all seven, and
none of them is the missing 22x.

**So no further static reading will find it, and that is the useful result.** The
mechanism is not in `Ship_UpdateCraft`. The next step has to be dynamic, and it is
blocked on a harness gap the docs already noted: **there is no angular-velocity
column in the capture**. Adding `body+0x160` to `scripts/psp-trace.py` and
`oag_trace::Frame` would let a capture answer directly what the yaw rate does
frame by frame, instead of it being inferred from consecutive basis rows. That is
the single highest-value thing to build next.

### Two changes were written, measured, and deliberately reverted

Both are recorded here rather than in the code because **neither survived
contact with the crate's own runtime-verified numbers**, and a half-landed force
term is worse than none.

**1. The grounded downforce.** `Ship_HoverTwoPoint` (`0x0884a658`) ends with

```text
worldForce += -averageNormal * (class[0x6c] * mass * grounded) * (1 - magLockBlend)
```

and `class[0x6c]` is `track_gravity` on the same offset arithmetic that makes
`class[0x64]`/`class[0x68]` the gravity pair the verified gravity term reads.
`hover::DOWNFORCE_SCALE` is `0.0` and is documented as "shape implemented,
magnitude unrecorded", so this looked like the missing magnitude. Implemented, it
improved the straight-line replay (excess climb `+9.85` to `+6.71` against the
recording's `+4.80`; speed runaway `55.5` to `41.5`).

**It is nonetheless wrong, or at least not this simple, and here is the argument
that stops it.** `TARGET_GLOBAL_SCALE` is `0.75` and **runtime-verified** (`4.125`
target against a `5.5` `ride_height`, read off a live Time Trial). This module's
own arithmetic gives a resting sag of `normal_gravity / (0.3 * K * (normal_gravity
+ track_gravity))` = `0.147`, so a resting ship sits at `3.978` - and the original
*measures* `4.002`. That sag is computed with **gravity alone**. A downforce of
`track_gravity * mass` is seventeen times grounded gravity and would put the
resting height near `1.6`, nowhere near the measured `4.002`.

So one of these is wrong: the reading of `class[0x6c]`, the assumption that
`craft+0x2b0` is the grounded fraction at that point, or the belief that this term
is active at rest. **Do not implement it until that is resolved.** The fact that
it improved the replay is not evidence - the replay diverges for other reasons.

**2. The hover probe geometry.** `Ship_InitCraft` (`0x08849354`) writes
`craft+0xc0`/`+0xd0` as constants `(0, -1.5, +/-6)` scaled by `0x08ab0e1c`, and
`Ship_HoverTwoPoint` reads the rotated copies at `craft+0x120`/`+0x130`.
`hover::probe_offsets` instead derives them from `<Misc length>/2` with **no
vertical offset**, and flags itself as a guess. So the spacing should not scale per
ship, and the probes should hang `1.5` below the centre of mass.

Implemented alongside the downforce it restored the settling height and passed
`a_ship_at_rest_on_flat_ground_stays_at_rest` - but it **destabilised the
suspension**: `a_ship_pressed_into_the_floor_never_ends_up_below_it` sinks through
the floor around tick 250. With the probes below the centre of mass, a ship whose
probes go under a surface sees no ground at all and nothing recovers it. That
needs the hull constraint to carry the case before the geometry can change.

What was **not** traced either time: whether anything overwrites `craft+0xc0`
per ship. Nothing was found that does.

### Where this leaves the ordering

1. Add the angular-velocity column to the capture harness. Everything else is
   guessing without it.
2. Resolve the downforce contradiction above - it is a real term in the binary and
   this crate applies none of it.
3. `ALIGNMENT_INERTIA = 19.8` is admitted as fitted and the original divides that
   torque by nothing. Note but do not build on the coincidence that
   `YAW_DRIVE_CALIBRATION` is `1/22.1` and this is `1/19.8`; the yaw equilibrium
   is provably inertia-independent, so they are unlikely to be one mechanism.
4. Drag and rolling resistance, which set top speed - still with **no clean ground
   truth**, because the only straight-line capture is a stunned ship coasting.

## The engine died on wall contact: the collision stun was frame-triggered

Reported from play as "at some point the racer completely stopped accelerating".
Not the engine force law - the **collision stun gate**, and the bug was a 30:1
runaway.

`ShipState::stun_timer` (`craft+0x290`) cuts thrust to exactly zero and suppresses
lateral grip while it runs. `wall::resolve` armed it with `+= 0.5` on **every
frame** the hull moved into a surface, while `forces::evaluate` decays it by `dt`.
At 60 Hz that is **30 seconds of dead engine per second of wall contact**, and it
is unrecoverable in practice.

`Ship_ApplyCollisionImpulse` (`0x0883f274`) was read to settle it, and it is
unambiguous:

```c
if (entity->0x4c+0x110 != 0) {          // a *pending impulse vector*
    if ((pickupFlags & 0x10) == 0) {
        ... project onto forward, apply via FUN_0884d64c ...
        craft+0x290 += 0.5;
    }
    entity->0x4c+0x110 = (0,0,0);       // cleared on every path
}
```

**The `+= 0.5` fires once per impulse the collision system posts, and the impulse
is consumed immediately.** It is an edge, not a level. So the accumulation the
previous note recorded (`+=`, not `=`) is right, and it is per *impact* - separate
hits still stack, a sustained scrape does not.

The fix is `ShipState::wall_contact_prev`, an edge test on the arming site, with
the flag cleared on both of `resolve`'s no-contact returns. Pinned by
`a_sustained_scrape_arms_the_stun_once`, which is not vacuous: on the old code it
reports `30.0` against an expected `0.5`.

Worth knowing for whoever models the collision system properly: our `resolve` runs
every frame and has no impulse queue, so the edge test is standing in for the
original's "consume the pending impulse". If a real contact-event queue is ever
built, that is where this belongs.

## Steering was 22x too strong, and fixing it left a real open question behind

Reported as "left and right turn the racer super strong" in `just play`. It was
real and it was large. What matters for whoever picks this up is that **the fix
is a calibration, not a recovered value, and the RE question underneath is still
open.**

The whole steering path has now been read at instruction level, and it is
*correct as this crate transcribes it*:

| Site | Reading |
| --- | --- |
| `HandlingXml_ParseTurning` `0x088398e0` | `swc1 f0,0xd0(a0)` - `amount` stored **verbatim**, no scale |
| `Xml_AttributeAsFloat` `0x0895379c` | plain decimal reader, no exponent |
| `Ship_UpdateSteering` `0x08848788` | `yaw = steer * amount`, `+=` into `craft+0x340` |
| `Ship_ApplyAngularDamping` `0x08848ed0` | `-5 * angularVelocity.y` into the **same** accumulator |
| `Body_AddTorqueLocal` `0x0884d5bc` | forwards `craft+0x340` to `body+0x120`, unscaled |
| PS2 `Body_Integrate` `0x0015d088` | `angularVelocity += inverseInertia * torque * h` |

Because steering and damping share one accumulator, **every common factor - the
inertia tensor included - cancels at equilibrium**, leaving
`omega = steer * amount / 5` unconditionally. That is about **22x** the
`1.5 rad/s` the hardware turns at, which is measured two independent ways (this
repo's own confidence-90 runtime capture, and both `data/traces/` captures
re-differentiated as `dot(cross(fwd_t, fwd_t+1), up)/dt`).

So a **yaw-opposing term in `Ship_UpdateCraft` is missing from `engine.md`**. Note
the precedent: the longitudinal axis had the identical shape of gap ("resistance
is 12x short") and it turned out not to be resistance at all but *thrust the
original does not apply*, gated on the collision-stun timer `craft+0x290`.
`Ship_ApplyLateralGrip` (`0x08848b78`) returns early on that same timer - that is
where to look first.

Meanwhile `oag_physics::forces::YAW_DRIVE_CALIBRATION = 0.0452` stands in,
fitted **separately** to each capture (`0.0454` left, `0.0450` right; RMS error
`0.10` and `0.04 rad/s` against a signal of `1.5`). It is applied to the summed
body-local yaw **drive** in `forces::evaluate`, after all three contributors and
before the damping.

**Do not narrow it back to the steering term.** That was the first attempt and it
was reported from play within minutes: "on slightly tilted tracks I'm not able to
steer in the opposite direction". `hover::BANK_TO_YAW_GAIN`'s `30 * right.y`
writes to the same accumulator, so scaling steering alone leaves the camber 22x
too strong relative to the player's input and the break-even bank falls to about
14 degrees. Pinned now by
`full_lock_out_yaws_the_bank_on_a_steeply_cambered_track`. Scaling the axis
instead also made the full-step replay *symmetric* - `+1.28` / `-1.28 rad/s`
against the original's `+1.52` / `-1.47`, where scaling steering alone gave a
lopsided `+1.22` / `-1.34`.
`crates/trace/tests/yaw_authority_ground_truth.rs` re-runs that fit under
`just test-data`, and it is not a vacuous test - at `1.0` it fails with RMS
`28.3`. Through the full `oag_physics::step` on the real disc the replay now
yaws `+1.22` / `-1.34 rad/s` against the original's `+1.52` / `-1.47`.

Two traps worth writing down:

- **It is applied to the input, not the damping, and that was measured rather
  than chosen.** Raising `YAW_DAMPING` to about `107` hits the same equilibrium
  but collapses the time constant from `0.2 s` to `0.009 s`. The captures rule it
  out: `steer` chatters +/-8 % at ~20 Hz (the authentic non-clamping ramp) and the
  recorded yaw rate does *not* follow it.
- **The input layer was not the bug and must not be "fixed".** The keyboard's hard
  +/-1.0 step is correct - the original's d-pad does the same and smooths in the
  sim via `ramp_steering`, which the captures confirm to under 1 % (`gain` and
  `falloff` reproduce the recorded 498/s rise and 924/s fall). A deadzone or
  filter in `oag-input` would diverge from the original *and* hide this.

Also settled in passing: the camera is **not** implicated - the disc gives
`spring_horiz`/`spring_vert` of 11, and `lag(11, 1/60) = 0.183` is nowhere near
saturating, so the eye does lag. And `engine::pitch` reads the raw `-1..1` axis,
not the ramped `+/-100` state, so pitch does **not** share this bug.

## In flight

A new pass started same day: **both binaries are now in one Ghidra project.**
`SCES_547.48` (PS2, `r5900:LE:32:default`, image base `0x00100000`, 28 overlay
spaces, 5,234 auto-analysed functions) is imported alongside `BOOT.BIN` and was
confirmed live via `list_instances`/`list_open_programs` - it is the *current*
program; `BOOT.BIN` is in the project but closed. Roadmap M2's "Load
`SCES_547.48` into Ghidra" checkbox was stale (unchecked despite being done) and
is now fixed.

**That import did not survive, and was rebuilt from scratch on 2026-07-27.** A
later session found `list_instances` reporting `BOOT.BIN` as the project's only
program, `data/extracted/ps2/` empty, and - the actual cause - **no
EmotionEngine extension installed at all**, so `r5900:LE:32:default` was not
even an available language. Why the program and the extension both went missing
is not established; no evidence either way was found, so no guess is recorded
here.

As of that rebuild the bridge reports **both programs open**, `SCES_547.48`
current and `BOOT.BIN` no longer closed - so the paragraph above is stale on
that detail. Keep passing `program=` explicitly on every Ghidra MCP call: with
two programs open, omitting it silently targets whichever is active.

Do not read that as the original numbers being invented. **The rebuild
reproduces them exactly**: same language, same `0x00100000` base, same 28
overlay spaces, same 5,234 functions, and all 135 rows of
`docs/ghidra/functions/ps2-pulse/names.tsv` resolve to live addresses under
`apply-ghidra-names.py --dry-run` (0 skipped, 0 failed). Auto-analysis is
deterministic given the same Ghidra and extension version, so the figures
matching to the function is strong evidence the `ps2-pulse` pages were written
against this binary, and they are now independently re-verified against a
second, clean import.

**The database itself is unnamed, though** - that check was a `--dry-run`, so
every one of those 135 symbols is still `FUN_*`/`DAT_*` in Ghidra. To restore
them:

```sh
python3 scripts/apply-ghidra-names.py --program SCES_547.48 \
    docs/ghidra/functions/ps2-pulse/names.tsv
```

Two traps in redoing it, both of which cost this session time:

- `just build-allegrex` has a script; the EE extension does not, and
  `toolchain.md`'s recipe assumed a system `gradle` that **is not installed on
  this machine**. What works is driving it with the Allegrex checkout's
  wrapper - see [toolchain.md](docs/reverse-engineering/toolchain.md), now
  updated with the exact command. JDK 21 is required there too, because that
  wrapper is Gradle 8.10.2 and the system JDK is 26.
- `import_file` returns `auto_analyzed: true` **immediately**, and
  `analysis_status` then reports `analyzed: true, function_count: 1` - which is
  exactly the signature the docs give for a wrong-language import. It is not;
  analysis simply had not run. Call `reanalyze` (it times out, which is normal
  for a 1.9 MiB binary - it means analysis started) and poll `analysis_status`
  until `analyzing` goes false. Judge the import on the function count *after*
  that, never on the count the import call leaves behind.

Two background agents were launched to work this concurrently, kept to two
rather than the allowed three to avoid Ghidra/build contention:

1. **PS2 Ghidra names catch-up** - cross-reference the PSP's 120 documented,
   evidence-backed symbols (`docs/ghidra/functions/psp-pulse/names.tsv` and the
   pages under that directory) against `SCES_547.48` by string/constant/
   call-graph correlation rather than address (addresses don't correspond
   across the two images), following ADR-0005 exactly: same base name across
   platforms, `_q` under confidence 70, no rename at all under 50. Scoped to
   `docs/ghidra/functions/ps2-pulse/**` plus "Cross-platform" table edits on the
   existing PSP pages, and told to always pass `program="SCES_547.48"`
   explicitly on every Ghidra MCP call so it can't collide with concurrent work
   against `BOOT.BIN`.
2. **Fall-through-floor / jittery-camera bug** - a user report against
   `just play --race`: the ship falls through the floor and the chase camera
   jumps up and down even at rest with no input held. This may be the same
   roll-oscillator instability already on record below (`h <= c/k = 0.005` vs
   the specified `1/180` sub-step, ~11% too large - which by construction should
   show up at rest too, not only while driving), one of the two "cheap to test"
   M4 explanations (hover target exceeding probe reach; missing inertia tensor),
   a regression since `19611e0`/`5ad69f3`, or a camera-side bug reading stale
   ship state independent of the ship actually moving. Scoped to
   `crates/physics`/`crates/gameplay`/`crates/render`, told explicitly not to
   tune a magnitude to hide the known oscillator instability without evidence.

Both report back before editing `HANDOVER.md` further; findings get folded in
here once they land, not written concurrently by the agents themselves.

`just` last passed on this branch: fmt-check, clippy `-D warnings`, **546
tests**, check-docs. `just audit-leakage` is clean.

## The PS2 binary just gave the whole tree its second-binary leg

`docs/ghidra/functions/ps2-pulse/` went from a stub to five subsystem pages -
**48 symbols named in `SCES_547.48`** (20 at 90-95, 22 at 80-89, 6 at 78-79,
nothing below 70) - correlated against the PSP's 120 by string/constant/
call-graph shape, never by address, since the two images share no address
space. `docs/ghidra/functions/README.md` now indexes both binaries side by
side, and each PSP page that gained a PS2 counterpart
(`wad-subsystem.md`, `engine.md`, `input.md`, `camera.md`) got a
**Cross-platform** section rather than a silent update.

Per the [confidence rubric](docs/reverse-engineering/confidence-rubric.md), a
second binary is worth more than a second reading of the first, and this is
the first time this project has had one for the physics-adjacent claims that
matter most:

- **The 32-field handling block (`0x94`-`0x114`, stride `0x80`) and its four
  load-time scale factors are exact matches**, from independent PS2 parsers,
  under a different compiler and a different ISA (Allegrex vs Emotion Engine).
  Same for the 27-float camera block, the `pi/180` `AirbrakeGraphics`
  conversion, and the WAD name hash (CRC-32, reflected, seeded to 0, `\`->`/`,
  uppercase-folded, `#`-hex escape) - two independent PS2 implementations,
  both matching the PSP reading.
- **`stats_base + 0x90` is answered**: `<Misc weight_distribution>`. The PSP
  `engine.md` page recorded that offset as an unidentified per-team scalar; the
  PS2 binary has a `<Misc>` XML element the PSP page never mentions, and its
  parser writes `0x90` directly.
- **Five deliberate, unreconciled divergences**, recorded as findings rather
  than papered over: PS2's WAD lookup restarts its scan every time instead of
  rotating from the last hit; `cancel`/`backward` bind to `triangle` on PS2
  where PSP uses `circle`; PS2's `Input_ConsumePress` clears the *entire*
  pressed mask and ignores the index argument, contradicting the PSP page's
  confidence-70 "clears one bit" reading; **the chase camera's unexplained 3/4
  factor from this session's PSP camera work does not exist on the PS2 path**,
  which computes its distance a visibly different way; and PS2 builds three
  separate lazily-constructed CRC tables plus a fourth static one, where PSP
  builds one.
- **`0x00213e58`, the PS2 LZSS stream constructor, is now the single
  highest-value RE target left** (confidence 55, decode loop not yet read) -
  the PS2 archives are the only corpus that exercises this project's LZSS
  reader at all, and it has so far only ever been validated against its own
  inverse.
- **Not yet attempted on PS2**: the collision walk, the front-end video/Movie
  widget, the main loop, and - most relevant to the M4 physics work happening
  elsewhere this pass - the `Ship_Update*` force terms themselves. Since the
  handling *parameter block* is now confirmed byte-identical in layout, its
  consumers reading those same offsets is the obvious next step, and would pull
  `engine.md`'s force law out of its current decompilation-only confidence cap.

One tooling fix landed alongside the docs: `apply-ghidra-names.py`'s evidence
check hardcoded `psp-pulse` as every row's base directory, which would have
silently failed every PS2 row. Evidence paths now resolve relative to the TSV
that carries them. Verified both ways: PSP's default set still applies clean
(433 applied, 0 failed), and a dry run of the new
[`ps2-pulse/names.tsv`](docs/ghidra/functions/ps2-pulse/names.tsv) applies all
48 with nothing failing. There is no `just` recipe for the PS2 set yet; see
`ps2-pulse/README.md` for the direct invocation.

**Left for later, out of this pass's scope:** `docs/formats/wad.md` should
record that its header/entry layout is now confirmed in a second binary, and
`docs/formats/handling-stats.md` should gain the `<Misc>` element.

## LZSS has its second binary now, and it agrees - with two new caveats

The same agent went on to read `SCES_547.48`'s LZSS decoder
(`Lzss_Decode`, `0x00214080`, plus `Lzss_ReadBits` at `0x00213f70`) end to end.
**No divergence.** Window `0x1fff` (8192), write cursor seeded to 1, a 13-bit
position used as a direct (not relative) ring index, length biased by two into
a loop that runs one iteration past the bias (3..18), flag bit 1 = literal /
0 = match, MSB-first bit reading with no byte alignment - every parameter
already in [`docs/formats/lzss.md`](docs/formats/lzss.md) is visible in the
PS2 code, under a different compiler and ISA. Committed as `23664bf` and a
follow-up doc amendment `a169521`.

**No live disc validation was possible** - `data/` doesn't exist in this
working tree for this agent (no image mounted). What it did instead:
transcribed a *second* Rust decoder directly from the PS2 disassembly, in the
PS2 code's own shape (resumable state machine) rather than the existing
PSP-derived decoder's shape, and diffed the two over 40,000 random streams, a
full ring wrap, and degenerate inputs - agreeing on every byte. It then
deliberately broke the reference eight different ways (length bias, flag
polarity, bit order, field order, mask reset point, cursor start,
absolute-vs-relative addressing, window size) to confirm the comparison
actually has teeth, after noticing its first sweep silently passed with a
no-op mutation.

**Confidence deliberately held at 94, not raised to 95**: the rubric's top band
needs a runtime trace *and* a second binary, and this only supplies the
second leg. Recorded explicitly so nobody "rounds up" later.

**Two new findings, neither reachable from the PSP side alone:**

1. **The PS2 decompression ring is never zeroed** - a fresh, unzeroed heap
   allocation per stream. Since ring positions are absolute rather than
   relative, a match *can* legally address a slot the stream hasn't written
   yet; the real console would read heap garbage there, our decoder returns
   zero. All 6,053 shipped streams happen to decode correctly under the
   zero assumption - so this is a fact about what the shipping encoder never
   emits, not a bug in the format reading. **A hand-built or fuzzed stream
   could tell the two apart where no real archive on either disc can.**
2. **A dead 98,316-byte encoder workspace** (`Lzss_AllocBuffers`/
   `Lzss_FreeBuffers`, allocated and freed, never read or written by anything
   else). `98,316 = 3 * (8192+1) * 4` is exactly Okumura's `lson`/`rson`/`dad`
   search-tree shape for an 8192 window - the *compressor's* structure,
   shipped compiled out of a binary that only ever decodes. Confirmed by a
   full instruction-operand scan (2 references to this workspace across
   361,812 instructions, versus 6 for the real ring, 3 of them inside
   `Lzss_Decode` itself) after a plain `get_xrefs_to` call under-reported and
   was caught rather than trusted.

10 new Ghidra symbols (58 total on `SCES_547.48` now), one below 70
(`g_lzss_encoder_tree`, 68, `_q`-suffixed). `cargo nextest -p oag-formats`:
192/192.

**Tooling note surfaced by this agent, worth the user's attention:** `sd`
(listed in the global CLAUDE.md as an available CLI tool) is **not installed**
in this environment, and whatever currently answers to that name **exits 0
while doing nothing** rather than failing loudly - it silently no-op'd this
agent's first negative-control mutation sweep and briefly made a broken test
read as passing. Worth installing `sd` for real, or removing it from the tool
list, before anyone relies on it for an edit that matters.

The PS2 Ghidra work paused here on purpose: the `0x00202d48`-`0x00203a80` XML
reader family (confidence 60, would unlock every XML-driven subsystem at once)
was flagged but deliberately not started, in favour of handing off one clean,
finished deliverable rather than a partial one.

## M1 is done: PS2 mesh batches decode too, and it's a different GPU's format

The last M1 gap - `oag-view --mesh` failing on PS2 vertex type `0x1b9` - is
closed. **The reason it looked unrecognisable is that it is a different
platform's format wearing the same bit fields.** PS2 has no GU: the vertex-type
word's position bits say `f32` instead of the PSP's hard-coded three-`s16`, and
that's not a wider PSP vertex format, it's a marker that the whole payload is a
**VIF command stream** (the PS2's DMA-driven equivalent of a GE display list) -
implemented in the new `oag_formats::vif` module and the PS2 path of
`vex::mesh_batches`, committed as `d9f14bb`.

**Confidence 94**, on invariants that can't come out by accident, checked over
**every PS2 `.vex` file**: 757 models, 98,243 batches, 11,767,670 vertices.
Three independent framing lengths (region header, DMA tag quadword count, and
where the command walk actually ends) all close exactly; every one of the 11.8
million decoded positions falls inside its own batch's bounding box with zero
slack; normals come out unit-length to `1.2e-7`; and the same model renders the
same size through both asset paths (PS2 `01_Track` radius 831.02 against the
PSP's 830.92; the Feisar ship at 6.45 on both).

Three things worth knowing before touching this code:

- **A strip split across VU1 memory repeats two vertices at the seam on
  purpose** - 44,967 of 89,302 strip batches split this way, and the repeat is
  what keeps winding parity, not a bug to dedupe.
- **Colour is a 0-128 range, not 0-255** (the GS's own convention for "full
  intensity" through the modulate path) - reading it as a normal 0-255 byte
  makes every PS2 model exactly half as bright, which reads as a lighting bug
  rather than a decode one. Confidence only 70 on this specific point, since
  nothing in the executable was actually read for it.
- **PS2 models still render untextured, and unlit too** - decoding the
  vertex/geometry format didn't touch the separate open question of where PS2
  textures live (the embedded block is empty, unchanged from before this
  session). Compounding it: every PS2 chunk happens to carry vertex colour,
  which `oag-view` already treats as "prelit, skip the light rig" for the PSP
  side - so a PS2 model comes out as a featureless white silhouette, geometry
  correct but with no visual parity to the PSP screenshot at all. **Say this
  precisely**: M1's exit criterion is met on geometry, not on appearance.
- **The PS2 disc also carries 6,080 ordinary PSP-format batches** alongside
  the VIF ones, so the PSP-vs-PS2 discriminator has to be per-batch (which it
  is), never per-disc.
- **The advisor caught a real gap before it shipped**: concatenating a split
  strip's chunks is only winding-correct if every non-final chunk has an
  *even* vertex count; the decoder now refuses odd ones outright rather than
  silently producing a wrong triangle. The disc-wide validation run (all
  98,243 batches) confirms no real file ever triggers the rejection - but the
  check stays, since nothing rules it out for a hypothetical or fuzzed file.

`docs/overview/roadmap.md`'s M1 exit criterion is updated to record this as
met on geometry, with the visual-parity gap stated explicitly rather than
implied.

## The PS2 XML reader is confirmed, and it answers two things from unrelated pages

The same agent that did the LZSS work went on to confirm the whole XML reader
family (`0x00202d48`-`0x00203a80`) - **35 more symbols, none below 70**, taking
`SCES_547.48` to 93 documented symbols total. It's a scanning parser with no
tree: a 160-byte cursor holds a begin/end pair into the loaded file buffer and
re-scans on every step, closing tags are found with a depth counter, and -
worth knowing before reimplementing anything against this data - **it edits
the buffer in place** (NUL over each closing quote, so attribute values are
bare pointers into the file) and **compares every name case-insensitively**.
Committed as `96a658a` and a follow-up correction `0cca5af`.

**Two findings land on pages this agent didn't write:**

1. **`docs/formats/fexml.md`'s 18-code attribute dictionary is now read
   straight off the parser, not inferred from data.** `Xml_OpenFile` stores a
   `code`-tagged element's attributes into an 18-slot table keyed by
   `name[0] - 'a'`, and two independent constants (`c - 'a' < 0x12` at both
   expansion sites, and `0x258 - 0x210 = 72 = 18 * 4` from the constructor)
   confirm the count the format page had only ever gotten by counting codes
   across `FEData.wad`. Also answers one of that page's open questions: PS2
   uses the same schema.
2. **The default float accessor is exponent-blind, and every handling
   parameter goes through it.** `Xml_AttributeAsFloat` skips any character
   that isn't a digit, `.` or `-` rather than stopping at it, so `1e-5` parses
   as `-15`. No shipped value uses exponent notation, so nothing is wrong
   today - but a tool that regenerates `HandlingStats.xml` must never emit
   one. Flagged on both `handling-xml.md` and `fexml.md`. This is PS2-only
   evidence; the PSP reader isn't located yet, so it doesn't transfer without
   checking.

Also fixed, from the same agent's own earlier work: `handling-xml.md` had
called two of `HandlingXml_ParseGlobal`'s five arrays "per-class scalars" when
they're actually `char*` strings. And it caught its own slip - it first wrote
"no PSP XML reader is documented" from a case-sensitivity mistake in its own
grep; `psp-pulse/frontend-video.md` already documents `Movie_ParseAttributes`
as a *consumer* of that reader (not the generic parser itself), which is now
the obvious anchor for locating the real one.

**Next targets it flagged itself, in the order it would pick them:** the PSP
XML reader (cheap now, and settles whether the exponent-blindness finding
applies to both builds); an `<FEGlobals>`/`<FEConst>` indirection at
`0x0017c630` that lets front-end XML reference a symbolic name where a number
is expected, not in `fexml.md` at all yet; and **the PS2 `Ship_Update*` force
terms** - directly relevant to the physics investigation elsewhere in this
file, since the handling *parameter block* is now confirmed identical in
layout, so a second reading of the force law could settle open questions
there (the roll-oscillator constants among them) the same way LZSS and the
handling loader just got settled.

## The roll-oscillator margin is now settled at instruction level, in a second binary - and the crate was already right

The same agent took its own suggestion and read the PS2 build's craft-update
and rigid-body path (`docs/ghidra/functions/ps2-pulse/craft-update.md`,
13 symbols, nothing below 78). **This agent hit a context limit
("Prompt is too long") right after finishing its work and could not send its
own final report** - its last edit landed uncommitted, was recovered and
committed on its behalf (`941a4ee` plus a follow-up), and is folded in here
instead of in its own words. Anyone picking up further PS2 Ghidra work should
start a fresh agent rather than trying to resume this one.

Three things settled, one deepened:

1. **The surface-alignment gain really is `-400.0`, confirmed at instruction
   level in both binaries.** `Ship_HoverFourCorner`/`Ship_HoverTwoPoint`
   materialise `0xC3C80000` as an immediate on PS2, same magnitude, same sign,
   same operand order as PSP. `docs/physics/README.md`'s standing suspicion
   that this was a transcription error is retired: two different compilers on
   two different ISAs producing the identical constant is not a coincidence.
2. **Roll damping is `-2.0`, confirmed exactly - but only outside "mode 0"**,
   which uses `-5.0` instead (`craft+0x2d4` selects; yaw damping is a hard
   `-5.0` always, pitch damping is per-ship). Whatever repro is used for this
   instability needs its ship in the right mode for `c = 2` to be the number
   that applies.
3. **Sub-stepping cannot rescue this instability, confirmed from the PS2's own
   `Body_Integrate` disassembly, not just inferred from the Rust integrator's
   contract**: the force/torque accumulators are filled once per frame and read
   *inside* the sub-step loop with nothing recomputing them between
   iterations - the original does this too, it isn't a simplification this
   project's reimplementation introduced. Redone properly (accounting for the
   real n-substep position/velocity update rather than a naive single Euler
   step), the required damping is `c >= k*H*(n+1)/(2n)`:

   | Sub-steps | `c` required | Against the actual `c = 2` |
   | ---: | ---: | ---: |
   | 1 | 6.67 | 3.33x short |
   | 3 | **4.44** | **2.22x short** |
   | infinite | 3.33 | 1.67x short |

   **The `n = 3` row is exactly what `crates/physics/src/hover.rs`'s own
   stability note already computes.** So the crate's math was already right;
   `docs/physics/README.md`'s older `h <= c/k` reading (which assumes forces
   are re-evaluated every sub-step, giving a falsely-reassuring 1.11x) is
   the thing that needs correcting, not the other way round. **This margin is
   real, it is confirmed in a second binary at instruction level, and it
   cannot be fixed by raising the sub-step count or by any of the constants
   just confirmed exact** - if a fix is needed, it has to come from somewhere
   this reading hasn't reached yet.
4. **The handedness question deepened rather than closed.** `Body_AddForceAtPoint`
   computes torque as `F x r`, not the physically-correct `r x F` - confirmed
   in raw VU assembly, on the one term whose correct sign is fixed by
   mechanics rather than by anyone's prose. That makes the anomaly
   **systematic across at least two terms**, corroborating `engine.md`'s
   standing "the sign lives downstream" hypothesis over "someone mistyped it"
   - but where it's compensated is still unlocated: `Body_AddTorqueWorld` and
   `Body_Integrate`'s rotation update are both plain `+=` with no negation on
   the path read so far. **A follow-up pass has since settled the operand
   convention against the Sony *VU User's Manual* v6.0 and a raw-encoding
   decode of the four instructions in question, confirming Ghidra prints them
   in the manual's mnemonic order rather than reversed** - so "maybe
   `vopmsub`'s convention is simply the reverse of what was assumed, and all
   three terms flip together" is **refuted**, not a live explanation. Two
   candidates remain, and this page picks neither: the compensation is in the
   unread inertia/rotation-matrix path at `body+0xc0`..`+0xf0`, or the force
   is already pre-negated before this call, uninspected component-by-component.
   See `docs/ghidra/functions/ps2-pulse/craft-update.md`'s "cross-product
   convention" section for the manual citation and the encoding table - a
   fresh agent picking this up should start from the `+0xc0` matrix rather
   than re-checking the operand convention.

One more unresolved difference, not yet chased: the PS2's four-corner
bank-to-yaw term uses `50.0` where the PSP uses `30`; neither value is
runtime-verified yet.

## The torque-sign mystery is closed: nothing was ever inverted

A fresh PS2 Ghidra agent (the previous instance died from context overflow
right after resolving the operand-convention question above) took the two
remaining candidates for where the `F x r`/`-400 * (up x n)` sign anomaly gets
compensated, and **both are dead, because there is nothing to compensate.**

**The engine integrates its rigid-body basis as `e' = e x w`, not the textbook
`e' = w x e`.** Read directly out of `Body_Integrate`'s skew-matrix
construction, whose three base constants were confirmed all-zero by reading
them out of memory: the built update gives
`d(row0) = h * (-z*row1 + y*row2)`, the negative of what `w x e0` would
produce. That means **`w_game = -w_physics`** - the engine's angular velocity
is the negative of the textbook pseudovector, consistently, throughout the
whole rigid-body formulation. Substitute that in and every apparent inversion
disappears: `tau = F x r` becomes exactly the textbook `r x F`, and
`-400 * (up x avgNormal)` becomes exactly the aligning `+400 * (up x n)` the
prose always said it was. **`docs/physics/README.md`'s prose and the
arithmetic agree after all - what was missing was the convention, not a
sign.**

Both remaining candidates from before are directly ruled out, not just
deprioritised: the hover spring's force argument at its call site is a plain
**positive** multiple of `up`, no negation anywhere in it; and the matrix at
`body+0xc0`..`+0xf0` is the basis's rotation inverse (built as the adjugate
scaled by `1/det`), which for an orthonormal basis is just the transpose -
determinant `+1`, structurally incapable of flipping a sign.

**This is explicitly not the handedness question**, and is independent of it.
The PS2's own basis orthonormaliser rebuilds `row0 = row1 x row2`, which is
the same statement as `engine.md`'s runtime-measured
`cross(row0, row1) = row2` on 200/200 PSP ticks - both agree the basis is
positively oriented under ordinary component-wise cross products. Handedness
was never the variable; the sign of `w` was, and the two had been conflated.

**Two independent legs support this**, one of which needs no assumption from
the other at all: the instruction-level integrator read, and separately,
`engine.md`'s own runtime steering measurement (accumulator sign in, observed
rotation sign out, bypassing every intermediate transform) and its weathervane
term (`cross(forward, velocity) * -0.1`, stated as turning the nose toward
travel - backwards under the textbook convention, correct under this one).
Confidence **88** in the PS2 build (raw disassembly, a vendor ISA manual,
memory-read constants, corroborated by a measurement on the *other* binary);
confidence **80** that the PSP build shares the convention, since the PSP's
own integrator has never been located.

**The actionable rule for a reimplementation, stated plainly by the finding
itself: apply exactly one flip, consistently.** Either keep the textbook
`e' = w x e` and negate every torque source read from the disassembly, or keep
the disassembly's torque expressions verbatim and integrate with `e' = e x w`.
**Doing both, or neither, is not a tuning problem - it's unconditional
divergence for the terms affected**, a different and much worse failure mode
than the mild growth-type instability the physics investigation elsewhere in
this file has been measuring. Since that investigation's own repro shows
growth rather than immediate blow-up, that's independent evidence its current
sign convention is already internally consistent - flagged to it as
reassuring context rather than a required change. Two things to note
regardless: **angular damping (`tau = -c*w`) is invariant under either
convention** and must not be "corrected"; and `Body_AddForceAtPoint`'s actual
argument order is `(body, force, point)` - its prologue's register shuffle
disguises this, and an earlier reading had force and point's roles described
backwards (never a behavioural bug, just a documentation one).

## `oag-trace` got its first real comparison, and it changes where the floor-bug fix should look

The agent that built `oag-trace` circled back to attempt a real capture, as
suggested once niri floated PPSSPP's window and the save profile was confirmed
present. **It got one, and it also hit a context limit ("Prompt is too long")
right after committing its work** - same failure mode as the PS2 Ghidra agent
above, and again the work itself landed cleanly (`75fc4dc`) before the process
died; there was nothing left uncommitted to recover this time.

**There is now a documented, reusable reference scenario**: Time Trial, Venom,
Talon's Junction White, Assegai - one ship, no weapons, no AI, 5,178 m, with the
exact menu key sequence from a cold boot recorded in
[`ppsspp-debugger.md`](docs/reverse-engineering/ppsspp-debugger.md#the-reference-scenario).
This is the scenario to reuse for anything that needs to compare against the
original, including the user's stated goal of eventually racing a full race on
both the original and the reimplementation on the same track.

**The first real trace-vs-simulation comparison found the ship falls through
the floor at the first step, at the original's own recorded starting
position** - not after driving for a while. `grounded` reads `1.0` on all 200
real ticks; the simulation's reads `0.0` on every tick but the seeded first
one. Everything downstream follows from that single fact: velocity is out at
tick 1 (max 211 units/s against a recorded 22.7), orientation.forward at tick
1 (max 1.52 rad), and position at tick 3, whose error keeps growing to a max
of 424 units by the last recorded tick (199) - not 423 units by tick 3, which
conflates when the field first crosses tolerance with its eventual maximum
and overstates the symptom's early severity. The control columns
(throttle/brake/steer/airbrakes) all match exactly, so input and scaling are
not the problem - **the entire divergence is the force law failing to find
ground contact where the original does, immediately.** This is a different,
and likely more primary, symptom than the roll-oscillator instability the
physics investigation elsewhere in this file has been focused on - that was
about a ship that eventually tumbles once already grounded; this is about a
ship never grounding at all from a correctly-seeded position. It also gives
the earlier "hover target height exceeds probe cast length" hypothesis its
first real numeric backing, rather than a static reading. Relayed directly to
the physics investigation.

**Two free findings from the same capture, one of which needed a correction:**

- **Handedness settled independently, by a method that needs no simulation at
  all.** The recorded ship travels along `+row 2`
  (`dot(velocity_hat, forward) = +0.997` every tick) and
  `cross(row0, up) = forward` holds 200/200 - so row 0 is confirmed as the
  **left** axis, corroborating the existing 84-confidence roadmap finding
  (which came from the sign of the yaw response to a held steer) by a
  completely independent route. Running the comparison the other way
  (`--basis right-up-back`) is worse on every field, which is the switch
  doing its job.
- **`speed_cached` is not the velocity's length - it is the previous tick's
  forward-projected speed, `dot(velocity, forward)`.** An earlier draft of
  this section called it "the length, one tick stale"; that was wrong, and
  worse, not even a new finding: `docs/ghidra/functions/psp-pulse/engine.md`
  already established the projection at confidence 95, and the error was a
  transcription slip into `psp-trace.py`'s own comment that briefly
  contradicted the project's own page. The capture corroborates `engine.md`
  rather than adding to it - residual mean `3.3e-6` against a speed of 24,
  indistinguishable from exact rather than merely inside tolerance - and
  further pins that the cache is written from **both** vectors' previous-tick
  values (not a mid-update mix): `dot(vel(t-1), fwd(t-1))` matches to `3.3e-6`
  against `1.3e-3`/`1.6e-2` for the two mixed-tick combinations. Scored **90**,
  not 95: the reference scenario is a straight, where the projection and the
  raw magnitude differ by only the 0.3% a `0.997` heading cosine allows, so a
  cornering capture (now on `oag-trace.md`'s "not yet" list) is what would
  separate the two readings for real. The live `speed` field (`+0x398`,
  distinct from `speed_cached`) still matches neither reading exactly (3.67%
  above the length, 4.02% above the projection) and stays an open,
  low-priority recording-model mismatch - not a physics bug, and **it now
  leads the comparison's first-divergence line at tick 0** purely because
  it's compared against the seeded row; read past that line to the tick-1
  group above for the actual physics finding.

## Eight of engine.md's force-law terms now have a second binary, before the agent stalled

Before going quiet without sending its own final report (stopped after
repeated idle checks with no reply - not a context-overflow crash like its
predecessors, just silence), the same agent that resolved the torque-sign
question kept going and identified **9 of `Ship_UpdateCraft`'s 16 PS2
callees**, checking each against `docs/ghidra/functions/psp-pulse/engine.md`
term by term. All the substance is already committed (`14f895d`, `5b3230b`,
`8596e07`, `8590e43`); this is recording it since the agent itself didn't get
to.

**Six terms reproduce in full** - engine, steering, pitch, lateral grip,
weathervane torque, and the still-unnamed track-section force - and two more
mostly do (quadratic drag, three of four coefficients; gravity, structure
only, two details differ). Each of these moves from the decompilation-only
84 cap to **88**, per the rubric's "second binary" leg. Three specific weak
points on `engine.md` got stronger evidence, not just corroboration:

- **The dead `Engine.gain`/`falloff` ramp** - flagged as the kind of thing an
  obvious reimplementation gets wrong - reproduces exactly: the PS2 also ramps,
  clamps, then throws the result away and recomputes thrust straight from the
  input. `85 -> 90`.
- **`slidegrip` as "percent of grip retained"** - previously an interpretation
  of a load-time scale factor - is a **literal expression** in the PS2 build:
  `airbrake * (0.01 - slidegrip) - 1.0`. `90 -> 93`.
- **The PSP's odd `S221` register-carry pattern**, previously scored 65 with a
  recommendation to read it as `accumulator.y = lift`: the PS2 builds an
  ordinary two-lane thrust/lift vector, which doesn't explain the PSP's
  codegen but does confirm the intended semantics. The recommendation moves to
  85; the PSP-specific explanation stays at 65.

**Four real cross-platform differences, none of them corrections to
`engine.md` - each needs the PSP side re-read before anyone decides which
build is the odd one out:**

- The PS2's drag term has **no `-0.9` mode-enum branch** at all.
- The PS2's gravity scale sits on **`flight_gravity`**, not `normal_gravity`
  as `engine.md` currently states for that term.
- **PS2 gravity is scaled by `(1 - magLockBlend)`** - `engine.md` records that
  factor for vertical damping and bank-to-yaw but not for gravity. Taken
  literally, a fully mag-locked PS2 ship has no gravity at all.
- **PS2 lateral grip multiplies both grip coefficients by `1.5`** under the
  same flag bit that gates the turbo add - not recorded on the PSP side.

Two structural notes worth knowing before porting any layout: the PS2's
controls pointer lives at `craft+0x98`, not `+0x78` - same member offsets
inside it, the struct just moved - and **the PS2 has no craft-side force
accumulators at all**: `Body_ClearAccumulators` zeroes the body's four
accumulators directly and every term writes the body, where the PSP keeps a
separate craft-side copy. Same four accumulators, one less layer of
indirection.

**Still not corroborated, and the concrete next step for whoever picks this
back up**: brakes, airbrakes, rolling resistance, and vertical damping remain
single-source at 84; and the fifteen-term *ordering* itself - which is what
the frame-stale-groundedness finding depends on - was never checked, since
every PS2 identification above came from matching callees rather than reading
the call sequence.

## Correction: the floor-fallthrough bug was a wrong track, not a physics bug

**Read this before trusting the roll-oscillator/hover-target sections above at
face value for "why does the ship fall through the floor."** Every fix
recorded in those sections (`TARGET_GLOBAL_SCALE = 0.75`, `ALIGNMENT_GAIN`/
`ALIGNMENT_INERTIA`, the angular sign convention) is real, well-evidenced, and
correct - none of it is retracted. What's wrong is the specific claim, made
earlier in this file, that the tick-0 grounding failure in the `oag-trace`
comparison ("the ship falls through the floor at the very first tick... the
entire divergence is the force law failing to find ground contact") was a
physics finding. **It wasn't. It was `race::DEFAULT_TRACK` pointing at the
wrong track.**

`01_Track` was assumed to be Talon's Junction White - the reference
scenario's track - by directory-numbering convention ("track directories are
numbered" was true, but the number in the directory name is not the same
thing as the front end's own registration order). It is not.
**`16_Track` is**, settled two ways: `Data\Plugins\PI001\Definition.xml`
lists `<PI_Track name="16_Track">` first, at `soundregister="1"`
(`01_Track` is `soundregister="18"`); and decisively, the reference capture's
own 200 recorded positions were cast against *every* track's collision
geometry on the disc - `16_Track` is the only one where all 200 of 200 find
ground, at a mean height of **4.002**, against the **3.978** this project's
own (corrected) hover spring predicts for a resting ship. Every other track,
including `01_Track`, misses outright or lands tens of units off.

**With the right track, the story completely changes.** The ship holds
*both* hover probes in contact for the entire 120-tick well-behaved window
(previously 6 of 120 with both, 83 of 120 with at least one), hover height
steady between 3.8 and 4.4 against a 4.125 target, and the envelope-leaving
tick moves from ~166 to ~257. `grounded` now agrees with the real capture
exactly for the first 29 ticks, where it previously disagreed from tick 1.
**The suspension was never broken. It was being measured on the wrong
ground.**

This is exactly why `craft-update.md`'s roll-oscillator work and the
`TARGET_GLOBAL_SCALE` measurement should still be trusted: they were derived
from direct PPSSPP memory reads and PS2 disassembly, independent of which
track anything was flown on, and the corrected track's measured resting
height (4.002) matching the spring math's predicted height (3.978) to within
0.6% is these fixes being *corroborated*, not undone.

**What's left is real, and now cleanly isolated: the ship has no speed
equilibrium.** The real capture holds 23.6-25.1 units/s at full throttle,
essentially flat or very slightly decelerating. This simulation passes 99.9
by tick 120 and keeps climbing past 115 - it now leaves the track by
outrunning what a turn can hold, not by falling through the floor. From the
recording's own numbers: a steady 24 units/s needs about 4.9 units of net
thrust against drag and rolling resistance; `oag_physics::engine::engine`
produces about **83.6**, roughly **17x** too much.

**Both previously-unrecovered engine multipliers are now found, and both are
ruled out as the cause - narrowing the search rather than closing it.** A
follow-up pass located real writers for both `craft+0x294` and `craft+0x2a0`:

- `craft+0x294` is a **start-line boost**, not a hidden global gain. Three
  writers agree - `Ship_InitCraft`, a per-race reset, and
  `Ship_UpdateStartBoost`, which drives it from a handling-XML group for the
  first second or two of a race and then stores `1.0` back. **Confirmed in a
  second binary too**: the PS2's equivalent at `craft+0x2c4` is also exactly
  `1.0` outside the start window. So `ENGINE_OUTPUT_SCALE = 1.0` is now a
  **recovered** value, not a placeholder - confidence 88.
- `craft+0x2a0` is a **speed-up pickup** multiplier, `1.2`, gated on a pickup
  flag found at its own writer. It's inert on a pickup-free Time Trial capture
  and, being `1.2`, could not explain a *deficit* even if it applied.

**So the 17x factor lives somewhere else, and elimination arithmetic narrows
it to exactly two candidates.** A steady 24 units/s needs
`min(throttle * amount, cap) ~= 2.44` before the engine's final `* 2.0`, but
`cap = 0.5 * 23.35 + accelcap = 11.68 + accelcap` cannot be as low as `2.44`
for any non-negative `accelcap` - so either **`Engine.amount` as loaded is
about 17x smaller than this crate's `0.418`** (an XML-value or load-time-scale
error, in `crate::params`, not in the force law), or **`Engine.accelcap` is
negative for Assegai/Venom** (about `-9.2`), which would mean the cap arm
binds differently than assumed and voids the arithmetic above entirely. The
discriminating check - reading the two actual loaded floats for this ship/
class, against a real disc or a running race - is written down but not yet
done. Nothing has been tuned to close the gap, per this project's own
standing rule against fitting a value to one capture.

**The methodological lesson, worth keeping**: `oag-trace compare` is built to
say *where* two runs diverge, and it did that correctly both times. What it
cannot do is notice its two inputs describe different worlds. A wrong
`--track` produces a clean, confident, entirely misleading physics report,
and three separate force-law explanations got built on top of this one
before anybody thought to cast the recording's own positions at the
candidate geometries and check which one it was actually standing on - a
check that needs no disassembly and takes a minute. `race::DEFAULT_TRACK`
and `crates/trace/src/main.rs`'s `DEFAULT_TRACK` are both now `16_Track`;
`docs/tools/oag-game.md` and `docs/tools/oag-trace.md` are rewritten to
match, and `docs/overview/roadmap.md`'s M3/M4 sections are corrected
accordingly. `the_ship_does_not_stay_on_the_track_yet` still fails on
purpose, for this one remaining reason rather than the three tangled ones it
used to record.

## The thrust-gap diagnosis just inverted: it was never too much thrust

With the real disc values in hand (previous section), the agent redid the
force balance and found the sign of its own investigation was backwards.
**The engine is not producing too much thrust. It is producing exactly the
right amount, and the original agrees.**

At the reference scenario's steady 24 units/s, the thrust cap arm binds and
both this crate and (recomputed from the real, disc-confirmed `Engine`
values) the original's own engine put almost exactly **58 units** into the
force accumulator. Since the recording holds that speed steady, the
*original's* total resistance at 24 units/s must also be about 58. This
crate's implemented resistance there - quadratic drag plus rolling
resistance - comes to **4.88**. **The gap is on the resistance side, by a
factor of about 12, not on the thrust side at all.** Every engine constant
this session has been suspicious of - `ENGINE_OUTPUT_SCALE`, the doubling,
`amount`, `accelcap` - is now positively confirmed rather than merely
unrefuted; the `0.084` fitted a few commits ago was fitting the wrong end of
a balance that was never broken.

**A precise, if probably coincidental, number worth recording**: a grounded
quadratic drag coefficient of `0.1` instead of the current `0.005` would put
equilibrium at 23.6 units/s - the exact floor of the recorded 23.6-25.1 band.
Flagged explicitly as likely a coincidence of magnitude rather than the real
mechanism (the drag coefficients are independently confirmed exact in the
PS2 binary, so simply enlarging one without evidence would repeat the
"tuned to hide it" mistake this project has avoided everywhere else) - but
it pins the actual size of what's missing: something contributing roughly
`0.095 * v^2` of opposing force while grounded.

**A specific, well-motivated next target, not yet checked**: the world force
accumulator has several writers this crate doesn't implement (brakes,
airbrake lateral/slide, gravity, the hover spring epilogue's downforce,
vertical damping) but the **track-section force** (`FUN_08848f9c` on PSP,
`FUN_0015a300` on PS2 - already known to have matching *structure* in both
binaries, per the earlier force-law corroboration pass) is the only one of
them that is both unimplemented and *track-dependent* - which matches this
being a per-scenario discrepancy rather than a universal one. It's a
per-section timed impulse scaled by two never-read per-speed-class tables
(`0x08b36bc0`, `0x08b36bd0` on PSP). Reading those tables is the concrete
next step, and it's Ghidra work, not further physics-crate tuning.

## The engine mystery is resolved: it was never a missing force, it's a missing gate

Following its own advice from the previous section - re-verify the
already-implemented writers, or find a call `Ship_UpdateCraft` makes that was
never enumerated at all - the agent did the second, exhaustively, and closed
the whole investigation.

**`Ship_UpdateCraft`'s complete callee set has now been read directly from
disassembly, not inferred from a step table, and there is no unaccounted
force term in it.** Every hover/engine/steering/pitch/airbrake/drag/grip/
weathervane/damping function is named, plus three previously-unnamed
`Body_*` helpers (`Body_SetMass`, `Body_ClearAccumulators`,
`Body_ClearVelocity`) - none of which is a force term. **The "12x too little
resistance" framing was itself wrong**, not just its candidates.

**What's actually there is an early return in `Ship_UpdateEngine`
(`0x0884c634`) that this crate's `engine()` doesn't implement**: with a flag
bit clear, `craft+0x290 > 0` zeroes the throttle state and returns with **no
thrust and no lift produced at all**, confirmed by reading the branch-likely
delay slots directly (confidence 88). `craft+0x290` is a **collision stun
timer** - `Ship_ApplyLateralGrip` early-returns on the same condition (so a
stunned craft also loses lateral grip and slides), and
`Ship_ApplyCollisionImpulse` (`0x0883f274`) is what arms it: every contact
impulse applied to the body adds `+0.5` seconds to the timer, and repeated
contact keeps re-arming it (confidence 85).

**That closes the arithmetic exactly.** With the real disc-confirmed `Engine`
values, the force law gives `T ~= 58` at the recorded speed - correct,
confirmed twice over now - and *nothing* in the fully-enumerated callee list
supplies that much resistance. But a craft inside the 0.5-second stun window
has `T = 0`, leaving only quadratic drag and rolling resistance
(`0.005 * 24^2 + 2.0 = 4.88`) to decelerate it gently and monotonically -
which matches the real capture's measured `-0.21` units/s^2 almost exactly,
for a craft mass near 23 (a check this session had no other way to reach, not
an independently recovered constant - worth remembering if it's ever cited
elsewhere).

**So: `ENGINE_OUTPUT_SCALE`, the doubling, `Engine.amount`/`accelcap`, and the
drag coefficients were never wrong, at any point in this whole investigation.**
What's missing is a piece of *collision response* - a stun timer armed by
contact impulses that gates both thrust and lateral grip - not a physics
constant or an unread force term. It hasn't been implemented in Rust yet;
that's the actual, final remaining step, and it belongs alongside the
wall-collision-response code from earlier this session, not inside
`engine.rs` alone, since arming the timer is a collision-response
responsibility even though checking it gates the engine. Handed back to
implement and verify against the real capture.

## The track-section force lead is dead - it was a speed-boost pad, not resistance

Following its own next step from the previous section, the same agent read
`FUN_08848f9c` and its two tables. **It's `Ship_ApplySpeedupPad`** - the
speed-pad boost, filled from `<GlobalClass><SpeedupPads amount="..."
time="..."/></GlobalClass>` per speed class - and it only ever pushes the
craft *along* the section direction. It can only add speed. **It cannot be
the missing resistance**, and the one candidate that was both unimplemented
and track-dependent is now ruled out.

**Two genuinely new, previously undocumented damping terms turned up while
reading `Body_Integrate`** for this: a linear velocity damping and an angular
one, both applied inside the sub-step loop after the force accumulators,
both defaulting to `0.01` per the ship-entity constructor (confidence 80).
Real, and worth implementing for fidelity, but confirmed **too small to be
the gap**: at `0.01`, the linear term opposes motion with about `0.24` units
of equivalent force at 24 units/s, against the ~53 that's missing.

**The agent then did something worth naming as good practice**: it stopped
and questioned its own framing before continuing to eliminate candidates.
The whole "resistance is ~12x short" conclusion assumes the 200-tick
(3.33-second) capture is a genuine speed *equilibrium* - but over a window
that short, a slow transient converging toward a much higher equilibrium
looks the same in raw speed range as a true steady state. The only thing
that tells them apart is the recorded *sign*, and the agent flagged this as
unverified rather than assumed. **I checked it directly against the real
trace file** (`data/traces/talons-junction-venom-assegai.csv`, still on
disk): speed goes `24.271 -> 24.264 -> 24.257 -> ... -> 23.620 -> ... ->
23.571` across the 200 ticks, monotonically declining. **It is genuinely
decelerating.** The equilibrium framing holds; this isn't a transient
climbing toward something higher, which would have flipped the whole
diagnosis toward the mass hypothesis instead. Relayed back so the
investigation can continue on the confirmed premise rather than an assumed
one.

**Where this leaves it**: the track-section force was the only lead with a
strong prior (unimplemented *and* track-dependent), and it's dead. What's
left is either re-verifying the already-implemented world-force writers
(brakes, airbrake lateral/slide, gravity, the hover epilogue's downforce,
vertical damping) against real numbers rather than trusting them because
they're already coded, or finding a term `Ship_UpdateCraft` calls that was
never enumerated as a world-force writer at all. Handed back to continue;
not yet resolved.

## The FEGlobals/FEConst indirection is read, and it retires a loose end

The agent that resolved the PSP XML reader question went on to read the
`0x0017c630` indirection the PS2 XML reader work had flagged but not chased.
**`FEConst->` and `FEGlobals->` are the same registry** (`g_fe_globals`,
`0x00301988`), differing only in *when* the referenced name resolves:
`FEConst->` bakes it in at parse time, `FEGlobals->` stores the name's hash
in the value field and sets a per-attribute flag bit so it re-resolves on
every read - a live global versus a parse-time snapshot of one. Registry
values are stored as strings and re-parsed per read, so the same global can
come out as an int to one attribute and a float to another. Two small
corrections along the way: **the sigil is an arrow**, not `>` as both XML
reader pages previously transcribed it (checked against the literal bytes in
each binary separately); and this indirection's float path uses `strtod`, so
unlike the ordinary `Xml_AttributeAsFloat`, **it is exponent-correct**. It
also retires a standing PS2 loose end: the third, previously-unidentified
CRC-32 family the `ps2-pulse` README carried at confidence 55 is this
registry's own string hash.

## The PSP has the same exponent-blind float parser - confirmed, not assumed

A fresh agent located the PSP's generic XML reader (`docs/ghidra/functions/psp-pulse/xml-reader.md`,
new page, anchored from `Movie_ParseAttributes`'s callees the same way the PS2
page anchored from `Handling_ParseStats`) and settled the one open
cross-platform question the PS2 work left behind: **`Xml_AttributeAsFloat`
(`0x0895379c`) is the identical hand-rolled, exponent-blind parser on PSP
too** - every character that isn't a digit, `.` or `-` is ignored rather than
ending the scan, so `1e-5` parses as `-15`. Confidence **95**: unambiguous
decompilation, a second binary agreeing instruction-for-instruction, and every
consumer traced rather than assumed - which is every handling parser
(`ParsePhysical`/`ParseAntigrav`/`ParseEngine`/etc.) and all five camera
parsers. `docs/formats/handling-stats.md` and `docs/formats/fexml.md`'s "no
scientific notation" caveat is no longer PS2-only evidence.

**The same split exists on both builds**: a second, correct accessor
(`Xml_AttributeAsFloatLibc`, a real `atof`/`strtod` wrapper) exists alongside
the blind one, used exclusively by the front-end widget layer while every
gameplay parser goes through the blind one. Neither Ghidra function existed
before this pass - both were only visible in raw disassembly.

## Audio decoded on both platforms - M1's checklist is now fully checked

The agent that decoded PS2 textures went on to crack both of M1's remaining
"unknown" formats: PSP `.bnk` sound banks and the PS2 music archive. Both
close on real invariants across full corpora, confidence 94 (container) /
92 (contents) on each, per the rubric's data-agreement cap.

**PSP `.bnk`**: an `SBlk`-magic container (Sony's SCREAM audio engine is the
strong, not yet confirmed, suspect - the PS2 disc ships `IOP/SCREAM.IRX`, the
PSP has no equivalent file to check against) holding PS-ADPCM waveform data.
Validated on all 39 banks on the disc: the waveform size is stated three
independent times in the header and agrees every time, every stride (cue 12,
command 8, voice 16 bytes) matches its own declared count exactly, and across
546,681 decoded ADPCM blocks the flag/predictor census stays in-spec on
546,450 of them. **What's still missing is where each individual sound starts
within a bank** - the command table is a small per-sound script, not a plain
offset directory, and the one bank simple enough to reverse by hand
(`frontend.bnk`, 10 commands, all one opcode) doesn't generalise to the more
complex race banks. A block-index encoding for that table was tested and
**ruled out** (a `u16` reading of the known boundaries matches only 2 of 7,
which is chance) rather than left unconsidered.

**PS2 music (`PS2MUSIC.WAD`, distinct from the ordinary WAD container despite
the name)**: raw 48 kHz 16-bit stereo PCM, no header worth the name. Two
numbers that aren't stated anywhere in the container were pinned by
cross-referencing the *other* disc rather than by guessing:

- **Which channel is left** can't be read off either autocorrelation or plain
  left/right correlation alone - both are symmetric under swapping the
  channels. Aligning each PS2 track against its PSP ATRAC3plus counterpart on
  the *mid* signal (`ch0+ch1`) and then correlating the *side* signal
  (`ch0-ch1`, which **is** antisymmetric under a swap) settles it: all three
  spot-checked tracks come out around +0.93 to +0.98, where a swapped channel
  order would read around -0.98. Channel 0 is left.
- **The sample rate is 48,000 Hz, not the also-plausible 44,100.** The PSP disc
  carries the same sixteen tracks as ATRAC3plus, which does state its rate;
  at 48 kHz every one of the sixteen PS2 track durations matches a *distinct*
  PSP track to within 11 ms (the residual being the ATRAC3plus decoder's own
  trailing-frame padding, consistent in sign and size across all sixteen); at
  44,100 Hz the match collapses, with one PSP track claimed by eight PS2
  tracks. Two discs encoding the same masters with different tools agreeing to
  four significant figures across sixteen tracks is not a coincidence.

Both formats' "not determined" lists are honest about what's left: per-sound
boundaries and the command opcodes for `.bnk`; track names and whether the
game streams PS2 music directly, for the music archive. `docs/overview/roadmap.md`'s
M1 audio checklist item is now checked.

## The `.fnt`/`.mip` atlas mystery is solved - reading the code beat guessing the bytes

Exactly the untried avenue paid off: rather than another blind statistical
transform, the agent went and read `Texture_SwizzleForGe` (`0x08926da8`,
confidence 90) in Ghidra and got the real algorithm directly. **The dozens of
prior rejected transforms were all solving the wrong problem** - the header
itself was misread (palette is at `+0x40`, texels at `+0x80` of a 64-byte
header, not the offsets earlier attempts assumed), and whether a texture is
swizzled **at all** is a flag bit, not a fixed transform to find. Confidence
**94**, on all five `.fnt` files: edge-ink (are both declared-tight edge
columns of a glyph box actually inked) jumps from 0.42 under the old reading
to 0.97-1.00 under the new one, and all five fonts now render as legible,
previously-unseen artwork - digits, the alphabet, accented capitals, and the
PSP's own L/R/START/SELECT/HOME/face-button glyphs.

**One thing that should have been the tell sooner**: reading the palette 48
bytes early (the old, wrong offset) gave only 4-5 distinct alpha levels with a
dozen dead entries, and `small.fnt`'s brightest visible entry came out as
alpha 26 of 255 - not enough to draw anything legible with, on any pixel
layout. The right offset gives all five fonts a clean 16-level antialiasing
ramp.

**This wasn't just a font bug.** Chasing the swizzle flag found that **6 of
`FE.wad`'s 13 standalone `.mip` textures set the identical flag in their own
header** and were being silently decoded as noise - the atlas was never a
special case, it was the general mechanism showing up first. Both `fnt.rs`
and `texture.rs` now share one unswizzling routine, with a ground-truth test
verifying seam-free decodes.

**Still open, and none of it blocks rendering**: the `+0x14` field (0 in
three fonts, 4 in two, correlating with which are missing a codepoint-table
terminator and which use a black rather than white palette - a version
marker is the best guess); the five mostly-`0xff` bytes at `+0x0d` of a
glyph record (kerning, guessed); and **why some assets ship pre-swizzled and
others don't**, which is now the interesting remaining question rather than
"what does the swizzle look like." Wiring the real atlas into the front end
(currently `crates/game/src/font.rs`'s own 5x7 fallback) is noted as a
separate, not-yet-done change from decoding it.

## The real font atlas is wired into `just play` - text finally looks like the game

Following straight on from the `.fnt` swizzle solve, the same agent replaced
the front end's 5x7 placeholder glyphs with the real decoded atlas. Verified
visually: the Language Selection screen now renders `Français` and `Español`
with their actual accents, in the disc's own proportional typeface, not blocky
5x7 pixel text. Three implementation details worth knowing:

- **The atlas stores coverage, not colour** - a 16-level alpha ramp over one
  flat RGB (white for the menu fonts, black for the two HUD ones) - so it's
  uploaded as a single `R8` channel into the existing pipeline rather than
  needing a new bind group.
- **It's padded with a deliberate 2x2 opaque patch** in two extra rows beyond
  the font's own pixels, because the UI pipeline draws solid fills by
  sampling an opaque texel from the same texture the glyphs come from - a
  single-texel patch would bleed under the linear filtering a real
  antialiased font needs (the built-in 5x7 fallback still samples nearest,
  correctly, since smoothing hard-edged pixel art turns it to mush).
- **The fallback is preserved, not replaced** - `boot::load` still drops to
  the 5x7 set (with a line in the boot report saying so) if the real font is
  missing or fails to decode, so a rendering bug stays distinguishable from a
  loading one. The accent-fold logic (case fold across Latin-1, then base
  letter, so `Français` degrades to `Francais` rather than the letter-dropping
  `FRANAIS`) now only runs as a second-choice path, since a real disc font
  usually already has the accented glyph directly.

## Wall collision response is real and validated - and it surfaced exactly why the ship still can't fly a lap

A genuine new physics subsystem, not a bugfix: ships now generate contacts
against `Wall`-class geometry (previously only the vertical hover probes ever
queried the collision world at all) and respond with a real restitution-based
projection, resolved after integration with swept-ray detection so a
fast-moving hull can't tunnel through the zero-thickness wall shells. The
`-1.0` "never bounce" sentinel is handled correctly rather than as a literal
negative restitution. Validated with a dedicated ground-truth test
(`a_ship_thrown_at_a_real_track_wall_does_not_pass_through_it`) against real
disc geometry, plus unit tests pinning the force-law evaluation cost and
constant per-tick cost. All 308 tests across `oag-physics`/`oag-gameplay`/
`oag-game`/`oag-render` pass; clippy is clean.

**It also did something nobody could have predicted from inside a sandbox
with no disc access: it made the standing `the_ship_does_not_stay_on_the_track_yet`
ground-truth test start failing** - which, per that test's own comment, is
supposed to mean "the physics improved, delete this test." **Read the actual
result before believing that literally, which is exactly what happened here.**
Checked directly (this agent couldn't check it itself - no real disc in its
sandbox, see the CLAUDE.md note added earlier this session):

```
tick  30  speed  19.88   tick  90  speed  79.62   tick 150  speed 111.72
tick 180  speed  84.16   tick 210  speed  32.20   tick 240  speed   4.76
tick 300  speed   2.37   position barely moving from tick 210 onward
```

**The ship is not flying a lap. It slams into a wall at roughly 4x the
recording's real steady speed (still the ~12x-too-much-thrust bug from
earlier this pass, unresolved) and gets stuck there, nose-first, at a
near-standstill.** A screenshot at tick 300 shows it exactly that way. The
test's formal check - stays inside the envelope, stays finite - is
technically satisfied, because a ship wedged into a wall neither leaves the
track nor blows up. That is not what the test or its surrounding
documentation mean by "the physics improved," so **the test and the
known-limitations section in `docs/tools/oag-game.md` are deliberately left
in place, not deleted**, even though a literal reading of the test's own
"when this fails, delete it" comment would say otherwise. The honest
one-line status: the suspension is fixed (confirmed twice now, independently),
wall collision is now real and correct, and the ship still cannot complete a
lap, for a third, already-identified and separately-being-chased reason
(the engine thrust magnitude bug). Whoever resolves that bug should re-run
this exact test and this exact screenshot before deciding the module
documentation's "it cannot fly a lap" framing is actually retired.

## Reset zones now respawn the ship, a genuinely new feature

Continuing straight on from wall collision, the same agent implemented the
other collision-response gap: touching `Reset`-class geometry now respawns
the ship, rather than doing nothing (Reset was already excluded from ordinary
raycasts, per `collision.md`'s confidence-86 reading, but nothing consumed a
contact against it). Detection queries the `Reset` `TriangleSoup` directly
rather than through the ordinary hover-probe path, specifically so it isn't
masked by a floor collider sitting in the same place. The respawn logic
itself has real safety margins that weren't strictly necessary for a debug
feature but are the right call for a live one: a 30-tick cooldown between
respawns, and a hard stop after 5 consecutive failed respawns rather than an
infinite loop. A ground-truth test
(`touching_a_real_reset_volume_respawns_the_ship`) was written alongside the
code but not committed - the agent can't run tests needing real disc data in
its own sandbox - so it sat uncommitted until verified and committed
centrally. It passes against the real PSP disc. 319 tests across
`oag-physics`/`oag-gameplay`/`oag-game`/`oag-render` pass, clippy clean.

## Walls also render in `just play` itself now, not only in `oag-view`

A correction to how the debug view above was first described: `oag-view
--collision` is a separate tool from `oag-game`, and `just play`/`just play
--race` did not draw collision geometry at all - only the driveable ribbon or
the art meshes. Wired directly (by the orchestrating session, not an agent):
`oag-game --collision` now overlays the same collision soup on top of a live
race, reusing `oag_render::collision::build_model` with no new rendering
code. `Options`/`Loaded`/`Scene` all gained a plain `Option<T>` field each;
nothing existing changed shape. Verified with a real screenshot against the
`16_Track`/Assegai/venom reference scenario - walls, floor and the mag-floor
patch all render correctly alongside the ship, and all 83 `oag-game` tests
still pass.

## The sweep-and-prune ±1024 contradiction is resolved - by measuring, not arguing

This page used to say "both cannot be right as stated" about the broadphase
packing (`bits[0:11] = ((int)coord + 0x400) * 2`, apparently ±1024) against
world extents that reach 1,335.8/1,554.1. **A full survey of all 16 track
environments on both discs, using `oag-view --collision`, turns that into a
falsifiable prediction instead of a standing contradiction.**

Two facts, and they only make sense together:

1. **The packing window is 2048 units, not 1024** - twelve bits hold
   `0..4095`, so `coord + 1024` runs the full `0..2047`. The earlier ±1024
   phrasing conflated the offset with the window.
2. **Seven of sixteen tracks individually exceed ±1024** - so the two
   whole-disc maxima this page used to quote (1,335.8/1,554.1) were never an
   outlier, they were just the two largest of seven ordinary tracks. **But
   not one of the sixteen exceeds a 2048-unit *span*** - the widest,
   `10_Track`, comes in at 2026.6, precisely 99% of the window with 21 units
   to spare.

A designer budgeting to exactly 2048 units and a track landing at 99% of it
is not a coincidence. So the resolution favours the packing being right and
the *input coordinate* not being raw world space: **the broadphase likely
packs relative to some per-level origin** (the level's own bounding-box
minimum, most plausibly), not the world origin every other subsystem uses.
**This is recorded as a prediction, not a finding** - nobody has re-read
`Sap_Init` yet - but it's now a specific, checkable one: a base subtracted
before the `+0x400` confirms it; a raw world coordinate with no base means
the packing reading itself is what's wrong instead. Confidence 75 on the
prediction; the 32 real measurements behind it (16 tracks x 2 discs) are
direct.

**Two bonus findings from the same survey**: collision geometry is
byte-identical across platforms for every one of the 12 environments both
discs share (same reach, same span, to the decimal), and a track's four
variants (forward/reversed/zone/zone-reversed) all share one collision set
- except PS2 `11_Track`, whose zone pair reaches 1132.0 against the ordinary
pair's 1129.2, worth knowing before assuming the variants are always
interchangeable.

**The same agent then went back and found its own survey undercounted, before
letting the conclusion stand.** The 16-track table above was built by probing
known track *names*, which found 40 PSP and 55 PS2 files against a census of
40 and 59 `Floor Collision` nodes - four PS2 collision-bearing files had no
recovered name and were silently missing from the table, which is exactly the
kind of gap that would later get treated as "measured everything" when it
hadn't been. Fixed by walking every `.vex` entry of both discs **by index**
instead of by name - genuinely exhaustive, no name recovery required. Across
**1,038 PS2 files and 340 PSP ones**, every collision node either disc ships:
the widest span anywhere is **2026.6057**, on both discs, and it's the same
track (`10_Track`) the named survey already found. The conclusion holds under
full coverage, not just under the sample. Three things in the original
write-up were quietly overstating and got walked back to match the actual
evidence: "the broadphase does not pack world space" narrowed to "*if* the
packing reading is right, its input cannot be raw world space" (a
conditional, not a determination); a clarification that both figures are the
*world*-level broadphase's input specifically, not the per-mesh level, which
nothing here speaks to; and the load-bearing number stated to full precision
(2026.6057 of 2048, 98.96%, clearing by 21.4 units). Also turned up a fifth,
previously unlisted track variant name in the process:
`11_Track\track_zone.vex`, found by hashing candidates against the directory
rather than assumed.

## Track walls render now, as a debug view - and it caught its own bug

`oag-view --collision <track.vex>` draws the collision triangle soup (Wall,
Floor, Reset, Mag Floor - Cage excluded, matching the PSP loader's own
branch-past behaviour) in wireframe or solid, coloured by surface class, with
a `--with-spline` overlay to check it against the driveable ribbon. No new
decoding was needed - the geometry was already loaded into the live
`CollisionWorld` at runtime; this is purely a render-side addition, kept
deliberately out of `crates/physics`/`crates/gameplay`/`crates/game` to avoid
the active physics investigation elsewhere in this file.

**It corrected itself once, honestly, which is worth recording as a method
note.** The first version of `--with-spline` compared the racing line's
bounding box against the *wall* geometry's box alone; walls are vertical
strips no taller than the walls themselves, while the spline sits
`track::HOVER_LIFT` above the floor, so a **correct** track would have
printed "NOT fully inside the walls" and read as a decode bug. Caught and
fixed in a follow-up commit: the check now compares against all collidable
geometry with hover-height slack, and is honestly worded as a coarse
sanity check rather than a proof of enclosure.

**One genuine, if modest, contribution to the standing sweep-and-prune
puzzle**: the tool prints each track's furthest reach from the origin and
flags anything past ±1024 explicitly, so the ±1024-packing-vs-1554-unit
contradiction collision.md has recorded since earlier this session is now
one command away from being checked **per track** rather than only as a
whole-disc maximum - nobody has run it across every track yet, but the
instrument now exists.

**Caveat the authoring agent flagged and I verified directly**: its own
sandbox has no access to `data/images/` (only `data/README.md`), so it could
not run its own tool against a real `.vex` - `collision::load`,
`report_collision`, `--with-spline` and the ±1024 warning were all untested
against real data when it reported. Run directly against the real disc from
outside that sandbox: `oag-view 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad'
--collision 'Data\Environments\16_Track\track.vex' --with-spline --screenshot ...`
works cleanly - correct per-class colouring, the spline-enclosure check
passes, and it gives the first real data point for the ±1024 question:
**`16_Track`'s own collidable extent is 856.2 units from the origin**,
comfortably inside the bound, so the reference track is not one of the ones
that trips the contradiction. Whichever track(s) reach the recorded 1,335.8/
1,554.1-unit extents are still unidentified - this is one data point, not a
survey.

## PS2 textures found, decoded, and rendering - M1's last real gap is closed

A fresh agent (replacing the one that decoded PS2 mesh geometry, after it hit
the same context-overflow failure noted above) went looking for where PS2
textures actually live, since the embedded block in a PS2 `.vex` file is
always length zero. **They are standalone WAD entries, never embedded**, and
a model's set is additionally bundled into a small nested WAD (same 8-byte
header, 16-byte entries as the outer archive) keyed by a hash that has not
been recovered - so `oag-view` takes the entry explicitly for now rather than
finding it automatically. Confidence **94**, on invariants checked across all
**5,348** real PS2 textures across both WADs: a dimension declared twice (a
packed log2 byte and explicit width/height words) agreeing on all 5,348;
both GS `GIFtag`s declaring their own payload size in a way that has to match
the dimensions' implied byte count, also 5,348 of 5,348; and the total blob
size closing by formula on 5,348 of 5,350 candidates.

**The format is a GS upload DMA packet, not a texture file** - `TRXPOS`/
`TRXREG`/`TRXDIR` register writes framing two GIF transfers (texels, then
palette), which is real PS2 hardware behaviour rather than anything specific
to this game. Two findings worth knowing if this code is ever touched:

- **The dimension words are stored height-first**, which only shows up because
  `TRXREG` is a fixed, asymmetric function of the true dimensions - reading
  width-first decodes every *square* texture correctly (most of them) and
  scrambles every other one silently.
- **Most textures (5,077 of 5,348) are 8-bit-indexed data pre-swizzled into
  PSMT8 order so it can be uploaded as PSMCT32**, the standard PS2 trick for a
  format the GS has no fast host-upload path for. The swizzle permutation is
  only well-defined for widths of 16 and up, and **the widths where it breaks
  down are exactly the widths the game switches to a direct linear transfer
  for (4 and 8)** - the arithmetic predicting the same fallback set the data
  actually uses is a corroboration, not a coincidence. The palette is in
  `CSM1` order (a bit-swapped index within each group of 32) and alpha runs
  0-128 rather than 0-255, the same GS convention already found for PS2 vertex
  colour.

**It renders.** `oag-view --mesh 'Data\Ships\Feisar\Ship.vex' --textures
0xfc3f75cf` against `WADS2.WAD` draws the Feisar ship in its actual blue and
yellow livery, team name legible, UVs landing on the right panels - not just
"doesn't crash," but recognisable, correctly-oriented artwork. `docs/overview/roadmap.md`'s
M1 exit criterion is updated from "met on geometry" to **met**, full stop, on
both platforms.

**Not determined, and not blocking**: the PSMT4 swizzle (5 of 5,348 textures,
refused rather than guessed at); what the two unexplained fields at `+0x08`/
`+0x0c` are (look like a GS memory allocation the file has no reason to fix,
not needed to decode); and the texture-set lookup mechanism above. Also
recorded as a live discrepancy: a PS2 `.vex`'s own `Texture` node payloads are
stale PSP-shaped leftovers (mip counts, sizes) that disagree with the actual
loaded texture entirely - the Feisar node claims 512x512 8bpp with 6 mips,
the real entry is 256x256 with one level and the PS2 blobs carry no mips at
all.

Also landed in the same commit: `scripts/psp-trace.py` gained `--ship`/`--craft`
selection, because a race breakpoints on `Ship_UpdateCraft` once per ship per
tick with a different craft each time - the previous capture logic stopped at
the second address, which is fine for a one-ship Time Trial but makes a race
capture meaningless without picking one craft to follow. Found in the process:
**the player's craft is the one whose throttle is exactly 0 or 100** (digital
input), where AI craft read fractional throttles; and only four of eight craft
were updated per tick in one race capture, for a reason not yet established.

## `oag-trace` exists now: M3's missing comparison side is built

A new crate, `crates/trace` (`oag-trace`, lib + bin, added to the workspace),
closes the gap [the roadmap](docs/overview/roadmap.md) and this file both
flagged as the top priority: **the instrument that reads a trace and a
simulation run and reports where they diverge, not whether they match.** 38
tests, all against hand-authored fixtures (real traces are derived game data
under `data/traces/`, gitignored, so nothing here can run in CI - see
[the tool's page](docs/tools/oag-trace.md) for why). Clippy, fmt, rustdoc and
`just check-docs` all clean; the full workspace gate was not run, deliberately,
per this pass's rule about not running `just` while other agents have crates
open.

`oag-trace show|run|compare`: `show` summarises a capture (tick count, dt
distribution in Hz, distance travelled - which is exactly the false-start
stall trap from earlier this session - and how well `cross(row0, up) == forward`
holds); `run` seeds a `World`/`Ship` from a recording's first row and replays it
through `oag_physics::step`/`oag_gameplay::ship_controls`, sampling **before**
each step to match where `psp-trace.py`'s breakpoint actually sits (craft
entry); `compare` diffs two traces on the 13 fields
[the protocol](docs/reverse-engineering/verification-protocol.md) specifies,
with the right tolerance per field (position 0.01 abs OR 1e-4 rel, orientation
1e-4 rad, velocity 1e-3 rel, `grounded` exact), and reports **the first
divergent tick, the field, the magnitude, everything else that moved on that
same tick, and a trend** - `exact`/`bounded`/`growing`/`shrinking` from a
least-squares slope plus a first-quarter-vs-last-quarter mean comparison - so a
bug that is small-but-growing doesn't hide behind a tolerance that only checks
magnitude.

**One thing worth knowing before trusting a `compare` run: a naive
tolerance check would have called a `NaN` a perfect match on every field**,
since every comparison against a non-finite value would read as "within
tolerance" by omission. The advisor that reviewed this crate's design caught
it; non-finite is now always reported as a divergence, never silently accepted.

**A sharper version of the handedness finding**, tightened while building the
replay path: the recording's own right/up/forward rows are a positively
oriented basis, while `Body`'s convention is not (`forward` is `-Z`), so the
mapping between them needs an **odd** number of sign flips, not one chosen
arbitrarily - flipping only row 0, as a naive port might, is a reflection that
`Quat::from_mat3` will silently turn into nonsense rather than error on.
`oag-trace run --basis left-up-forward` (measured, and the default) versus
`--basis right-up-back` exist as two concrete, opposing hypotheses; running
both against a real capture is a one-command way to settle
[the 84-confidence cross-product claim](docs/ghidra/functions/psp-pulse/engine.md)
that this session's earlier physics work left open. Nobody has run that
comparison yet - it needs a real capture, see below.

**No real emulator capture was attempted, on purpose, not because of a missing
piece.** `PPSSPPSDL`, a live display, and `data/images/pulse-psp-usa.chd` are
all present and usable; capturing one needs an interactive GUI session driving
Pulse's own menus on the user's actual desktop mid-race (and the SDL build
throttles when its window loses focus), which is a different kind of task from
everything else this pass did unattended. Doing that capture and then running
`oag-trace run`/`compare` against it is now genuinely one session's work away,
and it is the highest-leverage thing to do next: it would settle the handedness
question above and give the M4 physics work (see the fall-through-floor
investigation elsewhere in this file) its first real measurement instead of a
static reading.

**What the capture format still can't see, documented rather than faked**:
angular velocity, pitch, the landing/leap timer, and raw stick position aren't
columns `psp-trace.py` records, so `compare` can't diff them; a dropped tick is
invisible too, because the capture numbers rows by counting breakpoint hits,
not by reading a tick counter. There's no committed input script or save state
yet either, so every scenario run so far has been ad hoc rather than one of
[the protocol's seven](docs/reverse-engineering/verification-protocol.md).

**One coupling to know about before changing force-law code elsewhere**: the
crate's own coast-down self-test assumes rolling resistance and quadratic drag
are *forces*, using a large test mass to divide their contribution down to
near-zero. If a future physics change turns either term into an acceleration
instead, that test will start failing - correctly, as a signal that the
contract changed, not as a harness bug to silence.

`data/cache/pulse-psp-usa.iso` (457 MB) is the raw ISO the emulators want,
extracted with `just extract-iso`. It is gitignored twice over and it is a real
4 GB-class copy on `ecryptfs`, so keep it rather than re-extracting.

## The original is now scriptable, and it corrected four things

[`docs/reverse-engineering/ppsspp-debugger.md`](docs/reverse-engineering/ppsspp-debugger.md)
is the new load-bearing page. PPSSPP's websocket debugger gives breakpoints,
memory, registers, disassembly, threads and **scripted input**, and
`scripts/ppsspp_debugger.py` wraps it with the traps encoded rather than
described. `scripts/psp-trace.py` captures a per-tick CSV off a running race
(`just trace --ticks 300 --hold cross --warmup 4`).

**Four traps, each of which cost real time.** They are on that page in full, but
the two that will bite hardest:

1. **A breakpoint added while the CPU is running never fires.** It is accepted,
   it appears in `cpu.breakpoint.list`, and nothing happens - under the JIT and
   under `--ir` alike. Break first, then add, then resume.
2. **Changing the breakpoint list rebroadcasts `cpu.stepping` with the old pc**,
   which a naive client reads as an instant hit at 0.00 s. Three rounds of tests
   reported phantom hits before this was clear. Match on the pc.

And one that decides the architecture: **a memory read costs ~520 ms while the
CPU runs and ~0.3 ms while it steps.** Per-tick capture must be breakpoint-driven
with one batched read per hit. Polling a running CPU at 60 Hz is not slow, it is
impossible.

What that instrument then found:

- **`Ship_UpdateCraft`'s call site**, which engine.md's confidence was capped on
  and the roadmap listed as open: `0x0884ff70`, a virtual call through slot
  `0x70` of the vtable at `object+0x38`, inside a per-entity update loop at
  `0x0884f70c`. The enclosing function is deliberately **not renamed**.
- **Its register assignment was wrong.** The craft is `a0` and the body is `a1`,
  not `a1`/`a2`: `a0+0x1cc` holds exactly the value in `a1`, and the craft's `dt`,
  `grounded` and throttle all read sensibly off `a0` and as garbage off `a1`.
  Anything written against the old signature reads the wrong structure.
- **Delta time is variable, and its mean is `1/59.94`.** 72 distinct values over
  120 ticks, mean and median `0.016683`. That confirms ADR-0007's premise and the
  vblank-locked presentation finding in one measurement.
- **The three fixed-timestep globals are still zero after real racing**, closing
  frame-pacing.md's "a write through an unresolved pointer cannot be excluded"
  hedge as far as it can be closed. 88 -> 93.
- **`0x08b31784` is a pointer to the state machine, not the machine.** The state
  name at `+0x18c` is an inline buffer, proved by catching `"Show Logo\0election"`
  - a short name overwriting `"Language Selection"` in place. Reading it is a
  text-mode view of the front end and is how the menus were navigated.

### The false start, which reads exactly like a bug and is not

Most of the captures this pass were of a **stalled** ship, and the trap is worth
knowing before it costs someone else an hour. Pulse penalises a false start by
flooding the engine, and holding thrust through the countdown is what a script
does by default. The symptom: the throttle field reads the full 100 you are
sending, the cached speed climbs to 10 and resets, and the ship does not move at
all. That reads like a broken input path or a wrong position offset. It is
neither.

The cure is a restart with nothing held: `start`, `down` x4, `cross` for RESTART
RACE, then ~25 seconds of no input at all, then capture with thrust. That
produced the trace this pass measured from - 200 ticks, 73.85 units travelled,
speed steady around 22 - and it is what settled the cached-speed staleness
199/199. The scenario is Time Trial, Venom, Talon's Junction White, Assegai, and
the profile save now persists, so the first-boot walk never has to be repeated.

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

   **Settled since, against the original running** - see
   [camera.md](docs/ghidra/functions/psp-pulse/camera.md). The disc's
   `pos_length` is a **signed offset along the ship's forward axis** (negative =
   behind), on the same convention as `pos_height` and `lookat_length`, so
   `eye = position + forward * pos_length + up * pos_height` uses the file's
   value unchanged. Measured off a breakpoint in `Camera_UpdatePlayerView`:
   the eye's projection on the ship's forward row is **negative** for both
   external views and **positive** for the internal one, matching
   `<InternalCamera length>` exactly. So the current behaviour is right and the
   two negations must not both be removed; what is wrong is only `chase.rs`'s
   prose. **The Rust was deliberately not changed.** The minimal tidy-up is one
   sign flip in `oag_render::camera::chase::anchor` plus deleting
   `race::chase_pos_length`, with no behavioural difference; do it as one commit
   and check a screenshot is byte-identical across it.

   One new open question came out of the same measurement: the original puts the
   eye at exactly **3/4** of the authored external offset, at rest and at speed
   alike (seven samples over 541 world units, ratio `0.7500` every time), and
   **where that factor comes from was not found**. Do not hardcode it.
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

## `just play` now ends in a race, and getting there settled three conventions

The front end and the race were two disjoint entry paths chosen by `--race`, each
with its own window, event loop and wgpu device. They are one now: `oag-game` opens
one window, and the front end reaching **`Launch Game`** - the state picking a
language already fired - loads the track and hands the window over. `--race` is the
same race entered without booting the front end. There is no menu in between,
because there is no menu: the root XML's `MainMenu_Definition.xml` is unbuilt, so
`Launch Game` starts one race on whatever `--track/--team/--class` name.

Verified end to end three ways: a headless capture that follows the handoff
(`just play --screenshot ... --until "Launch Game" --press start,cross --ticks 60`),
a `#[ignore]`d ground-truth test that boots off the disc and flies what `Launch Game`
starts, and the real window driven by a synthetic keyboard through `/dev/uinput`
under niri, with the injection guarded on the compositor reporting the game window
focused. That last technique is worth keeping: it is the only way anything in
`main.rs` has ever been exercised.

**Three conventions were settled at the boundary, and one hypothesis was closed.**

1. **A `.vex` ship is authored nose along `+Z`** while `Body::forward` is `-Z`, so
   the ship was drawn facing backwards and the chase camera - correctly behind the
   body - showed it head-on. `race::MODEL_YAW` composes the half turn in
   `ship_model_matrix`, at the body-to-model boundary and **not** in
   `oag_render::mesh`, so `oag-view --mesh` still shows an asset in its own space.
   Confidence 85, measured: sliced along `z`, the hull is 5.3 units across at `-Z`
   with the model's full height and 1,175 of its 1,334 vertices, and tapers to 0.9
   across at `+Z`.
2. **The hypothesis that follows from it is false, and this is the useful part.**
   If any body-space offset from disc data were signed along `z`, the same flip
   would apply to it - which would have been a candidate explanation for finding 3
   above, single-probe contact. It is not: `oag_physics::hover::probe_offsets` is
   `[-half, +half]` about the centre from an **unsigned** `<Misc length>`, and a
   symmetric pair is invariant under a half turn. Grepping `oag-physics` and
   `oag-gameplay` for any other body-space offset built from handling data finds
   none. So the `+Z` finding has no consumer in the simulation today and explains
   nothing about the wobble. **Do not go looking for one.**
3. **The field of view is authored for one viewport shape and only one.** The PSP
   renders 480x272 and nothing else, so what a differently shaped window should show
   is a presentation choice, made in `oag_render::camera::fit_vertical_fov`: at the
   authored aspect and wider, the value is used unchanged; narrower, the vertical
   field opens up to hold the authored horizontal field. Without it a tiling
   compositor's portrait slot crops the track edges away and the camera reads as
   broken when the data is fine. It carries no confidence score because it is not a
   reading of the game, and it is constrained by a check rather than by trust: a
   capture at 1440x816 is **byte-identical** across the change. The window also asks
   for a fixed size now, which is what makes niri float it rather than tile it.

`--size WxH` renders a capture at any shape, which is how the above is checked with
no compositor involved.

**Still not wired.** No menu, no HUD, no lap counting, no restart and no pause: the
only way out of a race is Escape. The player route still draws the **diagnostic
ribbon** by default rather than the track's art meshes, and `--art` is
transformatively better to look at - that is a deliberate default from when `--race`
was a developer tool, and it is worth reconsidering now that `just play` leads here.

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

**That was a falsifiable prediction, and this pass falsified the mechanism behind
it.** Measured off a running race: `cross(row0, row1) = row2` exactly on 200 of
200 ticks, so the original's basis is positively oriented under the ordinary
component-wise cross product - the same arithmetic we use. There is no
frame-handedness difference available as the common cause of those two signs.

What the same measurements did settle is sharper and more useful. Holding left
gives `steer = -96` and `+1.51 rad/s` about row 1; holding right gives `+96` and
`-1.42`; 199/199 and 0/199 consistent, mirror-symmetric in sign and magnitude, so
it is steering rather than collision. Two consequences: **row 0 is the left
direction, not the right one** - it is named from use sites that fix the axis but
not its sign, and a reimplementation mapping it onto `+x = right` steers
backwards and puts the lateral grip term the wrong way with it - and any
reading of the angular accumulators now has a constraint to satisfy, namely that
`yaw = steer * Turning.amount` into local `.y` must come out as `-k * steer`
about row 1, `k` about `0.0155` at 22 units/s. If those two torque terms really
do need flipping, the sign lives in how accumulators are applied to the body, not
in the coordinate frame. See
[engine.md](docs/ghidra/functions/psp-pulse/engine.md).

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

**Runtime-verified this pass, against the game actually running:** the variable
timestep and its `1/59.94` mean, the three dead fixed-step globals, the racing
`display+0x116c` flag, `secondsPerTick = 1e-6`, the `Ship_UpdateCraft` call site
and register assignment, and the state machine's pointer-and-inline-buffer
layout. This is the first evidence in the repository that came from execution
rather than from reading.

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
| **Unverified lead: the same sRGB trap may be in `mesh_render.rs`** | The front end's sprite sheet had to have its texture format follow the *target's* colour space, because sampling an `Rgba8UnormSrgb` texture returns linear and the headless capture targets `Rgba8Unorm` on purpose. It cost the logo two thirds of its brightness and looked like art direction, not a bug - `(36, 147, 153)` in the texture came out `(5, 74, 81)` in the PNG. `crates/render/src/mesh_render.rs` has the same shape: `Rgba8Unorm` target at line 91, `Rgba8UnormSrgb` textures at 387. **Not checked** - a textured `oag-view` capture next to a window would settle it in a minute. |
| `Viewport`, and where an `Image` with no `x` goes | The front end draws `Image src=` widgets now, so `--screen "Show Logo"` shows the Pulse logo and "Press START button". Two gaps behind it: a `Viewport` is a clipping rectangle nobody has decoded, and `screen.rs` reads only a screen's direct children, so `Show Logo`'s `BOOT_LEGAL` line - which is inside one - is silently absent. And an `Image` with no `x` is centred on a guess: the only case on that screen is 512 wide on a 480-wide screen, where centring and left-pinning differ by 16 pixels and the picture cannot tell them apart. |
| **The Memory Stick and profile system** | **Scope decision, 2026-07-27, not a gap.** One automatic save slot, the way a modern PC game does it. The disc's boot chain between `Language Selection` and `Main Menu` is eight screens and all but two exist to serve Memory Stick mechanics: `RemoveMemoryStickWarning`, `MemoryStickWarning`, `NoStickAtBootScreen`, `DisplayInsufficientSpace`, `LoadProfileDialog`, `AutoLoadProfileCheckScreen`, `AutoLoadingProfileScreen`, the `LoadProfileFailed`/`LoadProfileDataCorrupt` arms, and the first-boot `NameSetup2FromBoot` -> `TagSetup2FromBoot` -> `CreateFromBoot` chain. None of it is to be built. When M6 opens, model `LogoFMV`, `LogoFMVRedirectScreen` and `Show Logo`, then go straight to `Main Menu`. Do not read the omitted screens as unimplemented work. |

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
breaks at start; it **is** now used, and everything about it lives in
[ppsspp-debugger.md](docs/reverse-engineering/ppsspp-debugger.md). Two things
that page will save you rediscovering: headless cannot get past Pulse's
first-boot name entry (no persistent memory stick), so use `PPSSPPSDL` with
`RemoteDebuggerOnStartup`; and the SDL build throttles to a crawl when its window
is unfocused, which looks exactly like a hang.

### Ghidra

`BOOT.BIN` must be loaded with the **Allegrex** processor module, not stock
`MIPS:LE:32`, or the decompiler builds confident fictional C from VFPU
instructions. `just build-allegrex`, needs JDK 21.

Image base `0x08804000`, confirmed against PPSSPP's own loader log and against
`BOOT.BIN`'s ELF program headers. The Ghidra project still lives in the repository
root (`OpenAntiGrav.gpr`) rather than `data/ghidra/`; moving it is safe once
closed.

## Session boundary: stopping here on purpose (usage window), not because anything is stuck

Both physics-side agents finished and committed cleanly moments after the
previous edition of this note was written (which had caught them
mid-diff - superseded, corrected below). No new agents were spawned per the
user's instruction; whoever resumes should either reconnect to the named
agents below if the session is still live, or brief a fresh one from this
description - don't re-derive any of it from scratch.

### `engine-scale-re` - the collision stun gate is implemented, real, and does NOT close the observed gap

Committed as `21798c6`. `ShipState::stun_timer` (`crates/physics/src/ship.rs`),
armed by `crates/physics/src/wall.rs`'s `wall::resolve` (`STUN_PER_CONTACT =
0.5`, added not assigned, only on a genuine inward impact so sliding along a
wall doesn't cut the engine every frame), and gated correctly in both
`engine()` (no thrust or lift while the stun or leap timer runs) and lateral
grip (`crates/physics/src/forces.rs`, decremented at the same step the
original does it, so the engine reads the pre-decrement value in the same
frame). Three real tests pass, `cargo nextest run -p oag-physics -p
oag-trace` is 223/223 green.

**I re-ran the actual verification this was built to satisfy, and it does
not close the gap.** `cargo run -p oag-trace -- run
data/traces/talons-junction-venom-assegai.csv --source
data/images/pulse-psp-usa.chd --team Assegai --class venom --hold cross`
still diverges exactly as before the fix - speed climbs to 118+ units/s by
tick 199, first divergence at tick 0, growing trend on position/velocity/
speed, nothing improved. **The likely reason, which is exactly the caveat
flagged when this task was handed off**: the reference scenario is straight
thrust with no steering (`--hold cross`, steer always zero), so the ship in
this specific replay probably never actually contacts a wall at all - the
stun timer is a real, correct, well-tested feature, but it never arms on
this capture, so it can't be what's suppressing thrust in the original's own
run. **The engine/resistance mystery is still open** for this scenario
specifically. The RE finding (the gate exists, confidence 88/85) stands on
its own merits regardless; what doesn't hold is the inference that it
explains *this* capture's deceleration. Next step for whoever resumes:
confirm directly whether the original ship in this exact capture ever
registers a wall contact (the recorded trace has no contact/grounded-on-wall
column today - may need a fresh capture with more state, or reasoning from
position vs. the track's collision geometry) before looking anywhere else;
if it genuinely never touches a wall, the missing resistance has to be
something this session's full `Ship_UpdateCraft` callee enumeration
(see "The engine mystery is resolved" above) already covers, which reopens
the question of why that enumeration didn't turn up a resistance term big
enough - or whether the "58 units of thrust" recomputation itself has an
error nobody's caught yet.

### `verification-script` - built, then actually run end to end against real PPSSPP, twice

Committed as `ca6323c` plus five follow-ups. `crates/trace/src/script.rs` (the
plain-text, run-length-encoded format - `60 cross` / `30 cross left` style
lines), `scripts/input_script.py` (a second, independent Python
implementation for the emulator side, with a documented cross-check for
drift: `diff <(oag-trace script F --expand) <(python3 input_script.py F
--expand)` should be empty), and an `Inputs::Scripted` path through
`replay.rs`/`main.rs` all compile and pass.

**This is not the important part. What matters is that this agent got real
PPSSPP access and actually ran the whole pipeline, twice, finding two real
bugs no amount of reading would have caught:**

1. **The debugger API table was wrong, taken on trust from the PSP's own
   naming rather than tested.** The first live capture died immediately:
   `input.buttons.send: Unsupported 'buttons' object key 'l'`. Probing every
   candidate against real PPSSPP v1.20.4 found `ltrigger`/`rtrigger` are what
   it actually accepts - `l`, `r`, `L`, `R`, `trigger.left`, `lt`, `rt`,
   `shoulder.left` are all refused. Fixed in `input_script.py`; the API
   reference table in `ppsspp-debugger.md` was wrong and needed the same
   correction. This is exactly the class of bug a documentation page
   copied from an assumption, rather than tested against the tool, produces.
2. **A real, measured input latency**: a button set at a breakpoint doesn't
   take effect until **three frames later**, which is two ticks off
   `oag-trace`'s own tick-numbering convention. Measured on all four
   transitions of the `steer-both-ways` scenario and confirmed by a second
   capture at the corrected offset. `--script-lead 2` now exists to account
   for it.

**With both fixed, the first-ever comparison against *varying* steering
input** (every earlier comparison this whole session used a centred stick,
so `steer` always read "exact" while testing nothing about steering at all)
came back **bounded: max error 2.6e-1, 1.1e-2 relative, over 200 ticks
including four full-deflection transitions.** The lateral/steering force law
holds up well against the real game under an actual varying input, which is
new, real, positive evidence this session didn't have before.

**One more finding, and it independently corroborates the collision-stun
gate from earlier this session without anyone planning for it to.** The
scenario's turns were originally 30 ticks; that put the real ship into a
wall, visible directly in the capture as `speed_cached` falling from 22.2 to
4.06 - a real ship, in a real recording, taking real collision damage to its
speed. Shortened to 12 ticks to keep the scenario clean, but the drop itself
is independent field evidence that the stun mechanism `engine-scale-re`
found by reading disassembly is a real, observable behaviour, not just a
plausible reading of code.

**Also explored and recorded, not resolved**: no PPSSPP save-state API exists
for this purpose (`savestate.save/.load`, `game.savestate`, `state.save` are
all unknown events; even an OS-level `xdotool F1` at the SDL window writes
nothing) - the menu walk remains the only fixed starting point. But
`replay.begin/.flush/.execute/.status` all answer, which is a real route to
recording a human-driven session and converting it into a committed script -
noted as a future capability, not built.

### `psp-xml-reader` - the determinism gate does not cover the sim crates, confirmed and left open on purpose

Committed as `be53698`. A real, honest, well-scoped audit result, not a fix:
**the gate genuinely does not reach `oag-physics`/`oag-gameplay`.**
`probe::run` (the thing `crates/core/tests/determinism.rs` actually hashes)
is a hand-built miniature simulation living inside `oag-core`, which is the
*bottom* of the dependency graph - it structurally cannot reach the
simulation crates above it, and `StateHasher` has no caller outside
`oag-core` at all. A second, sharper finding: **the three-OS CI job only
runs `-p oag-core`** - the cross-platform matrix this project's determinism
model leans on never executes a single physics or gameplay test on Windows
or macOS, only on the Linux-only workspace job.

What *is* covered, genuinely: the float pipeline, by a superset of what the
sim actually uses (the probe calls `sin`; the sim crates call no
transcendental function at all). What is **not**: order-dependent
reductions over collections, the fourteen-term accumulation order in
`forces::evaluate`, and every quaternion operation (`Quat::inverse`,
quaternion-vector rotation) - none of which the probe has any equivalent
for. `docs/architecture/determinism.md`'s enforcement table now carries a
scope column saying exactly this, replacing a line that had quietly been
carrying an expired premise ("once there is state" - there has been state
for most of this session).

**No gate was added, deliberately.** Closing this for real needs a
hash-and-compare over an actual `World` with its own committed reference
constant, and the agent correctly declined to generate that constant while
the force law is being actively rewritten in the same session - a reference
hash committed today would be stale within the hour, and under this
project's own "never update the constants to make a red test pass" rule,
that's worse than the gap staying open and documented. Real remaining work,
not done here: build that hash-and-compare once the engine/thrust
investigation settles down.

## Where I would go next

**Everything numbered 1-7 in the previous edition of this list is done**, this
same pass: `oag-trace` exists and works; the ship's suspension is fixed and
independently corroborated twice; the integrator-stability and handedness
questions are both settled at instruction level in a second binary; PS2
vertex type `0x1b9` is decoded (it was a VIF packet, not a GU variant); the
`.fnt`/`.mip` atlas is fully solved by reading `Texture_SwizzleForGe` instead
of more blind transform-guessing. Item 8 (the determinism gate) was not
checked this pass and is carried forward below. What's actually left:

1. **The engine resistance gap, ~12x short, confirmed genuine (the capture is
   really decelerating, not a transient - checked directly against the trace
   file) but its one strong-prior candidate is now dead.** See "The
   thrust-gap diagnosis just inverted" and "The track-section force lead is
   dead" above: it's not thrust, it's missing resistance, and the
   track-section force turned out to be a speed-*boost* pad (wrong sign
   entirely), not a resistance term. What's left: re-verify the
   already-implemented world-force writers (brakes, airbrake lateral/slide,
   gravity, hover epilogue downforce, vertical damping) against real
   numbers rather than trusting them for being already coded, or find a
   `Ship_UpdateCraft` call that was never enumerated as a world-force writer
   at all. This is still the one thing standing between the current build
   and a ship that can complete a lap - the suspension and wall collision
   are both already fixed and waiting on this.
2. **Make `oag-game` platform-generic.** Right now `oag-game`/`just play`
   hardcodes the PSP's archive layout (`PSP_GAME/USRDIR/Data.wad`) and
   flatly errors if pointed at the PS2 disc - confirmed directly this pass.
   `oag-view` (the separate asset viewer) already loads both platforms'
   assets correctly, including the PS2 mesh/texture/audio work landed this
   session, so the decoders are not the gap - only the game's own archive
   path resolution is. This is the prerequisite for ever running the actual
   *game* (not just the viewer) against the PS2 disc, or for any future
   "best of both" asset mixing the user has asked about.
3. **Check whether the determinism gate actually covers the new simulation.**
   Carried over, unchanged, from the previous edition of this list:
   `crates/core/tests/determinism.rs` compares a hash against a committed
   reference and predates `oag-physics`/`oag-gameplay`; nobody has checked
   whether either crate's state actually feeds it. Standing rule if it
   doesn't hold: **when that test fails, find the bug - never update the
   reference constants to make it pass.**
4. **LZSS is validated in a second binary now (self-consistency plus an
   independently-transcribed reference decoder), but still never against a
   runtime trace.** Lower priority than it once was, since the second-binary
   leg was this session's main gap for it.
5. **Two smaller open RE threads, both already scoped by name**: the
   torque-sign compensation site is fully resolved (it was never
   compensated, see "The torque-sign mystery is closed"), but the PS2
   `Ship_Update*` terms not yet corroborated (brakes, airbrakes, rolling
   resistance, vertical damping, and the fifteen-term ordering itself) are
   still single-source at confidence 84 - see "Eight of engine.md's
   force-law terms" above for exactly what's covered and what isn't.
6. **The sweep-and-prune ±1024 contradiction has a working instrument now
   and one data point** (`16_Track` itself is fine, at 856.2 units) **but no
   survey.** `oag-view --collision` prints the per-track reach; nobody has
   run it across every track on the disc to find which one(s) actually
   exceed the bound.
