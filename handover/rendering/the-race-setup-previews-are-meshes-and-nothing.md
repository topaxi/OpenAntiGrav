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
| Pure | unresolved | call site found 2026-09-08, camera/wiring still open |

## Open

- **Nothing in this build draws either preview.** `assets/ui/menu.toml`'s
  `race` page has no preview at all, and neither does `remix`.
- **Pure's craft preview call site is found, 2026-09-08** (spun off a
  `wipeout-pure-races-on-pulses-physics-and-what.md` session that landed on it
  while reading `psp-pure-eu`): `get_xrefs_to 0x08a76654` (the `%s\Phantom.vex`
  copy beside `%s\VR\Phantom.vex` at `0x08a76664`) returns one caller,
  `FUN_08951bd0`. It calls `Screen_FindElementByPath` (the same helper
  `race-box-screens.md` already names on Pulse), matches the selection against
  a list by `strcasecmp` on a `+0x7c` field, then composes `%s\VR\Phantom.vex`
  or `%s\Phantom.vex` from `*(matched_record + 0x9c)` gated on two byte flags
  on the *screen* object itself (`+0xe1`, `+0xe2`) - not on the craft's speed
  class. **`+0x9c` being the team `location` string is inferred by the same
  `+0x7c`/`+0x9c` field-pair pattern
  `wipeout-pure-races-on-pulses-physics-and-what.md`'s session found on the
  in-race side, not independently confirmed here with `read_memory`** -
  confidence 65 pending that. If the file the screen composes doesn't resolve,
  it falls back to unlock/livery UI (marks a `Livery` element's flags), not to
  `Ship.vex` - a different fallback than the in-race loader
  (`ship-models.md`) uses. **This is a different function from the in-race
  model loader** (`FUN_08927dac`/`FUN_08927694`, see that thread) - two
  separate Phantom-selection code paths, one per screen/context, not one
  shared mechanism. Full decompile is not yet written up on any docs page -
  re-run `get_xrefs_to 0x08a76654` on `psp-pure-eu` then `decompile_function`
  on the one caller it returns to reproduce it, settle the `+0x9c` question
  above, and write it up before wiring anything. For the track preview,
  nothing was found at all.
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
3. **Partly done, 2026-09-08**: the function referencing `0x08a76654`
   (`FUN_08951bd0` on `psp-pure-eu`) is found and decompiled - see the Open
   item above for what it shows and what's still unsettled (the `+0x9c` field
   identity, a docs page). A PPSSPP capture of `Team Selection`
   (`docs/reverse-engineering/ppsspp-debugger.md`) is still the cheaper way to
   settle the *track* preview, which this session did not touch.
4. Only then wire anything. A preview needs a camera, and the camera is the
   part that is not yet measured on the PSP titles.
