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
pointer. Those two load `entity+0x8bc`, which is the MagFloorFx object itself
(see [Who constructs it](#who-constructs-it-and-entity0x8bc-is-the-effect-object-itself)),
and when it is non-null call `MagFloorFx_Show`/`MagFloorFx_Hide`
(`0x088598ac`/`0x088598d8`) on it, which flip a byte at `obj+0xb4` and OR/AND
bit `0x4` on a field at `+0x2c` of the two models at `obj+0xac`/`obj+0xb0`.

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

## The anchor: `(0, -2.5, 0)` in the craft's own frame, two attachment modes (2026-10-05)

**This section replaces a 2026-09-02 reading that called the constructor's
16-float block "boilerplate, not the anchor". That reading was wrong in one
detail that decides everything.** The block's source is `0x08a907a0` (the
same identity matrix `Vex_UpdateNodeWorldMatrix` lazily copies for a null
node; the old page quoted it un-rebased as `0x0028c7a0`, which is why it read
as "unmapped"). It *is* shared and it *is* identity. But
`MagFloorFx_Construct` copies it to the stack and then overwrites one element
before passing it on:

```text
08859530  addiu a0,a3,0x7a0         ; a3 = 0x08a9_0000 -> 0x08a907a0, identity
08859534  lv.q  C400,0x7a0(a3)      ; four rows copied to sp+0x00..0x3f
...
08859560  lwc1  f12,0xef0(a0)       ; a0 = 0x08ab_0000 -> [0x08ab0ef0] = 0xc0200000 = -2.5
08859568  jal   Node_SetLocalMatrix ; (MagEffect1, sp, 1)
0885956c  _swc1 f12,0x34(sp)        ; element 13 = row 3 (translation), y
08859578  jal   Node_SetLocalMatrix ; (MagEffect2, sp, 0)
```

So both nodes get the local matrix **identity with translation
`(0, -2.5, 0)`**. Row 3 is the translation row (corroborated by
`Vex_UpdateNodeWorldMatrix`'s translate-only branch, which adds the rotated
`+0x30` row of the local matrix onto the parent's `+0x30` row). Row 1 of a craft's
matrix is its up axis ([input-bindings.md](input-bindings.md): `body+0x10` is
`up`, `body+0x20` the nose), so `-2.5` is **2.5 craft units below the craft
origin**, the "glow under the craft" the maintainer describes.

### Who constructs it, and `entity+0x8bc` is the effect object itself

`get_xrefs_to(0x088590a8)` has exactly one caller, `0x08841860` inside
`Craft_Construct` (`0x08840c74`), straight-line code with no gate other than
the craft pointer being non-null:

```text
08841824  jal  Mem_Alloc(0xc0, ...)        ; the 0xc0-byte MagFloorFx object
08841844  _sw  s0,0x8(s4)                   ; parent = the craft
0884184c  jal  Node_AttachChild(s0, s4)
08841860  jal  MagFloorFx_Construct(s4, s0) ; a1 = the craft -> obj+0x40
0884189c  sw   s3,0x8bc(s0)                 ; craft+0x8bc = the MagFloorFx object
```

That closes two open questions at once. **Every craft gets one** (the player
and every AI craft alike; no player-only gate exists on the path). And
**`entity+0x8bc` is the MagFloorFx object pointer**:
`Ship_MagFloorEnter`/`Exit` disassemble to `lw a0,0x8bc(a0); beq a0,zero;
jal MagFloorFx_Show/Hide` - the Ghidra decompile drops the argument, the
listing does not. The "gate" is only a null check on a construction that can
fail. The object `MagFloorFx_Construct` builds is a scene node (`Node_ConstructBase`,
vtable `0x08aca368` at `+0x38`), its `+0x40` is the craft.

### MagEffect1 rides the craft; MagEffect2 lies on the track

The two children are attached **differently**, which the old page could not
see:

| | MagEffect1 (`obj+0xac`) | MagEffect2 (`obj+0xb0`) |
| --- | --- | --- |
| Parent | the craft (`obj+0x40`, `Node_AttachChild` at `0x08859184`) | `g_race_manager` (`[0x08b317b4]`, the scene root, `0x08859240`) |
| `Node_SetLocalMatrix` mode | `1` -> `0x08000000`, **translate-only** | `0` -> `0x01000000`, full 4x4 |
| World matrix | the craft's world rows **unchanged** (rotation, roll, the `0.75` `g_craft_scale`), translation `craft.T + (0,-2.5,0)` carried through the craft's rotation | rebuilt every tick by `MagFloorFx_Update`, below |
| Scale | inherits `0.75` | unit rows |

`Vex_UpdateNodeWorldMatrix`'s `0x08000000` branch (`0x0894467c`) keeps the
parent's rows 0-2 and adds `vtfm3.t(parent 3x3, local row 3)` to the parent's
translation, so MagEffect1 is the craft's own frame moved down its up axis.

**`MagFloorFx_Update` (`0x0885962c`)** is vtable slot `+0x24` of `0x08aca368`,
the pre-update(dt) slot ([resource-loading.md](resource-loading.md)), and does
nothing unless `obj+0xb4` (the Show/Hide byte) is set. When it is:

```c
craft = obj->0x40;
d = normalize(-craft->0xb10.xyz);               // vneg.q, w zeroed; the located spline sample's down, negated
W = world matrix of craft (Vex_UpdateNodeWorldMatrix if dirty);
M = vmmul(O, W);                                 // O = identity with row3.y = -2.5, the same block as above
row1 = d;                                        // up: the track's up under the craft, unit
row2 = normalize(W.row2 - d * dot(d, W.row2));   // the nose, made perpendicular to d
row0 = cross(d, row2);                           // vcrsp.t
row3 = M.row3;                                   // = W.row3 - 2.5 * W.row1
Node_SetLocalMatrix(obj->0xb0, {row0,row1,row2,row3}, 0);
```

So MagEffect2 sits at the same point as MagEffect1 but is laid **flat on
the track** (its up is the track's, not the craft's, so it does not pitch or roll
with the craft) and is pointed along the craft's heading, at unit scale.
`craft+0xb10` is the located spline sample's down
([engine.md](engine.md#0xb10-is-splineptdown-and-the-whole-record-is-a-located-spline-sample)).

**The `vmmul` order is not a guess: the PS2 build spells it out.** PS2's
`MagFloorFx_Update` (`0x001659a0`, `SCES_547.48`) has no `vmmul`; it builds
each row of `O` against `W` with `vmulax/vmadday/vmaddaz/vmaddw` against `W`'s
four rows, which is `O * W` in row-vector form, so translation is
`W.row3 + (-2.5) * W.row1`. The rest is the same: up from `-craft+0xbd0`
normalised, nose orthogonalised with `vrsqrt`, side from `vopmula/vopmsub`
(the cross product), handed to the PS2's `Node_SetLocalMatrix` (`0x001fe888`)
with mode `0`. PS2's `MagFloorFx_Construct` (`0x00165688`) loads the same two
strings (`0x002a71e0`/`0x002a7208`), attaches MagEffect1 to the craft and
MagEffect2 to `[0x002e0280]`, and writes the `-2.5` from `[0x0027e950]`
(`0xc0200000`) into the translation row with `vaddx.y vf2,vf0,vf1` (y only)
before the same two `Node_SetLocalMatrix(.., 1)`/`(.., 0)` calls.

### The tint is opaque white

`Image_SetVertexColours` stamps both models with `[0x08b3bf30..0x08b3bf3c]`
packed `A<<24 | B<<16 | G<<8 | R`. The four floats have no writer the
decompiler attributes; the writer is a static initialiser at `0x08859904`
that Ghidra had never made a function (`MagFloorFx_InitTint`, created
2026-10-05): `lui a0,0x3f80` then four `swc1 f12` to `0x08b3bf30..3c` - all
`1.0`. PS2's `MagFloorFx_InitTint` (`0x00165ce0`) is the same, shaped as a GCC
static-init function (`a1 == 0xffff && a0 != 0`), writing `1.0` to
`0x002e35e0..ec`. So the stamp is `0xffffffff`: the authored vertex colours
are drawn as they are, with nothing multiplied in.

### Confidence

- **88**, the anchor `(0, -2.5, 0)` in the craft's own frame and the two
  attachment modes: the PSP decompile and listing are unambiguous, the
  same constant and structure are in an independent binary (PS2), and both
  of PS2's routes to the translation agree with the PSP's. Not
  runtime-verified, which is what keeps it below 95.
- **88**, MagEffect2's per-tick basis (`MagFloorFx_Update`): the PSP and PS2
  bodies agree row for row.
- **85**, the white tint: a static initialiser in each binary, read and
  matching. No runtime read of `0x08b3bf30`.
- **90**, every craft builds one and `craft+0x8bc` is the object: one
  caller, straight-line code, and the Enter/Exit listing.

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

- **Runtime confirmation.** Nothing on this page has been watched live; a
  PPSSPP read of `craft+0x8bc` -> `+0xac`/`+0xb0` world matrices on a
  magstrip, or a forced Show (`obj+0xb4 = 1`, bit `4` into both `+0x2c`) on the
  grid, would take the anchor from 88 to 95 and give a look reference.
- The sfx mechanism, now leaning toward "not attached to this effect at all"
  (see above) rather than a hidden emitter - not settled, since the
  per-track ambience reading is also unconfirmed.
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
- 2026-10-05: Anchor recovered (88). The 16-float block is identity with
  row 3's `y` overwritten by `[0x08ab0ef0] = -2.5`, so both models sit at
  `(0, -2.5, 0)` in the craft's frame; the 2026-09-02 "boilerplate" reading
  missed the `swc1 f12,0x34(sp)` in the call's delay slot. MagEffect1 is a
  translate-only child of the craft; MagEffect2 is a child of the scene root
  rebuilt each tick by `MagFloorFx_Update` (`0x0885962c`, named) on the
  track's up. Caller is `Craft_Construct` (`0x08841860`), which stores the
  object at `+0x8bc` (closes the `+0x8bc` question). Tint is white, from the
  static initialiser `MagFloorFx_InitTint` (`0x08859904`, created). All of
  it corroborated in the PS2 build (`0x00165688`, `0x001659a0`,
  `0x00165ce0`).
