# Particle system

**Status: partial.** The container, the slot table's shape and the name field
are decoded, implemented in
[`oag-formats::pob`](../../crates/formats/src/pob.rs) and validated against
all 35 `.pob` files on the PSP disc. What is **not** decoded is the payload:
the slot table's values are not understood, and neither is anything past the
name field.

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
       ...      payload, undecoded
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
particle systems for the effect `oag_render::sparks` currently authors by
hand (see [contact-response.md](../ghidra/functions/psp-pulse/contact-response.md)
and the `HANDOVER.md` open thread), and `WO_SHIP_COLL_SPARK_TRAIL` and
`WO_SHIP_COLL_SPARK_TRAIL_SMOKE` are two more in the same family.

## What the slot table holds is not known

The table reads as absolute file offsets, but into a region that starts at a
fixed boundary rather than right after the table itself - see the next
section for why that boundary, not the table, explains the one number that
looked most like it ruled offsets out. Census over all 35 files:

- **Every real (non-`0xffffffff`) slot value is inside `[1220, file_size)`**,
  and the values within one file are monotonically non-decreasing. Both hold
  on all 35 files with no exception - the shape an offset table into
  file-absolute positions has to have.
- **Slot 0 is exactly 1220 in every file that has one**, regardless of the
  table's own length (4 to 32 slots) or the file's total size (5,808 to
  55,392 bytes). Read as a *payload-relative* offset (from `name_end`, right
  after the table and name) this is impossible - `name_end` itself varies
  64 to 176 across those same files, so a self-relative value cannot stay
  fixed. Read as *file-absolute*, into a region whose start really is a
  constant 1220 regardless of the preceding table's length, it is exactly
  what the invariant should look like. See the fixed-first-region finding
  below for why that boundary itself is real.
- Slots 1-3 cluster tightly (2368-2508) across every file that has them, and
  later slots drift wider but still repeat: 468 slots across the corpus carry
  only 194 distinct values. Reused values across differently-authored effects
  are consistent with an offset table whose earliest entries point at
  small, similarly-sized shared records (see the ~224-byte record below),
  not with continuous per-file-unique offsets.
- `0xffffffff` (1 to 3 per file, on the files that have any) is always in the
  table's **trailing** slots, never an interior one, on all 35 files - so
  reading it as "used prefix, reserved tail" rather than "this specific slot
  index is semantically absent" is the reading the census supports. Slot
  count comes from `+0x08`, not from scanning for the sentinel - the
  scan-for-terminator reading was the original, wrong one, and is what broke
  on the files with no trailing `0xffffffff` at all.

A fixed slot per **attribute-mapper class** - `ParticleAgeMapper`,
`ParticleColorMapper`, `ParticleIncandecenceMapper`,
`ParticleTransparencyMapper` and others found by `search_strings` over the
executable, standard Maya nParticle/particle-dynamics node names - was the
working hypothesis for what a slot's *value* means before the offset reading
above. It is weaker now that the values are file-absolute offsets rather
than a small closed vocabulary, though the slot table's *index* could still
be a fixed per-mapper-class position even if its *value* is an ordinary
offset - the two are not mutually exclusive. Confidence 30, unverified in
either form.

## The payload: two structural leads, no schema yet

Past the name field, payload sizes range 5,744 to 55,216 bytes. The parser
exposes it as raw bytes - nothing below is implemented, because neither lead
is close to a full record layout - but two things came out of a corpus-wide
scan that are worth recording rather than losing.

**Byte offset 1220 (from the start of the file) is a fixed boundary on every
file.** The region between the name field and absolute offset 1220 is not
padding - its content differs across files that share the same table shape -
so it reads as a fixed-size, **1220-byte total** first region (header + table
+ name + a per-system parameter block that fills whatever the variable-length
table and name left over), with the real tabulated data starting at the fixed
boundary right after it. `name_end <= 1220` and slot 0 `== 1220` hold on all
35 files, but `name_end` maxes out at 176 against a 1220-byte budget, so the
inequality has roughly 1,000 bytes of slack and is close to free - the real
weight of the finding is slot 0 landing on the exact boundary every time, plus
every other real slot value landing inside `[1220, file_size)` and ascending
- not the inequality itself. Confidence 65: one exact repeated constant and a
corpus-wide range check, short of the arithmetic-invariant tier because
nothing inside either region (the parameter block, or the first tabulated
record) is decoded yet.

**A ~224-byte record repeats through the tabulated region, shaped like a
keyframe.** Scanning payloads for three consecutive `1.0f` values turns one up
in 34 of 35 files, always followed by one of two values: `10000000.0` (327 of
390 occurrences corpus-wide) or a small real number, most often `0.1` (45
occurrences). Consecutive hits inside one run are `224` bytes apart on **169**
separate gaps corpus-wide, with `448` (`224 * 2`, one record differently
shaped) the next most common. `10000000.0` reads as a "this segment does not
end" sentinel - the kind of oversized constant an exporter uses for unbounded
extrapolation - and the small trailing value as the last real, terminal
sample of whatever curve the record belongs to. All five `WO_SHIP_COLL_SPARK*`
files carry the identical record at the identical offset relative to their
own name field (`name_end + 0x4a8`), which is corpus-wide schema, not
anything specific to collision sparks. Confidence 55: the repeat spacing and
the sentinel/terminal-value split are both real and corpus-wide, but nothing
ties this candidate 224-byte record to a start offset, a field count inside
it, or a specific attribute-mapper class.

Neither lead is close to a `pob.rs` change: a boundary and a probable record
size are not a schema, and guessing the fields inside a 224-byte record from
three matching floats at its tail would be exactly the kind of unverified
structure this project's own rubric marks down for.

## Where the parser is in the executable

The loader is found and fully decompiled: `FUN_089156a0` builds
`Data\Psys\%s.POB`, calls `FUN_088f3540` to read and resource-cache the raw
bytes, and returns a handle - it does not itself interpret `SYSP`. Whatever
walks the slot table and the payload is a **different, not-yet-located**
function. Both a direct-caller search and an address xref search on
`FUN_089156a0` come back empty, meaning it is reached through an indirect
(vtable or function-pointer) call site that static analysis has not resolved.
`FUN_08a6bd18`, the class-descriptor slot `Vex_RegisterClass` installs for
`ParticleSystem` class `0x3c4`, is a trivial self-address-returning thunk -
the same dead end already documented for `Ship Collision Fx`'s slot
(`FUN_08a6ba70`) earlier in the collision-sparks work, now seen a third time.
Closing this needs a live PPSSPP capture (breakpoint on a particle-system
spawn during a scripted scenario), not more static reading - see
`HANDOVER.md`.

## Evidence summary

Every structural claim above holds on **35 of 35** files:

- `+0x04` equals `16 + payload.len()` exactly.
- The name field, read immediately after the table with no fixed offset,
  round-trips through `Data\Psys\<name>.POB` and [`wad::hash_name`](wad.md#the-name-hash)
  to the blob's own WAD entry hash.
- Slot count from `+0x08` matches the boundary the name-field check needs to
  land on, on every file.
- Slot 0 is 1220 on every file that has a slot 0.

Confidence: **90** for the container, the slot count and the name field - an
exact arithmetic invariant (the size field) plus an exact hash agreement
(the name), both across the whole 35-file corpus, capped per the
[rubric](../reverse-engineering/confidence-rubric.md) at 94 for data
agreement and marked down slightly because two header words (`+0x0a`,
`+0x0c`) are asserted constant from a 35-file sample with no corroborating
read of the executable. **65** for the 1220-byte fixed first region: real and
corpus-wide, but a boundary rather than a decoded field, and part of the
check (`name_end <= 1220`) is a loose bound rather than an exact one. **55**
for the ~224-byte repeating record: real and corpus-wide, but no field inside
it is identified. **30** for a fixed slot per attribute-mapper class -
weakened, not strengthened, by the slot values turning out to be file-absolute
offsets rather than a small closed vocabulary; see above.

**Validated against one disc**, `pulse-psp-usa`. Not checked against
`pulse-ps2-*` or Pure's UMD.

## Reproducing

```sh
just wad extract 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' -o /tmp/data-wad
# WO_SHIP_COLL_SPARK_DAMAGE.POB is entry 0533, hash 0xeff1f331
just wad cat 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' 0xeff1f331 > /tmp/spark_damage.pob
```

`oag_formats::pob::ParticleSystem::parse` on any of the 35 extracted blobs
recovers the name and slot table; no ground-truth test is committed because,
per [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md), only
hand-authored fixtures - never extracted game bytes - may land in the repo.
Unit tests in `pob.rs` build synthetic blobs instead.

## Not determined

- **What the slot values mean.** See above - the attribute-mapper-class
  hypothesis is unverified.
- **The payload layout.** Two structural leads (the 1220-byte fixed first
  region, the ~224-byte repeating record), neither close to a schema - see
  above. Turning either into fields needs the parser function or a live
  capture to correlate against, not more corpus scanning.
- **The two constant header words at `+0x0a` and `+0x0c`.** Always 1 across
  the corpus; whether that is a version, a type tag, or something else is
  unknown. [`ParticleSystem::parse`](../../crates/formats/src/pob.rs) refuses
  any other value rather than silently accepting an unrecognised header, the
  same choice `sblk.rs` makes for its own version field.
- **Where the SYSP interpreter lives in the executable.** See above.
