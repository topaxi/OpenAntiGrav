# `ship_alt.dat` / `ship_eliminator.dat` are four paletted textures, and the code that applies them

[`dlc-pack.md`](../../../formats/dlc-pack.md) and
[`handling-stats.md`](../../../formats/handling-stats.md) both left these two
files at a shape reading: a name, an `"ms"` tag, sixteen `RGBA8` entries at
`0x20`, then bytes in `0..15`. Paletted image data, dimensions unresolved, no
loader traced. This page traces the loader and settles all three of the
questions that were open - what precedes the payload, what the dimensions are,
and whether it is a texture upload. It also **retires the `"ms"` tag**, which
was never a tag at all.

**It is a texture upload.** A skin file is four palette-plus-pixels blocks,
one per texture slot of the hull model, matched to the model's own texture
names. No second mesh is involved.

**Static reading of `psp-pulse-usa`'s `BOOT.BIN`, corroborated against the
sixteen shipped `.dat` files on `pulse-psp-usa.chd` and against
`Data\Ships\Assegai\Ship.vex`. Not runtime-verified.**

## The cast

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08912988` | `Skin_ApplyToModel` | 88 |
| `0x08911a00` | `Texture_UploadPaletted` | 86 |
| `0x08911b8c` | `Skin_ComposeQuarterAtlas` | 80 |
| `0x08911888` | `Texture_Downsample4bpp` | 84 |
| `0x088b34bc` | `MPLobby_UpdateShipPreview_q` | 62 |
| `0x08a88440` | `g_skin_texture_slot_names` | 88 |

`Skin_ApplyToModel` and `Texture_UploadPaletted` are in the rubric's 85-94
band, on the strength of two independent confirmations meeting at the byte
(below). The other two are deliberately in the 70-84 Probable band and are not
in the higher one: `Texture_Downsample4bpp` (84) is read off its body and its
two call sites with no external corroboration, and `Skin_ComposeQuarterAtlas`
(80) is lower again because *what* it computes is unambiguous and *what it is
for* is not. `MPLobby_UpdateShipPreview_q` is a sub-70 name and carries the
rubric's `_q`: it is certainly the site that builds a lobby slot's preview
ship, but it does several unrelated things in one body, has no `jal` caller to
corroborate the verb, and the subsystem prefix is read off a string set rather
than off a call graph.

## How the string was found: `get_xrefs_to` fails on a `.rodata` string both ways

The lead was `%s\ship_eliminator.dat` at `0x08a7ff7c`. `get_xrefs_to` on it
returns "No references found" - expected, per
[HANDOVER's relocation section](../../../../HANDOVER.md). **But it also
returns nothing for the naive form `0x0027bf7c`**, which that section's
"try the naive address first for a data reference" note predicts should work.
It does not, for a `.rodata` string: `search_instructions` is likewise empty
for `addiu`/`-0x4084`, for `lui`/`0x8a8`, and for the literal pointer bytes.
Two controls whose consumers are already known behave the same way -
`%s\Ship.vex` (`0x08a7ba64`, naive `0x00277a64`) and `Data\Psys\%s.POB`
(`0x08a884dc`, naive `0x002844dc`) both return nothing - so **no path template
in this binary has a working xref**, and an empty result proves nothing about
whether a string is used.

**Sanity-check the naive address against the section table before believing
it.** `.rodata` runs `0x274000` to `0x2ac5e4`, so a naive `.rodata` address
below `0x274000` means the subtraction is wrong, not that the reference is
missing - a mis-borrowed digit turns `0x002844dc` into `0x000844dc`, which
lands in `.text` and would have "confirmed" the wrong thing.

What worked was scanning the ELF's own `.text` outside Ghidra. `BOOT.BIN` is a
PRX (`e_type` `0xff80`) whose `.text` runs `0x0` to `0x272a3c` at file offset
`0x80`, holding unrelocated instructions; walking it four bytes at a time and
tracking each `lui`'s immediate until an `addiu`/`ori` completes the pair finds
the one site that forms `0x0027bf7c`:

```
088b3868  lui   a2, 0x28
088b386c  jal   0x0016e4b4                 ; -> 0x089724b4, snprintf
088b3870  addiu a2, a2, -0x4084            ; -> 0x0027bf7c = 0x08a7ff7c
```

so `snprintf(buf, 0x100, "%s\ship_eliminator.dat", node->0x94)`, `node->0x94`
being the same directory field every other path template in
[`vex.md`](../../../formats/vex.md#path-templates) uses. The same scan finds
`jal` callers by their naive target, which is how the seven callers of
`Skin_ApplyToModel` below were counted.

## The string block the caller works from

Reading `0x08a7fee0` onward names the whole neighbourhood, and the
multiplayer-lobby strings around it are what dates the caller:

| Address | String |
| --- | --- |
| `0x08a7ff14` | `shipStatus` |
| `0x08a7ff20` | `ONL_WFP` |
| `0x08a7ff28` | `ONL_MSG_TRANSFERRING_DATA` |
| `0x08a7ff44` | `MPLobby Info` |
| `0x08a7ff68` | `PID` |
| `0x08a7ff6c` | `%s\%s_FE.vex` |
| `0x08a7ff7c` | `%s\ship_eliminator.dat` |
| `0x08a7ff94` | `ship` |
| `0x08a7ff9c` | `Normal` |
| `0x08a7ffa4` | `%s\%s.dat` |

Three of those are the `PI_TeamModel` schema showing through from the other
side: `ship` is the `location` attribute of the `Normal` variant, `Normal` is
that variant's `name`, and `%s\%s.dat` is the generic template the specific
`%s\ship_eliminator.dat` is a special case of. `MPLobby_UpdateShipPreview_q`
builds `%s\%s_FE.vex` from the same directory and the variant's `location`,
which is the front-end preview model
[`handling-stats.md`](../../../formats/handling-stats.md) already lists.

## The skin file's layout

`Skin_ApplyToModel(model, payload, flag)` at `0x08912988` walks the model's
textures. For each one it takes the last path component of the texture's name
(`strrchr(name, '\\')`) and compares it against a four-entry, sixteen-byte-stride
table of names at `g_skin_texture_slot_names` (`0x08a88440`):

| Table entry | Name | Payload offset it selects |
| --- | --- | --- |
| `0x08a88440` | `\TEXTURE1.TGA` | `0x20` |
| `0x08a88450` | `\TEXTURE2.TGA` | `0x2060` |
| `0x08a88460` | `\TEXTURE3.TGA` | `0x40a0` |
| `0x08a88470` | `\TEXTURE4.TGA` | `0x60e0` |

The comparison is **case-insensitive**: the table spells the names in upper
case and the shipped models spell them in lower case
(`Data\Ships\Assegai\Textures\texture1.tga`), and the match demonstrably
happens.

Those four offsets are plain immediates in the function, and they are the whole
format. With a `0x2040` stride for the first three and the file's own size for
the last:

| Offset | Size | Content |
| --- | --- | --- |
| `0x0000` | `0x20` | header: the team's **display** name, NUL-terminated. Everything after the terminator is exporter residue, not a field - see below |
| `0x0020` | `0x40` | palette 1 - sixteen `RGBA8` entries |
| `0x0060` | `0x2000` | texture 1 - 128x128, 4 bits per pixel, two pixels per byte |
| `0x2060` | `0x40` | palette 2 |
| `0x20a0` | `0x2000` | texture 2 - 128x128, 4bpp |
| `0x40a0` | `0x40` | palette 3 |
| `0x40e0` | `0x2000` | texture 3 - 128x128, 4bpp |
| `0x60e0` | `0x40` | palette 4 |
| `0x6120` | `0x800` | texture 4 - 64x64, 4bpp |
| | `0x6920` | total, 26912 bytes |

**Two independent confirmations meet at the exact byte.** The code side:
`Skin_ComposeQuarterAtlas` reads a source block as `0x80` by `0x80` and steps
its source pointer by `0x2040` per block, so the stride and the 128x128 are
immediates, not inferences. The data side: every one of the sixteen
`ship_alt.dat` / `ship_eliminator.dat` entries in `pulse-psp-usa.chd`'s
`Data.wad` is **exactly 26912 bytes**, which is `0x20 + 3*0x2040 + 0x840` with
nothing left over, and a hexdump of `Data\Ships\Assegai\ship_alt.dat` shows a
run of sixteen `RGBA8` entries beginning at each of `0x20`, `0x2060`, `0x40a0`
and `0x60e0`. `ship_eliminator.dat` for the same team is byte-for-byte the same
shape, header included.

### The `"ms"` tag does not exist - it is residue in a reused export buffer

Both earlier shape readings recorded "a two-byte `"ms"` tag" at `0x08`, off
`Assegai\ship_alt.dat`. That file really does read `Assegai\0ms\0`. But
`Assegai` is seven characters plus a terminator - exactly eight - so it is the
one team whose name cannot distinguish a fixed-width field from a terminated
one. Dumping the header of all eight base teams' `ship_alt.dat` settles it:

| Team | First sixteen header bytes |
| --- | --- |
| AG Systems | `AG Systems\0` then zero |
| Assegai | `Assegai\0` then `ms\0` then zero |
| EGX | `EGX\0` then zero |
| Feisar | `Feisar\0` then zero |
| Goteki | `Goteki\0` then zero |
| Piranha | `Piranha\0` then zero |
| Qirex | `Qirex\0` then `a\0` then zero |
| Triakis | `Triakis\0` then zero |

Six of the eight have nothing after the terminator at all, which alone disposes
of the tag. The two that do are explained exactly: **AG Systems is ten
characters and its last two are `ms`**, so writing the shorter `Assegai\0` over
a buffer that last held `AG Systems\0` leaves `m`, `s` at `0x08` and `0x09`
untouched. Qirex is the same trick one letter wide - `Piranha`'s `a` at index 6
survives `Qirex\0`. Nothing reads these bytes; the header is a `0x20`-byte
field holding one NUL-terminated string and nothing else.

Two consequences for an implementation. **A parser must not validate a tag at
`0x08`** - it would reject AG Systems, which has real name data there. And the
string is the team's **display** name, not its directory name: `AG Systems`
with a space, where the directory is `AG_Systems` with an underscore.

### There is no dimension field in the file

This is the single most important sentence for anyone writing a parser.
`Texture_UploadPaletted(texture, block)` at `0x08911a00` reads the geometry
from the **target texture's own descriptor** at `texture+0x9c`, never from the
block:

| Descriptor field | Meaning |
| --- | --- |
| `+0x00` `u16` | width |
| `+0x02` `u16` | height |
| `+0x05` `u8` | mip level count |
| `+0x08` `u32` | palette byte length, *and* the offset of the pixels within the block |
| `+0x14` | palette destination |

It then does exactly two copies - `memcpy(clut, block, desc[8])` and
`memcpy(pixels, block + desc[8], width * height / 2)` - generates the mip chain
by repeated `Texture_Downsample4bpp` rather than reading it, and finally calls
the format setter with `(width, height, 4, 1)`. The `/2` is what fixes 4bpp;
the `4` is the pixel format. So "sixteen-entry palette" is a *consequence* of
the target texture being 4-bit indexed, not a declaration the file makes, and
the `0x40`/`0x2000` block sizes are the sizes that the hull's own textures
happen to have. A parser that searches these files for a width or a height
will find nothing.

`Texture_Downsample4bpp(dst, palette, src, width, height)` at `0x08911888` is a
2x2 box filter that stays in index space: for each output pixel it reads four
source indices - two adjacent nibbles from each of two adjacent rows - and asks
a palette-aware helper for the index nearest their average, packing the results
back two per byte.

## The fourth slot is composed, not copied, when the flag is zero

`Skin_ApplyToModel`'s one special case: when the matched slot is `\TEXTURE4.TGA`
**and** its `flag` argument is zero, it does not upload `payload + 0x60e0`. It
calls `Skin_ComposeQuarterAtlas` into a 2112-byte (`0x840`) stack buffer - the
size of a 64x64 4bpp texture plus its palette - and uploads that instead. Any
other slot, or a non-zero flag, uploads the stored block directly.

What it composes: for each of the first three blocks it downsamples 128x128 to
64x64 to 32x32, then blits the 32x32 result into a 64x64 destination whose row
stride is `0x20` bytes. Reading the three destination offsets as a linear
layout, block 3 lands top-left (`+0x40`), block 1 bottom-left (`+0x440`) and
block 2 bottom-right (`+0x450`).

**Two details to reproduce rather than correct**, in the idiom
[`vex.md`](../../../formats/vex.md) already uses for `Loopend`:

- The composite's palette is **block 1's** palette, copied once at the top, and
  block 1's palette is also the one passed to all three downsamples. Blocks 2
  and 3 are re-indexed into it rather than carrying their own.
- The **top-right quadrant is never written**. The destination is an
  uninitialised stack buffer, so whatever is there is stack garbage. An
  implementation must not invent content for it.

**One caveat on the quadrant mapping.** It is stated above for a linear
texture layout. The format setter's trailing argument (`1`) may be a swizzle
flag, in which case the byte offsets map to different screen positions. The
three offsets and the stride are certain; which corner each lands in is
conditional on that.

**The shipped assets corroborate the linear reading without settling it.**
2026-09-09, measured off `pulse-psp-usa.chd` by
`crates/game/examples/pulse_skin_probe.rs`: Assegai's *own* `texture4.tga`,
the 64x64 slot the composite would replace, is itself a quarter atlas -
three quadrants of hull panels and a near-black top-right (channel mean 16.7
against 154.4, 149.0 and 148.9 for the other three). The `Alternative` skin's
stored block 4 has the same shape (42.6 against 193.5, 187.7 and 182.4). So
the *authored* data is laid out exactly the way the composite is read here,
empty corner included, which is what a linear layout with an unwritten
top-right predicts. It is corroboration and not proof: a swizzle that happened
to map the unwritten tile onto the same corner would look identical from the
outside. The caveat stands and the composite is still not reproduced - see
"What this build does" below.

**What the composite is for is not traced.** The natural reading is a
distant-ship / LOD sheet - one small texture in place of three large ones - and
the asset side is consistent with it: `Data\Ships\Assegai\Ship.vex`, the race
hull, references `texture1.tga` through `texture4.tga`, while
`Data\Ships\Assegai\ship_FE.vex`, the front-end preview, references only
`texture1` through `texture3` and so never matches the fourth slot at all.
That is consistent with the LOD reading and does not establish it.
Confidence 60 on the purpose; the mechanism above is not affected either way.

## The eliminator skin is selected by a mode check, not by an unlock

`MPLobby_UpdateShipPreview_q` chooses which skin file to load like this:

```
if (g_unresolved_flag == 0 && g_unresolved_mode == 0x12)
    snprintf(path, 0x100, "%s\ship_eliminator.dat", dir);
else if (slot_record->name[0] != 0)
    strcpy(path, slot_record->name);
```

and only then, if the variant's `location` is `ship`, loads `path` as a
resource and hands its payload to `Skin_ApplyToModel`. If that load fails it
falls back: it walks the team node's children for the one named `Normal` and
rebuilds the path from the generic `%s\%s.dat` template.

So `ship_eliminator.dat` is reached because **a global holds a particular
value**, not because anything was unlocked. Confidence 90 on that much: an
equality test against a plain immediate is what selects the file, so the
selection is state-driven and there is no unlock check anywhere on this path.

**What `0x12` denotes is not established, and is weaker than it looks.**
Eliminator being a race mode makes "the Eliminator mode is active" the natural
reading, but there is **no recovered Pulse mode-id table to check `0x12`
against** - `oag-race`'s `Mode` has no Eliminator variant, and the twenty-two
mode ids that are documented belong to HD and 2048, a different engine. The
enclosing function is multiplayer-lobby code, so a lobby game-type id is just
as consistent with the evidence as a race-mode id. Confidence 55 on the mode
reading; it is a hypothesis to test, not a finding to build on.

What that does settle, at confidence 75, is a split in what
[`dlc-pack.md`](../../../formats/dlc-pack.md)'s schema left as one question:
the two skins are not reached the same way. `Alternative` is the one the
`loyalty` thresholds gate; `Eliminator` is selected by this state check, which
never consults an unlock.

**Neither global's address is resolved and neither is written down as one.**
Both `0x000578b0` (the `0x12` comparison) and the per-slot record array at
`0x00057c44` fall *inside* `.text` (`0x0` to `0x272a3c`), which is HANDOVER's
own tell that the relocation base is wrong for them - the same shape as the
`&DAT_0005abb0` and `&DAT_00057778` that `Vex_LoadModel` shows.

**They are not `$gp`-relative, and that earlier hypothesis is retracted**: this
binary never uses `$gp` as a load/store base. They need the segment-1 base, and
`scripts/psp-relocate.py resolve <instruction-address>` reads it straight off
the relocation record rather than deriving it. That is the first thing to try;
tracing the six non-lobby callers listed under "Callers" below settles the same
question without either address if the resolve comes back empty. The structural
claim above does not depend on either address.

## What this build does, and it is chosen rather than measured

**This build has the player name the skin, and paints the player's craft
alone.** `--skin Alternative` / `--skin Eliminator` on `oag-game`, resolved
against what the team's own `PI_ModelSkin` declares and applied to grid slot 0
only. **No confidence score, because nothing was measured to arrive at it** -
it is a stand-in on the same footing `crates/game/src/livery.rs`'s
`teams_for_slots` already stands on, and it is labelled so in the load report
every run prints.

**No unlock is checked either.** What `loyalty` accumulates is untraced -
per-save or per-team, and what the `Team="any"` tier changes about the check -
so every declared skin is offered rather than gated on a number nothing can
interpret yet.

What would replace the choice, in order of cheapness:

1. `scripts/psp-relocate.py resolve <instruction-address>` on the `0x12`
   comparison, to name the global and then find what writes it.
2. Tracing the six non-lobby callers of `Skin_ApplyToModel` under "Callers".

Until one of those lands, a mode gate built on `0x12` would be a guess dressed
as a reproduction, which is exactly what the confidence rubric exists to stop.

**The composite fourth slot is not reproduced**: the *stored* block 4 is
uploaded instead, which is the applier's own non-zero-flag path and needs no
answer to the swizzle question above. The shipped block 4 is already a quarter
atlas (see the measurement under "The fourth slot is composed"), so this is
disc data on the hull rather than a substitute for it.

## Callers

`Skin_ApplyToModel` has seven `jal` callers, found by the naive-target scan:
`0x08825638`, `0x088256b8`, `0x08828398`, `0x0882885c`, `0x08843804`,
`0x088b3af4` and `0x088eaa84`. The six that are **not** the lobby's -
`0x08825638`, `0x088256b8`, `0x08828398`, `0x0882885c`, `0x08843804` and
`0x088eaa84` - are the ones a trace should start from. Only the last-but-one is in
`MPLobby_UpdateShipPreview_q`, so skin application is a general facility and
not a lobby-only path - which is why the format page, not the lobby, is where
this belongs. Tracing the other six is what would settle the mode-vs-unlock
question properly, and is left open.

`MPLobby_UpdateShipPreview_q` itself has **no** `jal` caller, so it is reached
through a vtable or a function pointer. That is one of the reasons its own
confidence is below 70.

## The PS2 port does not use this layout

The PS2 build never reads a `.dat`'s bytes: its `Skin_SwapAtlasSibling`
maps the file name to a whole sibling atlas beside the hull's one
`ALL_Textures.tga` and copies that over it. See
[`ps2-pulse-eu/ship-skin.md`](../ps2-pulse-eu/ship-skin.md).
