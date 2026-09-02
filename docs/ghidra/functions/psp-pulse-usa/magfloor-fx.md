# Mag-floor gfx/sfx: two real `.vex` effect models, trigger read from the tail of `Ship_CastHoverProbes`

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | ship scene-graph effects, resource loading |
| **Related** | [`engine.md`](engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02) (the trigger), [`vex.md`](../../../formats/vex.md) (`CLASS_MAG_FLOOR_COLLISION`, the *collision* mesh - a different thing from this page), [`track.md`](../../../formats/track.md) (the track's own static magstrip texture/material, also a different thing - closed as unanimated in the project's handover history) |

## Summary

Two ordinary `.vex` models ship on Pulse's disc and preload every race:
`Data\visual_effects\MagEffect1.vex` and `Data\visual_effects\MagEffect2.vex`,
separate from both the collision mesh (`Mag Floor Collision`, class `0x3e6`)
and the track's own glowing strip texture. `MagFloorFx_Construct`
(`0x088590a8`) loads both by exact string address into a per-entity object's
`+0xac`/`+0xb0` fields and starts them hidden; the tail of
`Ship_CastHoverProbes` edge-detects the mag-floor contact bool it already
computes and calls `Ship_MagFloorEnter`/`Ship_MagFloorExit`
([engine.md](engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02)),
which call `MagFloorFx_Show`/`MagFloorFx_Hide` (`0x088598ac`/`0x088598d8`) on
those same two fields on the rising/falling edge. **The node-identity
question this page originally left open is closed**: the constructor proves
`+0xac` is `MagEffect1.vex` and `+0xb0` is `MagEffect2.vex`, not an inference
from proximity - see [The trigger](#the-trigger).

**Found starting from an `AskUserQuestion` to the maintainer** (they play these
games): both a visible spark/glow below the craft and an audible hum were
reported. This page recovers the gfx half in full down to the trigger; the sfx
half is open - see below.

## The two assets are real and on the disc

Both filenames appear twice in `BOOT.BIN`'s data segment: once mixed-case
(`MagEffect1.vex`/`MagEffect2.vex`, in a `"file"`-tagged resource block shared
with every weapon's `.vex`/sound-cue declarations, `0x08a7bfa8`/`0x08a7bfcc`)
and once lowercase, immediately beside the per-race `Data\Psys\WO_*.POB`
preload list (`0x08a7c39c`/`0x08a7c3c0`) - so they preload every race, the same
as the weapon effects, regardless of whether the track has a magstrip.

`oag-wad hash` on both path spellings gives `ba9996ee` (`MagEffect1.vex`) and
`fd39ec3e` (`MagEffect2.vex`); both hashes are real entries in
`pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad` (index 1079, 15312 bytes and index
1080, 4976 bytes - adjacent, consistent with the pair being authored and
packed together). Extracted and parsed with the existing `.vex` reader:

```
Data\visual_effects\MagEffect1.vex: 1 mesh, 114 vertices, 112 triangles, radius 10.95
Data\visual_effects\MagEffect2.vex: 1 mesh, 103 vertices, 101 triangles, radius 10.53
```

Both files are ordinary version-6 VEXX, authored from
`Z:/WipeoutPSP/X2/Data/visual_effects/...` (the same dev-path convention every
other authored `.vex` on this disc carries), not generated or stray data.
`just view ... --mesh 'Data\visual_effects\MagEffect1.vex' --screenshot` and
the same for `MagEffect2.vex` both render: thin, additive-looking
purple/blue streak geometry, splayed outward from a point - `MagEffect1` a
narrower cluster of three or four spikes, `MagEffect2` a wider symmetric fan.
Matches the maintainer's "spark/glow below the craft" description well enough
to be the effect, though the identification is still via the trigger's
structural position (see below), not a side-by-side capture against the
running game.

**Confidence 90** that these are the mag-floor effect's geometry: the asset
names are unambiguous (`visual_effects\MagEffect*`, no other candidate string
anywhere in the binary), they parse as real authored content rather than
placeholder data, and they preload every race the way every other
per-craft/per-weapon effect model does. Not yet runtime-verified against a
`just play` capture on a magstrip track.

## The trigger

Read in full on [engine.md](engine.md#the-tail-of-ship_casthoverprobes-fires-the-mag-floor-gfxsfx-2026-09-02).
Short version: `Ship_CastHoverProbes` writes `craft+0x240` (the same byte
`Ship_UpdateMagLock` gates on) from a probe restricted to surface type 3 - the
mag floor - and edge-detects the write, calling `Ship_MagFloorEnter`
(`0x0883e5fc`) or `Ship_MagFloorExit` (`0x0883e624`) with the craft's entity
pointer. Those two gate on an unidentified `entity+0x8bc` condition and then
call `MagFloorFx_Show`/`MagFloorFx_Hide` (`0x088598ac`/`0x088598d8`), which
flip a byte at `entity+0xb4` and OR/AND bit `0x4` on a field at `+0x2c` of two
objects reached through `entity+0xac`/`entity+0xb0`.

**The node identity is proven, 2026-09-02.** `MagFloorFx_Construct`
(`0x088590a8`) is the constructor for this same object (it initialises
`+0x28`, `+0x38`, `+0x40`, `+0xac`, `+0xb0`, `+0xb4` - exactly the fields the
trigger and the show/hide pair touch). It loads two resources by **exact
string address**, not by inference:

```c
// entity+0xac:
Load(obj1, 0x08a7bfa8 /* "Data\visual_effects\MagEffect1.vex" */, 0x4d000000, 0xfdb2, 0x3e9, 0);
*(entity + 0xac) = obj1;

// entity+0xb0:
Load(obj2, 0x08a7bfcc /* "Data\visual_effects\MagEffect2.vex" */, 0x4d000000, 0xfdb2, 0x3e9, 0);
*(entity + 0xb0) = obj2;

// both start hidden, matching MagFloorFx_Show/Hide's bit 0x4 on the same +0x2c field:
*(int*)(entity->0xac + 0x2c) &= ~4;
*(int*)(entity->0xb0 + 0x2c) &= ~4;
entity->0xb4 = 0;   // the same "active" byte Show/Hide flip
```

Both objects also get a colour written through a shared helper
(`func_0x0010e2b4`, packed from the same four global floats for both, likely
a shared tint) before construction finishes. **Confidence 90** for the
identity chain end to end: the string addresses are exact matches (not
proximity, not a guess), and the field-offset overlap with the trigger and
show/hide pair (`+0xac`, `+0xb0`, `+0xb4`, the `+0x2c` bit) is total. Not yet
runtime-verified (no `just play` capture), and the containing object's
identity - whether `entity` here is literally the same "ship entity" pointer
`engine.md` documents elsewhere at `craft+0x1c4`, or a distinct sub-object
that happens to share field offsets - was not independently confirmed;
`MagFloorFx_Construct` was not traced back to a caller (no direct caller
found, consistent with the indirect/vtable-driven construction this codebase
has already documented for other node-class constructors, e.g. `Engine
Flare`'s on [exhaust.md](exhaust.md#the-engine-flare-constructor)).

## The 16-float block is boilerplate, not the anchor - the real anchor is still unlocated

After loading and hiding both objects, `MagFloorFx_Construct` builds a
16-float (0x40-byte) block (disassembly: `lui a3,0x29` / `addiu a0,a3,-0x3860`
constructs the absolute address `0x0028c7a0` directly, not through `$gp` or
an image-relative displacement) and passes it to `FUN_08945284` twice, once
per object (`param_3` `1`/`0` distinguishing them). That function copies the
16 floats verbatim into a lazily-allocated transform-cache slot on the target
object and then sets two bits in the *same* `+0x2c` field `MagFloorFx_Show`/
`Hide` already use - `0x01000000` when `param_3 == 0`, `0x08000000` when it
is not - which reads as a draw-key/layer selector in the same high-byte
position `draw-order.md` already decoded for the mesh-layer discriminator,
not anything positional.

**First read this as the authored per-effect anchor; it almost certainly
is not.** `get_xrefs_to` on `0x0028c7a0` returns **over eighty** reads from
completely unrelated constructors across the binary - `Gfx_Init`,
`Trail_InitPreset`, `ShipShield_Update`, `Missile_Update`, `Rocket_Update`,
`AnimTransform_EvalRotation`, `Mine_SpawnExplosion`, and dozens more with no
shared subsystem. A per-effect authored anchor would not be read by the
shield, a missile and the graphics init path alike; a shared default/identity
matrix constant used to seed a node's local transform before something else
positions it would be exactly this widely read. Reading it (`ram:0028c7a0`
is not backed by this Ghidra project's loaded segments, so the actual sixteen
values were not confirmed) is very unlikely to change this reading, so this
session stopped chasing it rather than spend more time reading a constant
that most likely does not vary per-object anyway.

**So the real placement question is still open, and differently shaped than
this page first thought**: `MagFloorFx_Construct` has no visible call that
positions either object relative to the craft - it loads, hides, colours,
and seeds an identity-shaped default transform, then returns. Wherever the
two nodes get parented onto the craft's scene graph (the mechanism
`exhaust.md` already documents for `Engine Flare`/`Trail`'s own placement)
is not in this function. `MagFloorFx_Construct` was not traced back to a
caller - finding that caller, not reading this constant, is the next step
for the anchor question.

## The sfx half is open

No call to any sound-play function was found in `Ship_MagFloorEnter`,
`Ship_MagFloorExit`, `MagFloorFx_Show`/`MagFloorFx_Hide`, or
`MagFloorFx_Construct` - the whole visible chain is scene-graph flag flips
and a resource load, nothing that looks like `Sound_Play`.

**`oag-wad sounds` was run against the whole of `Data.wad`** (36 banks, 582
cues, all named), closing off the more obvious version of "the cue was never
searched for". The `SHIP` bank (`#860`) - the one most likely to carry a
per-craft magfloor cue, since it already carries `~ENGINE`, `MALFUNCTION`,
`FLIP`, `.COLLISIONS` - has no candidate: its nine cues are `.COLLISIONS`,
`EXPLBIG`/`EXPLBIG_PC`, `EXPLSMALL`/`EXPLSMALL_PC`, `FLIP`, `MALFUNCTION`,
`RESET`, `~ENGINE`. `SHIP_ZM` (Zone mode's ship bank) has none either. A
broader grep for `mag|strip|hum|grind|lock` across every bank does turn up
`~HUM` (in the per-track ambience banks `basilic`, `dekonst`), `~bighum`/
`~hum` (in `gentrak`, alongside `~radardish`/`~billboard`/`~neon`/`~crowd`/
`~startline`) and `~SINGLE_GRINDER` (in `fortcle`) - but every one of these
banks is named after a *track*, not a craft, and sits beside plainly
environmental cues (crowd noise, a radar dish, a starting-line neon buzz).
Read as generic track-scenery ambience, not a per-craft magfloor cue - though
whether any of them happens to be *placed* along that track's magstrip
sections specifically was not checked, and would need the pad/emitter
placement data, not just the bank's cue list.

So: **no per-craft magfloor sfx cue exists in `Data.wad`.** The `Engine
Flare`-owns-`~ENGINE` reading this page originally carried as the live
hypothesis is now weaker than it looked: `Engine Flare`/`Trail` are their
own dedicated node classes with bespoke C++ update functions
([exhaust.md](exhaust.md)), which is *why* one can own a sound emitter.
`MagEffect1.vex`/`MagEffect2.vex` are not - `oag-view --mesh` finds a plain
`CLASS_MESH` (`0x125`) payload node in both (`crates/view/src/draws.rs`
filters on exactly that class id to find anything to draw at all), the
generic mesh class every ordinary prop uses, loaded through the same
resource loader every weapon model goes through - not a bespoke class with
its own update function the way `Engine Flare` is. A generic mesh node has
nowhere to hide an emitter. Combined with the bank census finding nothing,
**the more honest reading is that the hum is not attached to this effect at
all** - it is plausibly one of the per-track ambience loops found above
(`~HUM`/`~bighum`/`~SINGLE_GRINDER`), and the maintainer associates it with
magstrip sections because that is where those sections happen to place it,
not because riding the strip triggers it. Not confirmed either way - it
would need the pad/emitter placement data for a magstrip track, not just the
bank's cue list.

**Do not invent a cue for this.** Per `CLAUDE.md`'s do-not-invent rule, no sfx
is wired until the emitter reading above is confirmed or refuted.

## Cross-platform

| Title | Result |
| --- | --- |
| Pulse PSP (USA) | Both assets present, hashes `ba9996ee`/`fd39ec3e`, entries 1079/1080 of `Data.wad`. The trigger above. |
| Pure PSP (USA/EU) | **These two exact hashed names are absent.** `oag-wad hash` on the same two path spellings (`Data\visual_effects\MagEffect1.vex`/`MagEffect2.vex`) gives the same two hashes (the hash function is path-text-only, not per-title), and neither hash is an entry in `pure-psp-usa.chd`'s or `pure-psp-eu.chd`'s `Data.wad` (832 entries, censused in full). That is narrower than "Pure has no such effect": a different path, a different casing that hashes differently, or a different archive (`FE.wad` was not censused) would all read the same as absent here. Pure's own Ghidra binary was not reachable in this session (bridge only exposed one program at a time) so the *code* side - whether Pure even has an equivalent trigger, over a different or absent asset - is unchecked, not ruled out. |
| HD/Fury PS3 | **No `MagEffect` string anywhere in `EBOOT.elf`.** Whatever HD/Fury does for a magstrip section (if anything - HD's track set may not reuse Pulse's magstrip sections at all) is a different mechanism or absent; not investigated further this session. |

## Open questions

- **Where these two nodes are placed relative to the craft - the primary
  open question now.** `MagFloorFx_Construct` does not position them; the
  caller that parents them onto the craft's scene graph is unlocated.
- What `entity+0x8bc` gates in `Ship_MagFloorEnter`/`Ship_MagFloorExit`
  (possibly per-player scoping, unconfirmed - see `entity+0x368` on
  [shield.md](shield.md#entity--0x368-is-craft_construct_qs-own-second-argument)
  for a similarly-shaped but distinct field this codebase has already chased).
- Whether `MagFloorFx_Construct`'s `entity` parameter is literally the same
  "ship entity" pointer documented at `craft+0x1c4` elsewhere on this page's
  binary, or a distinct sub-object at the same relative offsets - no caller
  was traced for the constructor.
- The sfx mechanism, now leaning toward "not attached to this effect at all"
  (see above) rather than a hidden emitter - not settled, since the
  per-track ambience reading is also unconfirmed.
- What colour `func_0x0010e2b4` packs for the two objects, and whether it
  differs between them (not decoded - the source globals sit in the same
  unmapped-in-this-project low address range as the shared transform
  constant above).
- Pure's code side (does it have an equivalent trigger over a different or
  absent asset, under a path this session did not hash?) and HD/Fury's
  mechanism, if any, are both unchecked.

## History

- 2026-09-02: Assets found and parsed (90), trigger function tail read (85).
  Node identity closed (90): `MagFloorFx_Construct` loads both assets by
  exact string address into the same `+0xac`/`+0xb0` fields the trigger and
  `MagFloorFx_Show`/`Hide` (renamed from the unnamed enable/disable pair)
  operate on. The constructor's 16-float block, first read as a candidate
  authored anchor, is instead a shared default transform (80+ unrelated
  callers via `get_xrefs_to`) - the real anchor is wherever the two nodes get
  parented onto the craft, still unlocated. All 582 cues across Pulse's 36
  sound banks enumerated: no per-craft magfloor cue in `SHIP`/`SHIP_ZM`, and
  the `MAG*`/`HUM`/`GRIND` hits elsewhere are track-ambience banks, not craft
  ones; combined with both files parsing as a plain `CLASS_MESH` payload (no
  bespoke node class to hide an emitter in, unlike `Engine Flare`), the sfx
  reading shifted from "probably a hidden emitter" to "probably not attached
  to this effect at all" - still not confirmed either way. Pure ruled out for
  these two exact asset hashes in `Data.wad` only; Pure's code and HD/Fury
  both unchecked.
