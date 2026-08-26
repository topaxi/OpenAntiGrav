# The track geometry and collision loaders

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **The names here are applied**, from [names.tsv](names.tsv).
Found while chasing why `oag_render::mesh::rcs::build_scene` and
`oag_formats::collision::parse_chunks` do not read this title's
`track.rcsmodel` and `track_col.col` unmodified.

## `RcsModel_Load` - `0x812f15b2`

**Confidence: 85**

Loads one `.rcsmodel` file: resolves the path, reads a `0x60`-byte prefix,
allocates a "main memory" block and a separate GPU-resident block sized from
fields in that prefix, reads each in turn, then walks the loaded object's own
relocation table twice (once for pointers into the main-memory block, once for
pointers into the GPU block) before returning.

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
`cargo run -p oag-game --example vita_probe`:

```text
+0x0c  u32   section A size (bytes) - 189,824 on this file
+0x24  u32   section B size ("main memory"/CPU-resident) - 682,966 on this file
+0x44  u32   section C size ("GPU"/VRAM-resident) - 16,573,144 on this file
```

`size(+0x0c) + size(+0x24) + size(+0x44)` equals the file's own length to the
byte (`189824 + 682966 + 16573144 = 17445934`) - an exact arithmetic closure on
the one file checked, the same kind of evidence the `WO Track` point-stride
finding rests on. **Not yet corroborated on a second file or a second track**,
and the exact byte layout of section A's own header (what `+0x00`'s
`0xca5cad ed`-shaped value and the other unlabelled words in the first `0x60`
bytes are) is unread - this only establishes where the three sections start
and end, not what is inside sections A and C, or the relocation-table shape
this function applies to each after loading. That is real, remaining work.

## `KdTree_Load` - `0x8118d134`

**Confidence: 87**

Loads one k-d tree collision structure: validates a magic-plus-version tag by
`sscanf`-ing it against the literal format string `"kdtr%04x"`, then reads a
sequence of `"----"`-tagged sections (node array, leaf index array, bounding
box), fixing up each k-d node's two child fields from serialized indices into
real pointers before handing off to a second function
(`FUN_8118fac8`, unnamed) for the mesh data the tree indexes.

Evidence:

- Its own format string is unambiguous: `SceLibc_EC585241(&buffer,
  "kdtr%04x", &version)`, immediately preceded by reading exactly the bytes
  the format string describes (an 8-byte ASCII prefix) off the stream.
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
+0x04  char[4]  ASCII decimal-free hex version, "0001" on every file seen
+0x08  char[4]  section tag, literal "----", repeated before every section below
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

- 2026-08-26: 85 / 87, first RE pass on either function. Decompilation plus
  address-identical cross-check against `/vita-2048-usa-v104/eboot.elf`, plus
  a direct read of the real container each function loads against the
  offsets its own control flow predicts. No runtime trace yet, and neither
  container's payload (section A/C's interior for `RcsModel_Load`, the node's
  trailing 16 bytes and the whole mesh-shape trailer for `KdTree_Load`) is
  decoded - only where the pieces start and end.
