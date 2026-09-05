# The start gantry's 3-2-1-GO: recovered on three titles; 2048 to go

A directed four-title feature, Pulse first, then Pure, then HD/Fury, then
2048. **The Pulse, Pure and HD/Fury passes are all done** (2026-09-05) and
this thread carries what 2048 inherits. Full account, permanent, one page for
all three titles: [start-gantry.md](../docs/rendering/start-gantry.md).

| | Pulse | Pure | HD/Fury | 2048 |
| --- | --- | --- | --- | --- |
| The 3-2-1-GO mechanism | **recovered, 90** | **no track-side gantry, 85** | **recovered, 88** (packaging) / **40** (runtime) | open |
| Which model a mode selects | **none - one model, 85** | **no gantry to select, 85** | **substitution site found at slot 7, untraced - capped 84** | open |
| Placement (slot 8's transform) | **unrecovered** | n/a - no slot 8 exists | **unrecovered - second, independent witness, 90** | untried |
| Wired to `COUNTDOWN_TICKS` | no, and deliberately | n/a | not attempted | - |

## What HD/Fury turned out to be, in one paragraph

HD ships four distinct `321Go_*.vex` files, not Pulse's one, split across
`DATA00.PSARC`/`DATA02.PSARC` in a way that is itself uneven (`321go_zone.vex`
exists in *both*, at two different sizes). Its geometry lives in a sibling
`.rcsmodel`, not the `.vex` - a PS3-wide fact, not specific to this asset. The
glyph node (`pasted__Go_HD_start_light_321go`) authors **five** UV cells
against a 64x128 DXT-compressed texture that is Pulse's own 16x32
staircase-and-marker-column layout, 4x scaled and moved onto a diagonal;
adding the material's own static `u` offset (`0.04`) lands four of the five
cells' `x` positions in the same reading order the texture's three diagonal
white bands run in - strong evidence of the same authored concept, at
confidence 88. **What does not generalise is the encoding**: the offset that
would select a state is a static material parameter with no time axis at all
in this file format (`quads == 1` on every entry, confirmed against the same
parser that recovers Pulse's animated `TEXOFFSET` track and finds nothing
here) - and that same material, by name, backs at least nine other, ordinary,
single-state advert boards disc-wide, so a static value here is not on its
own evidence of anything special. Whether the countdown swaps states at
runtime is unresolved and scored low (40) on purpose: if it does, the write
has to land in this exact parameter, but nothing here caught that write
happening, and Pulse at least has an authored track to weigh against a
competing write where HD does not even have that much.

**The best result of the sequence is independent of all of the above**: HD's
glyph node teleports ~+10 in world Y at the exact same 6.000-second
loop-closing instant Pulse's own countdown panel does (Pulse: ~+9.99 at
frames 360/361; HD: +10.004 at frames 359/360) - two different platforms, two
different file formats, the same instant, neither one predicted going in.

## What Pulse and Pure turned out to be, in one paragraph each

**Pulse**: `Data\Environments\321_Go\321Go_StartFinish.vex` holds `3`, `2`,
`1` and `GO` as four distinct per-vertex UV cells of one mesh node, and
`321go_NOMIP.tga` (16x32) is a palette staircase whose opaque-white pairs sit
at rows 28-29, 26-27 and 24-25 over columns 0-5. The material's own
`TEXOFFSET` track slides one shared offset upward, so each glyph's column
crosses its own white pair at its own moment, then a `u` step hands the board
to `GO`, whose column strobes. Not node animation, not per-node visibility,
not a material swap, not an atlas cell swap.

**Pure**: ships no track-side start-line gantry at all. Every one of its 16
circuits' own `TrackStartup.xml` was read; none reaches billboard slot 8 and
none of its 42 billboards carries a `location`. The nearest asset,
`Data\HUD\Ready_GO.vex` (the on-screen HUD widget, not this page's subject),
uses a different mechanism again: static ribbon nodes, an all-white varying-
alpha texture, per-node `TEXOFFSET` tracks.

## What generalises across three titles, and what is per-title

**Generalises**: the model name and its internal node names
(`polySurface7`, `start_light_background`, `Final_Lap`, `start_light_321go`,
under whatever per-title prefix each export adds); the "one asset carries the
whole race's states" design; an authored countdown built from a palette
texture with unused marker columns baked in and never read; and, new this
pass, a hard ~10-unit Y move of the countdown glyph at the exact same
6.000-second loop-closing instant - independently authored on Pulse and HD.
**Placement generalises as an absence**: three titles, three different
geometry formats, and nothing anywhere reads a position, rotation or scale
for the gantry.

**Per-title**: whether slot 8 exists at all (Pure: no); how many files answer
it (Pulse: one: HD: four, unevenly archived); where the geometry lives
(embedded on Pulse, a sibling `.rcsmodel` on HD); and whether the state
selector is an authored, played-back track (Pulse, 55 on the "played" half) or
would have to be an engine-side write because the format has no room for a
track at all (HD, 40, weaker evidence than Pulse's because nothing here
caught either title's engine in the act).

## What is deliberately not done, and why

**Nothing is placed and nothing is wired to the race clock, on any of the
three titles.**

- Slot 8's transform is still unrecovered on Pulse and now, independently, on
  HD too - `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s own reading
  (nothing in `TrackStartup_Load` or either constructor reads a position) and
  this pass's geometry-side reading (the glyph's `.rcsmodel` chunk carries a
  bias near the origin, the same "node-local" shape every other HD advert
  takes) agree by two completely different routes. The loader report has
  nothing to name for HD yet because nothing here loads `321go_startfinish.vex`
  at all - that is next work, not this pass's.
- HD's slot-7 substitution (`num == 7`, gated on `mode_descriptor`) is a
  real, code-confirmed site, but it fires on slot 7 (`fx350.vex`), not slot 8
  - so the countdown asset itself loads unconditionally regardless of mode,
  and whether the four `321Go_*.vex` names are really what the substituted
  pointer resolves to is still a lead, not a finding, capped at 84 by that
  page's own static-reading ceiling. No new Ghidra or live-tracing time was
  spent chasing it this pass, on purpose - the mechanism question was the
  cheap one and needed answering first.
- Whether HD's countdown display changes at runtime at all is unresolved
  (confidence 40) - see the HD section above and
  `docs/rendering/start-gantry.md`'s own account. Settling it needs the same
  live-tracing step `billboards.md` already named for `mode_descriptor`
  itself; this pass stayed off Ghidra and RPCS3 entirely, per the thread's own
  priority order.
- HD's two archive copies of `321go_zone.vex` (8,368 B in `DATA00.PSARC`,
  2,672 B in `DATA02.PSARC`, each with its own texture) are recorded as a
  measured fact and not traced to which mode loads which.

## What landed

- `oag_formats::trackstartup` reads billboards from inside `<TrackStartup>`
  only, so `14_Track`'s post-root debris no longer makes it the one Pulse
  manifest whose `num` is not unique. Two unit tests. (Pulse pass.)
- `crates/formats/tests/start_gantry_ground_truth.rs`, four tests, no GPU: the
  four UV cells, the palette staircase, one digit lit per phase in order, and
  `GO`'s strobe. (Pulse pass.)
- `crates/formats/tests/start_gantry_pure_ground_truth.rs`, two tests, no GPU:
  reads all 16 circuits' own `TrackStartup.xml` and asserts none authors a
  slot 8 and none names a billboard `location`. (Pure pass.)
- `crates/game/tests/start_gantry_report_pure_ground_truth.rs`, one test:
  Pure's default track's load report never says "slot 8 is the start gantry"
  and its billboard-slot line reads "0 naming a model". (Pure pass.)
- `crates/hd/tests/start_gantry_hd_ground_truth.rs`, four tests, no GPU: the
  five UV cells and their alignment to the texture's diagonal staircase once
  the static offset is added; the texture's own band/marker-column layout;
  the negative that no texture-transform track exists anywhere on the node
  (checked with the same parser that finds Pulse's); and the 6.000 s
  loop-close teleport. All four pass against `hdfury-ps3-eu-dec.iso`. (HD
  pass.)
- [start-gantry.md](../docs/rendering/start-gantry.md), one page covering all
  three titles now, and the Track startup row in
  [formats/README.md](../docs/formats/README.md). `ui/hud.md` gained a
  pointer at `Ready_GO.vex`'s measurement. (All three passes.)
- The load report names slot 8 as the start gantry on Pulse and says it is not
  loaded; on Pure the same code path stays silent, correctly. HD's loader
  report gained nothing this pass - see "what is deliberately not done".

## Open

- **2048**: six or seven `321Go_*.vex` by string search only; no archive
  listing has confirmed them the way HD's four were. Confirm existence via a
  real archive listing before reading anything else.
- Slot 8's transform, on Pulse and on HD - the same open question, now with
  two title's worth of "nothing reads a position" evidence behind it.
- The pre-roll between Pulse's asset `GO` and the measured green (272 ticks =
  4.533 s against the asset's own 3.03-3.6 s `GO` onset).
- Which of the two candidates drives Pulse's offset at runtime, and whether
  HD's countdown changes at runtime at all - both need the same kind of
  live-tracing step, on two different platforms.
- Whether Pulse's `Text`/`Arrow` `LoopEnd` of 9.333 s means the sponsor board
  loops during a race while the one-shot nodes do not.
- **What actually draws Pure's `3`, `2`, `1`, `GO` glyphs**, on-screen or
  track-side. `Ready_GO.vex`'s three ribbon shapes are measured and do not
  read as digits.
- HD's `mode_descriptor` shape and whether its `field_0x4c + 0xf0` pointer
  really does resolve to one of the four `321Go_*.vex` names - a lead, not a
  finding, per `billboards.md`.
- HD's two-copy `321go_zone.vex` split (`DATA00` 8,368 B against `DATA02`
  2,672 B) - which archive a given mode actually reaches is unread.

## Next Steps

1. **2048 last.** Confirm its `321Go_2048*.vex` files exist by an actual
   archive listing first - they are string-search-only today, the same gap
   HD's four files closed by being PSARC-listed rather than just grepped.
2. Go in assuming nothing from any of the first three titles individually -
   read the table at the top of `start-gantry.md` for what actually holds
   (model/node names, one-asset-whole-race design, unused marker columns, the
   6.000 s loop-close teleport, placement-as-absence) versus what is
   genuinely per-title (file count, geometry location, state-selector
   encoding). 2048's engine is closest to HD/Fury's in lineage, so its own
   `.vex`/`.rcsmodel` split and material system are the more likely starting
   hypothesis than Pulse's - but that is a hypothesis to check, not an answer
   to inherit, the same lesson this thread has now learned twice.
3. Check for the same 6.000-second loop-close teleport shape on 2048's own
   glyph node first - it is the cheapest, highest-signal check available
   (one `Anim Transform` channel, two keys, no Ghidra) and it is the one
   result from this sequence that has held on every title with a gantry to
   check it against.
4. Do **not** place any gantry that turns out to exist. The transform is one
   open item on every title that has a gantry to place; solving it once solves
   it everywhere it applies, and guessing it once puts a wrong picture in
   every title it touches.
5. If 2048's own countdown mechanism looks like HD's (static parameter, no
   on-disk track), the runtime-write question is now open on *two* titles at
   once - worth a single live-tracing session (a live PSP debugger, the way
   [`psp-pulse-usa/billboards.md`](../docs/ghidra/functions/psp-pulse-usa/billboards.md)
   used one, or RPCS3 watchpoints on the PS3/Vita side) that answers both
   rather than two separate ones.
