# Particle system

**Status: partial, and the two halves are at very different depths.** The
container, the slot table (including the pointer fixup it drives at load
time, confirmed live), the name field, and **the emitter tree** are all
decoded, implemented in
[`oag-pob`](../../crates/pob/src/lib.rs) and validated against
all 35 `.pob` files on the PSP disc, all 41 on the PS2 disc and all 88 across
HD's seven PSARC archives. What is
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
involvement - see [`oag_fx::exhaust`](../rendering/README.md).

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
particle systems for the effect `oag_fx::sparks` draws (see
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

[`ParticleSystem::resolve_slot`](../../crates/pob/src/lib.rs) replays
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
  the generic resource loader `ParticleManager_LoadResource` the first time any container of
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
preload entries baked into the executable.

**The xref lead this page used to call "untried" was tried and was a clean
static dead end - every `ghidra-mcp` xref mechanism, plus a whole-program
`lui`/`_lui` sweep for the address's absolute high half, came back empty
against all four path-string addresses (2026-08-25 first pass).** That
dead end was real but not the final answer: absence of a `lui`/`addiu` pair
is exactly what a PIC-style, per-function base-pointer addressing mode
produces, and this binary uses one (`FUN_089156a0` itself has
`lui s3,0x2b` / `lw a0,-0x1db8(s3)` in its own prologue). A sibling `$gp`
trap was checked and ruled out (no register named `gp`, no matching
displacement, anywhere in this program's disassembly) - it's a different
mechanism, not that one.

**The caller is found (2026-08-25, live, PPSSPP v1.20.4 under Xvfb) and
named `ShipCollisionFx_Preload` (`0x08924550`-`0x089246ab`).** It decompiles
to four straight-line calls into `func_0x000ef540` (rebased:
`0x08804000 + 0x000ef540 = 0x088f3540`, i.e. `ParticleManager_LoadResource`, the generic
resource loader this page already names) each with the pool-pointer argument
`_DAT_002ae248` and a **decompiler-printed literal that reads as
unrelocated**: `0x2846f8`, `0x284720`, `0x284750`, `0x28477c`. This binary
is known to print pre-relocation literals in some regions of its decompiled
output rather than the runtime address (a "looks like `0x276c70`, is
actually `0x08a7ac70`" pattern documented elsewhere on a different string in
the same import) - add the `0x08804000` image base back and all four
resolve **exactly** to the four path-string addresses above, in the same
order (`0x2846f8 + 0x08804000 = 0x08a886f8`, and so on through
`0x28477c → 0x08a8877c`).

Confirmed live, not just by decompiling: a breakpoint at
`ShipCollisionFx_Preload`'s own entry never fired (ruling it out as its own
caller, and closing that specific guess), but a breakpoint at
`ParticleManager_LoadResource`'s entry fired eight times during one "loading the race"
step, with `a0` (the pool pointer) identical across all eight hits and `ra`
(the return address, i.e. the actual call site) cycling through exactly
four values sixteen bytes apart - `0x08924580, 0x08924590, 0x089245a0,
0x089245b0` - twice each. Those four addresses land inside
`ShipCollisionFx_Preload`, right after its four `jal`s, matching the
decompile exactly. The repeat (eight hits, not four) is consistent with
`ShipCollisionFx_Preload` itself being invoked twice for two grid entities
and `ParticleManager_LoadResource`'s own load-by-name cache (see "registers the cache
entry" above) absorbing the second round without new disk I/O - only the
mechanism this page already documented, encountered a second time.

**`ShipCollisionFx_Preload`'s own caller is found too, and it's generic
vtable dispatch, not a name-driven loader at all.** A breakpoint at
`ShipCollisionFx_Preload`'s own entry fired twice with a single `ra`
(`0x08909038`, inside `FUN_08908f98`) and two `a0` values `0x80` bytes
apart - one call per grid entity. `FUN_08908f98` decompiles to a bare `for`
loop over `*(param_1+0x48)+0xc` entities, each one dispatched through two
vtable calls read off `+0x38` on the entity itself
(`(**(code**)(*(int*)(iVar1+0x38)+0x74))(...)` then `+0x7c`). Nothing about
this loop is `.pob`-specific or ship-collision-specific.
`ShipCollisionFx_Preload` sits **immediately before** the already-named
`ShipCollisionFx_Trigger` (`0x089246b4`, confidence 85, from
[contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md#shipcollisionfx_trigger-0x089246b4-is-the-actual-spark-spawn-function))
with no other function between them, and being reached only through a
generic per-entity construction vtable rather than by name makes it read as
that same class's own constructor. `FUN_08908f98` is itself still
callerless by static search - the same xref-blind pattern as everything
else on this page, and not chased further.

**What this settles, and what it doesn't.** It closes "the preload's own
caller is unfound" cleanly, with both a decompile and a live capture
agreeing. It does **not** answer the thread's actual question - what a
*slot-resolved* record's bytes mean - because this whole chain is a
hardcoded four-file preload with no slot resolution in it at all; the lead
was already flagged stale for that reason before this pass confirmed it.
It also demonstrates, for future sessions in this executable: a callerless
function with a decompiled literal that looks suspiciously round (like
`0x2846f8`) is worth rebasing by `0x08804000` before concluding it is
unreachable, and a live breakpoint at the *callee's* entry - reading `ra`
at the hit, not searching for the call site's address - sidesteps this
whole class of PIC-addressing xref blindness entirely.

Confidence **85** for `ShipCollisionFx_Preload`'s name and its four preload
calls - a decompile and a live capture agreeing on the addresses and the
call count/pattern, plus direct adjacency to the same-confidence
`ShipCollisionFx_Trigger`; **60** for reading it specifically as that
class's *construction-time* vtable method rather than some other
per-entity lifecycle call - the vtable-dispatch shape and the one-per-entity
call count are measured, the "construction" framing is inferred.

Separately notable: only 4 of the 5 `WO_SHIP_COLL_SPARK*` files preloaded for
this ship+track combination - not `WO_SHIP_COLL_SPARK` (no suffix) or
`WO_SHIP_COLL_SPARK_TRAIL`/`_SMOKE`. Whether those three load for a different
ship, load some other way, or are unused in this build is not established.

### A second, larger preload-style table exists for weapon effects, previously unrecorded

`0x08a7c3e4`-`0x08a7c6ac` holds 24 more `Data\Psys\WO_*.POB` full path
strings back to back, the same packed-string shape as the four-entry ship
table above (`WO_MISSILE_HEAD`, `WO_MISSILE_BOUNCE`, `WO_MISSILE_EXPLO`,
`WO_ROCKET_FLARE`, `WO_ROCKET_EXPLO`, `WO_CANNON_SPARKS`, `WO_PLASMA_HEAD`,
`WO_PLASMA_FLASH`, `WO_QUAKE`, `WO_REPULSER`, `WO_REPULSER_BLAST`,
`WO_LEACHBEAM_ENERGY`, `WO_LEACHBEAM_CHARGING`, `WO_SHURIKEN_HEAD`,
`WO_SHURIKEN_TRAIL`, `WO_SHURIKEN_BOUNCE`, `WO_SHURIKEN_EXPIRE`,
`WO_ROCKET_EXPLO_TRACK`, `WO_WEAPON_ABSORB`, `WO_BOMB_SMOKERING`,
`WO_SHIP_FXNODE_EXPLO`, `WO_MINE_EXPLO`, `WO_SHIP_EXPLOSION`, plus the format
string `Data\Psys\%s.POB` itself at `0x08a884dc` immediately after). Found
while chasing the xref dead end above via `search_strings`, not previously
recorded in this document. It carries the same negative xref result as the
ship table (`get_xrefs_to` and `search_byte_patterns` on `0x08a7c3e4` both
empty) and every name in it
is already among the ones this document's corpus table names - it adds no
new effect, only a second static enumeration of which ones the executable
addresses by full path, and a second data point that this shape of table is
reached by something other than a plain absolute-address load.

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
builds `Data\Psys\%s.POB` and calls the generic `ParticleManager_LoadResource`, which loads
the raw bytes via `Resource_LoadFile`, runs the pointer fixup above
(`FUN_088f8e38`), registers the cache entry, and returns `resource_base` as
the resource handle (cached at the caller's `+0xb4`). None of this
*interprets* a resolved record - it only makes every slot's target a valid
pointer. Whatever reads a resolved texture-path string and actually creates
a GPU texture reference from it, or reads a resolved numeric record and
turns it into emission/colour/lifetime parameters, is a **different,
not-yet-located** function - the same gap the previous pass described, just
narrowed: it operates on already-fixed-up pointers rather than on the raw
container. ~~`FUN_08a6bd18`, the class-descriptor slot `Vex_RegisterClass`
installs for `ParticleSystem` class `0x3c4`, remains a trivial
self-address-returning thunk - the same dead end already seen twice for
`Ship Collision Fx`'s own slot.~~ **Corrected 2026-10-02:** `0x08a6bd18` is
`PsysNode_Tag`, the class's identity tag, and the class is live - circuits
place `ParticleSystem` nodes in their `track.vex` and `PsysNode_Init`
(`0x089156a0`) spawns each at load, confirmed on PPSSPP. See
[placed-particle-systems.md](../ghidra/functions/psp-pulse-usa/placed-particle-systems.md). The preload-array lead above is the more
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
| `+0x20` | u32 | flags (`0x0400001e`): `0x2` particles kept in the instance's frame and drawn through its live matrix (not world space, despite the old name: `particle-system.md`, "The emitter's clock and the burst laws"), `0x4` random initial roll, `0x8` random rotation sign, `0x20` sub-frame spread, pulling particle `i` back by `i / n` of the frame's motion, `0x200000` an evenly stepped ring (shape 3) or a Fibonacci sphere (shapes 4 and 7), `0x200` **gravity enable**, `0x800` immortal particles, `0x800000` repeat-count mode, `0x4000000` random atlas frame |
| `+0x24` | f32 | emitter duration, ticks (32.0) |
| `+0x28`,`+0x2c` | f32 | unknown pair (-0.0057292 both) |
| `+0x30` | u32 | emitter shape: 0 point, 1 line, 2 rectangle, 3 ring or disc, 4 sphere, 6 box, 7 hemisphere, 8 **half ring** (`ParticleSystem_EmitHalfRing`, shape 3 over `[0, pi]`, the frame's `+Z` half; `WO_REPULSER` only; measured 2026-10-04, see [particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md), "Shape 8, the class-6 bar and the wave's width") |
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
| `+0x4c8` | f32 | **aspect**: a class 3 particle's half-width over its half-height (`ParticleSystem_DrawRolledQuads`, `0x089178c0`). `1.0` on 43 of the 45 class 3 emitters, `4.0` on the Shuriken's head and trail; `2`, `3` and `0.05` on class 6 and 7 streaks. On class 6 it is the bar's cap: `ParticleSystem_DrawPoolBars` (`0x08917c7c`) reaches `aspect * size` past each end of a `2 size`-wide rectangle (confidence 90, measured 2026-10-04). Class 7's use is unread. Played since 2026-09-30 (confidence 80, static read). See [particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md), "An emitter's own particles" |
| `+0x4cc` | f32 | **playback rate** (1.0): the emitter's own ticks per frame tick, times instance `+0x3c` (`1.0`), into `+0x70`; every clock reads it (countdowns, drag, integration, age, life, roll, frame). `4` on `WO_REPULSER_BLAST`, `2`/`1.5`/`0.8` on the LeachBeam charge, `2` on the Missile explosion's root, `0.8` on the Shuriken bounce and expiry and the absorb. Played since 2026-10-04 (confidence 90) |
| `+0x4d0` | f32 | child velocity-inherit scale (1.0) |
| `+0x4d4` | f32 | child spawn probability (0.1) |
| `+0x4d8`,`+0x5b8`,`+0x698`,`+0x778`,`+0x858` | block | channel blocks: size, alpha, rotation speed (**always a rate**, radians per tick, on an emitter - played since 2026-09-30: flag `0x4` starts the angle at `U(-pi, pi)`, flag `0x8` flips a coin per particle, a random channel draws its rate once at spawn, a keyframed one turns the opposite way round from the others; class 3 only), atlas frame rate (added 2026-09-24, see below), emission scale - each `{period f32, mode u32 (0 keyframed / 2 constant / 3 random), key count i32, lo f32, hi f32, (time,value) f32 pairs}` |
| `+0x93c`,`+0x940` | i32, ptr | animated-attribute array (count, records of 0xec bytes: a channel block at `+0x00` and a selector at `+0xe0`, `1..6` to instance `+0x44, +0x48, +0x50, +0x4c, +0x54, +0x34`) - re-derives the severity-scaled params every tick when present (0, null). Parsed since 2026-10-01 as `Emitter::attribute_animations`; selector `2` is the extent's co-factor and widens `WO_BOMB_SMOKERING`'s spawn ring from 12.9 to 23.3 over its run, selector `5` the newborn's lifetime co-factor (`WO_REPULSER_BLAST`'s `1.5`), both played (`particle-system.md`) |
| `+0x944`,`+0x948`,`+0x94c` | ptr | on-death child system, per-particle child system, **next sibling emitter** (0, 0, `base+0xd20`) |
| `+0x9a0` | u16,u16 | sprite-atlas grid (1, 1) |
| `+0x9ac` | i32 | **not** the atlas frame count (corrected 2026-09-24): `ParticleSystem_InitParticle` ORs `Psys_RandIntRange(1, n) << 4` into a per-particle flag byte when it is above 1; its reader is untraced. The frame count is the `+0x9a0` grid's product |
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

### The authored emitter-node frame lives outside the `.pob`, in the hull's own locator

`+0x50`/`+0x54`/`+0x58` above are angles *relative to a frame*, and that
frame is not itself a field of this container - there is no rotation
anywhere in an emitter record. [particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md)'s
"The emit frame and the velocity dispatch, settled" section already
identified where it comes from instead: `ShipCollisionFx_Trigger` parents
the spawned instance to the nearest `Ship Collision Fx` locator on the hull,
and that locator's own node carries the frame - the same 64-byte,
row-major-with-translation-in-row-3 payload every locator class in a `.vex`
carries (`oag_vex::vex::transform`), with row 1 the up axis. All six nodes
checked on Assegai's `Ship.vex`, and spot-checks elsewhere, author a pure
translation: row 1 reads `(0, 1, 0)`, so the emitter's local `+Y` is world
up **there**.

That "there" is doing real work: a locator's own matrix places it in the
*hull's* model space, not the world's, and the hull itself keeps turning as
a craft flies. `oag_render::livery::SparkAnchor` (`crates/livery/src/lib.rs`)
carries both halves of the same locator matrix an earlier revision of this
engine read only the translation from - `position` (row 3) and `up` (row
1) - and `oag_fx::psys::System::advance` composes `up` with the ship's
*current* attitude every tick, the same way the position was already
recomposed every tick to ride a moving hull. So a spray fired while banking
hard into a wall tilts with the hull rather than assuming the craft is
level; on the measured corpus, where every locator is an identity rotation,
this reduces to "the emitter's `+Y` is the hull's own live up axis," which
is the general statement the previous "world up, unrotated" reading was a
special case of without saying so. Confidence **85**, the same figure the
emitter table above carries and for the same reason: the locator's own
byte layout is read directly and corroborated by `Start Position`'s
identical convention (`docs/formats/track.md`), but the *rotation* case -
a `Ship Collision Fx` locator that authors something other than identity -
has not been observed in the corpus, only prepared for.

**The banking half of this composition is now screenshot-verified, not just
proven algebraically.** `verification/scenarios/steer-left.inputs` (a
constant left lock from the grid, `just play --race --no-audio
--input-script verification/scenarios/steer-left.inputs --screenshot ...
--ticks N`) puts the craft into Talon's Junction's wall at tick 309, banked
2.45° at its peak (tick 320) - measured with a temporary debug print of
`race::tick`'s own `up` variable after normalising it, not read off a
screenshot by eye. Forcing that same `up` to `Vec3::Y` (the pre-fix
arithmetic) and re-rendering the identical tick produces a pixel-identical
frame outside the spark burst and a measurably different one inside it -
`compare -metric AE` finds 300-500 differing pixels at ticks 313/317,
entirely inside the burst's own streaks. So the composition is doing real,
render-affecting work and not just carrying dead weight. At 2.45° the
difference is real but subtle: legible in a same-tick diff, marginal to spot
in either frame on its own. No scenario tried (`steer-left`, `steer-both-ways`,
`airbrake-asymmetric`, the `talons-junction-inverted-section` autopilot replay)
produced a wall contact banked much past 3° - this reduces the honest claim
from "unverified past the algebra" to "verified, and subtle at the bank
angles this project's scenarios reach," not to "dramatically confirmed."

An earlier state of this project aimed the collision-spark cone along the
wall contact normal instead of any of the above. That was retired
2026-08-10 (`1f7f4e67`), before the frame-vs-locator distinction here was
written down, which is why a later work-tracking pass mis-filed "still uses
the contact normal" as an open approximation: the contact normal has not
driven a spawn direction since that commit, and the honest gap left behind
was narrower than that later note's own wording - "world up, unrotated"
instead of "the hull's live up," not "the wall's normal."

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
table above transcribes the recovered
*behavioural* constants the same way every other recovered constant in this
project is recorded, but the 256-entry colour tables stay out of both the
table and the repository per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) - the
"orange→ember" and "yellow→orange" columns above name only the two
endpoints for a human reader. **The committed port reads all 256 of each
emitter's own entries from the user's disc at runtime** (`crates/pob/src/lib.rs`,
`+0xc4` below), the same as every other field in this table; nothing is
committed and nothing is sampled down to two colours any more. This closes
what used to be the recorded follow-up here (until 2026-08-12, the port
really did sample only the measured endpoints, in
`oag_fx::sparks::Colour::Gradient`) - see
`crates/fx/tests/psys_ground_truth.rs`'s
`the_collision_spark_palette_is_read_from_disc_not_two_endpoints` and its
two neighbours for what the endpoints-only version got wrong: the smoke
ramp is not linear (three quarters of its `RandomEntry` population spawn
some shade of grey ember, not the uniform orange-to-grey mix two endpoints
imply) and the bright spark's hue is not monotonic (it brightens toward
yellow partway through its life before dimming to deep orange, which no
two-point gradient can reproduce).

Confidence **90** for the sibling-tree mechanism and the four emitters'
schedules (file bytes plus two independent live captures agree); **85** for
the per-emitter parameter semantics (they inherit the table above).

### The parser walks the tree - 2026-08-12

The table above was, until now, transcribed by hand into
`oag_fx::sparks::EMITTERS` for one file. It is now **parsed**:
[`ParticleSystem::emitters`](../../crates/pob/src/lib.rs) reads an
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
`crates/pob/tests/pob_ground_truth.rs`, run with `just test-data`:

| | systems | emitters | largest tree |
| --- | --- | --- | --- |
| PSP `Data.wad` | 35 | 76 | `WO_ROCKET_EXPLO`, 7 |
| PS2 `WADS2.WAD` | 41 | 90 | `WO_ROCKET_EXPLO`, 7 |
| HD's 7 PSARC archives | 88 | 249 | `WO_SHIP_EXPLOSION`, 12 |

HD's 88 is every `.pob` blob across all seven archives, not deduplicated - the
distinct count is 82. Five names account for the six extra blobs: three are
the same effect shipped in more than one archive (`WO_TRAIL_HITSHIP` in
three, `WO_DAMAGE_MILD` and `WO_DEBRIS_SPARKS` in two), but two are a second
file **inside `DATA02` alone** whose internal name was never changed off the
one it was copied from - `wo_ship_explosion_lightshafts.pob` still names
itself `WO_SHIP_EXPLOSION`, and `stesparkstest.pob` still names itself
`WO_SHIP_COLL_SPARK_DAMAGE`. `crates/game/tests/psys_inventory_ground_truth.rs`'s
`mod hd` checks the distinct count against the disc as its own tripwire,
separately from this corpus check, and prints which names those extra blobs
belong to.

**Two of the 82 are genuinely their own name, not a leftover copy, and
`DATA02`-only**: `numbers.pob` names itself `NUMBERS` and
`test_bombspikes.pob` names itself `TEST_BOMBSPIKES` - read off the name
field at `0x10 + slots * 4` the same way as every other row in this table
(`just psarc cat <image>:DATA02.PSARC /data/psys/numbers.pob | xxd`, or
`ParticleSystem::parse`), 2026-09-08. Neither string - bare name or as a
`Data\Psys\...POB` load path - occurs anywhere in `EBOOT.elf`'s string
table, checked two independent ways: `strings -a` over the whole binary
(both names, case-insensitive) and Ghidra's own defined-string table
(`search_strings`) against the imported program; both agree, zero hits for
both names. **The control that makes this null result mean something**:
this is not merely "unwired effects have no string" - `strings -a … | rg -o
'Data.Psys.[A-Za-z0-9_]+\.POB' | sort -u | wc -l` finds **46 of the 82** disc
names as a load-path string (case-insensitive on purpose - `WO_DustMotes`
below is the one mixed-case name in the corpus and an all-caps pattern would
silently drop it), well past the 17 `RACE_EFFECTS` wires,
including effects this engine has not built at all
(`WO_BOMB_EXPLO_DETONATOR`, every `WO_DAMAGE_*` tier) - so the original
executable's own string table tracks *its* load paths, not OAG's wiring, and
`NUMBERS`/`TEST_BOMBSPIKES` sitting outside that 46 is evidence about the
original game, not an artifact of this project's scope. **Confidence 82**
that both are disc-authored debug/test assets with no shipped trigger: the
36 authored names *also* missing from that load-path scan (some are
sub-effects reached from inside a parent tree rather than by a top-level
load, per this file's `embedded` bucket) mean absence from the 46 alone
isn't proof, and neither scan proves no code path reaches these two by a
route that never puts the name in the string table (a hash, a constructed
string, an index).

**Four more, 2026-09-08, all environment effects - but the string-table
method that worked above turned out blind to this whole category, not just
these four**: `wo_blue_welder.pob` (`WO_BLUE_WELDER`), `wo_modesto_steam_a.pob`
(`WO_MODESTO_STEAM_A`) and `wo_underwater_godrays.pob` (`WO_UNDERWATER_GODRAYS`)
are `DATA02`; `wo_dustmotes.pob` (`WO_DustMotes` - mixed case internally, the
only name in the corpus that isn't all-caps) is `DATA00`, paired with its own
`data/psys/tex/dustmotes_4x4.gtf` texture in the same archive, which is why
this one reads as a real shipped asset rather than a second debug leftover
despite the odd casing. None of the four occurs anywhere in `EBOOT.elf`'s
string table (`strings -a`, Ghidra `search_strings`, both zero) - but so does
*no* environment-flavoured name, including `WO_RAIN`, `WO_SNOW` and the two
above that are already-established `NO_TRIGGER_RECOVERED` entries on the PSP
disc: none of the environment family is in the 46-name load-path control
above, wired or not, on either disc. The engine flare family shows the same
blind spot from the other side - `WO_SHIP_ENGINEFLARE` is wired
(`RACE_EFFECTS`) and *also* absent from HD's string table, its trigger
established from the PS2 disc's own reasoning ("an engine flare is on while
the craft is") rather than from any HD-side string, and its two HD-only
siblings `WO_ENGINE_FLARE`/`WO_ENGINE_JETFLARE` are absent the same way. So a
zero hit for a continuous/ambient effect proves nothing either way; unlike
`NUMBERS`/`TEST_BOMBSPIKES`, this check does not discriminate for these four.

The bucket assignment - environment effect, no recovered placement trigger -
still stands on the same basis PSP's own `WO_RAIN` entry already rests on:
an admission that the track-placement format (`.effectsettings`, `aurora.xml`,
or a `track.vex` node) is not decoded by this project, not a string search.
Two of the four (`WO_BLUE_WELDER`, `WO_MODESTO_STEAM_A`) reuse that PSP
reasoning outright, sharing the exact name; `WO_DustMotes` and
`WO_UNDERWATER_GODRAYS` extend the category to two names new to HD on
naming and asset-pairing grounds, not on independent HD executable evidence.
**Confidence 65** for the bucket assignment being correct (down from the 82
above, which the string check earned and this one did not) - a genuine
placement mechanism for HD's environment effects is still unread, and
`WO_ENGINE_FLARE`/`WO_ENGINE_JETFLARE` are a live open question (see the
handover thread's Next Steps) rather than settled.

Six of the 67 unwired names (82 total minus the 15 `RACE_EFFECTS` wires)
this project owes a bucketing pass over are done - see
`crates/game/tests/psys_inventory_ground_truth.rs`'s `mod hd` doc comment;
the other 61 are unread.

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

### Flag `0x1` is "this effect loops" - 2026-08-12

The flags word at `+0x20` carries a bit that decides whether
`duration_ticks` at `+0x24` is a countdown at all. **Corpus-derived,
confidence 75**, and the split is clean on both discs - 76 emitters over the
PSP's 35 systems and 41 on the PS2, with **no file mixing set and clear
emitters**, which is what makes it a property of the effect rather than of
one emitter.

| `0x1` set | `0x1` clear |
| --- | --- |
| `WO_ROCKET_FLARE`, `WO_MISSILE_HEAD`, `WO_SHURIKEN_HEAD`, `WO_SHURIKEN_TRAIL`, `WO_PLASMA_HEAD`, `WO_LEACHBEAM_CHARGING`, `WO_LEACHBEAM_ENERGY`, `WO_REPULSER`, `WO_QUAKE`, `WO_RAIN`, `WO_RAIN_LENS`, `WO_SNOW`, `WO_BLUE_WELDER`, `WO_MODESTO_STEAM_A`; PS2 adds `WO_SHIP_ENGINEFLARE`, `WO_MODESTO_STEAM_B`, `WO_UNDERWATER_DEBRIS` | every explosion, every bounce, every spark burst, `WO_PLASMA_FLASH`, `WO_REPULSER_BLAST`, `WO_WEAPON_ABSORB`, `WO_BOMB_SMOKERING`; PS2 adds `WO_CANNON_HIT_SHIP`, `WO_TRACK_ROCK_DEBRIS`, `WO_DAMAGE_PLUME` |

Every attached or environmental effect is on the left; every impact is on
the right. **`WO_RAIN`, `WO_SNOW` and `WO_BLUE_WELDER` settle it on their
own**: all three author a *one-tick* duration, and none of the three can be
a one-tick effect. The PS2's `WO_SHIP_ENGINEFLARE` is the same argument from
an independently authored set - an engine flare runs for the whole race.

The executable has the matching mechanism, at `ParticleSystem_Update`
(`0x088f5b9c`). Under **instance** flag `0x10` at `+0x160`:

- the counter at `+0x138` counts *up* by one per update instead of down by
  the frame's ticks,
- the emitter-level channel age becomes `counter / 60` rather than
  `1 - counter / duration`,
- and the branch that sets the finished bit is skipped entirely.

**The countdown runs one tick short (2026-10-01, Pulse PSP).** An emitter emits on its first update and then
only while more than one tick of `duration_ticks` is left, so `duration 10` throws `9` rounds and `duration 1`
one - read on `WO_SHIP_EXPLOSION`'s four emitters, [particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md#the-instance-matrix-scales-a-root-emitters-spawn-and-a-run-emits-one-tick-short-2026-10-01)
(`EmitterSpec::short_run`).

What is **not** traced is the spec-`0x1`-to-instance-`0x10` assignment: the
instance initialiser is behind an unresolved import stub, so the two are
joined by the corpus rather than by a read. That is what holds the
confidence at 75 rather than higher.

Why it matters to a reimplementation: `WO_ROCKET_FLARE` authors 100 ticks
and a rocket flies for up to 600. Reading that as a countdown makes the
flare stop a sixth of the way through the flight, and the obvious repair -
re-triggering it - is an invention covering a misread flag.

#### "Property of the effect" is a Pulse regularity, not a rule - 2026-08-18

Wipeout HD walked with the same parser breaks the clean split, and only that
half of the claim. **Four of its 88 systems mix set and clear emitters inside
one tree**: `WO_QUAKE_DETONATOR_TRAILS`, `WO_CANNON_MUZZLEFLASH`,
`WO_MISSILE_HEAD` and `WO_ROCKET_FLARE` - the last two being on the *set* side
of the Pulse table above, so HD authors the same effects with some emitters
looping and some not.

The bit's meaning is untouched: nothing about a per-emitter flag contradicts
the executable's `+0x160` mechanism, which is per-instance and so per-emitter
already. What is retired is the corpus inference layered on top of it. A
consumer must read each emitter's own flag and must not take the root's as the
effect's. Pinned in
`crates/pob/tests/pob_ground_truth.rs::every_hd_particle_system_walks_the_same_way_byte_swapped`,
which asserts *which* four mix rather than tolerating any number that do.

### The colour table is RGB-only, and `+0xbc` says whether it animates - 2026-08-12

`ParticleSystem_UpdateParticles` (`0x088f635c`) resolves a live particle's
colour in four instructions, and they settle two questions this project had
open:

```c
local_60 = (*(int *)(spec + 0xbc) != 2);
...
if (local_60 != 0) {
    *p = (*p & 0xff000000)                                   // keep alpha
       | (palette[(int)(age * 255.999) & 0xff] & 0xffffff);  // RGB only
}
```

- **The palette entry's alpha byte is masked off** (`& 0xffffff`) and the
  particle's own alpha byte is preserved. The 256-entry table is an
  RGB-over-life ramp whose fourth column the interpreter never reads, even
  though it is authored as a ramp too. A renderer that multiplies the two
  makes every additive particle roughly twice as transparent as it should
  be.
- **`+0xbc == 2` means the colour does not animate**: the per-tick refresh
  is skipped and the particle keeps whatever colour it was given at spawn.
  What the spawn draw *is* remains unread - `func_0x000f16c4` is another
  import stub - so "constant at whatever it drew" is as far as this goes.

Alpha comes from the alpha channel scaled by an instance field:
`alpha = clamp(channel, 0, 255) * instance[+0x40]`, and drawn size likewise
`size = particle_size * instance[+0x34]` - the second being the severity
`ShipCollisionFx_Trigger` derives from the impact. Confidence **85** for
both, straight off the decompilation, with the caveat that nothing has read
what feeds `+0x40`.

### The two releases author colour at different scales - 2026-08-12

The one thing about a `.pob` that is a property of the **release** rather
than of the file. Measured over both corpora with one unmodified parser:

| release | RGB reaches | alpha channel reaches |
| --- | --- | --- |
| PSP, 35 systems | `255` | `255` |
| PS2, 41 systems | **`127`, never more** | **`127.5`, never more** |

Not one of the PS2's 41 systems puts a byte above `127` in any of the 256
palette entries of any emitter, and not one alpha channel's range exceeds
`127.5`. The **same effect** differs by exactly a factor of two:
`WO_ROCKET_FLARE`'s first palette entry is `[255, 255, 255, 255]` on the PSP
and caps at `127` on the PS2. That is the PS2 GS's own convention, where
`0x80` rather than `0xff` is 1.0, and `127.5` is exactly half of `255`.

**It cannot be detected from the file.** Three things rule that out:

- the container header carries no version or platform word - both releases
  write the same `+0x0a = 1` and `+0x0c = 1`;
- a per-file "nothing above 127, so it is PS2" test misreads a legitimately
  dark PSP effect. One PSP system's brightest channel is `40` and another's
  is `216`;
- the emitter record has no colour-depth field either - every offset in the
  table above holds the same kind of value on both discs.

So a reader has to be told, from the source it opened.
`oag_fx::psys::ColourScale` is that parameter and
`oag_assets::Layout::platform` is what chooses it - the only thing in this
project that branches on the platform rather than on the data, and it says
so.

Confidence **80**: the corpus split is total on both discs and the
same-effect factor of two is exact, but nothing in the PS2 executable has
been read to confirm how it consumes these bytes. What a wrong reading looks
like: every PS2 particle effect draws at a quarter of its intended additive
brightness - half the colour times half the alpha - which is how this was
found, as "the PS2 engine flare is barely visible".

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
  slots.len() * SLOT_LEN` with [`ParticleSystem::parse`](../../crates/pob/src/lib.rs)
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
(`oag_fx::psys`) now carries recovered units rather than
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
first four bytes) and running the unmodified `oag_pob` parser
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
  than "absent from the disc" is the honest claim. **"PS2-only" here is a
  Pulse-vs-Pulse statement and does not generalise across titles**:
  `WO_TRACK_ROCK_DEBRIS` also ships in Wipeout Pure's PSP `Data.wad`, absent
  from Pulse's own PSP corpus, as the name Pure's rocket plays on a track hit
  - see [`pure-status.md`](pure-status.md#pures-particle-systems-decode-unchanged)
  and
  [`rocket-and-collision-fx.md`](../ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md).

Per the rubric, **"a second binary is worth more than a second reading of
the first."** This is a second, independent binary agreeing on every
structural claim without a single adjustment to the parser - real
corroboration, not just a bigger PSP sample.

## Wipeout HD carries the same container, byte-swapped

**Confidence 93**, over all **88** `.pob` under `data/psys/` on the PS3 disc;
the survey is [hd-status](hd-status.md) and the parser runs on them in
`crates/pob/tests/pob_ground_truth.rs`. Read big-endian the container is
this one - the magic reads `PSYS` where the PSP and PS2 write `SYSP`:

- **88 of 88** carry the magic.
- **88 of 88** carry the two constant words, `1` at `+0x0a` and `1` at `+0x0c`.
- **88 of 88** agree with the `+0x04` length rule below.
- **88 of 88** carry a NUL-terminated ASCII name **immediately after the slot
  table**, at `0x10 + slots * 4` - the boundary this page had to work to pin
  down, holding on a third platform at a third set of slot counts - and on
  **86 of 88** that name predicts the file's own name, case aside
  (`WO_DAMAGE_ELECTRIC` for `wo_damage_electric.pob`). The two that do not are
  authoring rather than parsing: `stesparkstest.pob` holds
  `WO_SHIP_COLL_SPARK_DAMAGE` and `wo_ship_explosion_lightshafts.pob` holds
  `WO_SHIP_EXPLOSION`.
- **636 of 636** real slots resolve through the same two-hop pointer fixup to
  an in-bounds target, **526** of them onto NUL-terminated ASCII - the same
  developer texture paths, now rooted at `Z:\WipeoutPSP\HD\` and
  `Z:\WipeoutHD\`, with 17 still on Pulse's own `Z:\WipeoutPSP\X2\`.
- The emitter tree walks **249 records** across the 88, nine deep at its
  deepest, with no record reached twice and every link landing on a full
  `EMITTER_LEN` record.

### The order is settled at float granularity, not by the magic

A 4CC that reads both ways round is weak evidence on its own. Over those 249
records, read each way:

| | big-endian | little-endian |
| --- | --- | --- |
| named `f32` fields plausible | 3,214 of 3,237 | 1,015 of 3,237 |
| channel key counts within `MAX_CHANNEL_KEYS` | 996 of 996 | 0 of 996 |
| keyframe `(time, value)` pairs in `0..=1` | 2,823 of 2,823 | n/a |
| channel modes | only the three known | n/a |

### The one field that is not a plain re-read

The 256-entry colour table at `+0xc4` is a `u32` per entry, parsed as four
bytes, so on a big-endian file each entry arrives reversed and has to be
reversed back. It is legible in the data: a PSP ramp runs `b5 86 57 c8` ->
`30 2e 2e 00` with alpha last, an HD one `ff ff ff ff` -> `00 ff ff ff` with
alpha first. Reversed, HD's `WO_CANNON_SPARKS_DETONATOR` is a white-to-blue
fade, which is what a pulse cannon looks like.

Strings - the name field, emitter names, texture paths - are byte arrays and
read as-is. That is why the parser takes an `oag_formats::ByteOrder`
*parameter* and never byte-swaps the whole file: a blanket swap would mangle
every one of them.

### Retraction: `+0x04` never disagreed - 2026-08-18

**This section previously read "and one field stops agreeing", and reported
that the `+0x04` length check failed on all 88, short of the file length by 48
to 208 bytes, always a multiple of 16 - with a hypothesis that HD appends an
uncounted string block.** All of that was an artifact of the check, which
compared `+0x04` to the *file* length (`scripts/hd-survey.py`, since fixed).

The rule this page states everywhere else is that `+0x04` equals
`HEADER_LEN + payload.len()` where the payload begins **after** the 32-byte
name. That expands to `len(file) - base - 16`, so a file is *always* longer
than the field says, by exactly the header, the slot table and the name. The
same wrong check would have reported the same "shortfall" on all 35 PSP and
all 41 PS2 files. The observed 48-208 range is `base + 16` - which is why it
was always a multiple of 16, the slot table being four bytes per slot.

`crates/pob/tests/pob_ground_truth.rs` now asserts
`len(blob) - declared == resource_base() + 16` on **every file of all three
discs**, so the identity is checked rather than reasoned about.

The line "the emitter tree has not been walked on any HD file" is retracted
with it; it has now been walked on all 88.

### Two things HD authors that Pulse does not

Both are content, not byte order, and both are pinned as measurements:

- **`blend_class` 4**, on the seven emitters of `WO_NITRO_SHIP_DEATH` and
  nowhere else. Pulse writes only 1-3. Nothing is traced about what it draws,
  so it stays unnamed and `oag_fx::psys` refuses it by
  `Error::UnknownBlendClass` rather than falling back to one of the three.
- **`LOOPING` is per-emitter** - see the retraction under
  [flag `0x1`](#flag-0x1-is-this-effect-loops---2026-08-12) above.

### Which names HD does not author - 2026-10-06

The race loader used to ask HD for five effects that no archive of its disc
carries: `WO_RAIN`, `WO_RAIN_LENS`, `WO_SNOW` (Pulse's weather) and
`WO_MAGSTRIP_SPARKS`, `WO_MAGSTRIP_ZONE` (2048's magstrip pair). Each was a
`not in the archive set` warning, which was true and was HD inheriting a
Pulse list. `oag_hd::effects::EFFECTS` now takes them back.

- **Census, all seven PSARCs** (`crates/render/examples/hd_psarc_find.rs`,
  paths read from each archive's manifest): 82 distinct `.pob` systems
  under `/data/psys/`, none of the five. `rain` finds only `/data/tex/rain.gtf`
  and unrelated circuit textures; `snow` finds `/data/tex/snow.gtf` and
  `10_sebenco_climb`'s snow materials; `magstrip` finds the floor art and
  `wo_magstrip_lightning.pob`, which no `EBOOT.elf` string names.
- **Executable, `EBOOT.elf`** (`/hdfury/EBOOT-ps3-hdfury-eu.elf`, Ghidra
  `search_strings`): 99 `WO_` strings, none of the five, and none of
  `WO_BLUE_WELDER`/`WO_MODESTO_STEAM_A` either - an ambient effect's name is
  not a load-path string on HD, so the string search alone proves nothing for
  weather (the 2026-09-08 correction above). The disc census is what settles
  it: a file that is on no archive cannot be played.
- **Magstrip is a different mechanism on HD**: the arc wake (`MagstripWake`),
  never the `.pob` - `Title::magstrip_pob` is `false` for HD and the thread
  that recovered the law puts the `.pob` pair on the 2048 modes only
  (confidence 80, Vita and Omega readings).
- **HD's scenery list is two names**, `WO_BLUE_WELDER` and
  `WO_MODESTO_STEAM_A` (`DATA02`), the two that are on its disc. Where HD
  places weather, if it does (`rain.gtf` and `snow.gtf` are screen textures,
  not particle systems), is unread.
- **Checked against Omega / 2048**: Omega ships the magstrip pair under
  `Data/particles2048/` and 2048 names it (`ships-effects.md`), which is why
  those two are theirs; neither ships `WO_RAIN`/`WO_SNOW` (the 2048 census
  above). **Checked, differs**: HD has neither.

### HD names its own texture the same way Pulse does, and 80 of 82 resolve to a shipped `.gtf` - 2026-09-17

HD does not embed pixels (see "The sprite pixels are on the disc after
all" above), but every emitter still carries the same own-texture field
at `record_start + 0x4c4` this project already reads as "an emitter's own
texture slot" on PSP (`docs/ghidra/functions/psp-pulse-usa/particle-system.md`'s
collision-spark section). Reading it directly - a pre-fixup offset from
the resource base, big-endian, no slot-table indirection needed since the
field *is* the fixup site - resolved a `.tga` developer path string for
**every emitter in every one of the 84 distinct `.pob` files** extracted
from the disc's seven PSARC archives (`scripts/psarc.py extract ... .pob`),
zero missing. Taking each string's basename, lowercasing it and swapping
`.tga` for `.gtf` against `/data/psys/tex/*.gtf`'s own 
entries (a full census of all seven archives, `hd-entries.txt`-shaped)
finds:

- **80 of 82 distinct texture names resolve** to a shipped `.gtf` of the
  identical basename.
- **`vandergraf_balls_1024x1024.tga` does not, but a size-renamed sibling
  does**: `vandergraf_balls_1024x512.gtf` and
  `vandergraf_balls_1024x512_missile.gtf` both ship, at the export's final
  dimensions rather than the authoring path's. Read as the same asset
  under its shipped name, not a genuine gap.
- **`plasma_8x8_1024x1024.tga` has no match at all**, under any name, in
  any of the seven archives - the one real unresolved name in the corpus.

This is name resolution only: no `.gtf` was decoded and nothing is wired
into a draw path. Confidence **80** - the own-texture field read is the
same mechanism already confirmed on PSP, corroborated here by a 100% hit
rate for *some* string (a wrong field offset would produce garbage or
truncated strings on at least some of 249 emitters, not a clean string on
all of them), but the field's role at *render time* on HD specifically
(does the interpreter actually read this field to pick the bound texture,
the way `particle-system.md` traces for PSP) is not independently traced
here.

## Wipeout 2048 carries the same container, and the existing reader takes it unchanged - 2026-10-05

`SYSP`, little-endian, the same emitter record. **174 `.pob` across the five
archives that could hold one; 170 parse and play 240 ticks, 4 are refused by
name** - `crates/fx/tests/psys_2048_ground_truth.rs`.

| Archive | `.pob` |
| --- | ---: |
| base `data.psarc` | 151 (`data/particles` 66 incl. 2 `Get/`, `data/particles2048` 85 incl. 2 `Get/`) |
| `dlc1.psarc`, `dlc2.psarc`, patch `data1.psarc` | 0 |
| patch `data2.psarc` | 23 (17 `particles2048`, 6 `particles`) |

- **The two directories are two authorings.** 64 stems exist in both with
  different bytes (`WO_ROCKET_EXPLO` is 48,384 bytes in `Data/Particles` and
  12,656 in `Data/Particles2048`); 19 exist only in `Particles2048`. The
  executable's path builder chooses between them by game mode
  (`docs/ghidra/functions/vita-2048-eu-v104/particle-paths.md`), and the port
  reads `Particles2048`.
- **The refusals are the dialects the reader already names**: `render mode 3`
  on `WO_BARRIER_COLLISION` and `blend class 4` on `WO_NITRO_SHIP_DEATH`, each
  in both directories. Neither is wired anywhere.
- **Palettes run to 255**, so `ColourScale::Full` is right for Vita (asserted).
- **Sprites are separate `.gxt` files, and every emitter names one.** The
  field at `record + 0x4c4` (HD's own-texture field) is a base-relative offset
  to a `...\Tex\<stem>.tga` developer path on all emitters of the 85
  `Particles2048` effects: 76 distinct stems, every one a
  `data/particles2048/tex/<stem>.gxt` that decodes
  (`oag_pob::ParticleSystem::texture_path`,
  `oag_fx::psys::Effect::parse_with`). Before this the 2048 effects drew
  the procedural radial falloff, which on a colour table authored for a sprite
  is a white blowout.
- **The 17 patch-only effects** (chimney smoke, globes, lens flares,
  `WO_ZONE_CRACKLE`, pyramid lights, `WO_STEAM`, ...) are unreachable at
  runtime: `oag_2048::open` does not mount the 1.04 patch archives.
- **No 2048 circuit places a `ParticleSystem` node** (class `0x3c4`: none in
  any `track.vex`); only `weatherPos` nodes (`0x3da`) appear, on 11 circuits.

## Wipeout: Omega Collection carries the same container, and a new blend class - 2026-10-05

`SYSP`, little-endian. `data00.psarc` and the patch's `data05.psarc` each carry
97 `Data/particles` and 112 `Data/particles2048` effects (the base adds 2 `Get/`
effects to each); the same stems as 2048's directories, different bytes again.
`crates/fx/tests/psys_omega_ground_truth.rs`.

- **95 of 97 and 110 of 112 parse** (2026-10-06; 80 and 96 before blend class 8 was read). The two refused in each
  directory are `WO_NITRO_SHIP_DEATH` (blend class 4) and `WO_BARRIER_COLLISION`
  (render mode 3), both also refused on 2048.
- **Blend class 8 is read as `oag_fx::psys::Blend::Distort`: simulated, not drawn.** It
  is on 31 emitters of 20 Omega effects (all census runs 2026-10-06 over `data05.psarc`),
  every shock-distortion and heat-haze one: `shockdistort`, `distort`, `Heathaze` and the
  `WO_RB_HEAT*` set, always render mode 2. In the five explosions a player sees
  (`WO_ROCKET_EXPLO`, `_TRACK`, `WO_MISSILE_EXPLO`, `WO_BOMB_SMOKERING`,
  `WO_PLASMA_LIGHTNING_EXPAND`) it is exactly one emitter, `shockdistort`, and the
  fireball, smoke, spikes and sparks beside it are class 2 or 3 and now play (they were
  refused with it before). What `psys_normal_heathaze_vp/fp` does is **read** (2026-10-06,
  `docs/ghidra/functions/ps4-omega-eu/heat-haze.md`, conf 80): not a scene grab. The
  particle writes a signed offset `colourScale * alpha * 2/|w|^0.75 * tex.a * (tex.rgb - 0.5)`
  into a separate buffer (depth-gated against linear view depth), and the final
  composite resamples Frame, LowRes and Bloom at `uv + 0.0100021 * (16/9 * d.x, d.y)`.
  It stays **undrawn**, and the page now says what is read and what is not
  (2026-10-06, `heat-haze-2`). Read: `kColourScale` is the emitter record's own
  word at **`+0xc84`** (conf 85; `10.0` on the base eboot's explosions, `2.0` on the
  patch's, `0.8` to `5.0` on the heat-haze set), the program slot is exactly blend
  class 8, the target is `R8G8_SNORM` (75) with a pure additive blend (80), and the
  depth the fragment tests is an `R16F` 960 by 540 target (75). Unread: the target's
  clear value, which pass attaches it, and what writes the depth. The emitter ticks,
  spawns its children and draws nothing, and the loader names it at WARN. Nothing is
  drawn in its place. `Emitter` does not parse `+0xc84` yet: a drawn pass would add it
  as a field, never as a constant.
  **2048 cross-check: checked, differs.** No 2048 `.pob` (`data.psarc`, both patch
  archives, both DLC packs) carries class 8; its explosions have no `shockdistort`.
  Pulse PSP, PS2 and HD carry none either (HD's only unnamed class is 4).
  Pinned by `psys_omega_ground_truth.rs`.
- **Sprites are `.gnf`**, 521 under `Data/particles*/Tex/`, every one BC7 with
  tile mode 13 and every one decodes (`Sprite::from_gnf`). The authored path
  is `C:\WOPS4\Wipeout\Data\particles\Tex\<stem>.tga`, and it names the
  directory the sprite is in, which is **not always the effect's own** (one
  `particles` effect names a `particles2048` sprite): 168 distinct sprites,
  all present but `WO2048_Start_Laser`, which ships nowhere.
- **Which directory a race reads** is chosen by a runtime flag in the
  executable, not read down to a circuit: `docs/ghidra/functions/ps4-omega-eu/particle-paths.md`.
  The port picks by circuit directory (chosen, not measured).

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

**Validated against three discs**, `pulse-psp-usa`, `pulse-ps2-eu` and
`hdfury-ps3-eu` - the third big-endian, through the same parser with a byte
order passed in and no offset changed. That raises the container, the name
field, the slot fixup and the emitter tree to **93**: the same structural
claims now hold on three independently authored corpora and 164 files, one of
them written by a different compiler for a different endianness. What it does
not raise is anything resting on a *runtime* read - the fixup mechanism is
still traced live on PSP only. Not checked against Pure's UMD, or PS2's
`WADSP.WAD`/`PRERACE.WAD`.

## Reproducing

```sh
# WO_SHIP_COLL_SPARK_DAMAGE.POB is PSP Data.wad entry 0533, hash 0xeff1f331
just wad cat 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' 0xeff1f331 > /tmp/spark_damage.pob

# the PS2 corroboration read every candidate straight out of the disc image
# with oag_assets::Archive, entirely in memory - no bulk `wad extract` of
# WADS2.WAD (377 MB compressed, ~650 MB unpacked) needed or advised on a
# space-constrained /tmp
```

`oag_pob::ParticleSystem::parse`, `resolve_slot` and `emitters`
recover the name, slot table, every fixup target and the whole emitter tree
from any of the 35 PSP or 41 PS2 blobs. Per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) only
hand-authored fixtures - never extracted game bytes - may land in the repo,
so the unit tests in `pob.rs` build synthetic blobs replaying the confirmed
fixup and record shapes, and the corpus checks run against the user's own
disc, `#[ignore]`d and out of CI:

```sh
just test-data   # picks up both, along with every other ground-truth test
# or, on their own:
cargo nextest run -p oag-assets --run-ignored all --test pob_ground_truth --no-capture
cargo nextest run -p oag-render --run-ignored all --test psys_ground_truth --no-capture
```

`pob_ground_truth` is the structural corpus walk described above;
`psys_ground_truth` (in `oag-render`) plays the parsed effects, and pins the
collision-spark values against the hand transcription they replaced.

### The sprite pixels are on the disc after all - 2026-09-17

The "unlocated texture reference gap" below and in
[particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md)
said the pixels behind a slot-resolved `Z:\...\Tex\<name>.tga` string were
off the disc entirely. They are not, on PSP: **each `.pob` embeds a texture
record directly after an emitter's own fixed-size record**, at
`resource_base + emitter.offset + EMITTER_LEN` - addressed purely
positionally, with no slot, fixup or string lookup needed to reach it.
`oag_pob::texture` is the parser and carries the full evidence and
confidence scores in its own module doc; the summary:

- **32-byte header**: `u16 width, u16 height, u8 bpp(8), u8 levels(3-4),
  u8 unk, u8 unk, u32 palette_bytes(1024), u32 pixel_bytes, u32 pixel_offset,
  u32 palette_offset` - the last two are plain absolute offsets into the
  blob, not relative to the resource base or the header.
- **Every PSP root emitter carries one** (35 of 35), and so does every other
  emitter: 76 headers across the corpus, 64 at 8 bits per pixel and **12 at 4**
  (128x64 or 64x128, a 16-entry palette), measured by
  `crates/pob/tests/pob_ground_truth.rs`'s
  `every_psp_root_emitter_texture_is_where_the_layout_says`. **This page used
  to say 29 of 35, with `WO_PLASMA_FLASH`, `WO_RAIN`, `WO_SNOW`,
  `WO_LEACHBEAM_CHARGING`, `WO_REPULSER` and `WO_ROCKET_FLARE` carrying none
  ("plausibly drawing an untextured quad")**: the reader refused 4 bpp headers,
  and all six are 4 bpp. Found 2026-10-01 from a picture: the `FIRE` emitter of
  `WO_SHIP_FXNODE_EXPLO` is a 4 bpp sprite too, and a GE dump of the running
  original binds it (`TEXSIZE 0x607`, `TEXFORMAT 4`). The six, `FIRE`,
  `SHIP_DEBRIS` and the `DEBRIS` emitters of `WO_ROCKET_EXPLO`, `WO_MINE_EXPLO`
  and `WO_BOMB_SMOKERING` plus `WO_PLASMA_HEAD`'s `plasma_spikes` had been drawn
  with the procedural white disc.
- **`WO_SHIP_COLL_SPARK_DAMAGE` confirms it names the texture `pob.md`
  already resolved from the slot table**: its root's positional texture is
  32x32/3-level, matching `quakesmoke32x32.tga`'s own size, and decodes
  (`data/shots/quakesmoke32x32.png`, gitignored) to a soft grey puff; its
  three siblings' positional textures all point at one shared 64x64/4-level
  pool matching `orange_glow2.tga`, which decodes to the documented
  white-hot-core-to-orange glow. `WO_PLASMA_HEAD` is the counter-example -
  see `oag_pob::texture`'s own "positional texture is not proven to be
  the one a slot names" section for what it shows there.
- **PS2 embeds none of this**: the identical positional scan over all 41
  PS2 files, at every emitter offset, finds zero header-shaped records -
  consistent with the 62.24% vs 25.62% coverage gap already measured below.
  HD/Fury doesn't use this mechanism either; its sprites are separate
  `data/psys/tex/*.gtf` PSARC entries.
- **Coverage moved from 25.62% to 56.76%** of the PSP corpus's 781,104
  bytes (`crates/pob/tests/pob_coverage_ground_truth.rs`,
  `oag_pob::coverage`) once the header, palette and level-0 pixels of
  every found texture were claimed - more than half of what read as an
  unlocated gap was sitting on disc under a positional address the whole
  time.

Confidence **85** for the header layout and positional-addressing rule;
**40** for the two unknown header bytes and for what distinguishes the six
texture-less root emitters from the twenty-nine with one. The **25.62%**
coverage figure and the "past its last resolved string, unaccounted for"
framing in the first "Not determined" bullet below are now stale by this
section's own **56.76%** measurement - left in place with the correction
noted here rather than rewritten, since the bullet's remaining claim (the
slot-resolved record's *own* internal field layout is still undecoded)
still holds; only the byte-count context around it moved.
particle-system.md's "sits in main RAM with no WAD entry answering any
hashable `Data\Psys\Tex\...` name" note is superseded outright - see that
page's own correction.

### Correction: the two texture offsets are from the resource base - 2026-09-24

The section above read the header's `pixel_offset` (`+0x10`) and
`palette_offset` (`+0x14`) as plain offsets into the blob. **Both are
pointer-fixup sites**, so the loader's `*(resource_base + slot) +=
resource_base` turns each into a live pointer at `resource_base + value`:

- On **every** PSP emitter that carries a header (64 of them over the 35
  files), the header's `+0x10` and `+0x14`, taken relative to the resource
  base, are entries of that file's own slot table -
  `crates/pob/tests/pob_ground_truth.rs`,
  `every_psp_texture_pointer_is_a_fixup_site`. Relative to the emitter record
  they are `+0x9c8` and `+0x9cc`: the two slot targets the bullet below and
  [particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md)
  listed as unexplained. They are the sprite's pixels and palette.
- The runtime bind `FUN_08928b10`, called from `ParticleSystem_DrawParticle`
  on the emitter's texture block, returns without binding when either word
  is zero, passes `+0x10` to `Gu_TexImage` level by level and `+0x14` to the
  CLUT load (`ClutMode(8888)`, 256 entries), and hands bit 0 of `+0x06` to
  the texture mode as its swizzle argument. So byte `+0x06` is a flag byte
  (bit 0 swizzle, bits 3-4 the level-mode selector), not unknown.
- Read at the old offsets every sprite came out wrong in a way that still
  looked like *a* texture: the rows started half a row early with palette
  bytes as pixels, and the palette was shifted by the 96-byte resource base
  of `WO_QUAKE`, so its index 0 - most of the sprite - read as an opaque navy
  `(11, 30, 53, 88)`. Read from the base, every sprite of `WO_QUAKE`,
  `WO_ROCKET_EXPLO`, `WO_ROCKET_EXPLO_TRACK` and
  `WO_SHIP_COLL_SPARK_DAMAGE` is a clean, centred picture: index 0 black on
  all 51 additive emitters' sprites. The 2026-09-17 identifications survive
  the correction - the collision root still decodes to a soft grey puff and
  the three bright siblings' shared pool to an orange glow with a white core.

Coverage re-measured with the corrected regions: **56.89%** of the PSP
corpus's 781,104 bytes (was 56.76% on the misplaced regions). Confidence
**92** for the base-relative reading: the fixup mechanism is confirmed live
above, and these two fields are in its table.

## Not determined

- **The field layout inside a *slot-resolved* record.** A string-bearing
  record is `{ NUL-terminated string, small parameter block }`; a
  non-string record is a run of floats. The *emitter-level* layout that
  used to head this list is decoded ("The emitter record layout is
  decoded" above) and parsed into `oag_fx::psys` with
  confirmed units - what remains undecoded is the slot-table targets'
  own internals (beyond "site `0x4c4` resolves the texture path"). The
  `+0x9c8`/`+0x9cc` targets once listed here are the embedded sprite's
  pixels and palette - see the 2026-09-24 correction above.

  **Measured rather than left as a bare statement**: `oag_pob::coverage`
  claims everything else this page already decodes (header, slot table, name,
  each slot's own fixup site, every emitter record and modifier node) and
  claims a slot's *target* only where it is a NUL-terminated string, per the
  43%/rest-are-floats split above. Swept over
  `crates/pob/tests/pob_coverage_ground_truth.rs`: the PSP corpus first
  reached **25.62%** of 781,104 bytes across 35 files this way (436 resolved
  slots, of which every string target is claimed); the PS2 corpus reaches
  **62.24%** of 374,128 bytes across 41 files. Adding the embedded-texture
  claim below moved the PSP figure to **56.76%** - it does not touch PS2's
  62.24% at all, since PS2 embeds none - so the two numbers that used to say
  "PSP is much lower" now differ by 5.5 points rather than 36.6, and what
  remains of the gap is genuinely the slot-resolved record internals this
  bullet is about, not embedded pixel data nobody had found yet. The single
  worst file (55,392 bytes) still has real unaccounted bytes even after the
  texture claim; what it no longer includes is the header/palette/pixels of
  its own embedded textures. Two passes at the *slot-resolved record's own*
  internals came back a dead end (see the next two bullets), so this pass
  still measures the shape of that remaining gap rather than opening a
  third.
- **What a slot's *position* in the table means**, if anything - the
  attribute-mapper-class-per-index hypothesis is unverified.
- **What a shared target (several slots resolving to the identical offset)
  signifies** beyond "these channels fall back to the same default."
- **The two constant header words at `+0x0a` and `+0x0c`.** Always 1 across
  both the PSP and PS2 corpora; whether that is a version, a type tag, or
  something else is unknown. [`ParticleSystem::parse`](../../crates/pob/src/lib.rs)
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

### The fourth per-particle channel is the atlas frame rate - 2026-09-24

A fifth channel block sits at `+0x778`, `0xe0` after the roll block and
`0xe0` before the emission scale, with the same layout. It was unparsed.
`ParticleSystem_PrepareResource_q` (`0x088f3b68`) hands it to
`ParticleSystem_BakeChannels` (`0x088f9024`) as the fourth of the
per-particle channels, after alpha, size and roll. Its `hi` (`+0x788`)
switches the atlas animation on in `ParticleSystem_CacheModeFlags`.
`ParticleSystem_UpdateParticles` reads its value as frames per tick. Read
and confidence in
[particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md#the-atlas-frame-advances).
`oag_pob::Emitter::frame_rate` parses it.

On the PSP disc every emitter authors the block, and every one parses. The
ones that animate - `hi > 0` and a grid of more than one frame - are:

- the 4x4 smoke and fire of the Rocket, ship and fx-node explosions;
- the 2x2 Shuriken head, trail and rings;
- `WO_PLASMA_HEAD`.

The PS2 and HD corpora parse unchanged (`pob_ground_truth.rs`).

### The sprite templates at `+0x9a4`/`+0x9a8` - 2026-09-30

An emitter can carry **sprite templates**: records the instance turns into one
particle each the moment it exists. Found off a live struck craft (a
`ParticleSystem_DrawParticle` log naming two particles, `shazam` and `glow`,
that no emitter in the tree owned), then read off the instance initialiser
`FUN_088f58a4` (`ParticleSystem_InitInstance`, confidence **80**):

```c
if (res->+0x9a4 != 0) {                       /* a gate, not a count */
    for (r = res->+0x9a8; r != 0; r = r->+0x8cc)
        list_push(instance->+0x1b0, InitParticleFields(alloc(0x80), r, instance));
}
```

`+0x9a8` and the record's `+0x8cc` are resource-base-relative offsets, the same
convention `+0x94c` uses. The record is **not an emitter**; `0x900` bytes, and
confidence **85** for each field below, every one matched against the live
particle:

| offset | field | evidence |
| --- | --- | --- |
| `+0x00` | name (`shazam`, `glow`, `ring`, `BANG`, `PLASMA_GLOW`, `shazzam`...) | the live log |
| `+0x10` | size channel, the emitter's channel-block format | `shazam`: `3.9` held to 0.287, then to 0; live `9.36 = 3.9 * 2.4` |
| `+0x1d0` | alpha channel, `0..255` | live colour alpha `ff` while its hold lasts |
| `+0x390` | roll channel | read by `InitParticleFields` |
| `+0x470` | 256-entry colour table, walked over the life | `glow`'s `fff5f5ff` to `ffe3f2ff` over 15 of 40 ticks |
| `+0x874`, `+0x878` | render mode index, blend class (`2`: additive billboard) | `ParticleSystem_DrawParticle`'s reads |
| `+0x880`, `+0x884` | atlas grid, flags | `InitParticleFields` |
| `+0x888`, `+0x88c` | lifetime centre and spread, ticks | `shazam` 6, `glow` 40; live |
| `+0x8cc` | next record | the initialiser's loop |

**`+0xf0` is the sprite's stretch, and `+0x390` its roll - both played since
2026-09-30.** `ParticleSystem_UpdateParticleFields` (`0x088f7e64`) turns the
`+0xf0` block into the quad's aspect (`v > 0`: `1 + v`; `v <= 0`: size times
`1 - v`, aspect `1 / (1 - v)`), and every template draws as class 3, a
`ParticleSystem_DrawRotatedSprite` quad `aspect * size` wide and `size` tall,
turned by the roll. Confidence **85**, and the `1 + v` law is pinned on the one keyed stretch on
the disc: a Mine's `BANG` (`+0xf0` keyed `1..20` over its six ticks) drew
aspects `2.130, 5.272, 8.468, 11.623, 14.763` live, a bar up to 49 units wide.
Live on a struck craft: `shazam` aspect `1.500`, roll `0`
(`+0xf0` = `0.5`); `glow` aspect `1.700` (`0.7`), roll `2 pi` falling as its
nine-key channel says (flags `0x30`: angle, not rate, random start). The
`+0x2b0` block is the atlas-frame rate, constant `1.0` and unused on a one-cell
grid. Corpus: all 31 PSP templates are class 3 and author a `+0xf0` block - constant
`0.5` on the `shazam`s, `0.7` on the `glow`s of the collision and death sparks,
`1` on the Quake's `shazzam`, `5` on the Repulser's, `20` on the missile's
`redbar`, `0.2` on the welder and fx-node glows, keyed on `BANG` (`1..20`) and
the Plasma's `PLASMA_GLOW` (`0..1`), `0` elsewhere. A non-zero roll is authored
on the sparks' `glow`s (keyed angle), the Missile explosion's `glow` (a constant
rate of `1` rad per tick) and `booga` (keyed, `0..0.52`), and `redbar`
(periodic, `0.03`). Flags at `+0x884`: `0x10` random start angle, `0x20` the roll
channel is the angle, `0x08` a coin picks each particle's turning sense. See
[particle-system.md](../ghidra/functions/psp-pulse-usa/particle-system.md),
"The per-tick field update". **A channel's `period` is a loop in ticks** (the
`glow`'s is 10 over its 40-tick life): the live size peaked at ticks 2-3 and
again at 12-13, `v` at `fract(age / 10)`. **The sprite is the template
record's own, not its parent's (corrected 2026-10-01).** `ParticleSystem_DrawParticle` binds
`*(owner + 8) + 0x890` for a template particle (`FUN_08928b10`; pixel pointer at block `+0x10`, palette at `+0x14`,
so `+0x8a0` and `+0x8a4` of the record), a 32-byte texture header in the template record itself. The collision
sparks' templates carry the pool their parent shares (`orange_glow2` 64x64, read off the live record's pointers),
which is what had been read as a rule. Elsewhere they differ: the ship explosion's `Glow` hangs on `SHIP_DEBRIS`
(128x64, 4 bpp, a grey atlas) and carries a **32x32, 8 bpp, 3-level radial glow** (centre white, corner black); the GE
dump of the running original binds exactly that (`TEXSIZE 0x505`, `CLUT8`, a grey palette whose alpha equals its colour).
All 28 PSP templates parse a header at `+0x890` (`crates/pob/tests/pob_initial_particles_ground_truth.rs`).
Confidence **90** (decompile, GE dump and the file agree).

**Where they are.** 28 on the PSP disc, on 20 effects (31 in the PS2 port's, unread): the collision sparks'
`shazam` (on `WO_SHIP_COLL_SPARK`) and `glow` (on the `_TRAIL`), the Plasma's
`PLASMA_GLOW` (half-size 36), the Mine's `ring` and `BANG`, the Missile's `glow`
and `booga` (72 to 100), the Quake's and the Repulser's `shazzam`, the Shuriken's
`glow`, the ship explosions' `Glow`, the absorb effect's `glow`. **None of it
was drawn before this**: the parser never reached them. The struck hull's warm
envelope in the original is this pair. Read by `oag_pob::initial`,
played by `oag_fx::psys::template`; pinned by
`crates/pob/tests/pob_initial_particles_ground_truth.rs`.

**Not played:** a template on an emitter that is itself a child (it would start
with the child's own instances; `Effect::skipped_templates` counts them).
**The PS2 and HD files are muted**: the layout is read off Pulse's PSP
executable alone. **A template's first draw is at age 0**: the instance's first
update makes the particle and draws it before anything ages it (live: `glow`
`0.75` then `3.18`, `shazam` `9.36` twice), so `oag_fx::psys` skips the
first tick's ageing for a template particle. An emitter's particles are not
treated that way - unmeasured. (Their roll and aspect are a different law: a
constant `+0x4c8` aspect and a roll that is always a rate; see the emitter
record's table above.)

## Effects Pulse names that 2048 and Omega never author - 2026-10-06

`WO_PLASMA_FLASH`, `WO_SHIP_ENGINEFLARE`, `WO_LEACHBEAM_ENERGY`, `WO_BLUE_WELDER`,
`WO_RAIN`, `WO_RAIN_LENS` and `WO_SNOW` are Pulse's, and the engine's name table
inherits them to every title (`Effects::engine`). On **neither** 2048 nor Omega is any
of the seven authored:

- No `.pob` of that stem in any archive: 2048's `data.psarc`, `data1`/`data2`,
  `dlc1`/`dlc2` (both regions, `PCSF00007` and `PCSA00015`, identical `.pob` sets) and
  Omega's `data00`/`data05` (`WO_ENV_SEB_SNOW`, `WO_ENV_ALT_CAMFLASH_1` and the 2048
  `WO_LENS_FLARE_*` are different effects, not stand-ins).
- No string of the executable names them (`grep -a` over the v1.04 and base 2048
  `eboot.elf` and Omega's `eboot.bin`: 0 hits for each, against 2 for
  `WO_ROCKET_FLARE`, `WO_MAGSTRIP_SPARKS` and `WO_PLASMA_HEAD`).

So they are by design absent, not mislooked: `oag_2048::TITLE` and `oag_omega::TITLE`
drop the three triggers (`PlasmaBlast`, `EngineFlare`, `LeachbeamEnergy`) and keep only
`WO_MODESTO_STEAM_A` (which both ship) as scenery. 2048's engine flare is the per-ship
`engineflare.rcsmodel`/`.vex` the livery path loads, a model and not a `.pob`.
**Cross-check:** the same finding holds on both titles, recorded once; HD is
`logwarn-hd`'s. Omega alone drops `MagstripSparks`/`MagstripZone` too: its
`WO_MAGSTRIP_*` ship only under `Data/particles2048/`, and nothing on Omega fires them
(`weapon_models.magstrip_pob` is false; the arc wake is HD's mechanism, see
`ps4-omega-eu/ships-effects.md`), so loading them from `Data\particles` was an
unread-trigger absence. No cross-directory fallback was added.

### The weapon textures keep their stems and change extension

`Data\Weapons\Textures\Cannon_bolt`, `cannon_muzzle_flash`, `pulse_leechbeam1_ADD` and
`Data\Tex\staticglow` ship on 2048 as `.gxt` and on Omega as `.gnf` (Omega under
`Data/weapons/` and `Data/weapons2048/`), the same stems Pulse ships as `.mip`; the
loader asked for `.mip` and logged four absences. `oag_raceplay::assets::platform_sibling`
now swaps the extension on the Vita and PS4. Both executables name `Cannon_bolt` and
`Cannon_muzzle` (3 and 2 hits) and neither names `pulse_leechbeam1` or `staticglow`: the
bolt and flash textures are the engine's own, the leech ribbon and ghost static are
Pulse's law applied to an asset the title happens to ship (unmeasured title inherits
Pulse's rule, **chosen, not measured**).

### Omega's explosions were white because their sprites did not fit - 2026-10-06

`oag_fx::psys::Sheet` packs every loaded effect's sprite into one square texture; a sprite
that does not fit leaves its emitter on the procedural (white, radial) profile. Omega's
`WO_ROCKET_EXPLO` authors `explosion_BURN_L_8x8`, `explosion_tumult_8x8` and
`rb_WispySmoke01_8x8` at 1024x1024 (`.gnf`, mean colour warm orange, R 27-57 > G > B),
and the whole Tech De Ra effect set needs more than 2048: at the old 1024 the fireball,
tumult and smoke emitters drew the white disc (the first explosion frame was a flat
whiteout), at 2048 three emitters still did. `SHEET_SIZE` is now 4096 (**chosen, not
measured**; 64 MiB) and an emitter whose sprite still does not fit is a WARN report line
("did not fit the sprite sheet"), not a silent substitution. The authored palettes of these
emitters are plain white (`pal0 = pal128 = [1,1,1]`), so all of the colour is the sprite's.
Frames: `data/scratch/logwarn-2048/shots/ex/omega4096-strip.png` (ticks 90 to 240, orange
fireball, debris and sparks) against `omega-strip.png` (before). Pulse's own rocket blast
(`pulse-150.png`) is orange the same way. Effect on other titles: a sheet that held their
sprites at 1024 places them identically (rects scale with the size); a title whose sprites
did not fit now places them (not measured per title).
