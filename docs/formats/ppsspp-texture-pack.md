# PPSSPP texture packs

**Status: hash understood and measured, nothing wired.** A PPSSPP texture pack
replaces a texture by the **key PPSSPP computes from the texture's bytes in guest
memory**, before decode. This page is the key, its evidence, and how many keys of
the two Wipeout packs this engine reproduces from the bytes it loads off the disc.
Reader: [`oag_texture::replacement`](../../crates/texture/src/replacement.rs)
(hash and key), [`oag_texture::replacement_pack`](../../crates/texture/src/replacement_pack.rs)
(`textures.ini`). Test:
[`crates/texture/tests/ppsspp_pack_ground_truth.rs`](../../crates/texture/tests/ppsspp_pack_ground_truth.rs).
Fetching them: [texture-packs.md](../tools/texture-packs.md).

The two packs, both unofficial and unlicensed fan work, never committed
(`data/texture-packs/`, gitignored):

| Pack | Repo | Pinned commit | Serials in `[games]` |
| --- | --- | --- | --- |
| Pulse | `reTokyo/WipEout-Pulse-Upscale-Version` | `33fd6a3f1097` | UCES00465, UCUS98712 |
| Pure | `reTokyo/WipEout-Pure-HD-Texture-Pack` | `f91d55bbde51` | UCES00001, UCUS98612, UCJS10007, ULES00229, ULUS10058 |

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

## Oracle: PPSSPP's own keys, Pulse EU

PPSSPP v1.20.4 on `pulse-psp-eu.chd`, hardware (OpenGL) renderer under Xvfb (the
software renderer bypasses the texture cache and dumps nothing), with
`[Graphics] SaveNewTextures = True` and a one-section `textures.ini`
(`hash = xxh64`, `ignoreAddress = true`, `ignoreMipmap = true`; without it the
dump uses `quick` and addresses, and is not comparable). The walk covered the logo, the profile prompts, the campaign page, the ship select and about a minute
of Talon's Junction. It wrote **3,411 files**, each named by the key PPSSPP would
look it up by: the oracle for the key, independent of the pack and of this code.
Re-run it with `OAG_PPSSPP_DUMP=<TEXTURES/UCES00465/new> just test-data`
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

## Ready to wire

For the lane that draws a pack (opt-in, default off, rendering-only, lazy):

1. **Find the pack**, in this order: `data/texture-packs/<name>/` (and the
   device's `<XDG_DATA_HOME>/oag/texture-packs/<name>/`, where `just push-data`
   puts it), then a PPSSPP memstick `PSP/TEXTURES/<serial>/` (symlinked there by
   `just fetch-texture-packs --install-ppsspp`; `~/.config/ppsspp` on Linux). The
   folder is PPSSPP's own layout: `textures.ini` plus the PNGs it names, or a
   `textures.zip` with the same content (`LoadIni` prefers the zip). Choose the
   pack by the disc's serial against `[games]`.
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
   reproduced keys have one. Honour a blank value (leave the texture alone) and
   the `0909` wildcard.
6. **A replacement is larger**: over the 175 Pulse keys both the dump and the pack have, the PNG is 4x the original on 101, 2x on 29, 8x on 16, 16x on 12, 1x on 6, and a different factor per axis on 7. UVs are fractions, so a draw needs no change; a texture the game atlases by pixel offset (fonts, the HUD sprite sheet) must be checked per frame. Judge in a race, not a still.
7. **Not wired by this lane.** Nothing in the loader report names a pack yet; the
   wiring lane adds the line (`warn`: pack requested and absent; `debug`: how many
   textures it replaced).

### `oag-mod`

**Not a fit, and it should not import into one.** `oag-mod` carries menus, strings,
settings, tracks, rumble, campaigns, icons and simulation modules; it has no
texture-override path, and a pack is keyed by a hash of PSP guest memory
(a property of the original's loader), not by an asset name. A pack stays a
read-only directory looked up by key. A mod that carried a pack (an `oag-mod.toml`
field naming one) would change a mod format and must update
`assets/mods/examples/.claude/skills/oag-mod-author/SKILL.md` in the same change;
this lane changed no mod format.

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
```

The test prints the per-directory tables above.
