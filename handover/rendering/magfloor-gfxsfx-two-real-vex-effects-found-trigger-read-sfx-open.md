---
categories: [rendering, audio]
---

# Magfloor gfx/sfx: node identity is proven, where the effect anchors on the craft is the last gfx blocker, sfx leans "not attached at all"

Started from the user's request to investigate/implement additional gfx and
sfx while on a magfloor, in a worktree (`../oag-magfloor-fx`, branch
`magfloor-fx`). An `AskUserQuestion` to the maintainer (they play these games)
confirmed both a visible spark/glow below the craft and an audible hum exist
in the original. Full writeup: [`magfloor-fx.md`](../../docs/ghidra/functions/psp-pulse-usa/magfloor-fx.md),
with the trigger itself on [`engine.md`](../../docs/ghidra/functions/psp-pulse-usa/engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02).

**Gfx, Pulse PSP: node identity proven, placement is the one blocker left.**
Two real `.vex` models ship on the disc, `Data\visual_effects\MagEffect1.vex`
and `MagEffect2.vex` (entries 1079/1080 of `Data.wad`, hashes `ba9996ee`/
`fd39ec3e`), preloaded every race alongside the weapon effects. Extracted,
parsed with `oag_vex::vex`, and rendered with `just view --mesh` -
additive-looking purple/blue streak geometry, matching the maintainer's
description well enough to be the effect. `Ship_CastHoverProbes` edge-detects
the mag-floor contact bool it already computes and calls `Ship_MagFloorEnter`/
`Ship_MagFloorExit` (`0x0883e5fc`/`0x0883e624`), which call `MagFloorFx_Show`/
`MagFloorFx_Hide` (`0x088598ac`/`0x088598d8`, newly created as real Ghidra
functions this pass) to flip a flag and OR/AND a bit on two child objects.
**Those two children are now proven, not inferred, to be the two MagEffect
nodes**: `MagFloorFx_Construct` (`0x088590a8`, also newly named) loads both
by their exact string address into the same two fields, confidence 90.

**What's still missing for an honest implementation: where the two nodes
anchor on the craft.** The constructor's own 16-float transform block looked
at first like a candidate authored anchor, but `get_xrefs_to` on its source
address turns up 80+ reads from completely unrelated constructors (`Gfx_Init`,
`Missile_Update`, `ShipShield_Update`, ...) - a shared default/identity
transform, not per-effect data. The real placement happens wherever these two
nodes get parented onto the craft's scene graph, and `MagFloorFx_Construct`
has no such call - its caller, which would show that, was not traced.
Implementing the attachment point on a guess would be exactly what the
do-not-invent rule forbids.

**Sfx, Pulse PSP: no cue exists, and the mechanism now leans "not attached to
this effect at all".** No sound-play call anywhere in the trigger chain, and
`oag-wad sounds` against the whole of `Data.wad` (36 banks, 582 cues, all
enumerated) rules out a per-craft cue: `SHIP`/`SHIP_ZM` have none
candidate-shaped, and the `MAG*`/`HUM`/`GRIND` hits elsewhere are all in
per-track ambience banks (`basilic`, `dekonst`, `gentrak`, `fortcle`),
alongside plainly environmental cues like `~crowd`/`~radardish`/`~billboard`.
The earlier live hypothesis - a sound baked into one of the two vex nodes'
own emitter, the way `Engine Flare` owns `~ENGINE` - is weaker than it
looked: `Engine Flare`/`Trail` are bespoke node classes with their own C++
update functions, which is *why* one can own an emitter; both MagEffect
files parse as a plain `CLASS_MESH` payload (`oag-view`'s own class filter
confirms it), the generic mesh class every ordinary prop uses, with no
per-class code to hide an emitter in. The more honest reading now is that the
hum the maintainer hears is a per-track ambience loop that happens to sit
near magstrip sections, not a per-craft cue at all - still not confirmed.

**Pure and HD/Fury: neither implemented.** Pure's `Data.wad` (both regions,
832 entries, censused in full) does not contain either hash under the same
path spelling - a real negative for these two exact assets under that one
path, not a claim that Pure lacks the effect entirely (a different path or
casing, or `FE.wad`, was not tried). Pure's own Ghidra binary was unreachable
this session (the MCP bridge only had one program switched in at a time and
`psp-pure-usa`/`psp-pure-eu` would not switch in), so whether Pure has an
equivalent trigger over a different or absent asset is unchecked, not ruled
out. HD/Fury's `EBOOT.elf` has no `MagEffect` string at all; not investigated
further.

**Nothing was implemented in `crates/` this session** - the render/audio
wiring was deliberately left for a follow-up once the placement is located
and the sfx question settles one way or the other, per the do-not-invent
rule: attaching two real vex meshes at a guessed position, or firing an
invented sound cue, would be exactly the kind of plausible-looking stand-in
`CLAUDE.md` warns against - node identity alone is not enough, placement
matters too.

**A live-capture attempt was made this session and did not reach the
inverted section.** Following the "Next Steps" advice above, `MagFloorFx_Show`
and `Ship_MagFloorEnter` were both armed as PPSSPP breakpoints (PPSSPP v1.20.4,
SDL build under Xvfb, `docs/reverse-engineering/ppsspp-debugger.md`'s
workflow) and driven at Talon's Junction, Time Trial, White, three separate
ways:

1. Free-running `scripts/psp-drive.py drive --script
   verification/scenarios/talons-junction-time-trial-lap.inputs` (the fast
   path) while a breakpoint listened on a second connection - completed a
   full, clean 3146-tick lap (final speed ~56, matching a healthy racing
   speed) without either breakpoint firing.
2. The same script replayed breakpoint-driven through `scripts/psp-trace.py`
   (temporarily adding `mag_gate`/`mag_blend` columns at `craft+0x240`/
   `+0x280` to `psp_trace_fields.py`, reverted after) - also a clean 3146-tick
   run, and **the craft's `up_y` never went negative even once across the
   whole lap**. This is the useful negative: `talons-junction-time-trial-lap.inputs`
   is an *open-loop* replay of pre-recorded button presses, and it does not
   reproduce the original capture's line closely enough, over ~1,100 ticks,
   to still enter the inverted loop - it just drives a normal path around
   that stretch of track instead. `cornering-ground-truth.md`'s own
   confidence-90 finding that mag-lock engages there is not contradicted by
   this: that finding came from a *closed-loop* captured lap, and this
   replay diverged from it long before reaching the section in question.
3. `scripts/psp-autopilot.py` (closed-loop, chasing `oag-trace track`'s own
   spline) - the correct tool for this, since it self-corrects instead of
   replaying stale button presses. Two attempts both crashed a corner around
   32-35% of the way round (spline point ~1150-1200 of 3448, right around
   where the inverted section should start) before completing a lap, ending
   the race early ("the craft update stopped firing... the race is over").
   Neither run's `mag_gate` column ever went nonzero either, consistent with
   never reaching the section - `up_y` stayed within `0.81`-`1.0`.

**So the negative result across all three attempts is explained by never
reaching the inverted section at all, not by the trigger failing to fire
there.** This is a driving problem, not an RE problem: the autopilot's
pure-pursuit steerer (explicitly not a claim about the original's AI, just a
means to get a lap driven) needs to actually get around Talon's Junction's
technical inverted loop, which it did not manage twice in a row this session.
Scratch instrumentation (`mag_gate`/`mag_blend` columns, a throwaway capture
script) was reverted/removed before merging - nothing about the method
needs to be reconstructed from memory, it is all in this section.

## Open

- Getting a closed-loop autopilot run all the way through Talon's Junction's
  inverted section without crashing, so `MagFloorFx_Show`/`Ship_MagFloorEnter`
  can actually be observed live - the immediate blocker, ahead of "where the
  two nodes anchor" below (that question cannot even be attempted until this
  one is solved).
- Where the two nodes anchor on the craft - `MagFloorFx_Construct`'s caller,
  which would show the parenting/placement, is unlocated.
- The sfx mechanism - leaning "not attached to this effect", not confirmed;
  would need a magstrip track's pad/emitter placement data to settle.
- What `entity+0x8bc` gates in `Ship_MagFloorEnter`/`Ship_MagFloorExit`.
- Whether `MagFloorFx_Construct`'s `entity` parameter really is the same
  "ship entity" pointer documented at `craft+0x1c4` elsewhere, or a distinct
  sub-object at the same relative offsets - no caller was traced.
- Pure's code side is unchecked (only the asset-hash census ran).
- HD/Fury's mechanism, if any, is unchecked beyond the one string search.
- No render/audio implementation exists yet in `crates/` for any title.

## Next Steps

- Get a closed-loop `psp-autopilot.py` run all the way through Talon's
  Junction's inverted section - tune `--look-min`/`--look-speed`/
  `--look-max`/`--deadband`/`--brake-at` (see its own `--help`) rather than
  retrying with the defaults a third time, since two identical attempts both
  crashed at the same ~32-35% mark. `--gate`/`--gate-dir` (drive to a chosen
  world point instead of a full lap) may be the faster route in if a rough
  coordinate for the inverted section's entry can be estimated from the
  track mesh, avoiding the technical corners before it entirely.
- Once a run reaches the section live: with `MagFloorFx_Show` armed, read
  `a0` (the entity), then `entity+0xac`/`entity+0xb0`, then each object's
  `+0x3c` -> `+0x40..0x7f` (16 floats, the transform-cache slot
  `MagFloorFx_Construct`/`FUN_08945284` write to) and compare against an
  identity matrix - if it differs, that is the anchor; if it is still
  identity, the anchor really is set elsewhere and `MagFloorFx_Construct`'s
  caller needs to be found instead (a live breakpoint on `0x088590a8` itself,
  the same method `weatherpos.md` used for a similarly indirect constructor).
- Check whether any magstrip-carrying track's pad/emitter data places one of
  the `~HUM`/`~bighum`/`~SINGLE_GRINDER` ambience cues specifically along the
  strip, to settle the sfx question the other way from "not attached".
- Get a Ghidra session with `psp-pure-usa`/`psp-pure-eu` actually switched in
  (this session's bridge only exposed `BOOT.BIN` ambiguously across the two
  Pulse regions and never resolved to Pure) and repeat the string/asset search
  there.
- Once the anchor is located, implement: attach `MagEffect1.vex`/
  `MagEffect2.vex` to the craft in `oag-render` at the recovered transform,
  additively blended, shown/hidden off the physics crate's existing
  `Surface::MagFloor` contact signal (`crates/physics/src/maglock.rs`/
  `hover.rs` already compute the equivalent of `craft+0x240`).
- A side-by-side `just play` capture on a magstrip track (e.g. a Pulse circuit
  known to have one) against the geometry screenshots in `magfloor-fx.md`
  would corroborate the visual identification independently of locating the
  constructor's caller.
