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
| [LZSS decoder](lzss.md) | The decompressing stream: bit reader, ring, match encoding, and a dead encoder workspace |
| [XML reader](xml-reader.md) | The parser every data-driven subsystem goes through, and the `<code>` short-name dictionary |
| [Handling stats loader](handling-xml.md) | All 32 handling parameters, all 27 camera parameters, the five load-time scale factors |
| [Craft update and rigid body](craft-update.md) | The surface-alignment gain, angular damping, the integrator and its sub-step loop |
| [Input](input.md) | The abstract button layer, the four edge masks, per-player pad blocks |
| [Camera views](camera.md) | The player-selectable in-race views and the SELECT cycle |
| [Front-end globals](fe-globals.md) | The `FEGlobals->` / `FEConst->` indirection, its registry, and the flag-bit tagging that makes one a live binding |
| [Movie source paths](movie-paths.md) | How a `Movie` widget's `src` becomes a filename, and the one global that picks the PAL cut of both the intro and the backdrop |
| [Recovered C library](libc.md) | `strlen`, `strcmp`, `strcasecmp`, `tolower`, `_ctype_` |
| [Loading screen](loading-screen.md) | The same procedural heartbeat wave as the PSP, and the four places the port diverges |
| [Texture names](texture-names.md) | Why no `.mip` or `.tga` name resolves on this disc: the loader rewrites the extension to `.pct` before it hashes |
| [Batch draw state](batch-draw-state.md) | The port keeps the PSP's `pass_mask` bit for bit, and programs the same two blend equations into GS registers |
| [Collision feedback](collision-shake.md) | The hull-spark trigger and the shield-bubble hit-flash, both shared with the PSP, plus a camera shake on impact the PSP's own path was verified not to have |

## Renames

**Applied.** 169 symbols, collected in [names.tsv](names.tsv). There is no
`just` recipe for this set yet; run the script directly against a bridge with
`SCES_547.48` open:

```sh
scripts/apply-ghidra-names.py --program SCES_547.48 \
    docs/ghidra/functions/ps2-pulse-eu/names.tsv
```

One symbol scores below 70 and carries a `_q`: `g_lzss_encoder_tree`
(`0x00284fc4`, 68). Everything else is applied as written.

## Why this binary matters

The [confidence rubric](../../../reverse-engineering/confidence-rubric.md) says
a second binary is worth more than a second reading of the first, and caps
everything not corroborated in one at 94. This directory exists to supply that
corroboration for the PSP pages, so its most useful content is not the new
names but the agreements and the disagreements.

### Confirmed, in a second binary

| Claim | PSP page | How the PS2 agrees |
| --- | --- | --- |
| The 32-field handling block, `0x94`..`0x114`, stride `0x80` | [engine.md](../psp-pulse-usa/engine.md) | Every attribute lands on the same offset, from seven separate parsers |
| Four handling parameters are pre-scaled at load | [engine.md](../psp-pulse-usa/engine.md) | Same four fields, same `1e-3` / `-1e-2` / `1e-4` / `1e-4` factors |
| `AirbrakeGraphics.amount` is degrees, scaled by `pi/180` | [camera.md](../psp-pulse-usa/camera.md) | Same conversion, same `0x6c`/`0x70`/`0x74` triple |
| The 27-float camera block, and `<BackwardCamera headtilt>` being dead | [camera.md](../psp-pulse-usa/camera.md) | Same offsets, same four-attribute parser that drops `headtilt` |
| The WAD name hash: CRC-32 reflected, **initialised to 0**, `\`-to-`/`, uppercase folded, `#`-hex escape hatch | [wad-subsystem.md](../psp-pulse-usa/wad-subsystem.md) | Two independent implementations in this binary, both matching |
| `size_in == size_out` is the outer test, not the compression flag | [wad-subsystem.md](../psp-pulse-usa/wad-subsystem.md) | It is the only test `Wad_Open` makes when choosing a stream |
| The 8-byte WAD header and 16-byte entries, `{hash, offset, size_out, size_in}` | [formats/wad.md](../../../formats/wad.md) | `Wad_MountArchive` reads exactly that shape |
| The whole LZSS bit layout: 13-bit absolute position, 4-bit length, `+3` bias, 8192-byte ring, cursor at 1, MSB first | [formats/lzss.md](../../../formats/lzss.md) | `Lzss_Decode` reads the same fields in the same order; an independent transcription of it agrees byte for byte with ours. See [lzss.md](lzss.md) |
| The `<code>` short-name dictionary: per file, 18 codes, keyed on the first letter | [formats/fexml.md](../../../formats/fexml.md) | `Xml_OpenFile` builds an 18-slot table on the document from a `code` element, keyed by `name[0] - 'a'`. Inferred from data there, read off the parser here. See [xml-reader.md](xml-reader.md) |
| The surface-alignment gain `-400` and the angular damping triple `(-pitch_damping, -5, k)` | [physics/README.md](../../../physics/README.md), [engine.md](../psp-pulse-usa/engine.md) | Both appear verbatim in the PS2 build, so `-400` is **not** a transcription error. See [craft-update.md](craft-update.md) |
| The four-corner hover selector, the body accumulator offsets, and the craft update's vtable dispatch shape | [engine.md](../psp-pulse-usa/engine.md) | Same globals, same magic value 6, same `+0x100`/`+0x120`/`+0x130`, same `{i16 adjust, fn}` pair with `+0x370` cleared before the call |
| The abstract button layer and its four edge masks | [input.md](../psp-pulse-usa/input.md) | Same raw masks, same indices, same `pressed = held & ~last` derivation |
| Three player camera views cycling `OPT_INT` -> `OPT_CLOSE` -> `OPT_FAR` on SELECT | [camera.md](../psp-pulse-usa/camera.md) | Same three literals, same rotation, same hide-own-ship flag pattern |

### Where the two builds disagree

These are findings, not noise, and none of them has been reconciled.

| Difference | Detail |
| --- | --- |
| Bank-to-yaw coupling | PSP's hover epilogue uses `30 * right.y`; the PS2 four-corner path uses `50.0`. Neither is runtime-verified. See [craft-update.md](craft-update.md) |
| WAD lookup scan | PSP rotates the scan start from the last hit; PS2 restarts at the beginning every time. See [wad-subsystem.md](wad-subsystem.md) |
| `cancel` and `backward` | PSP binds them to `circle` (index 4); PS2 binds them to `triangle` (index 6). `activate`/`forward` are `cross` on both. See [input.md](input.md) |
| `Input_ConsumePress` | PS2 clears the entire pressed mask and ignores the button index it is handed; the PSP page describes clearing one bit, at confidence 70. See [input.md](input.md) |
| The chase camera's 3/4 factor | The PSP page measures the external offsets reaching the eye at exactly 0.75 of their authored value, from an unfound source. No such factor exists on the PS2 path, which computes its distance by a visibly different method. See [camera.md](camera.md) |
| Runtime CRC table construction | PSP builds one table; PS2 has three separate lazily-built copies in three translation units, plus a fourth, statically initialised, standard-`crc32` table |

### Answered here, open on the PSP side

**`stats_base + 0x90` is `<Misc weight_distribution>`.**
[engine.md](../psp-pulse-usa/engine.md) records that offset as "a per-team scalar
outside every class block; its element is not determined". The PS2 binary has a
`<Misc>` element the PSP page never mentions, and its parser writes `0x90`
directly. See [handling-xml.md](handling-xml.md).

**Every PS2 texture is addressable by the name its own data declares.**
`oag_pulse::PS2_IMAGES` was written with the note that "how the game itself
performs this lookup is not known", after a sweep of 60 spellings that all
missed. The loader rewrites `.mip` and `.tga` to `.pct` before hashing, which
finds 47 of the 50 raster names in this executable and reproduces all three
hashes that sweep had to recover by correlating pictures. See
[texture-names.md](texture-names.md).

## Candidates, not yet confident enough to name

Left as `FUN_*` deliberately, per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md)'s below-50
rule or because the hypothesis is untested.

| Address | Hypothesis | Conf |
| --- | --- | ---: |
| `0x0014e600` / `0x0014e518` | The two `<Global>` sub-element parsers `HandlingXml_ParseGlobal` dispatches to. Shape is clear, contents unread | 55 |
| `0x00257740` | The stored (uncompressed) WAD stream constructor | 55 |
| `0x001fd8c0` | Registers an already-loaded buffer in the resource registry beside `Resource_Find` / `Resource_Load`. Same node layout, but no caller was traced | 60 |
| `0x001fed88` / `0x001fede8` / `0x001fee90` / `0x001fef20` | A third CRC-32 family with its own table at `0x003025e0`: an **uppercase**-folding string hash, a raw string hash, and a two-word hash. Not the WAD hash (wrong fold direction). **What uses it is now traced**: the front-end globals registry, through the wrapper `Hash_Crc32String` (`0x001fed38`) - see [fe-globals.md](fe-globals.md). The four listed here are still unrenamed | 60 |
| `0x0014f108` | Returns the current camera view setting, from `g_settings` or from a per-player array depending on a global that reads like an attract-mode switch | 55 |
| `0x00201f30` / `0x0010aec8` | The pad read. A thin shim over a library call; no import table was resolved for this binary | 50 |
| `0x002849a0` | A per-pad global that suppresses every button except cross. Reads like a menu or demo lockout | 40 |
| `0x002dab3c` | A global selecting between profile-backed and per-player camera settings. Attract mode is the obvious guess | 40 |

## Not attempted

No PS2 counterpart was looked for on
[main-loop.md](../psp-pulse-usa/main-loop.md),
[collision.md](../psp-pulse-usa/collision.md),
[frontend-video.md](../psp-pulse-usa/frontend-video.md), or the `Ship_Update*` force
terms in [engine.md](../psp-pulse-usa/engine.md). The force terms are the obvious
next target: the parameter block is now confirmed to be laid out identically,
so the PS2 consumers read the same offsets, and a second reading of the craft
frame would raise the whole of `engine.md` out of its decompilation-only cap.
