# The track geometry and collision loaders

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **The names here are applied**, from [names.tsv](names.tsv).
Found while chasing why `oag_render::mesh::rcs::build_scene` and
`oag_formats::collision::parse_chunks` do not read this title's
`track.rcsmodel` and `track_col.col` unmodified.

## `RcsModel_Load` - `0x812f15b2`

**Confidence: 94**

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
the file's own length to the byte (`189824 + 682966 + 16573144 = 17445934`).

**Section A's interior is now read**, and the two size words above turn out to
be fields of a *table* rather than header slots of their own. Confidence
**94**, corroborated across **all 993 `.rcsmodel` files** the three EU packages
ship rather than the one this paragraph used to rest on:

```text
+0x00  u32   magic, 0xca5caded
+0x08  u32   section count - 2 with geometry, 1 without
+0x0c  u32   section A's own size
+0x20        one 0x20-byte descriptor per section:
               +0x00  u32  a tag, unread
               +0x04  u32  the section's size
               +0x08  u32  its relocation table, from the end of the descriptors
               +0x0c  u32  how many entries that table has
               +0x10  u32  the base its pointers were linked against
      then   each section's relocation table, 8 bytes per entry:
               +0x00  u32  offset of a pointer, in the section named next
               +0x04  u32  which section that pointer lives in
```

So `+0x24`/`+0x44` are descriptor 0's and descriptor 1's `+0x04`, and
`+0x28`/`+0x2c` and `+0x48`/`+0x4c` - which the relocation loops read - are
those descriptors' table offsets and counts. The loop's own address arithmetic
is what fixes the layout: it starts at `A + A[0x28] + A[0x08] * 0x20` and reads
its first entry `0x20` further on, which is `A + 0x20` (the descriptor base)
plus `count * 0x20` (the descriptors) plus the table offset. On `altima` that
lands the first table at `A+0x60` with 18,077 entries, the second immediately
after it at `A+0x23548` with 5,634, and 40 bytes of padding to A's stated end.

**The link base at each descriptor's `+0x10` is `0` on every shipped file**,
which is what makes the payload readable at all: a pointer on disc is already
the offset of its target within its section, so nothing downstream simulates
the rebase. Section B is a serialized object graph and section C is raw GPU
buffer data - on `altima` all 23,711 pointer locations are in B and none in C.

What is inside **section B** - its object layout - is still unread; see
[`2048-rcsmodel.md`](../../../formats/2048-rcsmodel.md) for what
`oag_formats::rcsmodel::psp2` does instead, and for the vertex fields past the
position that remain unplaced.

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

## `SimpleMesh_Load` - `0x8118fac8`

**Confidence: 92**

Reads the triangle soup the k-d tree indexes - the section `KdTree_Load` hands
off to, and the last one in the file. Called with the collision object and the
open stream, and reads, in order:

```text
        u16      vertex count V
        f32[3V]  positions, in chunks of at most 8,192 vertices
        u32      bytes per triangle
        u16      triangle count T
        …        T * that many bytes of triangle indices
        u16      surface byte count
        u8[…]    the surface bytes
        f32[6]   the soup's bounds: centre, then half-extent
```

Evidence:

- Every allocation it makes is tagged
  `"Backend/General/Collision/SimpleMesh.cpp"`, at four line numbers
  (`0x39`/57 for the vertex array, `0x3d`/61 for the staging buffer, `0x68`/104
  for the indices, `0x75`/117 for the surface bytes) - a source path naming the
  class, the same strength of evidence `RcsModel_Load` rests on.
- **The staging buffer's size is the arithmetic that fixes the vertex stride.**
  It allocates `0x18000` bytes once and then reads at most `0x2000` vertices
  into it per pass at `count * 0xc` bytes: `8192 * 12 = 98304 = 0x18000`
  exactly. So a vertex is 12 bytes on disc and 16 in memory, which is why the
  loop that follows widens each one.
- **The index buffer is allocated at `6 * T` and read at `stride * T`**, so any
  stride but 6 is a heap overflow in the real game rather than a layout the
  format permits. `oag_formats::kdcol` refuses one for that reason.
- **The surface array is allocated at exactly `T` bytes**, one per triangle -
  and `SimpleMesh_Construct` (`0x8118f9bc`) allocates the same three arrays at
  `V << 4`, `T * 6` and `T` from its own two count arguments, which is the same
  statement made twice.
- A sibling string, `"Built track collision mesh (SimpleMesh): %d verts, %d
  triangles, %d bytes\n"` (`FUN_811901fa`), prints the object's `+0x04` and
  `+0x0c` as the vertex and triangle counts - independently confirming which
  field each `u16` lands in.
- Checked against all 26 shipped `track_col.col` files, not one: every one of
  them ends **exactly** on its final `"----"` under this layout, with the
  surface-byte count equal to the triangle count on every file. See
  `crates/formats/tests/kdcol_ground_truth.rs`.

## `SimpleMesh_Construct` - `0x8118f9bc`

**Confidence: 88**

Constructs an empty `SimpleMesh` for a stated vertex and triangle count,
allocating the three arrays `SimpleMesh_Load` fills. Named off the same
allocator tag at lines `0x1b`/27, `0x1f`/31 and `0x20`/32, and off the array
sizes: `vertices << 4`, `triangles * 6`, `triangles * 1`. It is the constructor
`KdTree_Load` calls through before reading anything.

## `TrackCollision_MeshFromNode` - `0x8126f800`

**Confidence: 88**

**The function that names the surface bytes.** Walks a `.vex` node's collision
payload - the same chunk format `oag_formats::collision::parse_chunks` reads,
recognisable by its three chunk types (`1` vertices at 12 bytes, `2` triangles
at 6, `3` scalars at 4) - and hands the geometry to the mesh builder with one
byte chosen from the node's own class ID:

| `.vex` class ID | this project's constant | byte |
| --- | --- | ---: |
| `0x3b9` | `CLASS_FLOOR_COLLISION` | 2 |
| `0x3e6` | `CLASS_MAG_FLOOR_COLLISION` | 3 |
| `0x3ba` | `CLASS_WALL_COLLISION` | 4 |
| `0x3f2` | *`Force Field Collision`*, 2048's own | 5 |
| `0x3ed` | `CLASS_TRACK_WALL_COLLISION` | 6 |
| `0x3cd` | `CLASS_RESET_COLLISION` | 7 |
| anything else | | 15 |

Evidence:

- **The class IDs are this project's own, unchanged.** Six of the seven
  constants in `crates/formats/src/vex/classes.rs` appear as literals in one
  `if`/`else if` chain, which is not a coincidence a wrong reading produces.
- **`0x3e7` is branched past before any byte is chosen** - the decompiler
  renders the guard as `if (iVar7 != 999)`, and `999 == 0x3e7 ==
  CLASS_CAGE_COLLISION`. That independently reproduces the behaviour
  `oag_formats::collision::SurfaceKind::Cage` already documents from Pulse's
  own loader: cage geometry is parsed and then skipped.
- **`0x3f2` is a class only 2048 declares**, and its name comes from that
  title's own class-name table: the record at `0x81521f8c` pairs `0x3f2` with
  the string at `0x814ccc84`, `"Force Field Collision"`. The record before it
  pairs `0x3ed` with `"Track Wall Collision"`, which is how the table's stride
  and shape were confirmed. The seven `"… Collision"` strings in the whole
  binary are exactly the seven classes above.
- **A completely independent measurement reproduces the same six rows.** 2048's
  DLC re-ships twelve Wipeout HD circuits; HD keeps its collision in named
  `.vex` classes; matching the two titles' collision triangles by position gives
  **170,744 pairs and no disagreement** on any of the six. See
  [`2048-collision.md`](../../../formats/2048-collision.md).
- Address-identical and instruction-identical in
  `/vita-2048-usa-v104/eboot.elf` (`8126f800` in both), the same cross-check
  the two loaders above use.

`FUN_81190106`, which it calls, takes that byte as its last argument and writes
it once per triangle - so the byte is chosen per *mesh*, from the class, and
never per triangle.

## History

- 2026-08-26 (third pass): `RcsModel_Load` raised 85 -> 94. Section A's own
  interior is read - the header, the section descriptors and both relocation
  tables - and the reading is corroborated over all 993 shipped `.rcsmodel`
  files rather than the one `altima` the first pass checked. The container's
  payload is decoded far enough to draw: see
  [`2048-rcsmodel.md`](../../../formats/2048-rcsmodel.md).
- 2026-08-26 (second pass): `SimpleMesh_Load` 92, `SimpleMesh_Construct` 88 and
  `TrackCollision_MeshFromNode` 88 named, which finishes `track_col.col` -
  `oag_formats::kdcol` reads all 26 shipped files with every byte accounted for.
  The bounds correction above landed in the same pass. `RcsModel_Load`'s three
  sections are still unread inside.
- 2026-08-26: 85 / 87, first RE pass on either function. Decompilation,
  disassembly at the two points where it changed the reading (`RcsModel_Load`'s
  section-A-size source, `KdTree_Load`'s version-vs-return-value branch),
  address-identical cross-check against `/vita-2048-usa-v104/eboot.elf`, and a
  direct read of the real container each function loads against the offsets
  its own control flow predicts. No runtime trace yet, and neither container's
  payload (section A/B/C's interior for `RcsModel_Load`, the node's trailing
  16 bytes and the whole mesh-shape trailer for `KdTree_Load`) is decoded -
  only where the pieces start and end.
