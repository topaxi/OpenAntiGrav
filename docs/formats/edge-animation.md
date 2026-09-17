# Edge Animation Tools clip: HD's animated material curves

**Sony's own PS3 SDK middleware, not a bespoke Wipeout format.** A `.rcsmodel`
material record can carry a pointer to one of these at `+0x20`, then `+0xc`
(`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s 2026-09-17 sections);
the digit board on HD's start gantry, `321go_startfinish.rcsmodel`, is the one
file this has been checked against - four of its five materials carry a curve,
all four driving that material's own `uvOffset.x`/`.y`.

Implemented in [`oag_rcs::edgeanim`](../../crates/rcs/src/edgeanim.rs) (the
clip itself) and
[`oag_rcs::rcsmodel::material::curve`](../../crates/rcs/src/rcsmodel/material/curve.rs)
(a material's own pointer to one), checked against the disc by
[`hd_gantry_curve_ground_truth.rs`](../../crates/rcs/tests/hd_gantry_curve_ground_truth.rs).

## The headline

| Question | Answer | Confidence |
| --- | --- | --- |
| What this is | Sony's Edge Animation Tools clip format, PS3 SDK middleware | 95 - the evaluator's own embedded assert strings name its source file |
| Where a curve hangs off a material | `material_record + 0x20`, then `+0xc` | 90 |
| Container tag | `"EA02"` at the clip's own `+0x00`, no other magic | 95 - present, byte-identical, on all four curves checked |
| Offset convention | Self-relative: a field's own address plus its own value | 95 - confirmed by the frame-set table reading correctly only this way |
| Bit-packed keys | **Absent on every clip checked** - `offsetPackingSpecs` is always zero | 90 - the PPU evaluator itself asserts this before reading anything else |
| Joint (rotation/translation/scale) channels | **Absent on every clip checked** - all seven counts are zero | 85 |
| What the four checked curves actually animate | `uvOffset.x`/`.y`, confirmed by parameter-table hash, not position | 92 |
| What one curve's own shape is, in isolation | A per-material wipe/reveal ramp, not a four-state plateau | 85 - measured directly, see below |
| How this connects to `3`/`2`/`1`/`GO` reading correctly | **Material 2's curve alone is the mechanism** - it drives both the digit glyph mesh and the backdrop panel; materials 1/3/4's curves belong to geometry the pre-existing panel-clip and `fx350` strip already remove before a frame draws | 88 - real headless captures plus a curve-disabled control render, `scratch/lane-wire-report.md`, one boot |

## What this is, and how it was found

`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s 2026-09-17 sections
trace the runtime write chain that lights the countdown's digit board:
`Billboard_UpdateAndRender` -> `Billboard_UpdateInstanceUvs` ->
`AnimCurve_EvaluateChannels` -> `AnimCurve_SampleChannel` -> a deeper,
previously-unnamed evaluator. That evaluator's own decompiled body carries two
`_SCE_Assert` calls whose string arguments are not this project's own code:

```text
"edgeanim_evaluate_ppu.cpp:469 (anim->offsetPackingSpecs == 0)"
"edgeanim_evaluate_ppu.cpp:283 (frameInteger <= intraFrameCount)"
```

`edgeanim_evaluate_ppu.cpp` and `anim->offsetPackingSpecs` are Sony's own
source file name and struct field name, from the PS3 SDK's Edge Animation
Tools middleware (part of the same Edge suite many PS3 titles link for
compressed skeletal/curve animation) - not decompiled speculation, read
straight off the binary's own embedded strings. Renamed
`EdgeAnim_EvaluateClip` (`0x0066b840`, confidence 85) and
`AnimCurve_SampleChannel` (`0x0066c3c8`, confidence 80).

**Public SDK documentation for this format was searched for and not found
reachable** - the PS3 SDK itself is not something this project can fetch and
extract from here, and no header or manual describing Edge Animation Tools'
own struct layout turned up in a web search. So unlike the ideal this format's
own public-middleware status would allow, the byte layout below is **matched
against the evaluator's own decompiled control flow and cross-validated
against four independent clips on `hdfury-ps3-eu-dec.iso`**, not checked
against Sony's own published struct definitions. Where a reading is
structural (a self-relative offset resolving to a sane, monotonic table) that
is cited as the evidence, in the same spirit as every other format page here.

## Layout

A curve pointer (`material_record+0x20`, then `+0xc`) leads to a small struct:

```text
+0x00  u32   self-relative offset of the clip header (the "EA02" struct below)
+0x04  u16[] channel descriptors, one per animated channel - see below
```

The channel count is not in this struct - it is read from the clip header's
own `+0x24` (below). Each descriptor is one big-endian `u16`: `target_index`
in bits 2-15, `component` (0-3, which of the parameter's four `f32`s) in bits
0-1. `target_index` indexes the **material's own raw parameter table** (the
`+0x34` field `oag_rcs::rcsmodel::material::parameters` documents) - not the
filtered list that module's own `parameters()` function returns, which drops
every sampler entry and so does not preserve raw table positions.

The clip header itself:

```text
+0x00  u32   "EA02" - Edge Animation's own version tag
+0x04  f32   duration, in seconds (one authored loop)
+0x08  f32   sample_frequency, in frames per second
+0x12  u16   number of frame-sets
+0x16..0x22  u16 x 7  every joint channel count (rotation/translation/scale,
                      constant and animated) - must all be zero; this module
                      refuses to parse anything else
+0x24  u16   number of animated scalar ("user") channels
+0x34  u32*  self-relative offset of the frame-set DMA array
+0x38  u32*  self-relative offset of the frame-set info array
+0x4c  u32*  self-relative offset of "packing specs" - must be absent (zero)
```

`*` marks a **self-relative offset**: read directly off the evaluator's own
decompile (`iVar30 = iVar39 + 0x38 + iVar32;` - the field's own address plus
its own value, not the clip's start plus the value). A zero value means
"absent". This is confirmed empirically, not merely read off one decompiled
line: resolving the frame-set info offset this way produces a strictly
increasing `baseFrame` sequence across all four clips checked; resolving it
any other way (clip-relative, or off by a field) does not.

**Why `+0x4c` for `offsetPackingSpecs` rather than a rounder-looking offset**:
because that is where this binary's own evaluator reads it. The field is
pinned by `EdgeAnim_EvaluateClip`'s own assert - `edgeanim_evaluate_ppu.cpp:469
(anim->offsetPackingSpecs == 0)` - firing on the word at `+0x4c` and on no
other offset, checked against every neighbouring candidate. A rounder `+0x50`
does not fire it. The disc's own decompiled arithmetic is the only authority
this layout rests on, and it is sufficient on its own.

The struct and field names used on this page and in the code
(`sampleFrequency`, `numFrameSets`, `EdgeAnimFrameSetInfo`,
`offsetPackingSpecs`) come from the binary's own embedded assert strings,
which name them directly.

### Channel tables - located empirically, not off a named field

Between the clip header's fixed portion and the frame-set DMA array sits a
small table of `u16`s: constant-rotation, constant-translation,
constant-scale, constant-user, animated-rotation, animated-translation,
animated-scale, then animated-user channel index tables, each's length
rounded up per its own count (empty on every clip checked except the last).
**This module locates it as the 16-byte-aligned block immediately before the
frame-set DMA array**, rather than by a header field naming its own offset -
the header-field reading that would be expected by analogy with the other
offset fields above lands four bytes short of real data on every clip
checked, the same one-field discrepancy the `offsetPackingSpecs` note above
already found. On `321go_startfinish.rcsmodel`'s four curved materials this
table holds exactly `[0, 1]` (padded to four `u16`s of block alignment): the
two animated-user-channel ids [`Curve::sample`] matches a channel descriptor's
own `component` against.

### Frame-set info array

`numFrameSets` entries, 4 bytes each:

```text
+0x00  u16   base_frame - the first absolute frame this frame-set covers
+0x02  u16   num_intra_frames - frames strictly inside it
```

A binary search over `base_frame` (`_edgeAnimGetFrameSetIndex`'s own
bisection, reproduced in `Clip::frame_set_index`) finds which frame-set a
given frame position falls in.

### Frame-set DMA array

`numFrameSets` entries, 8 bytes each: `{u32 unread, u32 offset}`. The second
word is **relative to the clip header's own address**, not to the field
itself - the one place in this format the self-relative convention anchors
somewhere other than the offset field (`dmaArray[frameSetIndex*2+1] + anim` in
the evaluator's own decompile). It is where that frame-set's own data block
sits.

### One frame-set's data

```text
+0x00  u16 x8  byte sizes: initial R/T/S/U, then intra R/T/S/U
+0x10  ...     "initial" (boundary) values, one full-width value per channel,
               in R/T/S/U order - R/T/S empty on every clip checked
       ...     the intra-frame presence bitmap: one bit per (channel, frame)
               pair, MSB-first, R/T/S/U order, popcount-addressed
       ...     the intra keyframe values themselves, same order, one full
               value per set bit
```

A user (scalar) channel's value is a plain big-endian `f32`, 4 bytes -
**there is no quantisation or bit-packing on any channel this module has
read**, consistent with `offsetPackingSpecs` being absent. A channel's own
bracketing pair of keys (the frame just before and just after the sample
point) is found by counting set bits in the presence bitmap up to the sample
frame (`_edgeAnimGetBracketingKeyframes`, reproduced in
`edgeanim::bracketing_keyframes`), and the two keys are linearly interpolated
- `alpha = (b_bits + frame_fraction) / (a_bits + b_bits)`, where `a_bits`/
`b_bits` count the contiguous unset bits after/before the sample frame. The
"final" boundary value of one frame-set is the same bytes as the "initial"
boundary value of the next - they share one memory location by construction,
so there is no discontinuity possible at a frame-set seam, confirmed by
sampling either side of one and finding no jump.

Two boundary-value blocks look at the *next* frame-set's own size header to
find where they end - reading past a clip's very last frame-set (frame index
past the end) is not handled by this module and is not a case any of the four
curves checked exercises, since HD's own countdown never runs the clip past
one full loop before wrapping.

### Rotation, translation and scale channels: not read

`Clip::parse` refuses (`None`) any clip whose rotation, translation or scale
channel counts (constant or animated) are not all zero. Every clip this
module has been checked against is a pure scalar/"user channel" curve - HD's
material `uvOffset`/`uvScale` values are floats, not joint transforms - so
there has been no data to develop or check a joint-channel decoder against.
The evaluator's own decompile for that path involves a
`vectorReciprocalSquareRootEstimateFloatingPoint`-based slerp
(`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s 2026-09-17 section) and
a 48-bit packed quaternion encoding, neither of which this project has
decoded or had data to check against - a real, separate task if this format turns out
to be used for skeletal animation anywhere else in HD.

## What one curve authors in isolation: a wipe, not a plateau - and what four of them together turn out to be

**Read this section, then its own 2026-09-17 correction below it**: what
follows measures one curve at a time and is accurate as far as it goes, but
reading it alone leads to the wrong conclusion about the countdown - the
correction is not a footnote.

Sampling all four of `321go_startfinish.rcsmodel`'s curved materials across
their own shared 13.333 s loop (`each_curve_varies_continuously_not_in_glyph_sized_steps`,
`material_one_s_y_channel_ramps_smoothly_across_its_frameset_boundary`) shows:

| Material | `sample_frequency` | Shape of `uvOffset.y` across the loop |
| --- | --- | --- |
| 1 | 463.56 Hz | flat ~0 for 94% of the loop, then a smooth ramp 0 -> 1 in the last ~1.1 s |
| 2 | 120.00 Hz | a smooth ramp 0 -> ~0.525 over the first ~6.4 s, then flat |
| 3 | 398.09 Hz | flat ~0.25 for ~44%, a smooth ramp to ~1.0 around 44-56%, then flat |
| 4 | 348.71 Hz | flat ~0.02 for ~69%, a smooth ramp to ~0.40 around 69-94%, then flat |

Each curve is a **single continuous wipe at its own point in the loop**, not
four discrete plateaus at `{0, 0.25, 0.5, 0.75}` the way a glyph selector
needs - contrast Pulse's own gantry, where one shared offset track walks four
literal UV cells in discrete steps
(`crates/vex/tests/start_gantry_ground_truth.rs`). HD's mechanism reads
instead like a staggered reveal or wipe effect: four materials, each fading
or scrolling in at its own staggered window of one shared 13.333 s loop.

**What this does and does not settle.** The curve format is decoded and its
values, on this file, are real and correctly sampled - not invented, not
fitted to a picture. What it does not do is explain, by itself, how a player
reads `3`, `2`, `1`, `GO` in sequence over a ~6 s countdown while this curve's
own loop is 13.333 s: no time base connecting the two has been established
(`Billboard_UpdateInstanceUvs`'s own clock argument, where it ultimately comes
from, was not traced further - tracing it is a new RE thread, out of the scope
that recovered this format).

**2026-09-17, `lane/hd-gantry-wire`: playback is now wired, generically, and
this is not the same claim as "the digits are correct".** Every `.rcsmodel`
material carrying a curve - not gantry-specific, see
[`oag_render::mesh::rcs::curve_track`](../../crates/render/src/mesh/rcs/curve_track.rs)'s
own doc comment - becomes one more entry of
[`oag_render::mesh::Model::anim_tracks`](../../crates/render/src/mesh/anim_track.rs),
the exact table Pulse's own `TEXOFFSET` blocks already fill, sampled every
frame through the **same per-model clock every other animation on that model
already rides** - the race clock, unshifted, `crates/game/src/race/gantry.rs`'s
own `seconds = world.tick / 60` for the gantry, `mesh::rcs::skin`'s callers'
own clock for anything else. **No new time base was chosen**: this is the
standing "never invent" rule read the other way round - the invention this
page always refused was picking a clock to make the digits line up, and
reusing the clock the model's own node animations already use is not that,
it is the one clock this project already had evidence for. Verified end to
end against the real disc,
[`hd_gantry_curve_replay_ground_truth.rs`](../../crates/render/tests/hd_gantry_curve_replay_ground_truth.rs):
all four curved materials reach `AnimTrack::Rcs`, a vertex of each selects
one, and the sampled table differs between two points in time.

**Corrected twice the same day, and the second correction is the one that
holds.** The first pass at wiring the replay watched the board actually play
and saw `3`, `2`, `1`, `GO` in order, and concluded "four curves, one per
glyph, each its own material" - a real improvement on "not established", but
still wrong about *which* curve does the work, for a reason a control run
caught: it inferred the mechanism from the fact that four materials carry a
curve, without checking which of those four materials' own geometry a player
actually sees.

**Only material 2's curve is ever on screen. The other three belong to
geometry the pre-existing (unrelated) culling logic already removes before a
frame is drawn.** Dumping which node each animated draw call belongs to
(`hd_gantry_curve_replay_ground_truth.rs`'s own diagnostic pass) gives the
node name behind every one of the four `AnimTrack::Rcs` slots:

| Material | Node(s) | What it is | Reaches the screen? |
| --- | --- | --- | --- |
| 1 | `polySurface7Shape` | the chequered-flag state | **No** - `oag_render::gantry::clip_to_panel` parks it outside the mount's own panel, trigger unrecovered (`docs/rendering/start-gantry.md`) |
| 2 | `Go_HD_start_light_backgroundShape`, `pasted__Go_HD_start_light_321goShape` | the countdown backdrop **and** the digit glyph mesh (the one with the five UV cells) | **Yes** |
| 3 | `polySurface151Shape`..`polySurface157Shape` (7 nodes) | slot 7's own `fx350` art, embedded in this file | **No** - `oag_render::gantry::strip_fx350_art` already strips exactly these seven node names |
| 4 | `pasted__Final_LapShape`, `pasted__Final_Lap_1Shape`..`_1_6Shape` (7 nodes) | the `FINAL LAP` state | **No** - parked outside the panel by `clip_to_panel`, same as material 1 |

So the real mechanism is **one shared curve, on one material, walking across
one texture** - the same shape as Pulse's own gantry, not a distributed
four-way one. Material 2's single `uvOffset`/`uvScale` curve drives the
digit glyph's own five UV cells *and* the backdrop panel at once, because
both surfaces share the same texture (`321_go_64.gtf`, texture slot 2) and
the same material. **A plausible reading, not a measured one**: the
backdrop's own red-to-green transition likely comes from the same shared
offset crossing the "authored, unused by this node" red/green marker
columns at texel columns 48-54/57-59 `start-gantry.md`'s own texel-grid
table already found - the timing and the colours both fit - but nothing
here has actually sampled the backdrop's own UV cell against `321_go_64.gtf`
at the curve's `t=180`/`t=240` values to confirm it, the way that page's own
Pulse table confirms its digit windows. Left as a lead rather than a
finding.

Played for real: `oag-game --race --track
/data/environments/talons_junction/track.vex --no-audio --screenshot`,
ticks 0/90/180/240/360, `hdfury-ps3-eu-dec.iso` - blank at tick 0, `3` and
`2` both legible by tick 90 (1.5 s), `3`/`2`/`1` together by tick 180 (3.0 s),
`GO` alone on a banner that has gone from red to green by tick 240 (4.0 s),
and the whole board gone by tick 360 (6.0 s) - the pre-existing, unrelated
`Anim Transform` teleport-out this page's own "master timeline" already
documents, untouched by this lane. **Confirmed by a control render** with
every material's `anim` index forced to `0` (curve replay disabled): the
backdrop panel and the digits vanish together at every tick checked, rather
than only the digits going missing - which is what would have shown if a
separate, curve-independent node swap did the red/green change, and is not
what happened. Frames from both the real and the control render are in
`scratch/lane-wire-report.md`.

**What is still not settled**: `3` and `2` reveal together rather than
strictly one after the other (consistent with two of the glyph's five UV
cells crossing their own reveal threshold close together, the same shape
Pulse's own texel-grid table shows for its four cells), and the total
sequence runs in about 4 s of the asset's own clock rather than the ~6 s the
measured thrust gate takes - both read exactly as the disc authors them, not
smoothed into a tidier story. Only one circuit and one boot were checked.

## Coverage

Not instrumented against `oag_formats::coverage` - this module reads a
narrow, self-contained slice of one `.rcsmodel` (the four curves'
own bytes) rather than claiming ranges of the whole file the way
`oag_rcs::rcsmodel`'s own coverage tracking does. Every byte range this
module's `Clip`/`Curve` parse touches is bounds-checked against the file
before being read; nothing is read past what `data.len()` allows.

## What the later titles do instead: Edge is PS3-only, and Omega re-bakes

**Measured 2026-09-17 against the shipped archives and executables**, after
this format landed, to answer whether any of it transfers to Wipeout 2048
(Vita) or the Omega Collection (PS4) - Omega being a remaster that ships
HD/Fury's own content.

| | HD/Fury (PS3) | 2048 (Vita) | Omega (PS4) |
| --- | ---: | ---: | ---: |
| `edgeanim` assert strings in the executable | present | **none** | **none** |
| `1.2.3.0-PS3-SPU-EDGE` | present | absent | absent |
| `.rcsanimclip` / `.rcsskeleton` entries | **0** | 49 + 49 | 31 |
| `AnimCurve*` Maya type names | 9 | 9 | 9 |

**Edge does not transfer, and the reason is hardware.** Edge Animation Tools
is part of Sony's SPU-oriented PS3 Edge suite; the Vita and PS4 have no SPUs
and neither executable contains a trace of it. So `oag_rcs::edgeanim` is a
PS3-only reader by nature, not by omission.

**All three titles share the same authoring, though.** The nine `AnimCurve*`
strings (`AnimCurveTimeToAngular`, `AnimCurveUnitlessToDistance`, ...) are
Maya's own animation-curve type names and are byte-identical across the three
executables. What differs is only how the authored curves are baked for the
target.

**Omega re-bakes HD's circuits into sidecars this project already reads.**
Nine HD/Fury environments ship a `.rcsanimclip`/`.rcsskeleton` pair in Omega
where HD embeds Edge clips in the `.rcsmodel`: `talons_junction`, `amphiseum`,
`03_moa_therma`, `04_chenghou_project`, `05_ubermall`, `10_sebenco_climb`,
`12_sol_2`, `15_anulpha_pass` and `zone_3`, as `track.final.*` and
`track_reversed.final.*`. `oag_rcs::rcsanimclip`/`rcsskeleton` decode that
container already (see [`2048-animation.md`](2048-animation.md)).

**But the two are different kinds of animation and do not substitute for each
other.** The sidecars carry **node transforms** - a hierarchy plus keys for a
subset of nodes. Edge's clips here carry **material parameters**
(`uvOffset`/`uvScale`), which is why this project routes them through
`AnimTrack::Rcs`/`TexAnims` rather than the node-animation path. The two are
complementary: a circuit can have both, and reading one says nothing about
the other.

**Open, and deliberately not chased here**: Omega ships the billboard and
front-end flyer models (53 and 43 entries respectively, including
`billboards/HD_Adverts/321Go/`), and **none of them has a `.rcsanimclip`
sidecar** - every sidecar in the archive is an environment, a crowd rig, or
`startanim`. So either Omega's adverts and flyers are static, or they carry
material-parameter animation in some embedded form this pass did not look
for. Deciding which needs a reader for Omega's own `.rcsmodel` header, which
is little-endian with a different layout from HD's big-endian one - the same
reason HD's `material_record+0x20` reading cannot simply be pointed at it.
`321go_startfinish.rcsmodel` itself is **absent** from Omega; the nearest
thing is `Data/startanim/model/start.rcsanimclip`, which sits beside 2048's
own `startanim_fc07_*` materials rather than HD's gantry.
