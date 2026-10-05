# Ship visual-effect object constructors

Functions in `eboot.bin` (WipEout: Omega Collection, PS4, `CUSA05670`, EU),
`x86:LE:64:default`, imported with
[GhidraOrbis](https://github.com/astrelsky/GhidraOrbis) per
[toolchain.md#ps4](../../../reverse-engineering/toolchain.md#ps4). **The names
here are applied**, from [names.tsv](names.tsv). Found while checking the
lineage hypothesis [`vita-2048-eu-v104/README.md`](../vita-2048-eu-v104/README.md)
already confirmed for 2048 against `ps3-hdfury-eu` - does Omega Collection's
PS4 build share the same source tree too, and if so, does that let a name
recovered on one binary transfer to the other despite the architecture
change (x86-64 versus ARM Thumb-2 versus PowerPC64)?

## The lineage question, extended a third time

`search_strings("Backend")` against this binary returns 76 `.cpp` debug-tag
paths under `C:\WOPS4\Wipeout\Code\Backend\...`; the same search against
`vita-2048-eu-v104/eboot.elf` returns 82 relative paths under `Backend/...`
(no drive-letter prefix - a build-relative path rather than an absolute one,
the only difference). 74 of the 76 PS4 paths match a Vita path by suffix
exactly, filename for filename, subdirectory for subdirectory
(`General/Collision/Shape/KdTreeMeshShape.cpp`, `Ships/ShipCTRL.cpp`,
`Weapons/LeachBeamManager.cpp`, `World/WorldManager.cpp`, ...). The two
misses are `Weapons/BaseMine.cpp` and `Weapons/Bomb.cpp`, present on Vita and
not found in this binary's string table - not proof they were removed, since
[`vita-2048-eu-v104/race-hud-selection.md`](../vita-2048-eu-v104/race-hud-selection.md)'s
own caveat applies here too: a `search_strings` miss only means "not
independently corroborated", not "absent". **Omega Collection's PS4 build is
the same `Backend/` source tree 2048 and HD/Fury already share, recompiled a
third time**, not a fresh PS4-specific rewrite.

## `MagstripWake_Construct` - `0x012e38d0`

**Confidence: 85**

A constructor: sets a vtable pointer at offset 0, stores the literal
`"C:\WOPS4\Wipeout\Code\Backend\Ships\MagstripWake.cpp"` at `param_1[0xb]` -
the same tagged-allocation/tagged-object idiom
[`GameRoot_Construct`](../vita-2048-eu-v104/game-boot.md) and
[`RcsModel_Load`](../vita-2048-eu-v104/track-and-collision-loaders.md) rest
their own confidence on - looks up a resource named `"arc_anchor_point"`,
allocates and zero-initialises the object's working buffers, ORs `6` into a
flags field, and increments a live-instance counter before returning. Two
allocations further down build a small procedural texture (`HD_electric_arc_
8x8.gnf`, `HD_ElectricArc_Contact.gnf`) and a CRC-driven table lookup keyed
off the literal string `"AI track data"` - visual/effect setup specific to
this build, not present in the Vita version's much shorter equivalent.

**Evidence is the structural match against the same constructor on
`vita-2048-eu-v104`, decompiled side by side, not the filename tag alone:**

| | PS4 `FUN_012e38d0` | Vita `FUN_811aeee2` |
| --- | --- | --- |
| vtable store | `*param_1 = &PTR_FUN_01915060` | `*param_1 = &PTR_LAB_81221a9e_1_81511b70` |
| `__FILE__`-style tag, same field offset | `param_1[0xb]` = `"...MagstripWake.cpp"` | `param_1[0xb]` = `"Backend/Ships/MagstripWake.cpp"` |
| resource lookup by name | `FUN_012e42c0(param_2, "arc_anchor_point")` | `FUN_811aee9c(param_2, "arc_anchor_point")` |
| flags field | `*(undefined4*)(param_1+0xc) = 0x3006` then later `\| 6` | `param_1[0xc] = param_1[0xc] \| 6` |
| live-instance counter | `_DAT_01a1109c += 1` (rewritten `DAT_020e2f4c` region) | `DAT_818a64dc += 1` |

The field-offset match (`+0xb` on both, despite one binary being 32-bit ARM
and the other 64-bit x86) and the identical `"arc_anchor_point"` /
`\| 6` / instance-counter sequence are not something two independent
constructors converge on by chance - this is the same source function,
recompiled. The PS4 side additionally inlines texture and lookup-table setup
the Vita build calls out to separately (unread here), which is why its
decompile is longer without changing the shared skeleton above.

`MagstripWake` is the visible "electric arc" trail a ship leaves on a
magnetic strip pad - a WipEout-specific term, not a generic engine word,
which is part of why this match is trusted: a coincidental filename hit on a
word like `Init` or `Update` would not carry the same weight.

**Not yet checked**: address-identical cross-check against a second PS4
region/patch build (only one PS4 binary is imported so far, unlike the
Vita/PS3 pairs elsewhere in this project), and instruction-level
disassembly rather than only the decompiler's output.

## Same finding, other binary

The Vita-side half of this match
(`FUN_811aeee2` @ `0x811aeee2`) is recorded in
[`vita-2048-eu-v104/ships-effects.md`](../vita-2048-eu-v104/ships-effects.md)
under its own `names.tsv`, since a name applies to one binary's own database
and needs its own evidence page per [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md) -
this page and that one cite each other rather than one citing a name the
other's `names.tsv` has not actually recorded.

## 2026-10-05: the `MagstripWake` object end to end (magfloor-omega-re lane)

Read statically from the decompile and the vtable bytes, checked against the
Vita and PS3 builds (their own pages: [`vita-2048-eu-v104/ships-effects.md`](../vita-2048-eu-v104/ships-effects.md),
[`ps3-hdfury-eu/magstrip-wake.md`](../ps3-hdfury-eu/magstrip-wake.md)). No live
run was possible on a PS4, so nothing below is above the static-reading
ceiling; each row carries the evidence it rests on.

### The vtable at `0x01915060` (the object's `*param_1`)

| Slot | Address | Role | Name | Conf. |
| --- | --- | --- | --- | --- |
| 3 | `0x012e1260` | per-frame update `(float dt, wake)` | `MagstripWake_Update` | 65 |
| 5 | `0x012e2720` | queue the object into the render list | `MagstripWake_EnqueueRender` | 75 |
| 7 | `0x012e2770` | build and submit the vertex batches | `MagstripWake_Draw` | 65 |
| 9 | `0x012e3350` | destructor | `MagstripWake_Destruct` | 78 |
| 10 | `0x012e36d0` | deleting destructor (calls 9, then `free`) | `MagstripWake_DeleteSelf` | 72 |
| 0,1,2,4,6,8 | `0x0173c1xx`.. | base-class slots | not named | - |

- **Slot 5 is the strongest.** `0x012e2720` appends `{param_1, key}` to a
  per-frame list at `DAT_020e45e8 + 0x5a8` with key `0x4d000000 | (mode_depth & 0xfffff)`
  (or bare `0x4d000000` when `+8 == -1`). The PS3 build's `0x00109028` is the
  same body with the same `0x4d000000` key, so two independent builds agree
  (75, capped because the list owner is unnamed).
- **Destructor (78).** It undoes exactly what the constructor did: `_DAT_01a1109c -= 1`
  against the constructor's `+= 1`, the two `0x1b0` ribbon objects at `+0x88`/`+0x90`
  freed through `FUN_016fe980`, the `+0x98` arc pool freed, and the shared texture,
  shader and index-buffer handles released only when `DAT_020e2f4c` (a use count
  the constructor increments) reaches zero.
- **Update and Draw (65 each).** Read from one decompile; the PS3 slots do not
  line up one for one (see the PS3 page), so no second binary confirms the slot
  index. The per-arc numbers below are from this read alone.

### What the object holds

The constructor (`MagstripWake_Construct`, `0x012e38d0`) stores, with the
wake as `W` and the ship as `S` (`W+0xa8`, the constructor's `param_2`):

- `W+0xb8` = the `arc_anchor_point` locator, looked up by name through
  `FUN_012e42c0(S, "arc_anchor_point")`. This is a named locator **in the ship's
  own locator set**. The PS3 build prints
  `"**** WARNING **** : Ship has no arc_anchor_point locator"` when it is missing
  (`0x00782fd0`), which is the strongest evidence it is a ship-model locator and
  not a track marker (75). Which `Locators.vex` entries carry it is **not yet
  checked** (see Open).
- `W+0x88`, `W+0x90` = two `0x1b0`-byte ribbon objects built by `FUN_016fdef0`,
  sized by `S+0x68f4`. Both are driven with a lateral offset of
  `+/- 0.28 * clamp(speed - 40, 0, 120)` in `MagstripWake_Draw`'s tail
  (`FUN_016ff560`, called once per side), and are ticked and flushed from
  `MagstripWake_Draw` itself (`FUN_016ffb40`). Whether they are the trail-ribbon
  class [`docs/rendering/trail-ribbon.md`](../../../rendering/trail-ribbon.md)
  describes is **not established** (55).
- `W+0x98` = a `0x600` arc pool: **nine** arc slots of `0xa0` bytes starting at
  `+0x40`, loop bound `0x678` in the update.
- `W+0xa0` = the **active** byte (see the trigger below). `W+0x74/0x78/0x7c`
  are the activation state written when it flips on.

### The arc, per tick (update, `0x012e1260`, 65)

- Spawning is guarded by `W+0xa0` (the loop that ages live arcs runs regardless). When the ship's `+0x68fc` field is `0` or `2`
  and the global `DAT_01f998e8` has bit 0 set, the whole update is skipped.
  `+0x68fc == 0` is the local player (it is what picks `"MagStrip_Player"` over
  `"MagStrip_NPC"` below), so this reads as "do not show the wake on the local
  player's own ship in that configuration"; the meaning of `DAT_01f998e8` is unread.
- Spawn: `1 + rand() % 3` arcs per tick into free slots (life `<= 0`), each with
  life `0.2 + rand * 0.9 / 2^30` seconds (so 0.2 to 2.0), a random 8-by-8 atlas
  frame index (`rand() % 64`, stored at `+0xd8`, advanced by one each tick), a
  width `3.5 + rand * 3.26e-9` (3.5 to 10.5) and a colour scalar. Spread and brightness are blended by
  `t = clamp((speed - 10) / 190, 0, 1)` against slow-speed constants
  (`0.6`, `3.0`, `15.0`): below speed 200 the arc is narrower and shorter-ranged.
  `speed` is `body+0x4b8`, the same field the PS3 build reads at `+0x4c4`.
- Each arc's end point is walked along the **AI track data spline** (the
  `"AI track data"` CRC lookup in the constructor, `DAT_020e2fc0`, segment stride
  `0x60`, up to 20 segments) to a point `fVar29 * (dist / spacing)` ahead, so the
  arc follows the strip instead of flying off in a straight line (60: the walk is
  read, the exact reference distance is not).
- Per arc and tick: `life -= dt`, the frame index steps, the jitter terms are
  smoothed `x = rand*a + 0.85*x_prev` (decay `0.85`), and the arc dies early with
  probability `clamp((|anchor - ship|^2 - 368.64) * 0.0086685, 0, 1)` when the
  anchor has fallen behind the ship (`dot < 0`): arcs are shed when the ship leaves
  them behind.

### The draw (`0x012e2770`, 65)

- For each of the nine slots with `life > 0` it writes **six segments of a
  four-vertex strip** (24 floats per vertex pair, `pos.xyz, ABGR, uv`) into a
  triple-buffered vertex array (`W+0x98 + 0..2`, rotated by `W+0x30`), sampling the
  8-by-8 atlas cell `frame % 8, frame / 8` (cell size `0.125`) for the arc body,
  then a **contact quad** from the second texture (`uv` 0..1) at the anchor end.
- Colour is `0xb2RRGGBB`: alpha **fixed at `0xb2` (178)**, RGB grey equal to
  `intensity * 255` (the arc body) or `intensity * 76.5` (the soft outer strip) or
  `contact_scale * 255` (the contact quad). So tint is white; the texture carries the colour.
- Two draw calls are pushed (one for the arc strip, one for the contact quads)
  through a shader with the vertex inputs `inPos` (float3), `inCol` (ubyte4n) and
  `inTex`, constants `kWorldViewProj` and `kIntensity`, and sampler `inTex`
  (named in the constructor; the programs are `MagStripArc_vp`/`MagStripArc_fp`).
  `kIntensity` is written from `W+0x5f8`, which update sets to a value read from
  `DAT_020e52a0 + 0x1e0` (a global tuning block, unnamed).
- **Blend state is not decoded.** The constructor configures it through
  `FUN_012091a0(.., 1, 0, 1)` and `FUN_012091c0(.., 0, 0, 5)` into `DAT_020e2f88`.
  Reading those two functions is the next step; do not guess additive from the
  texture names.

### The trigger (who sets `W+0xa0`)

The per-ship update `FUN_01762e80` (`0x017659bb`) calls a **virtual on the ship at
`vtable+200`**. When it returns non-zero and `ship+0x71f5 & 0x10` is clear, the wake
is **activated** (`W+0xa0 = 1`, `W+0x7c = -1.0`, `DAT_01a11098 += 1`, the live-wakes
count), otherwise it is **deactivated** (`W+0xa0 = 0`, count decremented). A
second field, `ship+0x8860` (the 2048-style POB, below), is switched on and off by
the same predicate through the `0x1e0` flag bit of its emitter chain. This
predicate is the **"ship is over a magstrip"** state: the HD build logs it by name,
`"Send due m_overMagStrip change - now %i"` (`0x007a9320`), as a replicated
network field (75). The virtual itself (slot `+200`) was **not read**; the
magstrip-floor contact test lives behind it (Open).

### One effect or two: `DAT_01f999e4 < 0x17` is the switch (80)

`DAT_01f999e4` is the mode selector `weapons.md` and `billboards.md` already
document (`ModeManager_ConstructByMode` switches on it; `Rocket_Construct` uses
`< 0x17` to choose `Data/Weapons` over `Data/Weapons2048`). Against that:

- **The procedural wake is built only for `mode < 0x17`.** Both ship constructors
  (`FUN_01309e90`, `FUN_0130e540`) wrap `MagstripWake_Construct` in
  `if (DAT_01f999e4 < 0x17)` and store the result at `ship[0xe18]` (`ship+0x70c0`);
  otherwise they store 0.
- **The particle effect is built only for `mode >= 0x17`.** The ship constructor
  `FUN_0176d240` returns early for `< 0x17` and for `>= 0x17` plays
  `WO_MAGSTRIP_ZONE` or `WO_MAGSTRIP_SPARKS` (`.POB`, `FUN_01713a50`, anchor
  `ship+0x8860`, handle stored at `ship+0x8848`). `ZONE` is chosen when `(DAT_0203a728 && FUN_01669270())` or
  `(DAT_01e31910 && *(DAT_01e31910+0x21c) == 0)`, else `SPARKS`. A `0x8181` mask over
  `mode - 6` appears in the same condition but is dead there (it can only be true for
  modes 6, 13, 14 and 21, all below `0x17`).
- **The disc agrees:** both `.POB` files ship only under `Data/particles2048/` in
  the Omega archives, none under `Data/particles/`; the arc textures ship under
  `data/Tex/` and `Data/particles/Tex/`.

So Omega's HD-lineage modes get the **procedural arc wake**, its 2048-lineage
modes get the **particle effect**, and never both on one ship. (The Vita 2048
build tests a different predicate and, as read, builds both: see its page.)

### The sound (60)

`Ship_StartMagstripSound` (`0x012f8c70`, `FUN_012f8c70` before naming, conf. 55) creates a sound group named
`"MagStrip_Player"` (when `ship+0x68fc == 0`, group arg 1) or `"MagStrip_NPC"`
(arg 0), through `FUN_01736840(DAT_020f95b8, name, 0, arg)`, stores it at
`ship+0x5f38`, anchors it to `ship+0x70c8 + 0x10`, sets a `[300.0, 50.0]` pair at
`+0x48` and then starts the cue **`"_magstrip01"`** on it
(`FUN_01731480(1.0, group, 0, 0, "_magstrip01", ship+0x5f30, -1, 0)`). The cue
exists: `shiphd.bnk` in the HD archives carries `~magstrip01` and a
`### Magstrip` section (the `~` is a leading marker the HD strings keep; Omega
spells it `_`). `Ship_StopMagstripSound` (`0x01312770`, conf. 55) is its
inverse: it sets the stop bit `0x10` on the instance at `ship+0x5f30` and on every
voice of the group at `ship+0x5f38`, then clears both.

**The call pattern is not what a plain enter/exit would give, and is unresolved.**
In `FUN_01762e80` the stop is reached from the *deactivate* branch only, and that
branch also sets `ship+0x5f48 = 1`, which is the flag the later `FUN_012f8c70`
call tests. No separate "start on enter" site was found: `FUN_012eaf20`, the other
caller of `FUN_012f8c70`, has no direct callers (virtual). The second argument of
`FUN_012f8c70` (forwarded to `FUN_01731800(group, 2, arg)`) is not visible at the
call and was not recovered (the decompile shows one argument; it is probably a
float in `xmm0`). Do not implement the sound law from this page alone.

### Names added

`names.tsv` rows for the five methods above and the two sound helpers; the
constructor row is unchanged. `FUN_0176d240` is the per-ship constructor tail that
creates the POB, named by nobody yet (a hypothesis only: it also builds
`WO_DUST_TRAIL`, `WO_DUST_TRAIL_TARMAC` and `WO_WATER_TAIL`, so it is a race-ship
effect-setup function, conf. 50, not renamed).

## 2026-10-05, magstrip-omega-law lane: the contact predicate, the blend, the sound law, the anchor census

Static reading of `eboot.bin` plus the HD and Omega discs; nothing run live (no PS4
emulator). Every address below is a Ghidra address in `/omega/eboot-ps4-omega-eu.bin`.
Scratch decompiles sit under `data/scratch/magstrip-omega-law/`.

### The over-strip predicate (`vtable + 200`): `Ship_IsOverMagStrip`, conf 85

`FUN_01762e80` calls `(**(code **)(*ship + 200))(..., ship)` at `0x017659bb`. The
ship vtable `0x0192da00` carries `FUN_01773ef0` (the constructor tail that reaches
`FUN_0176d240` and so `FUN_01309e90`) at `0x0192db38`, slot 39; its sibling `0x0192d840`
(stored by `FUN_01772fb0`) carries `FUN_0176d240` itself in the same slot, at a constant
`-0x1c0`. Slot 25 (`+200`) of **both** holds `0x01768a40`. Read as bytes, not inferred.
The call passes the ship alone (`MOV RDI, RBX` then `CALL [RAX+0xc8]`, `0x017659bb`).

`Ship_IsOverMagStrip` (`0x01768a40`):

```text
if (ship+0x68fc | 2) == 2:            ; slot kind 0 or 2 - a locally simulated ship
    return Ship_IsOverMagStripBase(ship)       ; 0x012ff590, same test
else:                                 ; a remote (replicated) ship
    return (net_table[ship+0x68f4].byte41 >> 1) & 1
         ; DAT_020fd608 + 0x41 + slot * 0x21 ; bit 1 of the replicated state byte
```

`Ship_IsOverMagStripBase` (`0x012ff590`, conf 80): for kind 0 or 2 it returns
`*(char *)(*(ship + 0x6518) + 0x5d0) != 0` - a byte on the ship's controller object
(`ship+0x6518`, the object whose `+0xc8` the update also reads). Other kinds fall to
the same replicated-table bit, behind a `*(ship+0x6510) == 7` early out.

**Who writes `controller+0x5d0`: `FUN_0131b510`, conf 90 for the write, 65 for the
function's name.** Read straight off the decompile:

```text
old = c+0x5d0
if c+0x328 == 0:  c+0x5d0 = 0
else:             c+0x5d0 = (c+0x4d0 == 3)
if  old && !c+0x5d0:  FUN_01312b80(c+0x2c8)       ; falling edge  (exit rumble, below)
if !old &&  c+0x5d0:  FUN_01315b30(c+0x2c8)       ; rising edge   (enter rumble)
```

`c+0x328` is the result of the **fifth** of seven `FUN_012582a0` calls (a segment
against the track's collision triangles; the other results land in `+0x324`..`+0x32a`,
records of `0x60` bytes from `+0x330`, so the fifth record is `+0x4b0`), and `c+0x4d0`
is that record's `+0x20`. `FUN_012582a0` (`0x012582a0`) fills a hit record: `+0x00` point,
`+0x10` normal, **`+0x20` = a byte read from the hit triangle's own attribute array
(`mesh+0x20`, indexed by triangle)**, `+0x30..+0x50` the triangle. So the predicate is:

> **over a magstrip = the fifth surface probe hit a triangle whose surface-type byte is `3`.**

That byte is the **HD-lineage per-triangle surface class**, and the value is not an
accident: `oag_vex::kdcol::class_of` documents the table `TrackCollision_MeshFromNode`
(Vita `0x8126f800`) passes to the mesh builder - `Floor Collision` `0x3b9` = 2,
**`Mag Floor Collision` `0x3e6` = 3**, `Wall Collision` = 4, `Reset Collision` = 7 - and
re-derived over 170,744 HD/2048 triangle pairs. So `+0x4d0 == 3` is "the triangle came
from a `Mag Floor Collision` node", the class HD authors (`docs/formats/hd-status.md`:
Talon's Junction has 14 such objects). It is also the same literal Pulse compares: `oag_vex::collision::SurfaceKind::MagFloor.surface_type()`
is `Some(3)` and `Ship_CastHoverProbes`'s tail writes `craft+0x240` from a probe
"restricted to surface type 3" and edge-detects it into `Ship_MagFloorEnter`/`Exit`
(`psp-pulse-usa/engine.md`, "The tail of `Ship_CastHoverProbes`"). **Same law, same
shape, same two edge handlers; `+0x5d0` is Pulse's `craft+0x240`** (conf 85, the shape
and the literal agree; the probe's index differs - the Pulse page calls it the fourth
cast, Omega's is the fifth of seven results - and the segment endpoints were not
recovered, so "same probe" is the reading, not a measurement).

**The equivalent in our physics** is `oag_physics::maglock::probe(..)` returning
`Some`, i.e. `ShipState::mag_contact.is_some()` (`crates/physics/src/ship.rs`, set in
`forces.rs`): a raycast whose hit surface is `Surface::MagFloor`. It is **not** the
mag-lock blend (`craft+0x240` ramps `0.2` per frame and lingers; the wake switches on the
**instantaneous** contact, with no ramp). Edges are `contact && !previous` and
`!contact && previous` of that one boolean.

Other readers of the same two bytes: `FUN_012ed9b0` packs it into the replicated state
(`byte41 = (byte41 & 0xfffc) | bit0 | (c+0x5d0) << 1`, stride `0x21`), which is the
`m_overMagStrip` field HD logs - see the HD page. And `FUN_01762e80`'s activate branch
reads `c+0x4d0` too: for surface classes `2, 3, 10, 11, 12` (mask `0x703` over
`class - 2`: `Floor`, `Mag Floor`, and 2048's own measured-floor bytes `10`..`12`, so
"any drivable floor") it copies the probe record's point and normal (`c+0x4b0`/`+0x4c0`) into
the 2048-style POB's anchor frame at `ship+0x8860..` (`param_2[0x110c..0x1113]`, conf 60:
the copy is read, the consumer is not). The activation is also vetoed when `ship+0x71f5 & 0x10` is set (the
flag's meaning is unread). `FUN_0131b510` also **returns before touching `+0x5d0`** while
`c+0x2d8 == 0` or `c+0x2c5 & 4` (checked at entry and again after the probes): the flag
is then held, no edge fires, and a held "on" stays on. Neither field is identified.
(`FUN_0125e9b0`'s `+0x5d0` test and clear is a different object: the global
`DAT_02039628`, the pause-menu one `FUN_012e6890` also writes. Not a reset path.)

### The blend: **additive RGB**, decoded (conf 85)

`FUN_012091a0`, `FUN_012091c0`, `FUN_01209170`, `FUN_01209180`, `FUN_012091f0` are bit-field
packers into one 32-bit word, and the layout is **AMD GCN `CB_BLEND0_CONTROL`**
(the PS4 GPU's register), field for field:

| bits | field | packer |
| --- | --- | --- |
| 0-4 / 5-7 / 8-12 | colour source factor / combine function / colour destination factor | `FUN_012091a0(w, src, fn, dst)` |
| 16-20 / 21-23 / 24-28 | alpha source factor / function / destination factor | `FUN_012091c0(w, src, fn, dst)` |
| 29 | separate alpha blend | `FUN_012091f0(w, on)` |
| 30 | blend enable | `FUN_01209180(w, on)` |
| all | clear | `FUN_01209170` (stores `0`) |

The enumeration is confirmed by what other callers pass, not assumed: the opaque pass
(`FUN_0177f510`, `FUN_01780610`, `FUN_01782240`, three sites) is `enable 0`, colour
`(1, 0, 0)` and alpha `(1, 0, 0)`, i.e. ONE/ADD/ZERO, a replace; the ordinary
alpha-blended pass (`FUN_01710ec0`, `0x01710f01..0x01710f36`) is `enable 1`, separate
alpha, colour **`(4, 0, 5)`** = SRC_ALPHA/ADD/ONE_MINUS_SRC_ALPHA and alpha `(0, 0, 1)` =
ZERO/ADD/ONE. That pins `0` ZERO, `1` ONE, `4` SRC_ALPHA, `5` ONE_MINUS_SRC_ALPHA, and
`0` as ADD.

`MagstripWake_Construct` (`0x012e38d0`, `0x012e3c6a`..`0x012e3ca0`) builds
`enable 1`, `separate alpha 1`, colour **`(1, 0, 1)`**, alpha **`(0, 0, 5)`** and stores the
word (`0x65000101`) in `DAT_020e2f88`; `MagstripWake_Draw` copies that word into the
draw item at `+0x58` for both batches (`0x012e2fd3`, `0x012e3194`).

> **Colour: `out.rgb = src.rgb * 1 + dst.rgb * 1` - additive, with the blend stage NOT
> scaling the source colour by its alpha. Alpha: `out.a = dst.a * (1 - src.a)`** (it never
> reaches the frame in an opaque target, so it is irrelevant to the picture).

So "additive" is now decoded rather than guessed. What the decode does not say: the
vertex alpha (`0xb2`) is not a blend factor, but the fragment program may still use it.
The brightness the vertices carry is `intensity * 255` for the body, `intensity * 76.5`
for the soft strip, `contact_scale * 255` for the contact quad. `MagStripArc_fp` is Orbis
shader code and was not decompiled, so **the alpha handling is chosen, not measured**; the
batch state is measured. The second state word, `DAT_020e2f68` (`FUN_01209170` output), is zero (depth
and cull state, meaning not decoded). Depth-test and write for the draw are therefore
**unknown**, not "off".

### The sound start law (conf 80 for the shape, 55 for the arguments)

`ship+0x5f48` (`param_2[0xbe9]` byte) is an **arming flag**, and it is initialised to
**`1`** in both ship constructors (`0x0130a3d7` in `FUN_01309e90`, `0x0130ef35` in
`FUN_0130e540`); the earlier page missed this and so found no "raise" site. The law,
inside `FUN_01762e80` (and its twin `FUN_012eaf20`, the same block at `0x012ed452`..
`0x012ed482`):

```text
each tick:
  if Ship_IsOverMagStrip(ship) && !(ship+0x71f5 & 0x10):      ; the activate branch
      (switch the wake / POB on if it was off)
      if ship+0x5f48:                                           ; armed
          Ship_StartMagstripSound(ship, arg)                    ; FUN_012f8c70
          ship+0x5f48 = 0                                       ; disarm
  else if the wake (or POB) was on:                             ; the deactivate edge
      (switch it off)
      Ship_StopMagstripSound(ship)                              ; FUN_01312770
      ship+0x5f48 = 1                                           ; re-arm
```

The start sits **inside** the activate branch (the previous page read it as outside it);
the asm confirms it: `0x01765d89` `CMP [ship+0x5f48], 0` / `0x01765dad` `CALL 0x012f8c70` /
`0x01765db2` `MOV [ship+0x5f48], 0`. So: **start the `_magstrip01` cue once on entering
the strip, stop it on leaving.** The stop is only reached on a real on-to-off edge (the
deactivate code runs only when the wake was active). `arg` (the `esi` of the call,
forwarded to `FUN_01731800(group, 2, arg)`): `2` for a ship whose `ship+0x68fc != 0`,
and `byte(ship+0x648b) ^ 1` for the local ship (`ship+0x68fc == 0`); `ship+0x648b` is
written by the pause-menu function `FUN_012e6890` (the `OPT_INT`/`OPT_CLOSE` one), so it
is **probably a paused or menu-open flag** (50; not renamed), making `arg` a "heard
while the game is running" selector. The group is `MagStrip_Player` for the local ship,
`MagStrip_NPC` otherwise, created on first use at `ship+0x5f38`, anchored to
`*(ship+0x70c8) + 0x10` (the ship's own transform), with the pair `[300.0, 50.0]` at
group `+0x48` (a distance pair, meaning unread). Whether the cue's waveform loops is a
property of the bank entry (`~magstrip01`, HD `shiphd.bnk`), **not read here**; the
stop-on-exit law is what makes it a "while over the strip" sound either way. Only the
local player (`ship+0x68fc == 0`) hears the rumble below; the sound plays for every ship.

### The rumble: three handlers named by the edges (conf 70)

`FUN_01315b30` (rising edge) and `FUN_01312b80` (falling edge) both gate on
`ship+0x68fc == 0` (the local player) and queue a rumble item into the pad's ring
(`pad = DAT_020fd940 + 0x80 + ship+0x8290 * 0x1120`, or the first live pad when that is
`-1`): the **enter** handler queues the object at `pad+0x150` and then the one at
`pad+0x158`, keeping the second's handle in `ship+0x64e0`; the **exit** handler queues
`pad+0x160` and marks that held handle finished (`handle+0x10 = 1`). The rumble library
loads its `.xml`s in the order **Enter, Travel, Exit** (`FUN_0174b8d0`, strings
`0x0183a751`, `0x0183a776`, `0x0183a79c`, stored at library `+0xd0/+0xd8/+0xe0`, which
line up with the pad's `+0x150/+0x158/+0x160` at the constant offset `0x80`). So the
three `*_mag_rumble.xml` files are: **enter on the rising edge, travel as the held
handle while over the strip, exit on the falling edge**, local player only. The `+0x80`
alignment is read from the offsets, not from a store (65).

### `arc_anchor_point` census: every hull, both titles (conf 90)

`crates/game/tests/magstrip_anchor_ground_truth.rs` (`#[ignore]`d) reads every `Locators.vex`
through `oag_assets::Archives` and `oag_vex::vex::matrix::named_class_world_transforms`.
HD: 39 copies (`data/ships/<hull>/locators.vex`); Omega: 38 copies
(`Data/art/published/hdships/<hull>/Locators.vex`). **Every file carries exactly one node
named `arc_anchor_point`, class id `110`, on the hull's centreline (`x` within
`0.0004`), and HD and Omega agree on every hull to four decimals** (Omega is a straight
carry-forward). Model space, row-major, translation in row 3; `y` is the offset below
the hull origin, `z` along the hull. Base hulls (`_c1`/`_n1` variants repeat their own
row, equal to each other):

| hull | `(x, y, z)` | hull | `(x, y, z)` |
| --- | --- | --- | --- |
| ag_systems | `(0, -0.7356, 1.3616)` | harimau | `(0, -0.3025, -0.2922)` |
| assegai | `(0, -0.7321, -3.2227)` | icaras | `(0, -0.6551, -1.3131)` |
| auricom | `(0, 0.3942, -3.2736)` | mirage | `(0, -1.4144, -4.3541)` |
| detonator | `(0, -0.0137, -4.5266)` | piranha | `(0.0004, -0.6471, 1.7252)` |
| egx | `(-0.0000, -0.3025, -0.2922)` | qirex | `(0, 0.1809, -4.2535)` |
| feisar | `(0, -0.4459, 0.2239)` | triakis | `(0, -0.3701, -1.8527)` |
| goteki | `(0, -0.2204, -3.7911)` | zone | `(0, -0.0137, -5.0186)` |

The `_c1`/`_n1` rows differ from their base hull (e.g. `auricom_c1` is
`(0, -1.5407, -2.9819)`); the test prints all 38. The anchor is one point per hull, not a
set, so a wiring member takes `world = ship_model * anchor`. The name is a node name in
the locators file, not a CRC: the string is in the file.

### Names added

`names.tsv`: `Ship_IsOverMagStrip` (85), `Ship_IsOverMagStripBase` (80),
`Craft_UpdateSurfaceProbes` (65, the function holds seven probes and the edge test; the
name covers the part read), `Ship_MagStripEnterRumble` / `Ship_MagStripExitRumble` (70),
and the five blend packers `BlendState_SetColor`, `BlendState_SetAlpha`,
`BlendState_SetSeparateAlpha`, `BlendState_SetEnable`, `BlendState_Clear` (85).
`Ship_StartMagstripSound` / `Ship_StopMagstripSound` keep their rows (55); the law above
raises the shape to 80 for a reader of this page.

| address | name | conf |
| --- | --- | --- |
| `0x01768a40` | `Ship_IsOverMagStrip` | 85 |
| `0x012ff590` | `Ship_IsOverMagStripBase` | 80 |
| `0x0131b510` | `Craft_UpdateSurfaceProbes` | 65 |
| `0x01315b30` | `Ship_MagStripEnterRumble` | 70 |
| `0x01312b80` | `Ship_MagStripExitRumble` | 70 |
| `0x012091a0` | `BlendState_SetColor` | 85 |
| `0x012091c0` | `BlendState_SetAlpha` | 85 |
| `0x012091f0` | `BlendState_SetSeparateAlpha` | 85 |
| `0x01209180` | `BlendState_SetEnable` | 85 |
| `0x01209170` | `BlendState_Clear` | 85 |

### Still open after this pass

- Which probe the fifth `FUN_012582a0` call is (its two endpoints, `local_db8`/`local_dc8`).
- The cue's loop flag and the meaning of `ship+0x648b`, `ship+0x71f5 & 0x10`.
- The `MagStripArc_fp` fragment program (alpha handling) and the draw's depth state.
- HD's own `0x00109858`/`0x001095e0`/`0x00109720` arc build (not read).

## 2026-10-05, magstrip-wire-hd lane: `MagstripWake_Update` and `_Draw` re-read, and wired on HD

Decompiled again from `/omega/eboot-ps4-omega-eu.bin` (`0x012e1260`, `0x012e2770`); static, PS4 only, 65. The
implementation is `oag_fx::magstrip` (module doc lists each rule), `oag_raceplay::magstrip_wake` and
`oag_sound::sfx::magstrip`. HD builds the class (`oag_title::weapons::WeaponModels::magstrip_wake`,
`oag_hd::race::WEAPON_MODELS`); Omega's table is still `EMPTY` and inherits it when it races.

- **Corrections to the earlier reading.** `rand()` is used as a 30-bit source: every constant is
  `k / 2^30` with a `-0.5` or `-s` centring (life `0.2 + 0.9u`, contact scale `3.5 + 3.5u`, throw
  `0.55 + u`), so the "0.2 to 2.0" above is `0.2` to `1.1`. Arcs age whether or not the ship is over the strip
  (`W+0xa0` gates spawning only), so an arc outlives the strip by up to its life.
- **Per-arc record (`0xa0` bytes at `pool+0x40`):** `+0x40` life, `+0x60` end point, `+0x70..+0x80` five jitter terms,
  `+0x84` spread (`0.8`, or blended to `0.6` below speed 200), `+0x88` body brightness, `+0xd0` contact scale,
  `+0xd4` contact brightness, `+0xd8` atlas frame; `+0x90/a0/b0/c0` the contact diamond's corner vectors.
- **Per tick:** `scale = 0.525u + 0.525 + 0.85 scale`, `brightness = 0.0862u + 0.01875 + 0.85 brightness`,
  `glow = 0.0975u + 0.0075 + 0.85 glow`; an arc with the end behind the ship's nose (`dot(fwd, end - ship) < 0`) is
  shed with probability `clamp((|end - ship|^2 - 368.64) * 0.0086685, 0, 1)`; jitter `j = 0.85 j + 0.15 s (2u - 1)`.
- **Draw:** start = anchor + `0.4` toward the end flattened onto the ship's up; width vector =
  `normalize(cross(start - eye, start - end)) * 0.8`; five centre points at `t = 1/6..5/6` on start-to-end pushed by
  `jitter[i] * width`, then the end. Six quads, `(prev -w, prev +w, next +w, next -w)`, `u` over one atlas column,
  `v` stepping `1/48` down from the cell's bottom; the last quad's far edge is colour `0.3`. Colour grey
  `(uint)(x * 255)` with alpha `0xb2`. Contact: a diamond at the end, corners `-T, -L, +T, +L` scaled by `+0xd0`, uv
  `(0,0) (1,0) (1,1) (0,1)`.
- **Unread, chosen:** `DAT_02134210..20` (jitter scales) and `DAT_020e52a0 + 0x1e0` (`kIntensity`) are zero in the
  image (run-time tuning); `1.0` and `INTENSITY = 3.0` are this port's. The end point's spline walk is replaced by our
  spline (`ahead` metres, then across the road). The two speed ribbons are not drawn.
- **Textures** `Data/Tex/HD_electric_arc_8x8.gtf` (512x512, 64 purple lightning frames, one bolt a cell, `v` along it) and
  `HD_ElectricArc_Contact.gtf` (64x64) decode through the existing `.gtf` reader.
- **Sound:** `~magstrip01` in `shiphd.bnk` is a **35-waveform tree** the reader flattens (about a fifth loop);
  the port draws until a looping leaf comes up. Group `MagStrip_Player`/`_NPC` has no mixer counterpart.
- **Predicate on our side:** `ShipState::mag_contact` is sticky (last hit) and the probe result is a local, so the instantaneous
  contact is read back off the blend with `mag_floor_fx::contact_this_tick`, exact for every ramp sequence.
