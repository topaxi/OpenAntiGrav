# Magfloor gfx/sfx: node identity is proven, the anchor's numbers are the last gfx blocker, sfx and the other two titles are open

Started from the user's request to investigate/implement additional gfx and
sfx while on a magfloor, in a worktree (`../oag-magfloor-fx`, branch
`magfloor-fx`). An `AskUserQuestion` to the maintainer (they play these games)
confirmed both a visible spark/glow below the craft and an audible hum exist
in the original. Full writeup: [`magfloor-fx.md`](../docs/ghidra/functions/psp-pulse-usa/magfloor-fx.md),
with the trigger itself on [`engine.md`](../docs/ghidra/functions/psp-pulse-usa/engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02).

**Gfx, Pulse PSP: node identity proven, one blocker left before implementing.**
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

**What's still missing for an honest implementation**: the constructor also
builds a 16-float local transform and a per-object render-mode bit and hands
both to a generic node-transform setter, which is very plausibly the
authored anchor (where under the craft these sit) and blend/layer selection
- but the sixteen floats live behind a `$gp`-relative addressing quirk this
session's Ghidra bridge couldn't resolve to real memory (see
`magfloor-fx.md`'s "The anchor transform exists but its numbers were not
read"). Implementing the attachment point without those numbers would mean
guessing a placement, which is exactly what the do-not-invent rule forbids.

**Sfx, Pulse PSP: no cue exists, but the mechanism is still open.** No
sound-play call anywhere in the trigger chain, and `oag-wad sounds` against
the whole of `Data.wad` (36 banks, 582 cues, all enumerated) rules out a
per-craft cue: `SHIP`/`SHIP_ZM` have none candidate-shaped, and the
`MAG*`/`HUM`/`GRIND` hits elsewhere are all in per-track ambience banks
(`basilic`, `dekonst`, `gentrak`, `fortcle`), alongside plainly environmental
cues like `~crowd`/`~radardish`/`~billboard`. What remains is one
hypothesis: the hum is baked into one of the two vex nodes' own emitter, the
way `Engine Flare` owns `~ENGINE` - not yet checked, needs the node class's
own update function.

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
wiring was deliberately left for a follow-up once the anchor numbers and the
sfx mechanism are resolved, per the do-not-invent rule: attaching two real
vex meshes at a guessed position, or firing an invented sound cue, would be
exactly the kind of plausible-looking stand-in `CLAUDE.md` warns against -
node identity alone is not enough, placement matters too.

## Open

- The anchor transform's actual sixteen floats (and the per-object
  render-mode bit) - the constructor builds and hands them off, but they sit
  behind a `$gp`-relative addressing quirk this session's static Ghidra
  bridge could not resolve to real memory. Needs a live-emulator watchpoint
  read, per this project's own notes on the same class of trap.
- The sfx mechanism - no bank cue exists, so it is either baked into one of
  the two vex nodes' own emitter or the hum has some other source entirely.
- What `entity+0x8bc` gates in `Ship_MagFloorEnter`/`Ship_MagFloorExit`.
- Whether `MagFloorFx_Construct`'s `entity` parameter really is the same
  "ship entity" pointer documented at `craft+0x1c4` elsewhere, or a distinct
  sub-object at the same relative offsets - no caller was traced.
- Pure's code side is unchecked (only the asset-hash census ran).
- HD/Fury's mechanism, if any, is unchecked beyond the one string search.
- No render/audio implementation exists yet in `crates/` for any title.

## Next Steps

- Read the anchor transform's sixteen floats with a live emulator and a
  watchpoint (the static approach is exhausted this session) - this is now
  the single blocker on the gfx side, since node identity is settled.
- Read the node class `MagEffect1`/`MagEffect2` register as, and whether its
  own update function owns a sound emitter (the `Engine Flare` pattern) -
  the single open question on the sfx side, since the bank census ruled out
  a plain cue.
- Get a Ghidra session with `psp-pure-usa`/`psp-pure-eu` actually switched in
  (this session's bridge only exposed `BOOT.BIN` ambiguously across the two
  Pulse regions and never resolved to Pure) and repeat the string/asset search
  there.
- Once the anchor and the sfx mechanism clear confidence, implement: attach
  `MagEffect1.vex`/`MagEffect2.vex` to the craft in `oag-render` at the
  recovered transform, additively blended, shown/hidden off the physics
  crate's existing `Surface::MagFloor` contact signal
  (`crates/physics/src/maglock.rs`/`hover.rs` already compute the equivalent
  of `craft+0x240`).
- A side-by-side `just play` capture on a magstrip track (e.g. a Pulse circuit
  known to have one) against the geometry screenshots in `magfloor-fx.md`
  would corroborate the visual identification independently of the emulator
  read.
