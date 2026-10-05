# Trackside animation: the two mechanisms, and what each draws

What a Pulse circuit authors as moving scenery, measured over all twelve
circuits on 2026-08-18. Two independent mechanisms: one slides a texture
*coordinate* under fixed geometry, the other moves the geometry. Both are read
and both are played.

Read [`texture-animation.md`](../ghidra/functions/psp-pulse-usa/texture-animation.md)
first for the mechanism this page counts, and
[`vex.md`](../formats/vex.md)'s "The texture-transform keyframe block" for the
on-disc layout. The measurements below are reproduced by
`crates/render/tests/scenery_animation_ground_truth.rs`, which is `#[ignore]`d
and needs a disc image.

## Summary

| | State |
| --- | --- |
| Per-material texture transform (`TEXSCALE`/`TEXOFFSET` keyframes) | **Complete.** 922 of 922 gated materials reach the shader on all twelve circuits |
| `Anim Transform` `0x3c0` - keyframed node motion | **Read and played, 2026-08-18.** 393 nodes, 474 meshes below them. It was the gap, and it cost placement as well as motion: 245 of those meshes used to draw at the world origin. Eight still do, because their own keys put them there |
| `animationTrigger` `0x3dc` - what starts an animation | Authored zero times on the **44 `.vex` files checked** - twelve `track.vex`, twelve `start_grid.vex`, twenty ship files. Not a survey of the disc's ~340 |

So the arrows at the curves, the billboards and the team banners animate
through the texture path, and the scenery that **moves** rather than scrolls now
does too. Both halves are drawn from the disc's own authored data.

The second half was the gap this page was written to record, and the sentence
that stood here - "what is missing is the class that makes scenery move" - is
answered rather than merely amended: the class is read at instruction level in
[`anim-transform.md`](../ghidra/functions/psp-pulse-usa/anim-transform.md) and
replayed by `oag_vex::vex::anim_transform` and
`oag_mesh::mesh::AnimNode`.

**One recommendation this page used to make is withdrawn.** It said to recover
the rest pose first and leave the animation for later. There is no separate rest
pose: `AnimTransform_EvalTranslation` computes `base + keys[t] * quantum` at
every time including zero, so placing the node and animating it are the same
decode. Splitting them would have bought nothing and risked a pose wrong by
`values[0] * quantum`.

## There is no setting for this, and there used to be

`[graphics] animated_textures` is **removed**. It was a boolean from when the
animation was a guess at which surfaces scroll and how fast; once both
mechanisms replay the disc's own keyframes it could only switch the
reproduction off. Worse, after the `Anim Transform` port it silently covered
trackside objects *moving* as well as surfaces scrolling, which its name did
not say - and because settings are **persisted**, a file written before the
default flipped on 2026-08-11 kept `false` and left circuits static long after
the default changed. That cost a debugging session on 2026-08-18, which is what
retired it.

What replaces it is `--anim-seconds`, on `oag-game` and already on `oag-view`:
it *pins* the animation clock rather than stopping it. A comparison against a
still of the original wants our phase matched to theirs, and two runs at two
values are the headless way to show that a surface moves at all. A harness knob,
on the command line, where a stale value cannot outlive the run.

## The texture-transform path is complete, and this is what "complete" means

Every material carrying the engine's own `& 0x10` gate was followed from the
file to a drawn batch, on each circuit:

| Circuit | Meshes | Materials | Gated `& 0x10` | Parsed to a track | On a drawn batch | Distinct tracks |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `01` | 594 | 1,364 | 27 | 27 | 27 | 7 |
| `02` | 664 | 1,291 | 62 | 62 | 62 | 11 |
| `03` | 473 | 1,067 | 119 | 119 | 119 | 15 |
| `04` | 684 | 1,481 | 125 | 125 | 125 | 18 |
| `05` | 546 | 1,269 | 96 | 96 | 96 | 13 |
| `06` | 536 | 1,527 | 54 | 54 | 54 | 14 |
| `07` | 686 | 1,524 | 120 | 120 | 120 | 12 |
| `09` | 411 | 1,208 | 61 | 61 | 61 | 10 |
| `10` | 490 | 1,198 | 57 | 57 | 57 | 11 |
| `13` | 581 | 1,518 | 37 | 37 | 37 | 9 |
| `14` | 523 | 981 | 60 | 60 | 60 | 13 |
| `16` | 602 | 1,717 | 104 | 104 | 104 | 18 |
| **Total** | **6,790** | **16,145** | **922** | **922** | **922** | |

Nothing is dropped anywhere in the chain: no gated material fails to parse, no
parsed block belongs to a material no batch names, and the worst circuit
authors 18 distinct tracks against the 63 `oag_mesh::mesh::ANIM_TRACK_LIMIT`
allows. **The ceiling is not the constraint it looks like** - `mesh::build`
deduplicates tracks by value, and a circuit's 104 animated materials collapse
to 18 curves.

The GPU half was checked too, because the existing
`authored_uv_ground_truth.rs` stops at `TexAnims::sample`: every pipeline in
`mesh_render::build` - opaque, alpha-test, and all three transparent classes
plus the authored ones - is created from the **same** `pipeline_layout` and the
same vertex layout including attribute 5 (`GpuVertex::anim`), so bind group 3
survives `set_pipeline` and an additive surface animates like any other. That
matters because the loudest example, `col_arrows1_GLOW_ADD`, is additive.
`race::scene::frame` writes the table for the track, sky, collision overlay and
both pad models; the only drawable deliberately left out is the boost plume,
which has its own clock.

### Trackside advertising is not static, and `vex.md` used to say it was

[`vex.md`](../formats/vex.md)'s "Trackside advertising is static, and that is
measured on both axes" is **withdrawn**. It was measured before the keyframe
blocks were known, from quad spans against texture content, and the disc's own
authored blocks disagree - including on the very texture that section names as
its static counter-example:

| Circuit | Texture | Loop | Authored offset track |
| --- | --- | ---: | --- |
| `09`, `16` | `hub_banner_GLOW` | 10 s | `u` 0 to **4 tiles**, with a half-tile `v` step at 5 s |
| `01` | `WES_FEISAR_BANNER_D`/`_E` | 2 s | one whole tile of `u` |
| `04` | `ALTcol_banners3_ADD_nomip` | 2 s | one tile of `u`, and a second material one tile the *other* way |
| `06` | `piranha_banner_ADD_GLOW` | 1 s | one whole tile of `u` |
| `06` | `Moa_logo_GLOW` | 5.83 s | `u` to -2 tiles, held, then to -3.98 |
| `16` | `col_banners2_ADD` | 2 s | one tile of `u` |
| `02` | `logo_feisar_ADD` | 6.65 s | a six-key `u`/`v` path, not a straight scroll |
| `09` | `col_collogo2_glow_add_nomip` | 13.33 s | one tile of `u` |

The geometric survey that produced the static reading is not worthless - it is
still the record of which surfaces the *geometry* singles out - but where it and
the authored blocks disagree, the blocks win, exactly as
[`vex.md`](../formats/vex.md) already says for the `col_display7_GLOW` axis
error. This is the second instance of the same correction.

The renderer already plays all of these. No work follows from this row; it is a
doc correction.

## `Anim Transform` `0x3c0`, and what it cost while unread

Every circuit authors it:

| Circuit | `Anim Transform` | `Transform` | Meshes below one | Of those, at the world origin |
| --- | ---: | ---: | ---: | ---: |
| `01` | 39 | 715 | 48 | 37 |
| `02` | 26 | 291 | 36 | 22 |
| `03` | 12 | 383 | 18 | 14 |
| `04` | 18 | 289 | 29 | 27 |
| `05` | 52 | 371 | 52 | 39 |
| `06` | 71 | 534 | 71 | 8 |
| `07` | 46 | 456 | 86 | 56 |
| `09` | 7 | 409 | 7 | 7 |
| `10` | 8 | 310 | 9 | 4 |
| `13` | 15 | 337 | 16 | 16 |
| `14` | 25 | 614 | 26 | 7 |
| `16` | 74 | 323 | 76 | 8 |
| **Total** | **393** | | **474** | **245** |

`animationTrigger` `0x3dc` is authored **zero** times across the 44 `.vex` files
checked - the twelve `track.vex`, the twelve `start_grid.vex` and twenty ship
files - so whatever starts these animations is not a sibling node. That is a
bounded negative: the disc holds roughly 340 `.vex` files and this is not a
survey of them.

### It cost placement, not only motion - and that was the live defect

`oag_vex::vex::world_transforms` composed the `Transform` class and
contributed the identity for every other class. That is correct for a `Mesh` or
a `Skycube` and **was wrong** for `Anim Transform`: **245 meshes composed to the
world origin**, measured from the composed matrices themselves and needing no
reading of the payload at all. It now evaluates the class, and the count is
**8** - each of those authoring a base and a first key of zero.

For shape rather than proof, here is what hangs off the class. Only 32 of the
393 have a `Transform` child at all - and a child sits *below* the dropped node,
so it could not recover the parent's matrix even where it exists:

| Direct children of an `Anim Transform` | Count |
| --- | ---: |
| `Mesh` `0x125` | 419 |
| `Transform` `0x06e` | 84 |
| `Anim Transform` `0x3c0` (nested) | 14 |
| `ParticleSystem` `0x3c4` | 2 |
| `wopoint` `0x3cf` | 2 |

Two examples of the 245, both measured:

- **`01_Track`**, nodes 70 to 83: six meshes whose whole ancestor chain is
  `World -> Anim Transform -> Mesh`. Their world matrix is the identity and
  their vertices are local (centroids within 2 units of zero), so they draw at
  the origin. The `Anim Transform` payloads hold track-scale coordinates -
  `(-376.47, 20.50, 37.35)`, `(-334.27, 66.00, 67.77)`, `(-376.47, 99.50,
  67.51)`.
- **`16_Track`**, nodes 116 to 124: five sibling meshes that all compose to the
  *same* point `(494.64, 4.32, -52.00)` while their five `Anim Transform`
  payloads carry distinct offsets `-1.39, -2.72, -5.36, -6.67, -7.97` along one
  axis. Five objects the file spreads out are stacked on one spot.

**Confidence 90** on the placement defect itself: it follows from
`world_transforms`'s own code plus the measured node classes and composed
matrices, and does not depend on decoding the payload at all. **Confidence 60**
on reading the three floats at payload `+0x10` as that translation - the
magnitudes are track-scale, the `16_Track` siblings vary along one axis in the
way five spread objects would, and nothing has traced the field in Ghidra.

### The two findings overlap, and the overlap is the legible symptom

The 922 animated materials and the 474 `Anim Transform`-parented meshes are
mostly disjoint populations, but not entirely: **52 meshes are in both**, and
**38 of those 52 are at the world origin**. So there are 38 surfaces on the disc
that replay their authored `TEXOFFSET` scroll perfectly while sitting nowhere
near where the file puts them. `07_Track` carries 34 of the 52, `16_Track` 9,
`09_Track` 5, `03_Track` 3 and `05_Track` 1; the other seven circuits have none.

That is the shape to look for in a frame: not a frozen surface, a *scrolling*
one in the wrong place.

### What the payload turned out to be

Six keyframe arrays after a `0x50`-byte header - translation, rotation and scale
each with `u16` times in 60 Hz frames and `(s16, s16, s16)` values. Translation
is `value * quantum + base`, rotation is a **compressed unit quaternion** with
`w` reconstructed, and scale is 1/256 fixed point, the same convention the
texture-transform block uses. The full field map, how each field was pinned, and
the trap that every immediate in this program is link-time rather than runtime
are in
[`anim-transform.md`](../ghidra/functions/psp-pulse-usa/anim-transform.md).

The map is not merely plausible: **the six arrays tile `[0x50, len)` exactly on
all 393 nodes of all twelve circuits**, so one wrong offset or stride would break
on the first file. `scenery_animation_ground_truth.rs` re-runs that.

## How it is drawn

A moving node cannot be folded into the vertex bake, because the bake happens
once at load and the matrix changes every frame. So the chain is **split**:

- `vex::anim_anchors` gives each node its placement *relative to* the nearest
  `Anim Transform` above it, and which node that is. `mesh::build` bakes that
  into the vertex, and writes the anchor's slot into
  [`GpuVertex::xform`] - deliberately the same shape as the texture path's
  `anim` index.
- `vex::anchor_world` resolves the moving half at a given time, composing
  through nested anchors (14 of the 393 sit under another one).
  `NodeAnims::sample` packs the result into a matrix table and `mesh.wgsl`
  indexes it per vertex, from **binding 1 of group 3** - not a fifth bind group,
  because wgpu guarantees only four.

The two halves must compose back to the whole chain or every animated mesh draws
subtly wrong with nothing else noticing, so that is asserted directly against
`world_transforms_at` at four different times.

**The frustum test is switched off for a moving draw** ([`DrawCall::moving`]);
the section mask is not. Its bounds describe where it was at time zero and
nothing about where it is now, and an honest bound would be its whole authored
path - 5,000 units on the widest node, which would swallow most of the circuit
and cull nothing anyway. That leaves about 7% of a circuit's meshes drawn past
the frustum, against a moving object vanishing when the test disagrees with
where it is. The mask keys on the node, so it needs no bound, and **the original
applies it to moving meshes too** (2026-09-30, [frame-audit.md](frame-audit.md)
section 3): until that day this page said culling was off for both tests, and
the result was sixteen batches of slab transports drawn as dark roofs on Talon's
straight that the original never submits.

One simplification is deliberate and measured: `mesh.wgsl` does not build an
inverse transpose for normals, though 129 of the 920 authored scale keys are
per-axis. All 37 meshes under a non-uniformly scaled node are **prelit**, so
their normals never reach the light rig - asserted, so a title that lights one
fails the test before it renders a wrong frame.

## What is left

- **Which clock a model gets is answered, and the answer is that there is no
  per-model rule.** Both mechanisms read `+0x40` off one object, `g_ingame`
  (`0x08ab0818`), whose only two writers are `InGame_Construct` and
  `InGame_Destruct`. Driving every track node from the race clock is therefore a
  reproduction rather than a stand-in. What advances that field is not traced.
  The *wrap* is read as well: `AnimTransform_Update` (`0x088fe0a8`) does
  `fmodf(t, LoopEnd)`, which is what this reproduces.
- **`AnimEnd`** is read by the binder into `node+0x58` and nothing was traced
  reading it back.
- **The rate multiplier** `AnimTransform_Update` can apply is inert here:
  nothing read writes its `node+0x5c` numerator.
- **Three nodes author `Loopend`**, which the engine's own case-sensitive
  `strcmp` does not match - so they do not loop in the original either, and this
  reproduces that rather than "fixing" the artists' typo.

## Also measured, and not this

- **`start_grid.vex` decodes to no triangles** on every circuit whose file this
  survey read - all twelve resolve by name and every one `mesh::build` was
  pointed at reported "decoded to no triangles". Unrelated to animation, and not
  investigated here.
- **No CLUT/palette animation.** Ruled out separately at confidence 70 in
  [`texture-animation.md`](../ghidra/functions/psp-pulse-usa/texture-animation.md).
- **`FEISAR2anim.tga`-style filmstrips.** Still referenced by zero materials, so
  still nothing to draw.

## Wipeout HD: one mechanism is now played, the other is measured and unwired

Added 2026-08-31. The two halves diverged on the PS3, and they diverged in
opposite directions:

| | Pulse | Wipeout HD |
| --- | --- | --- |
| `Anim Transform` node motion | 393 nodes, played | **5,518 nodes, now played** |
| Per-material texture scroll | keyframe block in the mesh payload, played | **no keyframe block at all**; a fragment program off an engine `time`, unwired |

### `Anim Transform` is the same class, written big-endian

`vex::anim_transform` read its payload little-endian, on the strength of
`vex.rs`'s own comment that no big-endian file reaches the decoders around it -
true of every *geometry* decoder there, and this is not one. So all 5,518 of
HD's nodes decoded to `None`, fell back to the identity, and **dropped their
placement along with their motion**: the same defect, and the same cost, as
Pulse's before 2026-08-18.

The layout is field for field identical. What makes that a measurement rather
than a resemblance is the tiling: the six key arrays fill each payload
contiguously from `0x50`, with at most 15 bytes of alignment padding (Pulse
leaves none), on **5,518 of 5,518** nodes across all seven archives, and
`seconds_per_key` is `1/60` on every one.

| | Pulse | Wipeout HD |
| --- | ---: | ---: |
| `Anim Transform` nodes | 393 | 5,518 |
| `.vex` files authoring one | 44 surveyed | 669 of 742 |
| Busiest single file | 74 (`16_Track`) | **259** (`modesto_heights/track.vex`) |
| `Mesh` nodes anchored to one | 474 | 9,067 |

**What it moves on a circuit**: across the twelve `track.vex` in `DATA00` and
`DATA02`, **2,302 of 2,321** anchored meshes now draw somewhere other than
where the identity fallback put them. Nothing changes its `is_world_baked`
verdict, because a chunk's own `+0x07` space byte answers that outright and the
geometric fallback never sees the new matrix - measured, not assumed
(`crates/render/examples/hd_anim_placement.rs`).

`mesh::NODE_ANIM_LIMIT` went 128 to 384 for the busiest file. `mesh::rcs` now
takes the same anchor split `mesh::build_class` already had, so the geometry
moves rather than merely landing correctly.
`crates/render/tests/hd_scenery_animation_ground_truth.rs` is the harness.

### A widened key form, which Pulse never exercises

The payload's `+0x34` word is zero on all 393 of Pulse's nodes, so what it
selects was unknown. **97 of HD's carry `0x5`**, and under the `s16` reading
those 97 are exactly the 97 whose key arrays leave gaps. Bit 0 widens a
translation key to an `f32` triple and bit 2 widens a rotation key to a whole
`f32` quaternion; read that way, all 5,518 tile.

**Confidence 85 on the widths**, from the tiling closing disc-wide rather than
from an evaluator: the key counts and the six array offsets are separate
fields, so a wrong width leaves a gap on the first file rather than a plausible
animation. Corroborated from a second direction - the 21 of the 97 with
geometry below them author a translation `quantum` of `1.0` and a `base` at the
origin, which is what `base + value * quantum` wants when the value is already
in world units.

Two things deliberately not claimed. The quaternion's component order is taken
as `(x, y, z, w)`, continuing the `s16` form's own `(x, y, z)` plus the `w` it
reconstructs - **confidence 60, a choice rather than a reading**. And bits 0
and 2 are always set together on this disc, so nothing here separates them; a
file setting one alone is the test that would.

The 97 are the grid-camera paths in the `start_grid*.vex` family and one
billboard's shoal of fish (`piranha_landscape.vex`).

### The texture half has no keyframe block to read

HD scrolls a surface in the **fragment program**, off an engine-supplied `time`
parameter, with the material's own authored floats remapping the coordinate
first. The microcode, the parameter-name preimages and the disc-wide reach are
on [`rcsmaterial.md`](../formats/rcsmaterial.md), "A surface scrolls off an
engine `time`".

**Wired 2026-08-31, layer and clock together.** The family adds its scrolled
second texture to the diffuse where `mesh.wgsl` *selects* between the two, so
the layer had to land first and there was nothing for a scroll to move until it
did. `mesh::slots::ADD_SECOND` is the role bit - the same shape as the
`ALBEDO_FROM_SECOND` and `ALPHA_FROM_SECOND` beside it, not the per-material
*lighting* branch that has been refuted twice - and the tint and the two
coordinate constants ride in a per-material table `mesh_render::Emissives`
uploads once at build, because all three are authored and only the clock moves.

**119 material slots across the 16 circuits** draw it, which on the built model
is 62,904 of Modesto Heights' 1,151,776 vertices. The shape census that cleared
it and the reach that sized it are on
[`rcsmaterial.md`](../formats/rcsmaterial.md).

**`uvScale`/`uvOffset` are not the cheaper first step they looked like.** They
are the disc's two commonest parameters and this section used to name them as
the thing to do first; **2,510 of 2,582 records author the identity**, and
`uvScale` is `(1, 1)` on every record on the disc. About fifty billboard
surfaces carry a real sub-tile offset. The numbers are on
[`rcsmaterial.md`](../formats/rcsmaterial.md).

## Upload audit: which race models author texture tracks, and who uploaded them (2026-10-05)

A model animates its texture only if its draw site calls `Drawable::write_anims`
(the texture-offset table) as well as `write_node_anims` (the node table); the
two buffers are separate and a site that writes one leaves the other at phase 0.
The magstrip effect had exactly that gap and read dim and static; this is the
sweep for the rest. `crates/render/tests/anim_upload_census_ground_truth.rs`
(`#[ignore]`d) lists texture tracks / `Anim Transform` nodes per model on the
Pulse disc and pins the track counts.

| Model | Tracks / nodes | Draw site | Result |
| --- | --- | --- | --- |
| `Pulse_Mine`, `Pulse_Bomb` | 1 / 1, 2 / 2 | `write_one_kind` | **Gap, fixed**: texture now on the race clock the node table already rode (`mine_texture_scroll_ground_truth.rs`) |
| `pulse_plasma_halo1`, `hemisphere1`, `hemisphere2` | 1 / 2, 1 / 0, 1 / 1 | `write_plasma_blasts` | **Gap, fixed**: texture on the model's own `age * rate` scrub. `Node_SetAnimTimeTree` hands one time to the `Mesh` and `Anim Transform` nodes of a tree alike (`anim-transform.md`), so the node time is the texture time. `hemisphere1` has no node, so this is its only upload. The rate (`0.1` / `0.07`) is confidence 75, and the scrub reaches 0.15 s of a 1.98 s track in a 1.5 s blast, so the motion is small but visible |
| `explosion_hemisphere`, `Bomb_Shockwave`, `pulse_repulsorwave` | 1, 2, 1 / 0 | `write_bomb_blasts` | **Gap, fixed (2026-10-05, `fx-age-clocks`)**: texture on the object's own age. `BombBlast_Construct` and `Repulser_Init` seed each model with `Node_SetAnimTimeTree(0.0)`, and `Mesh`'s per-frame update then adds the clock's delta to `mesh+0x40`, so the texture time is **age at rate 1, start 0, one clock per object** (`anim-transform.md`, "A mesh's texture time"; confidence 88). Measured live on PPSSPP over two boots: every blast mesh's `+0x40` is `clock - spawn_clock` with slope `1.0000` and residual `0.0000`, the spawn clock equal to the detonation clock (60.9, 157.5, 294.1, 75.1) and not the race clock. The ship explosion's shockwave is the same model and `ShipShockwave_Construct` seeds it the same way (`0x0885ee44`), read statically only |
| `MagEffect1`, `MagEffect2` | 1 / 0, 1 / 1 | `write_mag_floor_fx` | Fixed earlier (`magfloor-fx.md`, "Look") |
| `Ship.vex`, `shipshield`, `Zone.vex` | 1 / 0 | `frame.rs` | Already uploaded (hulls; shells `shield_shell_scroll_ground_truth.rs`) |
| `shipboost`, `Zoneboost` | 1 / 0 | `Drawable::apply_uv_transform` | Deliberate: the plume's clock is its reveal timer, and `write_anims` on top would apply the transform twice (`frame.rs`) |
| `shipwreck`, `zonewreck`, `Rocket`, `pulse_muzzleflash` | 0 / 0 | n/a | Author none |
| `vr_shield_cockpit` | 1 / 1 | not loaded | Nothing draws it yet |
| Ship shine and hull overlays, the track's shine pass | n/a | `shine.rs`, `absorb_overlay.rs` | Not an authored transform: their coordinates are rewritten every frame (`write_view_map`, `write_overlay`, the original's `HullOverlay_Submit` scroll), so `write_anims` would double it |
| Track, sky, pads, collision, gantry | many | `frame.rs`, `gantry.rs` | Already uploaded (`scenery_animation_ground_truth.rs`) |

**How the mine test isolates the texture.** The mine's node loops every 119
frames and its texture every 1 s, so two clocks 59.5 s apart (thirty node loops,
half a texture loop) show the same node pose and opposite texture phases. 791,563
summed RGB over the mine cluster with the upload, 497 without; a no-mine control
over the same clocks moves 896. The bomb's node and texture both loop at 3 s, so
no clock pair separates them and the bomb is checked by eye only. Plasma blasts
are age-driven, so `--anim-seconds` does not reach them; their evidence is a
before/after pair at fixed ticks (impact against the start-line wall,
`--pose 6.07,-50.07,-196.05,90`, plasma fired at tick 421): the hemisphere
bands differ by 201,658 / 381,052 / 509,492 summed RGB at ticks 505 / 525 / 545.

**The three age-driven blasts (2026-10-05).** `Drawable::write_anims(queue, age)` now runs on the hemisphere, the shockwave and the repulser's field model. No `--anim-seconds` pair separates them: the blast's geometry is itself a function of age, so a second tick moves every pixel the texture does. The wiring is guarded at the draw struct (`bomb_blast::tests`, `tests::repulser`: the draw carries the age, tick for tick), and the picture is a before/after pair at fixed ticks. Repulser, `--give repulser` fired at tick 421, ticks 424 / 436 / 460: 898 / 706 / 767 of 130,560 pixels differ with the upload, all inside the ring's noise texture, a small change as the plasma blasts' was (the track is 1.98 s and the blast 1.5 s). A bomb frame could not be produced: no `oag-game` scenario detonates the player's own laid bomb (it sits behind the craft; 53 s of fuse did not end it either), so the Bomb path has the live measurement and the draw-struct test but no picture. Scratch: `data/scratch/fx-age-clocks/{before,after}/`.

**Wipeout HD and Pure.** Pure shares the Pulse draw paths above, so the fix
reaches its mine and bomb. HD's blasts take `write_clocked` with
`model_clock` instead and were not touched; its RCS materials are another lane's.
