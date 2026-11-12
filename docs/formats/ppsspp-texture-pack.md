# PPSSPP texture packs

**Status: hash understood and measured; wired for Pulse's `.vex` and whole-image `.mip` textures as an off-by-default mod ("Wired" below).** A PPSSPP texture pack
replaces a texture by the **key PPSSPP computes from the texture's bytes in guest
memory**, before decode. This page is the key, its evidence, and how many keys of
the two Wipeout packs this engine reproduces from the bytes it loads off the disc.
Reader: [`oag_texture::replacement`](../../crates/texture/src/replacement.rs)
(hash and key), [`oag_texture::replacement_pack`](../../crates/texture/src/replacement_pack.rs)
(`textures.ini`). Test:
[`crates/texture/tests/ppsspp_pack_ground_truth.rs`](../../crates/texture/tests/ppsspp_pack_ground_truth.rs).
Fetching them: [texture-packs.md](../tools/texture-packs.md).

Three packs were measured, all unofficial and unlicensed fan work, none committed
(`data/texture-packs/`, gitignored). The **2026 pack is the primary target**; the
two `reTokyo` repositories are early snapshots of the same work:

| Pack | Source | Pinned | Serials in `[games]` |
| --- | --- | --- | --- |
| **Pure + Pulse 2026-07-25** | NR74W's `WipEout Mods` download, `Texture Packs/WipEout Pure-Pulse - PPSSPP Pack (2026-07-25)/textures.zip` (553 MB, one `textures.zip` for both games; not fetchable by script) | its file name | Pure: UCES00001, UCUS98612, UCJS10007, UCKS45008, ULES00229, ULUS10058, UCED00017, UCET00001. Pulse: UCES00465, UCUS98712, UCAS40179, UCET00713. WPCE02025 (the patched Portable Collection) |
| Pulse (early snapshot) | `reTokyo/WipEout-Pulse-Upscale-Version`, 49 commits 2020-03-31 to 2020-06-10 | `33fd6a3f1097` | UCES00465, UCUS98712 (the forum thread adds UCET00713, UCAS40179) |
| Pure (early snapshot) | `reTokyo/WipEout-Pure-HD-Texture-Pack`, ends 2020-04-13 "Update #5" | `f91d55bbde51` | UCES00001, UCUS98612, UCJS10007, ULES00229, ULUS10058 |

The 2026 ini reuses the snapshots' keys: 2,359 of the Pulse repo's 2,377 live keys and
869 of the Pure repo's 895 are in it, so the two share one hash scheme and the same
oracle covers all three. The forum pack kept growing after the snapshots (hand-remade at
8x to 16x in menus and 2x to 8x in game, AI upscaling only for skyboxes), so a
replacement's size is **not a fixed multiple**: read each PNG's own size.

## `textures.ini` grammar both packs use

Read from PPSSPP v1.20.4 `GPU/Common/TextureReplacer.cpp` (`LoadIniValues`) and
checked against both files (`rg '^\['` finds only these three sections in each,
so `[hashranges]` and `[reducehashranges]`, which would change what is hashed,
are absent; `Pack::other_sections` is asserted empty).

```ini
[options]
version = 1
hash = xxh64
ignoreAddress = true
ignoreMipmap = true        # Pulse only; Pure omits it

[games]
UCES00465 = textures.ini   # serial = ini file; the same file here

[hashes]
000000007d9d618f1a5be898 = intro/sce/scee_presents.png
0000000000000909 =                                  # blank value
0000000000000000cd0aa2c1_1 =                        # mip level 1, blank
```

- A `[hashes]` key is PPSSPP's `sscanf("%16llx%8x_%d")`: 16 hex digits of
  **cache key**, optionally 8 of **data hash**, optionally `_<mip level>`. The
  `Line` type holds exactly those three fields and the target.
- **An empty value is not an error: it marks a texture PPSSPP must leave alone**
  (and not re-dump). `0000000000000909 =` is a cache-key-only wildcard: it
  blacklists every texture whose low cache-key word is `0x0909` (a 512x512
  non-palette texture, a render target). Pulse has that one line, 14 blank
  full keys; Pure has 11 blank full keys and 17 blank `_1`..`_3` level lines.
- Many keys name one PNG (at most 3 in Pulse, 7 in Pure), and **the pack does not
  ship every PNG it names**: Pulse's 2,380 live keys name 2,268 distinct files
  and 593 of them are not in the repo; Pure's 896 name 822 and 692 are absent
  (the repo holds 130 PNGs: it describes itself as "a basic ... pack which
  enhances the User-Interface"). A key whose PNG is missing replaces nothing.
- Pulse has 245 live keys whose cache-key half is zero
  (`0000000000000000dddddddd`): a **data-hash wildcard**, PPSSPP's last-resort
  lookup ("anything with this data hash"). Pure has none. 3 Pulse and 1 Pure
  keys are listed twice.
- **The 2026 pack is one ini for both games** (13 serials, `[options]` as Pulse's with `ignoreMipmap`): 7,664 24-hex level-0 keys, 7,476 live, 188 blank, one 16-hex wildcard, 359 zero-cache-key lines, 6,949 distinct PNG targets of which 2,752 are not in the zip. It is a **`textures.zip`** (4,801 entries, `textures.ini` at its root), which PPSSPP prefers to a folder (`LoadIni` tries the zip first, and refuses a zip without an ini). Top folders: `environments` 2,593 files, `ships` 848, `tex` 627, `skins` 283, `tournaments` 170, `fe` 102, `music` 53, `psys` 45, `defaults` 33, `weapons`, `gallery`, `hud`, `pads`. Its Readme: "Due to dynamic CLUT, the HUD font textures in WipEout Pure are almost impossible to replace reliably"; the default fonts are 4x, 8x ones sit in `fe/fonts/8x_resolution`.
- Lookup order for a texture with cache key `K` (address zeroed) and data hash `H`
  (`LookupWildcard`): `(K, H)`, then `(K, 0)`, then `(0, H)`. So a zero-key line
  matches any palette variant of a texture, and a full key matches only one.

## The key

With `ignoreAddress = true` the dumped file name and the `[hashes]` key are

```text
00000000  LLLLLLLL  DDDDDDDD
address   dim^clut  data hash
```

`HashName` prints `%016llx%08x` of `(cachekey, hash)`; `ignoreAddress` masks the
cache key to its low 32 bits, so the first 8 hex digits are zero.

**`DDDDDDDD`, the data hash (confidence 95).** `ComputeHash` hashes the texture's
bytes at its guest address, level 0 only, **without swizzling or decoding**:

- `XXH64(bytes, seed 0xBACD7814)` truncated to its **low 32 bits** (the `u64` is
  returned as `u32`).
- If `bufw <= width` (the buffer width in pixels): the span is
  `bits_per_pixel * (bufw * height + (width - bufw)) / 8` bytes, one hash.
- If `bufw > width` (a narrow texture in a wider buffer): the hash is the fold
  `result = result * 11 ^ XXH64(row, seed)` (as `u32`) over `height` rows of
  `width * bpp / 8` bytes at a stride of `bufw * bpp / 8`.
- `height` is the texture size the game declared (a power of two). A 512-tall
  texture is clamped to the highest V sampled (`maxSeenV`, at least 272) when
  drawn in through mode; `data_hash` takes that as an argument and the census
  passes 0 (no 512-tall palette texture is in either census).
- The swizzle is ignored (PPSSPP's own `TODO`): a swizzled texture is hashed as
  the bytes the GE reads, which is why the input here is the stored layout and
  not a decoded RGBA buffer.

**`LLLLLLLL`, `dim ^ cluthash ^ clutformat` (confidence 90).** For a palette
format (`format & 4`): `CacheKey` is `(addr << 32 | dim) ^ cluthash`, and the
`cluthash` passed in is `clutHash_ ^ gstate.clutformat`.

- `dim = texsize & 0xf0f`: `log2(width) | log2(height) << 8`, **the two exponents,
  not the pixel size** (256x128 is `0x708`).
- `clutHash_ = XXH32(clut bytes, seed 0xC0108888)` while a replacer is enabled
  (`UpdateCurrentClut`, `TextureCache{Vulkan,GLES,...}.cpp`; the non-replacing
  path uses XXH3, which is why a dump made without `[options]` is not comparable).
- `gstate.clutformat` is the **whole register word including its command byte**:
  format in bits 0-1, shift in 2-6, mask in 8-15, start in 16-20, `0xC5` on top.
  Every Wipeout palette is RGBA8888 with no shift, mask `0xff`, start 0, so the
  word is **`0xC500FF03`**. (`isClutIndexSimple` compares against `0xC500FF00`.)
- **How many bytes are hashed (the part that is not in any file).**
  `clutExtendedBytes = min(clutTotalBytes + startBytes, clutMaxBytes)`: the last
  `LoadCLUT` size, capped by the largest ever loaded. **The game always loads 256
  entries**, so the window is **1,024 bytes from the palette's first byte**. For
  an 8-bit texture that is the palette itself. For a **4-bit texture it is the 64
  palette bytes plus the 960 bytes that follow them in RAM**, which is the
  texture's own texel block when the loader leaves palette and texels adjacent
  and the block is long enough, and **unrelated heap otherwise** (below).

A texture with no palette (`format & 4 == 0`) has `LLLLLLLL = dim` alone; neither
pack's live keys include one except a handful of 512x512 render targets.

## Where the bytes are in RAM

What the hash sees is what the loader leaves in memory, so the input is the file
read the way `Texture_BindEmbeddedData` reads it
(`docs/formats/psp-texture.md`, ".mip" and `.vex` flag rules):

- **Flagged textures stay as stored; an unflagged one is swizzled at load, each
  mip level on its own at its own stride** (`oag_formats::swizzle::swizzle`, the
  inverse of the unswizzle every reader here already does). A flagged `.fnt` atlas
  is hashed as stored.
- Palette then texels are one allocation, in that order; a `.vex` block's rows
  are padded to 16 bytes, a standalone `.mip`'s level 0 is not.
- `bufw` in pixels is the 16-byte-padded stride times eight over the bit depth.

`Stored::replacement` does this and returns `Replacement { dim, data, key }`,
`key` being `None` exactly when palette and texels together are under 1,024
bytes (the window leaves the block).

## What was loaded: the census

One pass over every WAD of the disc, then the DLC archives of the title (Pulse's
four `.zip` packs, 16 archives; Pure's seven, 8 archives, decrypted with `data/keys/pure-dlc-keys.txt`). A `.vex` is read with
`oag_vex::vex::stored_textures` (new: the palette and texel bytes as the file
holds them, which `textures()` now decodes from).

| Image | `.mip` | `.fnt` atlases | `.vex` `Texture` nodes | keyed (window inside the block) | ship skins `.dat` (4 blocks each) | DLC archives |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `pulse-psp-eu` | 491 | 10 | 7,132 | 6,169 of 7,633 | 96 | 16 |
| `pulse-psp-usa` | 491 | 10 | 7,132 | 6,169 of 7,633 | 96 | 16 |
| `pure-psp-eu` | 619 | 10 | 3,944 | 4,149 of 4,573 | 0 | 8 |
| `pure-psp-usa` | 613 | 10 | 3,944 | 4,143 of 4,567 | 0 | 8 |

(`.mip` and `.vex` counts include the DLC archives; the disc alone is 435 and 5,375
on Pulse, 367 and 2,091 on Pure EU. 399 of Pulse's `.vex` files hold textures.) No
`.vex` texture node is unreadable. The `.dat` skins are counted, not keyed (below).
Not walked: Pure EU's `UCES00015/GENERAL/*.DRM` (four promo archives), the PNG and
TGA images in the WADs (below), `SHARE.BIN`.

### The packs' keys against it

Level-0, non-blank 24-hex keys: Pulse **2,380**, Pure **896**. Classes (the test's
`Class`):

- **exact**: the key is one a texture of ours produces;
- **wildcard**: a zero-cache-key line whose data hash is ours;
- **heap-tail**: our data hash matches but the palette window leaves the block,
  so the key cannot be known from the file;
- **other**: our data hash matches, the window is inside the block, the key
  still differs (a runtime palette);
- **no-data**: no texture of ours has this data hash.

**Pulse** (`pulse-psp-eu` = `pulse-psp-usa`: every count identical, which also
says the two pressings share the textures these keys name):

| Pack directory | keys | exact | wildcard | heap-tail | other | no-data |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `font` | 5 | 3 | 2 | 0 | 0 | 0 |
| `in_game` (`hud.png`) | 1 | 1 | 0 | 0 | 0 | 0 |
| `in_game/adverts` | 107 | 105 | 1 | 1 | 0 | 0 |
| `in_game/effects` | 30 | 9 | 4 | 1 | 0 | 16 |
| `in_game/ships` | 446 | 276 | 2 | 0 | 1 | 167 |
| `in_game/tracks` | 1,619 | 1,376 | 235 | 0 | 0 | 8 |
| `intro` | 7 | 2 | 0 | 0 | 0 | 5 |
| `main_menu` | 165 | 164 | 1 | 0 | 0 | 0 |
| **total** | **2,380** | **1,936** | **245** | **2** | **1** | **196** |

**N = 2,181 of 2,380 Pulse keys (91.6 %) are reproduced from the disc bytes**
(1,936 exact, 245 wildcard), 2,184 by data hash alone. Without the four DLC
packs the same count is 1,513 + 194 = 1,707: the four DLC circuits
(`13_edgewinter`, `14_vostok_reef`, `15_gemini_dam`, `16_orcus`) and four teams
are in the pack but not on the disc. Of the 2,181, **1,592 name a PNG the pack
contains**; 589 do not.

**Pure** (`pure-psp-eu` = `pure-psp-usa`):

| Pack directory | keys | exact | wildcard | heap-tail | other | no-data |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `font` | 26 | 2 | 0 | 0 | 22 | 2 |
| `in_game/adverts` | 103 | 103 | 0 | 0 | 0 | 0 |
| `in_game/effects` | 8 | 2 | 0 | 0 | 0 | 6 |
| `in_game/hud` | 3 | 3 | 0 | 0 | 0 | 0 |
| `in_game/ships` | 165 | 105 | 0 | 0 | 0 | 60 |
| `in_game/tracks` | 79 | 79 | 0 | 0 | 0 | 0 |
| `intro` | 9 | 4 | 0 | 0 | 0 | 5 |
| `main_menu` | 503 | 352 | 0 | 0 | 0 | 151 |
| **total** | **896** | **650** | **0** | **0** | **22** | **224** |

**M = 650 of 896 Pure keys (72.5 %)**, of which **only 110 name a PNG the pack
contains** (the other 540 are keys the pack's author dumped and never filled in).
Without the Pure DLC: 409 exact. Pure's `main_menu` is `previews` (376 keys), `skins`
(63), `music` (24), `class_selection`, `league_selection`, `menu`, `loading`.

### The 2026 pack against Pulse EU and Pure EU

Same census, same classes, the discs plus their DLC (`pulse-psp-eu` and `pure-psp-eu`
together: 1,110 `.mip`, 20 `.fnt`, 11,076 `.vex` textures, 24 DLC archives), against the
one 2026 `textures.ini` (7,476 live keys). Tests `pack_2026_against_pulse_and_pure_eu`,
`..._pulse_eu`, `..._pure_eu`; the zip is found at `OAG_TEXTURE_PACK_ZIP` or
`data/texture-packs/WipEout-Pure-Pulse-PPSSPP-Pack-2026-07-25/textures.zip`.

| Pack directory | keys | exact | wildcard | heap-tail | other | no-data |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `defaults` | 58 | 37 | 0 | 0 | 0 | 21 |
| `environments/NN_* (Pure circuits)` | 2,095 | 1,978 | 96 | 0 | 0 | 21 |
| `environments/classics` | 361 | 109 | 2 | 0 | 17 | 233 |
| `environments/exclusive` | 166 | 32 | 0 | 0 | 2 | 132 |
| `environments/pulse` | 1,791 | 1,525 | 224 | 1 | 4 | 37 |
| `environments/zone` | 91 | 90 | 1 | 0 | 0 | 0 |
| `fe` | 134 | 33 | 3 | 0 | 36 | 62 |
| `gallery` | 373 | 0 | 0 | 0 | 0 | 373 |
| `hud` | 14 | 8 | 1 | 0 | 0 | 5 |
| `music` | 80 | 38 | 0 | 0 | 0 | 42 |
| `pads` | 9 | 9 | 0 | 0 | 0 | 0 |
| `psys` | 80 | 0 | 0 | 0 | 2 | 78 |
| `ships/other` | 415 | 260 | 6 | 0 | 7 | 142 |
| `ships/pulse` | 483 | 274 | 2 | 0 | 1 | 206 |
| `skins` | 297 | 24 | 0 | 0 | 0 | 273 |
| `tex` | 23 | 18 | 1 | 0 | 0 | 4 |
| `tex/caustics` | 16 | 16 | 0 | 0 | 0 | 0 |
| `tex/cloud` | 1 | 1 | 0 | 0 | 0 | 0 |
| `tex/custom` | 431 | 0 | 0 | 0 | 0 | 431 |
| `tex/engineflare` | 3 | 3 | 0 | 0 | 0 | 0 |
| `tex/enginesmoke` | 2 | 0 | 0 | 0 | 0 | 2 |
| `tex/lensflare` | 3 | 3 | 0 | 0 | 0 | 0 |
| `tex/original` | 307 | 175 | 0 | 0 | 0 | 132 |
| `tex/screenfx` | 1 | 1 | 0 | 0 | 0 | 0 |
| `tex/unknown` | 19 | 9 | 2 | 1 | 0 | 7 |
| `tex/weapons` | 5 | 3 | 0 | 0 | 0 | 2 |
| `tournaments` | 176 | 16 | 0 | 0 | 0 | 160 |
| `visual_effects` | 3 | 2 | 1 | 0 | 0 | 0 |
| `weapons` | 39 | 31 | 5 | 0 | 0 | 3 |
| **total** | **7,476** | **4,695** | **344** | **2** | **69** | **2,366** |

**N = 5,039 of 7,476 keys (67.4 %) are reproduced by the two EU discs and their DLC**
(4,695 exact, 344 wildcard); 3,144 of those name a PNG the zip contains, 1,895 do not.
Per game: **Pulse EU alone 2,236 (1,994 + 242), Pure EU alone 2,900 (2,794 + 106)**; the
rest of a game's keys belong to the other game, which is why the union is the headline.

Where it misses, by the pack's own folders (what the table says, no more):

- **Circuits and ships the discs and our DLC do not carry:** `environments/classics`
  (233), `environments/exclusive` (132: Coke, Stealth, Wire, Puma specials),
  `ships/other` (142), `tex/custom` (431, the pack's own alternates), `skins` (273),
  `tournaments` (160), `gallery` (373, truecolour images converted at load, the
  no-data case below), `music` (42).
- **Numbered Pure circuits: 1,978 + 96 of 2,095 reproduced; 21 miss. Pulse circuits
  (`environments/pulse`): 1,525 + 224 of 1,791; 37 miss.** The base games' own tracks
  reproduce almost entirely; a patched-disc re-encode of their textures would show as
  misses *inside* these folders, and it does not.
- `fe` (134 keys): 33 exact, 3 wildcard, **36 other, 62 no-data**: the front-end fonts
  under runtime palettes, as the pack's Readme says, plus alternates.
- `ships/pulse`: 206 no-data, the hull texels rewritten after load (below).
- `psys` (80): 78 no-data, 2 other: particle textures, not found among our sources.

**On the lead's question whether a patched ISO changes the match:** the pack lists
`WPCE02025`, the patched Portable Collection, beside the stock serials, and the Readme
says it matches stock discs. On unpatched EU discs the misses cluster in content the discs
do not hold (other circuits, other ships, custom alternates, truecolour images), not in
the base circuits' folders, so these counts give **no sign** that a patch re-encodes
textures. A patched image was not opened.

## Oracle: PPSSPP's own keys, Pulse EU

PPSSPP v1.20.4 on `pulse-psp-eu.chd`, hardware (OpenGL) renderer under Xvfb (the
software renderer bypasses the texture cache and dumps nothing), with
`[Graphics] SaveNewTextures = True` and a one-section `textures.ini`
(`hash = xxh64`, `ignoreAddress = true`, `ignoreMipmap = true`; without it the
dump uses `quick` and addresses, and is not comparable). The walk covered the logo, the profile prompts, the campaign page, the ship select and about a minute
of Talon's Junction. It wrote **3,411 files**, each named by the key PPSSPP would
look it up by: the oracle for the key, independent of the pack and of this code.
Of the 3,411 dumped keys, 219 are keys of the 2026 ini and 205 of the Pulse snapshot's; the dump holds 248 distinct data hashes, 239 of them in the 2026 ini. Re-run it with `OAG_PPSSPP_DUMP=<TEXTURES/UCES00465/new> just test-data`
(`pulse_eu_oracle`).

| Dumped keys | count | exact in our census | meaning |
| --- | ---: | ---: | --- |
| data hash ours, palette window **inside** the block | 200 | **195** (97.5 %) | the key model |
| data hash ours, window **runs past** the block (4-bit, block under 1,024 bytes) | 3,186 | **0** | heap tail |
| no data hash of ours | 25 | 0 | not our bytes |

A second, picture-level cross-check (scratch, not re-runnable in CI: it needs a
PNG decoder the tests do not carry): each dumped PNG against every texture of the
disc decoded to RGBA. **3,182 of the 3,187 data-hash-matched keys that are not
exact decode to exactly our picture**, as do all 199 exact ones; the keys differ,
the pictures do not.

### Misses: our bytes or a texture the game builds?

1. **Heap tail: our bytes are right, the key is not knowable from them.** A 4-bit
   texture's window reads 960 bytes past its 64-byte palette. For one under 1,024
   bytes in all (palette plus every mip level) that is whatever the allocator put
   next. Sample: the 8x8 4-bit particle texture `04949fa8` (`vex`, 192 bytes)
   produced five keys in one session, `38a67a75`, `af723b65`, `9da4ce89`,
   `e1accc27`, `6b0c85a9`, against our `67ba55ee` (window `pal ‖ block` only); all
   five decode to our picture. 3,186 of 3,411 dumped keys are this, and none is exact. **A key for such a texture is a property of one
   session's heap.** The packs contain only 2 such keys; the zero-cache-key
   wildcard is how a pack makes one robust, and why Pulse has 245 of them.
2. **Runtime palette: our bytes are right, the palette differs per draw.** Pure's
   HUD fonts (`ocr_a_hud`, `fx300_hud`: 22 keys sharing 2 data hashes) are the
   same atlas under many palettes. Pulse's `ships` has 1, `tracks` none.
3. **Ship hull textures: the picture is ours, the RAM bytes are not.** Pulse ship
   `classic`/`concept`/`zone` hull textures (Assegai `Data.wad#627.1/.3/.4`,
   128x128 4-bit, flag byte `0xe4`) dump to keys whose data hash (`31ccdb80`,
   `48812a95`, `4fcd8029`) is not that of our swizzled, as-stored or unswizzled
   level 0 (`1bf1fe04`, `a9a8287a`, `f8d6c6fd`), yet the dumped PNG is pixel for
   pixel the picture we decode. So the loader rewrites the texels after load
   (each of these palettes has a yellow-green first entry, apparently a colour key, and a duplicated last entry). **Unresolved**: collapsing the duplicate, sorting the palette and
   remapping indices do not reproduce it. This is the 167 `ships` no-data keys
   (96 of them the `alternative` and `eliminator` livery keys, which should come from the `.dat` blocks hashed as stored or swizzled and match none; the oracle session raced a classic hull, so these are not dumped).
4. **Decoded at load from an image: not our bytes.** The WADs hold truecolour images (Pulse `Data.wad`: 12 PNGs, 480x272 and 144x80; Pure `Data.wad`: 146 uncompressed 24-bit TGAs, the ones sampled 480x272, and 6 PNGs). The game converts them; Pure's `main_menu/previews` (376 keys, 151 of
   all `main_menu` no-data) and `skins` previews are almost surely these. The key
   shape says they are palette textures (`LLLLLLLL` is not a bare `dim`), so the
   game quantises the image: the algorithm is unrecovered.
5. **No source on disc or DLC:** `intro/sce/copyright_2007_scee.png` (256x32) and
   about twenty effect textures (32x32 and 64x64: `collision_smoke`,
   `ship_debris`, `flare`, `cannon_bolt`...). No texture of ours decodes to the
   dumped picture for 20 of the 25 dumped no-data keys, so they are built at
   runtime or live in a container this census does not read; 5 more decode to our
   picture (the hulls above and two `.mip`s, `Data.wad#981` and `#1018`).

## Wired

A pack is a mod that starts off ([mods.md](../tools/mods.md), "Texture packs"): `oag-mod`
recognises a `textures.ini` folder or zip (or a folder holding `textures.zip`) and
`oag_mod::texture_pack::replacements` indexes it as an `oag_texture::replacer::Replacements`
over the mod's own files. The race load (`oag_raceplay::texture_pack::engage`, which is the one
place that knows the disc serial) makes the first enabled pack that names a PSP disc's serial
the active one, and the decoders ask it:

- **`.vex` textures** (`oag_mesh::mesh::embedded_texture_slots`): each `Texture` node is keyed
  with `Stored::from_vex`; a hit becomes `ModelTexture::replaced`, in the **same form the disc's
  texture took** (a `Texels::Chain` stays a chain, so the model's slope level selection stays
  on for every slot) with the levels the scale adds (a 4x picture gains two), so the deepest
  level is as small on screen as the original's. Lookup is PPSSPP's order, `(K, H)`, `(K, 0)`,
  `(0, H)`; a texture with no reproducible key (4-bit, under 1,024 bytes) is found by `(0, H)`
  alone; a blank line leaves a texture alone.
- **Whole-image `.mip`** (`oag_texture::replacer::mip`): the flare, cloud and mist textures the
  race load reads. Fractions of the picture are what they draw with, so a 4x picture needs no
  change.
- **Level selection.** The slope rule (`texlod_slope`, `crates/mesh/shaders/shade.wesl`) picks a
  level from view depth alone, so a 4x picture is sampled at the same level number as the
  disc's: four times the texels at every distance, and the extra levels a replacement chain gains
  are never reached. No shimmer showed in the frames; kept so a rule change cannot starve a deep level.
- **The race HUD's sheet** (`oag_hud::sprite::Sheet::build_replaced`, which only the race load's
  `load_atlases` calls): a `.mip` the pack has a picture for is stacked into the sheet at the
  picture's own size. `Placed.width`/`height` stay the disc's pixels (the layouts' rects, the
  default sprite size and the mod-icon fit are in them); the per-axis factor, picture over disc,
  rides on the placement itself, `Placed::scale` (`Sheet::scale(src)` reads it), and
  `Placed::sheet_rect` is the one way a layout's `[u, v, w, h]` becomes sheet pixels
  (`sprite_draw`, the sight rings, the now-playing bar, and every front-end draw). It is on
  `Placed` rather than the `Sheet` because `oag-ui`'s draws hold only a lookup returning a `Placed`
  (`oag-hud` depends on `oag-ui`, so they cannot reach a `Sheet`). `alpha_at` and
  `opaque_extent` follow the factor and answer in the disc's pixels as before. Pulse EU's
  `PulseHUD.mip` is 256x256 and both packs draw it at 2048x2048 (8x). A draw scales the sampled
  rect and never the drawn one (a rect derived from a uv's width is the mistake: the stepping
  arrows' glow was a screen-sized block until it took its size from the spec).
- **The front end's sheet** (`oag_game::boot::sprites::load`, `Sheet::build_replaced`): the boot
  keeps the pack engaged through the sheet build as it does for the faces, so the menus' frame,
  the footer capsules, the stepping arrows and the title logo draw the pack's picture at the disc's
  rects (Pulse EU: `pulse_assets.mip` 4096x1024 for the disc's 512x128, `pulse_logo.mip` the same,
  `hex_bg.mip` 128x64 for 32x16; the whole sheet 4096x2372 with the 2020 folder). Pure is held off
  with its faces. The loading screen builds a sheet of its own, not replaced (Pulse's draws no
  sheet picture). Off, the main, race, display and `Show Logo` captures are byte-identical to
  the build before; on, the header bar, footer capsules, arrows and logo are crisper at the same
  size (1440x816, no white block).
- **`.fnt` faces the HUD draws with** (`oag_texture::replacer::font`, `oag_ui::font::Atlas::from_replaced_font`, in
  `oag_raceplay::hud::hud_font`): the picture is the whole glyph atlas at a factor `f` the same on
  both axes. Each cell's box moves to the picture's pixels and `Atlas::texel_scale` (Omega's 2x
  mechanism) reads a texel as `1/f` grid units, so a glyph keeps its size and its pen advance and
  only gains texels; `borderExtend` is `f` times the authored one. Coverage is the picture's alpha
  and the body/outline mask its red channel, as the disc's palette is. **A pack's HUD font is one
  grey**: `x2_hud_1.png` is white with alpha, so the baked outline the disc's six greys carry is
  not drawn (`is_outlined` is false for it). Two traps, both in `pulse_text.fnt` (the `Default`
  face: flyby text and the Eliminator kill column), which drew every glyph box as a white block:
  its upscaled edge has red 254 under alpha 254, which is rounding, not a second grey, so greys
  are compared under fully opaque ink only; and the PNG is white under zero alpha, so the body
  plane is 0 wherever coverage is (the disc's transparent palette entry carries none either).
  Checked in frames of the Pulse 2020 folder, flyby and Eliminator. A picture that is not a whole multiple, differs per
  axis, or is over 8192 a side (`MAX_ATLAS_SIDE`) leaves the disc's face and says so in the load
  report. Pulse EU's four HUD faces key and hit in both packs (`PulseHud.fnt` 512x256:
  4x in the 2026 pack, 16x in the 2020 folder; `small.fnt` 4x and 16x; `Pulse_20.fnt` 4x and 16x;
  `pulse_text.fnt` 4x and 16x). The `x2_hud_1` and `x2_hud_2` lines are two different fonts, not a
  split atlas. A 16x face is a 8192x4096 atlas, two planes: about 67 MB per face while it is
  built.
- **A face is used at most 8x, and always within a texture** (`oag_ui::font::PACK_MAX_FACTOR`,
  `Atlas::from_pack`, `Atlas::reduced`). The front end's faces carry PromptFont's substitute
  button glyphs (`Atlas::with_prompts`) beside the disc's, each cell in its disc glyph's box, so at
  a pack's 16x the three Pulse faces were 4096x6672, 4096x18744 and 4096x12948 (157 Mtexel, 314
  MB per copy), and a Steam Deck's 16,384-texel textures refused the 18,744: `wgpu error: ...
  Dimension Y value 18744 exceeds the limit of 16384`, a panic opening the menus. `from_pack` now
  takes the largest whole fraction of the pack's factor that is at most 8 (the front end lays out
  in 480x272, so 8 texels per unit covers a 3840-wide screen) and that leaves the atlas **with its
  button glyphs** at most `MAX_ATLAS_SIDE` (8192) a side, and says which in the load report
  (`... 2048x1026 from the texture pack (8x of the pack's 16x)`). The 16x Pulse pack's
  faces come out 2048x3474, 1024x4752 and 2048x6508 (25 Mtexel, 51 MB): `Pulse_20.fnt` (the menu
  rows) falls to 4x because its button glyphs at 8x would be taller than 8192; 4x is exact for a
  1080p screen and a little soft at 4K. The reduction averages whole blocks of the pack's texels, so
  every cell stays on a texel edge. **The renderer guards the device as well**
  (`render::face::fit_to_device`): an atlas larger than the device's own
  `max_texture_dimension_2d` (below the loader's 8192 on a web or old phone adapter) is reduced by
  `Atlas::fitting` with a `warn` line, never a validation panic. **`OAG_MAX_TEXTURE_SIDE=<n>`**
  makes the device with at most that limit, to try a Deck (16384), a phone (8192) or 2048 on a
  desktop adapter. Open: the prompt cells are packed at the atlas's own width, so most of a
  face's rows are button glyphs; packing them as wide as the limit would let the menu face keep 8x.
  Every picture in both packs is at most 8192 a side (8192x4096: the two `fx400_hud_big` fonts;
  everything else 4096), so no single replaced texture exceeds a phone's limit; the HUD sprite sheet
  (`oag_hud::sprite::Sheet`, stacked up to 8192 tall in columns, width the sum of the columns) is
  not reduced to a device and measured 2048x2194 on Pulse's time trial with the Pulse pack; the
  front end's sheet measures 4096x2372 (above 2048, so a device with 2048-texel textures refuses it).
- **The front end's faces** (`oag_game::boot::load_shell`, `boot/fonts.rs`): the boot engages the
  pack (`oag_raceplay::texture_pack::engage`, the race load's own function) after the disc opens and
  switches it off once the faces are built, so nothing after them (the menus' other art, a race's
  preview) sees it. The switch is process-wide: a race load runs only in the Loading stage, never beside
  the boot or a language reload on the menus, so none overlaps it (checked by reading, not a race test). Each of the four faces the front end builds (`Default`, the menu face, the
  title face and the third slot) goes through `oag_ui::font::Atlas::from_pack`, the function the race HUD's
  `hud_font` calls (it moved there from `oag_raceplay::hud`), so a face keeps its size and pen
  advance and only gains texels exactly as in the race. The menu rows, the footer ticker,
  the nav legend and **the loading screen's text** (it draws in the boot's `Default` face) are the
  pack's. A language reload (`Session::resupply_language`) passes the same finder, so a language
  switch keeps the pack's faces. **A MODS page switch of a pack is live**: `switch_selected_mod` reloads the
  shell (`Session::resupply_language`, which also builds the menu tree again and gives the renderer
  the new sheet) instead of only rebuilding the menus, then puts the MOD row back on the pack; a
  switch is a second or so on the UI thread in a debug build. Test:
  `drive_ground_truth::the_mods_page_turns_a_texture_pack_on_and_off_and_the_front_ends_faces_follow_live`
  (the faces and the sheet are the pack's after the switch on, the disc's after it off). Cost: a 16x face is a 4096 a side atlas, about 0.3 s each
  to build in a debug build (the 2020 folder's three Pulse faces took 0.8 s more on the boot's first
  half), the 2026 zip's 4x faces under 0.1 s. Checked in frames at 1440x816 (Pulse EU, `--menu-page
  main` and `options`, `--loading-screen 0/1 --loading-step race`), the 2020 folder and the 2026 zip:
  rows, ticker and the loading line are crisp at the disc's size, no white block; the 2026 zip's
  hand-remade title face draws a `U`-like `V` (the pack's art). Test:
  `crates/game/tests/texture_pack_frontend_ground_truth.rs` (the default face is the picture's
  size with a texel scale under 1 and keeps its line height; off is the disc's face and says nothing; the
  pack is off again when the boot returns).
- **Pure's front-end faces are held off** (`oag_title::FrontEnd::runtime_palette_faces`, true for Pure
  only; `boot::fonts::engage_pack` does not engage the pack and says so in the report). Measured
  2026-11-11 on the 2020 Pure folder: `FX300ANG.fnt` and `LTe50325.fnt` hit tier `(K, H)`, the full key
  and not the data hash alone, at 16x, and drew as the pack's solid letters where the disc's are hollow
  outlines. The pack's readme says Pure's fonts take a dynamic CLUT ("almost impossible to replace
  reliably") and nothing here measured the palette the original draws them with, so Pure is left on the
  disc's faces until a lane takes a Pure oracle dump. Pure's race HUD still asks the same function (not
  changed here). Test: `pure_keeps_the_discs_faces_with_a_pack_on_because_its_palette_moves`.
- **Not replaced**: particle (`.pob`) textures and ship skins. Each is its own lane's check, in a
  frame. Pulse's loading screen (checked at 1440x816 with `--track 16_Track`, pack on and off) draws
  the tip, the circuit name and the generated wave and no sheet picture, so its picture is the face.
  The race-box pickers, the end-of-race screens and the campaign draw through the same
  `Placed::sheet_rect` now but were not captured with a pack on.
- **Frames** (Pulse EU, the 2020 reTokyo snapshot, `--race --seed 1 --ticks 600 --size 1280x720`):
  with the pack off the frame is byte-identical to `main`'s (re-checked after the HUD and font
  change: ticks 600 and 1500 `cmp` equal to the frames taken before it); with it on, 453 textures were
  replaced, the hull, the track surface and the walls read as the pack's art in the same
  places. Test: `crates/raceplay/tests/texture_pack_ground_truth.rs` (slots grow by a multiple,
  off changes nothing, a pack for another serial is not used and says so, the HUD sheet's scale and
  each face's factor and glyph cells match, off is scale 1.0 and texel scale 1.0).
- **HUD frames** (2026 pack, `--race --seed 1 --size 1920x1080`, ticks 600, and `--autopilot` at
  1800 and 3600): the lap, best, current and record readouts, the speed line, the shield and
  speed bars, the pickup hexagon and the clock icon sit where the disc's draw them and read at
  1080p as the pack's sharp art instead of the PSP's 256-pixel atlas stretched 4x. The shield and
  speed gauges are `.vex` models and take the pack through the model path.
- **The 2026 `textures.zip` drawn** (2026-11-10, same race and seed, `--size 1280x720`, packs under
  `OAG_MODS`, the zip extracted alone out of the `GitHub - WipEout Mods` download and read in place):
  7,646 keys against the 2020 folder's 2,392, and 578 textures replaced at load against 455.
  At tick 600 and tick 1500 the hull is the 2026 art (green-yellow panels with hull lettering, where
  the disc's is blue and yellow), the track surface is the same glass-and-panel reading as the 2020
  pack but sharper, and the grandstand wall carries painted lettering the 2020 pack
  leaves as the disc's blocks; the flare ("m" card) and the HUD are unchanged. The zip route is also
  covered by a synthetic zip in `oag-mod` and the folder route by the real snapshot. The 2026 zip
  holds no `.zim`. Pure is not wired.

### What the wiring was built from

For the lane that draws a pack (opt-in, default off, rendering-only, lazy):

1. **Find the pack**, in this order: `data/texture-packs/<name>/` (and the
   device's `<XDG_DATA_HOME>/oag/texture-packs/<name>/`, where `just push-data`
   puts it), then a PPSSPP memstick `PSP/TEXTURES/<serial>/` (symlinked there by
   `just fetch-texture-packs --install-ppsspp`; `~/.config/ppsspp` on Linux). The
   folder is PPSSPP's own layout: `textures.ini` plus the PNGs it names, **or a
   `textures.zip` with the ini at its root, read in place** (the 2026 pack is only
   that; `zip` is already a dependency of `oag-mod`). The zip is preferred when both
   exist, as in `LoadIni`. Choose the pack by the disc's serial against `[games]`
   (the 2026 ini lists 13, including `WPCE02025`).
2. **Parse** with `Pack::parse`; refuse a pack with `hash != xxh64`,
   `ignoreaddress` false, or any section but the three above (`other_sections`),
   by name.
3. **Key** each texture as it is bound, with `Stored::replacement`. Build
   `Stored` from the texture's file bytes (`.mip`: `Stored::from_mip`; `.fnt`:
   `from_font_atlas`; `.vex`: `vex::stored_textures`).
4. **Look up** in PPSSPP's order: `(dim^clut, data)`, `(dim^clut, 0)`, `(0, data)`.
   `Replacement::key` is `None` for a heap-dependent texture: look it up by
   **`(0, data)` only** (a zero-cache-key line, 245 in Pulse) or by data hash
   alone, since the exact key is not reproducible. **Do not replace by data hash
   alone when the palette is a runtime variant** (Pure's HUD fonts): `other` keys
   need the palette bytes, so leave them unreplaced.
5. **Skip a key whose PNG the pack lacks.** Only 1,592 Pulse and 110 Pure
   reproduced keys have one. Read each PNG's own size and scale UVs per texture (below). Honour a blank value (leave the texture alone) and
   the `0909` wildcard.
6. **A replacement is larger**: over the 175 Pulse keys both the dump and the pack have, the PNG is 4x the original on 101, 2x on 29, 8x on 16, 16x on 12, 1x on 6, and a different factor per axis on 7. UVs are fractions, so a draw needs no change; a texture the game atlases by pixel offset (fonts, the HUD sprite sheet) must be checked per frame. Judge in a race, not a still.
7. **Not wired by this lane.** Nothing in the loader report names a pack yet; the
   wiring lane adds the line (`warn`: pack requested and absent; `debug`: how many
   textures it replaced).

### A pack is a mod to the game

The maintainer wants a texture pack to be enabled and disabled like any other mod. Today
`oag-mod` has **no texture path**: its manifest carries menus, strings, settings, tracks,
icons, effects, rumble, campaigns and simulation modules, and a pack is keyed by a hash of
the original's guest memory, not by an asset name. The design that fits:

- **Native reading, not an importer.** An importer that writes an `oag-mod.toml` has to
  put it *inside* the pack: a 553 MB `textures.zip` cannot be given a manifest without
  repacking it, and a folder shared with a PPSSPP install (symlinked per serial) would
  gain a file PPSSPP ignores and a player did not put there. So `oag-mod` should
  **recognise a PPSSPP pack where it finds one** and synthesise the manifest: a folder or
  a zip with `textures.ini` at its root and a `[games]` table, and no `oag-mod.toml`.
  A hand-written `oag-mod.toml` stays possible for a pack whose author wants a name,
  version and licence, and wins over the synthesis.
- **Generated manifest (done).** The maintainer asked for one in the folders the fetch script
  makes, and the objection above holds only for a pack we did not fetch: `just
  fetch-texture-packs` writes an `oag-mod.toml` (`oag-mod init-pack`, rendered from the same
  `recognise` the synthesis uses) into each checkout, which wins over recognition; the file is
  git-excluded and never rewritten. A pack in a PPSSPP install, or a `textures.zip`, is still
  synthesised and never written to. 2048 and Omega: not applicable (PPSSPP keys are the PSP's;
  the manifest and its writer are title-blind).
- **Manifest (synthesised or written).** `id` `ppsspp.<folder or zip stem, lower-cased>`,
  `name` the same, `version` `0.0.0` unless written, `license` `License::None` (a mod
  without one "loads, and says so wherever it is listed": exactly the state of these
  packs), `titles` from the `[games]` serials through a serial-to-title table (the
  2026 pack: `pulse` and `pure`; the Pulse snapshot: `pulse`). A written manifest gains a
  `[texture-pack]` table: `format = "ppsspp"`, optionally `ini = "textures.ini"` and
  `serials`.
- **Kind.** A texture pack changes only what is drawn: no simulation module, no menu row,
  no state. It is the one mod kind that must be **off by default**: the manifest gains a
  `default-enabled` boolean (default `true`, as every mod is on until `[mods] disabled`
  names it), and a recognised pack is always `false`. **Done, with one correction to this
  design**: `disabled` cannot say "on although it starts off", so the settings file gained
  `[mods] enabled`, the opt-in list for a mod that starts off. The pack is filtered by the
  disc's serial against its `[games]` rather than by `titles`, which a recognised pack leaves
  empty (the serial-to-title table was not needed).
- **Search path (done).** `oag_mod::mods_search_path` gains `data/texture-packs/` (one level, a
  folder holding `textures.ini`, or `textures.zip`), the device's
  `<XDG_DATA_HOME>/oag/texture-packs/`, and, when it exists, a PPSSPP memstick's
  `PSP/TEXTURES/` (each `<serial>/` folder: a pack linked once per serial is one mod, by
  canonical path, as `dlc::discover` already de-duplicates). `just mods-list` shows them
  and what was left out, like any other.
- **Reading.** Through `Container` (folder or zip, read in place): `textures.ini` once,
  then PNG (or ZIM) entries lazily as a texture is first drawn. The key lookup is
  `oag_texture::replacement`, a render-side consumer; `oag-mod` stays ignorant of titles
  and textures, as its docs promise, and hands over the container.
- **Consequence for the skill.** The manifest field, the table and the recognition rule
  are a mod-format change, so the wiring change **must update**
  `assets/mods/examples/.claude/skills/oag-mod-author/SKILL.md` (committed on the examples
  submodule branch, then the pointer) and `docs/tools/mods.md`. This lane changed no mod
  format and left the skill alone.

Open: whether a pack that targets two games (the 2026 one) is one mod or two (one mod,
`titles = ["pulse", "pure"]`, is the cheaper reading); whether `default-enabled` is a
manifest field or a property of the kind.

### ZIM images

Later versions of the pack (PPSSPP v1.11.3 and later; "835 MB PNG, 1,009 MB ZIM" in
the forum thread) ship **ZIM**, PPSSPP's own image format, as `.zim`; PPSSPP accepts
`.png`, `.zim`, `.dds` and `.ktx2` (`ScanForHashNamedFiles`) and an alias names one.
Layout, from `Common/Data/Format/ZIMLoad.cpp` and `.h` (v1.20.4), little-endian:

```text
+0x00  4    "ZIMG"
+0x04  u32  width
+0x08  u32  height
+0x0c  u32  flags
+0x10  ...  pixel data
```

Flags: bits 0-3 pixel format (`0` RGBA8888, `1` RGBA4444, `2` RGB565, others refused),
`0x10` has a full mip chain (levels = `log2(min(w, h)) + 1`, each half the size), `0x20`
generate mips, `0x40` dithered on save, `0x80` clamp, `0x100` zlib-compressed, `0x200`
`0x400` `0x800` ETC1 quality (unused by the loader), `0x1000` zstd-compressed. The data
after the 16-byte header is **one** stream (the header's comment says each mip is
compressed separately; the v1.20.4 code decompresses the whole `sum(w*h*bpp)` at once):
raw, zlib or zstd, of the levels back to back. A wrong decompressed size is logged and
the image used anyway.

**Read (`oag_texture::zim`, 2026-11-10).** `Replacements` picks a decoder by what the file starts
with (`ZIMG` or PNG), so a pack's ini line may name a `.zim` and a corrupt one leaves the texture
alone. Level 0 only (the reader stops at `w*h*bpp` bytes of the stream, so a chain is never
expanded past it); a short stream, a size over 16384 a side, a format over 2, or both codec flags
are refused by name. zlib is `flate2` (already in the workspace); zstd is `ruzstd`, a pure Rust
decoder with no C toolchain (the `zstd` crate in the tree belongs to the mod loader's `zip` and
is native-only), `std` feature only so the frame checksum is not verified.

- **Oracle: PPSSPP's own assets.** A PPSSPP install ships three real `.zim` files
  (`/usr/share/ppsspp/assets/`: `asciifont_atlas.zim` 2048x256, `font_atlas.zim` 2048x2048,
  both flags `0x1040`: zstd, dithered, RGBA8888; `ppge_atlas.zim` 2048x2048, flags `0x1041`:
  RGBA4444). All three decode to their declared size and read as the glyph atlases they are
  (CJK and Latin glyphs, white on clear, checked in a rendering). `zim_ground_truth.rs` runs them
  (`#[ignore]`d; `OAG_PPSSPP_ASSETS` names another install).
- **Not verified against a real file**: RGB565 (no shipped file uses it), the `0x100` zlib
  stream (synthetic only) and the channel order inside RGBA4444 (red in the high nibble, as in
  `rgba4444`; the one real file holds only white glyphs, so the alpha nibble is proven and the
  colour order is not). No pack we hold contains a `.zim`: the 2026 `textures.zip` has 4,306
  PNGs and none, the 2020 folder none, and the `GitHub - WipEout Mods` download none. The forum
  thread's "1,009 MB ZIM" pack was not obtained, so a pack's own ZIM files, and the hash-named
  `<key>.zim` files PPSSPP also scans for without an ini line (not read: `Pack::parse` keys on
  the ini), are untested here. `.dds` and `.ktx2` are not read.

### 2048 and Omega

Not applicable: this is a PPSSPP-only artefact (PSP guest memory, PSP swizzle); the
Vita and PS4 packages share none of it. `Stored::replacement` is PSP-only by
construction.

## Open

- The ship hull texel rewrite (item 3): needs a guest-memory read of a hull texture
  at draw time (the debugger's `memory.read`) against the file's bytes.
- The load-time image conversion (item 4) and whether the Pure previews are the TGAs.
- Pure's oracle: the dump above is Pulse EU only. Pure's classes are inferred from
  the same model, which Pulse's 97.5 % and 0 of 3,186 split supports but does not
  prove.
- `max_seen_v` (the 512-tall clamp) is implemented and unit-tested, never needed:
  no 512-tall palette texture is keyed in either census.
- Whether a standalone `.mip` pads level 0 to a 16-byte stride in RAM: the two
  readings give identical counts on both discs (rows under 16 bytes are rare).
- The USA discs were censused (same key classes); no USA oracle was taken.

## Reproducing

```sh
just fetch-texture-packs      # clones both packs at the pin
just test-data                # runs ppsspp_pack_ground_truth (skips without the packs)
OAG_PPSSPP_DUMP=<dir> just test-data   # also pulse_eu_oracle
OAG_TEXTURE_PACK_ZIP=<textures.zip> just test-data   # the 2026 pack (else data/texture-packs/WipEout-Pure-Pulse-PPSSPP-Pack-2026-07-25/)
```

The test prints the per-directory tables above.
