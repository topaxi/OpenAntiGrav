# ADR-0011: Cull with the track's authored PVS, before the view frustum

## Status

Accepted.

Built and on by default. The authored set is decoded and validated
([`oag_formats::pvs`](../../../crates/formats/src/pvs.rs)), the draw-call
association and two-tier test live in
([`oag_render::pvs`](../../../crates/render/src/pvs.rs)), and the race loop runs
both tiers under `[graphics] pvs_culling` and `[graphics] frustum_culling`.

## Context

Tracks are static, winding corridors, which is the geometry a potentially
visible set is for: from inside a tunnel or below the lip of a hill, most of the
circuit cannot be seen at all, and that fact is fixed at authoring time. Testing
it per frame against the camera is work whose answer was knowable before the
game shipped.

**And it was known before the game shipped.** Every Pulse track declares up to 64
`section` nodes (class `0x3c9`), each carrying a 64-bit mask naming the sections
visible from inside it and an axis-aligned box. The layout is recorded in
[track.md](../../formats/track.md) and now decoded and validated against every
track file on both discs. The measurements that matter here:

| | PSP | PS2 |
| --- | ---: | ---: |
| Track files | 40 | 59 |
| `section` nodes | 2,268 | 3,045 |
| Mean sections visible from a section | **7.6 of 64** | **7.5 of 64** |
| Spline control points cross-checked | 34,261 | 50,218 |

A mean of 7.6 of 64 is the number this decision rests on: **the authored data
says roughly seven-eighths of a circuit is not visible from any given point on
it.** No estimate of ours produced that figure; the artists did, and it is
sitting on the disc.

The renderer already has the second tier. Frustum culling landed in M6: every
`DrawCall` carries a world-space bounding sphere tested against the camera's
frustum before `draw_indexed`. It is **on by default**, and its cost is known
and unconditional - measured on `16_Track` at ~59 microseconds a frame to check
~2,000 draw calls, confirmed as a real improvement on a weak integrated GPU and
unconfirmed on anything with more headroom.

That is what makes the ordering question worth asking rather than academic.
There is now a per-frame floating-point cost that every player pays on every
track, and it is paid over the whole circuit including the seven-eighths of it
the artists already recorded as invisible. A frustum test is six
plane-versus-sphere evaluations; a PVS test is an `and` against a mask already
in a register.

## Decision

**Two tiers, PVS first, frustum second.**

```text
[ all draw calls ] -> [ 1. authored PVS mask ] -> [ 2. view frustum ] -> [ GPU ]
```

1. **The visible set comes from the disc.** `TrackPvs` reads the `section`
   payloads at load; nothing computes visibility.
2. **The current set is the union of the craft's section and the camera's**,
   each expanded by its own mask, then padded by adjacency. Three reasons, all
   of which happen in a race: the chase camera lags and swings and is routinely
   in a different section from the craft it follows; a boundary crossed between
   two frames means the section becoming current must already have been drawn;
   and either lookup may fail.
3. **Padding follows the spline, not the numbering.** Section ids are neither
   dense nor ordered along the track - `09_Track` reversed has 44 sections and a
   maximum id of 45 - so `id ± 2` is not "next door". Adjacency is built at load
   by walking consecutive control points and joining paths across junctions,
   which is authored data rather than arithmetic.
4. **Unknown means everything.** A section id past the cap, or one the track
   does not declare, answers all-ones, matching the original's own lookup at
   `0x0891e908`. Every uncertain path degrades to drawing too much.
5. **Then the frustum**, unchanged, over whatever survived.

6. **A draw call belongs to every section it touches**, not to one. Its
   bounding sphere is intersected against each authored box at load and the
   result kept as its own `u64`; it is drawn when that mask and the frame's
   visible set share a bit. Geometry touching no box gets all ones and always
   draws. See [Alternatives](#place-each-draw-call-in-one-section-by-its-centre)
   for the cheaper rule this replaced and what it broke.

Both tiers are independently switchable, config-file only, and **both default
on** - `[graphics] frustum_culling` and `[graphics] pvs_culling`. The bar for
the second was the one the first had to clear: a screenshot comparison showing
it changes nothing about what is drawn. Thirty captures across three tracks,
several tick counts and every combination of the two tiers come out
byte-identical to culling nothing.

## Alternatives considered

### Bake our own PVS at import time, by raycasting

The shape this work was originally specified in: sample a camera at track centre
per segment, cast a dense cone of rays, record what it hits, cache the result in
a binary track format.

**Rejected, because the answer is already on the disc.** Building it would mean
computing a second visibility set that disagrees with the authored one in ways
nobody chose, then shipping the disagreement as pop-in. It also inverts this
project's premise - the original is the specification - by preferring a derived
result to a shipped one. And it is strictly more machinery: a baker, a cache
format, a cache to invalidate, and a rebake whenever the sampling changes.

The specification came with a well-argued warning attached: that a baking camera
must align its up-vector to the track surface normal, because a hardcoded
`(0, 1, 0)` points into the floor on loops and inverted sections and blinds the
raycaster. **The warning is real and it is aimed at the wrong game.** Inverted
track is not a future WipEout HD concern here - it is present in Pulse, which is
this project's current target: `01_Track` declares a `Mag Floor Collision` node,
and [angular-velocity-column.md](../../physics/angular-velocity-column.md)
records a real inverted stretch of a shipped circuit with `up.y` reaching
`-0.99994` across ticks 1,072-1,329. So the hazard is immediate rather than
hypothetical. It is also moot: with no sampling camera, there is no up-vector to
get wrong. This is the strongest argument for consuming the authored set - the
hardest case for a baker is one the artists already solved.

### Place each draw call in one section, by its centre

One `u8` per draw call instead of a `u64`, and a shift-and-and instead of an
`and`. Cheaper in both memory and instructions.

**Rejected, having been built and measured.** It loses geometry larger than a
section. A ridge line or a terminal building spans a dozen sections, its
bounding-sphere centre lands in exactly one, and the whole mesh disappears the
moment that section drops out of the visible set - while it is still plainly in
shot. A capture at 600 ticks on the default track differed from culling nothing
by 212,808 bytes, with a ridge and a large building missing down the left of the
frame.

Two things make this worth recording rather than quietly fixing. It is
**invisible in aggregate statistics**: centre placement reports *fewer* draw
calls surviving the first tier, which reads as better culling and is simply
wrong - the measurement that looked best was the broken one. And the cheaper
rule is exactly what someone optimising this later would reach for.

### The flat `segmentOffsets` + `visibleSegmentIds` pair

A count, an offsets array mapping section to a range, and a flat array of
visible ids clumped sequentially.

**Rejected for the masks themselves.** The engine caps at 64 sections, so a
visible set *is* a `u64`: 512 bytes for a whole track, one or two cache lines,
and a test that is a shift and an `and` with no indirection. The offsets-plus-ids
pair costs more memory, adds a dependent load, and turns a branch-free test into
a loop.

**Not adopted one level up either, though for a while it looked like it would
be.** Sorting draw calls by section at load and storing a range per section
would make an excluded batch cost nothing rather than one mask test. It does not
work here: a draw call belongs to *several* sections, so there is no single key
to sort by without duplicating entries. The transparent list is also drawn in
authored order, where blending makes reordering change the picture. The real
lever is elsewhere - see
[the granularity ceiling](#the-limit-is-our-batch-granularity-not-the-authored-data).

### Cull to the craft's section alone

Cheaper - one lookup, no union, no padding. **Rejected**: the camera is not the
craft, and culling to the craft's section cuts geometry out of the shot the
camera is actually framing. See the Decision's point 2.

### Frustum first, PVS second

**Rejected on cost.** It runs six plane-versus-sphere evaluations on geometry a
single integer test would have excluded. The ordering is the entire performance
argument for adding a second tier at all.

## Consequences

### The good part, measured, and smaller than the authored ceiling suggests

The authored ceiling is a mean of 7.6 of 64 sections visible. What a running
game gets is a different question, and a smaller answer.

Measured by
[`pvs_placement_ground_truth.rs`](../../../crates/render/tests/pvs_placement_ground_truth.rs)
over **all 40 track files on the PSP disc** - every circuit, and each one's
reversed and Zone variants - averaging across every section in turn, with the
camera trailing the craft into a neighbouring section the way the chase spring
puts it rather than sitting at a hand-picked viewpoint.

| Draw calls reaching the frustum test | Across 40 track files |
| --- | ---: |
| Best | **33.5%** (`01_Track/zone_track.vex`, 7.5 sections per draw call) |
| Median | 53.1% |
| Mean | 50.4% |
| Worst | **78.4%** (`05_Track/track.vex`, 14.1 sections per draw call) |
| Files under 50% surviving | 18 of 40 |
| Files over 70% surviving | 2 of 40 |

Every draw call is placed on most files and 99.7% or better on the rest, so no
track has geometry the authored boxes miss.

**The saving is real, roughly half on a median track, and varies by more than a
factor of two.** The first tier removes 66% of the frustum test's input on the
best file and 22% on the worst. Anyone who reads "a section sees 7.6 of 64" and
expects to keep a seventh of the work will be disappointed on every one of the
40.

**In the worst section of almost every track it saves nothing** - 88% to 100%
still reach the frustum test. From a section that can see a lot, there is
nothing for the first tier to exclude. There is an average to budget with and no
floor, and a player reporting "it barely culls here" is describing a real
section rather than a bug.

Two negative results are worth as much as the table. **Modelling the chase
camera barely matters**: putting the camera in a neighbouring section rather
than the craft's own moves the figures by 0 to 5 percentage points, so unioning
two sections and their padding is not what limits this. **The correlation that
does hold is with batch size** - the three best files average 8 sections per
draw call and the two worst 14.1 - which points at the granularity ceiling below
rather than at anything authored.

What none of this establishes is a frame-rate figure. It is not a locked-60
requirement and must not be recorded as one: nobody has measured a frame with
this on, and the ~59 microseconds the second tier costs is not a number this
project is currently struggling against on most hardware. What the table settles
is the narrower question the ordering rests on - if both tiers run at all,
running the mask first removes a fifth to two thirds of the second tier's work
for one integer test per draw call. That is worth having and it is not a
transformation.

### The first tier's own per-frame cost is bounded on purpose

A per-draw mask test is an `and`, but building the frame's visible set means
locating the craft and the camera on the spline, and `Spline::nearest` walks
every one of a track's ~3,400 samples. **Doing that twice a frame would have
cost the same order as the frustum tests it saves**, which would have made the
table above meaningless.

Neither scan happens. The simulation already locates the craft each tick and
leaves the sample index in `Ship::segment`, so the craft's half is a table read.
The camera is a chase spring a few units behind, so it is looked for in a
96-sample window around the craft's index - about thirty-five times cheaper than
the full scan, and safe when it misses, because a miss yields a sample too far
away, which becomes "draw everything" rather than a confident wrong section.

### The association between meshes and sections is invented

Sections attach to spline control points, not to `Mesh` nodes, and no read of
either binary has recovered how the original associates the two. So
`oag_render::pvs` decides it, by intersecting each draw call's bounding sphere
with the authored boxes and keeping every section it reaches.

**Geometry no box touches is never culled.** That is the direction the error has
to point - a misplaced batch pops, an unplaced one costs a frustum test - so
skyboxes and anything outside the partition keep drawing exactly as before.

That the rule places geometry was never evidence it places it **correctly**, and
the first version of it did not: see
[the rejected centre rule](#place-each-draw-call-in-one-section-by-its-centre),
which passed every aggregate check and lost a building. What retired that
question was the screenshot comparison, not the statistics. Recovering the
original's own association would retire it properly; until then the thirty
identical captures are the whole of the evidence, and they are frames from three
tracks rather than a proof.

### The limit is our batch granularity, not the authored data

The artists partitioned each circuit into 64 sections. **Our draw calls each
span 6 to 14 of them** - a `DrawCall` is one material run, and a material used
along the whole circuit produces one batch whose bounding sphere reaches most of
it. That is why the saving is 22-66% rather than the 88% the mean-7.6-of-64
figure suggests, and why it varies so much between files: the spread is
measuring how the art was batched, not how it was partitioned.

So the lever that would unlock the rest is **splitting batches by section at
load**, and nothing to do with the visibility data or the order of the tiers. A
material run cut into per-section runs would give each piece a mask naming one
or two sections, and the first tier would then approach what the authored data
allows. That costs more draw calls and more state changes, a trade with a real
downside, and it should be measured before it is built.

### Anything that can leave the partition must be treated conservatively

A craft that falls off the track, is airborne over a gap, is being reset, or has
crossed into geometry the artists left outside every box, has no trustworthy
section. **Being outside the partition is not the same as looking up an
out-of-range id**: a fallen craft is often still inside some box and will get a
valid, wrong answer, so the all-ones fallback does not fire on its own. The
runtime therefore keys the conservative path off craft state rather than off the
lookup failing, and draws everything while it holds. Correct, and slower exactly
when the picture is least ordinary - which is the right trade, because a
respawning craft with a swinging camera is precisely when culling to a stale
section would be most visible.

### 64 sections is a hard ceiling

From the mask width and the original's 64-entry gather buffer. Anything that
later wants a finer partition than the artists authored cannot get one from this
mechanism.

### Shipped data is not tidy, and the parser must not pretend otherwise

Validating against both discs turned up three things the format page did not
record, each of which would have made a stricter parser refuse real tracks:

- **~2% of mask bits name a section the file does not declare** (2.17% PSP,
  4.01% PS2) - masks that outlived a deleted section. Inert, because nothing
  ever asks about a section that does not exist.
- **One id is authored three times over** on 2 of 40 PSP and 2 of 59 PS2 tracks,
  with byte-identical masks. Unioned rather than treated as corruption.
- **116 of 50,218 PS2 control points name a section their own file lacks**
  (none on PSP). This is the all-ones fallback earning its keep on real data.

These also became the evidence for the field offsets. Rather than assert that
every mask bit resolves - which is false - the ground-truth test measures the
stray rate at the documented offset and four bytes either side, and requires the
documented one to be lowest. It is, decisively: 2.2% against 7.7% and 13.3% on
PSP, 4.0% against 8.7% and 22.9% on PS2.

### This does not carry to Pure

Pure does not share Pulse's `.vex` class numbering - its track files use classes
around `0x36f..0x393` where Pulse uses `0x3b9..0x3e9` - so `CLASS_SECTION`
matches nothing on that disc. Pure tracks will draw with PVS culling off until
its own numbering is recovered. Pinned as a test so the empty result is not
later rediscovered as a parser bug.

### Moving entities need a section each

Rival craft, weapons and effects have to be mapped to a section per frame to be
culled by this mechanism, and none of that exists yet. Until it does they are
outside the first tier entirely, which is safe: they draw.

## References

- [`docs/formats/track.md`](../../formats/track.md) - the `section` payload and
  its confidence scores
- [`crates/formats/src/pvs.rs`](../../../crates/formats/src/pvs.rs) - the decoder
- [`crates/formats/tests/pvs_ground_truth.rs`](../../../crates/formats/tests/pvs_ground_truth.rs) -
  the validation, and where every figure above comes from
- [`crates/render/src/pvs.rs`](../../../crates/render/src/pvs.rs) - the
  association rule and the two-tier test
- [ADR-0004](0004-asset-pipeline.md) - load from originals, which is why there
  is no bake step
