# `TrackSelection` and `TeamSelection`, decompiled

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pure PSP, EU pressing), read 2026-09-10
following [`docs/formats/race-setup.md`](../../../formats/race-setup.md)'s "Pure
differs in shape, not only in value" section and its "Captured live in
PPSSPP, 2026-09-10 (Pure)" subsection.
Mirrors `psp-pulse-usa/race-box-screens.md`'s method (vtable-slot comparison
between sibling screen classes) against a different binary; nothing here reuses
that page's addresses, which are Pulse's own.

**The relocation trap `string-anchors.md` documents did not bite this pass.**
Every `decompile_function` call below printed a resolved string literal
directly (`"%s\\VR\\Phantom.vex"`, not a raw pre-relocation constant), which is
only possible if the import's code immediates are correctly relocated. Either
the 2026-09-07 Allegrex relocation patch (`HANDOVER.md`, "Traps that are live")
was applied to this database too, or this particular import was never affected.
Not independently re-verified against the trap's own probes; noted here so the
next reader does not have to re-derive it from scratch.

## Finding the two `ApplySelection` overrides

`fe::TeamSelection_Screen` (string at `0x08a7664c`) and
`fe::TrackSelection_Screen` (`0x08a767d8`) each name a class the same way
Pulse's do. Their constructors -
`FUN_089523e4` (`TeamSelection`, vtable at `0x08abd3bc`) and `FUN_08953230`
(`TrackSelection`, vtable at `0x08abd5cc`) - were found the same way: `search_strings`
for the class name, `get_xrefs_to` the short form (`"TeamSelection"` /
`"TrackSelection"`, not the `fe::`-qualified one), one caller each.

Both vtables were read directly (`read_memory`, 256 bytes) and compared word
for word. They agree at every slot except four (word indices 9, 25, 27, 29,
and the self-referential RTTI pointer at index 1, which differs because it
points into itself). Word index 9 is the pair this page is about:

| Class | Word 9 (its `ApplySelection` override) |
| --- | --- |
| `TeamSelection` | `0x08951bd0` |
| `TrackSelection` | `0x08952fa4` |

Both take `(this, screen_element)` and end by calling the shared base method
at `0x088b5700` unconditionally - confirmed by decompiling it: it only
recomputes a child-loading-progress fraction (via `0x088b5800`, itself a
generic recursive tree walk with no path composition in it) and flips display
bits accordingly. **Neither the base method nor its callees compose or open
any file** - whatever loads the mesh a player actually sees (both screens do
show one; see "Corroboration" below) is not in this call chain. It is most
likely triggered by the selection-changed notification each override sends
to the matched *record* object (`0x088d7a2c`/`0x088d88a8`, passing short
strings like `"Track"`/`""` found at `0x08a76640`/`0x08a7664c` and
`0x08a767fc`/`0x08a76808`) - i.e. handled by a third class (the list record,
not the screen) that was not located this pass.

| Address | Name | Conf | Evidence |
| --- | --- | ---: | --- |
| `0x08951bd0` | `TeamSelection_ApplySelection` | 78 | decompiled directly, vtable-slot correspondence with `TrackSelection`'s sibling |
| `0x08952fa4` | `TrackSelection_ApplySelection` | 78 | same |

## `TeamSelection_ApplySelection`'s Phantom branch, and a correction

The function composes `"%s\\VR\\Phantom.vex"` or `"%s\\Phantom.vex"` (`%s`
being `*(matched_record + 0x9c)`, read as the team's own `location` string by
the same field-pair pattern used elsewhere in this codebase) and probes it
with `0x0889e308` (a "does this resolve" check, not named). This is the
craft candidate `the-race-setup-previews-are-meshes-and-nothing.md` already
flagged at confidence 45→65 from string adjacency; this pass reproduces the
same composition from a full decompile rather than adjacency alone.

**That thread already correctly noted the gating flags (`+0xe1`, `+0xe2` on
the screen object) are not the craft's speed class.** This pass traced where
they come from - `0x08951e58`, the screen's list-refresh routine, called on
entering the screen and again from the top of `ApplySelection` itself. It
computes them from campaign-progress and multiplayer/tournament-permission
checks (a `"Championship"`-named record's tier byte compared against `3`, a
`"Tournament"`/`"Track"` unlock lookup depending on mode) - **not from the
`Class` global the `Class Selection` menu writes**, which this function never
reads at all. So the composition is closer to "is Phantom-class content
available to this profile in this context" than "is Phantom the currently
selected class" - still not settled precisely, and still below the bar to
wire. Confidence 60 for "this is a progress/permission gate, not a
class-selection gate"; the exact trigger condition stays open.

`TrackSelection_ApplySelection` has no equivalent branch at all - it only
matches the selection against the track list and sends the two notifications
above. No `%s\...vex` composition anywhere in it.

## Corroboration: both screens render a real per-entity 3D preview

Captured live, `pure-psp-eu.chd` under PPSSPP v1.20.4 (SDL build, Xvfb),
2026-09-10, via the websocket debugger (`scripts/ppsspp_debugger.py`) driving
input and `import -window root` for screenshots - the same tool class this
project's other captures use, no dedicated driver script written for Pure's
menu chain this pass (blind `cross`/`down`/`right` taps with screenshots
after each, not a state-name-verified walk: Pure's `G_STATE_MACHINE` address
is unmeasured, unlike Pulse's).

- **`Track Selection`** on Vineta K: a wireframe-outline track spine, plus
  `RACE RECORD`/`LAP RECORD`/`LENGTH 4446 M`/`HEIGHT 222 M`. Confidence 95
  this screen has a real, working 3D preview (a screenshot is not a static
  reading) - directly contradicts this page's own earlier working
  hypothesis (this pass, before the capture) that the screen shows nothing.
- **`Team Selection`**: a distinct wireframe-outline ship per team, checked
  on Feisar/Auricom/Qirex - three different silhouettes, matching the three
  teams' own `Ship.vex` files existing and differing in size on disc
  (60416/73920/61664 bytes, `oag-wad cat` against `pure-psp-eu.chd`) - plus
  `SPEED`/`HANDLING`/`SHIELD`/`THRUST` bars. The model appears to rotate
  slowly on its own, the same shape as this project's existing `Orbit`/
  `orbit_for` mechanism already built for Pulse's pickers.
- **A `sceIoOpen` breakpoint sweep found nothing** when changing selection on
  either screen (all four `zz_sceIoOpen` stub addresses armed, `down` pressed,
  6 s window). Assets stream from an already-open archive rather than a
  discrete per-file open, so this technique cannot localise the exact read -
  it would need a breakpoint on the archive's own read/seek primitive
  instead, not attempted this pass.

**What this settles and what it does not:** the previews are real, on both
screens, confirmed by direct observation rather than static reading alone.
*Which file* each screen loads by default (non-Phantom-branch) was not
pinned down by decompilation or by the `sceIoOpen` sweep. `<location>\Ship.vex`
and `<location>\track.vex` are the only per-team/per-track files that exist on
disc and visually match what is rendered (correct, distinct silhouette per
entity); `oag_game`'s picker wires those two as the FE preview convention on
confidence ~70 - real, resolving, matching files, but not confirmed as the
exact ones the executable's own composer names.

## Open

- The record class that actually owns the mesh-loading virtual (reached via
  the `"Track"`/`""` notification pair) was not located.
- `TeamSelection_ApplySelection`'s Phantom-model trigger needs a live memory
  trace (break at `0x08951bd0`, read `+0xe1`/`+0xe2` at the moment the model
  visibly changes) to settle past confidence 60. Not attempted: it needs a
  save state further into the campaign than a fresh profile, which this
  pass did not have.
- Word indices 25, 27 and 29 (each screen's own `OnEnter`/`OnExit`/refresh
  overrides) were read in passing (`0x089519d0` is a destructor,
  `0x08951a2c`/`0x08951b7c` are enter/exit) but not renamed - none of the
  three carries preview-relevant logic distinct from what is written up
  above.
