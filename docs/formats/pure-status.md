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
| Handling stats | Same schema and addressing; **the parser refuses the file** |
| Collision classes | Not located - see [collision](collision.md#the-same-format-is-in-wipeout-pure) |
| `SBlk` sound bank | **Not present on Pure at all** |

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
| [`.vex` embedded textures](#pures-model-textures-ship-pre-swizzled) | **breaks** | Pure sets the pre-swizzle flag on model textures; `vex::textures` never reads that byte | 88 |
| `WO Track` payload | reads | 16 nodes carry the magic; version `0x103` against Pulse's `0x105`; `encoded_len == payload length` exact on **16/16** under the documented layout, reserved block included. Reported separately, see [below](#reported-elsewhere) | 94 |
| [Collision geometry](collision.md#the-same-format-is-in-wipeout-pure) | **breaks** | Already recorded: none of the five Pulse collision class IDs appears; three candidates decode exactly but which is which is undetermined | 90 / 45 |
| [Front-end XML](fexml.md) | reads | 291 XML entries across the three archives. **Zero** begin `<code`, so the name shortening is a Pulse-era addition and `--expand` is correctly a no-op on Pure | 94 |
| [Handling stats](handling-stats.md) | **breaks** | Addressing and schema hold; the parser rejects the file on three counts. See [below](#handling-stats-the-schema-holds-the-parser-does-not) | 92 |
| [Bitmap font](fnt.md) | reads | 6 fonts. Version `1` + `"FNT"`, codepoint table at `0x30`, offset table at `0x30 + 2*count`, and `atlas + 0x40 + clut_size + texel_size == file length` exact on **6/6**, with `texel_size == width * height / 2`. The atlas flag bit is set on 6/6, same as Pulse | 94 |
| [PSP movie](pmf.md) | reads | 14 movies. `stream_offset == 0x800` and `stream_offset + stream_size == file length` exact on **14/14**; two streams each, ids `0xe0` and `0xbd`. Version string `"0012"` only, where Pulse ships both `"0012"` and `"0014"` | 94 |
| [PSP sound bank](psp-audio.md) | **absent** | No entry in any Pure archive begins `SBlk`, though the executable names `Data\Sound\*.bnk` paths. Pure's 25 RIFF entries are `WAVE_FORMAT_EXTENSIBLE`, 2 channels, 44.1 kHz | 85 |
| Particle system (`.pob`) | unknown, both | 37 `SYSP` entries in `Data.wad` and 37 in `FE.wad`, against Pulse's 35 and 27. Still undecoded on either disc | - |

## The class-ID space is renumbered

`.vex` class IDs are **table indices, not a stable enum**: the exporter's
class table grew between the two titles and every ID after an insertion point
shifted. Pulse's `Transform` at `0x6e` is Pure's `0x6d`; Pulse's `Mesh` at
`0x125` is Pure's `0x11e`. None of Pulse's constants appears in any Pure file.

Each mapping below was recovered by an exact property of the payload, not by
counting nodes and matching:

| Node type | Pulse | Pure | How it was pinned | Confidence |
| --- | ---: | ---: | --- | ---: |
| `Transform` | `0x6e` | `0x6d` | 5,016 nodes: 606 with an empty payload (the identity), 4,410 of exactly 64 bytes, of which 4,365 have orthonormal rows 0-2 | 90 |
| `Mesh` | `0x125` | `0x11e` | 11,025 nodes whose payloads walk as batch lists, and whose 2.1 M decoded vertices all fall inside the batches' own declared boxes | 92 |
| `Texture` | `0x3c1` | `0x373` | `sum(clut_size + texel_size)` over these nodes equals the file's declared texture-block length on 156/156 files | 92 |
| `WO Track` | `0x3bb` | `0x36d` | 16 nodes, each carrying the `WOtd` magic at `+0x00` | 94 |
| The five collision classes | `0x3b9`, `0x3ba`, `0x3cd`, `0x3e6`, `0x3e7` | not determined | Three candidates decode exactly; see [collision](collision.md#the-same-format-is-in-wipeout-pure) | 45 |

**The consequence for the code is smaller than it looks.** `CLASS_MESH`,
`CLASS_TEXTURE`, `CLASS_TRANSFORM` in
[`oag-formats::vex`](../../crates/formats/src/vex.rs) and `CLASS_WO_TRACK` in
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

The `Texture` node payload has a flags byte at `+0x06` whose bit 0 means *the
texels are already in the GE's 16-byte by 8-row block order*. On Pulse this bit
is set only on [font atlases](fnt.md#the-texels-are-stored-already-swizzled).

On Pure it is set on model textures too: the Feisar ship's five embedded
textures read flags `0x61`, where Pulse's eight read `0xe4`.

[`vex::textures`](../../crates/formats/src/vex.rs) reads width, height,
`bits_per_pixel`, `mip_count`, `clut_size` and `texel_size`, and **never looks
at `+0x06`**. On Pulse that is harmless. On Pure it would return scrambled
texels for every model. Unswizzling by the block rule the font path already
implements turns the ship's environment map back into a clean radial highlight,
which is what confirms both the flag's meaning and that nothing else is wrong
with the block. Confidence **88**.

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

## Handling stats: the schema holds, the parser does not

`Data\Ships\<Team>\handlingstats.xml` exists on Pure at the same addresses, in
plain (unshortened) XML, with the **same element names, the same attribute
names and the same `<Stats team>` / `<Class name>` nesting**. Nine teams carry
one, against Pulse's eight.

[`oag_formats::handling::parse`](../../crates/formats/src/handling.rs) still
refuses every one of them, on three counts:

| What | Why it fails |
| --- | --- |
| Pure has a **fifth speed class** below the four Pulse ships | `SpeedClass` is a four-variant enum over a fixed-length array, so the extra `<Class>` block raises `UnknownClass` |
| `<Airbrake sideshift>` | Required by the parser, absent from Pure |
| `<Misc easyshield>`, `<Misc weight_distribution>` | Required by the parser, absent from Pure |

All three are Pulse-era **additions**, which is the useful direction: the
handling model grew between titles rather than changing shape. Every attribute
being required is a deliberate choice on that page - values are never defaulted
- so this is the parser working as designed on a file it was not designed for,
not a bug. Confidence **92**, from a direct schema diff of the two files.

## Tooling defects this probe surfaced

Neither is a format overfit; both are missing guards on paths that had only
ever seen Pulse data.

- **`oag-view --collision` panics on a file with no collision nodes.** It
  correctly reports `0 collision node(s)` and prints an empty per-class table,
  then hands an empty vertex buffer to wgpu and dies inside
  `buffer.rs`: `buffer slice can not be empty`. Any `.vex` without a
  recognised collision class reaches this, which on Pure is every file.
- **`oag-trace drive --source <pure>` fails on a hard-coded Pulse track path.**
  Expected, and recorded only for completeness: the asset layer resolves by
  name against the source's own directory, so this is the scenario naming a
  Pulse track, not a portability problem.

## What a real Pure asset milestone would cost

Ordered by cost, from the measurements above:

1. **Browsing Pure's models and textures: about a day.** A per-title class-ID
   table selected by the file's version word, plus honouring the pre-swizzle
   flag in `vex::textures`, plus the empty-model guard in `oag-view`. Every
   other line of the geometry path already works, on 2.1 M vertices of
   evidence.
2. **Tracks: a day on top of that**, and mostly the same change - the `WO
   Track` payload already decodes exactly under one more class ID.
3. **Collision: unknown, and the first real research.** Three candidate classes
   decode; deciding which is floor, wall and reset needs the loader read out of
   Pure's own executable. Days, not hours.
4. **Handling: a schema-version split.** A fifth speed class means the
   fixed-length array is the wrong shape for a two-title parser, and three
   attributes need to become title-conditional rather than defaulted.
5. **Audio: a new format.** `SBlk` does not exist on Pure; whatever indexes its
   RIFF streams has not been looked at.

None of that is on the roadmap, and this page does not put it there. What the
probe establishes is the ADR-0009 premise: **the format layer is genuinely
multi-title, and the places it is not are countable.**

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
