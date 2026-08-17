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

**80** rests on the syscall sequence, which is unambiguous: `0x144`
(`sys_memory_container_create`) on one branch, then `0x15c`
(`sys_memory_allocate`) and `0x15d` (`sys_memory_free`) around a
`malloc(0x1c8)` whose result becomes the heap pointer both allocate entry points
test for null.

Its report, read through the function's own TOC (see below - Ghidra's rendering
of it is wrong), corroborates that and **names the three pools**:
`"*******PS3 Heap Initial Sizes*********************"`, then
`"Small is  %3.2f MB (%d bytes)"` for the pool at `heap + 0x88`,
`"String is %3.2f MB (%d bytes)"` for `heap + 0x128`, and `"Large is …"` for
`heap + 0`. So the sub-`0x400` fast path in the allocate entry points is the
**Small** pool by the game's own name for it, and the main heap is **Large**.
The score stays at 80 rather than rising, because this is one binary read one
way with no runtime trace and no second binary - the rubric's ceiling for that
is 84 and nothing here has earned the top of it.

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

## Every function has its own TOC, and Ghidra uses one for all of them

**This is the single most important thing to know before reading data
references in this binary, and it took two wrong write-ups on this page to
pin down.** It was first recorded as a `r2` problem, then withdrawn on the
grounds that Ghidra's arithmetic checked out, then confirmed properly. The
arithmetic did check out - against the base Ghidra had chosen. That proved
self-consistency, not correctness, and withdrawing on it was the wrong call.

The evidence is the OPD. `FwMemHeap_Create` is at `0x003914b0`; searching the
image for that word finds exactly one hit, `0x0088acb8`, its OPD entry:

```
0088acb8:  003914b0 008bd3c4      {func, toc}
```

Its TOC is **`0x008bd3c4`**. Ghidra uses `0x008ad4d8` - the TOC from the ELF
entry point's OPD entry - for every function in the program. The two differ by
`0xfeec`, so every TOC-relative load in this function lands `0xfeec` away from
where it should.

What that does to one instruction, `lwz r3,-0x6634(r2)` at `0x00391594`:

| TOC used | Slot | Points at | Reads |
| --- | --- | --- | --- |
| Ghidra's `0x008ad4d8` | `0x008a6ea4` | `0x0077db68` | `"forward"` |
| The function's `0x008bd3c4` | `0x008b6d90` | `0x007afe68` | `"Small is  %3.2f MB (%d bytes)\n"` |

Both are real strings at real addresses. Only one is the one this code loads.
With the correct TOC the whole function resolves coherently - the header string
is `"*******PS3 Heap Initial Sizes*********************"`, and the float at
`-0x6628(r2)` is `0x35800000`, which is exactly `2^-20`, the bytes-to-MB
conversion its `%3.2f` needs. Under Ghidra's TOC that slot reads `1.0f` and the
format strings read as race telemetry.

### There are exactly two TOCs, and they overlap in address space

Walking the whole OPD settles the scale of it. The section runs
`0x00870520`-`0x008a54d8`, **27,127 entries**, and holds exactly **two** distinct
TOC values:

| TOC | Functions | Code range it covers |
| --- | --- | --- |
| `0x008ad4d8` | 11,037 | `0x010200`-`0x758110` |
| `0x008bd3c4` | 16,090 | `0x32d5e0`-`0x7579c0` |

Two statically linked modules, then. Ghidra uses the first for everything, so
**59% of this program's functions have every TOC-relative load resolved against
the wrong base.**

The ranges **overlap** between `0x32d5e0` and `0x758110`, which is the part that
matters in practice: a function's address does not tell you its TOC. Only the
two tails are safe to assume -

- below `0x32d5e0`: always `0x008ad4d8`, so Ghidra is right and its string
  references can be read directly. Everything in `Collision.cpp` and
  `RaceManager.cpp` falls here.
- above `0x758110`: always `0x008bd3c4`, so Ghidra is always wrong.
- between the two: read the OPD entry. There is no shortcut.

The memory layer documented on this page sits at `0x0039xxxx`, inside the
overlap, which is why it was the place the problem surfaced.

**So: do not trust a Ghidra string cross-reference in this program** unless the
function is below `0x32d5e0`. Elsewhere it yields both false positives and false
negatives, and the failure mode is a plausible wrong string rather than a visible
error. To check one by hand, three reads and no guessing:

1. `disassemble_function` the caller and find the real `lwz rX, disp(r2)`.
2. `search_byte_patterns` for the function's own entry address; the single hit
   is its OPD entry. Read 8 bytes: `{func, toc}`.
3. Read `toc + disp` for the pointer, then read the pointer for the bytes.

**The fix is `AssignPs3R2FromOpd.java`**, which walks the OPD and gives each
function the `r2` its own entry declares. It is in the script pack, upstream's
README does not mention it, and it was missing from
`scripts/import-ps3-eboot.sh` when this database was built. It is there now, as
a post-script, and this database predates it.

A closed lead, recorded so nobody re-opens it: `0x00455018` really is `printf` -
it has the varargs save area at `r1+0xd8`, builds a `va_list`, locks a stream,
calls a `vfprintf` worker and unlocks. The libc naming was never the problem.

None of the seven names above depends on any of this. Each rests on control
flow, argument counts, or calls to imports the NID database named, which is what
made them safe to land from a database with this defect in it.

## Not recorded

- `0x00676a78`, the TOC stub in front of `FwMemAllocator_AllocateAligned`. It is
  generated glue; naming it adds noise, and it will reappear for every
  cross-section call in the binary.
- `0x0038ddd0`, `0x0038d928`, `0x0038e258` - the three heap-block allocators
  under the entry points. Their split is clearly meaningful and no reading of it
  is better than a guess yet, which is below the rename threshold.
