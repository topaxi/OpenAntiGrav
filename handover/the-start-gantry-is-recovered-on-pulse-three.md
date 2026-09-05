# The start gantry's 3-2-1-GO: recovered on Pulse, ruled absent on Pure; two titles to go

A directed four-title feature, Pulse first, then Pure, then HD/Fury, then 2048.
**The Pulse and Pure passes are both done** (2026-09-05) and this thread
carries what HD/Fury and 2048 inherit. Full account, permanent, one page for
both titles:
[start-gantry.md](../docs/rendering/start-gantry.md).

| | Pulse | Pure | HD/Fury | 2048 |
| --- | --- | --- | --- | --- |
| The 3-2-1-GO mechanism | **recovered, 90** | **no track-side gantry, 85** | open | open |
| Which model a mode selects | **none - one model, 85** | **no gantry to select, 85** | four models, chooser untraced | open |
| Placement (slot 8's transform) | **unrecovered** | n/a - no slot 8 exists | unrecovered | untried |
| Wired to `COUNTDOWN_TICKS` | no, and deliberately | n/a | - | - |

## What Pulse turned out to be, in one paragraph

`Data\Environments\321_Go\321Go_StartFinish.vex` holds `3`, `2`, `1` and `GO` as
**four distinct per-vertex UV cells of one mesh node**, and
`321go_NOMIP.tga` (16x32) is a **palette staircase** whose opaque-white pairs sit
at rows 28-29, 26-27 and 24-25 over columns 0-5. The material's own `TEXOFFSET`
track slides one shared offset upward, so each glyph's column crosses its own
white pair at its own moment: `3` at 0.93-1.31 s, `2` at 1.68-2.06 s, `1` at
2.43-2.81 s, then a `u` step at frames 181-182 hands the board to `GO`, whose
column strobes rather than fades. All lettering is geometry; every texture on
the asset is a palette.

Not node animation, not per-node visibility, not a material swap, not an atlas
cell swap - all four were live hypotheses going in.

## What Pure turned out to be, in one paragraph: nothing, on the track side

**Pure ships no track-side start-line gantry at all.** Every one of its 16
circuits' own `TrackStartup.xml` was read; the eight that author
`<Billboard>` entries run slot numbers 1-7 and never reach 8, and none of
Pure's 42 billboards (across all 16 circuits) carries a `location` at all -
Pure's schema is `type`+`color` only, no model reference, full stop. No
gantry-shaped file turns up anywhere else on the disc either (guessed paths,
a `strings` sweep of `BOOT.BIN`), and the static "starting line" geometry
baked into each `track.vex` carries only short 2-key scroll accents, nothing
matching Pulse's 5-key countdown track. The nearest asset - `Data\HUD\
Ready_GO.vex`, the on-screen `<Mode3D>` widget `ui/hud.md` already names -
uses a **different mechanism**: 3 static ribbon mesh nodes, an all-white
varying-alpha texture, and per-node (not per-vertex-cell) `TEXOFFSET` tracks
scrolling a glow along the ribbon. Both halves of "what should generalise"
from the Pulse pass turned out to be Pulse-specific: the mechanism is unused
on Pure, and the packaging (a billboard slot at all) does not exist to
compare.

## What generalises, and what turned out Pulse-specific - now measured, not guessed

**The mechanism does not generalise to Pure, and needs no new code because
Pure's own gantry-adjacent asset does not use it.** `oag_formats::vex::
mesh_tex_transforms` and `oag_render`'s `TexAnims` remain title-agnostic and
correct for whatever *does* use this shape - they are exactly why the check on
Pure was fast (dump a node's UV cells and its texture's palette, no Ghidra, no
emulator) - but Pure's `Ready_GO.vex` carries a `TEXOFFSET` track and does not
build a countdown from it the way Pulse's gantry does.

**The packaging question dissolved rather than resolved**: Pure has no
billboard slot 8, model or colour, so there is no packaging to compare against
Pulse's "one model, whole race, one timeline" versus HD's "four files by
mode". **Check HD's packaging on its own terms** rather than expecting either
Pulse's or Pure's shape.

**Neither Pulse's nor Pure's per-mode-chooser answer should be inherited
either.** Pulse has one gantry model and no chooser (confidence 85, unchanged).
Pure has no gantry model to choose between (confidence 85, new). HD's four
distinct `321go_*.vex` files and its mode-descriptor pointer are still their
own, untraced question.

## What is deliberately not done, and why

**Nothing is placed and nothing is wired to the race clock, on either title.**

- Slot 8's transform is still unrecovered on Pulse - the same open item
  [a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)
  names as its highest priority. Placing the model at the start line would look
  right (the spline knows where that is, and the model is called
  `StartFinish`) and would be invented. The loader report names the file and
  draws nothing. **On Pure this item does not apply**: there is no slot 8 to
  place.
- The asset's `GO` lands at 3.03-3.6 s on its own clock; the measured thrust
  gate is 272 ticks = 4.533 s. That is **~1.5 s / 90 ticks of unexplained
  pre-roll**, and nothing measures it. No alignment was asserted. Unmeasured
  on Pure - there is no track-side timeline to measure it against.
- Whether the engine *plays* the authored track or writes the offset itself per
  countdown state is scored **55** on Pulse, separately from the 90 above. The
  four `+0x88`/`+0x90`/`+0x98`/`+0xa0` wrapper sub-objects and the flag-gated
  accumulator at `0x0890cf34` that
  [billboards.md](../docs/ghidra/functions/psp-pulse-usa/billboards.md) already
  found are a live competing candidate: four state groups for four states.
- **Pure's `Ready_GO.vex` mechanism is measured but not explained.** Its three
  ribbon shapes do not read as digit outlines, so what actually draws `3`,
  `2`, `1`, `GO` on Pure is unresolved and deliberately left that way rather
  than guessed at - see `start-gantry.md`'s Pure section, closing paragraph.

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
  and its billboard-slot line reads "0 naming a model" - the same
  title-agnostic report code Pulse's line uses, correctly silent here because
  the manifest itself says nothing. (Pure pass.)
- [start-gantry.md](../docs/rendering/start-gantry.md), one page covering both
  titles, and the Track startup row in
  [formats/README.md](../docs/formats/README.md). `ui/hud.md` gained a
  pointer at `Ready_GO.vex`'s new measurement. (Both passes.)
- The load report names slot 8 as the start gantry on Pulse and says it is not
  loaded; on Pure the same code path stays silent, correctly, because there is
  nothing to name. (Pulse pass; Pure pass added the ground-truth coverage that
  proves the silence is not an oversight.)

## Open

- **HD/Fury**: the four models are confirmed distinct content by rendering, and
  which one a race instantiates is still the mode-descriptor pointer nobody has
  traced. The *mechanism* question - are HD's glyphs UV cells against a palette
  too, or something else again now that Pure broke the "it must generalise"
  assumption - is separate, unasked, and answerable off `DATA00`/`DATA02` with
  `scripts/psarc.py` plus `oag-view`. **Do not assume either Pulse's or Pure's
  answer going in; both were guessed wrong once already in this thread's own
  "what generalises" section.**
- **2048**: six or seven `321Go_*.vex` by string search only; no archive listing
  has confirmed them the way HD's was.
- Slot 8's transform, on Pulse (and on HD, separately, if HD turns out to have
  one).
- The pre-roll between Pulse's asset `GO` and the measured green.
- Which of the two candidates drives Pulse's offset at runtime.
- Whether Pulse's `Text`/`Arrow` `LoopEnd` of 9.333 s means the sponsor board
  loops during a race while the one-shot nodes do not - read off the file, not
  reasoned about, and not checked against a running game.
- **What actually draws Pure's `3`, `2`, `1`, `GO` glyphs**, on-screen or
  track-side. `Ready_GO.vex`'s three ribbon shapes are measured and do not
  read as digits; nothing else on the disc was found that does either. Left
  open rather than guessed at - see `start-gantry.md`'s Pure section.

## Next Steps

1. **HD/Fury next**, and go in assuming nothing from Pulse or Pure - this
   thread's own "what generalises" section guessed wrong on both counts once
   Pure was actually measured, which is the strongest argument yet for
   measuring HD fresh rather than inheriting either prior answer. Start with
   `DATA00`/`DATA02`'s `/data/billboards/hd_adverts/321go/` - four distinct
   `.vex` files are already confirmed to exist and to differ in byte size
   (`ui/hud.md`'s HUD section); read one of them the same way the Pulse and
   Pure passes read theirs, `oag-view --nodes` then UV cells then the texture
   block, no Ghidra needed for that half.
2. Split HD's two questions and do the cheap one first: the mechanism
   (asset-side, answerable exactly like Pulse and Pure were) versus the mode
   chooser (Ghidra, expensive, already open on two threads named in
   `ui/hud.md`). The mechanism answer may make the chooser question moot for
   this feature, or may not - find out before spending Ghidra time.
3. **2048** last, and confirm its `321Go_2048*.vex` files exist by an actual
   archive listing first - they are string-search-only today, the same gap
   HD's four files closed by being PSARC-listed rather than just grepped.
4. Do **not** place any gantry that turns out to exist. The transform is one
   open item on every title that has a gantry to place; solving it once solves
   it everywhere it applies, and guessing it once puts a wrong picture in
   every title it touches.
