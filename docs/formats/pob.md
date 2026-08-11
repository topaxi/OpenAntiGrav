# Particle system

**Status: partial, and the two halves are at very different depths.** The
container, the slot table (including the pointer fixup it drives at load
time, confirmed live), the name field, and **the emitter tree** are all
decoded, implemented in
[`oag-formats::pob`](../../crates/formats/src/pob.rs) and validated against
all 35 `.pob` files on the PSP disc and all 41 on the PS2 disc. What is
**not** decoded is the record layout at a *slot-resolved* target: 43% of
them are readable developer strings (texture paths, layer names), and the
rest are floats whose field boundaries are still unread.

The distinction matters because it has been flattened to "the payload is
undecoded" more than once. The emitter records - schedules, shapes, speeds,
lifetimes, colour tables, channel blocks, the child/sibling tree - are read
at fixed offsets and need no fixup at all; see "The emitter record layout is
decoded" and "The parser walks the tree" below. Only the slot table's own
targets are unread.

These are the `SYSP` blobs under `Data\Psys\` in `Data.wad` - one per authored
effect, loaded by `Data\Psys\%s.POB` (built at `FUN_089156a0` in the PSP
`BOOT.BIN`). They back essentially every particle effect the game draws that
is not the ship exhaust: weapon impacts and trails, ship collision sparks and
death sparks, environmental effects (`WO_RAIN`, `WO_SNOW`, `WO_QUAKE`). The
exhaust flare and trail are a separate, hand-authored code path with no `.pob`
involvement - see [`oag_render::exhaust`](../rendering/README.md).

## Container

```text
+0x00  char[4]  "SYSP"
+0x04  u32      header length (16) + payload length
+0x08  u16      slot count, entries in the table at +0x10
+0x0a  u16      1 in every file seen
+0x0c  u32      1 in every file seen
+0x10  slot[count], 4 bytes each: u32, or 0xffffffff for an unused slot
       ...      32-byte NUL-terminated name, immediately after the table
       ...      the emitter tree, whose root record starts at that name
```

`+0x04` is redundant with the blob's own length - `HEADER_LEN(16) +
payload.len()` - and agrees exactly on all 35 files, which is what says the
table and name-field boundaries are being read at the right offsets rather
than plausibly. It is validated on parse, not exposed on the parsed struct.

The name field's position was the thing that needed checking: a single sample
first read as a fixed `+0x80`, which happens to be correct for one file only
because that file's slot count (28) makes `0x10 + 28*4` equal `0x80` by
coincidence. It is not fixed - it sits **immediately after the table**,
whatever the table's length, and this is confirmed on all 35 files at once by
the check below rather than by inspecting the string.

## Recovering all 35 names by their own hash

Every `.pob` blob is itself a WAD entry, named by a CRC-32 variant over its
path (`wad.md`'s [name hash](wad.md#the-name-hash)). Reading the 32-byte name
field, building `Data\Psys\<name>.POB`, and hashing it with
[`wad::hash_name`](../../crates/formats/src/wad.rs) reproduces the blob's own
WAD entry hash **on all 35 files**, which is both the confirmation that the
name field sits where this document says it does and, as a side effect, the
full recovered name list:

```text
WO_BLUE_WELDER, WO_BOMB_SMOKERING, WO_CANNON_SPARKS, WO_LEACHBEAM_CHARGING,
WO_LEACHBEAM_ENERGY, WO_MINE_EXPLO, WO_MISSILE_BOUNCE, WO_MISSILE_EXPLO,
WO_MISSILE_HEAD, WO_MODESTO_STEAM_A, WO_PLASMA_FLASH, WO_PLASMA_HEAD,
WO_QUAKE, WO_RAIN, WO_RAIN_LENS, WO_REPULSER, WO_REPULSER_BLAST,
WO_ROCKET_EXPLO, WO_ROCKET_EXPLO_TRACK, WO_ROCKET_FLARE, WO_SHIP_COLL_SPARK,
WO_SHIP_COLL_SPARK_DAMAGE, WO_SHIP_COLL_SPARK_NODAMAGE,
WO_SHIP_COLL_SPARK_TRAIL, WO_SHIP_COLL_SPARK_TRAIL_SMOKE,
WO_SHIP_DEATH_SPARKS, WO_SHIP_EXPLOSION, WO_SHIP_FXNODE_EXPLO,
WO_SHIP_SPARK_DAMAGE_LEACHBEAM, WO_SHURIKEN_BOUNCE, WO_SHURIKEN_EXPIRE,
WO_SHURIKEN_HEAD, WO_SHURIKEN_TRAIL, WO_SNOW, WO_WEAPON_ABSORB
```

`WO_SHIP_COLL_SPARK`, `WO_SHIP_COLL_SPARK_DAMAGE` and
`WO_SHIP_COLL_SPARK_NODAMAGE` are three distinct, separately authored
particle systems for the effect `oag_render::sparks` draws (see
[contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md);
`_DAMAGE` is the tree it loads and `WO_SHIP_COLL_SPARK` is one emitter
*inside* it as well as a file of its own), and `WO_SHIP_COLL_SPARK_TRAIL` and
`WO_SHIP_COLL_SPARK_TRAIL_SMOKE` are two more in the same family.

## The slot table is a pointer-fixup table

A slot's raw value is not the offset of its data. It is the offset of a
**fixup site**: a 4-byte field, relative to the resource's own runtime base
(`resource_base = HEADER_LEN + slot_count * 4`, i.e. exactly where the slot
table ends), holding a second, baked offset from that same base. At load
time the game adds the base to whatever is stored there, converting an
on-disk relative offset into a live pointer in place:

```text
fixup_site = resource_base + slots[i]
target     = resource_base + read_u32(fixup_site)   // read *before* the add
```

[`ParticleSystem::resolve_slot`](../../crates/formats/src/pob.rs) replays
this arithmetic entirely from the file's own bytes.

This closes the question the previous pass left open ("slot 0 is exactly
1220 on every file, which no per-file offset can be, so what is it") -
1220 is not itself meaningful data, it is simply the first file's fixup
*site* offset, which happens to recur because the earliest channels are
authored in the same order across most effects. The real content lives at
whatever the site's stored value resolves to, which does vary per file, per
slot, and is not a small closed vocabulary at all once resolved.

**Evidence, at the strongest tier this project's rubric has:**

- `FUN_088f8e38(base, resource_base, table_ptr)`, called unconditionally from
  the generic resource loader `FUN_088f3540` the first time any container of
  this exact shape (count-at-`+8`, table-at-`+0x10`) loads, decompiles to
  precisely the two-line loop above: skip `0xffffffff` slots, otherwise
  `*(resource_base + slot) += resource_base`. Not `.pob`-specific - this is
  shared engine-wide fixup machinery.
- **Confirmed live.** A PPSSPP session (`PPSSPPSDL` + the websocket debugger,
  `scripts/ppsspp_debugger.py`) broke at `FUN_088f8e38`'s entry during the
  real "loading the race" preload pass and read all 26 real fixup sites of
  `WO_SHIP_COLL_SPARK_DAMAGE` before the call, then again immediately after:
  every single site's post-fixup value minus its pre-fixup value equalled
  `resource_base` exactly - `26 of 26`, no exceptions - and the pre-fixup
  values read from live PSP RAM matched the file's own bytes exactly, byte
  for byte.
- **Corpus-wide, statically**: replaying the two-hop resolution from file
  bytes alone lands every fixup site and every resolved target in bounds on
  **all 436 real slots across all 35 files**, with zero exceptions.

Confidence **92**: a runtime trace (rubric ceiling 94 until a second binary
corroborates it) plus an exact arithmetic invariant across the whole corpus -
the two strongest evidence classes the rubric has, agreeing.

### The preload happens as a batch, and that is how the trace was taken

`.pob` files are not loaded lazily on first use. Breaking on the generic
`Resource_LoadFile` (`0x08943330`) while driving a live session from cold
boot through menu navigation into a Time Trial caught **zero** `.pob` loads
until the "loading the race" step, where four loaded consecutively -
`WO_SHIP_COLL_SPARK_DAMAGE`, `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`,
`WO_SHIP_COLL_SPARK_NODAMAGE`, `WO_SHIP_DEATH_SPARKS` - at path-string
addresses `0x08a886f8`, `0x08a88720`, `0x08a88750`, `0x08a8877c`, each
0x28-0x30 bytes apart. That spacing is consistent with a fixed-size array of
preload entries baked into the executable, which is a better lead for
finding the still-unlocated indirect call site than the function address
itself: searching for xrefs to *that array* (or to the four path-string
addresses above) is untried and more promising than re-searching
`FUN_089156a0`'s own address, which both `get_function_callers` and
`get_xrefs_to` already returned nothing for.

Separately notable: only 4 of the 5 `WO_SHIP_COLL_SPARK*` files preloaded for
this ship+track combination - not `WO_SHIP_COLL_SPARK` (no suffix) or
`WO_SHIP_COLL_SPARK_TRAIL`/`_SMOKE`. Whether those three load for a different
ship, load some other way, or are unused in this build is not established.

## Many resolved targets are readable developer strings

Resolving every real slot across the whole corpus and checking whether the
target opens with a run of printable ASCII followed by a NUL: **187 of 436
(43%)** do. Two kinds turn up:

- **Texture paths on the original build machine**, e.g.
  `Z:\WipeoutPSP\X2\Data\Psys\Tex\orange_glow2.tga`,
  `Z:\Art_Resources\Psys\Tex\pointglow_32x32.tga`,
  `Z:\wipeoutpsp\X2\Data\Weapons\Textures\Cannon_bolt.tga`. Two distinct dev
  tree roots appear (`Z:\WipeoutPSP\X2\Data\...` and `Z:\Art_Resources\...`),
  case varies (`WipeoutPSP` vs `wipeoutpsp`), and the drive letter is
  consistently `Z:` - a mapped network or virtual drive, standard for a
  Windows-based studio pipeline of this era. These are the actual on-disc
  PSP texture filenames for the particle sprites (`Data\Psys\Tex\*.tga`, an
  archive location not previously known to exist), left in unstripped from
  the exporter rather than resolved to an in-game asset reference.
- **Short authored layer/emitter names** - `GLOW`, `debris`, `thin_streaks`,
  `RINGS`, `ring`, `BANG`, `spikes`, `glow`, `drift_down`, `booga` - reading
  as the emitter or layer names an artist gave each channel in the authoring
  tool.

A string-bearing target is immediately followed by a short run of numeric
data once the string's NUL and padding end (see `WO_SHIP_COLL_SPARK_DAMAGE`'s
resolved target at `+0xce0`: a 12-byte string block, then `0.98, 0.98, 0.98`
- close to white, plausibly a tint), consistent with each resolved record
being `{ name-or-texture-path, small parameter block }`. The non-string
targets are pure floats of the same general shape the previous pass found by
naive scanning (a run ending `1.0, 1.0, 1.0, X` where `X` is either
`10000000.0` or a small real number) - now understood to be *some* of these
same resolved records, not a separately-discovered structure.

**One earlier claim is retracted, not just superseded.** The previous pass
measured a ~224-byte stride between consecutive `1.0, 1.0, 1.0, X` hits and
read it as a probable record size. That measurement scanned raw payload
bytes and differenced *hit addresses*, with no notion of where a record
actually starts - it had nothing to do with the real record boundaries,
which only the fixup table gives. Differencing `WO_SHIP_COLL_SPARK_DAMAGE`'s
18 actual unique resolved targets (sorted: `0xc90, 0xce0, 0xd20, 0x19b0,
0x19e0, 0x1a20, 0x22f0, 0x2320, 0x2fb0, 0x2fe0, 0x3c70, 0x3ca0, 0x3ce0,
0x45b0, 0x45e0, 0x49e0, 0x4f60, 0x5360`) gives gaps of `80, 64, 3216, 48, 64,
2256, 48, 3216, 48, 3216, 48, 64, 2256, 48, 1024, 1408, 1024` bytes - no 224
anywhere, and no other constant either. There is no fixed record size; a
record's length varies with how much parameter data follows its
string-or-name prefix. This document's own history read the values as "not
per-file byte offsets" in one revision and a "~224-byte record" in another -
both wrong, both because the offset table's real two-hop semantics were not
yet understood at the time.

**Not yet established**: the exact byte layout of either record kind (where
the string/parameter split falls per record, what the trailing floats mean),
or what a slot pointing at a *shared* target (the corpus's most common case -
`WO_SHIP_COLL_SPARK_DAMAGE` alone has two targets each referenced by 5 of its
26 slots) signifies beyond "these channels fall back to the same default."
Confidence **70** for the string/parameter-record shape itself (corroborated
by a live trace on the addresses, though the field boundaries within a
record are read, not decoded against a schema); **0** for a full field
layout of either record kind.

## Where the parser is in the executable

The loader chain is now fully decompiled and confirmed live: `FUN_089156a0`
builds `Data\Psys\%s.POB` and calls the generic `FUN_088f3540`, which loads
the raw bytes via `Resource_LoadFile`, runs the pointer fixup above
(`FUN_088f8e38`), registers the cache entry, and returns `resource_base` as
the resource handle (cached at the caller's `+0xb4`). None of this
*interprets* a resolved record - it only makes every slot's target a valid
pointer. Whatever reads a resolved texture-path string and actually creates
a GPU texture reference from it, or reads a resolved numeric record and
turns it into emission/colour/lifetime parameters, is a **different,
not-yet-located** function - the same gap the previous pass described, just
narrowed: it operates on already-fixed-up pointers rather than on the raw
container. `FUN_08a6bd18`, the class-descriptor slot `Vex_RegisterClass`
installs for `ParticleSystem` class `0x3c4`, remains a trivial
self-address-returning thunk - the same dead end already seen twice for
`Ship Collision Fx`'s own slot. The preload-array lead above is the more
promising next static step; short of that, a live capture that steps
forward from a resolved `WO_SHIP_COLL_SPARK_DAMAGE` target rather than from
the loader (which this pass did not attempt) is the next live one.

### An interpreter is now located, but its field semantics are not

Following the collision-spark spawn path documented in
[contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md#shipcollisionfx_trigger-0x089246b4-is-the-actual-spark-spawn-function)
(`ShipCollisionFx_Trigger` -> `FUN_08915484` -> `FUN_08916200` ->
`FUN_088f3174` -> `FUN_088f58a4`) does reach code that reads a resolved SYSP
resource's fields at **fixed offsets from `resource_base`**: `+0x944`,
`+0x948` and `+0x94c` are read as a tree of child/sibling pointers (matching
the same three offsets read once before, in a dead end from an earlier
pass), and `+0x9a0` is unpacked as two `u16` halves of a `u32`, with both
halves' reciprocals computed as floats immediately after - plausibly a
sprite-atlas grid dimension pair, but that was at the time a single-site
inference with no corpus check behind it. **That investigation has since
been run (2026-08-01)** - the sprite-atlas reading was right (`+0x9a0`
grid, `+0x9ac` frame count), `+0x944`/`+0x948`/`+0x94c` really are a
child/sibling tree, and the whole interpreter downstream is traced and
documented in
[particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md);
the emitter-record layout it recovered is the next section.

### The emitter record layout is decoded

**2026-08-01.** The "second, fixed-offset field block" below stopped being an
isolated curiosity: tracing its consumers through the runtime interpreter
([particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md))
decoded the whole emitter-level record. All offsets are relative to
`resource_base` (or to a nested sibling record's own base - see the next
section); values in parentheses are `WO_SHIP_COLL_SPARK_DAMAGE`'s root
emitter, read from file bytes and matched byte-exact against the loaded
resource in a live PPSSPP session:

| offset | type | meaning |
| --- | --- | --- |
| `+0x00` | char[] | NUL-terminated emitter name |
| `+0x20` | u32 | flags (`0x0400001e`): `0x2` emit in world space, `0x4` random initial roll, `0x8` random rotation sign, `0x20` sub-frame spread along emitter motion, `0x200` **gravity enable**, `0x800` immortal particles, `0x800000` repeat-count mode, `0x4000000` random atlas frame |
| `+0x24` | f32 | emitter duration, ticks (32.0) |
| `+0x28`,`+0x2c` | f32 | unknown pair (-0.0057292 both) |
| `+0x30` | u32 | emitter shape: 0 point, 3 cone, 4 sphere, 6 box, 7 hemisphere (4) |
| `+0x34`,`+0x38`,`+0x40` | f32 | emitter extent per axis, world units, severity-scaled (0.0144, -0.0003575, 0) |
| `+0x3c` | u32 | radius shaping: 0 exact, 1 spread, 2 `* sin(U(0,π/2))` (2) |
| `+0x44` | u32 | velocity mode: 0/2 cone, 1 aimed, 2 tangent on spheres (1) |
| `+0x48`,`+0x4c` | f32 | ejection speed centre, spread - units/**tick**, severity-scaled (0.048, 0) |
| `+0x50`,`+0x54` | f32 | aim **elevation** over the emitter's horizontal plane, and **azimuth offset** added to the spawn direction's heading - radians (0, 0). An earlier revision guessed "yaw, pitch"; the convention is pinned by `ParticleSystem_AimedVelocity`'s instruction-level read and live-measured distributions, see [particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md), "The emit frame and the velocity dispatch, settled". **Inert on sphere/hemisphere-shaped emitters**, whose velocity is radial for modes 0 and 1 alike. |
| `+0x58` | f32 | cone half-angle, **degrees** (0) |
| `+0x5c`,`+0x60` | i32 | particle lifetime centre, spread - integer **ticks** (16, 0) |
| `+0x64`,`+0x68` | i32 | emission interval min, max - ticks (4, 4) |
| `+0x6c`,`+0x70` | i32 | particles per emission min, max (1, 1) |
| `+0x74` | f32 | gravity, units/tick², dormant unless flag `0x200` (-0.011232) |
| `+0xa0` | i32 | live-particle cap (8) |
| `+0xb8` | u32 | render-mode index into the blend table at `DAT_08ab2260`; the entry's top nibble is the draw class - 3 rotating billboard, 6/7 two-point streak (2 → class 3 here) |
| `+0xbc` | u32 | colour mode: 2 = random table entry per particle; else over-life walk (2) |
| `+0xc0` | u32 | blend class, dispatched by `ParticleSystem_ApplyBlendClass`: 2 additive, 3 alpha-over (3) |
| `+0xc4` | u32[256] | RGBA colour table - a gradient, orange `(181,134,87,200)` → ember `(48,46,46,0)` here |
| `+0x4cc` | f32 | playback-rate base (1.0) |
| `+0x4d0` | f32 | child velocity-inherit scale (1.0) |
| `+0x4d4` | f32 | child spawn probability (0.1) |
| `+0x4d8`,`+0x5b8`,`+0x698`,`+0x858` | block | channel blocks: size, alpha, rotation speed, emission scale - each `{period f32, mode u32 (0 keyframed / 2 constant / 3 random), key count i32, lo f32, hi f32, (time,value) f32 pairs}` |
| `+0x93c`,`+0x940` | i32, ptr | animated-attribute array (count, records of 0xec bytes) - re-derives the severity-scaled params per tick when present (0, null) |
| `+0x944`,`+0x948`,`+0x94c` | ptr | on-death child system, per-particle child system, **next sibling emitter** (0, 0, `base+0xd20`) |
| `+0x9a0` | u16,u16 | sprite-atlas grid (1, 1) |
| `+0x9ac` | i32 | atlas frame count (1) |
| `+0x9b4` | ptr | modifier list; node = `{f32 params, type u32 at +0x24, next ptr at +0x30}`, type 3 = per-axis exponential drag per tick (→ `(0.98, 0.98, 0.98)`) |
| `+0x9f0`, `+0xa70`, `+0xc70` | scratch | zero in the file; the loader bakes the channel blocks into them (keyframe times, per-segment rates, spawn values) - confirmed by a live read of the loaded resource |

Two units fall straight out of the interpreter and matter for any port:
speeds are world units per **tick** (the integrator multiplies by a
tick-count `dt`), and lifetimes are integer ticks - `sparks.rs` converts
both through one `TICK_HZ` constant. Severity (the instance's `+0x34`,
`intensity * 2.0 + 0.4` for collision sparks) scales ejection speed,
emitter extent **and drawn particle size**; particle *counts* are fixed in
the file.

Confidence **85** for the table as a whole: every row is grounded in a full
decompile of its consumer, and the parenthesised values were confirmed
byte-exact live; single-file corpus, and the rows marked unknown are
unknown. See [particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md)
for the per-function evidence.

### The collision-spark file is a four-emitter tree

`+0x94c` chains **nested sibling emitter records inside the same file**,
each a full record of the same layout with its own name, schedule, palette
and modifiers. `WO_SHIP_COLL_SPARK_DAMAGE.POB` carries four - and a live
breakpoint run on the emit functions during a real wall crash confirmed
all four emitting, with the *nested* records' addresses (root `+0x0`,
`+0xd20`, `+0x2320`, `+0x2fe0`), not the standalone files of the same
names:

| emitter | role | schedule | lifetime | speed (u/tick) | shape | drag/tick | size | colour | draw |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `WO_SHIP_COLL_SPARK_DAMAGE` | smoke puffs | 1 every 4 ticks for 32 | 16 | 0.048 | sphere | 0.98 | grows 0.5→2.5 | orange→ember gradient, α 200 hold 21.5% | rotating billboard (mode 3), **alpha-over**, `quakesmoke32x32.tga` |
| `WO_SHIP_COLL_SPARK` | bright sparks | 3 per tick for 5 | 6±3 | 1.56±0.936 | hemisphere | 0.85 | random 0–0.312 | yellow→orange gradient, α 255 hold 45.9% | streak **from spawn point** (mode 6 + flag `0x2000000`), additive, `orange_glow2.tga` |
| `bits` | white debris | 2 per tick for 4 | 16±6 | 0.295±0.142 | cone 30° | none | shrinks 0.6→0.004 | white, constant α | per-tick motion streak (mode 6), additive, `orange_glow2.tga` |
| `WO_SHIP_COLL_SPARK_TRAIL` | lingering embers | 1 per tick for 32 | 20±10 | 0±0.3 | cone 21.8° | 0.95 | random 0.05–0.2 | yellow→orange gradient, α 255 hold 45.9% | per-tick streak (mode 7), additive, `orange_glow2.tga` |

The draw column comes from the second-pass trace of the draw layer
(`ParticleSystem_DrawParticle` and its helpers,
[particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md#the-draw-layer-added-2026-08-01-second-pass)):
the blend split is what makes the smoke a dark translucent puff (alpha-over)
while the bright emitters add light, and the spawn-anchored streak class is
what makes the sparks read as rays radiating from the impact. Each emitter
names its own texture through its slot region - the smoke's soft puff and
one shared glow for the bright three.

Severity was confirmed live to propagate to all four (identical `+0x34`
across siblings of one burst, `0.527`-`2.03` observed). All four leave the
gravity flag clear. The root's slot table also resolves the emitter's
texture: slot site `0x4c4` →
`Z:\WipeoutPSP\X2\Data\Psys\Tex\quakesmoke32x32.tga`, a soft 32x32 puff.
The per-emitter parameter values themselves are shipped tuning data; the
table above and `oag_render::sparks::EMITTERS` transcribe the recovered
*behavioural* constants the same way every other recovered constant in this
project is recorded, but the 256-entry colour tables stay out per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) - the
committed port samples their measured endpoints, and loading the real
tables from the user's own disc at runtime is the recorded follow-up.

Confidence **90** for the sibling-tree mechanism and the four emitters'
schedules (file bytes plus two independent live captures agree); **85** for
the per-emitter parameter semantics (they inherit the table above).

### The parser walks the tree - 2026-08-12

The table above was, until now, transcribed by hand into
`oag_render::sparks::EMITTERS` for one file. It is now **parsed**:
[`ParticleSystem::emitters`](../../crates/formats/src/pob.rs) reads an
`Emitter` per record and follows `+0x944`/`+0x948`/`+0x94c` into a tree,
returning it depth-first with the root first and the two child fields as
indices into the same vector. Nothing is converted on the way out - speeds
stay units per tick, lifetimes stay integer ticks - so a consumer converts
once, where its own units are.

**The tree is not a sibling chain.** `WO_SHIP_COLL_SPARK_DAMAGE` made it
look like one because all four of its emitters are peers. `WO_ROCKET_EXPLO`
is the counter-example that fixes the container's shape: seven records,
nested both ways.

```text
WO_ROCKET_EXPLO
├─ (per particle) SMOKEMUSHROOM        +0x948, 78±2 tick smoke, its own drag node
├─ DEBRIS                              +0x94c
├─ SMOKERING
├─ GLOW
└─ FIREMUSHROOM_PARENT_GLOWS
   └─ (per particle) FIREMUSHROOM      +0x948
```

`WO_ROCKET_FLARE` is two (`WO_ROCKET_FLARE` + `WO_ROCKET_SHAZZAM`),
`WO_ROCKET_EXPLO_TRACK` five.

**Corpus check, 2026-08-12, both discs, one parser, no adjustments** -
`crates/assets/tests/pob_ground_truth.rs`, run with `just test-data`:

| | systems | emitters | largest tree |
| --- | --- | --- | --- |
| PSP `Data.wad` | 35 | 76 | `WO_ROCKET_EXPLO`, 7 |
| PS2 `WADS2.WAD` | 41 | 90 | `WO_ROCKET_EXPLO`, 7 |

Every one of those 166 records passes the same invariants: the root's name
is the resource's own; the render-mode index lands inside the eight-entry
blend table; the blend class is 1, 2 or 3; the shape is inside the emit
dispatch's switch; all four channel blocks carry a known mode and keyframe
times that are ascending and inside `0..=1`; the emission interval and
per-emission count are at least 1 with `max >= min`; lifetime, live cap,
speed spread, cone angle (`0..=180` degrees), atlas grid and child
probability (`0..=1`) are all in range; and no tree revisits a record.

That is what raises this from "the layout of one file" to a layout: a wrong
offset would have to be simultaneously plausible as a schedule, an angle and
a probability across 166 independently authored records on two platforms.
**Confidence 90** for the record layout as parsed (up from 85), with the
individual rows still carrying the confidence the table above gives them.

Two corrections the corpus forced on the table:

- **`+0x38` is not an extent axis.** It reads `-0.0003575` on the
  collision-spark root, which looked like a small second radius, but
  `-15910579.0` and `8.8e23` on records whose every other field is sane. The
  parser keeps `+0x34` as the extent and exposes `+0x38`/`+0x40` raw and
  unread; `+0x40` is still a plausible second axis (`0.737` on
  `SMOKEMUSHROOM`, `2.94` on a `SMOKERING`).
- **Channel mode 3 ignores its keyframes.** `WO_SHIP_COLL_SPARK_TRAIL`'s
  size block authors six keys *and* mode 3, and
  `ParticleSystem_InitParticle` draws `Psys_RandFloatRange(lo, hi)` without
  consulting them - authored data the interpreter never reads, not a
  reading error. The block's `period` field is likewise authored (`4.0` on
  `WO_SHIP_COLL_SPARK`) with no traced consumer.

### A second, fixed-offset field block sits right after the name - no fixup needed

`FUN_088f4910` (the function `ShipCollisionFx_Trigger`'s spawned particle
instance reaches to derive its emission parameters, see
[contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md#fun_088f4910-derives-six-instance-fields-from-a-plain-scalar-block))
reads `instance+0x20` as a pointer (`iVar1`) and then reads six plain floats
off it at `+0x34`, `+0x38`, `+0x40`, `+0x48`, `+0x4c` and `+0x74`. This
looked, before this pass, like it might be yet another indirection through
the slot/fixup mechanism above. It is not: `iVar1` **is `resource_base`
itself** - the exact same runtime address the slot table's own fixup
arithmetic is relative to, holding the resource's NUL-terminated name at
offset `+0x00`. So these six fields are plain, static, authored data sitting
in the file immediately after the name (and after whatever padding/header
fields occupy the space up to `+0x34`), never touched by the pointer-fixup
pass at all - readable straight from disk with no runtime needed.

**Confirmed two ways, in agreement:**

- **Live.** A PPSSPP capture broke inside `ShipCollisionFx_Trigger` on a
  real wall hit, read `instance+0x20`, and dumped the first `0x20` bytes at
  that address: `"WO_SHIP_COLL_SPARK_DAMAGE\0"` - the resource's own name,
  proving `iVar1 == resource_base`.
- **Static.** Extracting the same resource
  (`WO_SHIP_COLL_SPARK_DAMAGE.POB`, PSP `Data.wad` entry `0xeff1f331`, see
  "Reproducing" above) and computing `resource_base = HEADER_LEN +
  slots.len() * SLOT_LEN` with [`ParticleSystem::parse`](../../crates/formats/src/pob.rs)
  lands on bytes that spell out the same name, and all six fields at
  `resource_base + {0x34, 0x38, 0x40, 0x48, 0x4c, 0x74}` read back **exactly**
  the values the live capture saw: `0.0144`, `-0.00035750002`, `0.0`,
  `0.048`, `0.0`, `-0.011232`.

The reconstruction goes further than the raw offsets agreeing: feeding those
six file values through `FUN_088f4910`'s own arithmetic (see
contact-response.md for the full derivation) reproduces all six of the
*derived* output fields the live capture separately recorded on the spawned
instance, to the same precision, with the two undetermined co-factor inputs
(`instance+0x44`/`instance+0x48`) both resolving to the neutral default
`1.0`. That is an end-to-end, byte-exact chain from file bytes through to
the values driving `ShipCollisionFx_Trigger`'s spawn - not just a pointer
match.

**Superseded the next day (2026-08-01): the units are now known.** The
three-float group is the emitter extent per axis (world units), `+0x48` is
the ejection-speed centre in world units per tick, and the full decode is
the emitter-record table above - see "The emitter record layout is
decoded". The caution this paragraph recorded did its job: nothing was
ported until the consumers were read, and the port
(`oag_render::sparks::EMITTERS`) now carries recovered units rather than
guessed ones.

This is a genuinely separate structure from the slot table and from the
`+0x944`/`+0x948`/`+0x94c`/`+0x9a0` tree-and-atlas fields the previous
subsection describes - both live inside the same resolved resource, at
different fixed offsets, serving different purposes (one is a small
authored parameter block, the other is a node/atlas structure walked by
`FUN_088f58a4`). Confirming whether the fields between `+0x00` (the name)
and `+0x34` (the first of these six) hold anything meaningful, and what lies
between `+0x4c` and `+0x74`, is unstarted.

## Cross-platform: the PS2 port uses the identical format

`SCES_547.48/WADS2.WAD` (the PS2 disc's main archive, same
[WAD container](wad.md) as PSP) carries **41** `SYSP` blobs. Scanning it
in-memory with `oag_assets::Archive` (no extraction needed - decompressing
each candidate entry through the crate's own LZSS path and checking the
first four bytes) and running the unmodified `oag_formats::pob` parser
against every one, with no code changes:

- **41 of 41 parse.** Same magic, same header shape, same two constant
  words.
- **266 of 266 real slots resolve** through the identical fixup arithmetic.
- **Every recovered name's hash agrees** with its own WAD entry hash, the
  same `Data\Psys\<name>.POB` / [`wad::hash_name`](wad.md#the-name-hash)
  check as the PSP corpus - zero mismatches.
- **32 of the 41 names are byte-identical to PSP hashes** (same CRC-32, so
  the same path string on both discs), confirming one shared asset pipeline
  rather than two independently-authored sets. The other 9 are PS2-only in
  this archive: `WO_CANNON_HIT_SHIP`, `WO_DAMAGE_PLUME`,
  `WO_MODESTO_STEAM_B`, `WO_SHIP_ENGINEFLARE`, `WO_SHIP_EXPLOSION_DEBRIS`,
  `WO_SHIP_EXPLO_SMOKE`, `WO_SHIP_FXNODE_BIGEXPLO`, `WO_TRACK_ROCK_DEBRIS`,
  `WO_UNDERWATER_DEBRIS`. Conversely `WO_SHIP_COLL_SPARK`,
  `WO_SHIP_COLL_SPARK_TRAIL` and `WO_SHIP_COLL_SPARK_TRAIL_SMOKE` (three of
  PSP's 35) are absent from `WADS2.WAD` - not checked against the smaller
  `WADSP.WAD` or `PRERACE.WAD` yet, so "absent from this archive" rather
  than "absent from the disc" is the honest claim.

Per the rubric, **"a second binary is worth more than a second reading of
the first."** This is a second, independent binary agreeing on every
structural claim without a single adjustment to the parser - real
corroboration, not just a bigger PSP sample.

## Evidence summary

Every structural claim below holds on **35 of 35** PSP files and, per the
section above, **41 of 41** PS2 files:

- `+0x04` equals `16 + payload.len()` exactly.
- The name field, read immediately after the table with no fixed offset,
  round-trips through `Data\Psys\<name>.POB` and [`wad::hash_name`](wad.md#the-name-hash)
  to the blob's own WAD entry hash.
- Slot count from `+0x08` matches the boundary the name-field check needs to
  land on.
- Every real slot resolves through the pointer-fixup arithmetic to an
  in-bounds target (436 of 436 PSP slots, 266 of 266 PS2 slots).

Confidence: **93** for the container and the name field - an exact size
invariant plus an exact hash agreement, both holding on **two independent
binaries** with an unmodified parser. **90** for the pointer-fixup
mechanism specifically: a byte-exact runtime trace on PSP (26 of 26 real
sites, pre- and post-fixup values both read live and compared) is the
strong leg; the PS2 check is corpus-wide bounds-consistency (all 266 real
slots resolve in-bounds) rather than a second runtime trace, so it
corroborates the *shape* of the mechanism without confirming PS2's own
loader performs the identical `+= resource_base` add - that would need
either decompiling the PS2 executable or tracing it live, neither done this
pass. **70** for the string/parameter-record shape a resolved target holds -
corroborated by the live trace's own reads, but the field layout inside a
record is not decoded. **0** for a full field layout of either record kind.
**30** for a fixed slot-table-*index* per attribute-mapper class - a
plausible reading of the recovered layer names (`GLOW`, `debris`, ...),
unverified. **90** for the fixed-offset scalar block at `resource_base +
{0x34, 0x38, 0x40, 0x48, 0x4c, 0x74}` existing and being un-fixed-up authored
data - a live trace and a static file read agree byte-exact on all six
values, and the full consumer arithmetic (`FUN_088f4910`) reproduces the
live-captured *derived* outputs from those same six values. **85** for what
those values and the rest of the emitter record mean - the 2026-08-01
interpreter trace ("The emitter record layout is decoded" above,
[particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md)),
which retired the previous **0** here; on the collision-spark file only,
not corpus-wide. **90** for the `+0x94c` sibling-emitter tree (file bytes
plus two live breakpoint captures).

**Validated against two discs**, `pulse-psp-usa` and `pulse-ps2-eu`. Not
checked against Pure's UMD, or PS2's `WADSP.WAD`/`PRERACE.WAD`.

## Reproducing

```sh
# WO_SHIP_COLL_SPARK_DAMAGE.POB is PSP Data.wad entry 0533, hash 0xeff1f331
just wad cat 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' 0xeff1f331 > /tmp/spark_damage.pob

# the PS2 corroboration read every candidate straight out of the disc image
# with oag_assets::Archive, entirely in memory - no bulk `wad extract` of
# WADS2.WAD (377 MB compressed, ~650 MB unpacked) needed or advised on a
# space-constrained /tmp
```

`oag_formats::pob::ParticleSystem::parse`, `resolve_slot` and `emitters`
recover the name, slot table, every fixup target and the whole emitter tree
from any of the 35 PSP or 41 PS2 blobs. Per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) only
hand-authored fixtures - never extracted game bytes - may land in the repo,
so the unit tests in `pob.rs` build synthetic blobs replaying the confirmed
fixup and record shapes, and the corpus check runs against the user's own
disc, `#[ignore]`d and out of CI:

```sh
cargo nextest run -p oag-assets --run-ignored all --test pob_ground_truth --no-capture
```

## Not determined

- **The field layout inside a *slot-resolved* record.** A string-bearing
  record is `{ NUL-terminated string, small parameter block }`; a
  non-string record is a run of floats. The *emitter-level* layout that
  used to head this list is decoded ("The emitter record layout is
  decoded" above) and ported to `oag_render::sparks::EMITTERS` with
  confirmed units - what remains undecoded is the slot-table targets'
  own internals (beyond "site `0x4c4` resolves the texture path") and the
  scratch regions the loader bakes into (`+0x9c8`/`+0x9cc` targets).
- **What a slot's *position* in the table means**, if anything - the
  attribute-mapper-class-per-index hypothesis is unverified.
- **What a shared target (several slots resolving to the identical offset)
  signifies** beyond "these channels fall back to the same default."
- **The two constant header words at `+0x0a` and `+0x0c`.** Always 1 across
  both the PSP and PS2 corpora; whether that is a version, a type tag, or
  something else is unknown. [`ParticleSystem::parse`](../../crates/formats/src/pob.rs)
  refuses any other value rather than silently accepting an unrecognised
  header, the same choice `sblk.rs` makes for its own version field.
- **Where the interpreter that reads a resolved record lives in the
  executable.** Resolved on 2026-08-01: the whole runtime chain is traced
  and named in
  [particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md).
  Still unread there: the shape-3 cone placement (`FUN_088fc634`), shapes
  1/2/8, modifier types other than 3, and the billboard draw itself.
- **Whether `WO_SHIP_COLL_SPARK`, `_TRAIL` and `_TRAIL_SMOKE` exist on PS2**
  outside `WADS2.WAD`.
