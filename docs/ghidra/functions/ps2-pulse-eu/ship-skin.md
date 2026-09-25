# A PS2 ship skin is a whole sibling atlas, picked by the `.dat`'s file name

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748, PAL), image base
`0x00100000`. The names here are applied, from [names.tsv](names.tsv). See
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

**Static reading of the executable, corroborated against all twelve teams'
entries in `pulse-ps2-eu.chd`'s `WADS2.WAD`. Not runtime-verified on PCSX2.**

## The short version

The PSP's `Skin_ApplyToModel`
([`psp-pulse-usa/ship-skin.md`](../psp-pulse-usa/ship-skin.md)) uploads the
four palette-plus-pixels blocks a `ship_alt.dat` carries into four
`\TEXTUREn.TGA` slots. **The PS2 port never reads a `.dat`'s bytes at all.**
`Skin_SwapAtlasSibling` (`0x001dfb70`) takes only the file's **name**, maps it
to a suffix, loads a *sibling* of the hull's one 256x256 atlas by a name built
from that suffix, and `memcpy`s the sibling's GS image over the hull atlas's
own buffer. There is no quadrant packing, no palette re-indexing and no block
layout: the paint is a whole authored atlas per skin, shipped as a separate
`.pct` entry.

Confidence **88**: the name mapping and the path construction are string
literals and `strcasecmp`/`strcat` calls read straight off the body, and every
name the function can build exists on the disc, for all twelve teams, at the
exact size of the atlas it is copied over.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x001dfb70` | `Skin_SwapAtlasSibling` | 88 |
| `0x001eea58` | `Texture_LoadVexNode` | 80 |
| `0x001117c0` | `strstr` | 90 |
| `0x0010ec30` | `strcat` | 85 |
| `0x00102d28` | `memcpy` | 90 |

## `Skin_SwapAtlasSibling(model, skin_path)` (`0x001dfb70`)

Found from the only xref to `\ALL_TEXTURES.TGA` (`0x002bca80`, referenced at
`0x001dfc50`); the same function also holds the only xrefs to `ship_alt.dat`
(`0x002bca40`) and `ship_eliminator.dat` (`0x002bca58`).

```c
Skin_SwapAtlasSibling(model, skin_path):
    suffix = NULL
    if (skin_path == NULL || *skin_path == 0) suffix = ""
    else {
        base = strrchr(skin_path, '\\') + 1
        if      (!strcasecmp(base, "ship.dat"))            suffix = ""
        else if (!strcasecmp(base, "ship_alt.dat"))        suffix = "livery"
        else if (!strcasecmp(base, "ship_eliminator.dat")) suffix = "eliminator"
    }
    if (suffix == NULL) return                       // any other name: no-op
    for each texture t of model:
        m = strstr(strrchr(t->name, '\\'), "\\ALL_TEXTURES.TGA")
         ?: strstr(..., "\\TEXTURES_ALL.TGA")
        if (!m) continue
        stem = copy(m + 1); *strrchr(stem, '.') = 0  // "ALL_TEXTURES"
        path = copy(t->name); file = strrchr(path, '\\') + 1
        if (!strcmp(suffix, "livery")) strcpy(file, "livery")
        else { strcpy(file, stem); strcat(file, *suffix ? "_" : "2"); strcat(file, suffix) }
        strcat(file, ".mip")
        tmp = new Texture; Texture_InitFromName(tmp, path)
        memcpy(image(t), image(tmp), size(t))       // size of the *hull's* atlas
        tmp->destroy()
        return
```

The string table it works from is consecutive in `.rodata`:

| Address | Value |
| --- | --- |
| `0x002bca30` | `ship.dat` |
| `0x002bca40` | `ship_alt.dat` |
| `0x002bca50` | `livery` |
| `0x002bca58` | `ship_eliminator.dat` |
| `0x002bca70` | `eliminator` |
| `0x002bca80` | `\ALL_TEXTURES.TGA` |
| `0x002bca98` | `\TEXTURES_ALL.TGA` |
| `0x002bcab0` | `_` |
| `0x002bcab8` | `2` |
| `0x002bcac0` | `.mip` |

So for a hull atlas declared `Data\Ships\AG_Systems\Textures\ALL_Textures.tga`:

| Skin file name | Sibling built | After `Texture_FindOrLoad`'s rewrite |
| --- | --- | --- |
| `ship_alt.dat` | `...\Textures\livery.mip` | `...\Textures\livery.pct` |
| `ship_eliminator.dat` | `...\Textures\ALL_TEXTURES_eliminator.mip` | `..._eliminator.pct` |
| `ship.dat` or empty | `...\Textures\ALL_TEXTURES2.mip` | `...\ALL_TEXTURES2.pct` |

It stops at the first matching texture. Where a model names the atlas from
two `Texture` nodes (some `ship_FE.vex` preview hulls do), both nodes resolve
through `Texture_FindOrLoad`'s name-keyed cache to one image, so the one
`memcpy` repaints both; this project replaces every slot carrying the atlas
label for the same outcome.

`.mip` becomes `.pct` inside `Texture_FindOrLoad`, per
[texture-names.md](texture-names.md). The second literal,
`\TEXTURES_ALL.TGA`, is not dead: Feisar's `Ship.vex` names its atlas
`Textures_All.tga`, the other eleven `ALL_Textures.tga` in some case.

`strstr` is case-sensitive against upper-case literals, which only works
because `Texture_LoadVexNode` upper-cases the name first (below).

### Callers, and where `skin_path` comes from

Four callers: `0x001289a8` in `FUN_00128860`, `0x00128d04` in `FUN_00128be0`,
`0x00129f64` in `FUN_00129e90`, and `0x001beecc` in `FUN_001bed60`. The last is
the front end: it inserts the chosen `PI_ModelSkin`'s `location` string into
the `FE_ModelSkin` front-end global (`FeGlobals_Insert`, key string at
`0x002a1258`), applies it to the preview model, and copies it into a
`0x40`-byte-per-player path table at `0x002f4990`. The three race-side callers
look `FE_ModelSkin` up (`FeGlobals_FindAndAcquire`) and, when present, apply
the player slot's path from that table to the craft's model. So `skin_path` is
the definition's own `location` attribute, verbatim, and only its last path
component is ever consulted.

## `Texture_LoadVexNode` (`0x001eea58`): upper-casing and per-player atlas copies

The `Texture` node constructor. It stores the node's name pointer at `+0x120`
and upper-cases it in place (a `_ctype_` lower-case test, then `- 0x20`), which
is what lets `Skin_SwapAtlasSibling`'s case-sensitive `strstr` match the
disc's mixed-case spellings. It hashes the name into `+0x124`.

It then works on a copy of the name and, **only while a craft is being
constructed** (the global at `0x00282a90` holds the craft index, set by
`FUN_00128860` just before and reset to `-1` just after), rewrites a
`DATA\SHIPS...` name ending in `ALL_TEXTURES.tga` or `TEXTURES_ALL.TGA` to
`<stem><index>.TGA` before `Texture_FindOrLoad`. That gives each player's craft
its own texture object, so a `memcpy` repaint touches that craft alone rather
than every cached user of the team's atlas. The disc carries
`ALL_Textures0.pct` and `ALL_Textures1.pct` for every team and no `3`, `4` or
`7`, which fits two local players; confidence 70 on that reading of the
index, since only the names were checked. The same constructor also redirects
the shared glow, glass and environment textures (`grid_glow.tga`,
`lights_glow.tga`, `engine_glow.tga`, `blink_glow.tga`, `glass2_add.tga`,
`envtest4bit.tga`) to `DATA\SHIPS\COMMON\TEXTURES\...`, from the literal table
at `0x002be478`..`0x002be5c8`; that redirect is recorded here and not
reproduced by this project.

## Corroboration on the disc

Measured 2026-09-25 against `pulse-ps2-eu.chd`: for all twelve teams
(AG_Systems, Assegai, Auricom, EGX, Feisar, Goteki, Harimau, Icaras, Mantis,
Piranha, Qirex, Triakis), `livery.pct`, `<stem>_eliminator.pct`,
`<stem>0.pct`, `<stem>1.pct` and `<stem>2.pct` are all present in `WADS2.WAD`
beside the atlas, every one **66,829 bytes and 256x256**, the same as the
atlas itself - which a fixed-size `memcpy` of the hull atlas's own length
requires. Each skin sibling decodes to a distinct picture. AG_Systems'
`livery.pct` is the cyan-and-white livery, the same colours the PSP's
`ship_alt.dat` for that team paints: an independent agreement between the
two ports' data.

The PS2 disc does still carry `ship_alt.dat` files in the PSP's 26,912-byte
layout. The PS2 executable never opens them.

### `<stem>2.pct` is the baseline restore, and differs for two teams

`ship.dat` (or no skin at all, when `FE_ModelSkin` exists) builds `<stem>2`.
For ten teams `<stem>2.pct` is byte-identical to the atlas itself. **For Feisar
and Harimau it is not**: Feisar's `Textures_All.pct` hashes `22d0c48a` and its
`Textures_All2.pct` `96e67765`; Harimau's `All_textures.pct` `48d4b92f` and
`All_textures2.pct` `94a63dcb` (fingerprints: `wad::hash_name_bytes` run over each decompressed entry, which
is CRC-32 with ASCII upper case folded, so only comparable with each other). Whether the
original repaints the baseline craft with `<stem>2` on every race depends on
whether `FE_ModelSkin` is always inserted, which is not traced. This project
does not apply `<stem>2` - see `crates/game/src/livery/ship_skin.rs`.
