---
categories: [rendering, tooling]
---

# The race-setup previews are meshes on Pulse and HD, sprites on Pure

2026-09-05, corrected 2026-09-10. Split out of the race-box investigation
because it does not depend on the flow plan.

The permanent write-up is
[`docs/formats/race-setup.md`](../../docs/formats/race-setup.md); this thread is
only the work still to do.

## The finding in one line

**Pulse PSP, Pulse PS2 and Wipeout HD draw a rendered 3D mesh for every track
and craft preview. Pure draws a pre-rendered 256x128 sprite for both, shipped
in `FEData.wad`.** Pure's *speed-class* preview is likewise a layered 2D stat
graph.

That the two answers differ is the point. The naive reading - a track
thumbnail - is wrong on Pulse and HD, where authoring a placeholder thumbnail
would have been an invention. It is right on Pure, where wiring a mesh was
one, and was wired for nine hours on 2026-09-10 before the maintainer spotted
it.

## What is already verified

| Title | Track preview | Craft preview |
| --- | --- | --- |
| Pulse PSP | `<location>\FE\forward.vex` / `reverse.vex` | `<team>\<variant>_FE.vex` |
| Pulse PS2 | same names, own geometry | same names, own geometry |
| HD / Fury | `<Model name="TrackModel">` | `<Model name="ShipModel">` |
| Pure | `FEData.wad` sprite, index unrecovered | `FEData.wad` sprite, index unrecovered |

Pulse's and HD's rows are resolved: paths resolve on the real discs, sizes and
the `VEXX` magic were checked, not assumed. See the docs page for the evidence
and the confidence scores.

**Pure's rows are resolved as to *kind* (confidence 97) and unresolved as to
*which entry*.** PPSSPP's `DumpTextures` caught 12 textures while the two
screens were up, and every one matches an `FEData.wad` entry at **RMSE 0** -
craft at indices 126/130/134/138, circuits at 175/176/182/188/193/194. The
entries carry no names, so index is the only handle, and only 10 of the ~52
preview sprites are tied to anything at all.

## Open

- **Which sprite index belongs to which circuit or team is unrecovered.**
  Craft sit three to a group at stride 4 (124-126, 128-130, ... 144-146) and
  circuits three to a group at stride 6 (150-152, ... 192-194). Within a craft
  group the three read as a top, a side and a 3/4 perspective view, and the
  capture drew the third. **Group order is not evidence of list order** -
  assuming it is would repeat the mistake this thread just corrected. Nothing
  is wired on Pure until a mapping is evidenced; the pickers draw no preview
  and say so.
- **Two different members of one circuit group were seen drawn** (175 *and*
  176; 193 *and* 194), so a screen shows more than one image per circuit.
  Whether that is a cycling slideshow, a rotation, or a selection change
  between screenshots is not settled, and it decides what the sprite draw path
  in `oag_ui::picker` has to be.
- **`oag_ui::picker` has no sprite path at all.** Adding one is a real change,
  not a name swap: the preview slot currently takes a mesh.
- **Pure's craft preview call site (`0x08951bd0`,
  `TeamSelection_ApplySelection`) is fully decompiled** - see
  [`race-box-screens.md`](../../docs/ghidra/functions/psp-pure-eu/race-box-screens.md).
  It composes `%s\VR\Phantom.vex` or `%s\Phantom.vex` from
  `*(matched_record + 0x9c)`, gated on two flags on the *screen* object
  (`+0xe1`, `+0xe2`) that trace to a `"Championship"` record's tier byte
  against `3` plus multiplayer/tournament-permission lookups - **not the
  `Class` global**. Confidence 60, not wired. **This is the one preview mesh
  Pure does load**, and it is a special case layered over the sprite, not the
  default path. Different function from the in-race model loader
  (`FUN_08927dac`/`FUN_08927694`).
- **Closed, not open: why neither `ApplySelection` override composes a default
  preview path.** Because there is no default preview mesh. The earlier
  next-step "find the record class that owns the mesh-loading virtual" was
  chasing something that does not exist for these screens, and the `sceIoOpen`
  sweep that found no hits was telling the truth - changing selection
  re-uploads a texture already in memory.
- **What camera the originals frame the *mesh* previews with is still
  unmeasured**, on Pulse PSP and PS2. HD states its own (`OriginX="1220"
  OriginY="412" nearZ="1.0" z="-24.0" RotX="0.4" RotY="-0.5"` on `ShipModel`);
  the PSP titles author no `<Model>` element, so their framing lives in the
  screen class, and `oag_game::preview::orbit_for` is this project's own pick.
  Pure no longer needs an answer here.
- **Whether Pulse's track mesh is the circuit ribbon or something else** has
  not been looked at - the files are 10-23 KB against a real `track.vex`'s
  4.25 MB, so they are purpose-built, but nobody has decoded one and looked.
- **`Top->Ship` is resolved, on Pulse PSP.** Not a static screen widget:
  `TrackSelection_ApplySelection` resolves it as a node inside *each track's
  own* dynamically-created preview scene, replaced every time the selection
  changes, fed the `%s\FE\%s.vex` mesh directly. Confidence 78, static
  decompilation only.

## Next Steps

1. **Recover the sprite-index mapping on Pure** before wiring anything. The
   cheapest route is another `DumpTextures` run that walks the *whole* track
   and team list one entry at a time, screenshotting after each, so every
   index gets tied to a named entry rather than 10 of 52 being tied to nothing.
   Settle the "two members of one group" question in the same pass by holding
   still on one circuit and dumping over several seconds.
2. **Then** add the sprite path to `oag_ui::picker` and wire Pure's two
   screens to it.
3. Camera framing is still unmeasured on both Pulse pressings - `orbit_for`'s
   framing is this project's own pick, not the original's. Settling it needs a
   breakpoint on whatever sets up the preview-scene camera.
4. Settle `TeamSelection_ApplySelection`'s Phantom-model trigger past
   confidence 60: break at `0x08951bd0`, read `+0xe1`/`+0xe2` at the moment
   the model would visibly change, on a save further into the campaign than a
   fresh profile.
5. Do the Pulse-side `Skin.vex` skin-swap check: whether `Skin_ApplyToModel`'s
   four-slot texture swap
   (`docs/ghidra/functions/psp-pulse-usa/ship-skin.md`) applies to
   `ship_FE.vex` the way it does to the in-race hull.

## The check that would have caught this

Raise PPSSPP's internal resolution and look again. It upscales geometry *and*
render targets, but not a fixed-size source image - so a preview that stays
blurry at 5x is a sprite, whatever it looks like at 1x. Per-entity
distinctness proves nothing: a per-team *sprite* is distinct per team too, and
that was the whole basis of the wrong reading. The art here is also
anti-aliased with soft gradients, which PSP hardware cannot produce at all.
