# 2048's PSARC and title plumbing are wiring-ready; two binary formats changed underneath a track

2026-08-26. Scoped to "load a Wipeout 2048 craft and track to spawn in via
`just play 2048 --race`, no menu/intro" - see [2048 vs HD/Fury lineage](vita-2048-vs-hd-fury-lineage.md)
for the codebase-lineage question this corroborates further. Worked in
`../oag-2048` (`git worktree add`), against `data/extracted/vita/PCSF00007/base/PSP2/data.psarc`
(1.6 GiB, EU base package, decrypted per
[the eboot thread](2048s-vita-eboots-are-imported-re-not-started.md)'s pipeline -
the patch archives were deliberately not touched, see below). Two scratch probes
are the reproducer for every claim here: `crates/game/examples/psarc_list.rs`
(lists/filters a loose `.psarc`'s manifest) and `crates/game/examples/vita_probe.rs`
(everything else, `cargo run -p oag-game --example vita_probe -- <psarc path>`).

## What already works, unmodified

**The PSARC container reads with zero code changes.** `oag_assets::psarc::Archive::open`
parses `data.psarc` (version 1.4, flags 1, 18,430 entries) exactly as it parses
HD/Fury's PS3 archives (version 1.3, flags 3) - the header fields are read
generically and nothing asserts either value. The manifest gives every path as a
real string (`data/art/published/environments/altima/track.vex`), same as HD.

**The asset *tree* is HD/Fury's, not Pulse's**, corroborating
[the lineage thread](vita-2048-vs-hd-fury-lineage.md) a second, independent way:
`.rcsmodel`/`.rcsmaterial`/`.rcsskeleton`/`.rcsanimclip`, `.pob`, `.pvs`/`.probes`,
`.envsettings`, `.bnk` - HD's whole extension family, present here too. 2048 also
ships all fourteen HD teams verbatim under `data/art/published/hdships/<Team>/`
(`Ship.vex`, `handlingstats.xml`, `engineflare.vex`/`.rcsmodel`, the `_c1`/`_n1`
Fury variants, everything), alongside its own five-team native roster (see
"What's new" below) and eleven native circuits
(`altima`/`arena`/`bridge`/`cathedral`/`mall`/`park`/`sol`/`square`/`subway`/`tower`,
plus a `shared` directory) under `data/art/published/environments/<name>/`, laid
out exactly like HD's own circuits (`track.vex`, `track.rcsmodel`,
`track_col.col`, `stats.xml`, `start_grid.vex`, `TrackStartup.xml`, `track.pvs`).

**Every HD-derived handling file decodes with the existing parser, unchanged.**
`oag_formats::handling::from_blob` on `data/art/published/hdships/AG_Systems/handlingstats.xml`
returns the expected four classes (`VENOM`/`FLASH`/`RAPIER`/`PHANTOM`).
`oag_formats::handling::global_from_blob` on `data/xml/handlingstats.xml` (the
`GLOBAL_ENTRY` analogue) returns a fully sane `Global` block - zone
start/increment/recharge, four classes' worth of speedup-pad and gravity
tunables, weapon-pad refresh times - so the shared `<Global>` schema is
unchanged too.

**The native roster's own handling files also decode**, which was not expected
going in: `data/HandlingStats/ag_systems2048/1/handlingstats.xml` parses with
five classes (`VENOM`/`FLASH`/`RAPIER`/`PHANTOM`/`SUPERPHANTOM` - a class the
existing schema has no trouble with, it is just another `<Class>` block). The
five teams are `ag_systems2048`, `auricom2048`, `feisar2048`, `piranha2048`,
`qirex2048` - 2048's real five-team roster - each with four numbered
subdirectories (`1`/`2`/`3`/`4`) whose relationship to speed class or ship
variant is unread.

**The ship-directory root is a title fact that has to change, and it is a clean
one.** `crates/title/src/race.rs`'s own module doc argues, in so many words,
that `Data\Ships\<Team>\...` is shared vocabulary and not a title field - true
for Pulse, Pure and HD alike, all three of which store it at `Data\Ships\`.
2048's HD-derived roster does not: `Data\art\published\hdships\<Team>\handlingstats.xml`
is confirmed resolving against the manifest (`Archive::contains`, exact literal
match after the normal backslash/case fold). This is the third-title
disagreement [ADR-0022] asks for before an axis becomes a field, and it is not
just `handling::entry_name` - `oag_pulse::race::ships::entry_name(team, model)`
builds the same `Data\Ships\{team}\{model}` shape and several call sites in
`oag-game` (`race/assets.rs`'s `ship_entry_name`/`boost_entry_name`/`shield_entry_names`,
`livery/flare.rs`, `boot/roster.rs`) go through it for the hull, boost plume,
shield and flare. None of that is wired yet - see Next Steps.

**Skipping the patch PSARCs is a deliberate, documented simplification, not an
oversight.** `data.psarc` alone carries all 18,430 entries the census above
depends on; the patch's `data1.psarc`/`data2.psarc` (136 MiB combined, inside
`patch-v104/PSP2/`) hold some subset of changed entries and were not inspected.
Patch-over-base is this project's policy for *which binary to decompile*
(EU 1.04, per `CLAUDE.md`); it says nothing about which PSARC an asset-loading
title package should prefer, and reading only the base keeps this pass's
footprint down. Worth mounting the patch archives ahead of base, HD's
`extra`-archive-order style, once something needs a file the patch actually
changed.

## What's new, and blocks a track from loading today

**`track.vex`'s class table is version 6, byte-order little-endian - identical
class IDs to HD's version 6** (measured via `vex::classes_of` on
`environments/altima/track.vex`: `mesh`, `wo_track`, `section`, `skycube`,
`floor_collision`, `engine_flare`, all the same numbers HD's version-6 tracks
carry). `mesh::geometry_is_external` reads `false` here because its whole test
is "is the vex file's own byte order big-endian" - a PS3 (PowerPC, BE) proxy
that a Vita (ARM, LE) file defeats by construction, even though the geometry
*is* external exactly as it is on HD. This one is a cheap, well-understood fix
once the two formats below are readable: the heuristic needs a third answer
alongside `Big`/`Little`, or a Vita-specific override.

**The `.rcsmodel` beside it is not HD's container**, and this one needed
Ghidra, not a stride search: `oag_formats::rcsmodel`'s header read is
`ByteOrder::Big.u32(data, 0)`, unconditionally, and expects `0x000a0000`;
`environments/altima/track.rcsmodel`'s first four bytes are `ed ad 5c ca` read
either way - not a byte-swap of the expected constant, a different value
entirely, at 17.4 MiB for one circuit (HD's biggest circuit `.rcsmodel` is
nowhere near that). `RcsModel_Load` (`0x812f15b2` in `vita-2048-eu-v104/eboot.elf`,
named this session, confidence 85 - see
[track-and-collision-loaders.md](../docs/ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md))
is `Live::Rcs::Model`'s own loader, named off its allocator tags
(`"PSP2/Psp2.RcsModelLoader.cpp"`) and two error strings that name the class
directly. It peeks the first 32 bytes to learn a size at `+0x0c`, allocates
and reads exactly that many bytes as one block ("section A"), then reads two
more fields *out of that just-loaded block* (`+0x24`, `+0x44`) and uses each
as the size of a further block it allocates and reads in turn - a "main
memory" (CPU-resident) block and a GPU block - before walking section A's own
relocation table twice. Checked at the instruction level, not just decompiled:
`812f160a` reads the allocation size from the 32-byte peek's own `+0xc`, and
`812f1658`/`812f1664` read the other two sizes out of the *loaded* section A,
not the raw file. Checked against the real file: `size(+0x0c) + size(+0x24)
+ size(+0x44)` equals the file's own length exactly (`189824 + 682966 +
16573144 = 17445934`) - the same "does the arithmetic close on the real
bytes" bar the `WO Track` stride cleared. **What is still unrecovered**: the
interior of all three sections - what the rest of section A's own fields are,
and the chunk/relocation shape inside the two loaded blocks. That is
genuinely more RE, not a quick follow-up.

**The `WO Track` (AI spline) payload is version `0x107`, one above every
previously-seen version (Pulse ships `0x105`), and its per-point record shrank
from 112 to 96 bytes.** This is the one format this pass got most of the way
into, and it is worth recording precisely:

- Header, path array, junction array are all **unchanged** - same offsets,
  same 32/16-byte strides. Confirmed by reading every path's and junction's
  header fields directly and getting plausible counts across all fourteen
  `track.vex` entries the archive carries (ten native circuits, four DLC).
- `AiTrack::encoded_len()`'s formula matches the payload **exactly, on all
  fourteen tracks** - `HEADER_LEN(0x20) + RESERVED_LEN(0x20) + paths*PATH_LEN(0x20)
  + junctions*JUNCTION_LEN(0x10) + total_points*96` lands on the file's own
  payload length to the byte for every one of them (`altima`: 6 paths/4
  junctions/2029 points/195104 bytes; `mall`, the largest, 18 paths/12
  junctions/3288 points/316480 bytes; and twelve more in between), and stride
  112 (the pre-2048 value) matches **none** of them. **Confidence 95** - a
  corpus-wide exact arithmetic closure with the alternative hypothesis
  (stride unchanged) failing on every member is about as strong as evidence
  gets without reading the loader itself.
- **`pos`/`tangent`/`down`/`lateral` stayed at their original offsets** (`0x00`,
  `0x10`, `0x20`, `0x30`, each a 16-byte slot for a 12-byte vector). Found by
  scoring every candidate *stride* 16..160 against how smoothly consecutive
  points' position vectors move (right stride: points sit metres apart; wrong
  stride: noise two to six orders of magnitude worse - 96 wins over the next
  candidate, 8497 vs 10.5 million), then, at the confirmed 96-byte stride,
  scoring every 4-byte offset by how close a `[f32;3]` read there sits to unit
  length across all 2029 points of `altima`. `0x10`/`0x20`/`0x30` land at
  ~2.5e-8 mean error (float rounding on a genuinely unit vector); every other
  offset is at least six orders of magnitude worse. (An earlier pass of this
  probe under-ranged that scan and left two unscanned slots at their
  initialised zero, which briefly looked like a suspicious perfect match -
  fixed; there is no unexplained zero-scoring offset once every slot is
  actually measured.) **Confidence 90.**
- **The tail is now placed, off Wipeout HD rather than off a statistical
  sweep.** 2048's DLC re-ships twelve HD circuits (`Vineta_K`, `Ubermall`,
  `Sebenco_Climb`, `Sol_2`, `amphiseum`, `modesto_heights`, `talons_junction`,
  `tech_de_ra`, `zone_1`..`zone_4`) whose path counts, junction counts and
  per-path control-point counts are **identical** to HD's own copies, so point
  `k` of path `i` is the same point in both files and HD's fully decoded
  112-byte record is ground truth for 2048's 96-byte one - 10,165 paired
  points. `half_width_left/right` and `ai_bound_left/right` **did not move**
  (`0x44`/`0x48`/`0x4c`/`0x50`), nor did `racing_line` (`0x54`); `section_id`
  and `flags` moved from `0x60`/`0x61` to **`0x5c`/`0x5d`**. Confidence 95 for
  the floats, 90/92 for `section_id`/`flags`, 80 for `racing_line` (HD authors
  zero everywhere, so the pairing cannot distinguish it - it rests on the
  layout either side of it being confirmed). `crates/formats/src/track.rs`
  carries this as `point_len(version)` plus a version-gated tail offset, and
  all fourteen circuits in 2048's base package now parse with
  `encoded_len()` landing on the payload length to the byte. Full evidence -
  the word-by-word agreement cliff at `0x58`, the exact `flags` histogram, and
  the `section`-node-count check that settles `section_id` without relying on
  HD's numbering - is in
  [`docs/formats/track.md`](../docs/formats/track.md); the reproducer is
  `crates/game/examples/vita_rosetta.rs`.
- **What is still unplaced in the record**: `0x58`/`0x59` (vary per point, up
  to 71 distinct values on one circuit), `0x5a`/`0x5b` (constant `0xff`/`0x00`
  on all 28,290 points, so nothing separates field from padding) and
  `0x5e`/`0x5f`. None of them is needed to drive the track. One circuit does
  not close on `section_id` either: `cathedral` has 6 distinct ids with a
  maximum of 9 against 6 `section` nodes.

**Collision moved out of the vex file entirely and into its own container -
a k-d tree, and its outer shape is now read.** `collision::from_vex` correctly
returns zero nodes (not a bug - the collision class IDs are in the class table
with no node instances, checked in the code before it settles for empty rather
than erroring). The 1,068,082-byte `track_col.col` beside `track.vex` is a
*new*, wholly separate top-level file, and `KdTree_Load` (`0x8118d134`,
confidence 87, same evidence page) is its loader - named off its own format
string (`sscanf`-style, literally `"kdtr%04x"`, required to parse to `1`) and
two allocator tags (`"Backend/General/Collision/KdTree.cpp"`, with a
companion string naming a `KdTreeMeshShape.cpp` for the part this session did
not reach). That the branch checks the *parsed version* rather than the
`sscanf`-style call's own return value is confirmed by disassembly, not
assumed: the compared stack slot is explicitly zeroed before the call and
reloaded from the same slot afterward, never from the return register - the
shape of an out-parameter, not a field-count return. Reading the real file
against what the decompiled control flow predicts, every tag and count landed
exactly where expected:

```text
+0x00  char[4]  "kdtr"
+0x04  char[4]  ASCII version, required to parse as 1 - "0001" on every file seen
+0x08  char[4]  section tag, literal "----", repeated before each section below
+0x0c  u32      node stride - 24 bytes, this file
+0x10  u32      node count N - 27,199, this file
        then    N * 24-byte k-d nodes: two child-node indices (serialized, fixed
                 up to real pointers at load) as the first 8 bytes, 16 bytes
                 unrecovered
       "----"
+…     u32      leaf/triangle index count M - 93,279, this file
        then    M * u16 leaf indices
       "----"
        then    axis-aligned bounding box, 2 * [f32;3]
       "----"
        then    the mesh-shape trailer `KdTreeMeshShape.cpp` owns - 228,692
                bytes on this file, entirely unread
```

Full evidence, including why each function is named at the confidence it is,
is in
[track-and-collision-loaders.md](../docs/ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).
**What is still unrecovered**: the k-d node's own 16 trailing bytes (split
axis/value and bounds, by hypothesis - not read), and the entire mesh-shape
trailer, which is where the actual triangle geometry this tree indexes lives.

## The wiring is in, and a race runs

`just play 2048 --race` (a new justfile case, naming the *directory*
`data/extracted/vita/PCSF00007` rather than an image) opens the package as
Wipeout 2048, reads Altima's spline, places the craft on the start line and
runs the simulation. What landed to get there:

- **`oag-2048`** (`crates/2048`), the fourth title package - archives, HUD
  layout entries, race defaults, sound banks. `front_end`/`loading`/`music` are
  `None` and `exhaust`/`flare`/`sights` are `Unread`, deliberately: 2048 ships
  HD's files under HD's names and changed the containers underneath them, so
  copying HD's answers would compose to something wrong rather than to nothing.
- **`Platform::Vita`**, and it is never identified from a disc - 2048's package
  is a directory, so it reaches `oag_assets::Layout` through the archive
  candidate that matched.
- **`RaceDefaults::ship_dir`**, the axis 2048 forced into existence, plus
  `RaceDefaults::ships()` and `oag_title::race::ShipPaths` bundling it with
  `zone_craft` so the two travel together. `handling::entry_name_in` and
  `ships::entry_name_in` take the directory; the old two-argument spellings stay
  as the three-title convenience wrappers.
- **`mesh::geometry_is_external` no longer asks the byte order alone.** Its
  whole test was "is this a big-endian `.vex`", which answers `false` for 2048
  on a little-endian console; it now also asks whether the first mesh batch
  declares GU vertex type `0`, which is what an exporter writes when the
  vertices are elsewhere. Without this a 2048 track went to the batch decoder
  and died on `unsupported GU vertex type 0x0000`.
- **A `.rcsmodel` that will not *decode* now degrades exactly like one that is
  not *there*** - the circuit falls back to the derived ribbon and a craft draws
  nothing, both reported. Two hard errors before this: `mesh::rcs::build_scene`
  and `mesh::rcs::build` were both `?`, so 2048's container stopped the race.

## The collision is in, and the craft flies

`track_col.col` is decoded whole - `oag_formats::kdcol`, validated over **all 26
shipped files** with every byte accounted for. `just play 2048 --race` now
reports `4 collision node(s) -> 4 collider(s), 15986 triangle(s)` on Altima and
the craft accelerates to 125 units/s down the circuit, `grounded 1.0` every
tick. Full evidence in [2048-collision.md](../docs/formats/2048-collision.md);
the three functions it was read from are on
[track-and-collision-loaders.md](../docs/ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).

Two things are worth carrying forward from how it was recovered:

- **The `.col` was expected to be the hard part and the surface byte was.** The
  container fell out of `SimpleMesh_Load` (`0x8118fac8`) in one read. What the
  per-triangle byte *meant* took two independent recoveries that agree exactly:
  `TrackCollision_MeshFromNode` (`0x8126f800`) switches on this project's own
  already-recovered `.vex` collision class IDs to pick it, and matching 2048's
  triangles against Wipeout HD's named collision classes over the twelve ported
  circuits gives 170,744 pairs with no disagreement on any of the six values.
- **Three values (10, 11, 12) come from the offline exporter and no class
  produces them.** They are placed as drivable floor on measurement - see the
  coverage table on the format page, whose control row is the part that makes it
  evidence. `12` is the weak one at confidence 50 and is flagged as such rather
  than folded into the other two's 85.

**The bounds correction is the trap worth naming.** Both bounding boxes in this
container are a **centre and a half-extent**, and the first pass recorded them
as a min and a max - which is exactly as plausible and puts the "max" corner
inside the geometry. It reads as a subtly wrong struct rather than a wrong
offset, the same shape as the `WO Track` reserved-block trap.

## The render geometry is in too, and 2048 races its own craft

`.rcsmodel` is read - **all three of the formats 2048 changed are now decoded**.
The container is nothing like Wipeout HD's: a linker-style image with relocation
tables, read little-endian, magic `0xca5caded`. `just play 2048 --race` draws
Altima at 413,358 triangles over 2,800 submeshes and the craft's own hull,
untextured and lit off computed face normals. Evidence in
[2048-rcsmodel.md](../docs/formats/2048-rcsmodel.md).

Three things from that pass are worth carrying:

- **The relocation table is the decode.** Section B is a serialized C++ object
  graph and its layout is unread; rather than guess it, the reader takes the one
  thing the header states exactly - where every pointer into the GPU section
  lives - and pairs them 28 bytes apart. That is checked rather than
  pattern-matched: 244,889 of 244,889 submeshes have an index buffer of exactly
  `count * 2` rounded to four with the count divisible by three.
- **No field states the vertex stride.** It comes from the buffer packing -
  section C is the buffers back to back, so differencing sorted pointer targets
  gives every length. Strides run 16 to 64 in fours.
- **`RaceDefaults` needed a second directory.** 2048's own five teams keep
  models under `Data\art\published\Ships\<team>\<1..4>\` and tuning under
  `Data\HandlingStats\<team>\<1..4>\`, so `handling_dir` joins `ship_dir`, and
  the team id is two levels (`feisar2048\3`). The numbered level is the four
  craft each team flies - fighter, agility, speed, prototype, in that order,
  recovered from the liveries each ships and corroborated by the front end's
  locked-ship art.

## What is left

None of it is a format any more; it is fields inside one, and presentation:

- **A 2048 model draws untextured.** The per-vertex normal and texture
  coordinate are in the file and unplaced (four bytes at `+0x0c`, `f16` pairs at
  the end), as is the material and texture binding. That is the next piece of
  work and it is what stands between this and a picture worth comparing.
- **The HD-derived roster is not reachable as a default on this title.** One
  `ship_dir` string cannot address both of 2048's trees; every file is there.
- No HUD art (2048's `.gxt` atlases sit under names this build does not ask
  for), no music, no front end. Zone has no hull here for the same `ship_dir`
  reason.

## Why nothing was wired up before that

Three things gate a race spawning at all, in `crates/game/src/race/load.rs`:
`track_render::load` (hard error via `?`, needs `WO Track`), `mesh::build_with_textures`
or `mesh::rcs::build_scene` for the drivable geometry (both currently error;
everything downstream of a missing model degrades to "drawn nothing" rather
than failing, per every other title), and `collision::from_vex` (does not error,
just returns nothing - a real track needs `track_col.col` instead, once its
own format is read). **`WO Track` was the one that was fatal, and is no longer**:
`oag_render::track::load` propagates its `OutOfBounds` error with `?`, and the
version-gated stride above is what clears it. Building the `oag-vita` crate, the `Platform::Vita`
disc variant, the `ship_dir` axis and the `just play 2048` justfile case now
would produce a command that compiles and then fails at exactly that line -
which is an honest result, but not the "spawn in" the task asked for, and
`CLAUDE.md`'s rule against inventing what the assets already author says not
to guess the tail-field offsets just to get past it: a wrong half-width or
`ai_bound` would silently steer the AI off the actual track rather than fail
loudly. Ghidra RE this session named the two loaders themselves
(`RcsModel_Load`, `KdTree_Load`) and their containers' outer section shape,
but not enough of either's interior, or the `WO Track` point tail, to write a
parser change yet - see What's New above and Next Steps below.

## Open

- The `.rcsmodel` container's three sections' own interiors - what fills the
  rest of section A past its own `+0x0c`/`+0x24`/`+0x44` size fields, and the
  chunk/relocation shape inside the "main memory" and "GPU" blocks
  `RcsModel_Load` reads whole. Only where the three sections start and end is
  known.
- The four unplaced bytes of the `WO Track` point tail (`0x58`/`0x59` and
  `0x5e`/`0x5f`), the constant `0xff`/`0x00` pair at `0x5a`/`0x5b`, and
  `cathedral`'s section id of 9 against 6 `section` nodes. The `WO Track`
  point reader in the eboot is still unnamed and unlocated - the layout was
  recovered from the HD pairing without it.
- The k-d node's `+0x12` half-word, `0x000b` on every node of every one of the
  26 files, so nothing distinguishes a field from a constant.
- What surface bytes `10`, `11` and `12` *are*, as opposed to how they behave.
  They are placed as drivable floor on measurement and no `.vex` class produces
  them, so the offline exporter is the only thing that knows.
- Whether the `.rcsmodel` container is shared verbatim by every 2048 track -
  `RcsModel_Load`'s section-size arithmetic was only checked on `altima`. The
  k-d tree half of this is now closed: all 26 files decode with every byte
  accounted for.
- The native roster's numbered subdirectories (`ag_systems2048/1..4`) and their
  relationship to speed class or ship variant.
- Whether the patch PSARCs (`data1.psarc`/`data2.psarc`) touch any of `altima`,
  `AG_Systems` or the global handling file at all - not checked, since base
  alone was sufficient for everything above.

## Next Steps

1. ~~Nail the `WO Track` tail layout~~ - done, see above. The HD pairing beat
   the planned statistical sweep outright: a sweep could only have said "this
   offset is track-width-shaped", where the pairing gives per-field ground
   truth per point.
2. ~~`RcsModel_Load`'s sections~~ - done, see above. What is left inside a 2048
   vertex is the **normal, the tangent and the texture coordinates**, plus the
   material and texture binding, which together are the difference between a
   grey model and a comparable picture. The strides are known exactly (16..64 in
   fours) and the position occupies the first 12 bytes of every one of them, so
   this starts from a known field boundary rather than cold. HD's own reading
   ([rcsmodel.md](../docs/formats/rcsmodel.md)) placed the same fields in a
   different container and is worth reading first.
3. ~~The `ship_dir` axis~~ - done, see above. It reaches the hull, the plume,
   the shield, the flare and the handling stats, and the roster filter in
   `boot::roster` goes through the same spelling, so the filter cannot disagree
   with the loader about where a ship is.
4. ~~The wiring~~ - done, see "The wiring is in" above. What it revealed is
   that **the k-d tree, not the `.rcsmodel`, is what stands between here and a
   flyable race**: a craft with no hull is an honest half-picture, a craft with
   no ground is not a race at all. So step 2's two halves are no longer equal
   in priority - `KdTreeMeshShape.cpp`'s trailer first, `RcsModel_Load`'s three
   sections after it.
