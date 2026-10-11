# Unity asset bundles (`oag-unity`)

A generic reader for **Unity content a player already has**: UnityFS asset
bundles, the SerializedFiles inside them, their typetrees, and the built-in
classes a scene is made of. It is the first layer of Unity content support
([ADR-0062](../architecture/adr/0062-unity-content-is-a-generic-reader-with-a-content-layer-on-top.md)):
[`oag-unity`](../../crates/unity) holds Unity's own specifics and knows no game.
A game's classes (what a section, a tile or a ship's stat block mean) belong to a
layer on top, which does not exist yet.

The test content is BallisticNG's Steam Workshop items and install. That is the
source of the files, not a framing of the crate: nothing in `oag-unity` names a
game.

**Confidence labels:** *measured* = read out of the files and checked
(typically against UnityPy as a second implementation); *inferred* = consistent
with the data, no second source; *not exercised* = implemented from the
published layout but no file in the census uses it, so it is untested.

## Project-authored fixture

`crates/unity/tests/fixtures` holds a track this project built itself with Unity
6000.6.5f1 (a Catmull-Rom loop: three 258-vertex meshes, a 64x64 DXT1 texture
with 7 mips, a material, one custom `MonoBehaviour`), as a prefab bundle and a
two-file scene bundle in LZMA, LZ4HC and uncompressed form. It contains no game
content, so it is committed and `tests/fixture_track.rs` runs in plain `just
test` and CI. It is the only real **UnityFS version 8 / SerializedFile version
23** sample, and UnityPy agrees with it (vertex and triangle counts, |x| sums,
texture size, mips and byte length). Its `README.md` says how to regenerate it.

## Census

Counted with `read_dir` over each item directory of the Workshop tree (92 items,
one file each) and over the install directory; the numbers are asserted in
`crates/unity/tests/*_ground_truth.rs`, and `install_census.rs` for the install.

| What | Count | Notes |
| --- | --- | --- |
| Item files | 92 | 47 `.csf` (ships, a bare UnityFS bundle), 32 `.wtrk` (tracks, a wrapper with the bundle at an offset), 13 `.shp` (a legacy non-Unity ship format, **out of scope**) |
| UnityFS bundles | 79 | 47 ship + 32 track. Archive version **6** x78 and **7** x1; engine `2018.3.8f1` x78 and `2020.3.5f1` x1 |
| Block codec | LZMA x32 (tracks, one block), LZ4HC x47 (ships, 14 to 116 blocks) | uncompressed blocks are read too, but no census file uses them |
| Bundle nodes | 236 | 111 SerializedFiles (flag `4`), 47 `.resS` and 78 `.resource` raw streams (flag `0`) |
| SerializedFiles | **111** | version **17** x109, **22** x2 (the 2020.3 bundle); typetrees present in all 111 |
| Objects | **123,083** | every one reads through its typetree and consumes exactly its bytes |
| Install SerializedFiles | 27 | `level*`, `*.assets`, `globalgamemanagers`; all version 22, **no typetrees** (a shipped player strips them), 278,778 objects listed |

Class census of the 111 files: `MonoBehaviour` 88,703, `GameObject` 8,459,
`Transform` 8,306, `Texture2D` 2,302, `Mesh` 1,570, `Material` 805.

**The `.wtrk` wrapper is not read here.** A track file is a short header (a magic
line, tags, a title) then a bundle; its UnityFS signature is found by searching,
once, in the test scaffolding (`crates/unity/tests/common/mod.rs`). Unwrapping it
is the content layer's job.

**Streams are in use.** An earlier probe read "no external `.resS`" off the
tracks; the ships carry **275 of the census's 2,302 textures** in their bundle's
`.resS` node (47 nodes, 136 MB), referenced by `m_StreamData`. They are read
(`Assets::texture`). The 78 `.resource` nodes (129 MB) are audio streams,
measured: all 134 `AudioClip` objects in the census have an
`m_Resource.m_Source` naming a `.resource` node (asserted in
`classes_ground_truth`; UnityPy agrees). They are not read: **audio is out of
scope**.

## UnityFS container

All integers big endian. Measured on all 79 bundles.

| Offset | Field |
| --- | --- |
| 0 | `"UnityFS\0"` |
| 8 | `u32` archive version (6 or 7; others refused as `UnsupportedArchiveVersion`) |
| | player version cstring (`5.x.x`), engine version cstring (`2018.3.8f1`) |
| | `i64` total size, `u32` compressed blocks-info size, `u32` uncompressed blocks-info size, `u32` flags |

Flags: low 6 bits = compression of the blocks info (0 none, 1 LZMA, 2 LZ4, 3
LZ4HC; 4 LZHAM is refused by name), `0x40` = blocks info and directory are
combined (set on every census file), `0x80` = blocks info at the end of the
file, `0x200` = padding before it. **Any other bit is refused** as
`UnsupportedFlags`; none occurs in the census, and `0x80`/`0x200` are *not
exercised*.

**Version 7 and later align to 16 bytes after the header** before the blocks info
(measured on the one v7 bundle and the v8 fixture; version 6 does not). **Archive
version 8 (Unity 6) is read**; with flag `0x200` the block data is also padded to
16 after the blocks info (measured on the fixture, whose blocks info ends 7 bytes
short of a boundary). Blocks info, once
decompressed: 16 bytes of hash (**not verified**), `i32` block count, each block
`u32` uncompressed size, `u32` compressed size, `u16` flags (compression in the low 6
bits, `0x40` "streamed" ignored), then `i32` node count, each node `i64` offset,
`i64` size, `u32` flags, cstring path. Block data follows, concatenated and
decompressed into one run that nodes slice. Every node range is checked against
that run.

LZMA blocks are Unity's framing: 5 property bytes (`lc/lp/pb` and the
dictionary size) and the stream, with **no size field**; the uncompressed size
comes from the block table, and the output must match it exactly. LZ4 and LZ4HC
share one block decoder (the HC variant differs only in the encoder).

**Bounds** (all of it is untrusted user content): decompressed total at most 1
GiB, node count at most 65,536 and at most what the info bytes can hold, block
count at most what they can hold, LZ4 declared sizes at most 255x the input,
LZMA dictionary memory at most 256 MiB, up-front reserve at most 64 MiB. The
reader returns a named `Error`; no input panics (synthetic tests in
`tests/synthetic.rs` flip 20,000 bits and cut every prefix; `tests/hostile_real.rs`
does the same to a real LZ4HC ship and a real LZMA track).

## SerializedFile

Versions 17 through 23 are read; anything else is
`UnsupportedSerializedVersion`. The Workshop census has 17 and 22; the
project fixture is **23**; **18 to 21 are implemented from the published layout
and not exercised**. **Version 23** prefixes each type's typetree with a 16-byte
content hash and an `i32` byte size (0 = no tree stored), and the blob itself
starts with `mhtt` and the version again; the node layout is unchanged from 19
(32 bytes). Measured on the fixture's 6 files: every object consumes exactly its
bytes.

Header (big endian): `u32` metadata size, `u32` file size, `u32` version, `u32`
data offset, `u8` endianness (0 little) and 3 reserved bytes. **Version 22
repeats these as wide fields** after the reserved bytes: `u32` metadata size,
`i64` file size, `i64` data offset, `i64` unknown (**unread**); the first four
are zero-ish stale copies there. Measured on 2 (bundle) + 27 (install) version 22
files.

Metadata, in the file's endianness: engine version cstring, `i32` target
platform, `u8` has-typetree, then the type table (per type: `i32` class id, `u8`
stripped, `i16` script index, a 16-byte script id **only when the class is 114**, a
16-byte type hash, the typetree blob; from version 21 also a dependency list or ref-type
names), the object table, script types, externals, from version 20 the ref
types, and a user-information string. An object is `i64` path id (4-aligned,
**8 bytes** in version 17+), `u32` byte start (`i64` in 22), `u32` byte size,
`i32` type index; its class id is its type's. **A MonoBehaviour type is per
script**, so objects are tied to a type by index, never by class id. Unread
fields: the type hashes, script ids and dependencies (kept or skipped, not
interpreted), the externals' GUIDs (kept), the user information string.

**PPtr resolution is `(file, path_id)`, never `path_id` alone**: ids collide
across the files of one bundle (the first probe's own scripts hit this).
`m_FileID` 0 is the same file, `n` is `externals[n-1]`, matched by the path's
last segment to a node of the bundle (`Assets::resolve`).

### Typetree blob

`u32` node count, `u32` string-buffer size, then nodes of **24 bytes (versions
< 19) or 32 (>= 19)**: `u16` version, `u8` level, `u8` type flags (bit 0 =
array), `u32` type-string offset, `u32` name-string offset, `i32` byte size, `i32`
index, `u32` meta flags, and from version 19 a `u64` ref-type hash. The size
`count x node size + string size` must fit the file. A string offset with the
**high bit set** indexes Unity's built-in common-string table
(`oag_unity::common_strings`), otherwise the file's own buffer. The table's
names were taken from UnityPy 1.25.4's copy (facts, not code); it is a newer
table than 2018 (it ends in Unity 6 names), but it only ever grew at the end,
and every high-bit offset of all 111 files lands on a string boundary and builds
a tree that consumes every object exactly - which a wrong name would not
survive. An unknown offset is `UnknownCommonString`.

### Value reader

A generic typetree walk into `Value` (`Bool`, `Int`, `UInt`, `Float`, `Double`,
`String`, `Bytes`, `Array`, `Struct`, `PPtr`). Rules, all *measured* by the
consumption check below:

- `bool`, `SInt8`/`UInt8`/`char`, 16-, 32-, 64-bit integers, `float`,
  `double` by their names; `unsigned int`, `Type*`, `FileSize`, `long long` etc. too.
- `string`: `i32` length, bytes, align 4. Non-UTF-8 is refused (`InvalidUtf8`).
- `TypelessData`: `i32` length, bytes (mesh vertex and index buffers, texture pixels).
- An array node (type flag bit 0): `i32` count, elements. Arrays of 8-bit
  integers come back as `Bytes` (so `SInt8` loses its sign: the caller applies it).
  A node whose only child is an array (`vector`, `map`, `set`) reads as that array.
- `PPtr<...>` reads as `PPtr { file_id, path_id }`.
- **Meta flag `0x4000` aligns the stream to 4 after the node** (relative to the
  object's first byte).
- Anything else with children is a struct; a leaf of an unknown type is
  `UnsupportedType`.

**Coverage is exact, not sampled:** `SerializedFile::read_object` fails with
`SizeMismatch` unless the walk consumed *every byte* of the object. The ground
truth reads all 123,083 objects of the 111 files with zero failures; nothing is
guessed past. Bounds: array and string lengths are checked against the bytes
left, node count against a budget of `16 x bytes + 4096`, depth at 128.

## Built-in classes

| Class | Id | What is read |
| --- | --- | --- |
| `GameObject` | 1 | name, layer, tag, active, component pointers (`ComponentPair` and the older `pair` shape) |
| `Transform` | 4 | local rotation (x,y,z,w), position, scale, children, parent; `Assets::transform_path` walks to the root |
| `Material` | 21 | name, shader pointer, texture slots by name (`_MainTex` ...) with scale and offset, float and colour properties |
| `Texture2D` | 28 | size, format, mip count, raw pixels (inline or from a `.resS` node), no decode of block formats |
| `Mesh` | 43 | vertex channels over stream-packed data, indices, submeshes; positions, normals, tangents, colours, UV sets, triangles |
| `MonoBehaviour` | 114 | script pointer, game object pointer and the **uninterpreted generic value tree** |

All 88,703 + 8,459 + 8,306 + 805 + 2,302 + 1,570 objects read; none is refused.

### Mesh

*Measured.* `m_VertexData` is `{ m_VertexCount, m_Channels[14], m_DataSize }`
(`m_DataSize` is the buffer itself, a `TypelessData`). A channel is `{ stream,
offset, format, dimension }`; slots 0 position, 1 normal, 2 tangent, 3 colour, 4
to 11 UV0 to UV7, 12 and 13 blend weights and indices (the last two are not
decoded). Per stream: stride = the largest `offset + dimension x component size`
over its channels; stream `s` starts at the previous stream's end rounded up to
16. **The reader requires `end of last stream == buffer length` exactly** and
refuses the mesh otherwise (`Malformed`): this held on all 1,570 meshes (41 of
them have no vertices). Then the index buffer is `u16` (`m_IndexFormat`
0) or `u32` (1); a submesh is a byte offset, an index count, a topology and a
base vertex that is added to each index. Indices past the vertex count are refused.

**The channel format ids are not one table.** UnityPy carries three: before 2017
five ids (float, float16, colour, byte, uint32); **2017 and 2018** thirteen, with
the 8-bit types one higher than later (colour 2, UNorm8 3, SNorm8 4, ...); 2019 and
later twelve, the public `VertexAttributeFormat` enum (0 float32, 1 float16, 2
UNorm8, 3 SNorm8, 4 UNorm16, 5 SNorm16, 6 UInt8, 7 SInt8, 8 UInt16, 9 SInt16, 10
UInt32, 11 SInt32). Ids 0 to 2 mean the same in all three. The reader accepts 0
to 2 for every engine, **ids 3 to 11 only for engine 2019 and later, and refuses
an id of 3 or more from an older engine by name** (`Unsupported`): the census
never uses one, the byte-exact stride check cannot tell the 1-byte types apart,
and the two older tables disagree with each other.

**What the census exercises:** Float32 on every channel of every mesh, both engine
versions, and UNorm8 x4 for 2018 vertex colours (186 meshes), all on stream 0
with 16-bit indices (and 32-bit on 2 meshes), triangle lists only. **Multiple
streams, Float16, the 2019 table's ids 3 to 11 and triangle strips are
exercised only by the synthetic tests in `tests/synthetic.rs`**, which build a
mesh from the reader's own reading of the layout: they prove the code does what
this page says, not that Unity does. Quads, lines and points are `Unsupported`; a
compressed mesh (`m_MeshCompression != 0`) and a mesh with data in an external
stream are refused by name (none in the census).

### Texture2D

*Measured.* Formats seen: `ARGB32` 584, `DXT1` 1,218, `DXT3` 1, `DXT5` 228,
`RGB24` 177, `RGBA32` 94, all `m_TextureDimension` 2. The reader checks that the
pixel bytes are **exactly the mip chain** for the format (a block format rounds
each level up to 4x4 blocks) and refuses an unknown format
(`UnsupportedTextureFormat(id)`), a size mismatch, a crunched format, and an empty texture
that carries pixels. A `0 x 0` texture is empty by name, not an error: Unity's
dynamic-font atlas is the one in every track. Rows are bottom-up in the file;
`base_level_rgba8` returns top-down for the uncompressed formats, and **its
channel order is checked pixel for pixel**: for all 823 `ARGB32`, `RGB24` and
`RGBA32` textures (`ARGB32` is bytes A,R,G,B) the CRC-32 of the reader's RGBA8
equals the CRC-32 of UnityPy's `image`. **Block formats are not decoded in the
crate**: `oag_texture::bcn::dxt1` / `dxt45` decode a block, and the export test
shows the whole path; the two DXT1 images it wrote (a ship livery and a track
atlas) were looked at and are upright and legible, which is a look, not a
measurement.

## Verification

Ground truth, all `#[ignore]`d and going through `oag-testdata`'s skip-or-fail
contract (`OAG_REQUIRE_GAME_DATA=1` makes a missing tree a failure):

- `bundle_ground_truth`: 79 of 79 bundles open, the census table above.
- `serialized_ground_truth`: 111 files, 123,083 objects, 0 leftover bytes.
- `classes_ground_truth`: every built-in object reads or is refused by name (0 refused).
- `oracle_ground_truth`: with `OAG_UNITY_ORACLE` set to a table written by
  [`scripts/unity-oracle.py`](../../scripts/unity-oracle.py) (UnityPy 1.25.4 as a
  second implementation), **1,529 meshes match** on vertex count, triangle count and
  the absolute sums of positions, normals and UV0; **2,302 textures match** on
  size, format, mip count and complete image size, and **823 uncompressed
  textures match pixel for pixel** (CRC-32 of the top-down RGBA8). UnityPy's exporter refuses 41
  meshes; all 41 have no vertices in this reader too.
- `known_items`: Lion's Den (`1392209921`, 43 meshes / 26,009 triangles / 38,604
  vertices, 79 textures, 2,409 + 162 objects) and Cliffhawk (`2050936018`, 17
  GameObjects, `MatCliffhawkClean`'s three slots resolve to the right textures).
- `hostile_real` (a damaged real LZ4HC ship and LZMA track, Lion's Den) and
  `synthetic` (a built bundle and SerializedFile with every prefix cut and 20,000
  bit flips, LZ4 vectors, and the mesh layouts above): damaged input returns
  errors, never a panic.
- `install_census`: the game install's 27 version 22 files open and refuse by name.
- `export_samples` (with `OAG_UNITY_EXPORT=<dir>`): an OBJ and wireframe PNG of
  Cliffhawk's body and Lion's Den's floor, and the two texture PNGs, written for a
  person to look at. Viewed: the ship reads as a ship, the atlas and the livery
  are upright with legible text.

## Confidence

Scores follow the [rubric](../reverse-engineering/confidence-rubric.md); the
evidence column names the check. Nothing here has a runtime trace of Unity's own
reader, so the ceiling is 94.

| Claim | Score | Evidence |
| --- | --- | --- |
| UnityFS v6 header, blocks info, block and node tables, LZMA and LZ4HC framing | 92 | all 79 bundles open; every block decompresses to exactly its declared size; every node range lies inside the data |
| Version 7 aligns to 16 after the header | 80 | one v7 bundle, which fails to parse without it |
| Combined info and directory flag `0x40` (set on all 79); block flag `0x40` "streamed" (ignored) | 70 / 55 | constant across the census, so the reader's ignoring them is untested |
| Flags `0x80` (info at end) and `0x200` (padding), uncompressed and LZHAM blocks | 55 | published layout only; no census file uses them |
| SerializedFile version 17 layout | 93 | 109 files, 123,083 objects, each object's walk consuming its bytes exactly |
| SerializedFile version 22 header with wide fields | 85 | 2 bundle files read exactly, plus 27 install files whose headers, type and object tables parse |
| SerializedFile version 23 (hash, size and `mhtt` prefix) and UnityFS version 8 | 85 | the fixture, all three codecs, exact consumption, UnityPy agrees; one engine build |
| SerializedFile versions 18 to 21 | 50 | interpolated from 17 and 22; never seen |
| Typetree blob nodes of 24 bytes (< 19) and 32 (>= 19) | 93 / 85 | `count x size + strings` closes on all 111 files; 19 to 21 not seen |
| The built-in common-string table (names from UnityPy; used offsets resolve) | 92 | every high-bit offset in 111 files lands on a boundary and builds a tree that consumes its objects exactly |
| "The table only ever grew at the end", so a newer table serves older files | 70 | inferred from Unity's habit; only the prefix the census uses is tested |
| Meta flag `0x4000` aligns to 4 after a node; arrays of 8-bit integers as bytes; `vector` as its array | 93 | exact consumption of every object |
| Mesh stream layout (stride = largest channel end, streams at 16-byte boundaries) for stream 0 | 93 | `end of last stream == buffer length` on 1,570 meshes and the oracle match on 1,529 |
| The same for more than one stream | 55 | synthetic test only |
| Vertex format ids 0 and 2 on engines before 2019; id 0 on 2020 | 93 | 1,570 meshes; 186 with UNorm8 colours |
| Id 1 (float16) in every table | 65 | agrees across UnityPy's three tables; synthetic test only |
| The 2019+ table, ids 3 to 11 | 72 | the public enum; never exercised |
| The 2017 and 2018 table, ids 3 and up | n/a, refused | UnityPy's two older tables disagree about them |
| Index format, `baseVertex`, submesh topology 0 | 92 | indices stay inside the vertex count on all meshes; triangle counts match UnityPy |
| Triangle strips | 60 | synthetic only |
| Texture mip-chain sizes for DXT1, DXT3, DXT5, ARGB32, RGB24, RGBA32 | 93 | pixel bytes equal the mip chain on 2,302 textures; sizes match UnityPy |
| Channel order of ARGB32, RGB24, RGBA32 | 94 | pixel CRCs match UnityPy on 823 textures |
| Texture pixels in a bundle's `.resS` node | 93 | 275 streamed textures validate against their mip chain and match UnityPy |
| `.resource` nodes are audio streams | 90 | 134 of 134 `AudioClip`s name one |
| `GameObject`, `Transform`, `Material` field meanings and `Assets::transform_path` | 88 | exact consumption on 17,570 objects; one ship's hierarchy path and material slots checked by hand |

## Not covered

- Meshes: compressed meshes, blend shapes and skinning (the data is read as
  values; nothing decodes it), and the exercised-only-synthetically items above.
- `Texture2D` formats beyond the six seen, crunched textures, cube maps and arrays.
- Audio clips and the 78 `.resource` streams. The `.wtrk` and `.shp` wrappers.
- Anything about what a `MonoBehaviour`'s fields mean.
- Bundles with a `0x80` blocks-info-at-end flag, LZHAM, and SerializedFiles of
  versions 18 to 21, which are implemented from the layout but never seen.
- Vertex format ids of 3 or more from an engine before 2019 (refused by name).

## Cross-title check

Unity content shares no code or data with the Wipeout titles, so the 2048 and
Omega cross-check has nothing to compare: **not checkable (no common ancestry)**.
The one overlap is the BCn block decoders, which `oag-unity` reaches only from
its tests through `oag-texture`.
