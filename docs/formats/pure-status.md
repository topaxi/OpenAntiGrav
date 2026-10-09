# Wipeout Pure: what the Pulse format layer already reads

**This page is a measurement, not a milestone.** It records what happened when
the existing tooling - `oag-unpack`, `oag-wad`, `oag-view` - was pointed at
*Wipeout Pure*'s UMD without changing a line of parser code, per
[ADR-0009](../architecture/adr/0009-multi-game-fanout.md)'s cheap probe. Nothing
here starts Pure support. Every success is a **third validation corpus** for a
format page; every failure is a **finding about where the parsers overfit to
Pulse**, recorded rather than patched.

Nothing on this page has been verified under an emulator. Every score rests on
static reading plus exact agreement with shipped data, which
[caps it at 94](../reverse-engineering/confidence-rubric.md).

The disc is `pure-psp-usa.chd`, serial `UCUS-98612`; its SHA-256 is in
[source images](../reverse-engineering/source-images.md).

## The headline

The container and geometry layers are **portable as read**. `.vex` renumbers its
class-ID space wholesale, and that single fact - not any difference in the
payloads themselves - is what stops `oag-view` drawing a Pure ship today.

| Layer | Verdict |
| --- | --- |
| Disc, WAD container, name hash | Reads unchanged |
| `.mip` texture, `.fnt` font, `.PMF` movie | Reads unchanged |
| `.vex` file header, node tree, mesh batches, texture block | Reads unchanged; **found by different class IDs** |
| `WO Track` payload | Reads unchanged; **found by a different class ID** |
| Front-end XML | Reads; the name-shortening optimisation **does not exist yet** |
| Handling stats | Same addressing, schema enumerated in full; the absent `<pitch>` is now optional and a labelled stand-in fills it |
| Collision classes | **All three named**: floor and wall by facing 2026-08-12, and the whole renumbering by [class-name table index](#the-renumbering-is-a-table-index-and-both-executables-carry-the-table) - which also settles reset at `0x37f` |
| [`SBlk` sound bank](#sound-effects-the-same-container-after-all) | **Present, 29 banks, and playing**: the same container Pulse uses, unmodified |
| [Music](#music-recovered-by-name-and-played) | **Recovered by name and played**: front end and all nineteen soundtrack tracks |

## Disc layout

`oag-unpack info` identifies the disc with no help: platform PSP, serial
`UCUS-98612`, boot `PSP_GAME/SYSDIR/EBOOT.BIN`, ISO 9660 with 2048-byte sectors,
volume created 2005-06-02. 38 files in 4 directories, 241 MiB. Same shape as
[Pulse's UMD](../psp/pulse-disc-layout.md), with four differences worth knowing:

- **Three archives, not four.** `Data.wad`, `FE.wad` and `FEData.wad` are all
  present under `PSP_GAME/USRDIR/`; Pure has no `BEData.wad`.
- **No firmware update payload and no `GSHARE/SHARE.BIN`.** Pulse carries a
  `SYSDIR/UPDATE/` tree and a per-serial `USRDIR/<serial>/` mirror of its own
  executable and modules. Pure carries neither, which is why its image is 241
  MiB against Pulse's 354 MiB despite comparable asset volume.
- **The network stack ships loose.** `USRDIR/PRX/` holds 24 modules on Pure
  (the `pspnet_*`, `libhttp*`, `libssl` family) against Pulse's 3.
- `USRDIR/DevMenu.cfg`, a plain-text key/value file, has no Pulse counterpart.

`PSP_GAME/SYSDIR/BOOT.BIN` is an unencrypted ELF, exactly as on Pulse, so the
Ghidra route needs no decryption step here either.

`oag-unpack sniff` flags 6 files with no recognised signature: the three
archives, `UMD_DATA.BIN`, `PARAM.SFO` and `DevMenu.cfg` - the same set it flags
on Pulse, for the same reason (the WAD header is a version word, not a magic).

## Results

Counts below are over all three archives, 1,229 entries, unless stated.

| Format | Verdict | Evidence | Confidence |
| --- | --- | --- | ---: |
| [ISO 9660 / CHD](README.md#platform-formats) | reads | Platform, serial and volume descriptor identified with no per-title branch; 38/38 files listed and extracted | 94 |
| [WAD container](wad.md) | reads | 3 archives parse; version word 1; the offset chain is consistent, 64-byte aligned and gapless in all three; `oag-wad verify` decodes **1,229/1,229** entries to their declared size | 94 |
| [WAD name hash](wad.md#the-name-hash) | reads | Pulse-shaped paths resolve directly against Pure's directories - no salt or seed change. See [name recovery](#name-recovery-transfers-intact) | 94 |
| [LZSS](lzss.md) | **not exercised** | Pure PSP stores **every** entry uncompressed, exactly like Pulse PSP; `verify` reports 0 compressed streams. Pure adds no validation to the LZSS page | - |
| [PSP `.mip` texture](psp-texture.md) | reads | **346** entries decode and write as PNG (182 `Data`, 143 `FEData`, 21 `FE`). Identification is by the header's own size arithmetic against the stored length, so a wrong reading would fail to identify rather than mis-decode. 5 of 346 carry the pre-swizzle flag, the same small minority as Pulse | 94 |
| [`.vex` file header](vex.md#structure) | reads | `16 + tree_len + texture_len == file size` exact on **171/171** files. The version word is **4** (156 files) or **3** (15), against Pulse's 6 | 94 |
| [`.vex` node tree](vex.md#structure) | reads | 23,677 nodes. The tree ends exactly at the declared tree length on **171/171** files, and the `u16` `child_count` sums to `node_count - 1` on **171/171** - the same exact invariant that settled the layout on Pulse, now holding on a third corpus | 94 |
| [`.vex` class IDs](#the-class-id-space-is-renumbered) | **breaks** | Renumbered wholesale. None of Pulse's constants appears in any Pure file | 94 |
| [`.vex` mesh batches](vex.md#vertex-format) | reads (v4) | **11,025/11,025** v4 mesh nodes walk: 20,832 batches, 2,116,976 vertices, same batch header, same `vertex_type` encoding, same per-batch `f32` scale. Every decoded vertex falls inside the batch's **own declared bounding box**, **20,832/20,832** | 94 |
| [`.vex` mesh batches](#version-3-shortens-the-batch-header) | **breaks** (v3) | 8 of 26 v3 mesh nodes walk under the Pulse header; a shifted-header hypothesis raises that to 23 of 26 | 65 |
| [`.vex` embedded textures](vex.md) | reads | `sum(clut_size + texel_size)` over `Texture` nodes equals the declared texture-block length on **156/156** v4 files, to the byte | 92 |
| [`.vex` embedded textures](#pures-model-textures-ship-pre-swizzled) | reads | Pure sets the pre-swizzle flag on model textures. `vex::textures` reads `+0x06` and unswizzles on **every version** since 2026-10-01 (version <= 4 only from 2026-08-12); version 4 keeps its base-level-only reading and a synthesised chain, and Pulse's 75 flagged version-6 nodes now unswizzle each authored level at its own stride | 88 |
| `WO Track` payload | reads | 16 nodes carry the magic; version `0x103` against Pulse's `0x105`; `encoded_len == payload length` exact on **16/16** under the documented layout, reserved block included. Reported separately, see [below](#reported-elsewhere) | 94 |
| [Collision geometry](collision.md#the-same-format-is-in-wipeout-pure) | reads | None of Pulse's five class IDs appears. All three that do are now named - `0x36b` floor, `0x36c` wall, `0x37f` reset - by the [class-name table index](#the-renumbering-is-a-table-index-and-both-executables-carry-the-table), with floor and wall independently corroborated by a facing statistic and reset by a ship respawning on Pure's own circuit | 94 |
| [Front-end XML](fexml.md) | reads | 291 XML entries across the three archives. **Zero** begin `<code`, so the name shortening is a Pulse-era addition and `--expand` is correctly a no-op on Pure | 94 |
| [Handling stats](handling-stats.md) | reads | Addressing holds and the schema is enumerated in full. `<pitch>` became **optional** on 2026-08-12 - Pure authors none in any `<Class>` - and `oag_gameplay::handling::PITCH_STAND_IN` fills it, reported by name. See [below](#handling-stats-the-schema-holds-the-parser-does-not) | 94 |
| [Bitmap font](fnt.md) | reads | 6 fonts. Version `1` + `"FNT"`, codepoint table at `0x30`, offset table at `0x30 + 2*count`, and `atlas + 0x40 + clut_size + texel_size == file length` exact on **6/6**, with `texel_size == width * height / 2`. The atlas flag bit is set on 6/6, same as Pulse | 94 |
| [PSP movie](pmf.md) | reads | 14 movies. `stream_offset == 0x800` and `stream_offset + stream_size == file length` exact on **14/14**; two streams each, ids `0xe0` and `0xbd`. Version string `"0012"` only, where Pulse ships both `"0012"` and `"0014"` | 94 |
| [PSP sound bank](psp-audio.md) | **understood, and playing** | **29 banks on each pressing**, all accepted by the unmodified reader. The earlier "absent" row was a scan for the magic at offset 0, where it sits at `0x18` - the same scan finds nothing on a Pulse disc either. Bank names, cue names and the six wired cues are all Pulse's, so a Pure race sounds with no code branch. 461 waveform spans, 0 of them non-PS-ADPCM | 94 |
| [Particle system (`.pob`)](#pures-particle-systems-decode-unchanged) | reads | 37 `SYSP` entries in `Data.wad` and 37 in `FE.wad`, against Pulse's 35 and 27. **Decodes unchanged**: three of the four effects a race loads by name resolve on Pure and play, emitter trees and all | 92 |

## The class-ID space is renumbered

`.vex` class IDs are **table indices, not a stable enum**: the exporter's
class table grew between the two titles and every ID after an insertion point
shifted. Pulse's `Transform` at `0x6e` is Pure's `0x6d`; Pulse's `Mesh` at
`0x125` is Pure's `0x11e`. None of Pulse's constants appears in any Pure file.

The first four mappings below were recovered by an exact property of the payload,
one class at a time. The collision classes were not reachable that way and came
from the exporter's own class-name ordering instead - see
[below](#the-renumbering-is-a-table-index-and-both-executables-carry-the-table):

| Node type | Pulse | Pure | How it was pinned | Confidence |
| --- | ---: | ---: | --- | ---: |
| `Transform` | `0x6e` | `0x6d` | 5,016 nodes: 606 with an empty payload (the identity), 4,410 of exactly 64 bytes, of which 4,365 have orthonormal rows 0-2 | 90 |
| `Mesh` | `0x125` | `0x11e` | 11,025 nodes whose payloads walk as batch lists, and whose 2.1 M decoded vertices all fall inside the batches' own declared boxes | 92 |
| `Texture` | `0x3c1` | `0x373` | `sum(clut_size + texel_size)` over these nodes equals the file's declared texture-block length on 156/156 files | 92 |
| `WO Track` | `0x3bb` | `0x36d` | 16 nodes, each carrying the `WOtd` magic at `+0x00` | 94 |
| `Floor Collision` | `0x3b9` | `0x36b` | Table index, plus 98.8 % of 23,286 downward casts hitting at median depth `0.00` | 94 |
| `Wall Collision` | `0x3ba` | `0x36c` | Table index, plus the exact complement: 0.5 % downward, 68 % lateral | 94 |
| `Reset Collision` | `0x3cd` | `0x37f` | Table index. The facing statistic could **not** settle this one - see [below](#the-renumbering-is-a-table-index-and-both-executables-carry-the-table) | 94 |
| `Mag Floor`, `Cage` | `0x3e6`, `0x3e7` | absent | Pure authors neither on any circuit | 90 |
| `Airbrake` | `0x3c5` | `0x377` | Table index, plus every one of 6 reachable ships names its pair `Airbrake_Left`/`Airbrake_Right`, each with one `Mesh` child - the shape `mesh.rs` already assumes | 90 |
| `LodGroup` | `0x2ee` | `0x2de` | Not in the run - a generic Maya class, like `Mesh`/`Transform`. Every one of 6 reachable ships names one node `lodGroup1` with exactly two `Transform` children, switching at 50.0 units - switched per frame by Pulse's rule, chosen rather than read here (`vex.md`, "implemented - the switch runs every frame") | 88 |
| `fogCube` | `0x3d3` | `0x385` | Not in the run either. 10 instances across 7/16 circuits, all 128 bytes (`fog::PAYLOAD_LEN` exact) and all `edge == 500.0` - the same figure Pulse's own shipped tracks carry | 94 |
| `Anim Transform` | `0x3c0` | `0x372` | Table index; 332 nodes across all 16 circuits, **332/332** parse under `anim_transform`'s own bounds-checked decoder | 94 |

**The consequence for the code is smaller than it looks.** `CLASS_MESH`,
`CLASS_TEXTURE`, `CLASS_TRANSFORM` in
[`oag-vex::vex`](../../crates/vex/src/vex.rs) and `CLASS_WO_TRACK` in
[`oag-render::track`](../../crates/render/src/track.rs) are the only places a
class ID is compared. Every other line of the geometry path is title-agnostic
already. Deliberately **not changed here**: an ADR-0009 probe records where a
parser overfits, it does not patch it, and the right shape for the fix (a
per-title table keyed off the file's own version word) is a design question,
not a constant swap.

What `oag-view --mesh` says today, on a Pure ship:

```
Error: Data\Ships\Feisar\Ship.vex decoded to no triangles
```

The node tree behind that message parsed perfectly: 39 nodes, tree closing to
the byte, `child_count` summing to 38. There were simply no nodes with class
`0x125` in it.

### The renumbering is a table index, and both executables carry the table

The four payload-property rows above were each recovered one class at a time, and
that method cannot reach a class whose payload has no distinguishing invariant -
which is why the collision classes sat undetermined at confidence 45, and why the
[facing statistic](collision.md) that later named floor and wall still could not
name the third.

**`PSP_GAME/SYSDIR/BOOT.BIN` holds the exporter's class names as one contiguous
run of NUL-terminated strings, and the run is identical on both titles** - string
for string, index for index, all 22 of them. At file offset `0x280e38` on
`pulse-psp-usa.chd` and `0x2463e0` on `pure-psp-usa.chd`:

```
Floor Collision, Wall Collision, WO Track, Start Position, Speedup Pad,
Weapon Pad, Engine Flare, Anim Transform, Dynamic Point Light,
Dynamic Shadow Occluder, ParticleSystem, Airbrake, Skycube, Quake, Trail,
section, gate, shadow, speaker, Reset Collision, Ship Collision Fx, wospot
```

Class IDs run **consecutively along that order, with three gaps**. The gaps are
not free parameters: each is forced by the next already-recovered Pulse constant
after it, and there is nothing left to adjust afterwards.

- **13 of Pulse's 15 independently recovered constants land exactly**, with the
  mapping fixed by the other two.
- The ID skipped at the first gap - after `Anim Transform` - is
  `CLASS_TEXTURE`, `0x3c1`: a class the numbering contains and this table does not
  name.

Anchor it on Pure's `WO Track` at `0x36d`, measured on 16 of 16 tracks by the
`WOtd` magic and long before this table was found, and every Pure ID follows:

| Index | Class | Pure ID | Independently confirmed by |
| ---: | --- | ---: | --- |
| 0 | `Floor Collision` | `0x36b` | 98.8 % of 23,286 downward casts, median depth `0.00`, and the facing statistic |
| 1 | `Wall Collision` | `0x36c` | 0.5 % downward, 68 % lateral - the floor's complement |
| 2 | `WO Track` | `0x36d` | **the anchor**: `WOtd` magic, 16/16 |
| 3 | `Start Position` | `0x36e` | one node per circuit, decoding to a unit forward axis |
| 4 | `Speedup Pad` | `0x36f` | 14 nodes on Vineta K, and 14 trigger volumes off a *separate* decode path |
| 5 | `Weapon Pad` | `0x370` | 11 nodes, 11 volumes, same cross-check |
| 6 | `Engine Flare` | `0x371` | exactly one locator on `Feisar\Ship.vex`, centred on `x`, at the tail |
| 7 | `Anim Transform` | `0x372` | - |
| *(gap)* | `Texture` | `0x373` | **`sum(clut_size + texel_size)` closing on 156/156 files** |
| 12 | `Skycube` | `0x378` | 477 triangles of sky on Vineta K |
| 15 | `section` | `0x37b` | **1,335 of 1,335 draw calls governed by an authored section** |
| 19 | `Reset Collision` | `0x37f` | a ship flown into it respawns, on Pure's own circuit |
| 20 | `Ship Collision Fx` | `0x382` | 4 locators on `Feisar\Ship.vex` |

Two of those are worth separating from the rest, because they are what makes this
a determination rather than a consistent story. **`Texture` at `0x373` was
measured off the assets earlier, by an invariant with no connection to any string
table**, and the run predicts it as the ID its first gap skips - so the gap is not
a fudge, it is a prediction that came true. And **`section` at `0x37b` places
100.0 % of a circuit's draw calls in an authored visibility section**; a wrong ID
cannot produce 100 %, it produces zero.

#### This overturns the `Cage` reading of `0x37f`

The facing pass concluded `0x37f` matched none of Pulse's four measurable
signatures and that `Cage` was the obvious guess. The table says `Reset
Collision`, and the two are reconcilable once Pure's geometry is measured rather
than Pulse's assumed: `0x37f` sits under **99.9 %** of the spline at a consistent
~10.7 units below the road, where Pulse's `Reset` covers **5.8 %** at ~21.5 and
only where a craft can leave. A continuous under-road surface is what produces
both the huge area per object and the mixed facing that refuted every Pulse
`Reset` signature - so the discriminator was reading a different *authoring
style* for the same class, not a different class.

`reset_zone_ground_truth::pures_reset_surface_respawns_the_ship_too` closes it
behaviourally: a craft flown through that surface on Pure's own circuit respawns
on the spline.

Confidence **94**: an exact ordering shared by two executables, fitted against 15
constants recovered independently of it, and confirmed on Pure's own assets by
seven separate properties of four different node types. The cap is the rubric's
for anything not watched executing under an emulator.

#### Pulse's side of this was already disassembled, and it agrees

The string run above was found by searching the file. **Pulse's `id -> name`
table had already been read properly** - `vex::CLASS_NAMES`, stride 12
`{u32 id, char *name, ptr}` at `0x08ab2370`, names at `0x08a84d40`, confidence 95
(`docs/ghidra/functions/psp-pulse-usa/exhaust.md`). The two are the same bytes:
this run's runtime address works out to `0x08a84db8`, which is `0x78` into that
names region - the space the nine generic Maya class names ahead of
`Floor Collision` occupy.

That makes the disassembled table an **independent check on the gap structure**,
and it holds exactly:

| | Pulse id, disassembled | index in the run |
| --- | ---: | ---: |
| `Floor Collision` | `0x3b9` | 0 |
| `Anim Transform` | `0x3c0` | 7 |
| `Texture` | `0x3c1` | *the first gap - named by the table, absent from the run* |
| `Dynamic Point Light` | `0x3c2` | 8, i.e. shifted by one |
| `Reset Collision` | `0x3cd` | 19, still shifted by one |

`0x3b9 + 19 + 1 = 0x3cd` is Pulse's own `Reset Collision`, so the arithmetic that
puts Pure's reset at `0x37f` is the arithmetic that reproduces Pulse's at `0x3cd`.

**Two honest limits.** The anchors are not established as independent of that
read. `exhaust.md` records that **ten IDs of that ~55-entry table** were already
in `vex.rs` from unrelated earlier passes and every one agreed - but it names only
two of them (`Mag Floor Collision 0x3e6`, `Cage Collision 0x3e7`), and *neither is
one of the fifteen anchors used here*. **How many of the ten fall inside these
fifteen is not established**, so this section does not claim a count. What it does
claim is weaker and checkable: the fifteen constants were in the file before this
string run was searched for, and the run reproduces all fifteen.

And **Pure's own table has not been read at all** - only its string run - which is
why this section exists rather than simply citing a second `CLASS_NAMES`.

**What this still does not establish** is *why* the numbering shifted, or what
occupies the IDs the run does not name. **No Ghidra name was recovered here and
none was written to `docs/ghidra/functions/psp-pulse-usa/names.tsv`**; the
`0x08a84db8` arithmetic is unverified in Ghidra. Reading Pure's registration
table the way Pulse's was read is what would take this to 100.

**Settled 2026-09-03.** `Airbrake` is in the run at index 11, so the same
arithmetic gives `0x377`, and `Anim Transform` at index 8 gives `0x372` - both
checked against Pure's own files now (six ships and all 16 circuits
respectively; see the table above), and both in
[`vex::classes::V4`](../../crates/vex/src/vex.rs). `crates/mesh/src/mesh.rs`
already reads `classes.airbrake` to animate a ship's flap geometry, so this is
not bookkeeping: before it, a Pure ship's flaps rendered in their base pose
with no deflection at all, on every team, because the field it needed was
`None`. `lod_group` (`0x2de`) and `fogcube` (`0x385`) are recovered too, the
same way `mesh`/`transform` were - a property of their own node shape, since
neither is a name in this run.

**Airbrake flaps swing** (2026-10-05, `airbrake-flaps`). Checked with frames, not trusted from this page: holding left then right on Pure (EU) raises the matching flap.
The swing is the same `Flap::deflect` Pulse uses (`hinge * Rx(angle) * hinge^-1`,
about the hinge frame's local X), scaled by the title's own `<AirbrakeGraphics>`
`amount` and rates through `RaceView::airbrake_flaps`. **Which way it turns is
checked, not read:** the title's own `Airbrake` handler is unread, so
`airbrake_flaps_ground_truth (Pulse's)` asserts the physical claim Pulse's recovered axis makes (a positive
deflection raises the flap and flares it outward, both sides) on Pure by eye only - no Pure disc test was added. Only
the player's craft swings, as on Pulse; a rival's flaps stay stowed.

### The geometry is there and it is sane

Decoding the same file through the *same* rules, only with `0x11e` as the mesh
class, gives a hull the right size and the right shape - symmetric about `x`,
long in `z`, flat in `y`, in the same world units:

| Model | Batches | Vertices | AABB (x, y, z) | Radius |
| --- | ---: | ---: | --- | ---: |
| Pulse `Feisar\Ship.vex` (renders today) | 18 | 1,354 | `-2.67..2.67`, `-1.18..1.27`, `-6.36..6.54` | 6.66 |
| Pure `Feisar\Ship.vex` (refused today) | 10 | 839 | `-2.75..2.75`, `-1.24..1.25`, `-6.99..6.98` | 7.31 |

A wrong stride, a wrong scale field or a wrong position offset does not produce
a symmetric box within a few percent of the other title's ship. Confidence
**92** that the PSP vertex path is correct on Pure, from the box agreement plus
the 20,832/20,832 per-batch containment check.

### Version 3 shortens the batch header

15 of Pure's 171 `.vex` files are version 3 rather than 4, and they hold 26 of
its 11,051 mesh nodes. They use the **same class-ID space** as version 4, so
this is a payload difference, not a table difference. Under Pulse's batch header
only 8 of the 26 walk.

The hypothesis that fits the rest: version 3 has **no `alternate` pair**. The
`u16` at `+0x06` (alternate vertex count) and the one at `+0x0e` (alternate
offset) are absent, so `primitive_type` sits at `+0x06`, `vertex_type` at
`+0x08`, `payload_size` at `+0x0a`, the `f32` scale at `+0x0c`, and the two
`s16` bounding-box corners at `+0x10` and `+0x18`. Header size stays `0x40`.

Under that reading 23 of 26 nodes walk, with `payload_size` equal to
`vertex_count * stride` rounded up to 16 on every batch. Three nodes still read
a nonsense `vertex_type`. Confidence **65**: it is a consistent reading over a
small sample with a residual, which is a hypothesis, not a determination - and
it covers 0.2 % of Pure's geometry, so nothing depends on it.

### Pure's model textures ship pre-swizzled

> **Corrected 2026-08-09: the sentence about Pulse below is wrong.** A
> corpus-wide sweep found bit 0 set on **88 of 5,375** `Texture` nodes on the
> Pulse PSP pressing and **120 of 8,972** on the PS2 one, and the nodes carrying
> it are *not* font atlases - they are ship liveries, glass, engine and
> environment maps across EGX, Feisar, Goteki, Piranha, Triakis and Zone, plus
> the mine, bomb and shuriken effects. See
> `crates/vex/tests/texture_swizzle_flag_ground_truth.rs`, which measures
> and pins the distribution.
>
> **Settled 2026-10-01: bit 0 means "already swizzled" on Pulse too, and
> `vex::textures` now reads it on every version.** A GE dump of Pulse's shield
> shell (flags `0xe5`) shows the GE reading every texture swizzled, the shell's
> bytes in RAM equal to the file's at all four levels, and an unflagged hull
> texture's bytes in RAM reordered by the loader. A corpus check (neighbour
> difference of the palette luminance, linear against unswizzled) finds every
> non-degenerate flagged texture smoother unswizzled (56 of 56, ratios 0.27 to
> 0.85; the 8 others are 32-wide or flat textures whose swizzle is the identity)
> and 4,111 unflagged ones smoother linear against 333 (a handful of them markedly smoother unswizzled, ratios 0.64 to 0.8 - repeating wall and grid textures where a block reordering can look smooth - and they are not flagged, so nothing here touches them).
> The affected Pulse PSP nodes (version 6) are the shield shells, `Pulse_Bomb.vex`'s
> three textures, the shuriken, cage and mag-effect textures and seven ship-shaped
> models whose use is unidentified (not `ship_FE.vex`, whose menu render is
> pixel-identical before and after, and not race hulls, which are `0xe4`). Version-4
> nodes keep exactly their old reading. The paragraph below is kept as the history.
>
> The consequence is the part that matters: **making `vex::textures` read
> `+0x06` is not free for Pulse.** It changes what 88 PSP nodes decode to, and a
> ground-truth screenshot of a circuit need not cover the one model that
> changed, so `just test-data` would stay green over the regression. Either read
> it gated on version word <= 4, or settle what bit 0 means first.
>
> What the sweep cannot say is *which* of the two claims below breaks. A
> histogram distinguishes "the Pulse claim is wrong" from "bit 0 is not the bit
> this claim means" not at all. Decoding one flagged Pulse texture both ways and
> looking at the result would. That is unstarted.

The `Texture` node payload has a flags byte at `+0x06` whose bit 0 means *the
texels are already in the GE's 16-byte by 8-row block order*. On Pulse this bit
is set only on [font atlases](fnt.md#the-texels-are-stored-already-swizzled).

On Pure it is set on model textures too: the Feisar ship's five embedded
textures read flags `0x61`, where Pulse's eight read `0xe4`.

**Implemented 2026-08-12, gated on the version word; the gate was removed 2026-10-01 (see above).**
[`vex::textures`](../../crates/vex/src/vex.rs) now reads `+0x06` and
unswizzles through `texture::unswizzle` when bit 0 is set **and the file's
version is 4 or below**. That is the generation where the evidence is
unambiguous, and the gate is what keeps the correction above from becoming a
silent Pulse regression: 88 version-6 PSP nodes carry the bit too, no
ground-truth screenshot covers the models they belong to, and whether they are
really swizzled is still open. Version 6 is therefore left reading as it did.

The effect is visible rather than inferred. Before the change, a Pure race drew
its circuit with scrambled texels on every surface; after it, `01_Vineta_K`
renders its chevron start line, glass tunnel and hex-panel walls cleanly. Pulse's
own first race frame is **byte-identical** across the change, which is the
regression check the gate exists for. Unswizzling by the block rule the font path already
implements turns the ship's environment map back into a clean radial highlight,
which is what confirms both the flag's meaning and that nothing else is wrong
with the block. Confidence **88** as written; the Pulse half of it is now
contradicted by measurement and should be read as **open**, not as 88.

## Pure's particle systems decode unchanged

**Measured 2026-08-12, and this row used to read "unknown, both".** Pointing a
Pure race at `oag_fx::psys::Library` - which loads any `Data\Psys\<name>.POB`
by name - decodes three of the four effects the race path asks for, with their
whole emitter trees, and plays them. Sparks fly off a Pure hull on contact with
no change to the decoder at all.

That makes Pure a **third corpus** for `.pob`, and the strongest evidence yet
that the effect format is a pipeline property rather than a release one.
Confidence **92**: three effects decoding to sensible emitter trees on a disc the
decoder was never written against, though nothing here has been checked against
the original running.

| effect | Pulse | Pure |
| --- | --- | --- |
| `WO_SHIP_COLL_SPARK_DAMAGE` | 4 emitters | 4 emitters, **same four names** |
| `WO_ROCKET_EXPLO` | 7 emitters | 5 emitters |
| `WO_ROCKET_FLARE` | 2 emitters | 1 emitter |
| `WO_ROCKET_EXPLO_TRACK` | 4 emitters | **absent, no name resolves** |
| `WO_SHIP_ENGINEFLARE` | **no entry** | **no entry** |

Three things in that table are worth separating, because they are three different
kinds of fact:

- **The format carries over.** Every effect present decodes; none needed a
  version branch, unlike `.vex`, whose entire class-ID space renumbered.
- **The content does not.** `WO_ROCKET_EXPLO` is 7 emitters on Pulse and 5 on
  Pure, and `WO_ROCKET_FLARE` 2 against 1 - so a shared *name* is not a shared
  *asset* here, which is the opposite of what the dev/pub reel turned out to be.
  The collision spark is the one that matches emitter-for-emitter by name.
- **`WO_SHIP_ENGINEFLARE` is absent from both discs**, so it is not a Pure gap at
  all: that name resolves on neither pressing and the engine already reports it
  and draws nothing. `WO_ROCKET_EXPLO_TRACK` is genuinely Pure-shaped - present
  on Pulse, absent on Pure - and Pure ships no such effect under any name
  rather than spelling it differently. Confidence **78** for the absence
  itself: `strings` on Pure's `BOOT.BIN`, both the USA and EU pressings, lists
  `WO_ROCKET_EXPLO` and `WO_ROCKET_FLARE` as literal `Data\Psys\...POB` paths
  and every sibling explosion effect (`WO_MISSILE_EXPLO`, `WO_DISRUPTOR_EXPLO`,
  `WO_MINE_EXPLO`, ...) the same way, but never a track-hit variant; six
  plausible alternate spellings all miss against Pure USA's `Data.wad`
  directory by hash.

**What Pure's rocket actually draws on a track hit, read directly rather than
guessed at: `WO_TRACK_ROCK_DEBRIS`, not `WO_ROCKET_EXPLO_TRACK` and not
`WO_ROCKET_EXPLO` either.** `Rocket_Update` (`0x0885f28c`,
[`rocket-and-collision-fx.md`](../ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md))
spawns it from both of its detonation branches, under two different internal
tags (`RODB`, `ROD2`) but the identical resource name, read straight out of
memory rather than inferred from the hashed constant. It parses cleanly
through the unmodified `.pob` decoder - 3 emitters, a root burst of 8
additive streaks plus a 16-particle billboard root whose particles each spawn
a fading `trail` child - which also identifies one of the two extra `SYSP`
entries the row above counts (37 against Pulse's 35): confirmed present in
Pure's `Data.wad` and confirmed **absent** from Pulse's own PSP `Data.wad` by
the same hash, so this is not the same name silently already covered by the
count elsewhere.

This means `docs/formats/pob.md`'s existing table - which lists
`WO_TRACK_ROCK_DEBRIS` as one of nine names present only in Pulse's PS2
archive - is still correct as a Pulse-vs-Pulse (PSP vs PS2) statement, but
does not generalise across titles: the name also ships in Pure's PSP corpus,
under a use this reading now names. Confidence **80** for the trigger read,
**92** for the decode (an unmodified parser, unmodified codepath, matching the
confidence already established for this section) - see
[`rocket-and-collision-fx.md`](../ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md)
for the full reasoning, including why the score stays short of runtime-verified.

**Two of the three effects' Pure triggers are now read, not borrowed.** The
same page reads `Rocket_Init` (fires `WO_ROCKET_FLARE`, tag `ROFL`),
`Rocket_SpawnCraftExplosion` (fires `WO_ROCKET_EXPLO`, tag `ROEX`) and
`ShipCollisionFx_Trigger` (fires `WO_SHIP_COLL_SPARK_DAMAGE`/`_NODAMAGE`/
`WO_WEAPON_ABSORB` by the same `kind`/`damaged` branching Pulse's function of
the same name uses) directly in Pure's own executable, all via the string-
and `jal`-relocation workaround `psp-pure-eu/string-anchors.md` established.
What remains unread is *whoever calls* these four functions - the `jal`
wart breaks call-graph lookups the same way it breaks the string lookups, so
each stays a name-and-tag match rather than a fully call-graph-verified one.

What is **not** established is any *trigger* on Pure. These play because the
engine fires them from its own recovered Pulse triggers; nothing has read Pure's
executable for where it fires its own. The do-not-invent rule in `CLAUDE.md`
applies here exactly as it does on Pulse.

## Name recovery transfers intact

The [name hash](wad.md#the-name-hash) is unchanged between titles - no salt, no
seed, no per-archive variation. Pulse-shaped paths hit Pure's directories
directly:

```sh
just wad cat <pure>:PSP_GAME/USRDIR/Data.wad 'Data\Ships\Feisar\handlingstats.xml'
just wad cat <pure>:PSP_GAME/USRDIR/Data.wad 'Data\Plugins\PI001\Definition.xml'
```

So does the whole four-source mining method that
[`scripts/mine-names.py`](../../scripts/mine-names.py) implements. Pure's own
executable carries the same `%s\...` path templates - `%s\handlingstats.xml`,
`%s\Ship.vex`, `%s\track.vex`, `%s\TrackStartup.xml`, `%s\Definition.xml`,
`%s\stringtable.xml` - and `Data\Plugins\PI001\Definition.xml` lists every track
and ship directory by `location` attribute, exactly as Pulse's does.

A scratch miner built on Pure's own templates and 1,278 candidates resolves:

| Archive | Entries | Named |
| --- | ---: | ---: |
| `Data.wad` | 832 | 305 |
| `FE.wad` | 157 | 72 |
| `FEData.wad` | 240 | 97 |

That is roughly 40 % coverage from an afternoon's mining, against a script that
took considerably longer to reach its Pulse coverage. **The repository script
was deliberately not modified**: it hard-codes Pulse's team list, plugin list
and output path, and forking it is work for a real Pure effort, not for a probe.

### Pure has four per-craft models Pulse has no name for, and no boost plume

**Added 2026-09-06**, and it is the reason the Pulse-shaped probe above stopped
where it did. Cross-checking Pulse's five ship-model names against Pure found
one - `Ship.vex` - and that miss rate was read for three weeks as "Pure's other
models are spelled differently and unrecovered". Half of that was right. Pure's
executable holds **five** per-craft templates in one contiguous block at
`0x08a7a5d4`..`0x08a7a61c`, inside `Ship.cpp`'s own string run, and four of them
have no Pulse counterpart at all:

| Template | Address | Resolves on |
| --- | --- | --- |
| `%s\Ship.vex` | `0x08a7a5f4` | all eleven `<PI_Team>` locations |
| `%s\Shipwreck.vex` | `0x08a7a61c` | all eleven |
| `%s\VR\Ship.vex` | `0x08a7a5d4` | ten - not `Zone_01` |
| `%s\Phantom.vex` | `0x08a7a5e4` | the eight core racing teams only |
| `%s\Phantom_shipwreck.vex` | `0x08a7a600` | the same eight |

Confidence **96**: the templates are read byte for byte with `read_memory`,
every composed name hash-resolves on both pressings, and each blob carries its
*own* exporter source path back (`Z:/Data/Ships/Feisar/Phantom.mb` inside
`Data\Ships\Feisar\Phantom.vex`), so the archive is corroborated by its payload
rather than by itself. Pinned by
[`crates/pure/tests/ship_models_ground_truth.rs`](../../crates/pure/tests/ship_models_ground_truth.rs).

**`Phantom` is the top speed-class rung getting its own model**, which **Pulse**
does not do - its recovered template set is mode-keyed, not class-keyed. **HD
and 2048 were not checked**, so that is a two-title comparison rather than a
lineage claim. See the ladder below and `Unlock Phantom Class` (`0x08a7c848`).
What *selects* it is unread. `VR` names a whole alternate
presentation set across the disc (`vr_bomb.vex`, `vr_env.tga`,
`vr_engine_noise.mip`); what mode uses it is also unread.

**Pure ships no boost-plume asset at all**, which retires the last entry name
this page listed as unrecovered. Pulse composes its plume from `%s\%sboost.vex`
(`0x08a84ccc` on `psp-pulse-usa`); Pure's whole 3.6 MiB executable holds two
strings matching `boost` case-insensitively and neither is a path, no declared
location answers to `shipboost.vex` on either pressing, and no Pure hull carries
the `boost_flare` anchor nodes Pulse's do. Confidence **93**. Full evidence on
[ship-models.md](../ghidra/functions/psp-pure-usa/ship-models.md).

**Not visually inert, though.** Pure's own executable carries an
`Exhaust_Update` and `Exhaust_UpdateEngineSound` pair structurally and
literally identical to Pulse's (confidence 88, decompiled in full on both
pressings) - same `half_size = ((i * 0.6 + 0.4) * 2.5 + boost_timer * 8.0)`
term, same `0.8s` arm on a speed pad. So the always-on `Engine Flare`
billboard still grows and re-randomises on boost; it is only the separate
`<Team>boost.vex` plume mesh that never existed. `oag_fx::exhaust::Exhaust`
already implements this generically and unconditionally, so nothing needed
changing in this engine - see
[exhaust-sound.md](../ghidra/functions/psp-pure-usa/exhaust-sound.md)'s
"The visual half" section for the addresses on both pressings.

`oag_title::race::RaceDefaults::boost` is the seat this axis now lives in:
`Some("shipboost")` on Pulse, `None` on Pure (measured absent, as above),
`None` on HD (its plume is a subtree of `engineflare.vex`, not a standalone
model - see [trail-ribbon.md](../rendering/trail-ribbon.md)), and `None` on
2048/Omega (unread). `oag_raceplay::assets::boost_entry_name` returns
`Option<String>` accordingly, and the load report now says "names no
standalone boost-plume model" for Pure rather than the misleading "not in the
archive set" it used to.

## Handling stats: the schema holds, the parser does not

`Data\Ships\<Team>\handlingstats.xml` exists on Pure at the same addresses, in
plain (unshortened) XML, with the same element names, the same attribute names
and the same `<Stats team>` / `<Class name>` nesting.

**Eleven ship directories carry one, against Pulse's eight** - not nine, which
is what this section said until the roster was read off the disc rather than
recalled. Pure's own `Data\Plugins\PI001\Definition.xml` declares eleven
`<PI_Team>` entries and all eleven files resolve out of `Data.wad`; `FE.wad` and
`FEData.wad` carry none of them. Eight are the racing teams, and the other three
are `Zone`, `Medievil` and `Zone_01`, the last of which is not a team at all -
see [below](#one-pure-file-has-no-class-blocks-at-all).

### `Van_Uber` is Pure content, not a Pure *team* on this disc

**Confidence: 94.** `Van_Uber` is not among the eleven `<PI_Team>` entries
above, and `Data\Ships\Van_Uber\Ship.vex` does not resolve by name-hash
against either Pure disc's `Data.wad` (hash `3a30c448`, checked on both USA
and EU) - the same absence `dlc-pack.md` and `exhaust.md` have recorded since
2026-08-09, just without checking it against this page's own roster until now.

That does not make it a Pulse or Fusion name adrift in this project's docs:
`data/dlc/` holds seven Wipeout Pure PSN packs, unlike Pulse's four - `A7`,
`Delta Pack`, `Gamma Pack 1`, `GamesRadar Pack`, `Oblivion`, `Omega Pack`,
`Voice of Cod`. **Confirmed, not just likely**: the Gamma pack's `pi.wad`
decrypts (see
[dlc-pack.md](dlc-pack.md#pures-packs-decrypt-with-an-external-key-table) for
the algorithm and key table) and its manifest declares
`<PI_Team name="Vanuber" ... location="Data\Ships\Vanuber">` - no underscore,
the id `oag_livery::entry::ship_entry_name` actually composes a path from, unlike the
underscored `Van_Uber` spelling every external source and this project's own
earlier name-hash checks used. `Data\Ships\Vanuber\Ship.vex` (`9e62d495`) and
`\handlingstats.xml` (`7e135ec1`) both hash-match real entries in the
decrypted pack, so the team is fully simulatable, the same as Pulse's four DLC
teams.

Recorded from
[`crates/pure/tests/handling_schema_ground_truth.rs`](../../crates/pure/tests/handling_schema_ground_truth.rs),
which walks every shipped file on both discs, folds the element and attribute
*names* into two sets and asserts the entire symmetric difference at once. Names
only: values stay on the player's disc, per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md).

What Pulse authors and Pure does not - **this list is complete**:

| Element | Absent from Pure | Consequence |
| --- | --- | --- |
| `<Stats><FE speed thrust handling shield/>` | the whole element | Presentation, the ship-select bars. `Stats::fe` is an `Option` |
| `<Class><pitch pitch_air pitch_ground pitch_damping antigrav_height_adjust/>` | the whole element | Feeds `oag_physics::Pitch`. **Was the live blocker; optional since 2026-08-12.** `Class::pitch` is an `Option` and `oag_gameplay::handling::PITCH_STAND_IN` substitutes Pulse's own block, which `race::load` reports by name. A malformed one is still an error - see `an_absent_pitch_is_none_and_a_broken_one_is_still_an_error` |
| `<Class><Airbrake sideshift>` | one attribute | Optional; `oag_physics::Airbrake` already defaults it to `0.0` |
| `<Misc easyshield>`, `<Misc weight_distribution>` | two attributes | Optional, same defaulting argument |

Everything else matches exactly: the five camera elements, `<AirbrakeGraphics>`,
and the `<Engine>`, `<Brakes>`, `<Turning>`, `<Antigrav>` and `<Physical>` blocks
inside a `<Class>` are attribute-for-attribute identical. Pure's ladder is five
rungs - `VECTOR` below Pulse's four - on all ten files that have a ladder at all.

Every difference is a Pulse-era **addition**, which is the useful direction: the
handling model grew between the titles rather than changing shape. Confidence
**94**: an exact set comparison over 19 shipped files on two discs, run by a
test rather than read by eye.

**Pure's EU pressing carries the identical schema** - same eleven directories,
same element and attribute sets, asserted by
`both_pure_pressings_carry_the_identical_schema`. That is what makes the claims
here about the title rather than about `pure-psp-usa.chd`.

### One Pure file has no `<Class>` blocks at all

`Data\Ships\Zone_01\handlingstats.xml` - `<Stats team="ZoneMode">` - authors
`<Engine>`, `<Brakes>`, `<Turning>`, `<Airbrake>`, `<Antigrav>` and `<Physical>`
**directly on `<Stats>`**, with no speed-class ladder over them. Its `<Airbrake>`
has no `slidegrip` and its `<Antigrav>` no `landing_rebound` or
`rebound_jump_time`, which are exactly the attributes a per-class block carries
and a per-team one does not.

This is a difference in *shape*, and it is the one no survey of Pulse could have
produced: there is nothing on Pulse's side to notice missing.

**The decoder accepts this file and silently drops all six blocks.**
`handling::parse` iterates `<Class>` children, finds none, and returns
`Ok(Stats { classes: [] })` - a document with a whole parameter set in it reads
as a document with none. That is the same shape that bit
`collision::from_vex`, where "nothing matched" and "I could not read the file"
became the same empty result. Nothing has yet decided
what a classless ladder *means*, so nothing is changed here; the behaviour is
pinned by `pures_zone_mode_file_authors_its_blocks_outside_any_class` so the
decision has to be recorded when someone makes it. Confidence **94** on the
observation, none offered on the interpretation.

### The second file needed no work at all

`Data\XML\HandlingStats.xml` - the engine-wide `<Global>` block, a different
caller and a different file that shares the same parser - **is on Pure and
parses today, unchanged.** Nothing predicted that either; it was simply never
opened.

Two differences, neither of which the decoder reads:

| Element | Pulse | Pure |
| --- | --- | --- |
| `<Global><StartBoost/>` | present, 7 attributes | absent |
| `<GlobalClass><WeaponPad>` | `refresh_time`, `elimination_refresh_time` | `refresh_time` only |

Pure has no elimination mode, so both are the same Pulse-era addition as every
difference above. Both discs author five `<GlobalClass>` blocks with `VECTOR`
first, which is the ordering property
[`handling::global_classes`'s skip depends on](handling-stats.md#there-are-five-globalclass-blocks-and-only-four-speed-classes).
Confidence **94**.

**The weapon table's own dialect is on its own page since 2026-09-15**:
[`weapon-stats.md`](weapon-stats.md)'s "The Pure dialect" section has the
ten-weapon roster against Pulse's thirteen, the `Disruptor`'s `<Effect>`
tree, and the fuse-less Bomb - all measured on both pressings and all
decoded, with the executable's reading of each on
`docs/ghidra/functions/psp-pure-usa/weapons.md`.

### Why this page could not have predicted any of it

The method lesson is about surveys, not about Pure. Every per-layer confidence
on this page came from **counting** - nodes walked, sizes closed, files
identified - and **a count cannot see an element that is simply not there.** The
88s and 92s were never wrong about what they measured; they could not measure
absence.

Absence needs a **comparison**, and the first four differences were found one at
a time by a parser panicking, each visible only once the one before it had been
handled. The set diff above is the form of the question that can answer "Pulse
has one of these and Pure does not" in a single pass, and it turned up two more
findings - the classless file and the global file - that no amount of pointing
the parser at Feisar would have reached.

## Tooling defects this probe surfaced

Neither is a format overfit; both were missing guards on paths that had only
ever seen Pulse data. The first is fixed, the second is expected behaviour.

- **`oag-view --collision` panicked on a file with no collision nodes. Fixed
  2026-08-04.** It correctly reported `0 collision node(s)` and printed an empty
  per-class table, then handed an empty vertex buffer to wgpu and died inside
  `buffer.rs`: `buffer slice can not be empty`. Any `.vex` without a
  recognised collision class reached this, which on Pure is every file. The
  backtrace put it at `wgpu::api::buffer::BufferSlice::size_expect_nonzero`
  under `set_vertex_buffer`, not at buffer creation - `create_buffer(size: 0)`
  and `write_buffer(&[])` both succeed, so clamping the buffer size would not
  have fixed it. Two changes: `mesh_render::capture_from` and `oag_view::orbit`
  now skip every draw command when the model has no geometry, so the pass emits
  the clear colour rather than panicking; and `oag-view --collision` bails with
  `nothing to draw - no collision geometry` before it gets there, because a
  screenshot of an empty frame is not what the command was asked for and a
  scripted caller wants the non-zero exit. Reproduce the former state with
  `oag-view <image>:<Data.wad> --collision 'n\Feisar\Ship.vex'`. Covered by
  `mesh_render::tests::an_empty_model_captures_a_frame_instead_of_panicking`,
  which probes for an adapter and skips without one rather than being
  `#[ignore]`d, so it runs in plain `just` wherever a GPU exists; the `orbit`
  guard needs a window and has none. One consequence worth knowing: `--mesh`
  on a model with no geometry now writes a blank PNG and exits 0 where it
  previously panicked, so a scripted caller gets a silent no-op rather than a
  crash. Only `--collision` refuses; the mesh path has no equivalent bail
  because a `.vex` that decodes to no geometry at all has not been seen.
- **`oag-trace drive --source <pure>` fails on a hard-coded Pulse track path.**
  Expected, and recorded only for completeness: the asset layer resolves by
  name against the source's own directory, so this is the scenario naming a
  Pulse track, not a portability problem.

## What a real Pure asset milestone would cost

Ordered by cost, from the measurements above:

1. **Browsing Pure's models and textures: about a day.** A per-title class-ID
   table selected by the file's version word, plus honouring the pre-swizzle
   flag in `vex::textures`. The empty-model guard this list also asked for is
   done, so it is no longer part of the estimate. Every other line of the
   geometry path already works, on 2.1 M vertices of evidence.
2. **Tracks: a day on top of that**, and mostly the same change - the `WO
   Track` payload already decodes exactly under one more class ID.
3. **Collision: done, all three, and it took hours rather than days.**
   The costing here said "needs the loader read out of Pure's own executable" -
   and that turned out to be one way rather than the only one, twice over. A
   facing statistic calibrated against Pulse named two of the three candidates at
   88 without opening Ghidra
   ([collision](collision.md#two-of-the-three-are-now-named-by-facing-rather-than-by-counting)),
   and the [class-name string run](#the-renumbering-is-a-table-index-and-both-executables-carry-the-table)
   then named all three at 94 - the executable as a *file* rather than as
   disassembly. Reading the registration function is still what would reach 100
   and say what the unnamed IDs are.
4. **Handling: done, and it was the cheapest item here as costed.** The fifth
   speed class and the four absent attributes were already handled; `<pitch>` is
   now optional with a labelled stand-in. What is *not* costed, and remains
   open, is deciding what a `<Stats>` with no `<Class>` ladder means, which is a
   design question rather than a parser one.
5. **Audio: a new format.** `SBlk` does not exist on Pure; whatever indexes its
   RIFF streams has not been looked at.

None of that is on the roadmap, and this page does not put it there. What the
probe establishes is the ADR-0009 premise: **the format layer is genuinely
multi-title, and the places it is not are countable.**

## Its front-end skin, and what it does not have

Read off `pure-psp-eu.chd`'s own
`Data\Plugins\PI001\GUI\Skin.xml`, which is plain `<?xml` - the finding above
that not one of Pure's 291 XML entries begins `<code`, holding at the front
end's own root file.

Pure states **six** layout globals where Pulse states dozens, and disagrees with
Pulse on every one they share:

| | Pulse | Pure |
| --- | --- | --- |
| `MenuXOffset` / `MenuScale` | 50 / 1.0 | 21 / 1.15 |
| `TitleXOffset` / `TitleYOffset` / `TitleScale` | 50 / 0 / 1.0 | 21 / 20 / 0.97 |
| `MSWarningScale` | 1.0 | 0.8 |

What it does **not** have matters as much, and is why `oag-pure`'s
`MenuSkin` is mostly `None` rather than filled in from Pulse:

- **No `TextColor` or `TitleColor` in this file.** Both are authored, in the
  *style* skin `Data\Plugins\PI001\Definition.xml` activates beside the UI one
  (`Data\Skins\Default\Skin.xml`, 41 globals, identical on both pressings) -
  see [race-setup.md](race-setup.md). They were pixel-sampled into
  `FALLBACK_GLOBALS` at confidence 65 until 2026-09-10, when the disc turned
  out to author them; that table is now empty. Pure's `TitleColor` is pink
  (`0xFFED4796`) where Pulse's is black, so borrowing would still have been
  wrong.
- **No `MainMenu_Definition.xml`.** Its `LoadXML` list names twelve other
  `*_Definition.xml` files instead. All twelve have now been read for row
  geometry: 19 of the 33 `<Menu>` widgets across them say `y="45"` and no other
  value reaches four, so `MenuSkin::first_row_y` is `Some(45.0)` at confidence
  80. Row *pitch* on `MenuSkin` itself is still `None` - one authored `gap` is
  not a measurement - but see the `Language Selection` pitch measured below,
  on a different screen than this bullet's own capture.
- **`background` and `selected`, filled in 2026-08-25.** A capture of `Main
  Menu` itself (`pure-psp-usa.chd`, PPSSPP 1.20.4) found two things this build
  drew wrong rather than merely incompletely: the screen is solid white, not
  the black this build cleared to for want of anything else, and the selected
  row is a *darker*, more saturated ink than the others - the opposite
  direction from Pulse's "brightening toward white". Both are measured, not
  authored: `FE Screen`'s own `BackgroundController->BackgroundImage` names no
  `src`, the same "engine carries a compiled-in default" gap as
  `TitleColor`/`DesignColor`/`TextColor`/`FrameLineColor`. See
  `oag_pure::frontend::MENU_SKIN`'s own `background`/`selected` field
  comments.
- **`Language Selection` has no highlight band at all, confirmed 2026-09-09.**
  A second capture, this screen rather than `Main Menu` (`pure-psp-usa.chd`,
  PPSSPP 1.20.4, 2x native under Xvfb, row 0 then row 1 selected in turn)
  finds no translucent `Fill` anywhere near the selected row - only a plain
  colour swap. Sampled at the glyph's own darkest pixel: selected-row ink is
  `rgb(21-22,173-174,208-209)`, pixel-for-pixel `MENU_SKIN.selected`
  (`0xFF16AED1`); unselected-row ink is `rgb(137-138,214,232)`, pixel-for-pixel
  `TextColor` (`0xFF88D6E8`). This is a second, independent confirmation of
  `selected` (first measured on `Main Menu`, above) and it settles what a
  1,142-line front-end draw module had been guessing at with an unmeasured
  translucent `Fill` since 2026-08-18: there was never a rect to position,
  because the real screen draws no such rect. `crate::frontend::draw::
  draw_language_selection` (`oag-game`) now swaps the selected row's ink to
  `MENU_SKIN.selected` and draws no `Fill`, gated on the title carrying a
  *static* measured `selected` (`selected_pulse_period_secs.is_none()`) so
  Pulse's own picker - whose `selected` pulses toward white on a clock this
  build does not yet drive here - keeps its prior, unverified-but-unbroken
  `Fill`-plus-row-colour behaviour untouched.

  The same capture shows a real selection cue this build still does not draw:
  a `6x11` pink (`0xFFED4796`, `Intro Screen->ArrowSelect`'s own
  `MenuHighLightArrowColor`) arrow image sitting to the selected row's left,
  `Data\FE\Images\FETextures.mip` at `U=53 V=0`. Left unwired: the widget's
  own definition carries `x="0" y="0"`, so its real per-row runtime placement
  is not authored anywhere this build reads and is not measured either - a
  guessed offset would be exactly the invented-stand-in this project's own
  rule against un-evidenced visuals exists to prevent.

  The same two captures also measured `Language Selection`'s own row pitch -
  a real two-row sample, which `first_row_y`'s own `MainMenu_Definition.xml`
  reading (above) never had. The selected row's ink top moves from native
  y=53 to y=71 between row 0 and row 1, and the arrow glyph's top moves from
  y=50.5 to y=68.5 - the same **18px**, independently, from two different
  features. `Language Selection`'s own `Menu` widget authors `y="46"
  font="Default" scale="FEGlobals->MenuScale"` (`MenuScale` = 1.15, the `Pure`
  column of the table above), so this build's `font_line_height("Default") *
  scale` comes out to `13.0 * 1.15 = 14.95`, about 17% short of the measured
  18. **Not corrected here**: `font_line_height`'s `13.0` fallback is shared
  by every unrecognised font role on both titles, and one screen's pitch
  cannot separate "the line height itself is wrong" from "a `gap` sits on top
  of a right one" from a single sample. Also unexplained: authored `y="46"`
  against measured ink-top `y="53"` (native), a 7px delta that reads as font
  ascent (baseline vs cap-height) rather than a second pitch error, but is not
  measured as one.
- **No `LeftLayer` element anywhere.** Its `transition` durations are authored
  (0, 0.25, 0.3, 0.5, against Pulse's 0, 0.2, 0.5, 0.7) but what they attach to
  is unread.
- **Eight font roles, the same number Pulse fills, differing by one.** An
  earlier reading here said "three roles" and was an artefact of reading one
  plugin through a truncating pager. Measured across every language plugin on
  both discs: both titles fill `Default`, `Small`, `Title`, `HUD`, `HUDSmall`,
  `InGame` and `Stats`, and the eighth is `Menu` on Pulse against `Scroll` on
  Pure. They share exactly one filename, `small.fnt`, and it is not the body
  face - so a build naming Pulse's files drew Pure's whole front end in the 5x7
  fallback. `oag_game::language::roles` carries the table;
  `crates/game/tests/font_roles_ground_truth.rs` re-derives it. Pure's menus
  draw in `Default`, which is why `MenuSkin::menu_font` stays `None`.

### The Title screen wordmark, `TitleFrame`

`Title Screen->TitleFrame` (`x="0" y="76" width="480" height="128"
TxtrWidth="480" TxtrHeight="128"`, directly under an XML comment reading
`<?This is the Title screen backdrop?>`) is the same class of gap
`BackgroundImage` is above: no `src` at all, assigned programmatically on the
original. Found by a full image-content scan rather than a name guess -
`Data\FE\Images\Logo.mip` (hash `be900df4`) does resolve to a real entry, but
at 2064 bytes it is far too small to be a 480x128 texture and is something
else entirely.

**The scan.** Every entry across all three PSP archives was extracted and
tried against `oag_texture::texture::Texture::parse` (`oag-wad extract --png`):

| Archive | Entries | Decoded as `.mip` |
| --- | ---: | ---: |
| `FE.wad` | 157 | 27 |
| `FEData.wad` | 240 | 143 |
| `Data.wad` | 832 | 191 |

Filtering the 361 decoded textures to width >= 200px leaves mostly `256x128`/
`256x256` track-select art and one `480x272` full-screen render (`498abb06`,
a track background, not a wordmark - checked by eye and ruled out). Exactly
three textures come out `512x128`: `Data.wad` entries 535 (`b6677aab`), 536
(`3f313472`, byte-identical to 535) and 537 (`3af18d90`) - all three absent
from `FE.wad` and `FEData.wad`, checked by hash. Decoded:

- 535/536: blue "wipEout" over orange "pure".
- **537: orange "wipEout" over a white-outlined "pure"** - an exact match to a
  real `Title Screen` capture (PPSSPP, `pure-psp-usa.chd`): orange "WipEout"
  over a white-outlined "pure", filling roughly the screen's own width.

**The corroborating measurement.** `TitleFrame`'s own `TxtrWidth="480"`
against entry 537's `512`-wide physical texture is not a coincidence - PSP
textures are routinely padded to a power of two, and the rightmost 32 columns
of this one are opaque white with no ink, so the widget's own authored sample
width crops exactly the padding. Trimming the decoded picture to its own
opaque content lands a `335x80` box entirely inside the `480`-wide crop.

Byte-identical on both pressings (`pure-psp-usa.chd`, `pure-psp-eu.chd`), same
hash, same 66,576-byte entry (512x128, 8bpp indexed: `512*128 + 256*4 palette
+ 16 header`). Recorded as `oag_pure::hashes::TITLE_LOGO`.

**2026-09-23: `entry 537 is the USA pressing's own value - the EU pressing draws entry 535 instead, a real bug, not a documentation gap.`**
The runtime mechanism this section's own confidence cap named as unread is
now read: `TitleScreen_AssignWordmarkTexture`
(`docs/ghidra/functions/psp-pure-eu/title-screen.md`) builds a texture name at
runtime by concatenating `Data\FE\Images\FMV_last_frame` with a literal,
**per-pressing** suffix baked into each disc's own executable - `_EU.mip` on
the EU binary, `_US.mip` on the USA one. `oag_formats::wad::hash_name` on each
full name lands exactly on this section's own two colourways: `_US.mip` hashes
to `3af18d90` (entry 537, the orange-over-white picture this section already
matched against a USA capture) and `_EU.mip` hashes to `b6677aab` (entry 535,
blue "wipEout" over orange "pure" - the *other* colourway this section
described but never assigned to a screen). A breakpoint on the EU function
fires on a real `pure-psp-eu.chd` boot under PPSSPP, and the settled `Title
Screen` on that same boot shows blue-over-orange - confirming the EU pressing
never drew entry 537 at all. `oag_pure::hashes::TITLE_LOGO_EU` records the EU
value; `oag_pure::frontend::title_frame_src` resolves the right one from the
disc's own serial (`oag_assets::Layout::serial`, added for exactly this),
consulted by `oag_game::boot::load_shell` rather than carried in the static
`FALLBACK_IMAGES` table `BackgroundTopRightImage` still uses, since this is
the one fallback whose own correct value is not the same on both pressings.
Confidence **90**, both regions - short of higher only because which field the
widget's own *draw* call reads from is not itself traced (the assignment is,
via a live breakpoint and a settled-frame capture on each pressing).
`crates/game/tests/pure_boot_ground_truth.rs`'s
`title_screens_own_wordmark_gets_the_measured_texture` now pins a **different**
expected hash per pressing rather than one shared value, which is what let the
bug through in the first place: both entries pass the rect-and-512x128 shape
check equally, so nothing before this asserted the right *content* per disc.

A third region variant sits on the disc unwired: entry 536 (`3f313472`,
byte-identical in decoded content to entry 535) hashes to
`Data\FE\Images\FMV_last_frame_JAP.mip`. No Japanese-region Pure disc is in
this project's corpus, so which pressing's own executable actually names that
suffix is unread, and nothing consumes it.

### The dev/pub reel and second boot movie's own region suffix

The same shape of bug as `TitleFrame`'s, on the same evidence class, and it
closed the same way. `oag_pure::names::INTRO_MOVIE_CUTS`/
`FMV_INTRO_MOVIE_CUTS` (2026-09-08's content scan) already had all four
regional cuts of both boot movies named and hash-verified against real
`Data.wad` entries, but the runtime mechanism that picks one per pressing was
unread - `Movie::entry_name` (`crates/ui/src/screen.rs`) appended a hardcoded
`_US` suffix to every `localised="true"` `<Movie>` widget regardless of which
disc was booted, so `pure-psp-eu.chd` showed the American dev/pub card
("SONY COMPUTER ENTERTAINMENT AMERICA PRESENTS") instead of its own European
one.

**2026-09-23: the mechanism is read, and it is `TitleScreen_AssignWordmarkTexture`'s
own shape.** `Movie_ParseAttributes`
(`docs/ghidra/functions/psp-pure-eu/movie-localised-suffix.md`) appends a
single literal suffix to a `localised="true"` widget's resolved name -
`"_EU"` on the EU binary, `"_US"` on the USA one, read directly off both
executables' own memory - and neither binary's string table names the other's
suffix, or `_JAP`/`_KO`, at all. There is no runtime language read, no region
enum: which cut plays is decided by which executable is running, exactly
`TitleFrame`'s own finding.

`oag_pure::frontend::localised_movie_region` resolves the pressing's own
region from its serial (`oag_assets::Layout::serial`, `None` or an
unmeasured serial defaulting to `"EU"`, the same convention
`title_frame_src` takes), and `Movie::entry_name` now takes that region as a
parameter instead of hardcoding one. `oag_game::boot::load_shell` resolves it
once per source and threads it through both the front-end report and the
two boot-movie names `oag_pure::frontend::BOOT_PROFILE.chain` declares -
`oag_pure::names::intro_movie`/`fmv_intro_movie` look the resolved region up
in the same `*_CUTS` tables the 2026-09-08 scan built.

**Verified two ways.** `crates/game/tests/pure_boot_ground_truth.rs`'s
`each_pressing_resolves_its_own_cut` loads both real discs and confirms each
one's own cut resolves rather than falling back; and a headless
`oag-game --reel --screenshot` capture of `pure-psp-eu.chd` at the dev/pub
reel's own frame 144 reads "SONY COMPUTER ENTERTAINMENT EUROPE PRESENTS",
not "AMERICA" - screenshot under
`~/.cache/oag/drive/reports/pure-movie-region/`. The two pressings' own
`WoFMVNew` cut also turned out **not** to share a frame count the way the
2026-08-10 measurement (taken off the `_US` entry loaded from both discs,
before this fix) assumed: the EU cut is 2901 frames against the USA cut's
2848, both 480x272.

Confidence **85** for the mechanism itself (unambiguous decompile, identical
structure cross-binary, hashes agree with already-verified entries) - short of
`TitleFrame`'s 90 because no live breakpoint confirms `Movie_ParseAttributes`
firing during a real boot, only the static read and the resulting screenshot.
The fix built on top of it is measured directly, both by the ground-truth
test and the screenshot.

`FE Screen->BackgroundImage`/`BackgroundTopRightImage` are the same shape of
gap. The scan below, run again and sized for these two, resolved one and ruled
out every candidate for the other - and 2026-09-23 read the mechanism behind
both, below.

### The dev/pub reel's hold duration is measured, not imported

The frame holds themselves - pause at frames 144 and 231, finish at 260 - were
already known to be Pure's own, from three `li` immediates unique in the
3.6 MiB binary (see [pure-boot.md](../architecture/pure-boot.md)). What was
open was the hold's *duration*: the code samples a global clock, and an
earlier reading of that read the clock sample as the duration itself, leaving
`oag_pulse::frontend::HOLD_SECONDS = 2.0` recorded as "imported" from Pulse's
own measurement rather than confirmed for Pure.

**It is confirmed, and it is `2.0`, identically on both pressings.** The
global the earlier reading flagged is a running "current time" the function
samples twice - once to record when a hold starts, again every frame after to
compute elapsed time - and neither read is the duration. The duration is what
elapsed time is compared *against*, and that comparison builds `2.0` as a raw
IEEE-754 bit pattern (`lui a0,0x4000` / `mtc1 a0,f14`) at the identical offset
in both binaries' `DevPubReel_UpdateFrameHolds` (`0x0894b1f0` EU / `0x0894b678`
USA) - the same three pause-frame immediates the handover thread already
found, now inside a fully read function rather than a partially read one. See
[`devpub-reel-hold.md`](../ghidra/functions/psp-pure-eu/devpub-reel-hold.md)
for the disassembly.

**Live-confirmed on top of the static read**, closing the thread's other open
question - whether the function is actually reached from the boot sequence,
previously "the only candidate ... inference rather than a traced call path".
A PPSSPP breakpoint on the frame-144 pause call fired 1.70s of wall clock
after confirming English on a true cold boot of `pure-psp-eu.chd`, consistent
with the disc reaching `Developer Publisher Screen` through the documented
chain; a second breakpoint on the matching resume call measured the actual
held span off the PSP's own emulated cycle counter, independent of wall-clock
jitter: **443,220,048 ticks at 222,000,000 Hz = 1.9965s**, against the
immediate's own `2.0` - the 0.0035s shortfall is under one 60Hz frame
(`1/60 = 0.0167s`).

No code changed: `oag_pulse::frontend::HOLD_SECONDS` was already the right
value for Pure, just unconfirmed. What changed is confidence and the doc
comments that called it "imported" - see `oag_pure::frontend::states::
DEVELOPER_PUBLISHER`'s own doc comment and `pure-boot.md`. Confidence **92**:
identical structure and constant across both pressings, plus two independent
live measurements (the call-path timing and the tick-counted span) agreeing
with the static read - short of higher only because the function this hold
logic lives inside was reached through a breakpoint rather than a full
call-graph trace back to a named screen-dispatch table, and because no JAP or
KO Pure disc exists in this project's corpus to check a third pressing bakes
the same constant.

### `FE Screen`'s corner logo, `BackgroundTopRightImage`

`FE Screen->BackgroundController` wraps two more `src`-less images, the same
class of gap `TitleFrame` is: `BackgroundImage` (`width="512" height="272"
TxtrWidth="480" TxtrHeight="272"`, no `x`/`y`, so a full-screen backdrop) and
`BackgroundTopRightImage` (`x="252" y="3" width="256" height="32" U="0" V="0"
TxtrWidth="256" TxtrHeight="32"`, the corner graphic). This is a player-
reported bug - "Pure's menu has no background" - not a documentation gap found
by inspection.

**The scan.** The same method `TitleFrame` used, run again against
`pure-psp-usa.chd`'s three archives and filtered for two different shapes
instead of `512x128`: a full-screen backdrop (`480x272`, or PSP power-of-two
padding of it - `512x256`, `512x512`) and the corner graphic's own `256x32`.
The 361-texture decode count is unchanged from the `TitleFrame` scan (same
archives, same disc); what differs is the filter:

| Shape | Candidates | Archive | Verdict |
| --- | --- | --- | --- |
| `480x272` | `498abb06` | `Data.wad`/`FEData.wad` | a track background render, checked by eye and ruled out (same entry `TitleFrame`'s own scan already found and ruled out) |
| `512x256` | `5c0ea854` | `Data.wad`/`FEData.wad` | blue/cyan speed-line art, no rule lines or menu chrome - a loading-screen candidate, not this one |
| `512x512` | `775fd47d` | `Data.wad`/`FEData.wad` | a white card bearing "A-G RACING", a barcode, `VS.0`/`//2197`, a world-map-style graphic and a memory-card outline, over a solid magenta lower half - **not** a match: none of that content appears anywhere on a real `Main Menu` capture |
| `256x32` | `09af556a` | `Data.wad`-only | a diagonal hazard-stripe pattern plus a small bracket - not a match |
| `256x32` | **`7ba78aca`** | `Data.wad`-only | **the "ワイプアウト" katakana wordmark beside the swoosh/arrow logo - an exact match** |

**`BackgroundTopRightImage` is resolved.** Entry 27 of `Data.wad`, hash
`7ba78aca`, `256x32` 8bpp indexed (`9,232` bytes: `256*32 + 256*4` palette
`+ 16` header, exactly - already a power of two on both axes, so unlike
`TitleFrame` there is no padding to crop). Absent from `FE.wad`/`FEData.wad`,
checked by hash. Byte-identical on both pressings (`pure-psp-usa.chd` entry
27, `pure-psp-eu.chd` entry 28 - same hash, different index). Cropping a real
`Main Menu` capture to the widget's own rect (`x=252 y=3 width=256 height=32`,
doubled for the `2x` capture) reproduces the texture exactly: same katakana
text, same swoosh logo, same layout, same cyan ink. Recorded as
`oag_pure::hashes::MENU_TOPRIGHT_LOGO`, wired through `FALLBACK_IMAGES`.

**2026-09-23: the runtime mechanism is read, and it is a declared global, not
a hash at all.** `BackgroundController_UpdateImages`
(`docs/ghidra/functions/psp-pure-eu/title-screen.md`) resolves
`BackgroundTopRightImage`'s texture every frame from a declared
`FEGlobals->BackgroundTopRightTexture` global - the same `FEGlobals->Name`
indirection `FrameLineColor`/`TitleColor` already go through for colours,
just carrying a WAD path string instead of an ARGB one.
`Data\Skins\Default\Skin.xml` (the activated style skin - see
`docs/formats/race-setup.md`) declares it:

```xml
<Variable global="BackgroundTopRightTexture">
    <Values String="Data\Skins\Default\Images\default_texture.mip"></Values>
</Variable>
```

`oag_formats::wad::hash_name` on that literal lands exactly on `7ba78aca` -
the value this section already found by content scan. **`FALLBACK_IMAGES`
now resolves the global directly** (`"FEGlobals->BackgroundTopRightTexture"`,
followed through `Screens::resolve` the same way a colour attribute already
is) rather than hard-coding the hash - the value was re-derivable from the
skin's own declared string, and `CLAUDE.md`'s "never invent what the assets
already author" section is explicit that a re-derivable value should not be
hand-transcribed even when, as here, the transcription happened to be
correct. `oag_pure::hashes::MENU_TOPRIGHT_LOGO` is kept only as the evidence
trail for the hash agreement, not as what the code consults any more.
Confidence **95**: an exact visual match to a real captured frame, agreement
across both pressings, and now the exact declared global and its literal
value read directly rather than inferred from content alone.
`crates/game/tests/pure_boot_ground_truth.rs`'s
`fe_screens_own_corner_logo_gets_the_measured_texture` pins the rect and the
decoded size against both real discs, now against the resolved literal name
rather than a bare hash spec.

**`BackgroundImage` is not.** Every full-screen-shaped candidate above was
checked against a real `Main Menu` capture - PPSSPP v1.20.4 under Xvfb
(`:97`), `--xres 960 --yres 544` for exact `2x` native resolution, a fresh
profile driven the whole way from a cold boot: `Language Selection` (cross)
-> `Developer Publisher Screen` (auto) -> `MemoryStickWarning` (cross) ->
`FMV Intro` (start, skipping the movie) -> `Title Screen` (start) -> `Profile`
-> `New` -> `Set Name` -> `Set Tag` -> `Save Profile` (yes) -> `Main Menu`,
using PPSSPP's own websocket debugger (`input.buttons.press`) rather than
keyboard injection, so no PPSSPP keymap needed guessing. **None of the three
full-screen candidates matches**: the real `Main Menu` background is flat
white with nothing drawn on it at all - sampled directly
(`magick ... -format "%[fx:mean...]"` over the open area between the row list
and the footer) reads exactly `255,255,255`, not close-to-white, to the pixel.
That is what `oag_title::MenuSkin::background` (`0xFFFFFFFF`, already measured
- see `menus-original.md`) already supplies as a plain colour fill, so the
existing behaviour is correct as it stands; nothing in `FALLBACK_IMAGES` was
added for `BackgroundImage`, on purpose - adding one of the three ruled-out
candidates anyway would be a guess dressed as a measurement, exactly what that
table's own doc comment says it will not hold. What this pass added was that
**none of the 361 decoded textures on this disc is it**, checked against one
real capture - not why.

**2026-09-23: the why is read, and it closes the open question.**
`BackgroundController_UpdateImages` resolves `BackgroundImage` from a declared
`FEGlobals->BackgroundTexture` global, the same mechanism
`BackgroundTopRightImage` resolves from above - and `Data\Skins\Default\
Skin.xml` declares it as the **empty string**:

```xml
<Variable global="BackgroundTexture">
    <Values String=""></Values>
</Variable>
```

The updater's own code treats an empty resolved string as an explicit
"no texture" branch, not a failed lookup falling through to nothing by
accident. So the open question above - "whether the real disc ever draws a
picture here" - is answered for this skin: **no, by its own declaration**,
not merely by absence from a 361-texture scan. `FALLBACK_IMAGES` now names
`BackgroundImage` explicitly (`"FEGlobals->BackgroundTexture"`) so a widget
that resolves to nothing does so because the disc says so, not because the
table happens not to mention it - `crates/game/tests/pure_boot_ground_truth.rs`'s
`fe_screens_backdrop_resolves_its_declared_empty_global` pins that the widget
carries no `src` on both real discs. Whether some *other* skin this project
has not seen activated declares a non-empty value is still open - only
`Data\Skins\Default` was read.

### `Title Screen`'s `<Animation><Key TextureWidth="...">` reveal - the struct is read, the renderer is not

`Title Screen->Viewport` wraps thirteen `<Animation>` elements, one per frame
line, bracket segment or textured patch, each carrying two or three
`<Key Time="..." TextureWidth="...">` entries and no other authored attribute.
Read via `Animation_ParseValuesOrKey`/`Animation_ConstructFromNode`
(`docs/ghidra/functions/psp-pure-eu/title-screen.md`): `<Key>` is a shared,
0x20-byte struct with seven fields - `Time`, `X`, `Y`, `TextureWidth`,
`TextureHeight`, `ScaleX`, `Scaley` (the disc's own spelling) - the same
struct `hud.rs`'s own `<Animation><Key>` reading already covers for `X`/`Y`
as a travel on Pulse's HUD icons; `Title Screen` is the case that authors
`TextureWidth` instead.

**The authoring convention, confirmed across all thirteen:** every `<Key>`
brackets `TextureWidth` between `-<width>` (the wrapped widget's own `width`/
`TxtrWidth`, negated) at an early `Time` and `0` at the final `Time`, several
holding the negative value through an intermediate `Time` before a fast snap
to `0` at the end (e.g. `TitleAnim13`: `-3` at `0`, `-3` at `0.63`, `0` at
`0.65`) - a hold-then-reveal shape, not a uniform ramp from the first `Time`
to the last.

**What is not read: how `TextureWidth` maps to a rendered pixel.** Whether the
widget's displayed width grows from `0` to its authored full width, whether
`TextureWidth` crops a texture-space sample instead of the render rect, and
which screen edge stays anchored, are none of them confirmed by a read of an
update or draw call - the object's own real leading vtable (C++ ABI offset
`+0x00`) was not located this pass; the `+0x3c` table this reading did locate
is a parse-time "which function reads this XML child tag" table, not a
runtime behaviour vtable. **`oag_ui::screen::Screens::collect_widgets` still
discards the `<Key>` timeline and draws every widget at its final, fully
revealed state** - a defensible simplification given the render mapping is
genuinely unread, not an oversight, and not something to guess at: `CLAUDE.md`
is explicit that a plausible-looking stand-in is worse than an honest gap.

**2026-09-26: a live watchpoint narrows the gap without closing it.** A
non-halting `memory.breakpoint` armed on twelve `Key.TextureWidth` addresses
(all twelve static and byte-identical across two independent cold boots, so
hardcodable rather than rescanned) through a full scripted run from
`Language Selection` to a settled `Title Screen` shows each written **exactly
once** (the authored-value parse) and **read by a normal CPU load never** -
not during construction, not during the authored 0.65-1.5 s reveal window,
not afterward. Writing a new value directly into an already-settled `Key`
and letting the emulator run confirms the same thing from the other side: no
visible effect next frame. The leading hypothesis is that the actual
consumer reads the `Key` struct's first VFPU quad (`Time`/`X`/`Y`/
`TextureWidth` are exactly 16 bytes) via `lv.q` rather than a scalar load,
which this debugger's watchpoints do not appear to hook - unverified, since
no consumer function has been located to check for it. The `Animation`
class's own leading vtable slot (offset `+0x00`) is confirmed zeroed by its
root-most constructor and never written by any constructor read so far, and
the one candidate update function the previous pass flagged from the `+0x3c`
table's neighbourhood (`FUN_088b9ca8`) is retracted - decompiled this pass,
it is an unrelated position/velocity integrator. Full method, the retraction,
and the exact watchpoint results: `docs/ghidra/functions/psp-pure-eu/title-screen.md`'s
own "2026-09-26" section and "Next steps" for where this picks back up.

### The language plugin id space is Pure's own, not Pulse's

`oag_pure::FRONT_END::language_plugins` used to be Pulse's USA set copied
outright - `PI008`-`PI012` - on the unchecked assumption that a title's
language ids are a shared convention. They are not: Pure's own id space was
never probed before this was measured, only assumed to line up with Pulse's,
and it does not.

Hashing every `Data\Plugins\PI0NN\Definition.xml` for `NN` in `000..032`
against both `pure-psp-eu.chd` and `pure-psp-usa.chd` finds nine plugins on
each pressing: `PI000`, `PI001`, `PI003`, `PI004`, `PI005`, `PI008`-`PI012`.
Five name a `<Font Language=...>` block and a self-naming `<Entry ID=
"<language>">` - the shape [`Language::from_definition`] treats as a real,
picker-worthy language:

| Plugin | Language | Native name | Font source (Default role) |
| --- | --- | --- | ---: |
| `PI000` | English | English | `FX300ANG.fnt` |
| `PI008` | French | Français | `FX300ANG.fnt` |
| `PI009` | German | Deutsch | `FX300ANG.fnt` |
| `PI010` | Spanish | Español | `FX300ANG.fnt` |
| `PI011` | Italian | Italiano | `FX300ANG.fnt` |

The other four are not languages at all, despite each carrying a `Language=`
attribute somewhere in its tree - which is exactly why probing the id space
naively is unsafe:

- **`PI001`** is a `PI_Skin` (UI chrome), and **`PI004`** is billboard
  placements - neither carries a `Language` attribute at all, so
  `Language::from_definition` already returns `None` for both.
- **`PI003`** carries no `<Font>` block, only a `<StringTable>` whose own
  comment reads `Japanese-English ie English string with Japanese Buttons` -
  a button-glyph substitution layer for use alongside Japanese, not a
  selectable language. `find_language_attribute`'s depth-first walk falls
  through to its first `<Entry Language="English">` for want of a `<Font>`
  match anywhere above it, so it *does* parse as "English" - a second,
  string-table-only "English" that must not be added to the picker list.
- **`PI012`** is the same shape for the same reason: no `<Font>` block, and
  its own comment reads `US-English ie English with different spellings` - a
  handful of American-spelling overrides (`Synchronizing` for
  `Synchronising`, `minimize` for `minimise`), not a base language. It also
  resolves as "English" under the same fallback, and it is what the old
  plugin list mistakenly wired up: [`load_strings`] found no
  `Dynamic Entry File Source` in it (it has none), so the picker's "English"
  drew with an **empty** string table - the `HUD_CURRENT`/`HUD_BEST`/`HUD_LAP`
  raw-key symptom a race actually showed on screen.
- **`PI005`** is a real, complete Japanese language plugin - eight `<Font
  Language="Japanese">` blocks and its own inline strings - present on both
  Pure pressings and, per its structure alone, exactly as picker-worthy as
  `PI000`. It is left out of `language_plugins` here on the strength of
  Pulse's own precedent rather than a measurement of Pure's: Pulse's boot-time
  plugin manifest, read directly out of the executable
  (`docs/architecture/frontend-boot.md#the-eu-disc-pulse-psp-eudchd`),
  loads its `PI005` but never offers it as a picker choice on either region.
  **Whether Pure's own boot manifest does the same is unread** - no Ghidra
  pass has walked Pure's plugin table the way Pulse's was walked. Left
  unread deliberately rather than tracked as a thread: Japanese Wipeout Pure
  shipped its own regional disc, so this project's Japanese support is a
  separate image and boot chain to add when that becomes a goal, not a
  question about the EU/USA pressings this page covers.

**Corrected 2026-09-08: the sentence that stood here said `PI000` was the
odd one out. It is not - none of Pure's five picker languages names an
external `entries.xml`, on either pressing.** Re-checking all five
`Definition.xml` files by hand finds zero occurrences of `Dynamic Entry File
Source` across `PI000`/`PI008`/`PI009`/`PI010`/`PI011`, each carrying its
whole table inline in its own `<StringTable>` instead - 906, 882, 879, 886
and 882 `<Entry>` tags on `pure-psp-usa.chd`, 910, 895, 892, 899 and 895 on
`pure-psp-eu.chd` (same five plugins, same order, counts differ only because
the EU text is longer). So this is Pure's convention for every language it
offers, on both pressings, not a one-off for English on one disc: unlike
Pulse, whose plugins name an external `entries.xml`, Pure's shipped
languages never do. `PI000`'s
own table is still worth calling out for size - the roughly 1,000-entry
table, HUD captions (`idstring="HUD_Lap"`, `HUD_current`, `HUD_best` -
matched directly against `Data\XML\TimeTrial_HUD.xml`'s own `idstring`
attributes) included, sits directly inside `Definition.xml`'s own
`<StringTable>` - but `Data\Plugins\PI000\entries.xml` not existing is not a
gap peculiar to English; nothing under `Data\Plugins\PI0NN\entries.xml`
exists for any of the five. [`load_strings`] re-parses the definition itself
as a string table when a language names no external one; a `<Font>`/
`<Values>` node carries no `<Entry>` tag, so this costs nothing on a plugin
that really points elsewhere and is what makes every one of Pure's five
languages resolve, German included - confirmed directly by
`--dry-run --no-video`'s own report line,
`Data\Plugins\PI009\Definition.xml: 873 strings for German`, which is the
exact language name the `German names no string table` failure this
mechanism replaced used to print.

Confidence **90**: every plugin id, its `Language`/`Entry` shape and its font
and string content are read directly off both pressings and asserted in
[`pures_picker_offers_its_own_manifest_english_included`]. The one
unmeasured claim - whether `PI005` is ever offered in Pure's own picker - is
carried at no confidence score, per the RE workflow's rule for an unread
question rather than a guess; it is out of scope rather than open, per the
note above.

**Settled 2026-10-06:** neither Pure manifest names `PI005` (or `PI003`), so the
Japanese plugin is never loaded, and Pure USA's picker offers three of the five
languages on its disc. See [the offered languages](../architecture/frontend-boot.md#the-offered-languages-are-each-executables-plugin-manifest).

[`Language::from_definition`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/game/src/language.rs
[`load_strings`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/game/src/boot.rs
[`pures_picker_offers_its_own_manifest_english_included`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/game/tests/pure_boot_ground_truth.rs

See [front-end menu definitions](fe-menu-definitions.md) and
[the original's menus](../ui/menus-original.md).

## Music: recovered by name, and played

Pure's front end and its races both have music now, and neither is found by
guessing at a population: **every path came off the disc.**

### Both halves are in `BOOT.BIN`

`strings` on the unencrypted ELF puts four `.at3` paths in the binary, three of
them adjacent to `c:/Work/Wipeout/Code/System/Sound/MusicManager.cpp`:

```text
242b20  MusicManager
242b30  c:/Work/Wipeout/Code/System/Sound/MusicManager.cpp
242b64  %s\%s
242b6c  music.at3
242b78  Data\Music\frontend.at3
242b90  SoundManager
242bd4  Data\Sound\frontend.bnk
242bec  Data\Sound\hud.bnk
242c00  Data\Sound\generaltrack.bnk
```

- **The front end's own music is `Data\Music\frontend.at3`**, a literal. Pulse
  expands a `Data\Music\FEMusic\frontend%d.at3` template instead, and none of
  that template's eight expansions hashes to an entry on either Pure pressing -
  so this is a different shape, not a different spelling.
- **A soundtrack track is `music.at3` inside a directory**, `%s\%s` supplying
  the join. The directory comes from the plugin definition below.

The three `Data\Sound\*.bnk` names are present on both pressings and **are
decoded** - see [below](#sound-effects-the-same-container-after-all). The
sentence that stood here until 2026-08-23 said the opposite.

### The directories are declared, not derived

`Data\Plugins\PI001\Definition.xml` carries a `PI_Music` node per track beside
its `PI_Track` and `PI_Team` ones, in the same schema
[`oag_raceplay::catalogue`](../../crates/raceplay/src/catalogue.rs) already reads:

```xml
<PI_Music name="A Piece Of Music">
  <Values location="Data\Music\SomeArtist"></Values>
  <Entry Artist="Some Artist"></Entry>
  <Entry Label="Some Label"></Entry>
</PI_Music>
```

(Shape only - the shipped titles, artists and labels are content and are read at
run time, never written down here. See
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md).)

There are **nineteen** of them to Pulse's sixteen, and joining each `location`
with `music.at3` resolves to a real `Data.wad` entry **19 of 19 on both
pressings**. `Artist` and `Label` are parsed past rather than kept: nothing
displays them yet.

### Why this matters beyond Pure

The order is now the release's own rather than the archive directory's, which
is what makes "track 0" mean something. Pure's longest track is 326 s against
205-229 s for the other eighteen, it is *not* first in `Data.wad` offset order,
and the declaration puts it first - which is the single assertion that
distinguishes the two orderings, pinned in
`crates/game/tests/pure_music_ground_truth.rs`.

**Pulse declares its sixteen the same way**, and every one of those resolves
too. This build still finds Pulse's by what the entries are, because switching
it over reorders its race playlist and changes which track a PS2 boot's menu
plays - a separate change with its own evidence to record. See `HANDOVER.md`
and [`oag_title::Music::tracks`](../../crates/title/src/lib.rs).

Confidence **90**: both path halves are read off the executable, the
declaration is read off the disc, and every expansion hits on both pressings -
but nothing has been watched running under an emulator, so which track the
original's own menus start on is not established.

### Reproducing it

```sh
just wad cat "data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/Data.wad" \
    'Data\Plugins\PI001\Definition.xml' | grep -A3 PI_Music
just wad hash 'Data\Music\frontend.at3'          # -> 1ddc4f8e
cargo nextest run -p oag-game --test pure_music_ground_truth --run-ignored all
```

## Sound effects: the same container after all

**2026-08-23.** Pure has **29 `SBlk` sound banks** on each pressing, every one
accepted by [`oag_formats::sblk`](../../crates/formats/src/sblk.rs) with no
change, and **a Pure race now makes the same six sounds a Pulse race does**.

### How the absence claim came about

This page and [psp-audio.md](psp-audio.md) both recorded, at confidence 85,
that no entry in any Pure archive begins with `SBlk`. That is **true**, and it
is equally true of Wipeout Pulse: the magic sits at offset `0x18`, behind the
container's own eight-byte header and its two eight-byte section-table entries.
The Pulse census never tripped over it because it went through
`sblk::looks_like_bank`, which reads `0x18`; the Pure probe scanned offset 0.

A search that would have returned the same empty result on a disc known to be
full of banks was read as evidence of absence. **A magic-scan miss is only
evidence when the scan has been shown to hit on a positive control.**

### What is there

| | Pure USA | Pure EU |
| --- | ---: | ---: |
| `SBlk` banks | 29 | 29 |
| Parsed unmodified | 29 | 29 |
| Sound names | 254 in `Data.wad`, 18 in `FE.wad` | same |
| Waveform spans | 461 | 461 |
| Spans in a non-PS-ADPCM codec | 0 | 0 |

The bank paths are Pulse's, hash for hash - `Data\Sound\hud.bnk`, `ship.bnk`,
`ship_zone.bnk`, `weapons.bnk`, `speech.bnk`, `frontend.bnk`,
`generaltrack.bnk` - and so are the cue strings the game fires. `SPEEDUPPAD`,
`.COLLISIONS`, `ABSORB`, `~ENGINE`, `~SHIELD` and `shieldactive` all resolve,
so `oag_sound::sfx` needed **no Pure branch at all**.

Pure's own content differs in ways that are worth recording because one of them
was load-bearing:

- **`~SHIELD` binds four waveforms of which only two loop**, where Pulse's
  binds two and both do. That is the case that forced the loop flag to be kept
  per waveform rather than collapsed to the cue - a play site that assumed the
  cue was uniform would loop a one-shot on about half the draws.
- Pure's `ship.bnk` has **six** cues to Pulse's nine, and carries `~AMBIENCE`
  and `rumble`, which Pulse has not got.
- Pure names its circuit banks in full - `SEBCLIM`, `CITTANU`, `VINETTA`,
  `MODESTO`, `CHENGOU` - where Pulse abbreviates.

Verified end to end: `pure-psp-eu.chd` under `--race --autopilot --ticks 600
--dump-audio` reports all six cues loading and writes ten seconds of audio
peaking at 0.83 with three silent ticks at the head.

### A circuit's own ambience: `woSound` is class `0x393`, found without Ghidra

`docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md`'s own Pure
section (2026-09-08) found the *nodes* by name alone - a text scan of a raw
extracted `track.vex` turned up `woSound1`..`woSoundN`, each carrying a
bank/cue pair - but left the class ID unfound, expecting the table-index
technique above to be needed.

**It was not.** `oag_vex::vex::Node` already carries the class ID *and*
the scene name for every node; `just view '<pure image>:.../Data.wad' --nodes
'<track.vex>'` prints both with no Ghidra bridge at all. That is enough: a
`woSoundN` name is not itself the sound class - it names a `Transform`
(Pure's renumbered `0x6d`) that parents one child named `woSoundNShape`, and
**every one of those children is class `0x393`**, an 80-byte payload whose
bytes read the same shape `track-sound-emitters.md` documents for Pulse's
`sound` - a bank label and a `~`-prefixed cue name as ASCII text, a `u16`
count and two relocatable curve-array offsets past them. Confirmed by content,
not just position: `VINETTA`, `GENTRAK`, `~TRACKLIGHT`, `~CROWD`, `~TRAIN` all
read straight out of the payload bytes, the same names the bank census above
already established.

129 `0x393` nodes were read across the three circuits this pass checked - 48
on `01_Vineta_K`, 28 on `10_Sebenco_Climb`, 53 on `04_Chenghou_Project` - and
**the first two `f32`s of every one of the 129 are bit-equal**, the same
no-cone-split signature `track-sound-emitters.md`'s text scan already read
off the node names (no `woSoundCone`/`woSpeaker` name anywhere). So this
corroborates that finding on a different axis - the payload's own bytes,
not what the scene graph calls the node - rather than merely repeating it.
**Whether Pure distinguishes a cone at all stays a real "no" now, not just an
absent name.**

The circuit counts here are roughly half the thread's own 48/56/106 text-scan
figures on the latter two circuits (Vineta K matches exactly). Read as: that
scan most likely matched both a parent's `woSoundN` name and its child's
`woSoundNShape` name as two separate hits on some circuits and not others,
depending on how its pattern anchored - **48 is confirmed exactly by both
methods**, so this is a discrepancy in the older count, not in this one,
which is counting actual class-`0x393` nodes rather than name-string matches.
Not chased further, since the node count was never this pass's own question.

Pure's own field layout is a different shape from Pulse's (the bank string
starts at payload `+0x08` here, not `+0x14`, so nothing beyond the class ID
and the no-cone-split negative was read) and a full derivation is future
work - what this settles is only what the thread's Next Step asked for.

## Reported elsewhere

The `WO Track` result belongs on [track data](track.md) and was handed over
rather than written there, to avoid colliding with concurrent work on that page.
The finding, for whoever folds it in:

- 16 Pure `.vex` files carry a `WO Track` node, under class ID `0x36d`.
- Version is `0x103` on all 16, against Pulse's `0x105`; both are at or above
  `MIN_VERSION` and both take the `>= 0x101` reserved block.
- `encoded_len() == payload.len()` holds **exactly on 16 of 16** under the
  documented layout - the same closure that settled it on Pulse's 40 files.
- One divergence: the header word at `+0x18`, documented as always `1`, is `0`
  on all 16 Pure tracks. Worth a sentence on the page, since "always 1" is now
  "1 in Pulse, 0 in Pure".

## Reproducing this

Every number above comes from the shipped tools plus read-only scratch scripts
that were **not** committed, per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) - extracted
assets and rendered PNGs live outside the repository. The tool half:

```sh
just unpack info   data/images/pure-psp-usa.chd
just unpack list   data/images/pure-psp-usa.chd
just unpack sniff  data/images/pure-psp-usa.chd

for w in Data FE FEData; do
    just wad list   "data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/$w.wad"
    just wad tags   "data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/$w.wad"
    just wad verify "data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/$w.wad"
done

just wad cat "data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/Data.wad" \
    'Data\Ships\Feisar\handlingstats.xml' --expand
```

## Pure's weapon models were Pulse's names (2026-10-08)

`oag_pure`'s `WeaponModels` named `Pulse_Mine.vex`, `Pulse_Bomb.vex`, `pulse_muzzleflash.vex` and the
`pulse_plasma_*` trio, none of which is on Pure's disc, so a Pure Mine and Bomb were never drawn. The
executable's own list (`Mine.vex`, `Bomb.vex`, `explosion_hemisphere.vex`, `Bomb_Shockwave.vex`,
`plasma_halo.vex`, `plasma_hemisphere(_noglow).vex`, `disruptor_*.vex`, `electric_halo2.vex`,
`explosion_gaseous.vex`) is 50 of 50 present by hash. Wired: Mine, Bomb, the Bomb blast; the Plasma blast and
the Disruptor and Missile effects are not. Both charges are laid at the craft's rear anchor, `4.875` behind the body (Feisar only), and the Bomb is drawn
at `0.4`, both read off the running original. Evidence, captures and the open list:
[`weapons-gfx.md`](../ghidra/functions/psp-pure-usa/weapons-gfx.md).
