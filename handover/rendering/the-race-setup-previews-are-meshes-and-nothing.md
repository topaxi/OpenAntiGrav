---
categories: [rendering, tooling]
---

# The race-setup previews are meshes, and nothing draws them

2026-09-05. Split out of the race-box investigation because it does not depend
on the flow plan: the assets are located, verified and in a format
`oag-render` already draws. This is pickup-able on its own.

The permanent write-up is
[`docs/formats/race-setup.md`](../../docs/formats/race-setup.md); this thread is
only the work still to do.

## The finding in one line

**Every track and craft preview on the race-setup screens is a rendered 3D
mesh** - not a flat 2D map, not a prerendered image - on Pulse PSP, Pulse PS2
and Wipeout HD alike. The one exception is Pure's *speed-class* preview, which
is a layered 2D stat graph.

That matters because the naive reading, and the one the maintainer's
from-play description allows, is a track thumbnail. Authoring a placeholder
thumbnail would have been exactly the invention CLAUDE.md's "never invent what
the assets already author" forbids, and it is now avoidable.

## What is already verified

Paths resolve on the real discs; sizes and the `VEXX` magic were checked, not
assumed. See the docs page for the full evidence and the confidence scores.

| Title | Track preview | Craft preview |
| --- | --- | --- |
| Pulse PSP | `<location>\FE\forward.vex` / `reverse.vex` | `<team>\<variant>_FE.vex` |
| Pulse PS2 | same names, own geometry | same names, own geometry |
| HD / Fury | `<Model name="TrackModel">` | `<Model name="ShipModel">` |
| Pure | unresolved | unresolved, a candidate exists |

## Open

- **Nothing in this build draws either preview.** `assets/ui/menu.toml`'s
  `race` page has no preview at all, and neither does `remix`.
- **Pure's two previews are unresolved.** For the craft, `%s\Phantom.vex` and
  `%s\VR\Phantom.vex` sit inside the `fe::TeamSelection_Screen`..`TeamSelection`
  string span in `psp-pure-eu/BOOT.BIN`, and `Data\Ships\Feisar\Phantom.vex`
  resolves in `Data.wad` - but no xref was recovered, so the association is
  string-pool adjacency alone. Confidence 45. **Do not build on it.** For the
  track, nothing was found at all.
- **What camera the originals frame these meshes with is still unmeasured.**
  HD states its own (`OriginX="1220" OriginY="412" nearZ="1.0" z="-24.0"
  RotX="0.4" RotY="-0.5"` on `ShipModel`); the two PSP titles author no
  `<Model>` element, so their framing lives in the screen class. Pulse's own
  `TrackSelection`/`TeamSelection` classes are now decompiled
  (`docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`), but the camera
  itself was not specifically traced there - the functions read cover mesh
  loading and stat-panel binding, not a camera setup call. A framing picked
  by eye would still be ours, and would have to say so.
- **Whether the track mesh is the circuit ribbon or something else** has not
  been looked at - the files are 10-23 KB against a real `track.vex`'s 4.25 MB,
  so they are purpose-built, but nobody has decoded one and looked.
- **`Top->Ship` is resolved, on Pulse PSP.** It is not a static screen widget:
  `TrackSelection_ApplySelection` resolves it as a node inside *each track's
  own* dynamically-created preview scene, replaced every time the selection
  changes, fed the `%s\FE\%s.vex` mesh directly. See
  [`race-box-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md#top-ship-has-a-confirmed-referent-after-all),
  confidence 78, static decompilation only.

## Next Steps

1. **Decode one and look at it.** `oag-view` already opens a `.vex`:
   `just view 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' --mesh 'Data\Environments\16_Track\FE\forward.vex' --screenshot /tmp/fe-fwd.png`
   (single backslashes are correct when running `oag-wad`/`oag-view` through
   cargo directly; `just` eats one layer and needs them doubled). That answers
   "what is this mesh" in one command and is the cheapest next action here.
2. Do the same for `Data\Ships\Assegai\ship_FE.vex`, and check whether
   `Skin_ApplyToModel`'s four-slot texture swap
   (`docs/ghidra/functions/psp-pulse-usa/ship-skin.md`) applies to it as it
   does to the in-race hull. If it does, the livery cycler and the preview are
   one mechanism.
3. Settle Pure's craft preview, either by capturing `Team Selection` in PPSSPP
   (`docs/reverse-engineering/ppsspp-debugger.md`) or by decompiling the
   function that references `0x08a76654`. **The PSP relocation patch landed
   2026-09-07 across all four PSP databases, both Pure ones included**
   (`HANDOVER.md`, "Traps that are live"), so this decompile is unblocked and
   not yet attempted. A capture is still far cheaper and settles the track
   preview at the same time.
4. Only then wire anything. A preview needs a camera, and the camera is the
   part that is not yet measured on the PSP titles.
