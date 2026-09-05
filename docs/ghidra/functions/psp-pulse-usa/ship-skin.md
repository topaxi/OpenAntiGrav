# `ship_alt.dat` / `ship_eliminator.dat` are four paletted textures, and the code that applies them

[`dlc-pack.md`](../../../formats/dlc-pack.md) and
[`handling-stats.md`](../../../formats/handling-stats.md) both left these two
files at a shape reading: a name, an `"ms"` tag, sixteen `RGBA8` entries at
`0x20`, then bytes in `0..15`. Paletted image data, dimensions unresolved, no
loader traced. This page traces the loader and settles all three of the
questions that were open - what precedes the payload, what the dimensions are,
and whether it is a texture upload.

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

The four texture functions sit in the 85-94 band on the strength of two
independent confirmations meeting at the byte (below); `Skin_ComposeQuarterAtlas`
is one band lower because *what* it computes is unambiguous and *what it is
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
| `0x0000` | `0x20` | header: NUL-terminated team name at `0x00`, ASCII `"ms"` at `0x08`, zero to `0x20` |
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

So `ship_eliminator.dat` is reached because **a global compared against `0x12`
holds a particular value**, not because anything was unlocked - which reads as
the Eliminator race mode being active. That splits what
[`dlc-pack.md`](../../../formats/dlc-pack.md)'s schema left as one question:
the `Alternative` skin is the `loyalty`-gated one, and the `Eliminator` skin
looks mode-selected. Confidence 70 on the split, 90 on the mechanical fact
that a `== 0x12` comparison is what selects the file.

**Neither global's address is resolved and neither is written down as one.**
Both `0x000578b0` (the `0x12` comparison) and the per-slot record array at
`0x00057c44` fall *inside* `.text` (`0x0` to `0x272a3c`), which is HANDOVER's
own tell that the relocation base is wrong for them - the same shape as the
`&DAT_0005abb0` and `&DAT_00057778` that `Vex_LoadModel` shows. They are most
likely `$gp`-relative; see
[the `$gp` thread](../../../../HANDOVER.md)'s entry for that addressing mode.
The structural claim above does not depend on either address.

## Callers

`Skin_ApplyToModel` has seven `jal` callers, found by the naive-target scan:
`0x08825638`, `0x088256b8`, `0x08828398`, `0x0882885c`, `0x08843804`,
`0x088b3af4` and `0x088eaa84`. Only the last-but-one is in
`MPLobby_UpdateShipPreview_q`, so skin application is a general facility and
not a lobby-only path - which is why the format page, not the lobby, is where
this belongs. Tracing the other six is what would settle the mode-vs-unlock
question properly, and is left open.

`MPLobby_UpdateShipPreview_q` itself has **no** `jal` caller, so it is reached
through a vtable or a function pointer. That is one of the reasons its own
confidence is below 70.
