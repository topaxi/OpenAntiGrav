# Memory: the allocator, its heap, and the mutex around both

The first sweep of `EBOOT.elf`, 2026-08-17. Seven names, all in the framework's
memory layer. Nothing here is Wipeout-specific - which is the point of starting
here: it is the part of a stripped binary whose shape is least ambiguous, and it
establishes the `Fw` prefix the rest of the framework uses.

## Why the allocator was findable at all

`EBOOT.elf` is stripped of its symbol table, but not of **462 `.cpp` filename
strings** - `GameRoot.cpp`, `Collision.cpp`, `RaceManager.cpp`,
`SPArcade_ModeManager.cpp` and so on. They survive because the allocation macro
passes `__FILE__` and `__LINE__` at every call site, which makes every tagged
allocation in the game point back at the allocator.

The clearest example, at `0x00034350`, is a Collision-system init that makes six
allocations in a row from `Collision.cpp` lines 134-139:

```c
uVar3 = FUN_00676a78(0x400000, 0x80, PTR_s_Collision_cpp, 0x86);
uVar3 = FUN_00676a78(0x90,     0x80, puVar2,              0x87);
uVar3 = FUN_00676a78(0x10,     0x80, puVar2,              0x88);
uVar3 = FUN_00676a78(0x11b40,  0x80, puVar2,              0x89);
uVar3 = FUN_00676a78(0xb440,   0x80, puVar2,              0x8a);
uVar3 = FUN_00676a78(0x4000,   0x80, puVar2,              0x8b);
```

So the call shape is `(size, alignment, file, line)`. `0x00676a78` is only a TOC
stub - it loads `r2` and tail-calls the real entry point.

The **name** comes from a second string. The binary carries
`Live::FwMemAllocator::Allocate` at 22 addresses, and `0x006c0fa8` shows what
for: walking allocation records and substituting that string wherever a record's
own name field is null. It is the default tag of an untagged allocation, which
names the function the tag belongs to.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00392478` | `FwMemAllocator_Allocate` | 85 |
| `0x00392220` | `FwMemAllocator_AllocateAligned` | 85 |
| `0x00392790` | `FwMemAllocator_AllocateDefault` | 88 |
| `0x003914b0` | `FwMemHeap_Create` | 80 |
| `0x003a1e78` | `FwMemHeap_AddChunk` | 88 |
| `0x003931e0` | `FwMutex_Lock` | 90 |
| `0x003931b8` | `FwMutex_Unlock` | 90 |

### `FwMemAllocator_Allocate` / `FwMemAllocator_AllocateAligned`

The two entry points are the same function with and without an alignment
argument, and they run the same sequence:

1. Return null for a zero size.
2. Create the heap if the global at `PTR_PTR_008a6e68 + 4` is still null.
3. If a custom allocator is installed at `+8` **and** the calling thread is the
   one recorded at `PTR_PTR_008a6e70`, dispatch to its vtable and return on
   success. The aligned entry uses vtable slot `+0x14`, the plain one `+0x10` -
   adjacent slots differing by one trailing argument.
4. Take the heap mutex at `heap + 0x110`.
5. For sizes **below `0x400`**, try a small-block pool at `heap + 0x88` first.
6. Otherwise, or on failure, allocate from the main heap; on failure grow the
   heap and retry in a loop.
7. If growing fails, release the mutex, call the out-of-memory hook at
   `*PTR_PTR_008a6e68` with the requested size, and return null.

**Why 85 and not higher.** The control flow is unambiguous and the two entry
points corroborate each other. What is *not* proved is that the second argument
is an alignment: every call site seen so far passes `0x80`, so the value alone
does not discriminate between an alignment and a pool id. The reading rests on
the vtable pair - two adjacent slots where one takes an extra trailing argument
is the conventional `Allocate(size)` / `Allocate(size, align)` split. A call site
passing something other than `0x80` would settle it either way and none has been
looked for yet.

The plain entry takes a second `char` argument that the aligned one does not,
selecting between two heap-block allocators (`0x0038ddd0` and `0x0038d928`). Both
are reached identically otherwise. A bottom-versus-top split is the obvious
guess and is **not** recorded as a name, because nothing here proves it.

### `FwMemAllocator_AllocateDefault`

`FUN_00392790(size)` is exactly `FwMemAllocator_Allocate(size, 0)`. A one-line
wrapper, hence 88 - the only claim is that it is the default-flag entry point.

### `FwMemHeap_Create`

Called from both allocate entry points, guarded by the heap pointer being null,
so it runs at most once. It reserves a memory container sized from a value the
system reports (`sys_memory_container_create`), calls
`sys_memory_allocate(0x600000, 0x200)`, mallocs a `0x1c8`-byte heap descriptor,
initialises it, frees the temporary, and then prints a heap-usage table through
`printf`/`puts`.

**80** rests on the syscall sequence alone, which is unambiguous: `0x144`
(`sys_memory_container_create`) on one branch, then `0x15c`
(`sys_memory_allocate`) and `0x15d` (`sys_memory_free`) around a
`malloc(0x1c8)` whose result becomes the heap pointer both allocate entry points
test for null. It does not rest on the report it then prints - see the open
question below.

### `FwMemHeap_AddChunk`

`(heap, size)`, returns a boolean the allocator's retry loop tests. It `malloc`s
`size + 0x1f`, links the block into the chunk list at `heap[1]`, 16-byte-aligns
the usable region, and registers it through the heap's vtable at `+0x38`.

The failure path is what makes this unambiguous: when the first `malloc` fails it
probes **downward** in `0x1400` steps until one succeeds, backs off `0x4800`,
frees the probe and retries - giving up below `0x9001` and returning 0. That is a
largest-available-block search, and it only makes sense in a grow-the-heap
routine. Calls named libc `malloc`/`free` directly, so nothing here depends on
symbol guesses.

### `FwMutex_Lock` / `FwMutex_Unlock`

One-line wrappers over the named PS3 imports `sys_lwmutex_lock(mutex, 0)` and
`sys_lwmutex_unlock`. The behaviour is certain; the 90 is capped below the
established band only because these are the project's usual reasons - no runtime
trace, no second binary.

The `FwMutex` prefix is **inferred**, not recovered: `Fw` is the framework prefix
the binary itself uses in `Live::FwMemAllocator`, but nothing names the mutex
class. If a string ever gives it a real name, rename both.

## Open question: two TOC reads that make no sense

**Corrects an earlier reading on this page.** These were first written up as
Ghidra resolving `r2` wrongly. That diagnosis was wrong and is withdrawn: TOC
resolution here is **correct**, checked at eight displacements across two
functions against the TOC base `0x008ad4d8` that the ELF entry point's own OPD
entry declares. For example `lwz r3,-0x663c(r2)` at `0x003916bc` resolves to
`0x008a6e9c`, which is exactly the slot Ghidra names. What remains is not a tool
problem but an unexplained one.

Two sites read strangely, and the instruction stream really does say so:

- **`FwMemHeap_Create` passes string pointers to `printf` that read as
  telemetry labels.** `-0x6634(r2)` → slot `0x008a6ea4` → `0x0077db68` →
  `"forward"`, and `-0x6630(r2)` → `0x008a6ea8` → `0x0077db70` →
  `"EndRace Results"`. Read three ways - displacement arithmetic, TOC slot
  contents, raw string bytes - and all three agree. A heap-usage report does not
  print `"forward"`.
- **`FUN_006be928` stores zero through the pointer in slot `0x008a5ec8`**, and
  that slot holds `0x0077afe0`, which is where `"Collision.cpp"` lives.
  `lwz r9,-0x7610(r2)` then `stw r0,0x0(r9)`, with `r0` zero.

**The lead worth following**: `0x00455018` and `0x0043b438` are named `printf`
and `puts` by Ghidra's analysis, not by any symbol in this stripped binary. If
either guess is wrong, both oddities dissolve - a function that is not `printf`
has no format-string argument, and the "string" pointers are just pointers.
Check what those two thunks actually reach before spending time on the TOC.

None of the seven names above depends on this. Each rests on control flow,
argument counts, or calls to imports the NID database named, and that was the
deliberate stopping line for a first sweep rather than a consequence of the
oddity. The obvious next targets - `Collision.cpp`'s own functions and the
subsystem attribution behind all 462 filename strings - do lean on TOC string
reads, so settle the `printf` question first.

## Not recorded

- `0x00676a78`, the TOC stub in front of `FwMemAllocator_AllocateAligned`. It is
  generated glue; naming it adds noise, and it will reappear for every
  cross-section call in the binary.
- `0x0038ddd0`, `0x0038d928`, `0x0038e258` - the three heap-block allocators
  under the entry points. Their split is clearly meaningful and no reading of it
  is better than a guess yet, which is below the rename threshold.
