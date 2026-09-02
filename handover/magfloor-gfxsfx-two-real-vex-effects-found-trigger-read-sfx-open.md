# Magfloor gfx/sfx: node identity is proven, where the effect anchors on the craft is the last gfx blocker, sfx leans "not attached at all"

Started from the user's request to investigate/implement additional gfx and
sfx while on a magfloor, in a worktree (`../oag-magfloor-fx`, branch
`magfloor-fx`). An `AskUserQuestion` to the maintainer (they play these games)
confirmed both a visible spark/glow below the craft and an audible hum exist
in the original. Full writeup: [`magfloor-fx.md`](../docs/ghidra/functions/psp-pulse-usa/magfloor-fx.md),
with the trigger itself on [`engine.md`](../docs/ghidra/functions/psp-pulse-usa/engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02).

**Gfx, Pulse PSP: node identity proven, placement is the one blocker left.**
Two real `.vex` models ship on the disc, `Data\visual_effects\MagEffect1.vex`
and `MagEffect2.vex` (entries 1079/1080 of `Data.wad`, hashes `ba9996ee`/
`fd39ec3e`), preloaded every race alongside the weapon effects. Extracted,
parsed with `oag_formats::vex`, and rendered with `just view --mesh` -
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

## Open

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

- Find `MagFloorFx_Construct`'s caller (no direct caller resolved statically;
  a live emulator breakpoint on `0x088590a8` while loading a magstrip track,
  the same method `weatherpos.md` used for a similarly indirect constructor,
  is the precedent to follow) - this is now the single blocker on the gfx
  side, since node identity is settled.
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
