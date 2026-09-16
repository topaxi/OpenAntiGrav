# PSP memory map

Where Pulse's PSP build sits in memory: the fixed part the ELF brings with
it, the part the kernel hands over at boot, and how the game carves the
latter. This is the static half of M2's "memory management and the heap
layout"; the allocator and the heap sizes are read on
[memory.md](../functions/psp-pulse-usa/memory.md).

All addresses are the USA `BOOT.BIN` (UCUS-98712) as loaded at
`0x08804000`; the EU build differs in every section boundary but not in
the shape. Section ranges are Ghidra's, from the ELF's own headers;
confidence 95 for them, since they are read from the file and not inferred.

## The console

| Range | Size | What | Notes |
| --- | ---: | --- | --- |
| `0x00010000` - `0x00013fff` | 16 KiB | scratchpad | not referenced by this binary |
| `0x04000000` - `0x041fffff` | 2 MiB | VRAM | the GE's frame buffers, depth buffer and any texture the game uploads; `g_display` ([main-loop.md](../functions/psp-pulse-usa/main-loop.md)) owns the split |
| `0x08000000` - `0x087fffff` | 8 MiB | kernel partition 1 | firmware; user code never sees it |
| `0x08800000` - `0x09ffffff` | 24 MiB | **user partition 2** | where everything below lives; 24 MiB on every retail PSP for a UMD game (the extra 32 MiB on later models is not granted to a game that does not ask) |
| `0x08800000` - `0x08803fff` | 16 KiB | loader reserve | between the partition base and the ELF |

The game asks the kernel for memory exactly twice through the sizes on
[memory.md](../functions/psp-pulse-usa/memory.md): once with
`sceKernelCreateFpl` for the main heap and once for the data heap, both from
partition 2. Everything else the kernel allocates on the game's behalf -
thread stacks, the `sceMpeg`/`sceAtrac` work areas, `sceUtility` dialogs -
comes out of the 1,126,400 bytes the two sizes leave free.

## The ELF, section by section

`BOOT.BIN` is a plain relocatable PRX-style ELF (no encryption; see
[the disc layout](../../psp/pulse-disc-layout.md)). Loaded, it spans
`0x08804000` - `0x08b95abf`: **3,742,400 bytes**, 3.57 MiB, of which
2.45 MiB is code.

| Section | Start | End | Bytes |
| --- | --- | --- | ---: |
| `.text` | `0x08804000` | `0x08a76a3b` | 2,566,716 |
| `.sceStub.text.*` (39 blocks, one per imported library) | `0x08a76a3c` | `0x08a774b3` | 2,680 |
| `.lib.ent` / `.lib.stub` | `0x08a774b4` | `0x08a777cb` | 792 |
| `.rodata.sceModuleInfo` / `.sceResident` / `.sceNid` | `0x08a777cc` | `0x08a77fff` | 2,100 |
| `.rodata` | `0x08a78000` | `0x08ab05e3` | 230,884 |
| `.data` | `0x08ab0600` | `0x08ad9717` | 168,216 |
| `.eh_frame` | `0x08ad9718` | `0x08ad978b` | 116 |
| `.dtors` | `0x08ad978c` | `0x08ad9793` | 8 |
| `.cplinit` | `0x08ad9798` | `0x08ad9e8f` | 1,784 |
| `.linkonce.d` | `0x08ad9e90` | `0x08adb297` | 5,128 |
| `.ctors` | `0x08adb298` | `0x08adb2a3` | 12 |
| `.bss` | `0x08adb300` | `0x08b95abf` | 763,840 |

Three of those deserve a sentence:

- **`.sceStub.text.*`** is the import table: 335 stubs, 306 resolved by
  NID, one 8-byte `jr $ra` pair each, grouped by library
  ([imports.md](../functions/psp-pulse-usa/imports.md)). The
  library list is itself a finding - `sceNp`, `sceNpAuth`, `sceHttp`,
  `sceSsl`, `sceNetInet`, `sceNetAdhoc*` are all here, so the binary links
  the infrastructure and ad hoc stacks even though no network PRX ships on
  the disc ([networking](../../networking/README.md) has the disc-side reading).
- **`.cplinit`** is the SDK's C++ static-initialiser list: 223 entries of
  (function pointer, 0), 8 bytes each, ascending by address. Entry 179 at
  `0x08ad9d30` is `Mem_Init`, which is how the main heap exists before
  `Game_Bootstrap` runs - it answers the "what table is this" question
  [memory.md](../functions/psp-pulse-usa/memory.md) left open. The
  GCC `.ctors` beside it holds just three words.
- **`.bss`** at 746 KiB is where every named global lives; the map of it
  below is the useful part of this page.

## `.rodata` and `.data`: the tables

What the static data holds, by the globals already named
([names.tsv](../functions/psp-pulse-usa/names.tsv)):

| Address | Name | Page |
| --- | --- | --- |
| `0x08a7c098` | `g_plasma_charge_seconds` (the hard-coded `1.0f`) | [plasma.md](../functions/psp-pulse-usa/plasma.md) |
| `0x08a84cf0` - `0x08a84cfc` | bloom buffer dimensions and scratch | [bloom.md](../functions/psp-pulse-usa/bloom.md) |
| `0x08a90d20` | `_ctype_` (newlib) | [wad-subsystem.md](../functions/psp-pulse-usa/wad-subsystem.md) |
| `0x08ab062c` / `0x08ab067c` | mode and class name tables (front end) | |
| `0x08ab0788` | `g_game_mode_names`, the 19-entry enum | [state-machine.md](../functions/psp-pulse-usa/state-machine.md) |
| `0x08ab07e3` | `g_debug_mode_override`, never written | [state-machine.md](../functions/psp-pulse-usa/state-machine.md) |
| `0x08ab0a90` - `0x08ab0e1c` | grid orders, tournament points, Zone milestones, SAP clamps, class gravity scale, craft scale | [grid.md](../functions/psp-pulse-usa/grid.md), [tournament.md](../functions/psp-pulse-usa/tournament.md), [zone-mode.md](../functions/psp-pulse-usa/zone-mode.md) |
| `0x08ab0eac` / `0x08ab0ec0` | weapon AI rate tables | [weapon-ai.md](../functions/psp-pulse-usa/weapon-ai.md) |
| `0x08ab1f88` | `g_unlock_all_code` | [state-machine.md](../functions/psp-pulse-usa/state-machine.md) |
| `0x08ab2370` - `0x08ab4be4` | `g_vex_class_table`, 863 records of 12 bytes | [vex.md](../../formats/vex.md) |
| `0x08abf448` - `0x08abf474` | the loading thread's state and wave tables | [loading-screen.md](../functions/psp-pulse-usa/loading-screen.md) |
| `0x08abf500` - `0x08abf54c` | environment-map light basis | [resource-loading.md](../functions/psp-pulse-usa/resource-loading.md) |
| `0x08abf5cc` - `0x08abf5d9` | `g_display` and the three never-written fixed-step bytes | [main-loop.md](../functions/psp-pulse-usa/main-loop.md) |
| `0x08ac1dd8` - `0x08ac1f44` | the VFS, resource, node and heap globals, contiguous | [resource-loading.md](../functions/psp-pulse-usa/resource-loading.md), [memory.md](../functions/psp-pulse-usa/memory.md) |
| `0x08ac29d4` | `_impure_ptr` (newlib's reent, `rand` state at `+0x58`) | [prng.md](../functions/psp-pulse-usa/prng.md) |
| `0x08ac326c` - `0x08ac4a2c` | the Scream opcode, semitone, fine and pan tables | [sound.md](../functions/psp-pulse-usa/sound.md) |
| `0x08ac7ab0`, `0x08ad0fb4`, `0x08ad3314` | vtables: in-game, anim transform, WAD device | |
| `0x08ad9d30` | `Mem_Init`'s `.cplinit` entry | above |

The `0x08ac1dd8` - `0x08ac1f44` run is worth noticing as a block: music
player, VFS device and file lists, resource list and semaphore, node count,
pending-destroy count, heap list, main and data heap pointers, heap
semaphore - one translation unit's statics, laid out in declaration order,
which is why reading one of them tends to find the next.

## `.bss`: the live layout

| Address | Name | Bytes | Page |
| --- | --- | ---: | --- |
| `0x08af25c0` | `g_loading_tip` | | [loading-screen.md](../functions/psp-pulse-usa/loading-screen.md) |
| `0x08afbffc` | `g_wad_crc_table` | 1,024 | [wad-subsystem.md](../functions/psp-pulse-usa/wad-subsystem.md) |
| `0x08b04880` | `g_heap_dump_buffer` | ~4,096 | [memory.md](../functions/psp-pulse-usa/memory.md) |
| `0x08b059f0` | `g_pad_buffer` | | [input.md](../functions/psp-pulse-usa/input.md) |
| `0x08b30f00` | `g_game` | | [main-loop.md](../functions/psp-pulse-usa/main-loop.md) |
| `0x08b30f10` | `g_pending_destroy_nodes` | 128 | [resource-loading.md](../functions/psp-pulse-usa/resource-loading.md) |
| `0x08b30f90` | the game-state block: `+0xb0` speed class, `+0xb8` `g_game_mode`, `+0x18` weapons/damage bytes | | [state-machine.md](../functions/psp-pulse-usa/state-machine.md), [pads.md](../functions/psp-pulse-usa/pads.md) |
| `0x08b31780` / `0x08b31784` | `g_input`, `g_state_machine` (pointers) | 8 | [main-loop.md](../functions/psp-pulse-usa/main-loop.md) |
| `0x08b32c60` | `g_bloom` | | [bloom.md](../functions/psp-pulse-usa/bloom.md) |
| `0x08b34310` - `0x08b34360` | camera FOV and Zone recharge | | [camera.md](../functions/psp-pulse-usa/camera.md), [zone-mode.md](../functions/psp-pulse-usa/zone-mode.md) |
| `0x08b36be0` - `0x08b36bfc` | the handling-parse scratch: autospeed, jumps, barrel roll | | [engine.md](../functions/psp-pulse-usa/engine.md), [input-bindings.md](../functions/psp-pulse-usa/input-bindings.md) |
| `0x08b620a8` - `0x08b620f4` | the particle RNG | 80 | [prng.md](../functions/psp-pulse-usa/prng.md) |
| `0x08b62c08` | the fallback class descriptor | | [vex.md](../../formats/vex.md) |
| `0x08b66444` | `g_vfs_file_sema` | | [resource-loading.md](../functions/psp-pulse-usa/resource-loading.md) |
| `0x08b66450` | `g_decompress_staging_buffer` | | [wad-subsystem.md](../functions/psp-pulse-usa/wad-subsystem.md) |
| `0x08b67610` | `g_small32_bitmap` | 80 | [memory.md](../functions/psp-pulse-usa/memory.md) |
| `0x08b67660` | `g_small32_pool` | 20,480 | [memory.md](../functions/psp-pulse-usa/memory.md) |
| `0x08b6c660` | `g_small16_bitmap` | 752 | [memory.md](../functions/psp-pulse-usa/memory.md) |
| `0x08b6c950` | `g_small16_pool` | 96,256 | [memory.md](../functions/psp-pulse-usa/memory.md) |
| `0x08b84150` | `g_main_heap_storage` (the heap descriptor, 24 bytes) | 24 | [memory.md](../functions/psp-pulse-usa/memory.md) |
| `0x08b84168` | `g_small_heap_sema` | 4 | [memory.md](../functions/psp-pulse-usa/memory.md) |
| `0x08b95ac0` | end of `.bss` | | |

The small-block pools alone are 117 KiB of the 746 KiB, the largest single
thing in `.bss` this project has named.

## The heap, after boot

What the kernel gives back is not at a fixed address - it is whatever
`sceKernelCreateFpl` returns - but its size is a closed form, and on a
PSP-1000 the arithmetic is worth writing down once:

```
partition 2                                      25,165,824   (24 MiB)
  - loader reserve                                  -16,384
  - the ELF, .text through .bss                  -3,742,400
  - main thread stack and kernel bookkeeping        (varies, order of 300 KiB)
  = sceKernelMaxFreeMemSize() at Mem_Init          ~21.0 MiB
  - 0x7e49b0 kept back                            -8,276,400
  = main heap                                      ~12.7 MiB     (0xc20650 = 12,715,600)
data heap, created on first FEData/BEData load     7,150,000   (0x6d19b0)
left for the kernel's own allocations              1,126,400   (0x113000)
```

The "measured value became the sentinel" reading of `0xc20650` on
[memory.md](../functions/psp-pulse-usa/memory.md) rests on that
`~12.7 MiB` line coming out where the sentinel is; it is not confirmed by a
runtime read of `sceKernelMaxFreeMemSize`, which is the obvious next
measurement (PPSSPP logs the FPL creation with its size, and
`Mem_LogKernelFree`'s `printf` reaches the emulator's log).

Inside the main heap, the first thing allocated from the bottom is the game
object (`Game_MainLoop`'s `Mem_Alloc(0x48)`), then its four root children;
the front end's `FEData.wad` image is placed at the **top of the data
heap** by the from-top mode. So a live memory dump reads: small objects
climbing from the main heap's base, the 7 MB archive pinned at the top of
the second FPL, and everything the front end allocates while `FEData` is
resident filling the data heap upward from *its* base.

## History

- 2026-09-16: first version, from Ghidra's section list, the named globals
  and the heap constants; the partition arithmetic is the one part not read
  from the binary.
