# Pulse PS2 functions

Functions from `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`, language `r5900:LE:32:default`.

An unencrypted Emotion Engine ELF, so no decryption step is needed. The
`IOP/*.IRX` modules are separate binaries and get their own directory if they
become relevant.

Auto-analysis finds 5,234 functions. `.data` and `.rodata` are mapped three
times over (`0x002xxxxx` cached, `0x202xxxxx` uncached, `0x303xxxxx` uncached
and accelerated); every address in this directory is the canonical `0x002xxxxx`
form, and Ghidra's string search will return all three.

| Page | Covers |
| --- | --- |
| [WAD subsystem](wad-subsystem.md) | Two copies of the name hash, mount, lookup, and the entry layout |
| [Handling stats loader](handling-xml.md) | All 32 handling parameters, all 27 camera parameters, the five load-time scale factors |
| [Input](input.md) | The abstract button layer, the four edge masks, per-player pad blocks |
| [Camera views](camera.md) | The player-selectable in-race views and the SELECT cycle |
| [Recovered C library](libc.md) | `strlen`, `strcmp`, `strcasecmp`, `tolower`, `_ctype_` |

## Renames

**Applied.** 48 symbols, collected in [names.tsv](names.tsv). There is no
`just` recipe for this set yet; run the script directly against a bridge with
`SCES_547.48` open:

```sh
scripts/apply-ghidra-names.py --program SCES_547.48 \
    docs/ghidra/functions/ps2-pulse/names.tsv
```

Nothing in this set scored below 70, so no `_q` names exist yet.

## Why this binary matters

The [confidence rubric](../../../reverse-engineering/confidence-rubric.md) says
a second binary is worth more than a second reading of the first, and caps
everything not corroborated in one at 94. This directory exists to supply that
corroboration for the PSP pages, so its most useful content is not the new
names but the agreements and the disagreements.

### Confirmed, in a second binary

| Claim | PSP page | How the PS2 agrees |
| --- | --- | --- |
| The 32-field handling block, `0x94`..`0x114`, stride `0x80` | [engine.md](../psp-pulse/engine.md) | Every attribute lands on the same offset, from seven separate parsers |
| Four handling parameters are pre-scaled at load | [engine.md](../psp-pulse/engine.md) | Same four fields, same `1e-3` / `-1e-2` / `1e-4` / `1e-4` factors |
| `AirbrakeGraphics.amount` is degrees, scaled by `pi/180` | [camera.md](../psp-pulse/camera.md) | Same conversion, same `0x6c`/`0x70`/`0x74` triple |
| The 27-float camera block, and `<BackwardCamera headtilt>` being dead | [camera.md](../psp-pulse/camera.md) | Same offsets, same four-attribute parser that drops `headtilt` |
| The WAD name hash: CRC-32 reflected, **initialised to 0**, `\`-to-`/`, uppercase folded, `#`-hex escape hatch | [wad-subsystem.md](../psp-pulse/wad-subsystem.md) | Two independent implementations in this binary, both matching |
| `size_in == size_out` is the outer test, not the compression flag | [wad-subsystem.md](../psp-pulse/wad-subsystem.md) | It is the only test `Wad_Open` makes when choosing a stream |
| The 8-byte WAD header and 16-byte entries, `{hash, offset, size_out, size_in}` | [formats/wad.md](../../../formats/wad.md) | `Wad_MountArchive` reads exactly that shape |
| The abstract button layer and its four edge masks | [input.md](../psp-pulse/input.md) | Same raw masks, same indices, same `pressed = held & ~last` derivation |
| Three player camera views cycling `OPT_INT` -> `OPT_CLOSE` -> `OPT_FAR` on SELECT | [camera.md](../psp-pulse/camera.md) | Same three literals, same rotation, same hide-own-ship flag pattern |

### Where the two builds disagree

These are findings, not noise, and none of them has been reconciled.

| Difference | Detail |
| --- | --- |
| WAD lookup scan | PSP rotates the scan start from the last hit; PS2 restarts at the beginning every time. See [wad-subsystem.md](wad-subsystem.md) |
| `cancel` and `backward` | PSP binds them to `circle` (index 4); PS2 binds them to `triangle` (index 6). `activate`/`forward` are `cross` on both. See [input.md](input.md) |
| `Input_ConsumePress` | PS2 clears the entire pressed mask and ignores the button index it is handed; the PSP page describes clearing one bit, at confidence 70. See [input.md](input.md) |
| The chase camera's 3/4 factor | The PSP page measures the external offsets reaching the eye at exactly 0.75 of their authored value, from an unfound source. No such factor exists on the PS2 path, which computes its distance by a visibly different method. See [camera.md](camera.md) |
| Runtime CRC table construction | PSP builds one table; PS2 has three separate lazily-built copies in three translation units, plus a fourth, statically initialised, standard-`crc32` table |

### Answered here, open on the PSP side

**`stats_base + 0x90` is `<Misc weight_distribution>`.**
[engine.md](../psp-pulse/engine.md) records that offset as "a per-team scalar
outside every class block; its element is not determined". The PS2 binary has a
`<Misc>` element the PSP page never mentions, and its parser writes `0x90`
directly. See [handling-xml.md](handling-xml.md).

## Candidates, not yet confident enough to name

Left as `FUN_*` deliberately, per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md)'s below-50
rule or because the hypothesis is untested.

| Address | Hypothesis | Conf |
| --- | --- | ---: |
| `0x00202d48`..`0x00203a80` | The XML reader family: document open, first/next child, element-name compare, first/next attribute, attribute-name compare, attribute as float / int / string. Obvious from use in `Handling_ParseStats`, individually unverified. Naming these would unlock every XML-driven subsystem at once | 60 |
| `0x00213e58` | The decompressing WAD stream constructor. `0x400`-byte staging buffer, a bit accumulator seeded to `0x80`: the shape of an MSB-first flag-bit LZSS reader, but the decode loop was not read. **The PS2 archives are the only corpus that exercises LZSS**, so this is the highest-value target left | 55 |
| `0x00257740` | The stored (uncompressed) WAD stream constructor | 55 |
| `0x001fd618` / `0x001fd6b8` / `0x001fd8c0` | A name-hashed resource registry: find-by-hash, load-file-and-register, register-from-memory, over a singly linked list rooted at `0x00284840`. It is what calls `Resource_HashName`; it is not the WAD directory, so mapping these three onto PSP `Wad_*` names would be wrong | 60 |
| `0x001fed88` / `0x001fede8` / `0x001fee90` / `0x001fef20` | A third CRC-32 family with its own table at `0x003025e0`: an **uppercase**-folding string hash, a raw string hash, and a two-word hash. Not the WAD hash (wrong fold direction); what uses it was not traced | 55 |
| `0x0014f108` | Returns the current camera view setting, from `g_settings` or from a per-player array depending on a global that reads like an attract-mode switch | 55 |
| `0x00201f30` / `0x0010aec8` | The pad read. A thin shim over a library call; no import table was resolved for this binary | 50 |
| `0x002849a0` | A per-pad global that suppresses every button except cross. Reads like a menu or demo lockout | 40 |
| `0x002dab3c` | A global selecting between profile-backed and per-player camera settings. Attract mode is the obvious guess | 40 |

## Not attempted

No PS2 counterpart was looked for on
[main-loop.md](../psp-pulse/main-loop.md),
[collision.md](../psp-pulse/collision.md),
[frontend-video.md](../psp-pulse/frontend-video.md), or the `Ship_Update*` force
terms in [engine.md](../psp-pulse/engine.md). The force terms are the obvious
next target: the parameter block is now confirmed to be laid out identically,
so the PS2 consumers read the same offsets, and a second reading of the craft
frame would raise the whole of `engine.md` out of its decompilation-only cap.
