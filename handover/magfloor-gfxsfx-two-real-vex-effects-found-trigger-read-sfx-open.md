# Magfloor gfx/sfx: two real `.vex` effects found, the trigger is read, sfx and the other two titles are open

Started from the user's request to investigate/implement additional gfx and
sfx while on a magfloor, in a worktree (`../oag-magfloor-fx`, branch
`magfloor-fx`). An `AskUserQuestion` to the maintainer (they play these games)
confirmed both a visible spark/glow below the craft and an audible hum exist
in the original. Full writeup: [`magfloor-fx.md`](../docs/ghidra/functions/psp-pulse-usa/magfloor-fx.md),
with the trigger itself on [`engine.md`](../docs/ghidra/functions/psp-pulse-usa/engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02).

**Gfx, Pulse PSP: found and parsed, not yet implemented.** Two real `.vex`
models ship on the disc, `Data\visual_effects\MagEffect1.vex` and
`MagEffect2.vex` (entries 1079/1080 of `Data.wad`, hashes `ba9996ee`/
`fd39ec3e`), preloaded every race alongside the weapon effects. Extracted,
parsed with `oag_formats::vex`, and rendered with `just view --mesh` -
additive-looking purple/blue streak geometry, matching the maintainer's
description well enough to be the effect. `Ship_CastHoverProbes` edge-detects
the mag-floor contact bool it already computes and calls two newly-named
functions, `Ship_MagFloorEnter`/`Ship_MagFloorExit` (`0x0883e5fc`/
`0x0883e624`), which flip a flag and OR/AND a bit on two child objects. Those
two children are inferred, not proven, to be the two MagEffect nodes -
confidence 55 on that specific claim, which is why the lower-level
enable/disable pair (`0x088598ac`/`0x088598d8`) is cited by address rather
than renamed.

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
wiring was deliberately left for a follow-up once the sfx mechanism and the
node-identity question are resolved, per the do-not-invent rule: showing two
real vex meshes attached to the craft without confirming they're actually the
right two objects (or firing an invented sound cue) would be exactly the kind
of plausible-looking stand-in `CLAUDE.md` warns against.

## Open

- The node-identity claim (`entity+0xac`/`+0xb0` == the two MagEffect nodes) -
  needs the constructor that writes those two fields, not yet located.
- The sfx mechanism - no bank cue exists, so it is either baked into one of
  the two vex nodes' own emitter or the hum has some other source entirely.
- What `entity+0x8bc` gates in `Ship_MagFloorEnter`/`Ship_MagFloorExit`.
- Pure's code side is unchecked (only the asset-hash census ran).
- HD/Fury's mechanism, if any, is unchecked beyond the one string search.
- No render/audio implementation exists yet in `crates/` for any title.

## Next Steps

- Read the node class `MagEffect1`/`MagEffect2` register as, and whether its
  own update function owns a sound emitter (the `Engine Flare` pattern) -
  now the single open question on the sfx side, since the bank census ruled
  out a plain cue.
- Get a Ghidra session with `psp-pure-usa`/`psp-pure-eu` actually switched in
  (this session's bridge only exposed `BOOT.BIN` ambiguously across the two
  Pulse regions and never resolved to Pure) and repeat the string/asset search
  there.
- If the node-identity confidence can be raised past 70 (locate the
  constructor for `entity+0xac`/`+0xb0`), rename the enable/disable pair and
  raise `magfloor-fx.md`'s confidence accordingly.
- Once both the trigger and the sfx mechanism clear confidence, implement:
  attach `MagEffect1.vex`/`MagEffect2.vex` to the craft in `oag-render`,
  additively blended, shown/hidden off the physics crate's existing
  `Surface::MagFloor` contact signal (`crates/physics/src/maglock.rs`/
  `hover.rs` already compute the equivalent of `craft+0x240`) - the render
  side does not need to wait on the exact `entity+0xac`/`+0xb0` struct
  offsets, only on confirming *which two assets* the original shows.
- A side-by-side `just play` capture on a magstrip track (e.g. a Pulse circuit
  known to have one) against the geometry screenshots in `magfloor-fx.md`
  would settle the visual identification without needing the constructor.
