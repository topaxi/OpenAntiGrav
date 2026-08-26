# The track geometry and collision loaders

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **The names here are applied**, from [names.tsv](names.tsv).
Found while chasing why `oag_render::mesh::rcs::build_scene` and
`oag_formats::collision::parse_chunks` do not read this title's
`track.rcsmodel` and `track_col.col` unmodified.

## `RcsModel_Load` - `0x812f15b2`

**Confidence: 85**

Loads one `.rcsmodel` file: peeks the first 32 bytes to learn a size at
`+0x0c`, allocates and reads exactly that many bytes as one block (call it
section A), then reads two more fields *out of the just-loaded section A*
(`+0x24` and `+0x44`) and uses each as the size of a further block it
allocates and reads in turn - a "main memory" (CPU-resident) block and a
separate GPU-resident block. It then walks section A's own relocation table
twice, once retargeting pointers into the main-memory block and once into the
GPU block, before returning.

Evidence:

- Its own allocator call tags every allocation
  `"PSP2/Psp2.RcsModelLoader.cpp"` at two different line numbers (`0x49`/73
  for the CPU-side block) - a source path naming exactly the file class this
  loads, the same strength of evidence `GameRoot_Construct` rests on for its
  own tag string.
- Two of its own error strings name the resource type directly:
  `"Not enough main memory to load Live::Rcs::Model: %s.\nRequired main
  memory: %d bytes\n"` and `"Not enough Vram and MainUncache memory available
  to allocate gpu section of RcsModel. %s (requested size %d)\n"` - so the
  class is `Live::Rcs::Model`, and the function unambiguously reads one field
  as "how much main memory" and a different field as "how much GPU memory".
- Address-identical in `/vita-2048-usa-v104/eboot.elf` (`812f15b2` in both),
  the same cross-check `Game_Main`/`GameRoot_Construct` used.

**What this fixes on the container itself**, checked against
`data/art/published/environments/altima/track.rcsmodel` (17,445,934 bytes) via
`cargo run -p oag-game --example vita_probe`, and confirmed at the
instruction level rather than only decompiled (the allocation size at
`812f160a: ldr r1,[sp,#0x20]` reads the peeked buffer at its own `+0xc`, and
the two further sizes at `812f1658`/`812f1664` are loaded from the
*just-allocated* section A at `+0x24`/`+0x44`, not from the file directly):

```text
+0x0c  u32   section A's own total size (bytes) - 189,824 on this file;
             section A is read whole, this many bytes, into one allocation
+0x24  u32   section B size ("main memory"/CPU-resident), read out of the
             loaded section A - 682,966 on this file
+0x44  u32   section C size ("GPU"/VRAM-resident), read out of the loaded
             section A - 16,573,144 on this file
```

Sections are laid out back to back: A, then B (sized by A's own `+0x24`), then
C (sized by A's own `+0x44`). `size(+0x0c) + size(+0x24) + size(+0x44)` equals
the file's own length to the byte (`189824 + 682966 + 16573144 = 17445934`) -
an exact arithmetic closure on the one file checked, the same kind of evidence
the `WO Track` point-stride finding rests on. **Not yet corroborated on a
second file or a second track**, and section A's own interior past `+0x0c`,
`+0x24` and `+0x44` (what `+0x00`'s `0xca5caded`-shaped value and the other
unlabelled words are, and the chunk/relocation-table shape section A carries
for both loaded blocks) is unread - this only establishes where the three
sections start and end, not what is inside any of them. That is real,
remaining work.

## `KdTree_Load` - `0x8118d134`

**Confidence: 87**

Loads one k-d tree collision structure: validates a magic-plus-version tag by
`sscanf`-ing it against the literal format string `"kdtr%04x"` and requiring
the parsed version to equal `1`, then reads a sequence of `"----"`-tagged
sections (node array, leaf index array, bounding box), fixing up each k-d
node's two child fields from serialized indices into real pointers before
handing off to a second function (`FUN_8118fac8`, unnamed) for the mesh data
the tree indexes.

Evidence:

- Its own format string is unambiguous: `SceLibc_EC585241(&buffer,
  "kdtr%04x", &version)`, immediately preceded by reading exactly the bytes
  the format string describes (an 8-byte ASCII prefix) off the stream.
- **The `== 1` branch right after compares the parsed version, not the call's
  own return value**, confirmed at the instruction level rather than assumed:
  disassembly at `8118d1d6..8118d1ea` shows the stack slot passed as the
  third argument (`&[sp+8]`) is explicitly zeroed *before* the call
  (`str r6,[sp,#0x8]` with `r6` known zero) and reloaded from that same slot
  *after* it (`ldr r0,[sp,#0x8]`) for the `cmp r0,#0x1` - the call's actual
  return value in `r0` is discarded unread. Pre-zeroing an output slot and
  reading it back rather than trusting the return register is the shape of an
  out-parameter, not a field-count return; this genuinely is a version gate
  (`version == 1`), not merely "one field scanned" - the two readings looked
  indistinguishable on this corpus (every file seen is `"0001"`) until this
  instruction-level check settled it.
- The two allocator call sites are tagged `"Backend/General/Collision/
  KdTree.cpp"` at lines `0x8a` (138, the node array) and `0xb7` (183, the leaf
  index array) - a source path naming the exact subsystem, and a companion
  string (`"Backend/General/Collision/Shape/KdTreeMeshShape.cpp"`) names what
  the trailing, unread section belongs to.
- The child-index fix-up loop (`if (index != -1) { pointer = base +
  index * 6 } `, applied to both fields of every node, 6 `int`s = 24 bytes
  matching the node stride read from the file) is exactly the shape a
  serialized binary tree's child links take once index-based storage is
  converted to a live pointer graph - not a generic copy loop.
- Address-identical in `/vita-2048-usa-v104/eboot.elf` (`8118d134` in both).

**What this fixes on the container itself**, checked against
`data/art/published/environments/altima/track_col.col` (1,068,082 bytes):

```text
+0x00  char[4]  "kdtr"
+0x04  char[4]  ASCII 4-hex-digit version, required to read as 1 - "0001" on
                every file seen
+0x08  char[4]  the same "----" section-tag read used before every section
                below, its first occurrence
+0x0c  u32      node stride in bytes - 24 (0x18) on this file
+0x10  u32      node count N - 27,199 on this file
        then    N * 24-byte k-d tree nodes: two child-node *indices* (0x7fffffff-
                style null not yet confirmed; this file's node 0 has left=1,
                right=12226) as the first 8 bytes, 16 bytes unrecovered
                (split axis/value, bounds - a hypothesis, not read)
       "----"   section tag
+…     u32      leaf/triangle index count M - 93,279 on this file
        then    M * u16 leaf indices
       "----"   section tag
        then    2 * [f32;3], an axis-aligned bounding box - min
                (-113.5, 333.6, 350.5) then max (1047.7, 398.9, 1296.5) on this
                file
       "----"   section tag
        then    the mesh-shape trailer `KdTreeMeshShape.cpp` owns - 228,692
                bytes left over on this file, entirely unread
```

Every tag, both counts and the bounding box were read directly out of the real
file at the offsets this structure predicts and landed exactly where the
control flow says they should - not a guess fitted after the fact. **The
trailing ~229 KiB (the actual triangle geometry `KdTreeMeshShape.cpp` owns) is
completely unread**, and a k-d tree's own interior 16 bytes past the two child
indices are a hypothesis, not a measurement.

## History

- 2026-08-26: 85 / 87, first RE pass on either function. Decompilation,
  disassembly at the two points where it changed the reading (`RcsModel_Load`'s
  section-A-size source, `KdTree_Load`'s version-vs-return-value branch),
  address-identical cross-check against `/vita-2048-usa-v104/eboot.elf`, and a
  direct read of the real container each function loads against the offsets
  its own control flow predicts. No runtime trace yet, and neither container's
  payload (section A/B/C's interior for `RcsModel_Load`, the node's trailing
  16 bytes and the whole mesh-shape trailer for `KdTree_Load`) is decoded -
  only where the pieces start and end.
