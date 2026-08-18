# How a PS2 texture name resolves: `.mip` and `.tga` become `.pct`

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748, PAL), image base
`0x00100000`.

**The names here are applied**, from [names.tsv](names.tsv). See
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

## The short version

Both discs' executables name the **same** texture literals - the PSP's
`Data\Tex\engineFlare\Engine_noise.mip` is byte-for-byte the string the PS2
build passes to its loader. Neither disc stores its textures under a second
name. What the PS2 build does is **rewrite the extension before it hashes**:
`.TGA` and `.MIP` both become `.PCT`, which is the PS2's compiled texture
container ([`ps2-texture.md`](../../../formats/ps2-texture.md)).

That one substitution is why `Data\Tex\engineFlare\Engine_noise.mip`
(`008d70a2`) is in `Data.wad` on the PSP and in no PS2 archive at all, while
`Data\Tex\engineFlare\Engine_noise.pct` (`e9f16c12`) is entry 6876 of
`WADS2.WAD`.

## The measurement that says it is general

Every string in `SCES_547.48` that looks like an asset path was hashed with
`wad::hash_name` and looked up in both PS2 archives (7,393 entries), and the
same names were looked up in all four of the PSP EU disc's archives
(`Data.wad`, `FE.wad`, `FEData.wad`, `BEData.wad`, 1,461 entries) as a control:

| Extension | Literals in the PS2 exec | Found on PS2 | Found on PSP |
| --- | ---: | ---: | ---: |
| `.pob` | 28 | 27 | 27 |
| `.vex` | 32 | 17 | 16 |
| `.bnk` | 12 | 12 | 12 |
| `.xml` | 25 | 11 | 11 |
| `.mip` | 18 | **0** | 17 |
| `.tga` | 32 | **0** | 0 |

Every other kind of asset resolves on both discs under the name the executable
spells. The single PSP `.mip` miss is `Data\FE\Trophies\textures\trophy.mip`,
which the PS2 disc does carry as `.pct` - a difference between the two releases'
content, not a lookup failure. Only the raster formats miss, and they miss **completely** - which is a
class-level fact and not eighteen separate name-mining problems. Rewriting the
extension to `.pct` and hashing again finds **47 of those 50**; the three that
still miss are `Z:\WipeoutPSP\Art_Resources\Psys\Tex\psysed_default_glow.pct`
(an absolute authoring path off a build machine), `\ALL_TEXTURES.pct` and
`\TEXTURES_ALL.pct` (stubs with no directory).

Confidence **95**. Corroborated three ways below.

## `Texture_FindOrLoad` (`0x0010c1e0`)

The resolver. Takes a name, returns a refcounted texture handle.

```c
Texture_FindOrLoad(name):
    s = strcpy(name)
    if (p = strstr(s, "WIPEOUT PSP\\PS2\\")) s = strcpy(p, p + 0x10)   // strip build path
    String_ReplaceAll(s, ".TGA", ".PCT")
    String_ReplaceAll(s, ".MIP", ".PCT")
    key = Wad_HashNameString(s)
    if (node = map_find(g_texture_cache, key)) return node->handle
    if (file_exists(s)) { ... insert and return a new handle ... }
    s = "Data/Tex/Missing.pct"                                        // and recurse
    return Texture_FindOrLoad(s)
```

The five string constants are consecutive in `.rodata`, which is what makes the
reading unambiguous - they are one table, laid out by one compilation unit:

| Address | Value |
| --- | --- |
| `0x0029eb40` | `WIPEOUT PSP\PS2\` |
| `0x0029eb58` | `.TGA` |
| `0x0029eb60` | `.PCT` |
| `0x0029eb68` | `.MIP` |
| `0x0029eb70` | `Data/Tex/Missing.pct` |

`String_ReplaceAll` (`0x0020bd20`) is a plain find/erase/insert loop from
index+1, so it replaces every occurrence rather than the last - the same result
`strrchr` gets on these names, and the reason
[`loading-screen.md`](loading-screen.md) describes the loading screen's own
rewrite in `strrchr` terms.

`Wad_HashNameString` (`0x0020c588`) is the string-object overload of
`Wad_HashName`: it calls `Wad_ParseLiteralHash` first, exactly as
`Wad_HashName` does, and otherwise runs the identical CRC loop over
`g_wad_crc_table` (`0x00303618`) with `g_tolower_table` (`0x002c6d98`). See
[wad-subsystem.md](wad-subsystem.md) for the hash itself. Confidence **88**.

`g_texture_cache` (`0x002db9e0`) is the `std::map` the handles are cached in,
keyed on that hash; the walk in the decompiler is the ordinary red-black lower
bound. Confidence **75** - the container's shape is unambiguous, its element
type was not read.

**`Data/Tex/Missing.pct` is on the disc.** It hashes to `d407e4cb` and is entry
6910 of `WADS2.WAD`, which is a check on the whole reading: the fallback branch
names a file, and the file is there, in the archive, under the rewritten
spelling. A misread of this function would not produce a name that exists.

## Corroboration 1: three hashes recovered by picture, reproduced by the rule

`oag_pulse::PS2_IMAGES` holds three PS2 entries that were found on 2026-08-08,
**ten days before this function was read**, and by a completely different
method: each PSP `.mip`
was decoded, reduced to a silhouette, and correlated against every same-shaped
PS2 texture on the disc (see that constant's own documentation). All three fall
straight out of the rewrite:

| Declared name | Rewritten | Hash | Found by picture at |
| --- | --- | --- | ---: |
| `Data\HUD\Textures\PulseHUD.mip` | `...PulseHUD.pct` | `beaf613c` | entry 3518 |
| `Data\FE\Images\pulse_logo.mip` | `...pulse_logo.pct` | `1e6c873e` | entry 3421 |
| `Data\FE\Images\pulse_assets.mip` | `...pulse_assets.pct` | `0d31af1b` | entry 3420 |

That check runs offline, with no disc, in `oag-pulse`'s
`the_pct_rule_reproduces_every_picture_matched_hash`.

## Corroboration 2: the two exhaust textures decode, at the PSP's dimensions

`Texture_LoadEngineNoise` (`0x001efae8`) allocates a `0x140`-byte texture
object, initialises it from `0x002be7d0`
(`Data\Tex\engineFlare\Engine_noise.mip`) through `Texture_InitFromName`
(`0x001ef040`), and stores the result into **three** consecutive globals at
`g_engine_noise_textures` (`0x003586d0`) - the same three-slot shape the PSP's
`Trail_DrawRibbon` indexes per layer in
[`exhaust.md`](../psp-pulse-usa/exhaust.md). `Texture_LoadEngineFlare`
(`0x001d5540`) is its twin for `0x002b7f90`
(`Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip`), into the single
`g_engine_flare_texture` (`0x003586c0`). Confidence **88** for both: the
literal, the slot count and the call shape all match the PSP page.

Pulling the two rewritten names out of `WADS2.WAD` and decoding them with
`oag_formats::ps2_texture` gives 64x64 at 8 bpp and 128x64 at 8 bpp -
**the PSP's dimensions exactly**, and `grabbedEngineFlare128x64x8` states its
own shape in its name.

## Corroboration 3: it was already half-recorded

[`loading-screen.md`](loading-screen.md) recorded, for the loading screen alone,
that `Loading_ScreenConstruct` rewrites a tip XML's `.mip` to `.pct` with
`strrchr` and that the PS2 uses `.pct` "throughout". That finding was correct
and never generalised, and in the meantime `oag_pulse::PS2_IMAGES` was written
with the note that "how the game itself performs this lookup is not known" -
after a sweep of 60 spellings that did not include this one.

**The trap, for the next contributor:** a name-variant sweep is only as good as
its extension list, and a per-screen note is easy to read as a quirk of that
screen. When a whole *class* of names misses and every other class hits, the
answer is a transformation the loader applies, not sixty more spellings.

## Names applied

| Address | Name | Kind | Confidence |
| --- | --- | --- | ---: |
| `0x0010c1e0` | `Texture_FindOrLoad` | function | 88 |
| `0x001ef040` | `Texture_InitFromName` | function | 75 |
| `0x001efae8` | `Texture_LoadEngineNoise` | function | 88 |
| `0x001d5540` | `Texture_LoadEngineFlare` | function | 88 |
| `0x0020bd20` | `String_ReplaceAll` | function | 82 |
| `0x0020c588` | `Wad_HashNameString` | function | 88 |
| `0x003586d0` | `g_engine_noise_textures` | data | 85 |
| `0x003586c0` | `g_engine_flare_texture` | data | 85 |
| `0x002db9e0` | `g_texture_cache` | data | 75 |

## What this changed in the code

`oag_pulse::ps2_texture_name` is the rewrite, and `oag_pulse::read_image`
applies it as a second name to try when the declared one misses. No platform
test: on a PSP source the declared name has already answered, and no `.pct`
entry exists there to collide with.
