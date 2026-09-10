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
| Pure | `<location>\track.vex` - confirmed rendered live 2026-09-10, composer not located | `<location>\Ship.vex` - same |

**Both Pure rows are "confirmed rendered, composer not located", not
"resolved" the way Pulse's rows are** - a PPSSPP capture (2026-09-10) proved
both screens draw a real per-entity 3D preview, but neither
`TeamSelection_ApplySelection` nor `TrackSelection_ApplySelection`
(`docs/ghidra/functions/psp-pure-eu/race-box-screens.md`) composes the
*default* path in its own decompiled body, and a live `sceIoOpen` breakpoint
swept nothing when changing selection (assets stream from an already-open
archive). `<location>\Ship.vex` / `<location>\track.vex` are wired because
they are the only per-entity files that exist on disc and visually match -
confidence ~70, evidenced by existence-plus-appearance rather than by a
located call site.

## Open

- **Both previews now draw, on Pulse and on Pure**, through
  `oag_ui::picker`/`oag_game::picker_stage` - `assets/ui/menu.toml`'s `race`
  page still has none, and neither does `remix`, but that is a separate
  screen from the pickers this thread is about.
- **Pure's craft preview call site (`0x08951bd0`,
  `TeamSelection_ApplySelection`) is fully decompiled, 2026-09-10** - see
  [`race-box-screens.md`](../../docs/ghidra/functions/psp-pure-eu/race-box-screens.md).
  It composes `%s\VR\Phantom.vex` or `%s\Phantom.vex` from
  `*(matched_record + 0x9c)`, gated on two flags on the *screen* object
  (`+0xe1`, `+0xe2`). **This session traced where those two flags come from**
  (`0x08951e58`, the screen's own list-refresh routine): a
  `"Championship"`-named record's tier byte compared against `3`, plus
  multiplayer/tournament-permission lookups - **not the `Class` global at
  all**, which that function never reads. So the earlier "not on the craft's
  speed class" note was right, and the likelier reading is now "is
  Phantom-class content available to this profile in this context", not
  "is Phantom the selected class" - still confidence 60, still not settled
  precisely, and still not wired. If the file the screen composes doesn't
  resolve, it falls back to unlock/livery UI (marks a `Livery` element's
  flags), not to `Ship.vex` - a different fallback than the in-race loader
  (`ship-models.md`) uses. **This is a different function from the in-race
  model loader** (`FUN_08927dac`/`FUN_08927694`, see that thread) - two
  separate Phantom-selection code paths, one per screen/context, not one
  shared mechanism.
- **Neither `ApplySelection` override composes the *default* (non-Phantom)
  path**, on either screen, on Pure. Both end by calling a shared base method
  (`0x088b5700`) that turned out, on decompile, to be generic child-load-
  progress bookkeeping with no file composition in it either. The actual
  loader is one level further down - most likely triggered by the
  selection-changed notification each override sends to the matched record
  object, handled by a third class (the list record, not the screen) that
  was not located. A live `sceIoOpen` breakpoint sweep across a
  `down`-triggered selection change found no hits (assets stream from an
  already-open archive, not a discrete per-file open), so this needs a
  breakpoint on the archive's own read primitive to close, not attempted.
- **The track preview is confirmed real by PPSSPP capture, 2026-09-10**,
  correcting this thread's own working assumption from earlier in that same
  session (a decompile of `TrackSelection_ApplySelection` alone, with no
  capture yet, read as "nothing loads here" - true of that one function,
  not true of the screen). See `docs/formats/race-setup.md`'s "Captured live
  in PPSSPP, 2026-09-10 (Pure)" for the screenshot description.
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

1. **Done.** Both pickers are wired on Pure, reusing the same
   `oag_ui::picker`/`oag_game::picker_stage`/`oag_game::preview` machinery
   Pulse's do, with `<location>\Ship.vex` / `<location>\track.vex` as the
   preview convention and the same `orbit_for` framing Pulse's screens use
   (chosen, not measured, on both titles alike - see that function's own doc
   comment).
2. **Camera framing is still unmeasured on every PSP title**, Pure included
   - `orbit_for`'s framing is this project's own pick, not the original's.
   Settling it needs the same kind of live trace this session tried for the
   mesh path and did not land (a breakpoint on whatever sets up the
   preview-scene camera, not attempted).
3. Find the record class that owns the mesh-loading virtual (see the Open
   item above) to replace the existence-plus-appearance evidence for
   `Ship.vex`/`track.vex` with a located composer. A breakpoint on the
   archive's read primitive rather than `sceIoOpen` is the next thing to try,
   not a re-read of the two `ApplySelection` overrides - those are fully
   decompiled and confirmed not to hold it.
4. Settle `TeamSelection_ApplySelection`'s Phantom-model trigger past
   confidence 60: break at `0x08951bd0`, read `+0xe1`/`+0xe2` at the moment
   the model would visibly change, on a save further into the campaign than
   a fresh profile (a fresh profile may never make the flags true, which
   would explain why nothing in this session's capture ever showed the
   Phantom hull).
5. Do the Pulse-side `Skin.vex` skin-swap check this list used to open with:
   whether `Skin_ApplyToModel`'s four-slot texture swap
   (`docs/ghidra/functions/psp-pulse-usa/ship-skin.md`) applies to
   `ship_FE.vex` the way it does to the in-race hull. Still untouched by this
   thread's Pure work.
