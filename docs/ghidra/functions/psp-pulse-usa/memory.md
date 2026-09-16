# Memory management and the heap layout

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`, language `Allegrex:LE:32:default`.

**The names below are applied**, from [names.tsv](names.tsv) via
`just apply-names`. Per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md), applying a
rename needs a page carrying its evidence; this page is that evidence.
Nothing here is under 70, so nothing carries a `_q`.

The engine does not `malloc`. Every allocation goes through one front door,
`Mem_Alloc(size, file, line)`, which tries a **small-block bitmap pool** for
anything up to 32 bytes and otherwise walks a **linked list of heaps**, each a
single kernel FPL block carved by a first-fit free-list allocator with a
16-byte guarded header per block. There are exactly **two heaps** in a
running game - the main heap, sized off `sceKernelMaxFreeMemSize()` at
static-constructor time, and a 7,150,000-byte **data heap** that is created
on demand for whichever of `FEData.wad` / `BEData.wad` is resident - and the
way an archive is steered into the data heap is by **temporarily replacing
the heap list head** around the mount. That trick, the from-top allocation
mode the resident WAD image uses, and the sizes are the memory layout.

## Summary

| Address | Name | Kind | Conf |
| --- | --- | --- | ---: |
| `0x08946c58` | `Mem_Init` | function | 90 |
| `0x08946e40` | `Mem_Alloc` | function | 95 |
| `0x08946e20` | `Mem_AllocUntagged` | function | 88 |
| `0x08946f0c` | `Mem_AllocFromTop` | function | 92 |
| `0x08947044` | `Mem_AllocAligned` | function | 92 |
| `0x08946fd8` | `Mem_AllocAlignedFromTop` | function | 92 |
| `0x089470b0` | `Mem_Free` | function | 95 |
| `0x08945ef4` | `Mem_LogStatus` | function | 88 |
| `0x08945fc8` | `Mem_LogKernelFree` | function | 88 |
| `0x089471a0` | `Mem_DumpAllHeaps` | function | 85 |
| `0x0894602c` | `Heap_Create` | function | 92 |
| `0x089461f0` | `Heap_Destroy` | function | 88 |
| `0x0894637c` | `Heap_AllocBlock` | function | 92 |
| `0x08946668` | `Heap_AllocBlockFromTop` | function | 90 |
| `0x0894669c` | `Heap_FreeBlock` | function | 92 |
| `0x08946b9c` | `Heap_FindBlockHeader` | function | 88 |
| `0x08946860` | `Heap_DumpBlocks` | function | 85 |
| `0x08946310` | `Heap_LastFreeBlockSize` | function | 82 |
| `0x0894633c` | `Heap_FragmentedFreeBytes` | function | 82 |
| `0x089462e0` | `Heap_UsedBytes` | function | 85 |
| `0x08947224` | `Heap_Lock` | function | 92 |
| `0x0894724c` | `Heap_Unlock` | function | 92 |
| `0x089471f4` | `Heap_CreateLockSema` | function | 88 |
| `0x08947270` | `Heap_DeleteLockSema` | function | 88 |
| `0x08945bfc` | `SmallHeap_Alloc` | function | 92 |
| `0x08945d7c` | `SmallHeap_Free` | function | 92 |
| `0x0888b6d0` | `Game_LoadFEData` | function | 85 |
| `0x08813ecc` | `Game_LoadBEData` | function | 85 |
| `0x0893e0dc` | `Vfs_FindDevice` | function | 88 |
| `0x0893de94` | `Vfs_Open` | function | 85 |
| `0x08972454` | `printf` | function | 90 |
| `0x08ac1f0c` | `g_heap_list` | data | 92 |
| `0x08ac1f10` | `g_main_heap` | data | 90 |
| `0x08ac1f14` | `g_data_heap` | data | 88 |
| `0x08ac1f44` | `g_heap_sema` | data | 90 |
| `0x08b84150` | `g_main_heap_storage` | data | 90 |
| `0x08b84168` | `g_small_heap_sema` | data | 92 |
| `0x08b6c660` | `g_small16_bitmap` | data | 92 |
| `0x08b6c950` | `g_small16_pool` | data | 92 |
| `0x08b67610` | `g_small32_bitmap` | data | 92 |
| `0x08b67660` | `g_small32_pool` | data | 92 |
| `0x08b04880` | `g_heap_dump_buffer` | data | 80 |

`Wad_MountArchive` (`0x08941e70`) and `Vfs_SplitDevicePath` (`0x0893e308`)
are on [wad-subsystem.md](wad-subsystem.md) and keep their rows there.

## The front door: `Mem_Alloc` (`0x08946e40`)

Confidence **95**. Over a hundred direct callers (Ghidra's caller listing
truncates at 100) plus every `operator new` in the binary through the
untagged thunk below.

```c
void *Mem_Alloc(uint size, const char *file, int line) {
    void *p = SmallHeap_Alloc(size);          // size <= 32 only, else 0
    if (p) return p;
    for (Heap *h = g_heap_list; h; h = h->next) {
        p = Heap_AllocBlock(h, size, file, line);
        if (p) return p;
    }
    printf("OUT OF MEMORY - need %d bytes\n", size);
    Mem_LogStatus("OutOfMemory", 1);
    return 0;
}
```

The `file`/`line` pair is a debug tag the retail build still passes -
`Wad_MountArchive` passes the literal `"file"` and lines `0x62`/`0x65`/`0x6f`,
`ParticleManager_Construct` (`FUN_088f2f74`) passes
`"ParticleManager.cpp"` and `0x4b` - but **`Heap_AllocBlock` never stores
it**: the block header has no room for a tag and the arguments are dead on
arrival. Whatever recorded them was compiled out.

`Mem_AllocUntagged` (`0x08946e20`) is `Mem_Alloc(size, 0, 0)`; two
byte-identical thunks to it at `0x08946ce4` and `0x08946d1c` are, by their
pairing with the `Mem_Free` thunk at `0x08946d00`, the C++ `operator new` /
`operator new[]` / `operator delete` triple. They are not named - the split
between the two `new`s cannot be told from the code.

`Mem_AllocFromTop` (`0x08946f0c`) is the same walk with
`Heap_AllocBlockFromTop`. `Mem_AllocAligned` (`0x08947044`) and
`Mem_AllocAlignedFromTop` (`0x08946fd8`) over-allocate by `align`, round the
returned pointer up, and zero the pad - which is why `Heap_FindBlockHeader`
has to *search* backwards for a header rather than subtract 16.

`Mem_Free` (`0x089470b0`) is the mirror: `SmallHeap_Free` first (it returns
1 if the address fell in either small pool), else the heap whose
`[base, base + size]` range contains the pointer gets `Heap_FreeBlock`. A
pointer inside no heap is silently ignored.

## The small-block pools: `SmallHeap_Alloc` (`0x08945bfc`)

Confidence **92**. Two static bitmap pools in `.bss`, one semaphore
(`"Small HeapWO"`, `g_small_heap_sema`, created by `Mem_Init`), no headers:

| Pool | Slot | Slots | Bitmap | Storage | Bytes |
| --- | ---: | ---: | --- | --- | ---: |
| 16-byte | 16 | 6,016 (`0xbc` words x 32) | `0x08b6c660` | `0x08b6c950` - `0x08b84150` | 96,256 |
| 32-byte | 32 | 640 (`0x14` words x 32) | `0x08b67610` | `0x08b67660` - `0x08b6c660` | 20,480 |

`size > 32` returns 0 at once, which is what sends the request on to the
heaps. `size <= 16` scans the 16-byte bitmap first; a miss there (or a
17-32 byte request) scans the 32-byte one; both full returns 0 and the
request goes to the heaps too, so the pools are an optimisation, never a
limit. The slot address is `pool + word * (32 * slot) + bit * slot`, and
`SmallHeap_Free` inverts that arithmetic from the address alone - the two
range checks in it are the pool bounds above, and the 16-byte pool's upper
bound is `g_main_heap_storage`, the next thing in `.bss`. Nothing records a
slot's size, which is why the two pools have to be contiguous ranges.

## A heap: `Heap_Create` (`0x0894602c`)

Confidence **92**. `Heap_Create(heap, size, register, base, at_front)`
fills a 24-byte descriptor:

| Offset | Field | Set to |
| --- | --- | --- |
| `+0x00` | `next` | 0; then linked per `register`/`at_front` |
| `+0x04` | `base` | the FPL block, or the caller's `base` |
| `+0x08` | `free_head` | `base` - one free block spanning the heap |
| `+0x0c` | `size` | |
| `+0x10` | `owns_fpl` | 1 when the FPL was created here |
| `+0x11` | `alloc_from_top` | 0; toggled around one call by `Heap_AllocBlockFromTop` |
| `+0x14` | `fpl_uid` | `sceKernelCreateFpl("HeapWO: %d", partition 2, attr 0, size, 1 block)` |

When `base` is 0 the heap's memory is **one fixed-length-pool block** from
user partition 2, named after its size, allocated with
`sceKernelTryAllocateFpl` immediately. `register` links it into
`g_heap_list` - at the head if `at_front`, else appended - and the first
heap ever created also creates the `"Memory Sema"` (`g_heap_sema`) that
`Heap_Lock`/`Heap_Unlock` take around every list operation. Creation is
bracketed by `Mem_LogKernelFree("Before Heap Creation")` / `"After Heap
Creation"`, which print `sceKernelTotalFreeMemSize`/`MaxFreeMemSize` - the
retail binary still has the `printf`, so a PPSSPP log shows them.

**The magic size `0xc20650`.** A request for exactly `0xc20650`
(12,715,600) bytes means "everything": the size is replaced by
`sceKernelMaxFreeMemSize() - 0x7e49b0` and the heap is recorded as
`g_main_heap`. `Mem_Init` (`0x08946c58`) is the only caller that passes it,
building the main heap in the static descriptor `g_main_heap_storage`
(`0x08b84150`). `Mem_Init` has **no caller**: its address sits in `.cplinit`
(`0x08ad9798`-`0x08ad9e8f`, 223 entries of 8 bytes, entry `0x08ad9d30`),
the SDK's C++ static-initialiser section, so it runs before `main` and the
heap already exists when `Game_Bootstrap` is reached - see the
[PSP memory map](../../memory-maps/psp-pulse-usa.md).

`Heap_Destroy` (`0x089461f0`) unlinks, frees and deletes the FPL if owned,
and deletes the lock semaphore when the list empties; bit 0 of its flags
argument also frees the descriptor itself. It, too, has no direct caller
found - consistent with heaps living for the whole process.

### The block format: `Heap_AllocBlock` (`0x0894637c`)

Confidence **92**. Every block, free or used, starts with a 16-byte header;
the pointer handed out is `header + 16`:

| Word | Free block | Used block |
| --- | --- | --- |
| `+0x00` | `size << 1 \| 0` | `size << 1 \| 1` (size includes the header) |
| `+0x04` | next free (address order) | `0xfeadfead` |
| `+0x08` | prev free | `0xab12ab34` |
| `+0x0c` | (unused) | `0x43658721` |

Requests round up to 16 and add the header; the free list is kept in
**address order**. In the default mode the walk is **first fit from
`free_head`** and the block is split from its *front*: the remainder becomes
a new free block at `block + needed` inheriting the list links. With
`alloc_from_top` set (`Heap_AllocBlockFromTop`, `0x08946668`, which sets the
byte, calls this, and clears it) the walk visits the *whole* list, keeps the
**last** fitting block - the highest address - and carves the request off
its *end*, so the free block shrinks in place and the allocation lands as
high as possible. Either way a block that would leave less than 32 bytes
(`needed + 0x30` test) is taken whole rather than split.

`Heap_FreeBlock` (`0x0894669c`) locates the header with
`Heap_FindBlockHeader` (`0x08946b9c`), which checks the three guard words at
`ptr - 12/-8/-4` and, failing that, steps back 16 bytes at a time for up to
17 tries - enough to reach the header from a 256-aligned pointer. **If no
header is found it dumps the heap and spins for ever** (`Heap_DumpBlocks`
then an empty infinite loop): a bad free is a deliberate hang, not a
return. Otherwise the block is reinserted in address order and coalesced
with its predecessor and successor when either is adjacent.

### Accounting

`Mem_LogStatus(name, dump)` (`0x08945ef4`) prints, per heap,
`"Memory %s : Free Bytes %d(%d) : Allocated Bytes %d"` from three walks over
the free list: `Heap_LastFreeBlockSize` (the last free block in address
order - normally the untouched tail, so effectively "largest contiguous"),
`Heap_FragmentedFreeBytes` (the sum of every free block *except* that last
one) and `Heap_UsedBytes` (`size` minus all free). The `dump` flag calls
`0x08946858`, which is an empty `jr ra` - the retail build stripped whatever
it did. `Heap_DumpBlocks` (`0x08946860`) is the surviving debugger: it walks
every block writing `"A  0x%08X size % 9d"` / `"D  0x%08X size % 9d"` lines
and a `"%d Ptrs %d Alloc %d Frag %d Free"` total to
`host0:memory%s_%X.txt` (`%s` is `DAT_08ab07f0`; the `%s%d_%X` variant adds
`FUN_0895ebf0()` when the mode index `DAT_08b31048` is `0xe` or above - a
network game), through `Vfs_Open` with flags `0x604`, i.e. only reachable on
a devkit with a host file system. `Mem_DumpAllHeaps` (`0x089471a0`) runs it over the list.

## The layout at runtime

| Region | Where | Size | Who |
| --- | --- | --- | ---: |
| `.bss` small pools | `0x08b67610`-`0x08b84150` | 116 KiB + bitmaps | `Mem_Init` (static) |
| Main heap | one FPL block in partition 2 | `sceKernelMaxFreeMemSize() - 0x7e49b0` | `Mem_Init`, static constructor |
| Data heap | one FPL block in partition 2 | `0x6d19b0` = 7,150,000 | `Game_LoadFEData` / `Game_LoadBEData`, first call |
| Left to the kernel | | `0x7e49b0 - 0x6d19b0` = `0x113000` = 1,126,400 | thread stacks, the audio and video FPLs, `sceUtility` |

The two constants close: the main heap leaves exactly 8,276,400 bytes of
the user partition free, the data heap takes 7,150,000 of those, and 1.07
MiB remains for everything the kernel allocates later. On a PSP-1000's 24
MiB user partition, with the ELF itself loaded, `sceKernelMaxFreeMemSize()`
at that point is on the order of 20 MiB, which puts the main heap at roughly
12 MiB - and `0xc20650` is 12,715,600, so the sentinel looks like a
measured value from an earlier build that later became the "use everything"
flag. Confidence 70 on that last reading; it is an inference from the
number, not from code.

### Steering an archive into the data heap

`Game_LoadFEData` (`0x0888b6d0`) and `Game_LoadBEData` (`0x08813ecc`) are
the only callers of `Heap_Create` besides `Mem_Init`, and both share one
descriptor, `g_data_heap`: whichever runs first allocates the 24-byte
descriptor and creates the 7,150,000-byte heap, **appended** to
`g_heap_list` (`register = 1`, `at_front = 0`), and the other reuses it.
Then, if `Vfs_FindDevice("fedata:")` finds nothing mounted yet and the
"WADs are resident" byte `DAT_08ac1e44` is set:

```c
Heap *saved = g_heap_list;
g_heap_list = g_data_heap;                  // list head = data heap, whose next is 0
Wad_MountArchive(new Wad, "fedata:", "umd:FEData.wad", flags 9, 0);
g_heap_list = saved;
```

Every allocation the mount makes - the device object, the name copy, and
the whole resident image - lands in the data heap because for that window
it is the *only* heap in the list. Afterwards the head is restored and the
data heap is back at the tail, where `Mem_Alloc` reaches it only when the
main heap is full. `Game_LoadBEData` mounts `bedata:` the same way but
**without** the head swap, so `BEData.wad`'s allocations go wherever
`Mem_Alloc` puts them - into the data heap only if the main heap is
exhausted. The front-end archive is placed deliberately; the back-end one
is not.

Flag `9` on the FEData mount is `0x01 | 0x08`: resident (the whole file is
read into one buffer) and **from the top** - `Wad_MountArchive` picks
`Mem_AllocAlignedFromTop(size, 256)` over `Mem_AllocAligned` when bit 3 is
set. This resolves the "`0x08` selects a different allocator" note on
[wad-subsystem.md](wad-subsystem.md): it is the same allocator in
high-address mode, so the 7 MB image sits at the top of the data heap and
the small objects the front end creates fill upward from the bottom
without fragmenting around it.

## `Vfs_FindDevice` (`0x0893e0dc`) and `Vfs_Open` (`0x0893de94`)

Both named here because the mount paths above go through them; the
resource-loading page that will own the VFS proper is still to be written.

- `Vfs_FindDevice(name)` walks the device list from `DAT_08ac1e38` by
  `strcasecmp` on the device name at `+0x00`, next at `+0x24`. Confidence 88.
- `Vfs_Open(path, flags, mode)` splits the device prefix with
  `Vfs_SplitDevicePath`, calls the device's `open` (vtable `+0x1c`), and for
  a successful open without flag `0x200` asks the device for the size
  (vtable `+0x3c` with the engine's own whence code `1`, then `0` to rewind;
  `Wad_MountArchive` uses the same three codes as tell/end/set) and rejects
  an empty file, closing it (vtable `+0x24`). A failed or empty open on a
  path *with* an explicit device prefix returns 0; a prefix-less path moves
  on to the next device in the list and tries again. The file object is
  0x110 bytes, filled by `FUN_0893e3e0`. Confidence 85.

## Cross-platform

| Platform | Notes |
| --- | --- |
| PS2 (Pulse) | Not located on this pass. The heap walker's guard words `0xfeadfead` / `0xab12ab34` / `0x43658721` are the search key. |
| PSP (Pure) | Not located on this pass; same search key. |

## Open questions

- Which of `0x08946ce4` / `0x08946d1c` is `operator new` and which
  `operator new[]`. Unanswerable from the code alone; unimportant.
- What `FUN_08971afc(&DAT_08ac1f1c)` / `(&DAT_08ac1f28)` in `Mem_Init`
  initialise. Two more lock-shaped objects, not read.

## History

- 2026-09-16: first reading, static. Corroborated internally by the fact
  that three independent constants (the small-pool bounds, the main-heap
  reserve and the data-heap size) close against each other; no runtime leg,
  which caps the page at 92.
