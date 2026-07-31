# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); this file
covers what a fresh reader would otherwise have to rediscover.

**Pruned 2026-07-29.** Until then this was an append-only session log that had
reached 4,393 lines, most of it superseded by the docs pages each pass wrote as
it went. Nothing is lost: `git show 0c3e78d:HANDOVER.md` is the last full
edition and `git log -p HANDOVER.md` is every intermediate one. What survives
here is what is still true and still needed. **Keep it that way** - a finding
belongs on a docs page with its evidence and its confidence score, and this file
points at it. Prune again rather than appending indefinitely.

## Read this first

- **The disc images are in place.** `data/images/` holds `pulse-psp-usa.chd`,
  `pulse-ps2-eu.chd` and `pure-psp-usa.chd`, all three hashes matching
  [`source-images.md`](docs/reverse-engineering/source-images.md), so
  `just test-data` works. Earlier sessions have lost them and spent the session
  unable to verify anything - check they are still there before concluding a
  ground-truth test "cannot run".
- **`data/traces/` is a different story, and it has already bitten once.** The
  2026-07-29 pass found `talons-junction-time-trial-lap.csv`, `-lap-omega.csv`
  and `talons-junction-standing-start.csv` all gone, leaving only `steer-left`,
  `steer-right` and `venom-assegai`. `force-balance-ground-truth.md`,
  `contact-response.md` and three citations inside `wall.rs` point at files that
  were not there. **Derived evidence under `data/` is not durable; the scenarios
  and scripts that regenerate it are.**
- **`rg` and `fd` cannot tell you either of the above.** Both respect
  `.gitignore` and `data/` is gitignored, so both return nothing with exit code
  0 whether the directory is full or empty - no error, no warning, and it reads
  exactly like "there is nothing here". Use `--no-ignore`, or `/bin/ls` /
  `find` / `grep`. Note plain `ls` may be shell-aliased (`ls -la` failing with
  `ls:1: command not found: -la` reads like a broken argument and is not), and a
  spawned agent has a second, independent reason to see `data/` as empty: being
  gitignored, it does not travel into a worktree or a restricted sandbox.
- **Check `git status` before assuming the tree is clean.** A whole milestone's
  work once sat uncommitted for a day.
- **Gate status:** green at 1,049 tests, `audit-leakage` clean. `just test-data`
  is 1039 of 1040 (with `--no-fail-fast`; without it, the failure below stops
  the run at 362), and **the one failure is the machine rather than the code**:
  `oag-disc::ground_truth listing_matches_chdman_and_the_reference_iso` shells
  out to `chdman extractdvd` into `/tmp/oag-ground-truth.iso`, `/tmp` here is a
  6.8 G tmpfs, and the extracted PSP ISO does not fit in what is free. It fails
  at "Extracting, 29.4% complete... Error writing to file; check disk space",
  which `nextest` reports as `chdman extractdvd failed` two frames up and reads
  like a broken invocation. Free `/tmp` or point the test elsewhere; do not go
  looking for a disc bug.

## Where the project stands

M4. The force law, the angular-momentum model, the contact response and the
mag-lock attitude hold are all recovered at instruction level and implemented,
and **every fitted constant that once stood in for a recovered one has been
retired** - `YAW_DRIVE_CALIBRATION`, `ALIGNMENT_INERTIA` and the
`<Misc>`-derived box inertia each had a real value found for it. The evidence
lives in [`docs/physics/`](docs/physics/README.md) (the two ground-truth pages
and [`angular-velocity-column.md`](docs/physics/angular-velocity-column.md)) and
under [`docs/ghidra/functions/`](docs/ghidra/functions/README.md) - `engine.md`,
`rigid-body.md`, `contact-response.md`, `craft-update.md`.

**The wedge is gone, and it was never physics.** For one session the blocker was
stated as "618 units of path against the original's 1,045, wedged at one place on
the circuit" - a craft that stopped dead with full thrust. It was the capture
harness. `psp-trace.py --script-lead 2` sends the script's tick `k + 2` at the
breakpoint for tick `k`, which cancels the emulator's three-frame input latency
correctly for every transition *after* the start and, in doing so, **never sends
the script's first two states at all**: at tick 0 the pointer already stands at
index 2. Our replay applied them. Two ticks of head start, and they do not wash
out, because `ramp_steering` travels toward its target without clamping to it and
a saturated axis oscillates around it forever - unlike the airbrake ramp, which
clamps and resynchronises on every transition. The whole chain is measured in
[`oag-trace.md`](docs/tools/oag-trace.md#the-first-two-ticks-of-a-script-never-reach-the-emulator):
a standing 17 % steering offset, 8 degrees of heading by tick 70, 15 units wide of
the original on a half-width of 17.9, and a wall at tick 147 the original never
touches.

`oag-trace run --script-lead N` and `drive --script-lead N` now exist to release
the same ticks. **Pass `--script-lead 2` against everything in `data/traces/`**,
and pass it on both sides of any new capture.

The two facts that made this findable, if a comparable thing ever comes up again:
the offset shows on `throttle` and `airbrake_r` as well as `steer`, on **two
independent captures**, and the *control columns are a pure function of the script
and `dt`* - they do not depend on the trajectory at all, so a mismatch there is
never a physics symptom.

**One older hypothesis stays dead.** The run report's angular velocity averages
21.2 rad/s over the lap, which reads like a craft spinning three and a half times
a second. The original reads **22.96 rad/s on the same scenario**, slightly
*higher* than ours. Whatever that column is, both sides do the same thing with it.

**Read lap fidelity off the *seeded* comparison** (`oag-trace run --reseed N`),
never the open-loop one, and note the roadmap's exit criterion is now stated as
two decidable numbers for exactly this reason. The original integrates the frame
duration it measured
([ADR-0007](docs/architecture/adr/0007-fixed-timestep-vs-original.md)), those
durations follow host load, and **two runs of the original itself, from a start
pose pinned to 0.0000 degrees, are 100 units apart by tick 495** with `dt`
agreeing on about 1 % of ticks. A 3,146-tick single-seeded trajectory comparison
measures the emulator's scheduler, not the force law, and a byte-exact
implementation would fail it.

**Score the lap on the window where the capture is clean - and a fresh, far
cleaner capture now exists (2026-07-30).** The 2026-07-29 recapture was
wall-free on only 32.7 % of its ticks; `data/traces/talons-junction-clean-lap.csv`
(a fresh `just autopilot` lap, not a replay of any committed file) is wall-free
on **96.3 %**, both now `oag-trace show`'s own numbers rather than an ad hoc
count, see [`oag-trace.md`](docs/tools/oag-trace.md#show-and-the-cleanliness-report-in-it).
It still first touches a wall at tick **256**, not much past the old 171 - a
second attempt chasing the lifted centre line (`--column lift`) landed at 249,
no better, so per this pass's plan two attempts with no material gain is the
stopping point, not a reason to keep tuning the autopilot's steering.
`--script-lead 2` is confirmed correct for this capture too (`throttle`/`brake`
bit-exact across all 2,977 ticks only at lead 2, same check as before, run
again because this capture came from `psp-autopilot.py` rather than
`psp-trace.py --script`).

**The real finding is that the wall was never the whole story.** Even inside
this capture's own clean window, single-seeded position error is already 17.5
units by tick 180 and 44.1 by tick 240 - **47.76 max / 13.28 mean over ticks
0-255**, while the same ticks 0-170 window used before still reads 10.12
max / 2.50 mean, consistent with the old 6.54/2.41 (a different lap through the
same opening straight, not a regression). So closing the wall-contact ceiling
from 171 to 256 did not buy 85 more ticks of "the physics agrees" - it exposed
that the force law itself, not contact geometry, was already the thing limiting
comparison length well inside the old window's tolerance. That is the harder
problem left blocking M4's single-seeded number, not another capture. Under
`--reseed 60` this capture reads **28.86** at tick 1,739 (bounded) and worst
orientation axis **0.4858** rad at tick 1,129 (bounded) - both larger than the
old capture's 10.3/0.0824. **Checked before attributing that to the lap
rather than the collision-box scale fix that landed the same session**:
re-run under today's physics, the old capture's worst reseeded orientation
axis is 0.0541 rad, a modest shift from 0.0824 and nowhere near the new
capture's 6x jump - so the gap is this lap holding racing speed and cornering
hard throughout rather than spending most of its length wedged at low speed
against a wall, not the physics regressing. The whole-run single-seeded
figure is 1,245 units at tick 1,628 and growing - same character as the old
capture's 253.7 at tick 2,494, not comparable tick-for-tick across two
different trajectories, and meaningless past a few hundred ticks either way.
(The tool's own "first divergence at tick 0, field angular_velocity" headline
for this capture is the `1e-4` tolerance floor on an idle craft, not a
tracking failure - read the position row, not the top line.)

**Do not read `grounded` as the headline it looks like.** The original's column
is `1.0` on all 3,146 ticks of this capture, so agreement means only that our
ship also never leaves the ground - a real improvement over the 2026-07-28 run,
which read `0.5` against `1.0` and then left the surface for good at tick 313,
but no evidence that the hover model quantises contact the way the original
does. A constant column cannot test that, and under `--reseed` the field is one
of the ones the seed restores anyway.

**The standing start was retaken too, and it is the pass's best result.**
`verification/scenarios/standing-start.inputs` is new - 300 ticks of held thrust
from the start line - and it reconstructs a capture that was lost before
scenarios were committed. It reproduces *both* legs of the force-balance
conclusion on data that did not exist when they were derived, from a run that is
demonstrably not the same one (clean for 186 ticks rather than 66; hits the wall
at 119 units/s rather than 54):

- **launch acceleration 32.69** on its third tick, against the recorded 32.52
  and the predicted 31.8. *"Third tick" now has an explanation rather than being
  a quirk of the capture: it is the first tick the emulator was sent thrust, the
  first two having been eaten by the lead.* Both legs are properties of the
  capture, so neither moves with the replay fix;
- **the `0.035` contact-friction floor never crossed** - minimum loss 3.534 %
  over 114 contact ticks, decaying 3.835 % -> 3.633 %, with **0 of 114** ticks
  above 5 units/s falling below it.

One condition on ever re-running that test, learned by getting it wrong on the
lap capture first: it only measures friction while the craft is *moving*. Below
a few units per second the normal impulse dominates and restitution can push
`|velocity|` above `speed`, giving a negative loss - the lap capture has ticks
reading `-380 %` on a craft wedged at 0.15 units/s.

**A clean lap scenario was re-derived (2026-07-30), and it is what the section
above scores.** The old committed lap script had stopped flying clean when
replayed open-loop (67.3 % wall contact, down from 95.2 % on an earlier
capture of the same script) because it was recorded *closed-loop* and an
open-loop replay drifts into walls - the same negative seen from the
authoring side. `just drive menu` (cold boot) + `just drive restart` +
`just autopilot --spline /tmp/spline.csv --laps 1` produced
`talons-junction-clean-lap.{inputs,csv}` fresh, script and trace from the same
run, which is now the committed scenario and the capture both. First-attempt
stalls are still worth one retry before diagnosing: this capture's own first
attempt at the earlier committed script timed out at tick 561
(`never stopped at 0x08849618`) and completed on a straight retry after
restarting PPSSPP.

**A save-state fixed start was investigated and not adopted - `--start-heading`
above already beats it.** The unchecked M3 roadmap item asked for a save-state
driven fixed start; `--state=FILE` turns out to exist (the websocket debugger's
own `savestate.*` commands do not, confirmed by string-dumping the binary) and
`wtype` (Wayland's virtual-keyboard protocol) can drive PPSSPP's own Escape
overlay to create one - both undocumented before this pass, both now on
[ppsspp-debugger.md](docs/reverse-engineering/ppsspp-debugger.md#save-states-and-the-input-recording-api-that-may-replace-them).
But measured against two independent loads of the same state (craft cut at a
settled start-line pose), the pose disagreed by **0.031 units / 1.41 degrees** -
no tighter than the plain `restart` menu walk's 0.03/2.51, and two orders of
magnitude worse than `--start-heading`'s already-committed 0.0022/0.0001. The
roadmap checkbox is now closed **via heading-pinning, not a save state** -
[see above](#where-the-project-stands) - and the state-loading mechanism is
kept documented rather than wired in, in case a future need a heading pin
cannot reach (mid-race state, a different track's start) makes it worth
returning to. `.ppst`/`.p2s`/`.state` are now in `just audit-leakage`'s
extension list, which did not cover them before.

## Open threads

Each is a real, named next step. The Task numbers are the ones the agent passes
used, kept because commits and docs cite them.

| Thread | What is known, and the next step |
| --- | --- |
| **The hull-spark visual trigger is still unread, and three plausible reads of it were ruled out this pass** | **2026-07-31.** Planning collision sparks (`oag_render::sparks`) sent this pass into `FUN_088418e0`, the per-tick consumer of the wall-contact observation ring `contact-response.md` already cited but had not fully read. Three separate reactions came out of it - camera shake, hull damage (decompiled in full, confidence 80), and a shield hit flash (decompiled in full, confidence 75) - and **none of them spawns a particle**. See [contact-response.md](docs/ghidra/functions/psp-pulse/contact-response.md#fun_088418e0s-contact-loop-drives-three-separate-reactions-and-none-of-them-is-a-particle) for all three, with addresses and confidence. What is left: the same loop arms flag bits `0x20`/`0x400020` on `craft + 0x860` whenever the damage reaction fires, and nothing found so far reads that bit - it is the one remaining candidate for a real spark trigger. The `Ship Collision Fx` `0x3d0` class's own registration wrapper was also checked and is a dead end: it installs a three-instruction self-address-returning thunk (`FUN_08a6ba70`) into the constructor slot, not a real constructor. **Static reading has hit its ceiling here** - the next step is a live PPSSPP capture, breakpoint on a read of the `0x860` flag during a scripted wall impact, the same method that settled the exhaust trail's draw-gate question. Until then, `oag_render::sparks` is implemented as an authored effect anchored to our own physics contact data rather than to this trigger - see that module's doc comment. **A smaller, cheap follow-up**: `FUN_088439ac` (the damage reaction) and `FUN_0885eb04` (the shield flash) are both read to confidence ≥70, which the project's naming rule would normally promote to a rename plus a dedicated function doc page under `docs/ghidra/functions/psp-pulse/` (ADR-0005) - not done this pass, to keep the collision-sparks task bounded, and left as plain `FUN_*` citations in `contact-response.md` instead. |
| **PVS culling is on, and the ceiling on it is our own batching** | **2026-07-30.** The track's authored `section` `0x3c9` payload is decoded (`oag_formats::pvs`, validated over both discs) and culls before the frustum test (`oag_render::pvs`, `[graphics] pvs_culling`, on by default). See [ADR-0011](docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md). **No baker was written and none should be** - the artists' visibility set ships on the disc, so the "sample a camera per segment and raycast" design is reimplementing shipped data, and the up-vector-on-inverted-track hazard that design has to solve does not exist when there is no sampling camera. Three next steps, in value order: **(1)** the saving is capped at 22-66% (median ~47%) because a `DrawCall` is one material run whose bounding sphere reaches 6-14 of the 64 sections - **splitting batches by section at load** is the only lever that moves this, and it trades draw-call count for it, so measure before building; **(2)** which section a *mesh* belongs to is still unrecovered - `oag_render::pvs` invents it by sphere-versus-box overlap and says so, and recovering the original's association would retire the only invented part of the pipeline; **(3)** moving entities (rival craft, weapons) have no section and so are never culled by tier one. |
| **The HUD draws, and the four things left on it are named** | **The layout was never a reverse-engineering problem** - Pulse ships five HUD layouts in `Data.wad` as `Data\XML\*_HUD.xml`, exact pixel rectangles, atlas UVs, colour constants and font roles, and they decode with the `fexml` machinery this project already had. A pass concluded the opposite from *negative* evidence (no HUD element in `fexml.md`'s known-element table) and planned to measure rectangles off screenshots; the cheap check that would have caught it - grep the archive listing for `HUD` - was never run. See [hud.md](docs/ui/hud.md). What is left: **(1)** the default outline colour is the layout's `HudBGColour` at 25 % alpha and the original's reads crisper, so measure it off a frame; **(2)** `TotalTime` overflows the right edge by 12 px and it is not clear whether the original clips too; **(3)** whether `SpeedBarMark` slides with speed - needs one frame at speed, the start-line frame cannot say; **(4)** a mode's code substitutes string keys into widgets the layout positioned - a time trial's top right reads `record` (`IG_HUD_RECORD`) where `TimeTrial_HUD.xml` says `IG_HUD_TOTAL`, and 26 of the binary's 38 `IG_HUD_*` keys appear in no layout at all - so the substitution rule is unread and is deliberately **not** worked around by hardcoding. Zone, Eliminator, the `<Mode3D>` countdown and sights, and text outlines as a second pass are all scoped out and listed on the page. |
| **The two HUD fonts are pre-outlined, and one doc claim was wrong about it** | `font.rs` asserted the `.fnt` palette was "an alpha ramp over a single RGB … so the colour carries no information the vertex colour does not already supply". True of the three menu fonts, **false of `PulseHud.fnt` and `small.fnt`**: they carry six distinct greys where the menu fonts carry one white, alpha is the silhouette of glyph *plus* a baked outline, and the grey level separates body from outline. Reading alpha alone filled the outline with glyph and turned a 25-pixel lap time into a solid white box. The glyph atlas is `Rg8Unorm` now and text composites `mix(BorderColor, Color, mask)`, which is exactly the two colours the XML supplies. Menu fonts are unaffected *by construction* (constant mask) and that was checked on screen, not assumed. Pinned by `fnt_ground_truth` asserting exactly two of five fonts are outlined. **A screenshot caught this and no test would have** - the arithmetic was all correct. |
| **Capturing a PPSSPP reference frame has three traps worth not rediscovering** | **`RemoteISOPort` does not pin the debugger port** - PPSSPP bound an ephemeral 46659 while `psp-drive.py` defaults to 47810, so `preflight` reported no debugger at all; read the real port off `ss -ltnp \| grep PPSSPP` and pass `--port`. **An unfocused window grabs as pure black** with no error, the documented SDL throttle - focus it and check the grab's mean luma before reading anything off it. **`niri msg windows` prints no absolute coordinates** but `niri msg --json windows` gives `tile_pos_in_workspace_view`, which is what `grim -g` wants; getting it wrong crops a corner and looks like a HUD missing half its widgets. Also note **two emulator instances can be running** (another agent's), so match on pid and window id rather than on app id. |
| **The HUD art is raster, and 26 mesh files named after HUD elements are referenced by nothing** | Everything drawn today is pixels authored for 480x272 - `PulseHUD.mip` is 256x256 8 bpp paletted, both HUD fonts are 4 bpp paletted bitmap atlases - so a modern window magnifies it (the project's 1440x816 captures are already ~3x, 4K is ~8x). Meanwhile 26 `Data\HUD\*.vex` files are **polygonal geometry** named exactly after the elements (`Bar_1`, `Speed`, `Shield`, `Lap`, `Position`, `Thrust`, eleven `Weapon_*`, `Zone_Bar_*`), with Maya node names `polySurfaceShape1`, `Cube1_nolight`, `Lap1_nolight` and `AnimEnd` markers. No layout's `Src=` and no string in `BOOT.BIN` mentions them, and they are `.vex` **version 4** with root class `0x0ee`, which [vex.md](docs/formats/vex.md) says does not occur in Pulse. **What that means is not established** - baked source, abandoned approach, or another platform's path - and the decoder for that class does not exist yet. Do not write it up as "the HUD was vector". **Redrawing the art as vector is a live future option** and the useful part is the ADR-0006 asymmetry: a transcription of the disc's layout cannot be committed, but art we draw ourselves can, so this is the part of a "commit an artifact" idea that is actually available. Keep the *layout* disc-derived. See [hud.md](docs/ui/hud.md#the-shipped-art-is-raster-and-the-disc-suggests-it-was-not-always). |
| **The six-tick-early wall contact is resolved into a real bug fix, and a smaller, reversed residual** | **2026-07-30.** The named next step - reading `Collider_BoxSamplePoints` and `Body_SetBoxDimensions`/`Collider_SetBoxDimensions` for both axes - found that `Ship_InitCraft` (`0x08841360`-`0x0884139c`) scales `<Misc width height length>` by `0.75` (the same global `hover::TARGET_GLOBAL_SCALE` already reads for the hover target height) **before** building the collider, and `crates/physics/src/wall.rs::hull_sample_points`/`hull_extent` never applied that scale - the hull box was a third larger than the original's in every dimension. The same read also settles the width/length ordering `Body_SetBoxInertia`'s square `(12, 8, 12)` box couldn't: confidence on the axis-to-dimension mapping goes **88 -> 95**. See [collision.md](docs/ghidra/functions/psp-pulse/collision.md#the-dimensions-feeding-the-collider-are-scaled-not-the-authored-misc-values). **The fact was already on record** - [rigid-body.md](docs/ghidra/functions/psp-pulse/rigid-body.md) noted the same `0.75` scale in passing on 2026-07-28, while establishing the inertia tensor is a code literal - and never reached the collision code; see the working rule below, now a fourth instance. Fixing it moved the physics determinism reference hashes (regenerated deliberately, `crates/physics/tests/determinism.rs`) and six unit tests in `wall.rs` whose fixtures hardcoded the old, unscaled half-extents. **The gap did not close to zero - it crossed it.** `wall_contact_ground_truth.rs`'s two ignored tests, re-run after the fix: our hull now finds the standing-start wall at tick **190** against the original's **187** (3 ticks *late*, was 6 early) and the lap's at **176** against **171** (5 late, was 3 early) - both a smaller magnitude than before, on the other side, and by *different* amounts on the two captures, which argues for an approach-angle-dependent residual rather than a second uniform scale error. **Do not fit a second scale to close this** - `0.75` is recovered, not fitted. Instrumented (`firing_probe_index` in the test file): the probe that fires is still a lower corner (index 2 on the standing start, index 1 on the lap; `hull_sample_points`' `0..3` range), not the flank pair the scale shrank furthest, so this is the same *kind* of probe, just no longer at the same corner-to-wall distance. The ground-truth tests now assert the sign explicitly (not just a magnitude bound), so a regression back to the unscaled box fails loudly rather than passing inside a symmetric tolerance - checked directly by reverting the scale and re-running. |
| **The grid: one slot is recovered, seven are not** | **Resolved half.** `Start Position` (class `0x3bc`) is decoded and `oag_game::race` and `oag-trace drive` both spawn on it rather than on spline sample 0, which was never a recovered value. The reading is confirmed against the running game by *heading*: the slot's forward is within **1.12 degrees** of the original's craft at the start of a captured time trial. What it is **not** is pole - the capture starts 137.9 units ahead of the slot and 22.4 to its left, both about 10 units off the centreline on opposite sides. Those three numbers are what an account of the grid has to explain and no account is offered. Exactly **one** node per track on all 40 files, so the other seven slots are laid out by unread code. **One coincidence is recorded with its refutation attached**: `track_reversed.vex`'s slot is 2.2 units from the capture's start, `y` agreeing to 0.0096, and 4.011 above its own surface against a 4.002-4.009 resting craft height - but the recorded craft faces along `track.vex`'s tangent at `+0.9999` and the reversed variant's at `-0.9999`, so `DEFAULT_TRACK` is right. That check was worth writing down because **M3 identified the track by casting positions against collision geometry, which the forward and reversed variants share** - the `01_Track` trap one level down, and nothing had ruled it out. **Next step is that code**, not more data: nothing in `.vex` will produce slot 2. Two traps recorded on [`track.md`](docs/formats/track.md#start-position): the authored `y` is **not** a ride height (1.03-7.36 units above the collision surface across the 40 files, so the spawn raycasts instead), and `just scripted-sim`'s path length is still not comparable to a capture's - it now starts from the grid rather than 51 units past the line, but that is a third journey, not the capture's. Use `run --script --script-lead 2`. |
| **The original's "1,045 units of path" implies a fifth of a lap** | Noticed while checking something else, unexplained, and load-bearing enough to write down: the reference lap capture runs 3,146 ticks and the figure quoted for the original's path length is 1,045 units, which is **0.33 units per tick** on a circuit whose resampled spline is roughly 5,000 units long. Either the capture is not a lap, the path-length measure is not what it is read as, or the original spent most of the capture barely moving - and the third is consistent with the 67 % wall-contact figure. Nothing here depends on it *yet*, but HANDOVER and the roadmap both state it as a whole-lap number. Check it before any exit-criterion argument leans on it. |
| **The exhaust's flare is close; the trail is ~6x too prominent and may not belong at all** | Flare and trail recovered and drawn - [`exhaust.md`](docs/ghidra/functions/psp-pulse/exhaust.md), `oag_render::exhaust`. **A valid matched comparison now exists** (2026-07-30, both intensity-saturated, original 359 km/h against ours 451): the original is a *compact violet-white bloom at the nozzle, about a third of hull width and a quarter of hull length, with no long ribbon*. Ours adds a trail running off the bottom of the frame. Only 1.25x of the ~6x gap is the speed difference. **The obvious cause is ruled out.** It is not the sample count: the original runs at ~30 fps, so its 10-sample ring spans `10/30 * 100 = 33` units against our `10/60 * 125 = 21`. By that arithmetic the original's ribbon should be the *longer* one. It plainly is not, so the ribbon there is either far dimmer than modelled or not drawn for the player's craft. **Two candidates, and candidate 2 is the cheaper decisive read.** (1) The colour reading is wrong: `Exhaust_Update` is read as writing the ramp into all four channels, giving `L^2 ~ 0.49` per layer additive over three, which saturates - if the tint at `flare+0x68`..`+0x74` is not `(1,1,1,1)` in practice, or those writes land elsewhere, the real ribbon could be near-invisible. (2) **The draw path may be gated off.** `Trail_DrawRibbon` gates on `DAT_08ab10b0` and the `& 0x20` flag at `+0x2c`; the *update* path was confirmed to run for a racing craft, **the draw path never was**. **If the player's trail never draws, the ribbon is an addition rather than a reproduction and should be removed** - revert the ribbon half of `1c1b163` and `08cb938` - which would also explain why no amount of tuning it has converged. Do that read before touching another constant. **Retracted:** an earlier entry claimed our craft accelerates ~5x faster than the original's. A clean run went 0 -> 101 units/s in three polls, so the slow run that produced that claim was impeded (open-loop scripts drift into walls). There is no evidence of a physics gap here. **Capture recipe that works**, worth not rediscovering: spawn a private instance with `niri msg action spawn -- env SDL_VIDEODRIVER=wayland PPSSPPSDL --appendconfig=<ini> --windowed <iso>` (note `niri msg spawn` is not a subcommand; it is under `action`), pick a debugger port other than another agent's - 47810 is `psp-drive.py`'s default, a second instance falls back to something like 46659. Scripts need `uv run --with websocket-client`. Go **`menu` from a cold boot, not `restart`** - after `restart` the craft did not accept `drive`'s inputs. Fullscreen the window by id (`niri msg action fullscreen-window --id N`) so plain `grim` captures only the game and the two instances cannot be confused. Read speed from `psp-drive.py state` as **`speed:`** with a colon. Pick the frame by speed **and** mean luma, and require a lap clock of at least 4 s so intensity has saturated. **Three constants remain ours:** `HALF_SIZE_TO_WORLD` (55), the per-segment billboard basis, and nothing else - `TRAIL_LENGTH` became recovered-as-zero at 85. |
| **Trackside texture animation is implemented, measured against the original, and switched off** | **2026-07-30.** Eight track textures animate by V-axis scroll, the ship blink lights' mechanism generalised (`oag_render::mesh::ANIMATED_TEXTURES`, `[graphics] animated_textures`). **The enabling fix was a parser gap, not a rendering one**: `vex::textures` read a texture's name only from the node *header*, and a track's `Texture` nodes use the short 32-byte header with the name field zeroed, so every track texture came back unnamed. The runtime path at payload **`+0x38`** - a field [vex.md](docs/formats/vex.md) had documented all along - is the only name a track texture has. Ships worked by accident: their header holds the `Z:/WipeoutPSP/...` authoring path, which happens to contain the matched substring. **A doc claim was right for the wrong reason and is corrected with a `## History` entry**: "no track resolves to `colours_flashing_GLOW.tga`" held on re-checking, but the check that produced it could not have found a match whatever the data said, and the same gap hid every other animated track texture. **Two readings are ruled out rather than untested**: material `+0x0c..0x14` is `0` on every material of all 12 circuits, so there is no authored scroll rate, and the material `flags` word does not separate animated from static (`hub_banner_GLOW` and `flicker1nonalpha_GLOW` both `0x91`). So the key is an enumerated list, and each entry earns its place by a **per-draw** V-span measurement - narrow band means a quad picks a phase by V; full-tile means a scroll would slide the artwork. **Measured per draw, never aggregated**: separate quads sit at different V deliberately, and a per-texture min/max reads a narrow-band texture as a wide one - which is what the first pass did, making the ship control look wide too. The separation is total: entries at or under 1.50 rows, exclusions at or above 31.94, pinned by `crates/render/tests/animated_uv_ground_truth.rs` from both sides. **What is not established is that the original animates any of them** - confidence **65**, and every rate reuses the blink light's 2 sweeps per 120 ticks because it is the only one ever measured. The next step is a PPSSPP capture at `07_Track`'s pulse lights and `col_display7_GLOW` (on all twelve circuits); if a surface is static there, delete the entry rather than turning the setting off. **A capture must isolate the tick**: a race with `--hold cross` drifts into walls, and on/off comparisons at ticks 300-1800 came back byte-identical simply because no tagged surface was in frame. Ticks 2 vs 62 at the start line on `07_Track` shows it - 6,957 pixels, 0.59 %, confined to the track-edge light band. **`flicker1/2nonalpha_GLOW` looks like the best candidate on the disc and is deliberately excluded**: banded exactly like the blink palette, and every circuit embeds it - but **zero materials reference it** on eleven of twelve, its draws tile ~4x vertically, and the `Data\Pads\` geometry that would use it is never loaded. `Speedup Pad`/`Weapon Pad` have no consumer at all. **The capture was then run, and it refuted the mechanism.** A breakpoint on `Gu_TexOffset` through a live Time Trial on Talon's Junction - a circuit carrying two of the eight - hit 60 times from three sites, and **the only non-zero offsets came from `Trail_DrawRibbon`** (9 calls, 9 distinct advancing values); the other 51 passed exactly `(0,0)`. `Gfx_BindTexture` hit 80 times over the same window, so the track was drawing. Five candidate palettes were byte-static over 3.5 s, so a CLUT scroll is not it either. **So `[graphics] animated_textures` defaults off**, and the table is kept only because the geometry that selected it is a real measurement. **This also supersedes part of a 85-confidence claim in vex.md**: the ship lights' *pulse* is measured and stands, but the *mechanism* read off it - the engine scrolling the shared texture's V - is contradicted, because no such offset is ever submitted for the ship either. The untested candidate that fits everything is an animated per-draw **colour**; `Trail_DrawRibbon` already sets one via `FUN_0881125c`. **The generalisable lesson**: a filmstrip-shaped texture plus a measured pulse does not identify the mechanism, and the cheap check was a breakpoint rather than a screen capture - no window focus, no compositor traps, five minutes. Reach for the debugger before `grim`. Also two GU primitives named on evidence, `Gu_TexOffset` `0x08811630` and `Gu_TexScale` `0x08810ab4` - see [texture-animation.md](docs/ghidra/functions/psp-pulse/texture-animation.md), which also records that **`Gfx_BindTexture`'s `0.2` threshold is a VRAM re-upload heuristic, not an animation clock**, and that the global scroll clock is still unfound. |
| **`engine_fire` and `exitglow` are authored but never registered** | Neither has one of `Vex_RegisterClass`'s 46 call sites - alphabetical order puts neither between `Engine Flare` and `fogCube` - yet `exitglow` is authored **13 times** on `16_Track`. Same shape as `gate`, which [vex.md](docs/formats/vex.md) already records as never registered. So "has instances" and "has a handler" are independent properties of a class, which is worth knowing before reading any census as a to-do list. |
| **The `.vex` class-ID table's extent is still unknown** | All ~55 game classes are transcribed into [vex.md](docs/formats/vex.md) and `vex::CLASS_NAMES`, self-validating at 95 against the ten IDs earlier passes had found independently. But the walk stopped at `0x08ab26a0` without reaching the `id == -1` terminator, so the generic Maya classes past `0x3eb` are only partly covered. Cheap to finish; nothing depends on it. |
| **The five teams whose `Ship.vex` does not resolve by name** | `Auricom`, `Harimau`, `Icaras`, `Mirage`, `Van_Uber` have no `Data.wad` entry hashing to `Data\Ships\<Team>\Ship.vex` on the PSP disc, so eight of thirteen teams were censused. A `mine-names` job, not a format one. Note the path templates resolve with the **FE team-model name**, whose default is the literal `"ship"` at `0x08a84cb4` - hence `shipwreck.vex` and `shipboost.vex`, *not* `Assegaiwreck.vex`. Guessing the team name into those templates 404s, which cost a wrong conclusion once. |
| **The 13 % roll-stiffness gap** | The recovered tensor runs the roll oscillator 13 % stiff against measurement. Untouched by the downforce fix by construction (the probes are on the centreline), so it is now the only gap of its kind. |
| **Menus: rebinding is the one thing on them that does not work** | The shell navigates and every other row is live - see [menus.md](docs/architecture/menus.md). A `binding` row shows what `oag_input::keys::map_key` returns and cannot change it, because that function is a hardcoded `match`: rebinding needs a table, persistence and conflict handling. `keys::bound_keys` probes `map_key` with a candidate set rather than keeping a second table, so the page cannot drift from the game, but the candidate list can under-report a key added to `map_key` and not to it. **Escape now backs out of a race into the menus**, which is most of the road to a pause menu and is deliberately *not* one: the `World` is dropped rather than suspended, so re-entering loads a fresh race. Suspending it is the remaining work, and the menu stage already builds its own renderer so it can be opened from somewhere that is not the front end. |
| **MONITOR has only ever run on a one-screen machine** | It landed with the DISPLAY/GRAPHICS split and it is the one row there with no coverage of its working path. The *miss* path is confirmed on `eDP-1`: a settings file naming `nope-not-here` prints `no monitor named ... (this machine has: eDP-1)` and falls back to the default. **Picking a second screen has never been executed**, in either window mode. `display::centred` and `Monitor::choose` are unit-tested; `centred_on` in `main.rs` is not, and it is the part that reads winit's rectangle and scale factor - a wrong scale factor offsets the window by the difference on any screen not at 100 %, and it centres on the *inner* extent while setting the *outer* position, so a decorated window sits high by about a title bar (known, documented on the function, not corrected). **Try it on two screens before leaning on it**: `just play`, OPTIONS -> DISPLAY -> MONITOR, windowed and borderless. Note the compositor may refuse all of it and a tiling one will. |
| **Movie decode is off the render thread; what is left is the audio it has no counterpart for** | **Resolved, and measured on both sides.** `movie::Feed` owns the `FrameStore` on a worker thread, decodes four frames ahead into a ring, and hands the drawing thread the newest frame at or before the playhead; both call sites went through it, the menu backdrop and the intro (`FrontendStage::sync_video`, which had always done the same blocking decode). [ADR-0010](docs/architecture/adr/0010-movie-decode-thread.md) has the design, the rejected alternatives and the full tables. The headline, same 45-second run through both stages, same instrumentation: PSP menu at a 240 limit went from **197.7 fps min / 51.85 ms worst frame** to **240.0 / 4.71**, the PSP front end from 213.5/43.35 to 240.0/4.44, and the PS2's 512x512 `BG512.IPF` - the worst case at **82.25 ms** - to **7.04 ms**. **The "front end" figures are the intro's code path playing the *backdrop's* stream**, not `Intro.PMF`: every run passed `--movie 'Data\Movies\Backdrop.PMF'` so that one nine-second movie reaches both stages. Same cache format, same decoder, so `sync_video` is exercised faithfully - but a 240-limited loop over `Intro.PMF`'s own 1200 frames is the one thing nobody measured either side, and its cache is warm if anyone wants to. Unlimited, the PSP menu's worst frame went 46.51 -> 7.45 ms while the *median* barely moved (942 -> 1016 fps), which is the whole point: the average was never the symptom. **Three things worth knowing before touching it.** `--menu-page` cannot check this path - the capture goes through its own `Movie` whose frames were never taken, so it looks right even when a real window draws on black; use a screenshot of an actual window. `Feed::restart` carries an epoch because a restart can land mid-decode, and getting that wrong breaks the *second* menu open only. And the ring is deliberately not a channel: the consumer polls several times per movie frame and must be able to look at positions without consuming them. **What is genuinely left is not this**: the movie has no audio at all (ATRAC3+ is demuxed and discarded), and whenever it gets some, the playhead becomes something two consumers pace against rather than one - `movie::Player` is the place that changes, not `Feed`. |
| **The menu backdrop is verified as a loop now, including the wrap and the reopen** | Was "verified as a still". The menus draw over `Data\Movies\Backdrop.PMF` (PS2: `BG512.IPF`), the movie `FE Screen`'s own `Movie` widget names - see [menus.md](docs/architecture/menus.md#the-background-the-menus-sit-on). **The playhead advancing is now confirmed on the running game rather than argued**: an instrumented run printed the frame actually in the renderer's planes beside `Player::position()` every 200 frames for 45 seconds and they tracked exactly, through the wrap (position 278 showing frame 8 of 270) and on both discs, with a live-window screenshot on each showing the picture and the PS2 pillarbox. `--no-video` is confirmed back to black. **Menu -> race -> escape -> menu was verified by forcing a second `open_menus()` rather than by pressing escape**, because no key-injection tool (`wtype`, `ydotool`) is installed - see the ESC row, which has the same gap. That is the `Feed::restart` path and it came back at frame 0 and wrapped again correctly. What nobody has done is *watch* it: `just play` and look at the background for twenty seconds. |
| **Brightness and gamma are covered, and not by a screenshot** | Recorded because the obvious check is the wrong one. `--screenshot` and the race capture write the offscreen frame straight out and never run the blit that grades it - that is [deliberate](docs/architecture/menus.md#brightness-and-gamma), so **no capture will ever show these settings** and a picture that looks ungraded is not a bug. The real check is `upscale::tests::the_grade_moves_the_picture_in_the_direction_the_setting_names`, which builds the actual pipeline on a headless `Rgba8Unorm` target and reads the pixel back; it was mutation-checked by swapping brightness and the exponent in the shader, and it fails. It skips with a note where there is no adapter, so **a green CI run does not mean it ran**. |
| **The ESC rewiring has no live-keypress check** | Escape is "back one level" - a race returns to the menus, a menu page pops, the front end and `--race` quit. `Menu::back` is unit-tested from both ends, and `Session::escape` in `main.rs` is not: it needs a window and a real keypress, and no key-injection tool (`wtype`, `ydotool`) was installed on the machine it was written on. **Press it once by hand before leaning on it**: `just play`, reach a race, escape; then `just play --race`, escape. Two things it would catch that the compiler will not - a held escape walking out through key repeat (guarded by `!event.repeat`, unverified), and re-entering the menus mid-race leaving a stale title or cursor. |
| **Nothing airborne has ever been captured** | `grip_air`, the airborne pitch gain and the `-0.3` airborne weathervane consequently have no runtime leg at all. One capture with a real jump in it closes several at once. |
| **Task #33: `oag-trace` cannot exercise the mag-lock hold** | `replay`/`drive` take one `Environment` for the whole run and the track samples change per tick, so a replay's blend is 0 by construction. Needs per-tick locator plumbing. It also carries the locator-fidelity lead: the hold explains 49.5 % of the inverted-section residual with nothing fitted, and the remaining magnitude points at the *locator* - our 4-per-segment resampled spline may not be the original's evaluated curve. |
| **Ship-to-ship collision stun** | `stun_timer`, its constant and the engine gate all exist and nothing arms them, deliberately: track contact does not arm it in the original. Ship-to-ship needs the writer of the pending impulse at `entity->0x4c + 0x110`, plus `0x0884ef30` (the two-body resolver) and the `+0xb8 == 6` zero-friction entity class - all unread. |
| **Sideshift's input side** | The force half is recovered and pinned (world-space, row-0-aligned, 0.2 s per-side timer, grounded-gated; the direction is evidence now rather than a coin flip). The tap history that triggers it is unread; the capture recipe is written down in the physics docs. |
| **Task #38: the penetration-escape surface gate** | The original gates escape on surface type 1 only; the crate uses 1-or-3. Noticed, not acted on. |
| **Track art's texture pairing is now solved too, checked separately from ships** | The delta-(-1) directory rule (proven for ships, 11 of 11 exact) turns out to also hold, checked independently, for each circuit's own `track.vex`/`track_reversed.vex`: 27 of 32 exact, the other 5 short 1-2 slots. `oag_game::race::load` now tries it for the track model the same way it already did for a ship. What is **still** a separate, harder problem: a model built from several small pieces sharing one atlas (the disc-wide 535/975 figure is about that pool, not about tracks) - see [`ps2-texture.md`](docs/formats/ps2-texture.md#how-a-model-finds-its-texture-set-directory-position-not-a-name). |
| **Task #37: the PS2 `.ipf` backdrop renders but is not wired as the looping FE Screen backdrop** | Boot never reaches FE Screen. Front-end work, not format work - [`ipf.md`](docs/formats/ipf.md) is closed at 92. |
| **Which movie cut plays, and what plays the three 260-frame reels** | The reel set is region-invariant, so the disc's region cannot be the picker, and nothing in the boot path plays them at all. [`pmf.md`](docs/formats/pmf.md). |
| **The PS2 PAL/NTSC selector's ultimate trigger** | `Movie_ResolveSourcePath` picks on global `0x0027a85c`; what decides that value (a numbered case in dispatcher `FUN_00186ed8`) was time-boxed away deliberately and nothing needs it yet. |
| **Task #31: `oag-view --collision` panics** | On an empty vertex buffer, for any file with no recognised collision class. Found against Pure; a shipped tool bug. |
| **Two PS2 identifications, one of which is wrong** | `0x0015d058` zeroes the same four accumulators as the already-named `Body_ClearAccumulators` (`0x0015ca48`). Left unnamed per ADR-0005; cheap to resolve. |
| **The SAP clamp globals** | Both are all-zero in shipped `.data` with no writer anywhere in the image (xrefs, operand scans over all 635,898 instructions, `.ctors` - all checked), so taken literally the clamp is degenerate. Confidence 45, and nothing depends on which way it resolves. |
| **The sRGB lead: real, and *not* the sprite sheet's fix** | Confirmed and fixed for `oag-view`'s mesh capture: `mesh_render::screenshot` now targets `Rgba8UnormSrgb` so the hardware encodes on write, as a window's surface does. **The fix is different from the sprite sheet's, and that is the part worth remembering.** The sprite sheet forks its *texture* format on `format.is_srgb()`, which works only because sprites are unlit - at light 1.0, `enc(dec(s)) == s`. `mesh.wgsl` multiplies a linear texel by a light rig, so the multiply lands between decode and encode and the texture-format fork stops being equivalent. Measured on the Feisar livery, same view rendered both ways: encode-on-write reproduces `srgb_encode(old)` to 99.3 % within 1/255 (mean 0.29/255), while the texture-format fork differs from it by a mean of 54.6/255. Applying the sprite fix here would be the wrong one. |
| **FSR 1 is in, off by default, and the default question is open** | `oag_render::post::fsr1` transliterates AMD's MIT `ffx_fsr1.h`; the route for FSR 3.1 is settled in [ADR-0012](docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md). On the GRAPHICS page as `[graphics] upscaler = off|fsr1`, honoured **only where it is magnifying** - it is a magnifier, and asked to minify it undoes the supersampling it was handed, so a 200 % capture with it on is byte-identical to one with it off, deliberately. **Verified**: constants against upstream's formulas; a hard-edge upscale read back; ringing pinned at 10/255 across a 128-level edge at the shipped sharpness (16/255 at maximum); and that switching it on does not move a flat colour, which is the whole colour-space arrangement in one assertion and fails by 5/255 if the sRGB decode is swapped for `pow(c, 2.2)`. **The one comparison run** - `just compare-upscalers <image> <scale>`, one frame of one track at 50 % - says FSR 1 is markedly sharper, while the aggregate error against a 200 % reference ranks it *worse*. That is the trap in "Measuring a renderer change" showing up twice in one session; trust the images. **Why the default did not move anyway**: one frame of one track is not the sample `animated_textures` was held to, and the specific doubt is about content that frame did not contain. `Framebuffer::resolve` runs for the front end and the menus too, and those are 480x272-era paletted sprite art and glyphs off an R8 coverage atlas - what a sharpener rings on. Modest ringing on a synthetic edge is not the same as invisible on a glyph. **What would settle it**: a few tracks including a dark one and one with heavy alpha, plus a menu page. **The menu page is the blocker** - `--presented` only reaches the race hand-off, because `capture::run`'s front-end path has no `Framebuffer`, and giving it one is the same piece of work as the UI-compositing restructure ([modern features](docs/overview/modern-features.md) has the prerequisite table). **Do not** read a menu capture taken with `--upscaler` as evidence either way: the flag changes the UPSCALER row's own text, so the two images differ for a reason unrelated to the resampler - that nearly produced a false conclusion here. |
| **Menu rows can now warn, not just grey** | `warn_when` marks a row whose setting is stored but currently doing nothing, in amber, with one message line under the page - as against `disabled_by`, which greys a row that cannot be changed *now*. The distinction is who caused it: a warned row is live and takes effect the moment the row it conflicts with moves. A warning takes **at least two** conditions and the loader enforces that, because a conflict is between settings plural; the first version named only the render scale and so fired whether or not the upscaler was even selected. Its value list is pinned to `upscale::magnifies` by a test, so the guard that declines to run the upscaler and the message that explains it cannot drift. [menus.md](docs/architecture/menus.md) has the rule. |
| **Do the *game's* capture paths need the same encode-on-write?** | `race.rs`'s and `capture.rs`'s targets are still `Rgba8Unorm`, and the 3D scene in `race.rs` has exactly the defect `mesh_render` just had. What stops it being the same one-line change: switching the target also encodes the HUD's and the front end's **authored text and fill colours**, and whether that matches intent or breaks it is unestablished. `render.rs` measured the sprite case, and sprites are unlit so the answer was clean; text has no equivalent measurement and there are no committed reference PNGs to diff against. **Settle it with a window-versus-capture comparison of a text-heavy screen first** - do not apply it on the inference that the colours were tuned against the window, however likely that is. |
| **Front-end gaps behind `Image`** | A `Viewport` is an undecoded clipping rectangle; `screen.rs` reads only a screen's direct children, so `Show Logo`'s `BOOT_LEGAL` line is silently absent; an `Image` with no `x` is centred on a guess (16 pixels either way, on the one case that exists). |
| **Pure's dangling `FEGlobals->TextColor`** | Referenced at seven sites in Pure's own `Skin.xml` and defined nowhere in that file. Either a base skin XML that `Screens::from_xml` does not merge across `LoadXML` includes defines it, or the original engine has a genuine dangling reference. **Not a parser bug on this side** - worth knowing before someone debugs one. |
| **SteamOS's own glibc version** | Unestablished. `just appimage-portable`'s floor is `GLIBC_2.34` (`objdump -T` and `readelf -V` agree, and the determinism test passes inside that container, so the two builds are interchangeable for the simulation). If `GLIBC_... not found` ever appears on a Deck, the version it names is the missing datum. [`packaging.md`](docs/tools/packaging.md). |
| **The chase camera's 3/4 factor** | The eye sits at exactly 0.75 of the authored external offset, at rest and at speed alike, and where that factor comes from was not found - do not hardcode it. [`camera.md`](docs/ghidra/functions/psp-pulse/camera.md) has the samples. The behaviour is right; only `chase.rs`'s prose is wrong, and the tidy-up (one sign flip in `oag_render::camera::chase::anchor` plus deleting `race::chase_pos_length`) is behaviour-neutral and undone. |

## Pending maintainer decision: shipped design data in tracked docs

[`handling-stats.md`](docs/formats/handling-stats.md)'s own rule, per
[ADR-0006](docs/architecture/adr/0006-no-copyrighted-content.md) - "a field name is a
description of the format, a tuning table is the content itself" - is breached
in places by earlier passes. `angular-velocity-column.md`'s shipped
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
| **The Memory Stick and profile system** | One automatic save slot, the way a modern PC game does it. The disc's boot chain between `Language Selection` and `Main Menu` is eight screens, and all but two exist to serve Memory Stick mechanics. When M6 opens, model `LogoFMV`, `LogoFMVRedirectScreen` and `Show Logo`, then go straight to `Main Menu`. Do not read the omitted screens as unimplemented work. |
| **PSP over PS2 where they diverge** | User directive, now policy in [goals.md](docs/overview/goals.md#scope). PS2 stays the corroboration leg for confidence scoring; genuine behavioural divergences implement the PSP side and document the PS2 one in a table. |
| **The collision stun is not armed by track contact** | The original's is gated on a pending impulse the contact path never writes, and `stun_timer` reads 0.0 on all 3,446 recorded ticks across both captures. The timer and its gate stay; nothing arms them yet. |
| **Sweep and prune is not reimplemented** | The original's packing clamps world space rather than rebasing it, so it never drops a genuinely overlapping pair - it is an accelerator, and exact AABB overlap is a legal and strictly better substitute. The 1-unit quantisation and 1024-id cap are properties of the original's index, not of the format. |
| **The four-corner hover variant** | Not implemented. Its selector is known (`DAT_08ab07e3 == 0 && DAT_08b31048 == 6`, which also disables the brakes and swaps in an auto-speed law), and it is why the PS2's bank-to-yaw `50` was never a contradiction with the two-point path's `30`. |
| **Pure asset work** | Deferred behind M4's exit per [ADR-0009](docs/architecture/adr/0009-multi-game-fanout.md); the probe ran and [`pure-status.md`](docs/formats/pure-status.md) has the table. Two render blockers: the `.vex` class-ID table is renumbered wholesale (five constants, but the right shape is a per-title table keyed off the version word, which is a design call), and Pure's embedded model textures ship pre-swizzled behind a flag byte at `+0x06` that `vex::textures` never reads. |
| **A "CPU renderer" switch** | There is nothing to switch. wgpu ships no software rasteriser (`Backend::Noop` draws nothing), so CPU rendering exists only when the *system* has a driver - Mesa lavapipe, or WARP on Windows - which wgpu then enumerates as an ordinary adapter reporting `DeviceType::Cpu`. The GRAPHICS -> RENDERER row therefore lists what the machine actually has and tags that one `(cpu)`; it is absent on a machine without lavapipe rather than present and failing. `force_fallback_adapter` was rejected for exactly that reason. See `crates/game/src/adapter.rs`. |
| **RENDERER applies on the next launch** | The device is made once at boot and everything hangs off it. Live switching means tearing down the surface, the upscaler framebuffer, every pipeline and the current `Stage`; that is a bigger change than the row and it buys a relaunch. What *is* handled now is recovery: `Gpu::bring_up` retries on `default` when the named adapter enumerates but will not produce a device or configure the surface, because `apply_setting` saves on every keypress and a setting reachable from inside the game must not be able to lock a player out of it. Verified by forcing an impossible `required_limits` on the named adapter. |
| Everything milestone-scoped | Weapons, AI, audio, netcode, the shell: [roadmap](docs/overview/roadmap.md), not here. |

## Working rules that were learned expensively

- **You cannot measure the original's exhaust by thresholding a screenshot, and the
  exhaust has *two* independent inputs that both have to be matched.** Two traps, one
  pass, both cost a wrong conclusion.

  First, **intensity is driven by time under thrust (+0.25/s to a ceiling of 1.0), and
  ribbon length by speed (`10 samples x speed`)**. Matching only speed produced a pair
  3x apart on brightness and an "our flare is too dim" reading; matching only thrust
  time gives the opposite. A comparison frame is worthless unless it pins both, and our
  craft reaches 64 units/s in 1.2 s where the emulator took roughly 6 s - so the two
  may not be *able* to sit at the same speed and the same intensity at once. If so that
  gap is a physics question surfacing through the exhaust, and it must be separated
  before another exhaust constant is touched.

  Second, **brightness thresholding measures Talon's Junction, not the plume.** On the
  start straight the road's own light strips saturate a **245-pixel** run in a band
  with no exhaust in it, against 108 px in the band containing the ship. Any
  "widest saturated run" statistic is therefore dominated by scenery, and it reads as a
  clean number. Our own frames do not have this problem (flat ribbon over black), which
  makes the comparison asymmetric in exactly the direction that flatters us.

  **The cheap remedy is to capture on a dark part of the circuit.** Talon's Junction
  has tunnel sections where the scenery contributes almost nothing, and the plume
  reads clearly against them; the start straight is the worst possible choice and is
  what `straight-line.inputs` happens to run along. Pick the frame by *both* speed and
  mean frame luma - a burst logging the two is a few lines - rather than by speed
  alone.

  **The rigorous method, if a number rather than a picture is wanted, is difference
  imaging against the emulator's own memory**: break, capture, write zero into the trail's three layer-colour fields
  (`child+0xf8` / `+0x128` / `+0x158`) or the intensity at `flare+0xbc`, capture again,
  subtract. Same pose, exhaust removed, so the difference *is* the plume in isolation
  and can be measured against hull width. The missing piece is the flare object's
  runtime address - it is a scene-graph child of the craft, and `craft+0x794` is the
  already-traced ship node to walk from. **Do that before tuning
  `HALF_SIZE_TO_WORLD`**; until then it stays at 1.0 and confidence 55, and "our bloom
  looks bigger" stays an impression rather than a measurement.
- **An xref count on a class descriptor bounds *authored* instances only.** The
  exhaust pass concluded that racing craft have no trail, because `Trail`'s class
  descriptor is referenced by nothing but its own registration function and no
  `Ship.vex` authors a `Trail` node. Both facts are true; the conclusion is false.
  The descriptor is used **only** by the class-table lookup the `.vex` loader takes
  to instantiate a node by ID - code that builds an object directly calls its
  constructor and assigns the vtable itself, touching the descriptor never, which is
  exactly what `ExhaustFlare_Init` does. **What refuted it was looking at the
  emulator**, which shows a trail plainly. A side-by-side against PPSSPP costs
  minutes and would have pre-empted a day's worth of wrong prose; take one before
  concluding anything is absent from a *visible* subsystem.
- **A claim that the crate is missing a term is a claim about the crate.** It
  has been wrong three times - the airbrake drag term, the hover downforce
  magnitude and the roll-oscillator note were each recorded as unimplemented in
  prose nobody had checked against the code, and twice the correct value was
  sitting in the same file a hundred lines above the claim.
- **A page can go stale while a sibling page in the same tree already carries
  the correction.** It has now happened three times (the front-end font claim,
  the airbrake drag term in four places, M4's blocker for 31 commits).
  `git log -S <symbol>` against the doc's own last-touched commit settles it in
  one command. **A fourth, one hop further downstream: the *code* can be stale
  while a doc already carries the correction**, for the same reason - nobody
  reread the sibling page before writing the implementation. `rigid-body.md`
  recorded on 2026-07-28, in passing, that `<Misc>`'s dimensions are scaled by
  `0.75` before reaching the box collider; `crates/physics/src/wall.rs` kept
  building that box unscaled for two days until a second, independent chase (the
  six-tick-early wall contact) landed back on the same instructions. `git grep`
  for a crate's own constants across `docs/` before implementing a value that
  "must be" a plain field read costs less than rediscovering the same
  instruction sequence.
- **Never write a fitted constant in as a recovered value.** Every one that has
  been chased turned out to be something real - an inverse-inertia entry, a box
  tensor, a friction coefficient - and writing the fit in would have closed the
  question at exactly the wrong moment.
- **Check the control columns agree before reading anything else in a
  comparison.** They are a pure function of the script and `dt` - no trajectory
  reaches them - so a mismatch there is a harness bug by construction and every
  physics number below it is meaningless. The two-tick lead offset sat in plain
  sight in `oag-trace compare`'s own output (`throttle max error 1.000e2 at tick
  1`) for a session while the rows underneath it were read as a wedge. **Read the
  table from the top.**
- **A control that ramps without clamping has no attractor, and an input error
  in one never decays.** The steering ramp oscillates around a saturated target
  forever; the airbrake ramp clamps and resynchronises on every transition. Same
  two-tick error, one washed out in six ticks and the other survived 3,146.
- **Check a capture is clean before fitting anything to it.** `speed` and
  `|velocity|` agree to 1e-6 in free flight, so `speed/|velocity| == 1.0000` is
  a free contact detector. Two reference captures were recorded scraping a wall
  for their whole length, and the resulting "missing linear resistance" stood as
  the M4 blocker for a session and a half.
- **"Which way round the circuit" is not a question the sample index can
  answer.** Asking whether the nearest-sample index rises or falls along a
  capture pointed at the *wrong* variant: 278 ticks forward against 366 back on
  the layout the craft is provably driving. Two reasons, both structural.
  `Spline::from_track` concatenates paths in file order, so on a track with
  junctions index order is not travel order; and on a capture averaging 0.33
  units/tick with 67 % of its ticks in wall contact, 2,501 of 3,145 windows step
  by zero and the rest are jitter. `dot(craft_forward, sample.tangent)` has
  neither failure mode and reads `+0.9999` against `-0.9999`. **Ask the tangent,
  not the index.**
- **A number in authored data that looks like a height usually is not one.**
  `Start Position`'s `y` reads exactly like a ride height on `01_Track` (2.42,
  between the surface and the lifted spline) and is not one: measured against
  each track's own collision mesh it ranges **1.03 to 7.36** across the 40 files.
  Checking it on one track would have shipped a spawn that only works there. The
  same shape of check is cheap for any authored constant: ask what it is a height
  *above*, then measure that on every file rather than the convenient one.
- **When position and heading disagree about whether a reading is right, the
  heading is the one that can carry the argument.** The authored grid slot is
  139.7 units from where the original's craft actually starts, which is equally
  consistent with a misread matrix and with a grid behind the line. Its forward
  agreeing to 1.12 degrees is consistent with only one of those. Rotations have
  no free parameters; positions have three.
- **A fidelity fix that changes nothing measurable is still a finding.**
  `raycast_all` moved the whole-lap scenario by not one digit (617.538 units
  either way) and single-sided rejection moved it by 1.1 - **both measured from
  the old spline-sample-0 spawn, so neither figure reproduces since `4b2236a`
  moved the start onto the authored grid slot; the finding stands, the absolute
  numbers do not**; both were real
  divergences from `Collision_BoxAgainstMesh` that `16_Track`'s geometry never
  exercises. Written down, that is knowledge; unwritten, it gets re-litigated.
- **The ramped columns (`craft+0x2c0`-`0x2c8`) lag the frame that used them by
  one tick** - ramp-then-consume in one call, captures sample at entry. Naive
  pairing multiplies a yaw fit's rms by 5.6.
- **The recorded basis columns must be read as Left-Up-Forward.** Taken at their
  column names the basis is a reflection, and a 1.25 rad/s residual reads as 32.
- **Never read timing off the HUD under breakpoints.** The race clock does not
  count emulated frames; laps of identical tick counts timed `0.50.25` and
  `1.11.08`. The lap *counter* is unaffected.
- **"Read far enough to rule X out" is how 80 instructions of a different
  mechanism stayed unread for three passes.** `Ship_UpdateMagLock` had been
  opened twice, each time only to the depth that day's question needed; its tail
  turned out to rewrite the basis directly and was the whole of the remaining
  attitude gap.
- **Time-boxing pays.** Two unread leads (the PAL/NTSC selector's trigger, the
  `.IPF` container) were left alone deliberately and the video got playing
  anyway, because the measured facts did not need them.
- **Diff the output, not the compile.** A shader uniform field existing is not
  the same as a shader reading it: `Draw::Video`'s `rect` was computed and
  discarded on the way to the GPU, and the first "fix" produced a
  pixel-identical screenshot while silently doing nothing.

## Traps that are live

**Reading the discs:**

- **`ends_with("Data.wad")` also matches `BEData.wad` and `FEData.wad`**, and
  `BEData.wad` sorts first and holds no tracks. A survey written that way
  reports **zero** section nodes on the PSP disc and reads exactly like "this
  format is not on this platform". Cost a full detour on 2026-07-30. Match the
  full path, the way `collision_ground_truth.rs` always has.
- **Pure does not share Pulse's `.vex` class numbering.** Pure track files use
  classes around `0x36f..0x393` where Pulse uses `0x3b9..0x3e9`, so any Pulse
  `CLASS_*` constant matches nothing on that disc and every Pure survey comes
  back empty. Pinned by `pure_does_not_share_pulses_class_numbering` so it is
  not rediscovered as a parser bug. Recovering Pure's own table is open work.
- **Shipped `section` data is not tidy, and a strict parser refuses real
  tracks.** ~2 % of PVS mask bits name a section the file does not declare, one
  id is authored three times over on four tracks, and 116 PS2 control points
  name a section their file lacks. All benign; all would be "corruption" to a
  parser that assumed otherwise. See `docs/formats/track.md`.

**Measuring a renderer change:**

- **An aggregate statistic can rank a broken change as the better one.** The
  first mesh-to-section rule placed a draw call by the single section holding
  its bounding-sphere *centre*. It reported **fewer** draw calls surviving the
  cull - which reads as better culling - and it deleted a ridge line and a
  building from the frame, because geometry wider than a section has its centre
  in only one of them. **A screenshot comparison caught it and no counter
  would.** `--screenshot` now honours both culling settings precisely so this
  comparison is runnable; two captures at the same `--ticks` differing only by a
  setting must be byte-identical.
- **Do not add per-frame work to save per-draw work without measuring both
  sides.** Building the PVS visible set needs the craft and the camera located
  on the spline, and `Spline::nearest` walks all ~3,400 samples - twice a frame
  would have cost the same order as the frustum tests it saves. The craft's
  index is already cached in `Ship::segment` by the simulation's own tick, and
  the camera is found in a 96-sample window around it.

**Process, in a shared tree:**

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

**This machine:**

- `data/` is on `ecryptfs`. **`cp --reflink` does not work** (copying the three
  images is a real 4 GB copy), and `ls`/`rg` intermittently fail with
  "permission denied" or "no such file" on directories that plainly exist. `fd`,
  `find` and absolute paths work.
- **Backticks in a justfile are command substitution at parse time**, even
  inside an `@echo` string - an echoed example command once ran a full
  trace-compare on every `just` invocation, `just --list` included.

**Tools that fail silently rather than erroring:**

- **`ffprobe -show_entries` does not print fields in the order requested** - it
  uses its own. Parse `key=value` pairs by name (`-of
  default=noprint_wrappers=1`, keys kept); positional parsing reads the wrong
  field and says nothing.
- **`ffmpeg -lossless 1` on its own is silently a lossy encode.** The quantiser
  must be pinned and rate control disabled too: `-b:v 0 -crf 0 -qmin 0 -qmax 0
  -aom-params lossless=1`. Even then `-cpu-used` below 6 is reproducibly not
  bit-exact; 6 and 8 are exact, and faster.
- **A niri screenshot piped through `wl-paste` can return the PREVIOUS clipboard
  image.** One lap "verification" shot came out as a menu from an earlier
  session. Check the image is what it claims before citing it.

**PPSSPP** - the rest is in
[`ppsspp-debugger.md`](docs/reverse-engineering/ppsspp-debugger.md), worth
reading before a capture session rather than after:

- `pkill -f PPSSPPSDL` inside a shell command whose own text contains that
  string kills the calling shell.
- A `memory.write` with an empty base64 payload kills the emulator outright.
- `psp-drive.py restart` fails about one time in three with a transient
  `Invalid address` (the craft is read while the race loads) - retry.
- `replay.flush` crashes v1.20.4 whenever the recording spans a screen
  transition, three reproductions. The replay API is closed as an option.
- Restore the savedata backup (move, do not delete) before concluding a session
  that touched `~/.config/ppsspp/PSP/SAVEDATA/UCES00465P0000`.
- **The USA PSP disc never shows the Language Selection picker** - the build is
  English-only and the front end skips it when there is one language to show.
  Proving that costs about an hour. A live capture of it needs a EU PSP disc or
  a PS2 BIOS in PCSX2, neither of which is available here; do not suggest
  sourcing one.

## Verification status: what to lean on

**Measured against the original running.** The forward force law end to end (a
standing start fits from `fs = 0.56` to `47.67` at rms 0.127 on a one-parameter
fit); lateral grip at 0.9985 of the disc value, 99.95 % explained; the yaw
accumulator term by term at racing speed, 99.94 %; the inertia tensor on all
three axes, pitch to 0.13 % and the `I * omega` identity to 0.01 %; the pitch
step response at 1.006x; the contact friction coefficient as a one-sided bound
approached from above; the mag-lock probe re-finding the magstrip from geometry
that knows nothing about the capture (91 % of inverted poses against 0.3 % of
upright ones); and the authored `Start Position` frame, whose forward lands
within **1.12 degrees** of the original's craft at the start line and whose left
lands within 1.54 - read off the disc by a parser that knows nothing about the
capture. Method and numbers in the two ground-truth pages under
[`docs/physics/`](docs/physics/README.md). Add to that the whole-lap result
above, now that the input is aligned: over the 170 ticks of the recapture that
are wall-free, **position tracks to 6.54 units at the worst and 2.41 on average**
on a craft doing 100 units/s, with speed inside 6.01 - and `grounded` exact on
3,146 of 3,146 ticks. That is the broadest runtime evidence the harness has
produced. Its limits are worth stating in the same breath: the capture is clean
on only a third of its ticks, so nothing after tick 171 measures our physics, and
`grounded` is a constant column on this capture, so agreeing with it says only
that our ship also never left the ground.

**Instruction-level reading with no runtime leg yet.** Mag-lock's blend weights
and its `|h - d| > 5.0` fallback; the swept collision path; everything airborne.
Single-sided wall rejection is measured against the *data* rather than against
the original running: **2,076 of 2,084 wall triangles on `16_Track` (99.6 %)**
are wound toward the nearest point of the track's own spline, which is what
makes reproducing the original's `dot(boxCentre - sample, n) > 0` rejection
faithful rather than merely literal.

**The determinism gate reaches the simulation now** (`dd53c1f`).
`oag_physics::probe` steps the same `oag_physics::step` the race loop steps,
over a scripted input and a synthetic corridor built in the file, so it runs in
CI on all three OSes rather than needing a disc image. Two mechanisms keep the
hashed field list from rotting and **both are needed**: exhaustive destructuring
makes a new `ShipState`/`Body` field a compile error, and a perturbation test
proves each field actually reaches the hash rather than merely being bound -
that second test caught a real hole on its first run, since `time_since_landing`
defaults to `1.0` and perturbing it *to* `1.0` was a vacuous no-op. What it does
not cover is stated in its module docs: maglock, reset, and the swept tunnelling
path.

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
