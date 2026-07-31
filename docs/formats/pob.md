# Particle system

**Status: partial.** The container, the slot table (including the pointer
fixup it drives at load time, confirmed live), the name field, and what a
slot resolves to are all decoded, implemented in
[`oag-formats::pob`](../../crates/formats/src/pob.rs) and validated against
all 35 `.pob` files on the PSP disc. What is **not** decoded is the record
layout at a resolved target: 43% of them are readable developer strings
(texture paths, layer names), and the rest are floats whose field boundaries
are still unread.

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
unverified.

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

`oag_formats::pob::ParticleSystem::parse` and `resolve_slot` recover the
name, slot table and every fixup target from any of the 35 PSP or 41 PS2
blobs; no ground-truth test is committed because, per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md), only
hand-authored fixtures - never extracted game bytes - may land in the repo.
Unit tests in `pob.rs` build synthetic blobs replaying the confirmed fixup
shape instead.

## Not determined

- **The field layout inside a resolved record.** A string-bearing record is
  `{ NUL-terminated string, small parameter block }`; a non-string record is
  a run of floats of a similar general shape. Neither has decoded field
  boundaries.
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
  executable.** See above - the preload-array lead is the next static step.
- **Whether `WO_SHIP_COLL_SPARK`, `_TRAIL` and `_TRAIL_SMOKE` exist on PS2**
  outside `WADS2.WAD`.
