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
| What the curves' own shape is | A per-material wipe/reveal ramp, **not** a four-state glyph selector | 85 - measured directly, see below |
| How this connects to `3`/`2`/`1`/`GO` reading correctly | **Not established** | - |

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

## What the curves actually author: a wipe, not a glyph selector

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
that recovered this format). **Playback is not wired into
`crates/render/src/mesh/rcs.rs`** for exactly this reason: wiring it would
mean picking a time base with no evidence behind the choice, which is the
invented-mechanism failure `CLAUDE.md`'s "never invent what the assets
already author" rule exists to stop - a curve landing on a plausible-looking
frame by construction of a guessed time base is not the same thing as the
digit board actually reading correctly for the reason this curve drives it.

**A live capture already shows the digit board's glyphs light up during an
actual countdown** (`docs/rendering/start-gantry.md`,
`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s 2026-09-13 section), and
the per-node `+0xe4` static UV override table `Billboard_UpdateInstanceUvs`
also reads (billboards.md's 2026-09-17 first section) is at least as likely a
candidate for the actual glyph-selection mechanism as this curve - it is a
second, distinct write path this project has located but not chased, per this
lane's own scope. Whoever picks this up next: start there rather than
re-deriving a time base for the curve documented on this page.

## Coverage

Not instrumented against `oag_formats::coverage` - this module reads a
narrow, self-contained slice of one `.rcsmodel` (the four curves'
own bytes) rather than claiming ranges of the whole file the way
`oag_rcs::rcsmodel`'s own coverage tracking does. Every byte range this
module's `Clip`/`Curve` parse touches is bounds-checked against the file
before being read; nothing is read past what `data.len()` allows.
