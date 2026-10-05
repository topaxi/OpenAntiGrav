# PS2 texture

**Status: understood, including the ship and track lookup and all three
transfer shapes.** Located, decoded, implemented in
[`oag-texture::ps2_texture`](../../crates/texture/src/ps2_texture.rs); every
ship on the roster now draws in its own livery in `just play pulse-ps2`
itself, not only through `oag-view --mesh ... --textures ...` naming an entry
by hand, and so does every circuit's own `track.vex` - all 32, since
`oag-render` moved from a flat ordinal index to resolving each `Texture`
node by its own declared name (2026-09-05, see [the original never suffers
this
collapse](#the-original-never-suffers-this-collapse-it-resolves-every-texture-by-name-not-by-ordinal)
below). See [how a model finds its texture
set](#how-a-model-finds-its-texture-set-directory-position-not-a-name) below
for the other half: which *entry* is a model's set, as opposed to which of
that set's textures a given node means.

Found as standalone entries in the PS2 [WAD](wad.md) archives. Not the same
format as the [PSP `.mip`](psp-texture.md) at all, and not embedded in the model
the way the PSP's are: see [`.vex`](vex.md#embedded-textures).

## Where they are

The PS2 `.vex` files declare a texture block of length **zero**. Their textures
are **separate WAD entries**, and there are 5,348 of them across `WADS2.WAD` and
`WADSP.WAD`.

A model's set is also gathered into a **nested WAD** — the same 8-byte header
and 16-byte entries as the outer archive, [`oag-formats::wad`](wad.md) parses it
unchanged — with one entry per `Texture` node of the model, **in node order**.
For `Data\Ships\Feisar\Ship.vex` that set is entry `0xfc3f75cf`: 5 entries
against the model's 5 `Texture` nodes, and decoding them gives the Feisar hull
atlas, its lights sheet, an environment map and a glass gradient, matching the
node names `Textures_All`, `Lights_GLOW`, `envtest4bit`, `Glass2_ADD`,
`blink_GLOW`.

Every one of the 2,252 distinct hashes inside the 443 texture sets in
`WADS2.WAD` is *also* a top-level entry of the same archive, so the sets are a
locality optimisation rather than the only copy.

## Layout

```text
+0x00  u8    packed log2 dimensions, high nibble log2(height), low nibble log2(width)
+0x01  u8    bits_per_pixel      4 or 8
+0x02  u16   flags               0x2000 or 0x2040
+0x04  u16   height
+0x06  u16   width
+0x08  u32   unknown, correlates with +0x0c
+0x0c  u8    unknown
+0x0d  qword[8]  GS state, not decoded
+0x8d  qword     A+D register write, TRXPOS (0x51)
+0x9d  qword     A+D register write, TRXREG (0x52)
+0xad  qword     A+D register write, TRXDIR (0x53)
+0xbd  qword     GIFtag, FLG=IMAGE, NLOOP*16 == texel bytes
+0xcd  texels, width * height * bpp / 8 bytes
       qword     A+D register write, TRXPOS
       qword     A+D register write, TRXREG
       qword     A+D register write, TRXDIR
       qword     GIFtag, FLG=IMAGE, EOP, NLOOP*16 == palette bytes
       palette, (1 << bpp) * 4 bytes, RGBA8888
       padding
```

The blob is not a texture *file*. It is the **DMA packet that uploads one**: a
13-byte header the game reads, then GIF packets that transfer the texels and
then the palette into GS local memory. `BITBLTBUF` is absent, so the destination
buffer is chosen by code, not by the file.

Each transfer block is budgeted 256 bytes even when it needs fewer, so the total
size is

```text
205 + max(width * height * bpp / 8, 256) + 64 + max((1 << bpp) * 4, 256)
```

but the blocks themselves sit back to back and the slack lands at the **end** of
the file. A 4x4 texture is 1,549 bytes, 240 of them trailing padding. Padding
the palette *offset* instead of the total reads correctly on all but 70 of the
disc's textures and then falls apart on the small ones.

## The dimensions are stored height first

`+0x04` is the **height** and `+0x06` the width. Reading them the other way
round decodes every square texture perfectly — which is most of them — and
scrambles every other one.

`TRXREG` is what settles it, because it is a fixed function of the dimensions
and it is asymmetric:

| Stored `+0x04`, `+0x06` | `TRXREG` | Consistent with |
| --- | --- | --- |
| 256, 128 | 64 x 128 | width 128, height 256 |
| 128, 256 | 128 x 64 | width 256, height 128 |
| 64, 128 | 64 x 32 | width 128, height 64 |

Under the width-first reading none of those are `(w/2, h/2)`, `(w, h)` or
anything else with a rule; under the height-first reading every one of the 5,348
textures matches one of exactly three shapes.

## Three transfer shapes, and the file says which

Dividing the texel byte count by the `TRXREG` area gives the bytes per
destination pixel, which identifies the transfer format from the data rather
than by assumption:

| `TRXREG` | Bytes/pixel | Meaning | Count |
| --- | ---: | --- | ---: |
| `(width/2, height/2)` | 4 | 8-bit texels blitted as **PSMCT32** | 5,077 |
| `(width, height)` | 1 | direct **PSMT8** transfer | 266 |
| `(width/2, height/4)` | 4 | 4-bit texels blitted as PSMCT32 (**PSMT4**) | 5 |

The first is the standard PS2 trick for uploading an indexed texture: the GS has
no host-to-local path that is fast for 8-bit, so the data goes across as 32-bit
pixels and is **pre-swizzled into `PSMT8` order** on disc so that reading it back
as `PSMT8` comes out right. That swizzle is what has to be undone, and the
decoder does:

```rust
fn psmt8_offset(x, y, width) -> usize {
    let block = (y & !0xf) * width + (x & !0xf) * 2;
    let swap = (((y + 2) >> 2) & 1) * 4;
    let row = ((((y & !3) >> 1) + (y & 1)) & 0x7) * width * 2;
    let column = ((x + swap) & 0x7) * 4;
    let byte_select = ((y >> 1) & 1) + ((x >> 2) & 2);
    block + row + column + byte_select
}
```

This is a permutation of `0..width * height` only for widths of 16 and up, and
the widths where it is not are **exactly** the ones the game stores linearly
instead — 4 and 8. That is a satisfying corroboration rather than a coincidence:
the halved rectangle the 32-bit path needs cannot be formed below that width, so
the game falls back, and the fallback set the arithmetic predicts is the set the
data has. The three shapes that use the direct path *and* could have used the
other one (32x128, 64x128, 512x512) are not explained - but they are not
mislabelled: all 185 testable linear textures decode smoother linearly than
swizzled, see [the permutation evidence](#the-permutation-separately).

### `PSMT4`, and why it is the same formula with one new fact

The five 4-bit blobs are the **five PS2 font atlases** - see
[`.fnt`](fnt.md#the-ps2-keeps-the-same-five-fonts-and-moves-the-pixels-out) -
which is why there are so few and why nothing needed them until the PS2 menus
were asked to draw text.

`psmt8_offset` above answers "where in the **linear `PSMCT32` source image**
does this indexed texel live", not "where is it in GS memory". That is the
whole reason it needs no block table: the blob is the *source of a blit*, and
the GS scatters it into pages and blocks on the way in. So the `PSMT4` version
is the same question with different geometry:

| | `PSMT8` | `PSMT4` | `PSMCT32` (the source) |
| --- | --- | --- | --- |
| source rectangle | `(width/2, height/2)` | `(width/2, height/4)` | - |
| page | 128x64 | 128x128 | 64x32 |
| block | 16x16 | 32x16 | 8x8 |

`PSMT8`'s page and block are both exactly 2x the source's in both axes, so its
blocks land in the source in plain raster order and the block table cancels out.
`PSMT4`'s page is 2x wider and 4x taller while its block is 4x wider and 2x
taller - they disagree, and the disagreement is exactly one transposition:
within a page, the block at block-column `bx`, block-row `by` sits at source
block-column `by`, source block-row `bx`. That is `blockTable4` being the
transpose of `blockTable32`, and it is the only fact here that is not already
carried by the `PSMT8` formula.

Inside a block the split follows from the counts. A 32x16 `PSMT4` block is 512
nibbles over an 8x8 patch of 32-bit source pixels, so `x`'s low three bits pick
the source column, `x`'s next two bits pick the byte inside that word, `y`'s
bit 1 picks the nibble, and `y`'s remaining bits pick the source row through the
same two-of-four row swap:

```rust
fn psmt4_offset(x, y, width) -> (usize, usize) {
    let page = (x / 128) * 64 + (y / 128) * 32 * (width / 2);
    let block = ((y % 128) / 16) * 8 + ((x % 128) / 32) * 8 * (width / 2);
    let swap = (((y + 2) >> 2) & 1) * 4;
    let column = (x + swap) & 0x7;
    let row = (((y & !3) >> 1) + (y & 1)) & 0x7;
    let word = page + block + row * (width / 2) + column;
    (word * 4 + ((x >> 3) & 3), (y >> 1) & 1)
}
```

**The transposition is not a choice between two plausible readings.** Without
it the mapping is not even a bijection: source words collide and others are
never read, which the unit test catches before a single pixel is looked at.
With it, it is a permutation of `0..width * height` at every shape on the disc.

The page term only works from 128x128 up, because a page is 128x128 texels and
a smaller texture would address source pixels that are not there. Everything on
the disc is 256x128 or larger, and `parse` refuses anything smaller rather than
indexing off the end.

## The palette is in `CSM1` order

A 256-entry CLUT is uploaded as a 16x16 `PSMCT32` rectangle, which is `CSM1`
layout: within each group of 32 entries, 8-15 and 16-23 are the other way round.
Undoing it is a bit swap on the index:

```rust
palette[i] = stored[(i & 0xe7) | ((i & 0x08) << 1) | ((i & 0x10) >> 1)]
```

The two failure modes are distinguishable on sight, which is worth knowing
before debugging one: a missed **texel** swizzle scrambles the image into blocks,
a missed **palette** swap leaves the shapes perfectly intact and bands the
colours every eight indices.

A 16-entry CLUT is a plain 8x2 rectangle and needs no reordering.

## Alpha is 0-128

Palette alpha runs 0 to 128, not 0 to 255: the GS treats 128 as full intensity
through the texture-modulate path, the same convention `.vex` documents for
[vertex colour](vex.md#ps2-the-vertex-type-still-names-the-attributes-but-the-data-is-a-vif-packet).
`Ps2Texture` keeps the palette exactly as stored and `to_rgba` doubles it, so a
caller that wants the raw bytes still has them. Not one of the 5,343 decoded
textures has a palette entry above 128, which the ground-truth test asserts.

## Evidence

### The arithmetic

Almost every field is declared twice, and the two declarations have to agree:

- The **packed log2 byte** at `+0x00` and the dimension words at `+0x04` say the
  same thing in two encodings. 5,348 of 5,348 agree; nothing else in either
  archive does.
- Both **`GIFtag`s** declare their payload in quadwords, and `NLOOP * 16` has to
  come out at exactly the texel and palette byte counts the dimensions imply.
  This is a field *in the file* stating the decoded size, which is a much
  stronger check than a size delta: 5,348 of 5,348, `FLG` = 2 (`IMAGE`) on both,
  `EOP` set on the second.
- The **register addresses** at `+0x95`, `+0xa5`, `+0xb5` are 0x51, 0x52, 0x53 —
  `TRXPOS`, `TRXREG`, `TRXDIR`, in that order, on all 5,348.
- The **total closes**, by the formula above, on 5,348 of 5,350 blobs that pass
  the log2 check.

### The permutation, separately

Everything above says the blob *is* a texture and where its parts are. It says
nothing about whether the texels were unpicked correctly, and plenty of wrong
permutations are bijections, so that needs its own check.

Scoring each decoded texture's **total variation** - the sum of `|delta|`
between adjacent indices, horizontally and vertically - against the same texels
read under the *opposite* layout is the check that discriminates: a wrong
permutation scatters pixels that belong together and raises local discontinuity.
Over the whole disc:

| Layout | Textures 32x16 or larger | Smoother as decoded |
| --- | ---: | ---: |
| `PSMT8` | 4,956 | **4,856** (98.0%) |
| linear | 185 | **185** (100%) |
| `PSMT4` | 5 | **5** (100%) |

The hundred `PSMT8` exceptions are flat or noise-like blobs with almost no
variation in either reading. The linear column is the one that matters most,
because that group has no other evidence behind it: it says the `TRXREG`-based
classifier is not mislabelling swizzled textures as linear, including the
32x128, 64x128 and 512x512 cases where the swizzle formula *would* have been a
valid permutation.

The neighbour-**asymmetry** test that [`Texture::looks_swizzled`](psp-texture.md)
uses was tried first and does not work here, which is worth recording so nobody
tries it again: PSP swizzle moves data in 16-byte rows, so reading it linearly
leaves columns correlated and rows not, but the GS `PSMT8` permutation is local
in both axes. It flagged 541 textures decoded against 531 read raw - no signal
at all.

### It renders

`oag-view --mesh "Data\Ships\Feisar\Ship.vex" --textures 0xfc3f75cf` against
`WADS2.WAD` draws the ship in Feisar's blue and yellow with the team name legible
along the hull and the UVs landing on the right panels. Decoded standalone, the
same textures come out as recognisable ship atlases, a logo sheet and panel
artwork rather than noise.

Confidence: **94** for the header layout, the packet framing, the height-first
dimensions and the alpha scale - every declared size closes against every other
one across 5,348 real files, and the framing is what those closures actually
evidence.

Confidence **93** for the `PSMT4` permutation, which is the best-evidenced of
the three despite having only five files behind it: it is a *derivation* from
the already-verified `PSMT8` formula rather than an independent guess, the
non-transposed alternative fails the bijection test outright, all five decode
smoother than the unswizzled reading, all five come out as legible character
sets, and on `pulse_text` **every single lit texel of 9,816 falls inside a glyph
box that font's own metrics declare** - an exact agreement between two
independent parts of the data, which is what the rubric wants. One point under
94 because the block transposition is asserted from GS geometry and the
bijection test rather than read out of the executable.

Confidence **92** for the `PSMT8` permutation, the linear/swizzled split and the
`CSM1` palette order. The evidence is different in kind: a corpus-wide
smoothness comparison against the opposite reading rather than an exact
arithmetic identity, plus the ship render, where texture coordinates decoded by a
completely separate path land on the right panels of the right artwork. Strong,
and not exact.

Per the [rubric](../reverse-engineering/confidence-rubric.md) all three are data
agreement rather than a runtime trace, which caps at 94 regardless.

## Reproducing

```sh
just test-data                       # runs the ground-truth test below
just view 'data/images/pulse-ps2-eu.chd:54748/WADS2.WAD' \
  --mesh 'Data\Ships\Feisar\Ship.vex' --textures 0xfc3f75cf --screenshot /tmp/feisar.png
```

The ground-truth test is
[`crates/texture/tests/ps2_texture_ground_truth.rs`](../../crates/texture/tests/ps2_texture_ground_truth.rs).
It walks both PS2 archives and reports:

```text
blobs        7377
textures     5348
decoded      5343
refused      5
pixels       140638128
layouts      {"linear": 266, "psmt4": 5, "psmt8": 5077}
flags        {8192: 3925, 8256: 1423}
shapes       39
testable     {"linear": 185, "psmt8": 4956}
smoother     {"linear": 185, "psmt8": 4856}
```

## How a model finds its texture set: directory position, not a name

**Solved for ships (confidence 90) and, separately, for tracks (confidence
88 - see below).** A model's texture set is never looked up by name or hash
at all - it is the archive entry **directly before** the model's own entry
in the WAD directory. `oag_assets::Archives::read_preceding`
implements this, and `oag_game::race::load` uses it for a ship and for the
track file: when a model's embedded texture slots are all empty (the PS2
signature), it reads the preceding entry and tries it as a texture set.

**The track file's set is not just the track model's.** A `Skycube` and a
`Speedup Pad` node's materials name a texture by its ordinal among *all* of
the file's `Texture` nodes, the same shared ordinal space the track's own art
meshes use (see [`build_class`'s embedded-texture handling](vex.md#embedded-textures)) -
they are not separate per-class texture sets. So `oag_game::race::load`
resolves the set once, against the track model, and passes that same
resolved `Vec<Option<ModelTexture>>` into `mesh::build_sky` and
`mesh::build_pads` too, rather than each doing its own `read_preceding`.
Missing this was a real bug until 2026-08-07: both builders hardcoded
`external: None`, so on PS2 the sky and the pads always fell back to the
(empty) embedded texture block and drew untextured - white, in the sky's
case, since it has no lighting to hide it. See
[`skycube.md`](skycube.md#open) for the before/after.

This was checked independently against **every** team on the roster, not
inferred from one example: `AG_Systems`, `Assegai`, `Auricom`, `EGX`,
`Feisar`, `Goteki`, `Harimau`, `Icaras`, `Piranha`, `Qirex` and `Triakis` each
have the entry immediately before their `Ship.vex` decode as a texture set
with *exactly* as many entries as the model has `Texture` nodes. Eleven
independent hits at delta -1 rules out coincidence - a wrong or accidental
adjacency would show some teams fitting and their neighbours not, not all
eleven fitting exactly. `Mirage` and `Van_Uber` were not found under those
names in `WADS2.WAD` and are not covered. `Mirage` was later found under the
folder `Mantis`; `Van_Uber` is Pure DLC, not on any disc this project reads -
see [pure-status.md](pure-status.md#van_uber-is-pure-content-not-a-pure-team-on-this-disc).

**This is not a ship-specific finding after all - it also holds for tracks,
checked separately.** The naive version of this rule - pairing by directory
adjacency at all - was tried and rejected earlier at confidence too low to
act on: the nearest preceding texture set matched entry counts for only 535
of the 975 models with `Texture` nodes disc-wide. That earlier check was
disc-wide, over every model regardless of kind (ships, ship parts, weapons,
tracks, front-end props all pooled together), and a texture-set-per-model
count match failing for most of that pool does not mean it fails for large,
single-owner models like a track's - it means most of the pool is small
shared-atlas geometry that a per-model rule was never going to fit.

Checked directly: every `Data\Environments\<n>_Track\track.vex` and
`track_reversed.vex` on the PS2 disc (all 16 circuits, both directions, 32
models) against the entry directly preceding it. **27 of 32 match exactly**
(same entry count as `Texture` nodes); the other 5 (`02_Track`,
`02_Track_reversed`, `06_Track_reversed`, `12_Track`, `12_Track_reversed`)
are short by only 1 or 2 slots, never more, and `oag_game::race::load` already
leaves a texture ordinal beyond a short set's length untextured rather than
misaligning the rest - see `mesh::build_with_textures`'s doc comment. Same
rule (`sub_entries == texture_nodes` at delta exactly **-1**), same
`read_preceding`/`ps2_texture_set` call `oag_game::race::load` already made
for ships, now also made for the track model when its embedded slots are all
empty. **Confidence 88**: the same kind of exact-count data agreement the
ship finding scored 90 on, one point short because 5 of 32 are near misses
rather than exact and the reason for the shortfall (a handful of `Texture`
nodes per track that reference no unique pixel data, a merge during
packing, something else) is not identified. **`12_Track`'s shortfall used to
be visible, not just an untested edge case**: under the old ordinal scheme,
its `Skycube` named the ordinal that fell off the short set's end, so one
whole face of its sky drew the untextured white 1x1 - see
[`skycube.md`](skycube.md#what-this-engine-does-with-it). The other four
near-miss circuits' missing ordinals happened to fall outside what their sky
and pads actually referenced, so they drew fully textured even under the old
scheme, despite the same short set. All five draw fully textured now
regardless: the shortfall is still real (the nested set genuinely holds one
entry fewer than the model has `Texture` nodes), but a by-name resolver never
consults the set's length at all, so a duplicate name simply resolves twice
to the one entry it always named - see [the original never suffers this
collapse](#the-original-never-suffers-this-collapse-it-resolves-every-texture-by-name-not-by-ordinal)
below.

Track and environment models made of many small shared-atlas pieces (crates,
decorations, ship-part-sized props) are the ones the wider, ungrouped 535/975
figure says this rule will not fit - that piece is still open and is a
separate, smaller RE task from "does a circuit's own `track.vex` get its
textures back," which this section now answers.

### The near-miss shortfall is a duplicated `Texture` node name, not missing data

**Confidence 88, measured 2026-09-05.** The reason the five near-miss circuits'
resolved sets are short by exactly 1 or 2 entries: **one or two of their
`Texture` nodes share an identical declared name with an earlier node in the
same file, and the nested texture-set WAD holds one physical entry per unique
name, not one per node.** `sort | uniq -d` over `12_Track`'s 152 `Texture`
node names turns up exactly one repeat -
`Data\Environments\Tex_Animations\col_display7_GLOW.tga`, at 0-indexed node
positions 55 and 63 - and 152 nodes minus that one duplicate is 151, the
nested set's exact entry count. Checked on all five near-miss circuits, same
method:

| Circuit | `Texture` nodes | Duplicate names found | Previously-measured shortfall |
| --- | ---: | --- | --- |
| `02_Track` | 148 | 2 | 2 |
| `02_Track_reversed` | 148 | 1 | 1 |
| `06_Track_reversed` | 166 | 2 | 2 |
| `12_Track` | 152 | 1 | 1 |
| `12_Track_reversed` | 153 | 1 | 1 |

Every circuit's duplicate count matches its shortfall exactly. None of the 27
exact-match circuits were re-checked for duplicates (not needed - their count
already matches with none unaccounted for), but the arithmetic closing on all
five outliers with no exceptions is the same kind of agreement the ship and
track counts themselves were scored on.

**The duplicate's own real pixel data was traced independently**, three ways,
all agreeing: `12_Track`'s repeated node (`col_display7_GLOW.tga`) aside,
every one of its 152 `Texture` node names was rewritten `.tga` -> `.pct` (the
already-95-confidence extension rule above) and looked up by name-hash
against the *outer* archive, and the fetched bytes' sha256 was matched
against all 151 extracted nested-set entries:

- Node ordinals **0-62**: name-hash lookup lands on set index == node ordinal
  (identity, no shift yet).
- Node ordinal **63** (the duplicate's second occurrence): resolves to set
  index **55** - the same physical entry as its own first occurrence at node
  55, which independently resolves to set index 55 too. This *is* the
  collapse: the packer wrote one entry for the name, and both node ordinals
  that declare it point at it.
- Node ordinals **64-151**: every one resolves to set index `ordinal - 1`,
  with no exceptions, ending at node 151 (`sky12_4.tga`) -> set index 150 -
  the last physical entry of the 151-entry set.
- A second, unrelated pair inside the same file corroborates the shift point
  independently: node 24 (`viewing_tower006_GLOW.tga`) and node 113
  (`viewing_tower006.tga` - a different declared name, authored twice under
  two names rather than a literal duplicate) are byte-identical content.
  Node 24 sits before the collapse and resolves to set index 24 (identity);
  node 113 sits after it and resolves to set index 112 (`ordinal - 1`) -
  exactly what the col_display7 collapse point predicts, from data that has
  nothing to do with it.

So `sky12_4.tga`'s pixel data is not absent from the disc at all: it is
byte-identical to nested-set entry 150 (the set's own last entry, which
decodes fine) **and** to a standalone top-level `WADS2.WAD` entry reachable
under its own declared name via the extension-rewrite rule, with negative
controls (`sky12_4.tga` unrewritten, and a bogus `sky12_99.pct`) both erroring
honestly rather than silently matching something else.

**This is a real, source-confirmed consequence for the current renderer, not
speculation about the original.** `mesh::build_with_textures`
(`crates/mesh/src/mesh.rs`) resizes a short external set with
`set.resize_with(set.len().max(slots), || None)` - appending `None` at the
*end* so a short set's tail goes untextured rather than shifting anything -
which is a flat "node ordinal `N` reads external-set entry `N`" scheme with no
awareness that the set's own entries were built by deduplicating names. Under
that scheme every one of `12_Track`'s node ordinals from 64 to 150 binds the
*wrong* decoded texture (a real, validly-decoded entry - just its neighbour's,
one slot early) rather than nothing, and only ordinal 151 - past the end even
after padding - falls back to the white 1x1. The white sky face is the one
visible symptom of a wider mis-binding, not an isolated one-off.

**Confidence 88** for the root cause and the exact ordinal-to-index mapping:
this is data agreement (152 independently-resolved node-to-content matches
with zero exceptions, corroborated by an unrelated second duplicate pair
inside the same file), the same tier the directory-position finding itself
carries, and is capped below 94 for the same reason - it is not a runtime
trace.

### The original never suffers this collapse: it resolves every texture by name, not by ordinal

**Confidence 88, measured 2026-09-05.** The question the section above left
open - whether the original PS2 loader is dedup-aware or shares this
project's own flat ordinal bug - resolves to **dedup-aware, and for a
stronger reason than "aware": the original never indexes a texture set by
ordinal position at all.**

`Texture_FindOrLoad` (`0x0010c1e0`, [`texture-names.md`](../ghidra/functions/ps2-pulse-eu/texture-names.md))
is the **only** texture-resolution primitive anywhere in `SCES_547.48` - a
`search_functions` sweep for `Texture_` and `Wad_` turns up nothing else that
returns a texture handle - and its signature leaves no room for an ordinal: it
`strcpy`s its argument as a C string, rewrites `.TGA`/`.MIP` to `.PCT`, hashes
the result with `Wad_HashNameString`, and walks a hash-keyed cache/BST
(`g_texture_cache`, confidence 75 for the container). There is no code path in
this function, or in any of its ten static call sites, that takes a bare
integer and returns a texture.

**A model's own per-node material resolution was traced one level deeper to
close the gap between "the primitive is name-based" and "the model loader
actually uses it that way".** `FUN_001c73d0` (a by-name resource loader used
for particle effects and referenced from ship/HUD code) calls `Resource_Load`
by name, then `FUN_001c7550` on the freshly-loaded resource: a **recursive
walk of the model's own node tree** that calls `Texture_FindOrLoad` on a field
each node carries at a fixed offset, for both the node's own material and a
linked list of sub-items hanging off it. That field is populated by
`FUN_001cc1d0`, a generic file-offset-to-runtime-pointer fixup pass run once
after load (turns every stored offset in a table into `base + offset`,
skipping a `-1` sentinel) - exactly the mechanism that would turn a `Texture`
node's own inline declared-name string into a live pointer, which is what
`Texture_FindOrLoad`'s `strcpy` then requires. This confirms name-based
resolution end to end for **one** model-loading pipeline.

**The track loader is a separate pipeline, and was not traced to the same
depth.** `%s\%strack%s.vex` (the track path format string) is built and
loaded by `FUN_00145900` through `FUN_001debe8`, which calls the generic
`Resource_Load` and then dispatches into a per-class node constructor through
a virtual table (`(**(code**)(class_vtable+0x74))(...)`) rather than through
`FUN_001c73d0`'s linear walk - the same per-class dispatch shape this
project's PSP corpus already documents for `.vex` node classes. The specific
constructor that resolves a **track mesh's** own materials was not located:
`FUN_001c7550` has no caller besides `FUN_001c73d0`, so it is not literally
what runs for a track. What carries the finding across to tracks anyway is an
absence-of-evidence argument, stated as one rather than dressed up as a
trace: `Texture_FindOrLoad` is the *only* texture-loading function in the
binary, full stop, so whatever a track's per-class Mesh/Skycube constructor
does to resolve a material, it has nothing else to call.

**Independent, disc-data-only corroboration, needing no code reading at
all.** If the nested per-model texture-set WAD's own entries are keyed by the
same name hash the outer archive uses (which the format - "the same 8-byte
header and 16-byte entries" - implies but the earlier dedup finding never
checked), then a name-based resolver never even needs to leave the nested
set to get the collapse right, regardless of which function does it. Checked
directly: for every one of the exact-match control (`01_Track`, 133 nodes,
133 entries) and all five near-miss circuits, each `Texture` node's own
declared name, stripped of its `Wipeout PSP\PS2\` build prefix (the same
strip `Texture_FindOrLoad` performs) and rewritten `.tga`->`.pct`, hashes via
`wad::hash_name` to an entry **already present** in that model's own nested
set, keyed by that entry's own `name_hash` field:

| Circuit | `Texture` nodes | Nested set entries | Resolved by own name-hash |
| --- | ---: | ---: | ---: |
| `01_Track` (control) | 133 | 133 | 133 / 133 |
| `02_Track` | 148 | 146 | 148 / 148 |
| `02_Track_reversed` | 148 | 147 | 148 / 148 |
| `06_Track_reversed` | 166 | 164 | 166 / 166 |
| `12_Track` | 152 | 151 | 152 / 152 |
| `12_Track_reversed` | 153 | 152 | 153 / 153 |

856 node-to-entry hash matches, **zero exceptions**, and the duplicate-name
nodes on the five near-miss circuits resolve to the *same* physical entry as
their first occurrence rather than failing to resolve - which is correct,
since they declare the same name. This needs no assumption about which
function does the resolving: it says the nested WAD's own directory already
carries everything a by-name lookup needs, so *any* by-name consumer - the
one this project traced, or one that was missed - gets every node right
without an ordinal ever entering the picture. Confidence **94** for this half
alone (pure data agreement, exact, reproducible offline with the disc image
and no Ghidra project).

**Combined confidence: 88.** The blend is bounded by the untraced link (the
specific call site inside a track's own Mesh-class constructor) rather than
by either half above; both of those individually clear 90. **This meant
`oag-render`'s old flat ordinal scheme in `mesh::build_with_textures` (and
`mesh::ps2_texture_set`, which already parsed each entry's `name_hash` and
discarded it on a debug label) was not a faithful reproduction of an original
bug. It was simply the wrong resolution mechanism**, and fixing it carried no
risk of trading a known-white face for an unverified "corrected" one: the
original never had an ordinal to get right or wrong in the first place.

**Fixed 2026-09-05.** `mesh::ps2_texture_set` is now `Ps2TextureSet`, keyed by
each entry's own `name_hash` rather than by directory position, and
`mesh::build_class` resolves each `Texture`-class node against it by the
node's own declared name (`mesh::resolve_texture_slots`). `TextureSlots`
itself is unaffected - still one `Vec<Option<Arc<ModelTexture>>>` entry per
node in node order - only how a slot gets filled changed, which is why the
ripple into `oag-game`'s call sites (`race/assets.rs`, `race/load.rs`,
`livery.rs`) was mechanical rather than a second design decision.
`zone_grade.rs`'s and `mesh/rcs/skin.rs`'s own `TextureSlots` usage is
unrelated to `Texture` nodes at all and was not touched. See
`crates/render/tests/ps2_texture_binding_ground_truth.rs`, which asserts the
27 exact-match circuits reproduce the old positional answer exactly,
`12_Track` fills its previously-white sky slot and rebinds all 87 of its
previously mis-bound ordinals, and the twelve-team ship roster - which never
showed a duplicate name - is a byte-for-byte no-op.

**Two things this fix did not settle, both still open**: the exact call site
inside a track's own Mesh-class constructor that resolves its materials was
never located (see the untraced-link paragraph above, which is what caps the
combined confidence at 88 rather than raising it); and whether the outer
archive ever needs consulting for a track's own textures, or the nested set
alone is always sufficient, was checked only on the exact-match control and
the five near-miss circuits, not the other 26 - untested whether some
`Texture` node on some other circuit declares a name that misses its own
nested set and needs the fallback
[`oag_pulse::read_image`](../../crates/pulse/src/lib.rs)'s ship/extension-
rewrite path already established. Neither blocks the fix above: both are
about raising an already-actionable 88 further, not about whether the
mechanism is right.

**A third model type, and the first checked exhaustively: the boost plume**
(2026-08-23, confidence 90). Every one of the PS2 disc's 24 plume files -
`Data\Ships\<Team>\shipboost.vex` and `Zoneboost.vex`, twelve teams each -
declares exactly two `Texture` nodes, embeds neither, and is preceded at delta
-1 by a set that decodes **exactly two** entries. 24 of 24 exact, with no near
misses at all, which is the cleanest agreement any of the three model types has
produced; the ship finding's eleven and the track finding's 27-of-32 both
predate it. `oag_livery::ps2_skin` is the call site, on the same
all-slots-empty gate the hull takes, and
`crates/game/tests/boost_plume_ground_truth.rs` asserts both halves for every
file - that the preceding entry decodes, *and* that rebuilding through it
leaves no slot empty. The second half is the one that matters as a regression
test: a set shorter than the slot count would pass the first and draw a plume
half-skinned rather than visibly broken.

Unlike the hull, the plume also names its textures per team
(`Data\Ships\<Team>\Textures\Lights_GLOW.tga` and `Engine_GLOW.tga`) rather
than sharing one `Data\Tex\` file the way the PSP model does - so the PS2
build carries twelve plume skins where the PSP build carries one. That is a
fact about the two authorings, not about the lookup rule, but it is why the
plume could not simply borrow the PSP path's single embedded texture.

**A fourth: the shield shell and the cockpit sphere** (2026-09-24, confidence
90; the shell's file corrected 2026-10-01). All twelve teams' PS2
`Data\Ships\<Team>\extrashield.vex` (the file the PS2 executable names - the
2026-09-24 walk was over `shipshield.vex`, which it does not) and
`Data\Weapons\vr_shield_cockpit.vex` each declare one `Texture` node, embed
nothing, and are preceded by a set that decodes exactly one entry. That is 13
of 13 exact. The shell's one texture is `pulse_shield_extra_ADD`, 128x64
`PSMT8` (15 colours of 256); the game uploads its bytes to the GS unchanged
(checked byte for byte against EE RAM and a GS dump), and its `CLUT` is stored
`CSM1`-swizzled (palette entry `i` at
`(i & ~0x18) | ((i & 8) << 1) | ((i & 0x10) >> 1)`), which the decoder's
`unswizzle_clut` already undoes.
`oag_livery::shield::shield_model` now calls `ps2_skin` on the same
all-slots-empty gate. Before that, both models bound the white 1x1, which is
half of why the PS2 shell drew as a flat pale dome.
`crates/game/tests/ps2_shield_ground_truth.rs` walks all thirteen the way the
plume check walks its 24, and also asserts the eight shells a race fields
come out of the load skinned. The other half of that report was the shell's
blend, not its texture - and then the file itself. See
[shield-pickup.md](../ghidra/functions/ps2-pulse-eu/shield-pickup.md#the-model-is-extrashieldvex-not-shipshieldvex-gs-dump-2026-10-01).

## A standalone texture is under its declared name with the extension rewritten

The PS2 build shares its XML, its models and its authored asset paths with the
PSP build. What it does not share is the compiled texture container, so its
texture resolver **replaces the source art's extension before it hashes**:
`.TGA` and then `.MIP` become `.PCT`. One declared name therefore serves both
pressings.

| Declared | PSP entry | PS2 entry |
| --- | --- | --- |
| `Data\Tex\engineFlare\Engine_noise.mip` | `008d70a2` | `e9f16c12` (`....pct`) |
| `Data\HUD\Textures\PulseHUD.mip` | `57d37d8c` | `beaf613c` (`....pct`) |

Of the 50 `.mip` and `.tga` literals in `SCES_547.48`, **0 resolve on the PS2
disc as spelled and 47 resolve rewritten**; the three that do not are an
absolute authoring path off a build machine and two extension-only stubs. Every
other class of asset name - `.pob`, `.vex`, `.bnk`, `.xml` - resolves on both
discs unrewritten. `Data/Tex/Missing.pct`, the placeholder the resolver falls
back to when a name reaches no file, is itself entry 6910 of `WADS2.WAD`.

**Confidence 95.** The rewrite is read off `Texture_FindOrLoad` (`0x0010c1e0`)
directly - the five string constants are consecutive in `.rodata` - and it is
corroborated by the section below, whose three entries were recovered by an
unrelated method and are reproduced exactly by the rule. Implemented as
[`oag_pulse::ps2_texture_name`](../../crates/pulse/src/lib.rs); see
[`texture-names.md`](../ghidra/functions/ps2-pulse-eu/texture-names.md).

### How this was missed for so long, and the sweep that missed it

Worth recording, because the failure is reusable. The PS2's own screens ask for
`Data\HUD\Textures\PulseHUD.mip` and `Data\FE\Images\pulse_logo.mip`, and
neither hashes to an entry on the disc. Nor did any variation tried at the
time: 60 candidates across path shape, case and extension were hashed against
all 7,393 entry hashes in `WADS2.WAD` and `WADSP.WAD` and every one missed -
**`.pct` was not among the extensions tried**. Directory position does not help
either; the five `Data\XML\*_HUD.xml` entries sit in a run of XML with no
texture near them. So the entries below were found by their pictures instead,
and this page recorded the lookup as unknown.

The general lesson: when a *whole class* of names misses and every other class
hits, the answer is a transformation the loader applies, not more spellings.
Meanwhile [`loading-screen.md`](../ghidra/functions/ps2-pulse-eu/loading-screen.md)
had already recorded the `.mip`-to-`.pct` rewrite for the loading screen alone,
and it read as a quirk of that screen rather than as the general rule it is.

The entries were found **by their pictures**: decode the PSP `.mip`, reduce it
to a silhouette (one bit per pixel, "is this texel's palette entry
transparent"), and compare against every same-shaped PS2 texture on the disc.
The measure ignores palette order, which the two builds do differently.

| PSP name | PS2 entry | Hash | Silhouette | Best runner-up |
| --- | ---: | --- | ---: | ---: |
| `Data\HUD\Textures\PulseHUD.mip` | 3518, 3583 | `beaf613c`, `f012b5af` | 0.9999 | 0.7085 |
| `Data\FE\Images\pulse_logo.mip` | 3421 | `1e6c873e` | 0.9946 | 0.5149 |
| `Data\FE\Images\pulse_assets.mip` | 3420 | `0d31af1b` | 0.9999 | 0.5147 |

All three were then decoded and looked at: 3518 is the speed and shield bars
with the weapon icons, 3421 is the Wipeout Pulse wordmark. The two `PulseHUD`
entries are a genuine duplicate, not an ambiguity - the PSP ships that atlas
twice as well, and both PS2 copies are 13,446 stored / 66,829 unpacked. For the
HUD atlas there is a fourth, independent agreement: the UV boxes in the PS2
layouts are identical to the PSP's, so both index the same arrangement.

`Data\FE\Images\gameshare_backdrop.mip` is **confirmed absent**, which fits -
Game Sharing is a PSP ad-hoc feature. Note the silhouette test cannot say so:
that image is 93.75 % opaque, so every candidate scores 0.9375 for free.
Correlating the picture settles it at 0.02 across all 28 same-shaped
candidates. Do not re-run the weaker test on it.

A repeated 3,656-byte blob sitting beside the HUD assets looked like a name
table and was ruled out along the way: none of its 914 words is an entry hash in
either archive.

**All three hashes fall out of the extension rewrite above.**
`Data\HUD\Textures\PulseHUD.pct` hashes to `beaf613c`, `pulse_logo.pct` to
`1e6c873e`, `pulse_assets.pct` to `0d31af1b` - the picture-matched entries,
exactly. Two methods sharing no assumption reaching the same three entries is
what raises the rewrite to **95** and these three mappings with it. The table
stays in
[`oag_pulse::PS2_IMAGES`](../../crates/pulse/src/lib.rs) as that check, run
offline with no disc; nothing loads through it any more. It is also still
pinned by `crates/assets/tests/ps2_image_ground_truth.rs`, which re-derives the
match instead of asserting the constants against themselves.

## A ship's paintable surface is one atlas on PS2, four slots on PSP

The PSP build's alternate-livery `.dat` files (`ship_alt.dat`,
`ship_eliminator.dat` - see
[`ship-skin.md`](../ghidra/functions/psp-pulse-usa/ship-skin.md)) repaint four
128x128 texture slots a model names `texture1.tga`..`texture4.tga`. A PS2
`Ship.vex` names no such slot at all: `AG_Systems`, for example, declares
`ALL_Textures.tga` (256x256) and seven other, differently-shaped textures, and
the nested texture set it resolves against (the entry immediately before
`Ship.vex` in `WADS2.WAD`) carries no `texture1.tga`..`texture4.tga` entry
whatsoever - confirmed by resolving all four names against the set directly,
not merely by their absence from a node's own declared name.

**The PS2 port has its own skin mechanism, and it is not a block upload -
confidence 88.** `Skin_SwapAtlasSibling` (`0x001dfb70` in `SCES_547.48`, found
from the only xref to `\ALL_TEXTURES.TGA`) consults only the skin file's
**name**: `ship_alt.dat` loads `livery.pct` from the atlas's own directory,
`ship_eliminator.dat` loads `<stem>_eliminator.pct`, and the whole sibling is
`memcpy`d over the hull's atlas at the atlas's own size. The `.dat`'s bytes are
never read on PS2, even though the disc still carries them in the PSP layout.
There is no quadrant packing to recover. Every sibling exists for all twelve
PS2 teams, each 256x256 and the same 66,829 bytes as the atlas, and
AG_Systems' `livery.pct` is the same cyan livery the PSP's `ship_alt.dat`
paints. Full reading, the per-player atlas copies `Texture_LoadVexNode`
makes, and the untraced `ship.dat` baseline restore in
[`ps2-pulse-eu/ship-skin.md`](../ghidra/functions/ps2-pulse-eu/ship-skin.md);
implemented by `oag_mesh::mesh::ship_skin::apply_ps2_atlas` and
[`oag_livery::ship_skin`](../../crates/livery/src/ship_skin.rs),
pinned by `crates/game/tests/ps2_ship_skin_ground_truth.rs`.

## Not determined
- **`flags` at `+0x02`.** 0x2000 on 3,925 textures and 0x2040 on 1,423. Nothing
  correlates it with dimensions or depth.
- **`+0x08` and `+0x0c`.** They move together — `0x00400000` with 68,
  `0x00100000` with 20, `0x01000000` with 4 — and look like a GS memory
  allocation, which the file would have no reason to fix. Not needed to decode.
  A wider sweep (403 textures across `01_Track`, `02_Track`, `05_Track`, run
  while investigating a mirrored-speed-pad texture pair) measured both fields
  against dimensions directly rather than against each other: `+0x0c` takes 9
  distinct values and tracks size closely
  (`0x04` at 256x256, `0x14` at 64x64, `0x0c` at both 64x32 and 128x16, `0x08`
  at 32x32, `0x05` at both 8x8 and 16x16), and `+0x08` is a single bit walking
  alongside it across five observed values (`0x01000000`, `0x00100000`,
  `0x00080000`, `0x00040000`, `0x00010000`). Read together this looks like a
  size class (`+0x0c`) paired with a log2-derived allocation hint (`+0x08`) -
  plausible given the "GS memory allocation" reading above, but a correlation
  across files, not a formula checked against the GS's own allocation rules or
  a runtime trace. Confidence 55: consistent across a few hundred real files,
  meaning not verified. Still not needed to decode.
- **The eight quadwords at `+0x0d`.** GS state the game replays; `TEX0`-shaped
  values are in there but nothing depends on reading them.
- **The `Texture` node payloads in a PS2 `.vex` are stale.** They still carry
  PSP-shaped `clut_size`/`texel_size` including mip chains, and disagree with the
  actual texture: the Feisar node 0 says 512x512 8bpp with 6 mips, the entry the
  game loads is 256x256 with one level. The PS2 blobs carry **no mips at all** —
  the size formula would not close if they did.
- **Two outliers.** Entries `0xb077d3bd` and `0x8d17fa0d` of `WADS2.WAD` start
  with a valid 256x256 texture packet and then continue with more GIF packets to
  561,667 and 1,123,333 bytes. Some multi-part resource with a texture on the
  front; refused rather than half-decoded.
