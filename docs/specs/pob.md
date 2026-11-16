# `.pob` particle systems

One authored effect (sparks, an explosion, rain, a weapon trail): a tree of
emitters, each with a fixed-layout record, colour table and animation curves.
Files live at `Data\Psys\<NAME>.POB` inside the [WAD](wad.md) (for example
`WO_SHIP_COLL_SPARK_DAMAGE`, `WO_MISSILE_EXPLO`, `WO_RAIN`).

| Title / platform | Count | Notes |
| --- | ---: | --- |
| Pulse PSP | 35 | Little-endian, magic `SYSP`. Emitters carry their sprite pixels inline |
| Pulse PS2 | 41 | Same layout; sprites are separate textures; colour scale is 0 to 127 |
| HD/Fury (PS3) | 88 | Big-endian, magic `PSYS`; same layout otherwise. Sprites are `.gtf` |
| 2048 (Vita), Omega (PS4) | 174; 209 per archive | Little-endian `SYSP`; sprites are `.gxt` / `.gnf` |

Not checked on Pure. Confidence scores follow the
[rubric](../reverse-engineering/confidence-rubric.md) (0-100). Integers and
floats are in file byte order (little-endian except HD); offsets below are the
same on every platform.

## Container

```text
+0x00  char[4]  "SYSP" (or "PSYS" on a big-endian build)
+0x04  u32      16 + length of everything after the name field
+0x08  u16      slot count
+0x0a  u16      1
+0x0c  u32      1
+0x10  u32[count]   slot table: 0xffffffff = unused
       char[32]     resource name, NUL terminated   <- the resource base
       ...          emitter records and the data they point to
```

The name field's position is immediately after the table, not fixed. Hashing
`Data\Psys\<name>.POB` with the [WAD name hash](wad.md#the-name-hash) gives the
blob's own WAD entry hash on every file. Confidence 93.

**Everything else is addressed relative to the resource base**, which is the
first byte of the name field (`0x10 + 4 * slot_count`).

### Slot table: pointer fixups

A slot value is the offset (from the base) of a **fixup site**: a 4-byte field
that holds a second offset, also from the base. The game's loader adds the base
to every such field in place, turning it into a pointer. To resolve slot `i`:

```text
site   = base + slots[i]
target = base + read_u32(site)
```

All 436 PSP slots and 266 PS2 slots resolve in bounds. The targets are usually
NUL-terminated strings (texture paths such as `...\Psys\Tex\orange_glow2.tga`,
layer names such as `GLOW`) or short float runs; 187 of 436 are strings. The
slot table is only needed to find texture names and the pointer fields listed
below; the emitter tree is walked without it. Confidence 90 for the mechanism
(read live on PSP, 26 of 26 sites); 70 for the string records; the float
records are not decoded.

## Emitter record

The root emitter's record starts at the resource base, so its first 32 bytes are
the resource name. Every emitter has the same layout. Tree pointers are offsets
from the base. Walk the tree depth first:

- `+0x944`: effect spawned where a particle of this emitter dies, or 0.
- `+0x948`: effect attached to each particle, or 0 (spawned with probability
  `+0x4d4`).
- `+0x94c`: next sibling emitter, or 0.

A rocket explosion is seven records deep in both directions. A reader must cap
the walk and reject revisits (the largest shipped tree has seven records).

| Offset | Type | Field |
| ---: | --- | --- |
| +0x00 | char[32] | Name (resource name on the root, layer name such as `GLOW` below) |
| +0x20 | u32 | Flags, below |
| +0x24 | f32 | Duration in ticks; bursts if flag `0x800000` |
| +0x28, +0x2c | f32 | Unknown pair |
| +0x30 | u32 | Shape: 0 point, 1 line, 2 rectangle, 3 ring or disc, 4 sphere, 6 box, 7 hemisphere, 8 half ring |
| +0x34 | f32 | Spawn extent (radius), world units, scaled by severity |
| +0x38, +0x40 | f32 | Further extent values; `+0x38` often holds garbage, so treat as unread |
| +0x3c | u32 | Radius shaping: 0 exact, 1 spread, 2 `* sin(U(0, pi/2))` |
| +0x44 | u32 | Velocity mode: 1 aimed, 2 tangent on spheres, 0 and 2 otherwise cone |
| +0x48, +0x4c | f32 | Ejection speed centre and spread, units per tick, scaled by severity |
| +0x50 | f32 | Elevation over the emitter's horizontal plane, radians |
| +0x54 | f32 | Azimuth added to the spawn heading, radians |
| +0x58 | f32 | Cone half-angle, **degrees** |
| +0x5c, +0x60 | i32 | Particle lifetime centre and spread, ticks |
| +0x64, +0x68 | i32 | Ticks between emissions, min and max |
| +0x6c, +0x70 | i32 | Particles per emission, min and max |
| +0x74 | f32 | Gravity, units per tick squared; used only with flag `0x200` |
| +0xa0 | i32 | Live particle cap; an emission that would exceed it is skipped whole |
| +0xb8 | u32 | Render mode, indexes an 8-entry table; draw class is the index plus 1 (3 rotating billboard, 6 and 7 two-point streaks, 1 and 2 camera-facing quad) |
| +0xbc | u32 | Colour mode: 2 means the colour is fixed at spawn; otherwise the table is walked over the particle's life |
| +0xc0 | u32 | Blend class: 1 alpha test, 2 additive, 3 alpha over; Omega adds 8 (distortion) and HD authors a 4 that is unread |
| +0xc4 | u8[256][4] | Colour table, RGBA |
| +0x4c8 | f32 | Aspect (half-width over half-height) for class 3; also the bar cap on class 6 |
| +0x4cc | f32 | Playback rate: multiplies the tick step for every clock of this emitter |
| +0x4d0 | f32 | Fraction of the parent particle's velocity a child inherits |
| +0x4d4 | f32 | Probability a parent particle spawns this system |
| +0x4d8 | channel | Size (half-size, world units, times severity) |
| +0x5b8 | channel | Alpha, 0 to 255 |
| +0x698 | channel | Roll speed, radians per tick (class 3 only) |
| +0x778 | channel | Sprite atlas frame rate, frames per tick |
| +0x858 | channel | Multiplier on the emitter extent, over the **emitter's** age |
| +0x93c, +0x940 | i32, ptr | Animated attributes: count, and array of 0xec-byte records (a channel at +0x00, a selector at +0xe0) |
| +0x9a0 | u32 | Atlas grid: columns in the high half, rows in the low half, read in the file's byte order (`docs/formats/pob.md`, "The atlas grid word"). The frame count is their product |
| +0x9a4, +0x9a8 | u32, ptr | Initial sprite templates (below) |
| +0x9ac | i32 | Not the frame count; reader unknown |
| +0x9b4 | ptr | Modifier list |
| +0x9b8 | 32 bytes | Embedded sprite header, PSP only (below) |
| +0xc84 | f32 | Distortion strength, blend class 8 (Omega); on a draw class 6 emitter the `kColourScale` of its batch (`WO_TRAIL_HITSHIP`: 0.7, 1.0; Omega's reading) |

Zero bytes between and after these fields (`+0x9f0`, `+0xa70`, `+0xc70`) are
scratch the loader fills.

Units are the original's own and should be converted once: speeds and gravity
are per **tick**, lifetimes and schedules are integer ticks. Severity (an
instance scalar the owner supplies, for example from collision strength) scales
speed, extent and drawn size, never particle counts.

Confidence 85 for the table as a whole, and every row inherits it unless noted:
each row is grounded in a decompiled consumer, and one file's values were
matched byte for byte against a live emulator session. The playback rate is 90;
the looping flag is 75.

### Flags (`+0x20`)

| Bit | Meaning | Score |
| ---: | --- | ---: |
| `0x1` | Looping: runs until the owner stops it; duration is not a countdown | 75 |
| `0x2` | Particles stay in the instance's frame and ride its matrix (local space) | 85 |
| `0x4` | Random initial roll | 85 |
| `0x8` | Random roll direction per particle | 85 |
| `0x20` | Sub-frame spread of a burst along the emitter's motion | 85 |
| `0x200` | Gravity enabled | 85 |
| `0x800` | Particles never expire | 85 |
| `0x200000` | Evenly stepped ring or Fibonacci sphere instead of random | 85 |
| `0x800000` | Duration counts bursts, not ticks | 85 |
| `0x2000000` | Streak's second point stays at the spawn position | 85 |
| `0x4000000` | Random atlas frame per particle | 85 |

On Pulse, looping is uniform across one effect. On HD it is per emitter.

### Channel block

Used five times above and inside attribute records. 0xe0 bytes:

```text
+0x00  f32   period (unused)
+0x04  u32   mode: 0 keyframed, 2 constant, 3 random
+0x08  i32   key count (at most 25)
+0x0c  f32   lo
+0x10  f32   hi
+0x14  (f32 time, f32 value)[count]    time and value both 0 to 1
```

Keyframed: linear between keys, clamped at both ends, then
`lo + (hi - lo) * value`. Constant: `hi`. Random: one uniform draw in
`[lo, hi]` at spawn. Time is the particle's normalised age (the emitter's, for
`+0x858` and attribute records).

### Colours

The table is walked at index `floor(age * 255.999)` where `age` is 0 to 1. The
interpreter uses only RGB from an entry and keeps the particle's own alpha;
the table's alpha column is authored but never read. Confidence 85.

The PSP stores colours up to 255 and the PS2 up to 127 (its GS treats 0x80 as
1.0), with alpha channels capped at 127.5. Nothing in the file says which: the
reader must be told by where the file came from. Confidence 80. On big-endian
builds the table is a `u32` per entry, so bytes arrive reversed.

### Modifiers (`+0x9b4`)

A linked list of nodes: nine floats, a type `u32` at `+0x24`, the next node
(offset from the base, 0 ends) at `+0x30`. Only type 3 is understood: per-axis
exponential drag, with the first three floats as per-tick decay factors (for
example 0.98 on each axis). Other types dispatch to handlers nobody has read.
Confidence 85 for type 3.

### Initial sprite templates (`+0x9a4`, `+0x9a8`)

A non-zero `+0x9a4` means a list of 0x8d0-byte templates starts at `+0x9a8`, each
linked to the next by `+0x8cc`. Each template makes one particle the moment the
instance starts. A template is a size channel at `+0x10`, stretch at `+0xf0`,
alpha at `+0x1d0`, roll at `+0x390`, colour table at `+0x470`, render mode at
`+0x874`, blend class at `+0x878`, atlas grid at `+0x880`, flags at `+0x884`,
lifetime centre and spread at `+0x888` and `+0x88c`. The collision-spark effect
uses two (a white flash and an orange glow). Confidence 85.

## Sprites

On PSP an emitter's sprite is stored directly after its record, at
`base + record_offset + 0x9b8`:

```text
+0x00  u16  width
+0x02  u16  height
+0x04  u8   bits per pixel, 8 or 4
+0x05  u8   mip levels (3 or 4)
+0x06  u8   flags: bit 0 = GE swizzled
+0x08  u32  palette bytes (1024, or 64 at 4 bpp)
+0x0c  u32  pixel bytes, whole mip chain
+0x10  u32  pixel data offset, from the base (a fixup site)
+0x14  u32  palette offset, from the base (a fixup site)
```

Pixel and palette formats are those of [textures](textures.md) (4 bpp is low
nibble first). Several headers may share one pixel pool. A PSP sprite is not
proven to be the texture the emitter's own name slot names; use it as the
"this emitter has a sprite" signal. Confidence 85 for the header and for the
base-relative offsets (92). PS2, HD, 2048 and Omega embed no sprite; they name a
`.tga` path in a slot and ship the sprite as a separate texture entry
([PS3](../formats/gtf.md), [Vita](../formats/gxt.md), [PS4](../formats/gnf.md)).
The path field is at `record + 0x4c4` on HD and 2048.

## Known unknowns

- The float records behind non-string slot targets.
- `+0x28`, `+0x2c`, `+0x38`, `+0x9ac`, and modifier types other than 3.
- What HD blend class 4 and render mode 3 (refused on 2048 and Omega) draw.
- Which of 2048's two particle directories a race reads (chosen by a runtime
  game-mode flag).
- Triggers: which game event starts which effect lives in the game code, not the
  file.

## Evidence

- Evidence page: [`docs/formats/pob.md`](../formats/pob.md); interpreter
  functions in
  [`docs/ghidra/functions/psp-pulse-usa/particle-system.md`](../ghidra/functions/psp-pulse-usa/particle-system.md).
- Reader: [`crates/pob/src/lib.rs`](../../crates/pob/src/lib.rs) and its sibling
  modules; ground truth in `crates/pob/tests/pob_ground_truth.rs`.
