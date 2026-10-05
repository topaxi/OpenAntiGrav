# Texture animation: the GU transform primitives

Where the engine can move a texture under fixed geometry, and what is known
about who does it.

The renderer's side of this - which surfaces animate and why - is in
[`docs/formats/vex.md`](../../../formats/vex.md), "Tracks animate too". This
page is the code half: the two GE state setters the effect must go through, and
a bounded statement of who calls them.

## The two primitives

| Address | Name | Conf | Signature |
| --- | --- | ---: | --- |
| `0x08811630` | `Gu_TexOffset` | 88 | `void (float u, float v)` |
| `0x08810ab4` | `Gu_TexScale` | 88 | `void (float u, float v)` |

Both are thin two-float wrappers in the same address neighbourhood as the
already-named `Gu_CallList` (`0x08810598`) and `Gu_DrawArray` (`0x08810e98`),
which is what a statically linked `libgu` looks like.

**Evidence.** `Trail_DrawRibbon` (`0x0892acc8`) advances a per-layer scroll and
immediately passes it to these two, in this order:

```c
*(float *)(iVar24 + 8)   += *(float *)(iVar24 + 0x10) * _DAT_08a889d0 * 3.0;
*(float *)(iVar24 + 0xc) += *(float *)(iVar24 + 0x14) * _DAT_08a889d0;
/* each wrapped into [0,1] */
FUN_08811630(*(undefined4 *)(iVar24 + 8),  *(undefined4 *)(iVar24 + 0xc));   /* offset */
FUN_08810ab4(*(undefined4 *)(iVar24 + 0x18), *(undefined4 *)(iVar24 + 0x1c)); /* scale  */
```

That block is already independently documented in
[`exhaust.md`](exhaust.md#trail): `+0x10`/`+0x14` are the authored `u`/`v`
scroll **rates**, `+0x08`/`+0x0c` the animated offsets, `+0x18` the `u` texture
scale, and `_DAT_08a889d0` is `0x3c888889` = `0.016666668`, one 60 Hz tick. So
the argument roles are fixed by a caller whose semantics were recovered
separately, which is why this scores 88 rather than being a guess from position.

88 and not higher because no import NID names them - `sceGu*` is statically
linked, not imported (see [imports.md](imports.md)) - and no runtime trace has
confirmed the GE commands they emit.

`DAT_08ab0628` gates the advance: when it is non-zero the offsets are submitted
but not stepped, which is the shape of a global pause flag. Not traced to what
sets it; **confidence 50**, recorded rather than named.

## Who calls them

`Gu_TexOffset` has **12** callers, `Gu_TexScale` **16**. Only one is
identified.

| Caller | Status |
| --- | --- |
| `Trail_DrawRibbon` `0x0892acc8` | Identified. The exhaust ribbon, documented in [`exhaust.md`](exhaust.md). |
| `FUN_089271cc` | A two-line helper: applies scale from `+0x18`/`+0x1c` and offset from `+0x20`/`+0x24` of a struct. Called only from `FUN_089307b4`. **Its gate is now read - see below.** |
| `FUN_089307b4` | Large; also calls both primitives directly. Reached from `FUN_0892f35c` (4 sites) and `FUN_0893021c`. |
| 9 others | Not examined. |

`FUN_0893021c` calls both `FUN_089307b4` and the VRAM texture upload
`FUN_08928550`, which is the shape of a bind-texture-plus-material-state path.
That makes `FUN_089307b4` the most likely home of a general per-material texture
transform, and `+0x20`/`+0x24` of its struct the most likely place a track
surface's UV offset would be written.

**This is a hypothesis, not a finding.** Nothing traces a track material to that
struct, and no name is proposed for either function - **confidence 45**, below
the [rubric](../../../reverse-engineering/confidence-rubric.md)'s rename
threshold. Written down instead, per that rubric's rule.

### The gate is a per-material bit, read 2026-08-08

The "most likely home" reading above is right about the shape and can be
tightened: `FUN_089307b4` calls the pair at three sites, each guarded by the
same test on the **runtime material** (`*(mesh+0x5c) + material_index * 0x14`,
the stride-`0x14` repacked array), and the branch picks between the immediate
form and a prebuilt list:

```c
if ((*(u16 *)(*(int *)(mesh + 0x5c) + material_index * 0x14) & 0x10) != 0) {
    if (mode == 1) Gu_CallList(*(int *)(mesh + 0x64) + material_index * 0x18);
    else if (mode == 0) FUN_089271cc(*(int *)(mesh + 0x60) + material_index * 0x40);
}
```

So `& 0x10` on the material's first `u16` is **the per-material "this surface
has a texture transform" bit**, and `mesh+0x60 + index * 0x40` is the block
whose `+0x18`/`+0x1c` and `+0x20`/`+0x24` `FUN_089271cc` submits.
Corroborated from the data side: `Data\Ships\<Team>\shipboost.vex`'s single
material carries flags `0x212`, which has the bit, and that model's authored
`u` spans only `[0.000, 0.008]` - one step of an 8-bit texcoord on its 64x16
texture - which is geometry that *needs* a scale to make sense of. Confidence
**75** for the gate (the test is read at instruction level at three sites and
the one model checked agrees); still **0** for where the block's values come
from, since the on-disc material's `+0x0c..0x14` is zero on that ship exactly
as it is on every track.

This does **not** contradict the live negative below. That measured
`Gu_TexOffset`, and found only the ribbon submitting a non-zero one;
`Gu_TexScale` was never sampled, and a scale with a zero offset would have
looked identical to it.

Still no rename: the gate is read but the data path into the block is not, and
`FUN_089307b4` is a large function whose name would have to cover much more
than this.

**The writer chain is now read too (2026-08-09), and it does not close the
values gap.** `FUN_0890e160(mesh)` fires when `mesh+0x40` (a time) differs from
`mesh+0x18c`, walks the materials, and for each one carrying `& 0x10` calls
`FUN_08927204(time, block)` then `FUN_08927358(slot, block)`. `FUN_08927358`
builds the five-word list - `0x48` `TEXSCALEU`, `0x49` `TEXSCALEV`, `0x4a`
`TEXOFFSETU`, `0x4b` `TEXOFFSETV`, `RET` - from block `+0x18`/`+0x1c`/`+0x20`/
`+0x24`, the same fields `FUN_089271cc` submits immediately. `FUN_08927204` is a
curve evaluator, and **its failure path is the useful part: no scale track
writes `1.0`/`1.0`, no offset track writes `0`/`0`.** So the transform defaults
to identity, which means the live negative below - only the ribbon scrolls - is
consistent with every track material simply having no track data, rather than
with the mechanism being absent. Confidence **85** on the chain and the
defaults; **still 0** on where any non-identity values would come from, exactly
where this page already stood. See
[mesh-draw.md](mesh-draw.md) for why this mattered: it was the leading candidate
for the boost plume's missing rim multiplier, and defaulting to identity is what
argues against it.

## The values gap is closed: the keyframes are authored in the `.vex` file

**2026-08-10, confidence 90.** The "still 0 on where any non-identity values
would come from" above is resolved, live on PPSSPP (Talon's Junction TIME
TRIAL, boost held up by rewriting `flare+0xb8`) and corroborated byte-for-byte
on the disc. Two functions in the chain are now read to the instruction and
named:

| Address | Name | Conf | Signature |
| --- | --- | ---: | --- |
| `0x08927034` | `TexAnim_EvalKeyframes` | 90 | `int (float out[2], int count, u16 *times, s16 *values, int t_int, int step, float t_frac...)` - args in `a0`-`a3`, `t0`, `t1`, `f13` |
| `0x08927204` | `TexAnim_UpdateTransform` | 90 | `void (anim_block *a0, float seconds in f12)` |

`TexAnim_EvalKeyframes` is the "curve evaluator" the writer-chain paragraph
above left unnamed. Read whole: keys are `u16` times paired with `s16`
`(u, v)` values in **1/256 units** (it loads `0x3b800000` = 1/256 and scales
both outputs); below `times[0]` it clamps to the first key, past
`times[count-1]` to the last; between keys it lerps by
`(t - t_prev) / (t_next - t_prev)` unless the step flag is set, in which case
it snaps to the key. Returns 0 only for an empty track, which is what makes
the identity default above fire.

`TexAnim_UpdateTransform` (`FUN_08927204` above) drives it, twice per block -
scale track then offset track, exactly the two calls at `0x089272cc` and
`0x08927308`: it stores the absolute time (seconds) at `block+0x28`, wraps it
with `fmodf(t, *(float *)(block+0x2c))`, divides by `block+0x0c`
(`0.016667` = seconds-per-frame, so **key times are in 60 Hz frames**), and
truncates to the integer key time. The low bit of the *word* at `+0x2c` is
the step flag - the same word whose float value is the wrap period.

### The block layout, and where it lives on disc

The runtime block (`mesh+0x60 + material_index*0x40`, the one
`FUN_089271cc` submits from) is the **file's own bytes, relocated in place**.
On disc it sits immediately after the material array, at mesh payload
`+0x30 + material_count*0x14`:

```text
+0x00  u16   offset-track key count
+0x02  u16   scale-track key count
+0x04  u32   offset-track times,  relative to this block
+0x08  u32   scale-track times,   relative to this block
+0x0c  f32   seconds per key-time unit (1/60 on every block read)
+0x10  u32   offset-track values, relative to this block
+0x14  u32   scale-track values,  relative to this block
+0x18  f32[2] out: scale u, v     (initialised 1.0 in the file)
+0x20  f32[2] out: offset u, v
+0x28  f32   last update time, written at runtime
+0x2c  f32   authored loop period, seconds; bit 0 of the word = step flag
```

**`+0x2c` is per-block authored data, not a constant** - an earlier revision
said "600.0 everywhere read", which was true of the two plume blocks it had
read and wrong as a generalisation. Surveyed across all eight `Ship.vex`
files and the `01_Track`/`16_Track` circuits: blink-light blocks author
**1.0 s**, scenery authors 0.667, 0.833, 2, 4, 5 and 10 s - in every case
the loop ≈ its track's own span - and the plume's 600 s is the outlier
*because* its clock is externally reset per reveal rather than wrapped.

Verified two ways on `Data\Ships\Assegai\shipboost.vex` (and Feisar's, byte
identical): the struct parses at `payload+0x30+0x14` on **both** plume meshes
with counts `(2, 1)`, relative offsets landing on the key data, `1/60` and
`600.0` in place; and the live block at `mesh_header+0x44` in PSP RAM is the
same bytes with the relative offsets replaced by absolute pointers.

### The boost plume's authored tracks, and the recovered law

```text
scale  track: times (90),    values (256, 256)            = constant (1.0, 1.0)
offset track: times (1, 90), values (2, 0), (253, 0)      = u ramps 2/256 -> 253/256
                                                            over frames 1..90, v = 0
```

Sampled per frame over 360 consecutive frames (breakpoint on
`Ship_UpdateCraft`, reading the compiled five-word transform list): both plume
meshes carry the **same** `u` at every sample, `v` is always exactly 0, the
scale is always `(1.0, 1.0)`, and `u` ramps and wraps with period
**amplitude/slope = 0.9805/0.010911 ≈ 89.9 frames ≈ 1.5 s** - the track's own
span.

**The clock is settled (same day, follow-up session): the time argument is
the flare's own life timer at `flare+0x88`, which resets to 0 at each plume
reveal.** Measured at `TexAnim_UpdateTransform`'s entry with a conditional
breakpoint on the plume's two blocks: `f12 == *(float *)(flare+0x88)` on
every one of 90 hits across a single boost, `mesh+0x40` is written to the
same value each frame, and the updater does not fire at all while the plume
is hidden (0 hits over 3 idle seconds). `flare+0x88` was watched resetting
`62.72 -> 0.000` at the reveal on two isolated boosts. So the plume's
animation **plays the 90-frame track exactly once per reveal** - clamped at
the bright first key at t < 1 frame, darkening down the ramp, reaching the
last key exactly as the plume's own 1.5 s life
(`oag_fx::exhaust::PLUME_SECONDS`, `flare+0x88`'s hide threshold at
preset 2) expires. Track span and plume life are both 1.5 s by authoring.
The "looping" in the paragraph above is the held-boost artifact: pinning
`boost_timer` at 0.8 re-reveals the plume the moment its 1.5 s expires,
which resets the clock every 90 frames and reads as a sawtooth. A
free-running or race-clock-driven scroll is therefore wrong in two visible
ways - a boost can begin mid-ramp already faded, and one outliving the wrap
re-brightens as a second pulse.

The time argument for **world meshes** is different: the same one-frame
census saw `521.68` (the race clock) for eleven of them and `4.80` for
another, all through the same call site (`0x0890e1e4`). Each model appears
to be updated with its own clock; only the plume's is pinned to a source.

**The double-pad case is measured too, frame-exact, and a second pad never
resets or overlaps the running sweep** - worth pinning because "surely a
second pad should restart the animation" is the natural expectation and the
original does not do it. Two simulated pads (`0.8` written to `flare+0xb8`
at frame-counted spacings, every frame sampled):

- **30 frames (0.5 s) apart**: across the second hit, `flare+0x88` runs
  `0.467 -> 0.484 -> 0.501` and `u` runs `0.306 -> 0.317 -> 0.328` -
  perfectly continuous; only the boost timer jumps (`0.316 -> 0.800`). The
  sweep completes at ~frame 90 and no second sweep plays, because the
  second pad's timer has already emptied (decay 1/s from 0.8) by then.
  **Two quick pads share one animation.**
- **72 frames (1.2 s) apart**: same non-reset at the hit, but at the
  sweep's 1.5 s expiry the timer still reads `0.466` - above the reveal
  gate - so the plume re-reveals at once: `flare+0x88` snaps to 0, `u`
  restarts at `2/256`, and a second full sweep plays back-to-back.

So the reveal rule composes into a ~0.9 s threshold: the timer holds above
the gate for `(0.8 - 0.2) / 1.0 = 0.6 s` after a pad, so a second pad
replays the sweep only when it lands inside the sweep's final 0.6 s - i.e.
later than `1.5 - 0.6 = 0.9 s` after the reveal. The boundary was swept
live at 6-frame resolution: timer `0.183` at expiry does not replay,
`0.283` does, bracketing the `0.2` gate.

**And the simulated write is a proven proxy for a real pad, so these
results are the real-pad behaviour.** Two legs, same day: (1)
`Ship_ApplySpeedupPad` (`0x08848f9c`) is now read whole - on a new pad
identity (`craft+0x1d0` edge, so two *different* pads both fire even
back-to-back) it zeroes `craft+0x318`, calls `ExhaustFlare_OnSpeedupPad`
(**its only exhaust interaction**), bumps the pad-hit stats, and arms the
physics shove from the class tables; nothing else in the path touches the
flare or the boost model. (2) One *real* pad crossing was captured during
a breakpoint-paced autopilot lap (pad 4, Talon's Junction): plume idle,
`flare+0x88` 8.54 -> 0.017 at the hit, `u` restarting at `2/256` - the
same reveal the writes produce. What a mid-sweep pad
*does* change is every
timer-driven effect: the flare's half-size term, the engine note, the
physics boost. `oag_fx::exhaust`'s edge-triggered reveal plus
next-tick re-reveal reproduces all three regimes.

### Two earlier readings this corrects

- **"Every track material simply having no track data" (the writer-chain
  paragraph above) is wrong.** One frame of evaluator hits on Talon's
  Junction shows scenery blocks with real authored tracks:
  `(1, 240) -> (0,0), (3072,0)` (12 `u` tiles per 4 s), `(0, 60) -> (0,0),
  (0,256)` (one `v` tile per second), and a seven-key `v` plateau track
  `(0, 20, 59, 60, 130, 175, 200)` - authored texture-transform animation is
  on the disc for track scenery too, not just the plume.
- **The live negative below ("no track surface receives a texture-coordinate
  offset") measured the wrong choke point.** The compiled-list path writes
  the `0x48`-`0x4b` words straight into each material's five-word list
  (`FUN_08927358`), never calling `Gu_TexOffset` - so a `Gu_TexOffset`
  breakpoint cannot see it, and the negative's conclusion does not follow
  from its measurement. What the negative still shows is that the
  *immediate-mode* path (`FUN_089271cc`) is not the one running during a
  race.

## The reveal chain, read statically, and four names recovered

**2026-08-10, follow-up Ghidra session** (live `psp-pulse-usa/BOOT.BIN`
project). The whole reveal-and-animate chain is now read at decompiler
level, and it confirms every live measurement above instruction by
instruction:

`Exhaust_Update` (`0x089058b0`, already named) carries the reveal verbatim -
both `0.2` gates are literals in its body:

```c
flare->0x88 += dt;                                       // free-running life
engine_on = thrust > 0 || flare->0xb8 > 0.2;             // gate literal 1
if (engine_on && 0.2 < flare->0xb8                        // gate literal 2
    && (boost_model->0x2c & 4) == 0) {                    // hidden -> visible
    boost_model->0x2c |= 4;                               // show
    Node_SetAnimTimeTree(0.0, boost_model);               // zero the anim clock
    flare->0x88 = 0;                                      // zero the life
}
if (1.5 <= flare->0x88) boost_model->0x2c &= ~4;          // hide
```

So the updater's time argument is the **model's own animation clock**, not
`flare+0x88` read directly - the two are zeroed in the same statement and
advance together while the model updates, which is why the live trace saw
them equal on every hit, and why `u` freezes at hide while `flare+0x88`
runs on. One refinement to the section above, no behavioural change.

| Address | Name | Conf | What was read |
| --- | --- | ---: | --- |
| `0x0890e160` | `Mesh_UpdateTextureTransforms` | 90 | Decompiles whole: fires when `mesh+0x40 != mesh+0x18c`, walks the materials (`mesh+0x5c`, stride `0x14`), and for each `& 0x10` calls `TexAnim_UpdateTransform(time, mesh+0x60 + i*0x40)` then `TexAnim_CompileTransformList(mesh+0x64 + i*0x18, block)`, caching the time at `+0x18c`. Live-corroborated: the updater breakpoint's return address `0x0890e1e4` sits inside it and its time argument was sampled. |
| `0x08927358` | `TexAnim_CompileTransformList` | 90 | Decompiles whole: builds exactly the five words read live from the plume's list - `float_bits >> 8 \| 0x48/0x49/0x4a/0x4b << 24` from block `+0x18..+0x24`, then `RET`, then `sceKernelDcacheWritebackRange` - direct memory writes, no `Gu_TexScale`/`Gu_TexOffset` call, which is the instruction-level proof of the choke-point correction above. |
| `0x089114fc` | `Node_SetAnimTimeTree` | 75 | Recursive node-tree walk: for nodes of three classes (Mesh's class-identity `0x08a6bb84` plus two unidentified), dispatches vtable slot `+0x84` with the payload offset short at `+0x80`, then recurses into children. Named from the one dispatch target read (below) and the reveal call site passing `0.0`; the two other classes' methods are unread. Thin wrapper `FUN_08912890` (the reveal's call) tail-calls it, left unnamed. |
| `0x0890e240` | `Mesh_SetAnimTime` | 78 | The Mesh class's `+0x84` method, found through the vtable at `0x08ad1994` (init slot `+0x7c` = `0x0891ff50`, the `Mesh_InitFromPayload` trampoline, which pins the base). First instruction `swc1 f12, 0x40(a0)` - stores the time argument to `mesh+0x40`, the exact field `Mesh_UpdateTextureTransforms` gates on. A tail also copies a global reference mesh's `+0x40` (from `DAT_08ab0818`, else `DAT_08b317b0`) into `mesh+0x194`; that purpose is unread, which is what holds this below 85. | **See the correction below: the first branch is `Anim Transform`, not Mesh.**

### Correction, 2026-08-18: which three classes the walker dispatches

The row above names `0x08a6bb84` as "Mesh's class-identity". **It is `Anim
Transform`'s.** Mesh's is `0x08a6bd48`, the walker's *second* branch. Each is
pinned by its own registration function storing the tag into the class
descriptor's `+0x04` - `AnimTransform_Register` (`0x0890009c`) and
`Mesh_Register` (`0x089100a0`), the only `li a1, 0x3c0` and `li a1, 0x125` in
the image. The third branch, `0x08a6bd18`, is still unidentified.

Nothing on this page's *behaviour* changes: all three branches dispatch the same
vtable slot `+0x84`, and `Mesh_SetAnimTime` is still the method Mesh's branch
reaches. What changes is why the walker has three branches at all - it feeds
this page's texture-transform clock and the scenery-animation clock through one
tree walk. See [`anim-transform.md`](anim-transform.md).

**`DAT_08b317b0`, above, is confirmed rather than merely carried forward from
an earlier import.** A later pass on `anim-transform.md` could not reproduce it
from this binary's `+0x08804000` image base and flagged it as an open "second
relocation base" - resolved there: this ELF's two `PT_LOAD` segments each get
their own base once loaded, and `.bss` (where this global lives) is segment 1's
`+0x08ad9798`, not segment 0's `+0x08804000`. See
[`anim-transform.md`](anim-transform.md#the-second-relocation-base-is-found-it-is-per-segment-not-per-image)
for the arithmetic and the relocation-record corroboration.

## Measured in a live race: only the trail scrolls

**Scope corrected 2026-08-10: this negative covers the immediate-mode
`Gu_TexOffset` path only.** The compiled-list path animates surfaces without
ever calling `Gu_TexOffset` - see "Two earlier readings this corrects" above -
so the section's final sentence is withdrawn; the measurement itself stands.

**Confidence 88, and it is a negative.** A breakpoint on `Gu_TexOffset` through
a running Time Trial on Talon's Junction, 60 hits:

| Return address | Hits | Offsets passed |
| --- | ---: | --- |
| `0x0892b088` in `Trail_DrawRibbon` | 9 | **9 distinct**, advancing every frame - `u` 0.100 to 0.800, `v` 0.067 to 1.933 |
| `0x088a7b74` | 26 | `(0, 0)` every time |
| `0x0891e8ec` | 25 | `(0, 0)` every time |

So the **only** non-zero texture offset the engine submits during a race is the
exhaust ribbon's, whose scroll rates were already recovered independently
([`exhaust.md`](exhaust.md#trail)). The other two sites reset the offset and are
called about three times a frame each, which is pipeline setup rather than
per-batch state. **No track surface receives a texture-coordinate offset.**

The read is on-target rather than incidental: Talon's Junction is `16_Track`,
which carries two of the surfaces
`oag_pulse::textures::ANIMATED_TEXTURES` lists (`col_display7_GLOW` and
`col_display7_BLEND_GLOW`), and `Gfx_BindTexture` was hit 80 times over the same
window with many distinct texture objects, so the track was plainly drawing.

**A CLUT scroll is not the explanation either.** Five palette regions reached
through bound texture objects were byte-identical over a 3.5-second window with
the CPU running. That does not cover every palette on the disc, so it is a
bounded negative - **confidence 70** - but it removes the obvious second
candidate.

### What this does and does not overturn

It does **not** touch the measurement that a ship's shoulder lights pulse: that
came from 120 frame-accurate screenshots with pixels sampled at a real light,
and it stands. What it contradicts is the *mechanism* inferred from it -
[`vex.md`](../../../formats/vex.md) reads that pulse as the engine scrolling the
shared texture's V globally, and no such offset is ever submitted, for the ship
or for anything else. The pulse is real and its cause is **not** a texture-
coordinate offset.

The untested candidate that fits every observation is an animated **colour**
rather than an animated coordinate: `Trail_DrawRibbon` already sets a per-draw
colour through `FUN_0881125c`, and a per-object colour advanced on a clock would
pulse a light without moving a UV or rewriting a palette. Not investigated.

**Consequence for the renderer** *(the setting named here was removed on 2026-08-18; see [`scenery-animation.md`](../../../rendering/scenery-animation.md))*: `[graphics] animated_textures` defaulted
**off** while the eight track surfaces were selected on geometry alone - narrow
authored V bands against full-tile static art - because making them scroll on
chosen rates was a departure from the original rather than a reproduction of
it.

***(2026-08-11: done, and the default is now on.)*** The renderer reads the
authored blocks: `oag_vex::vex::mesh_tex_transforms` parses the
per-material array, `TexTransform::sample` reproduces
`TexAnim_UpdateTransform`'s wrap-divide-clamp-lerp, and
`oag_mesh::mesh_render::TexAnims` samples every track of a model once a frame
into a uniform table that `mesh.wgsl` indexes per vertex. The name table stays
as the record of the geometric survey and no longer draws anything - the two
disagree, and the table was wrong: `col_display7_GLOW` is authored as a **U**
scroll on all twelve circuits and the table scrolled it in V.
`crates/render/tests/authored_uv_ground_truth.rs` is the evidence.

## What was ruled out

**`Gfx_BindTexture` (`0x08928460`) does not animate anything.** It compares the
texture's `+0xb0` against a global at `g_display + 0x5df4` and re-runs
`FUN_08928550` when they differ by more than `0.2`, which reads like a global
animation phase against a per-texture copy of it. It is not: `FUN_08928550`
walks the mip chain into VRAM (`FUN_089287c0` per level), sets a
resident flag, and *then* stores the global into `+0xb0`. So `+0xb0` is a
last-uploaded timestamp and the `0.2` is a texture-cache re-upload heuristic.
Recorded because the shape is genuinely misleading and cost a read.

## The plume does not scroll, and the `& 0x10` bit does not mean it does

2026-08-10, chasing a report that the boost plume's wing spikes read as "thick
fog animating towards the camera". The chain was followed end to end and the
answer is a **negative**, recorded because the bit makes it look like a yes.

`FUN_08927204` is now read: it stores the time at block `+0x28`, wraps it by
the period at `+0x2c`, divides by the duration at `+0x0c`, splits off the cycle
count, and calls the keyframe sampler `FUN_08927034` **twice** - once writing
the scale pair at `+0x18`/`+0x1c`, once the offset pair at `+0x20`/`+0x24`.
Each call's return value is a found/not-found flag, and **the not-found path
writes the identity**: `1.0`/`1.0` for the scale, `0`/`0` for the offset. Its
driver `FUN_0890e160` walks the materials whenever `mesh+0x40` differs from
`mesh+0x18c` and re-evaluates every material carrying `& 0x10`.

So the mechanism is a genuine per-material keyframed UV transform. **What is
missing is any authored track to feed it.** Measured through
`oag_vex::vex` on the European disc, over the on-disc material record's
`+0x0c..0x14`:

| File | materials | with `& 0x10` | with a non-zero track |
| --- | ---: | ---: | ---: |
| `Assegai\shipboost.vex` | 2 | **2** | **0** |
| `Assegai\Ship.vex` | 12 | 1 | **0** |
| `16_Track\track.vex` | 1,717 | 104 | **0** |

**Both of the plume's materials set the bit and neither carries a track**, so
the transform it replays every frame is the identity, every frame. And the bit
is not rare - 104 track materials have it - so `& 0x10` marks "this surface
*may* carry a transform", not "this surface animates".

That leaves the plume's apparent motion unexplained by this page's mechanism.
The remaining candidate is the third row of
[`methodology.md`](../../../reverse-engineering/methodology.md)'s table -
**environment-mapped uvgen, which slides the coordinates as the model turns and
needs no clock at all** - and that one is already implemented
(`oag_render::texgen`). A boost changes the craft's attitude continuously, so
its fins' generated coordinates really do flow without anything advancing a
timer.

## The heuristic this page is the evidence for

[`methodology.md`](../../../reverse-engineering/methodology.md) now carries
"when something animates or glows, look at the texture-coordinate path first -
but confirm before concluding" as a standing rule, with this page's live
negative as the counterexample that keeps it a *hint*. Both halves matter: the
mechanisms here are real and repeatedly the answer, and the one time the
project asserted a scroll from a screenshot it was wrong.

Worth adding for a future reader, from the boost-plume pass on 2026-08-10:
`FUN_0892733c` is a bare thunk to `Gu_CallList` and is the **second** replay
site for the per-material transform list, on `Mesh_CompileGeometryPass`'s draw
path rather than `FUN_089307b4`'s. So a surface can receive an animated
`TEXOFFSET` through either path, and a search that finds only `FUN_089271cc`
has seen half the mechanism.

## Open

- **The global V-scroll clock has not been found.** `vex.md` establishes the
  mechanism and measures its period (~30 ticks per authored cycle) from a
  capture, but no code has been read that advances it. It must be reachable from
  `Game_UpdateFrame` (`0x08804978`) - see [main-loop.md](main-loop.md) - or from
  a texture-manager tick under it. Finding it would pin the rate this project
  currently reuses as a guess across every animated track surface.
- **The nine unexamined `Gu_TexOffset` callers.** A caller inside track drawing
  would move the whole trackside-animation reading from inferred (65) to
  evidenced. (Less urgent as of 2026-08-10: trackside animation is now known
  to flow through the compiled-list path, which does not call `Gu_TexOffset`.)
- **What sets `DAT_08ab0628`.**
- **The per-model animation clock, for models other than the plume.** The
  plume's is settled: `flare+0x88`, reset at reveal (see "The values gap is
  closed"). World meshes receive the race clock, and at least one object a
  different local time (`4.80` observed); the general rule for which clock a
  model gets was not traced.
- ~~**Whether the blink lights are a colour or a keyframe animation after
  all.**~~ **Settled same day, from the files: they are keyframe tracks.**
  Every one of the eight teams' `colours_flashing_GLOW` meshes
  (`glowingShape`, Feisar's `underbrake_flash*`/`self_illuminatedShape`,
  material flags `0x91` - the `& 0x10` gate set) authors the identical
  block: offset `v` from `0` to `-256` (one full tile) over key times 1..60,
  scale constant `(1.0, 1.0)`, loop period **1.0 s** at `+0x2c`. So the
  original's blink is the compiled-list texture transform - the "engine
  scrolls the texture's V" reading vex.md retired in 2026-07-31 was right
  about the mechanism, and the live negative that killed it was blind to
  the list path (see the scope correction below). The "animated per-draw
  colour" candidate is dead. The pulse rate this project measured from
  pixels (~30 ticks per authored half-cycle, i.e. one V sweep per second)
  **equals the authored rate**, so `oag_pulse::textures::BLINK_V_CYCLES`
  is numerically right and can eventually be replaced by reading the block
  itself - noting the authored scroll direction is **negative** `v`.
  Confidence 85: the tracks, gate bits and rate agreement are file reads
  corroborated by the pixel measurement; no breakpoint has watched a ship
  glow mesh's updater specifically.
- ~~**Track scenery's tracks are authored too, with per-mesh phase.**~~
  **Read, and the renderer port is closed (2026-08-11).** `16_Track` carries
  90 animated meshes authoring **17 distinct tracks**: continuous `u`, `v` and
  diagonal scrolls at 0.667-10 s loops, and **stepped** tracks (`v` dropping
  `-64` at key pairs `(3,4)`, `(7,8)`, `(11,12)`, with sibling families at
  `(5,6)...` and `(11,12)...` carrying the same steps phase-shifted) over one
  shared **50-frame** loop. Two things this settles that the earlier summary
  got wrong: the loop is the block's `+0x2c` and **not** the last key time -
  those three families end at frames 12, 18 and 24 - and the blocks are **per
  material**, with a later block's key offsets relative to the array base
  rather than to itself.

  The surface that prompted the port: **Talon's Junction's turn arrows**
  (`col_arrows1_GLOW_ADD`, three meshes, material flags `0x0292`) author `v`
  from 0 to one whole tile over 40 frames on a 40-frame loop. They were drawn
  frozen until this landed.
