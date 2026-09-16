# PS2 memory map

Where Pulse's PS2 build sits in memory. The static half - the ELF's own
sections and the Emotion Engine's fixed regions - is read; the runtime
half is **not the PSP's allocator** and is recorded here only as far as
one function and its strings show, so that nobody carries
[the PSP heap reading](../functions/psp-pulse-usa/memory.md) across by
assumption.

All addresses are `SCES_547.48` (EU, the only PS2 release) at image base
`0x00100000`. Section ranges are Ghidra's, from the ELF headers; confidence
95 for them.

## The console

The Emotion Engine's map, as this binary uses it. Ghidra's import maps the
`.data`/`.rodata` region three times, which is why a string search returns
every literal at `0x002xxxxx`, `0x202xxxxx` and `0x303xxxxx`
([ps2-pulse-eu/README.md](../functions/ps2-pulse-eu/README.md)); the
canonical form is the cached one.

| Range | Size | What |
| --- | ---: | --- |
| `0x00000000` - `0x000fffff` | 1 MiB | kernel; the ELF is loaded above it |
| `0x00100000` - `0x01ffffff` | 31 MiB | user RAM, cached view - the ELF and everything it allocates |
| `0x20000000` - `0x21ffffff` | | the same RAM, uncached |
| `0x30100000` - `0x31ffffff` | | the same RAM, uncached and accelerated |
| `0x10000000` - `0x1000ffff` | 64 KiB | EE registers (timers, DMAC, VIF, GIF, IPU) |
| `0x11000000` - `0x11000fff` / `0x11004000` - `0x11004fff` | 4 KiB / 4 KiB | VU0 code / data |
| `0x11008000` - `0x1100bfff` / `0x1100c000` - `0x1100ffff` | 16 KiB / 16 KiB | VU1 code / data - where `.vutext` is uploaded |
| `0x12000000` - `0x12001fff` | 8 KiB | GS privileged registers |
| `0x1c000000` - `0x1dffffff` | 2 MiB | IOP RAM as the EE sees it - `SCREAM.IRX` and the other eight modules live here ([disc layout](../../ps2/pulse-disc-layout.md)) |
| `0x70000000` - `0x70003fff` | 16 KiB | scratchpad |
| `0x9fc00000` / `0xbfc00000` | 4 MiB | BIOS, cached / uncached |

The GS's own 4 MiB of VRAM is not in the EE address space at all; it is
reached through GIF packets, which is why the frame-buffer and texture
layout is a [rendering](../functions/ps2-pulse-eu/batch-draw-state.md)
question rather than a memory-map one.

## The ELF, section by section

`SCES_547.48` is a plain EE ELF. Loaded, it spans
`0x00100000` - `0x003586ff`: **2,459,392 bytes**, 2.35 MiB - smaller than
the PSP build's 3.57 MiB despite being the same game, because the
Allegrex build carries 1 MiB more code (`2,566,716` against `1,509,504`
bytes of `.text`): the PS2 port moved work into VU microcode and the IOP
modules, and the PSP build links the whole network stack the PS2 does not
have.

| Section | Start | End | Bytes |
| --- | --- | --- | ---: |
| `.text` | `0x00100000` | `0x0027087f` | 1,509,504 |
| `.vutext` | `0x00270880` | `0x0027a53f` | 40,128 |
| `.data` | `0x0027a580` | `0x0029cdff` | 141,440 |
| `.rodata` | `0x0029ce00` | `0x002da687` | 252,040 |
| `.gcc_except_table` | `0x002da700` | `0x002da80f` | 272 |
| `.sdata` | `0x002da880` | `0x002da8d7` | 88 |
| `.sbss` | `0x002da900` | `0x002da9f0` | 241 |
| `.bss` | `0x002daa00` | `0x003586ff` | 515,328 |

Plus **28 `.DVP.overlay` sections**: the VU microprograms, 2 KiB each at
most, three programs (`11805059`, `3007363`, `89293187` in Ghidra's
naming, by their content hash) sliced into 2 KiB pages at VU-space offsets
`0x0`, `0x800`, `0x1000` ... `0x3800`. `.vutext` is the packed source they
are uploaded from. Nothing on the PSP corresponds to them; this is the
geometry pipeline the PS2 port re-implemented on the vector units.

Note the ordering: on the PS2 `.data` comes **before** `.rodata`, the
reverse of the PSP layout, and `.sdata`/`.sbss` exist at all because the
EE toolchain uses `$gp`-relative small-data addressing. A reader carrying a
"data is above rodata" habit from the PSP binary will misjudge which
section an address is in.

## What is placed where

The PS2 globals that place a region. Only the vtable row is in the PS2's
[names.tsv](../functions/ps2-pulse-eu/names.tsv); the other three were read
on the pass that wrote this page and are recorded as prose on the cited
pages, not as applied names - a PS2 evidence page for them is still owed.

| Address | What | Section | Where it is read |
| --- | --- | --- | --- |
| `0x0027b3e4` | `_impure_ptr` (newlib reent; `rand` state at `+0x58`) | `.data` | [prng.md](../functions/psp-pulse-usa/prng.md), cross-platform row |
| `0x00284e08` | the memory manager object (below) | `.data` | this page |
| `0x0029c620` | `g_wad_device_vtable` (named) | `.data` | [wad-subsystem.md](../functions/ps2-pulse-eu/wad-subsystem.md) |
| `0x002f8a60` - `0x002f8aac` | the particle RNG buffer, pointers and scale | `.bss` | [prng.md](../functions/psp-pulse-usa/prng.md), cross-platform row |

The rest of the PS2's named data is on the pages under
[ps2-pulse-eu/](../functions/ps2-pulse-eu/README.md).

## The runtime heap is a different design

**Do not read the PSP's `Mem_Alloc` into this binary.** The PSP's guard
words (`0xfeadfead`, `0xab12ab34`, `0x43658721`) appear nowhere in the
PS2 executable - `search_instructions` for `lui 0xfead` and `lui 0x4365`
both return zero - and its `"OUT OF MEMORY - need %d bytes"` string is
absent. What is present instead:

- `"Fatally out of memory trying to allocate %d bytes from %s\nHeap has
  %d bytes allocated in %d allocations\nHeap sizes: %dKb large, %dKb
  small\n"` at `0x002c6c50`, formatted by `FUN_0020a360`, which then
  spins for ever - the same deliberate-hang policy the PSP's bad-free path
  has, but on allocation failure.
- That function reads a manager object at `DAT_00284e08` through **two
  embedded sub-objects with virtual accessors** - the object itself
  (`+0x08` vtable, accessors at slots `+0x8c`, `+0x94`, `+0x9c`) and a
  second at `+0x84` (own vtable at `+0x8c`) - and reports their combined
  totals as "large" and "small". So the PS2 keeps **one manager with a
  large heap and a small heap as sub-objects**, reached through vtables,
  where the PSP keeps a linked list of plain descriptors and two bitmap
  pools.
- `"Wipeout is out of memory"` and `"Special heap is out of memory"` at
  `0x0029e660` / `0x0029e680` name a third, "special" heap that has no
  PSP counterpart either.

None of it is read past that one function. It is recorded so the
divergence is on the page: the PSP is the reference implementation, and
this is one of the subsystems where the PS2 port is *not* corroboration
([ADR-0009](../../architecture/adr/0009-multi-game-fanout.md) has the policy).
A reader of the PS2 allocator should start at `FUN_0020a360`'s callers and
the vtable at `DAT_00284e08 + 8`.

## History

- 2026-09-16: first version, from Ghidra's section list and one function.
  The heap design divergence is a negative result (three searches, one
  string set) plus one positive read; confidence 85 that the PS2 allocator
  is a different implementation, 0 for its layout.
