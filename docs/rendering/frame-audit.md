# Pulse frame audit: our race frame against the original's, same state

Read 2026-09-30, PPSSPP v1.20.4 (`pulse-psp-usa.chd`) against `oag-game --race`.
The question was which visible differences a player would notice first, and
whether their cause could be traced in the data or the decompile. **One was fixed
(texture level selection, below, later given the hardware's own law); everything else measured close, and the
remainder is ranked with what was ruled out.**

## How the two sides were matched

Every comparison is one frame of each, from one state, at the PSP's own size,
with nothing either side adds:

| | Original | Ours |
| --- | --- | --- |
| Size | 480x272, `InternalResolution = 1`, window 480x272 | `--size 480x272 --render-scale 100` |
| Filters | no post shader, `TexScalingLevel = 1`, `AnisotropyLevel = 0`, FPS counter off | `--msaa off --motion-blur off --screen-filter off --anisotropy off` |
| State | the craft's pose and camera read per tick by `psp-trace.py --camera --shot-every 1` | `--pose-from` that CSV, `--pose-tick`, **`--ticks 1`** |
| Config | own `HOME`/`XDG_CONFIG_HOME` under a scratch directory, not kept | an isolated `XDG_CONFIG_HOME` (English, bloom on) so the maintainer's `settings.toml` (200 % render scale, 4x MSAA, `psp-3000` filter) never reaches a screenshot |

Circuits: Talon's Junction (`16_Track`, bright outdoor), Metropia (`03_Track`),
Tech De Ra (`04_Track`, indoor). Same team (Assegai), same Venom time trial.
Moments: on the grid, mid-straight, at 41 units/s on a straight, mid-corner. The
corner moment was unusable - `place` left the original's craft tumbling in a
tube - and is not cited.

Three traps that each produced a false difference before they were understood:

1. **`--ticks 0` skips the PVS.** The visibility set is built from the craft's
   spline position, which does not exist before the first tick, so a
   `--pose-from ... --ticks 0` frame draws all 2,051 draws and a whole-circuit
   perimeter wall (`fw_wall01_pr`, `Building_05`, radius 1,270) covers the
   horizon. It reads as a large geometry bug and is not one. **Use `--ticks 1`.**
2. **The exhaust flare is state, not geometry.** At `--ticks 0` intensity is 0 and
   no flare draws; the original's idle craft shows one. Ours shows it from about
   tick 30 (`Exhaust_Update`'s ramp). A comparison of the hull's brightness at
   tick 0 reads the missing flare's bloom as a 15-35 % darker spine.
3. **The gantry and the strip lights are time.** The original was captured 0.1-2 s
   after GO; ours is a fresh state. The GO banner, the countdown board and the
   pulsing cyan wall strip differ for that reason alone.

## Ranked differences

Ranked by how much a player would notice. "Doc" names where it is already
recorded.

| # | Difference | Size | Cause | Status |
| ---: | --- | --- | --- | --- |
| 1 | **Scenery, trees, mountains and track surface drawn soft** | Laplacian standard deviation in four regions of one frame: original 13.4 / 6.1 / 14.9 / 7.5, ours before 10.8 / 4.4 / 11.3 / 6.3, ours after 12.7 / 6.2 / 15.5 / 8.2 | our box-filtered mip chain, where the original resolves the base level | **fixed** (the disc's levels, selected by the GE's slope law) |
| 2 | Hull sheen: a whiter, glossier spine and canopy in the original | spine mean 143,139,113 against 91,77,53 at tick 0, but 191,193,143 against 143,144,76 at tick 90 and the close crops read nearly equal once the flare's bloom is in | part flare bloom (state), part the hull's `0x2000` extra pass: six batches (not the one glass batch) drawn a second time under environment mapping with `envtest4bit.tga` | **drawn** (`oag_render::shine`); closes roughly a third to a half of the measured gap, the rest is bloom and pose, see below |
| 3 | Dark bowl-shaped objects on the grass left of Talon's straight at (161,-47,-185) | a few percent of the frame (about 600 pixels at 480x272) | draws **1479..1494** of the opaque list: the sixteen batches of four animated slab transports (nodes 789, 793, 804, 806 of `16_Track`'s `track.vex`, 320-460 units long, each with a `sound` child). The original's GE list never contains them; ours drew every moving batch past the section mask | **fixed** (the mask applies to moving draws), see section 3 |
| 4 | Track neon strips and the start gantry | animated | timing (trap 3), not measured as a defect | not a defect on this evidence |
| 5 | Sky, fog and tone | regional means agree within 5 % on all four frames (e.g. Talon mid, 18 cells: 101,129,146 against 108,137,156 at worst) | - | matches |

Camera height and FOV were not re-measured: `--pose-from` carries the original's
own camera, so this audit cannot see a camera difference. That question is the
race-camera FOV thread's (see `HANDOVER.md`'s open threads) and stays there.

## 1. Texture level selection (fixed)

The original **never minifies a scenery texture** at the distances a race frame
shows. Measured four ways against PPSSPP on Talon's Junction:

| Region (px) | Original | Chain (before) | Base level (after) |
| --- | ---: | ---: | ---: |
| left trees and mountain, 130x100 | 13.45 | 10.78 | 12.73 |
| road right, 100x60 | 6.13 | 4.38 | 6.24 |
| horizon, 200x60 | 14.94 | 11.32 | 15.47 |
| skyline, 120x50 | 7.54 | 6.26 | 8.19 |

(Laplacian standard deviation of the luma channel, 8-bit; higher is sharper.) The
same holds on the Talon grid frame's far track, where a chain would have cost the
most: 16.0 against the original's 15.9, and 12.8 for the chain. `AnisotropyLevel`
was 4 in the copied profile for the first pass and 0 for the confirming one; the
result did not move, so it is not the emulator sharpening oblique floor.

**What the binary says.** `Gfx_FlushRenderManager` programs the texture unit
once per frame: `Gu_TexFilter(7, 1)` (trilinear minify, bilinear magnify),
`Gu_TexLevelMode(2, 1.0)` (slope mode, bias 1.0) and one call to a `0xd0`
`TEXLODSLOPE` emitter, **`Gu_TexLodSlope`** (`0x08811694`), with the float
`0x3b800000` = `1/256`. That emitter is the only one in the binary, and this is
its only call. The GE's slope law is `level = log2(|z| * slope) + bias`, which
depends on **depth, not on texel density**: level 0 up to a view depth of 128,
level 1 at 256, level 2 at 512. That is why a 480x272 race frame barely leaves
level 0, and why PPSSPP, whose GPU backends do not implement slope mode, draws
level 0 everywhere.

**Implemented as the game's rule, 2026-09-30 follow-up.** The first fix capped
every PSP `.vex` texture at one level, which is PPSSPP's behaviour. The law is
now in the shader:

- **The levels are the disc's, not ours.** The texel block of a `.vex` texture
  carries every declared level after the base, at the padded stride the sum-check
  on `texture_row_stride` establishes. `oag_vex::vex::EmbeddedTexture::levels` reads
  them (all 135 textures of `16_Track`, and the level 1 of most differs from a box
  filter of the base, which is asserted), and `Texels::Chain` carries them to the
  GPU in place of the box-filtered chain this project used to make. A pre-swizzled
  Pure texture keeps a synthesised chain capped at its declared depth.
- **The selection is the game's**: `mesh.wgsl` takes
  `textureSampleLevel(..., max(log2(view_depth / 256) + 1, 0))` when the model
  carries a chain (`texlod_slope`/`texlod_bias` pipeline constants, from
  `mesh_render::PSP_TEXLOD_SLOPE` and `PSP_TEXLOD_BIAS`), clamped to the levels
  that exist by the sampler. The fractional part blends two levels, as the
  trilinear filter asks. **Chosen, not measured**: `|z|` is the vertex's view
  depth (`clip.w`) - an unverified reading of the GE's own `z` - and
  `Texture_BuildBindList` emits its own mode and bias per texture, unread, so a
  texture on its mode-0 branch may not follow this rule. The two constants are
  recovered (confidence 85 on the emitter, and the values are read off the
  call), the mapping to our depth is not.
- **Proof.** `crates/render/tests/psp_slope_lod.rs` draws one quad with a
  red/green/blue three-level chain at view depths 64, 128, 256, 512 and 100,000
  and reads levels 0, 0, 1, 2, 2; a model without `Texels::Chain` stays on level
  0. Disabling the constants fails it at depth 256.

**Near field kept, far field softens.** Same four Talon-mid regions as above,
luma Laplacian standard deviation: original 13.45 / 6.13 / 14.94 / 7.54, base
level only 12.73 / 6.24 / 15.47 / 8.19, slope law 12.73 / 6.24 / 15.08 / 8.19.
Three regions sit under 128 units and are byte-for-byte the base level; the
horizon strip, which reaches past it, moves from 15.47 to 15.08 - nearer the
original's 14.94. A long-straight frame (`--pose -297.96,-50.5,-172.83`, Talon,
`--ticks 1`, no original: PPSSPP draws level 0 here by construction) reads, base
level against slope law, 24.51 to 22.77 and 28.36 to 26.40 in two strips past
about 256 units (whole frame 16.74 to 16.56, 74 pixels differing by more than
1 %). That gap is the level 1 the hardware rule selects and the emulator does
not; whether real hardware shows it is still unmeasured.

**TEXTURE DETAIL** (`[render_profiles.<title>] texture_detail`, `--texture-detail`,
the graphics page's row beside MODEL DETAIL) is one multiplier on that law, as
levels: `original` (default) is the recovered slope law exactly; `high` doubles
the depth of every level step (the level shifted by -1); `maximum` never steps
(level 0 at every depth). `high` and `maximum` are **chosen, not measured**. It is
`Fog.texlod_shift` in the scene uniform (`mesh_render::TextureDetail::level_shift`,
zero for `original` and for any buffer nothing writes), added to the level in
`mesh.wgsl`, and the race hands the preset over every frame, so the row applies
live. Proof: `psp_slope_lod.rs` reads the level each preset selects at depths
64 to 100,000 (a preset that stops reaching the shader fails its own test), and
`texture_detail_ground_truth.rs` runs the binary per preset on the Talon straight
and asserts the three pictures differ pairwise.

Talon long straight (`--pose -297.96,-50.5,-172.83`, `--ticks 1`, 480x272),
luma Laplacian standard deviation, same metric as above (`magick ... -colorspace
Gray -define convolve:scale='!' -morphology Convolve Laplacian:0`, sd x 255):

| Region | original | high | maximum (= the old base level) |
| --- | --- | --- | --- |
| Whole frame | 16.56 | 16.70 | 16.73 |
| Strip 120x40+120+80 | 21.37 | 22.49 | 22.80 |
| Strip 120x40+120+0 | 32.52 | 32.92 | 32.92 |

`original` is softest and `maximum` sharpest, and the whole frame reproduces the
previous pass's 16.56 (slope law) and 16.74 (base level). The strips are the two
with the largest change. The previous pass's own boxes were not recorded, and its
24.51/22.77 and 28.36/26.40 cannot be recovered from the saved frames
(our own frames, not kept in the repository, reproduce its whole-frame
16.74 and 16.57, but hundreds of boxes match each pair to two decimals), so they are
not directly comparable. Only 74 pixels differ
between `original` and `maximum` at this size: the far field is small at 480x272.

**`--anisotropy` works again on a PSP `.vex` model.** The slope law first sampled
with `textureSampleLevel`, an explicit level, and a GPU takes no anisotropic
footprint from one: the same pose (Talon straight) with `--anisotropy off` against
`16x` differed in 602 pixels over 1 % with the derivative path (`textureSample`
over the same chain) and 5.6 with the slope law. `mesh.wgsl`'s
`sample_at_slope_level` now takes `textureSampleGrad` with the screen-space
derivatives reshaped so the level the hardware picks is still the law's (plus
TEXTURE DETAIL's shift). With `w = 2^level` the width of one texel of that level
and `pmax` the footprint's long axis in base texels, `probes = clamp(ceil(pmax / w),
1, clamp)`; the long gradient is set to `probes * w`, the short one to `w`, each
along its own direction, so the hardware reads a ratio of `probes` and a level of
`log2(probes * w / probes)`. The sampler's clamp reaches the shader as the
`aniso_max` pipeline constant. Anisotropy off, or a footprint no wider than a
texel of the level, is one probe and the plain explicit level.

Two things this is not. It **cannot sharpen**: the level is the slope law's and
stays so, so anisotropy only antialiases along the long axis of a foreshortened
surface (less shimmer and moire on far girders and floor), and TEXTURE DETAIL is
the control that shows finer levels. And a first version that set the probe count
from the surface's own ratio, ignoring how big the footprint was, blurred near
walls by up to 16x the footprint; sizing it from `pmax / w` leaves everything a
texel or narrower alone. The same pose, `--anisotropy off` against `16x`, now
differs in 22156 of 130560 pixels at 480x272 (72975 of 921600 at 1280x720),
concentrated on far edges and grazing floor; near surfaces are unchanged.
`crates/render/tests/psp_slope_lod.rs` asserts the level under every anisotropy
setting with a stretched footprint (so a level that drifts fails), that 16x moves
a foreshortened striped surface, and that it moves nothing when the footprint is
under a texel.

**A consequence to watch**: a coarse authored level of a cutout texture has alpha
above the reference nearly everywhere, so a cutout batch (trees, fences, pad
glows) reads fatter at depth than it did with a base level. Wipeout Pure's
speed-pad guard (`pad_alpha_test_ground_truth`, an orbit framing hundreds of
units out) stopped discriminating for exactly that reason and now compares at the
base level, which is what it is about; the discard operator's own test is
unchanged. Whether the original's distant cutouts thicken the same way is part of
what a hardware capture would settle.

The change applies to every title that loads a `.vex` with embedded textures:
**Wipeout Pure is unmeasured** against its own capture and shares the rule.
`crates/render/tests/psp_texture_levels_ground_truth.rs` fails if a circuit's
textures stop arriving as the disc's own chain.

## 2. The hull's sheen (drawn, gap partly closed)

What was ruled out: the hull's light list (it is the recovered `AmbientLight` and
three `DirectionalLight` nodes, and the wings match to 1 %), bloom (toggling it
changes neither the trees nor the spine), and the idle flare (a state artefact,
trap 2). What remained was a gloss the original has on the yellow spine and the
inner wings.

**Cause, read live 2026-09-30**: the original draws six of the hull's fourteen
batches **twice** - `Ship.vex`'s `0x2000` batches (`shipShape`'s three blended
ones, both airbrakes' and the canopy glass) - first under environment mapping
with their material's **second** texture, `envtest4bit.tga` (a 64x64 black map
with three white glints), then their ordinary pass added on top. The lead this
page used to carry, the single `Glass_ADD` batch, was one of the six and not the
cause alone; the "naive wiring" that left the hull unchanged drew the wrong
texture (the glass's own) on the wrong batches. The hull model's `+0x1a8` is **1**
(read live), so the coordinates come from the fixed world-space basis and follow
the ship, not the camera. Evidence, the recorded GE list and the loader rule that
makes five of the six *replace* what is under them: `mesh-draw.md`, "The hull's
extra pass". Implementation: `oag_render::shine`, drawn after the hulls through
the absorb overlay's plumbing; `crates/game/tests/shine_ground_truth.rs`.

Matched poses (Talon's Junction, Assegai, 480x272, no filters; RGB means, spine
box of `talon-mid2`, ship box of `talon-fast`):

| Frame | Original | Ours without | Ours with |
| --- | --- | --- | --- |
| `talon-mid2` spine | 153, 154, 95 | 135, 139, 85 | 143, 148, 94 |
| `talon-mid2` whole ship | 142.5, 161.0, 161.4 | 134.6, 154.1, 155.2 | 137.1, 156.6, 157.8 |
| `talon-fast` ship box | 155.6, 173.2, 170.5 | 149.1, 161.7, 158.9 | 150.8, 163.4, 160.8 |
| `talon-corner` ship box | 14.4, 26.5, 27.8 | 35.0, 36.8, 60.2 | 35.4, 37.1, 60.5 |

The pass moves every measured region toward the original and never past it, and
the glint reads as a soft sheen on the inner wings and canopy rather than a flat
lift. **It does not close the gap**: a third to a half of it on the first two
frames, nothing on `talon-corner`, where the ship is mostly out of frame and the
difference is framing. What remains is the flare's bloom and small pose and
camera differences, and, until 2026-10-01, the airbrakes' batches not being deflected with their
flaps (they are now, see `mesh-draw.md`). Re-read 2026-10-01 on `07_Track`
([mesh-draw.md](../ghidra/functions/psp-pulse-usa/mesh-draw.md), "Re-read on a
second dump"): one additive redraw **is** the original's replace-then-add sum, and
the pass is fogged to black, as the original's is. A circuit's own `*_shinemap`
batches are drawn under the view-matrix matcap the same day.

## 3. The factory roofs (fixed)

**The first reading was wrong twice.** The audit named draws `1381..1391`
(`factory_floor_01_rp_shinemap`, `Building_05`, `Building_07`, `energy_GLOW`,
`piston_end_shinemap`) and asked whether the original's terrain hid them. An
index bisect of the opaque loop at the audit's own pose (`talon-fast`, `--ticks
1`, PVS on) shows those draws change **no pixel** (0 differing at a 3 % fuzz);
the bowls are draws `1479..1494`, and excluding only them removes the bowls and
leaves the terrain, the silo and the trees as the original shows them. Nor was it
the terrain drawing late: the roof meshes are not in the original's frame.

**What the roofs are.** Four animated nodes of `Data\Environments\16_Track\track.vex`
(`0x3c0 Anim Transform` parents with a `sound` child each; nodes 789, 793, 804 and
806), three batches apiece, flat slabs: 22, 143 and 71 vertices, 320 to 460 units
long, 15.6 units tall, 29.7 wide. They move along authored keys (across
`--anim-seconds 0 .. 8` they slide but stay beside the straight, so this is not an
animation-phase difference) and all carry `DrawCall::moving`, which exempted them
from **both** culling tests.

**Discriminating question, written before the capture.** In the original's GE list
at this pose, is a batch with these vertex counts and extents submitted? *Present*
would make it a depth or draw-order difference on our side; *absent* a section or
node-visibility difference. PPSSPP's `gpu.record.dump` returns the frame's whole GE
list ([`psp-ge-dump.py`](../../scripts/psp-ge-dump.py); method in
[ppsspp-debugger.md](../reverse-engineering/ppsspp-debugger.md), "Reading the
frame's GE list"). Each PRIM's world-space box under its world matrix gives a
signature (vertex count, sorted extents), and the same signature is taken off our
own draws.

**Result: absent.** Six dumps over four poses on one boot (`talon-fast` twice,
`talon-mid`, `talon-mid2` twice, the start grid) and a cold second boot at the
fast pose: no PRIM of any of the seven carries a roof signature (0 hits, not a
near miss). A moving mesh's box changes with its rotation, so the absence was
re-checked with numbers a rotation cannot change: **no dump holds a single
143-vertex PRIM** (the slabs' main batch, in all four roof nodes), and the only
22/35/71-vertex PRIMs whose longest vertex-to-vertex distance comes within 5 % of
a slab's are a 35-vertex object at one fixed world box in three dumps taken
seconds apart (principal extents 264 x 117 x 71 against the slab's 264 x 30 x 2.5,
so not the slab, and not moving). Positive control, same method: 88 to 98 % of our static batches find
their twin in the original's list (135 of 153 at `talon-fast`, 151 of 157 at the
grid, 182 of 188 at `talon-mid2`, 265 of 271 on the second boot; 97 of 152 at the
second fast dump, where the craft had drifted 5 units/s off the pose), so the
signature reads batches when they are there.

**Cause: a node-visibility difference, not depth.** `oag_render::pvs::visible`
returned `true` at the top for a moving draw, skipping the authored section mask as
well as the frustum. The mask keys on the draw's node, not its position, so it
needs no trustworthy bound. Applying it to moving draws (frustum still exempt):

| Pose | moving draws we submit, before / after | original's moving batches we still draw, before / after |
| --- | --- | --- |
| `talon-fast`, boot 1 (two dumps) | 214 / 81 | 60 / 60 and 76 / 76 |
| `talon-fast`, boot 2 | 214 / 81 | 16 / 16 |
| `talon-mid2` (at rest) | 214 / 81 | 78 / 78 |
| start grid (at rest) | 214 / 21 | 18 / 18 |
| `talon-mid` (at rest) | 214 / 24 | 0 / 0 |

Every moving batch whose signature the original's list contains survives the
rule (the dropped ones are "no signature match", which for a moving batch is
weaker than "not submitted"), and the sixteen roof batches go (16 roof draws before, 0 after, at all of the poses rendered). The
original's own frame omits what its sections hide, and the section mask is
authored data. **Confidence 75** when this was written (raised to 80 by the repeat on two more circuits, below): four poses on Talon's Junction, one circuit; the original's loader has not been read doing the
walk (`docs/formats/track.md`), and the signature is a match by shape and not by
address. A moving draw that passes the mask but is absent from the original's list
still exists (21 to 24 at the rest poses): the original frustum-culls by a real
bound, which we cannot. At `talon-mid` the original draws no moving batch at all,
so that pose confirms nothing for the rule.

**Repeated on Metropia and Tech De Ra (2026-10-01).** *Question and falsifier,
written before the capture:* with the mask on moving draws, does it hide any
moving batch the original submits at the same pose? It would come out the other
way if some moving draw our mask drops still has a twin (same vertex count and
diameter) in the original's frame that nothing else accounts for - *DROPS* above
zero. Seven poses at rest, same Venom time trial: Metropia `03_Track` forward
(spline rows 400, 2300, 3200) and Tech De Ra `04_Track` reversed (rows 600, 900,
1800, 2000), one boot, the craft placed with `psp-drive.py place --speed 0
--settle 12`, the GE list recorded by `psp-ge-dump.py dump`, the craft and camera
by `psp-trace.py --camera --ticks 3`. (The profile's track list had to be walked
to find them: Track Select is not in the order the dev-unlock table gives, and
the circuit that came up was identified from its start position and heading
against every `NN_Track` spline, as `docs/reverse-engineering/ppsspp-debugger.md`
describes; Tech De Ra came up reversed.) Ours is `crates/render/examples/
pvs_moving_census.rs`: every draw call with its authored mask, its rest-pose
vertex count, box and diameter, and whether it passes the running game's visible
set (`VisibleSet::around`, craft and camera sections taken off the nearest spline
rows) and the craft's own mask alone, which is what the original culls with. The
join is `scripts/pvs-moving-census.py`. Two things the first pass got wrong, kept
because they are traps: the original's PRIM `cnt` is a strip's vertex count, not
our index count (match on the vertex count the buffer holds), and **the original
submits many batches twice at one place with two textures** (an object's second
pass), so its PRIMs are folded to one per (vertices, box) before matching, or a
single object reads as two twins.

| Pose | Our moving draws (all) | Pass our set / the craft's mask alone | Of those we pass, no twin in the original | Moving draws we drop that have an unclaimed twin |
| --- | --- | --- | --- | --- |
| Metropia, row 400 | 40 | 20 / 18 | 4 | **0** |
| Metropia, row 2300 | 40 | 29 / 29 | 6 | **0** |
| Metropia, row 3200 | 40 | 32 / 32 | 6 | **0** |
| Tech De Ra, row 600 | 53 | 19 / 19 | 9 | **0** |
| Tech De Ra, row 900 | 53 | 19 / 19 | 11 | **0** |
| Tech De Ra, row 1800 | 53 | 27 / 27 | 12 | **0** |
| Tech De Ra, row 2000 | 53 | 36 / 36 | 5 | **0** |

**Result: nothing the original submits is hidden by our mask.** DROPS is `0` at
all seven poses, and again `0` at a second placement and dump of Tech De Ra rows
600, 900 and 2000 taken later on the same boot (the craft landed 0.9 to 16 units
off the first pose this time; passing counts `19`, `19` and `36` as before).
One boot only: no cold second boot was taken here. The control that the count can move: with the craft's section
forced wrong (`--force-sections`), the same pose reports `8` drops. Before the
fold of the second-texture passes the first pass showed two apparent drops on
Metropia, and a few more on Tech De Ra from small four-vertex quads; both were a
static draw's twin or the same object's second pass, and vanish once static draws
claim their twins first. The static control is weaker than the 88 to 98 % above,
because this side applies no frustum: static draws passing the craft's mask with
a twin were `68`, `62`, `78` % on Metropia and `86`, `68`, `76`, `91` % on Tech
De Ra.

**The other direction, a count rather than a finding.** Moving draws that pass
the mask and have no twin: 4 to 6 on Metropia and 5 to 12 on Tech De Ra. The 21 to
24 quoted above for Talon's Junction came from the earlier pass's own method (no
fold of the second-texture passes, a different count), so the two are not a
like-for-like comparison and no ranking of circuits follows. **Corrected
2026-10-01**: the Metropia figures were inflated by this census's own diameter
(a double sweep, `8 %` short of the exact diameter the dump side computes, outside
the `3 %` tolerance), and Metropia node 225 - the one that "recurred" - has twins
at every pose; with both sides exact Metropia reads 0 to 4 (rows 400, 2300, 3200:
2, 4, 4; none at row 400 by the craft's mask alone) and Tech De Ra is unchanged
(9, 11, 12, 5). Tech De Ra's nodes 1076 and 1078 were not among the residue.
What is left, and what explains it, is the next subsection.

**Confidence 80** for "the mask hides no moving batch the original submits":
three circuits, eleven poses, zero counterexamples, a positive control, no
loader reading, and moving batches matched on count and diameter alone, which is
weaker than the static match.

### The original culls a moving draw by its section's box (2026-10-01)

*Question and falsifier, written before the work:* does the original frustum-cull
moving meshes by a bound we lack? It would be wrong if a draw the original submits
sat in a section whose box the view rejects, and it would explain the residue only
if the rejected sections hold the draws the original does not submit.

**Found in the executable, then measured.** The shared node predicate
`Node_TestSectionVisible` (`0x0892b638`, a vtable method of twelve node classes)
tests the governing section's mask bit and then the section's own authored
world-space box against the view with a corner outcode (`x < -w`, `y < -w`,
`z < 0`, `x > w`, `y > w`; culled when every one of the eight corners is outside
the same plane; no far plane). The box is the `.vex` `section` node's own, which
`oag_vex::pvs` already reads. Evidence, the disassembly and the live reads are in
[section-view-cull.md](../ghidra/functions/psp-pulse-usa/section-view-cull.md).

Against the 22 dumps (view and projection are the dump's own registers,
`scripts/pvs-cull-check.py`): **0 of 2,004 draws in a culled section are in the
original's list** (static 1,936, moving 68), the same test with the view turned a
quarter circle finds 73 and 135 at two poses, and it explains 64 % of the static
and 47 % of the moving draws that pass the mask and have no twin. Moving draws
need no bound of their own for this: their section is static.

| | section culled, no twin | culled, **twin** | visible, no twin | visible, twin |
| --- | --- | --- | --- | --- |
| Metropia moving (8 poses) | 23 | **0** | 25 | 163 |
| Tech De Ra moving (14) | 45 | **0** | 52 | 141 |

**Ported** as `oag_render::pvs::sections_in_view` and `VisibleSet::within_view`,
wired in the race scene. Seven native 480x272 poses are pixel-identical before and
after (0 differing pixels each), with 0 to 10 fewer draws submitted: the original's
cull is conservative, so a correct port moves submissions and not pixels.
**Chosen, not measured**: the test uses this camera's view-projection where the
original hard-codes 480/272 into its planes.

**What this is not.** The original also runs the same corner test per `Mesh` node
against its own box (`Mesh_SubmitNode`, `0x0890cc80`), and that one does *not*
predict the GE list - 19 of 46 nodes it rejected at the start grid still had a PRIM
in the frame - because most static geometry also reaches the GE through merged
batch sets that never take it. It is not ported. **Still open**: a bound on the
batch-set path, if there is one (1,071 static draws pass the section test with no
twin), the small transparent quads the census cannot match, and a circuit with a
tunnel or a loop.

**Seen as a player.** The frames, not kept in the repository, show the
original, ours before and ours after at the fast pose. The dark slabs left of the
silo are gone and the grass, the silo and the trees read as the original's.

**Not done.** Static draws: 2 to 12 % have no twin in the original's list at each
well-posed dump (batches it splits differently, or culls by a bound we lack); that
is unexamined, not a finding. The end-to-end picture check is one
pose; the two rest poses show a few-pixel change before and after.

### Moving draws the original does not submit: frustum or rule (2026-10-02)

*Question and falsifier, written before the work (`pulse-moving-draws`):* of the
moving draws that pass the section mask and the section's box and still have no
twin in the original's list, is the cause the view (the draw's own box is outside
it) or something else? "The view" is falsified by a draw the original **submits**
although its own box is outside the view, and by a missing draw whose box is
inside it. **The brief's premise was already closed**: Metropia node `225` and Tech
De Ra `1076`/`1078` were a census artifact (the subsection above), so they are not
what was separated; the residue is the "visible, no twin" column of the table
above.

**Method.** `AnimTransform::sample` documents one global clock that every node
reads, so the dump's moving PRIMs pin the instant: `pvs_moving_phase` scores every
`T` on a 1/60 s grid (0 to 620 s, refined to 1/480) by how many observed boxes
coincide, to 0.7 units, with a moving draw of ours of the same vertex count and
diameter, and prints the world box of **every** moving draw at that `T`.
`scripts/pvs-moving-residue.py` then tests each box against the dump's own view and
projection (the game's corner outcode, as `pvs-cull-check.py`) and calls a draw
*present* when the dump holds a box of its vertex count within 0.7 units of it
(`--tol`; held apart from the clock's `--solve-tol`). The 22 existing Metropia and
Tech De Ra dumps were re-used, and the loop was captured fresh (below). **Chosen,
not measured**: the 0.7 and 1.5 unit tolerances, the box being the extremal
vertices along 13 axes (a lower bound on the exact box, a unit or two small, so it
is grown by 1.5 before the view test), and the mesh's own box being stood in for by
the union of its batches' vertices.

**The clock is identified on Metropia only.** Seven Metropia dumps solve to
`T` = 89.2 to 94.2 s with 17 to 28 boxes matched, 4 to 8 above the best
non-peak time (a constant-position baseline of 13 to 21 matches everything
else); the same `T` recurs 120 s and 600 s later, which is read here as the moving
nodes' loop lengths and **not** checked against their `loop_seconds`. Tech De Ra
dumps score 0 to 2 above that baseline and the Talon loop dumps 0 to 1 (its only
moving draws near the loop are two large rotating structures, nodes `223` and
`915`, whose boxes the solver cannot pin): their `T` is **not** known, so a
verdict that depends on a *missing* draw's position is not available there and is
not claimed. (On Talon this showed up as 22 draws "missing by box" that the
count-and-diameter join finds in the list: a wrong clock, not a missing draw.)

**Result, Metropia (7 dumps, one boot; 188 moving draws pass the craft's mask in a
section the view reaches - 156 opaque, 22 transparent, 10 cutout - the same 188
`pvs-cull-check.py` counts as 25 twinless and 163 with a twin).**

| List | missing, own box outside the view | missing, inside | submitted, outside | submitted, inside |
| --- | --- | --- | --- | --- |
| transparent | 11 | 0 | 6 | 5 |
| opaque | 2 | 0 | 115 | 39 |
| cutout | 0 | 0 | 8 | 2 |

1. **Missing by box: 13. By count and diameter as well: 10.** The old join and this
   one differ on 3 (at `--tol` 1.5 and 3, 1: the extremal box undershoots, and the
   one that stays is a rotated mesh); those 10 are the unambiguous residue, and all
   10 are outside the view. The two 'missing' opaque draws are one batch of node
   `154`.
2. **That is not evidence for the view, because most of the population is outside
   it.** 142 of the 188 (75 %) are outside the view at the dump's instant, so 13
   of 13 missing draws being outside is what chance gives about once in 40
   (Fisher, p about 0.02) - on draws that come from about eight nodes at seven
   poses and are not independent. Among the transparent ones, 11 of the 17 outside
   are missing and 0 of the 5 inside; the margins do not separate the groups
   either (the missing transparent draws are 108 to 3,620 clip units outside, the
   submitted ones 280 to 1,180).
3. **The own-box test is not a rule `pvs::visible` can take.** It rejects 129
   moving draws the original *does* submit (6 transparent, 115 opaque, 8 cutout),
   and for those the box tested is the dump's own PRIM box, so it needs no clock.
   The brief's acceptance - a count that drops without dropping a draw the
   original submits - cannot be met, so nothing was changed in
   `oag_render::pvs::visible`. For opaque and cutout moving draws (123 submitted
   outside the view, 2 missing) the own box is plainly not what the original culls
   by. For the transparent list the six are two objects (node `229`, and `117`,
   `119`, `121` at one pose), so that half is a small-sample reading and is scored
   below accordingly.
4. **The hold counter does not rescue it.** `Mesh_SubmitNode` keeps a mesh it saw
   in view drawing for `8 + rand() % 7` more frames (`section-view-cull.md`). Taking
   the node's box at each of the previous 14 frames against the *same* view, 2 of
   the 6 transparent submitted-and-outside draws turn out to have been inside it
   (4 of 6 remain). The camera moves across 14 frames and the original's `rand()`
   is unseeded, so this is a bound, not a refutation.
5. **Player-visible, Metropia: nothing.** Every one of its missing draws lies wholly
   outside the view, so the GPU clips it to no pixel either way; what the
   difference costs is the submission. That is scoped to Metropia: Tech De Ra's
   residue is unresolved for want of a clock.

Tech De Ra (14 dumps, clock unpinned) reads 36 missing transparent draws outside
the view and 8 inside, 4 submitted outside and 39 inside. The 8 are nodes `962`,
`966` and `288` at four poses; nodes `962`/`966` and `284`/`288` share a signature
(diameters 56.9 and 53.7), so which instance the dump's one box belongs to depends
on the clock, and with none they are **not** evidence of a rule. No opaque or
cutout moving draw is missing on Tech De Ra.

**A circuit with a loop: Talon's Junction `16_Track`, 20 dumps on two cold boots
(12 and 8), PPSSPP's software renderer (the GE list does not depend on it).** The
track's inverted stretch is rows 1263 to 1401 (`oag-trace track`: 139 samples with
the craft's down axis pointing up); Moa Therma and Arc Prime were not used because
Talon is the circuit the menu walk reaches with no track select. Placed with
`psp-drive.py place`, forwards and backwards along the spline; the tool said "looks
like a respawn" for 12 of the 25 placements (the game moved the craft), so each
dump is labelled by where the trace shows the craft: nearest spline row 1198 (twice),
1261, 1292, 1294, 1361, 1388, 1402, 1415, 1422, 1427, 1444 on the first boot and
1199 (twice), 1214, 1224, 1295, 1361, 1402, 1420 on the second, six of them inside
the inverted stretch. With the section mask and the count-and-diameter twin: **9
moving draws pass at every pose and 9 have a twin, 0 missing** on both boots (at
the five poses in section 24 the mask passes 23, and the 14 extra are all in a
section the view rejects, 0 of them in the list; `pvs-cull-check.py` finds no
falsifier at any of the 20). `DROPS` is 0 at 17 dumps and 1 at three, each a
four-vertex quad (node `258`, diameter 38.1 against 38.2, first boot near row 1261;
node `351`, 38.8 against 39.7, on **both** boots, near rows 1292 and 1295), matched
on count and diameter alone - the weakest signature the census has, and not
distinguishable from a coincidence. So the loop adds no moving-draw difference, on
either boot, and the frustum-or-rule question has no subject there.

**The collision-cage sphere (note, not a fix).** The yawed-pose sphere the thread
names, `pSphereShape1` with `cage_collision2_ADD.tga`, is the whole of **WAD entry
1078 of `DATA.WAD`**, `Data\visual_effects\cage_collision_curved.vex` (its source
path inside the file is `cage_collision_curved.mb`; 9,552 bytes). It is **not** the
track node `1078` this page keeps meeting - same number, different namespace. What
spawns it was read 2026-10-02 (`docs/ghidra/functions/psp-pulse-usa/collision-cage.md`):
`CageEffect_Construct` (`0x0883170c`, one caller in `RaceManager_Construct`, once per
race load) loads the vex and builds a ring of four scene objects parked at the identity;
`CageEffect_Spawn` (`0x08832b0c`) poses one at a contact point and advances the ring.
**Not recovered:** what reaches the spawn. The only path read is a per-craft query
(`0x08831aa4`) that spawns on a hit within 15 units; it returned "nothing" (127) on every
sampled tick on `03_Track`, and the spawn's entry never fired through a wall scrape or four
off-track teleports. So nothing is drawn: the pose, fade and trigger are unread
(`pulse-cage` lane, 2026-10-02). The roadmap's `Cage Collision` (`0x3e7`) has no node on the
PSP disc, and the `collisionCageEnabled` attribute (`0x088c40a8`, stored at definition
`+0x16f`) has no reader found; the constructor does not test it.

**Confidence.** 80 for "the Talon loop dumps hold every moving draw the mask
passes" (20 dumps, two cold boots, a count-and-diameter match). 70 for "a moving
opaque or cutout draw is not culled by its own box" (123 submitted outside the view,
the box being the dump's own). **No score** for "the transparent missing draws are
outside the view" or for any transparent rule: 13 draws from about eight nodes on
one circuit and one boot, against a base rate of 75 %. Scripts:
`scripts/pvs-moving-residue.py`, the `pvs_moving_phase` example; raw dumps
a scratch directory, not kept.

## Reproducing

```sh
# original: own HOME and XDG_CONFIG_HOME, own Xvfb, ppsspp.ini with
# InternalResolution=1 TextureFiltering=1 TexScalingLevel=1 AnisotropyLevel=0 iShowStatusFlags=0
uv run --with websocket-client scripts/psp-drive.py --port 45093 menu
uv run --with websocket-client scripts/psp-drive.py --port 45093 place --pos=X,Y,Z --tangent=X,Y,Z --up=X,Y,Z --speed 0
uv run --with websocket-client scripts/psp-trace.py --port 45093 --ticks 3 --camera --out t.csv --shot-every 1 --shot-dir shots
# the original's submitted batches at a pose (section 3)
uv run --with websocket-client --with zstandard scripts/psp-ge-dump.py dump --port 45093 --out frame.ppdmp
uv run --with zstandard --with numpy scripts/psp-ge-dump.py census frame.ppdmp --out frame.prims.json
# the moving-draw census, ours and the join (section 3, second table)
cargo build -q -p oag-render --example pvs_moving_census
python3 scripts/pvs-moving-census.py --spline spline.csv --pose-dir dir \\
    --entry 'Data\\Environments\\03_Track\\track.vex'
# which of the missing moving draws are outside the view (needs `--with numpy` on the census: without it
# `diam` is silently null and every join reads 0 matches)
cargo build -q -p oag-render --example pvs_moving_phase --example pvs_section_boxes
python3 scripts/pvs-moving-residue.py --spline spline.csv --pose-dir dir --prims dir/prims.json \\
    --entry 'Data\\Environments\\03_Track\\track.vex' --list
# ours
oag-game --race --no-audio --size 480x272 --render-scale 100 --msaa off --motion-blur off \
    --screen-filter off --anisotropy off --ticks 1 --pose-from t.csv --pose-tick 2 --screenshot ours.png
```

`psp-drive.py place` takes `--tangent=-0.9,...` with an equals sign: a value that
starts with `-` is otherwise read as a flag. Other circuits than Talon's Junction
need the dev-unlock byte (see the debugger page) written once `Main Menu` is up,
and `menu --track-down N --any-track`. Crops and montages from this pass are not kept (derived game data, not committed).
