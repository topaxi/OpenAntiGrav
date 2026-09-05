# The start gantry's 3-2-1-GO is recovered on Pulse; three titles to go

A directed four-title feature, Pulse first, then Pure, then HD/Fury, then 2048.
**The Pulse pass is done** (2026-09-05) and this thread carries what the other
three inherit. Full account, permanent:
[start-gantry.md](../docs/rendering/start-gantry.md).

| | Pulse | Pure | HD/Fury | 2048 |
| --- | --- | --- | --- | --- |
| The 3-2-1-GO mechanism | **recovered, 90** | open | open | open |
| Which model a mode selects | **none - one model, 85** | open | four models, chooser untraced | open |
| Placement (slot 8's transform) | **unrecovered** | unrecovered | unrecovered | untried |
| Wired to `COUNTDOWN_TICKS` | no, and deliberately | - | - | - |

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

## What generalises, and what looked Pulse-specific

**The mechanism should generalise, and needs no new code to play.** Both halves
are already implemented and title-agnostic: `oag_formats::vex::mesh_tex_transforms`
parses the `TEXOFFSET` block big-endian for HD as readily as little-endian for
Pulse, and `oag_render`'s `TexAnims` replays it per frame. A later pass should
be able to answer "how does this title do it" by dumping one node's UV cells and
one texture's texel grid - no Ghidra, no emulator. The Pulse pass used exactly
two tools: `oag-wad cat` and `oag-view --nodes/--draws/--only/--anim-seconds`.

**The packaging looked Pulse-specific.** One model holds the whole race's gantry
states - countdown, then the sponsor board, then FINAL LAP, then chequered - as
non-overlapping windows on one 60 Hz timeline (frames 0-358, 350-435, 560-741,
740+). HD splits the equivalent across four files by *mode*, which is a
different axis entirely. Pulse's own manifest shows it used to split by *state*
too: `14_Track` carries two `#`-prefixed `<Billboard num="8">` lines after its
root close naming `Checkered_StartFinish.vex` and `Final_Lap_StartFinish.vex`,
neither of which is on the disc. **Check the packaging on Pure and 2048 before
assuming either shape.**

**Pulse has no per-mode model and no chooser to find.** A `strings` sweep of all
three WADs and of `BOOT.BIN` turns up one gantry name, and Zone on Pulse runs
`26_Track`, which authors no `TrackStartup.xml` at all. Do not inherit HD's
four-model shape onto an earlier title.

## What is deliberately not done, and why

**Nothing is placed and nothing is wired to the race clock.**

- Slot 8's transform is still unrecovered - the same open item
  [a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)
  names as its highest priority. Placing the model at the start line would look
  right (the spline knows where that is, and the model is called
  `StartFinish`) and would be invented. The loader report names the file and
  draws nothing.
- The asset's `GO` lands at 3.03-3.6 s on its own clock; the measured thrust
  gate is 272 ticks = 4.533 s. That is **~1.5 s / 90 ticks of unexplained
  pre-roll**, and nothing measures it. No alignment was asserted.
- Whether the engine *plays* the authored track or writes the offset itself per
  countdown state is scored **55**, separately from the 90 above. The four
  `+0x88`/`+0x90`/`+0x98`/`+0xa0` wrapper sub-objects and the flag-gated
  accumulator at `0x0890cf34` that
  [billboards.md](../docs/ghidra/functions/psp-pulse-usa/billboards.md) already
  found are a live competing candidate: four state groups for four states.

## What landed

- `oag_formats::trackstartup` reads billboards from inside `<TrackStartup>`
  only, so `14_Track`'s post-root debris no longer makes it the one Pulse
  manifest whose `num` is not unique. Two unit tests.
- `crates/formats/tests/start_gantry_ground_truth.rs`, four tests, no GPU: the
  four UV cells, the palette staircase, one digit lit per phase in order, and
  `GO`'s strobe.
- [start-gantry.md](../docs/rendering/start-gantry.md), and the Track startup
  row in [formats/README.md](../docs/formats/README.md).
- The load report names slot 8 as the start gantry and says it is not loaded.

## Open

- **Pure**: nothing measured. Its `TrackStartup.xml` was never read for content
  (`docs/formats/pure-status.md` reached the plugin id and stopped), and the
  race-start thread records that `psp-pure-usa` was unreachable through
  `ghidra-mcp` in an earlier session - but the Pulse pass needed no Ghidra at
  all, so try the disc first.
- **HD/Fury**: the four models are confirmed distinct content by rendering, and
  which one a race instantiates is still the mode-descriptor pointer nobody has
  traced. The *mechanism* question - are HD's glyphs UV cells against a palette
  too - is separate, unasked, and answerable off `DATA00`/`DATA02` with
  `scripts/psarc.py` plus `oag-view`.
- **2048**: six or seven `321Go_*.vex` by string search only; no archive listing
  has confirmed them the way HD's was.
- Slot 8's transform, on every title.
- The pre-roll between the asset's `GO` and the measured green.
- Which of the two candidates drives the offset at runtime.
- Whether Pulse's `Text`/`Arrow` `LoopEnd` of 9.333 s means the sponsor board
  loops during a race while the one-shot nodes do not - read off the file, not
  reasoned about, and not checked against a running game.

## Next Steps

1. **Pure, and start on the disc.** `oag-wad cat` a circuit's
   `TrackStartup.xml`, see whether slot 8 names a gantry, dump its node tree
   with `oag-view --nodes`, then its UV cells and its palette. If Pure's
   glyph node has four UV cells sharing a `v`, the mechanism is title-wide and
   the Pure pass is a doc page plus a ground-truth test.
2. If Pure's differs, say how before generalising anything: the Pulse page's own
   "what generalises" section is a hypothesis for three titles and a
   measurement for one.
3. **HD/Fury** third, and split the two questions it carries - the mechanism
   (asset-side, cheap) from the mode chooser (Ghidra, expensive, already open on
   two threads). Doing the cheap one first may make the expensive one
   unnecessary for this feature.
4. Do **not** place any of them. The transform is one open item across all four
   titles; solving it once solves it everywhere, and guessing it once puts a
   wrong picture in four places.
