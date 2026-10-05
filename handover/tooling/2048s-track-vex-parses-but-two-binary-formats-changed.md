# 2048's PSARC and title plumbing are wiring-ready; two binary formats changed underneath a track

2026-08-26. Scoped to "load a Wipeout 2048 craft and track to spawn in via
`just play 2048 --race`, no menu/intro" - see
[vita-2048-eu-v104/README.md](../../docs/ghidra/functions/vita-2048-eu-v104/README.md#the-lineage-question-is-answered-confirmed)
for the codebase-lineage question this corroborates further (confirmed
2026-09-01). Worked in
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
[the confirmed lineage finding](../../docs/ghidra/functions/vita-2048-eu-v104/README.md#the-lineage-question-is-answered-confirmed)
a second, independent way:
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
`oag_tables::handling::from_blob` on `data/art/published/hdships/AG_Systems/handlingstats.xml`
returns the expected four classes (`VENOM`/`FLASH`/`RAPIER`/`PHANTOM`).
`oag_tables::handling::global_from_blob` on `data/xml/handlingstats.xml` (the
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
`oag-livery` (`entry.rs`'s `ship_entry_name`/`boost_entry_name`/`shield_entry_names`,
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
Ghidra, not a stride search: `oag_rcs::rcsmodel`'s header read is
`ByteOrder::Big.u32(data, 0)`, unconditionally, and expects `0x000a0000`;
`environments/altima/track.rcsmodel`'s first four bytes are `ed ad 5c ca` read
either way - not a byte-swap of the expected constant, a different value
entirely, at 17.4 MiB for one circuit (HD's biggest circuit `.rcsmodel` is
nowhere near that). `RcsModel_Load` (`0x812f15b2` in `vita-2048-eu-v104/eboot.elf`,
named this session, confidence 85 - see
[track-and-collision-loaders.md](../../docs/ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md))
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
  layout either side of it being confirmed). `crates/vex/src/track.rs`
  carries this as `point_len(version)` plus a version-gated tail offset, and
  all fourteen circuits in 2048's base package now parse with
  `encoded_len()` landing on the payload length to the byte. Full evidence -
  the word-by-word agreement cliff at `0x58`, the exact `flags` histogram, and
  the `section`-node-count check that settles `section_id` without relying on
  HD's numbering - is in
  [`docs/formats/track.md`](../../docs/formats/track.md); the reproducer is
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
[track-and-collision-loaders.md](../../docs/ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).
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
  `None` and `sights` is `Unread`, deliberately (`exhaust` and `flare` were read on 2026-10-05 - see 2048-status.md): 2048 ships
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

`track_col.col` is decoded whole - `oag_vex::kdcol`, validated over **all 26
shipped files** with every byte accounted for. `just play 2048 --race` now
reports `4 collision node(s) -> 4 collider(s), 15986 triangle(s)` on Altima and
the craft accelerates to 125 units/s down the circuit, `grounded 1.0` every
tick. Full evidence in [2048-collision.md](../../docs/formats/2048-collision.md);
the three functions it was read from are on
[track-and-collision-loaders.md](../../docs/ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).

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
[2048-rcsmodel.md](../../docs/formats/2048-rcsmodel.md).

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

## 2026-08-27, later: `Uv1` is cracked, the material table is read, and two independent gaps still keep everything untextured

Picking up exactly where "Open" below left off. `Uv1`'s type nibble (`8`, 2
components) is now placed the same way `normal`'s was: two little-endian
`f16`s, found not per-submesh but by anchoring on `position` per file and
keying a declaration map by stride (`psp2::vertex_decl::find_by_stride`) -
a per-submesh pointer at `+0x28` exists but resolves only 14.2% of the
corpus where the stride map resolves 92.5%. Confirmed against the same
index-exact oracle that settled the normal: 1,432 vertices, mean squared
distance to Wipeout HD's own diffuse UV indistinguishable from zero, against
0.41 (chance) for the only other 2-byte-per-component encoding the byte
budget allows. Confidence 96.

The material table is read too, found the same way as everything else in
this container: `.gxt`/`.rcsmaterial` text sits in the clear in section B,
so pair a pointer to what it points at. A file-level header names an offset
table of 64-byte material headers (name, technique name, and - since the
shader input table's own struct shape changes with entry count in a way this
pass did not solve - every texture found by scanning a material's own byte
extent for a pointer resolving to `.gxt` text). One real trap along the way:
the first cut capped the file-level material count at 64, calibrated from
small props and the 16-submesh craft this pass used as its primary sample,
which silently rejected altima's own 527 real materials as implausible - the
race report said "no material table" for a file that had one, and the fix
was raising the cap once 527/527 offset-table entries were confirmed to
resolve real headers, not guessing at the count.

**Two independent things still keep a race untextured, and neither is a
follow-on to the other.** First: which submesh uses which material is
genuinely unresolved, not merely unread - every submesh field was checked
against every material's identity and against its own still-uninterpreted
64-bit hash, and nothing matches. A model with exactly one material draws
through the binding regardless (`oag_mesh::mesh::rcs::psp2::build` now
takes a texture-loading closure and does this), but neither the raced craft
(`feisar2048\3`, 6 materials) nor the circuit (`altima`, 527) is that model.
Second: even a resolved binding would meet `oag_texture::gxt` refusing to
decode the pixel format almost every 2048 texture is stored in - a sweep of
every distinct diffuse `.gxt` the corpus's materials name (6,135) found 15
decode; 6,037 of the rest are `PVRTII4BPP`, which `docs/formats/gxt.md`
already named as undecoded before this session, for reasons that predate and
are unrelated to anything above.

Full evidence for both: `docs/formats/2048-rcsmodel.md`'s "`Uv1`'s type is
cracked too" and "The material table is read" sections. Reproducers:
`crates/game/examples/vita_rcsmodel_uv_oracle.rs` (the UV oracle),
`vita_rcsmodel_material_probe.rs` (finding the table structurally),
`vita_rcsmodel_material_count_bound.rs` (the 64-cap trap),
`vita_rcsmodel_render_e2e_check.rs` and `vita_rcsmodel_texture_format_census.rs`
(the PVRTII sweep).

## 2026-08-27, later still: both gaps are closed and a race draws textured

The two things the section above named as independent blockers both fell the
same day, in the order they had to. `just play 2048 --race` now reports:

```text
Ship.vex:  7416 triangle(s) over 16 submesh(es), 16/16 draw(s) textured
           from 5 of 6 material(s), 0 unresolved
track.vex: 413358 triangle(s) over 2800 submesh(es), 2782/2800 draw(s)
           textured from 521 of 527 material(s), 18 unresolved
```

`data/shots/2048_altima_textured.png` is the picture: Altima's start straight
with legible grandstand advertising, the overhead gantry banner, a panelled
road surface, and the Feisar craft in its livery with its wordmark readable on
the tail.

**`PVRTII4BPP` decodes** - `oag_texture::pvrtc`, confidence 92, 85% of every
`.gxt` on the disc. Three things are worth carrying:

- **The reference had to be the right one.** The public PowerVR SDK
  decompressor is PVRTC-**I** only, and PVRTC-II is a near-lookalike: decoding
  a `PVRTII4BPP` payload with it renders something that reads as a picture
  while being wrong. Vita3K's `renderer/src/texture/pvrt-dec.cpp` carries a
  PVRTC-II path its own team added, and that is what this ports. The
  difference is not academic - 7.1% of this corpus's words set the
  hard-transition bit that only PVRTC-II has.
- **Wipeout HD is the oracle again, and it is the strongest one yet.** 2048's
  DLC re-ships HD's circuits and roster, so 2,284 textures exist as both a
  `.gtf` and a `.gxt`. Median difference between the two decodes is 3.83 of
  255, against 10.01 with HD's copy flipped, 34.60 for the same payload read
  in raster word order, and 59.74 for chance. That is the same method that
  settled the `WO Track` tail, the normal and `Uv1`, and it should be reached
  for first on anything else 2048 shares with HD.
- **The smoothness metric this module used for `UBC2` is useless here** and
  was tried first: an untwiddled control scores only 1.4x rougher, because
  PVRTC's bilinear upscale smooths a wrong answer too. A font atlas
  (`RussianHud.gxt`, full Latin and Cyrillic, legible) is the picture check
  that works.

**The submesh-to-material binding is read** - a plain `u32` index into the
material offset table, `0x18` bytes **before** the record's own start,
confidence 90. The trap here is the interesting one, and the previous section
of this thread is the record of falling into it:

> Every submesh field was checked against every material's identity and against
> the submesh's own 64-bit hash, and none matched.

That search was sound and its conclusion was wrong, because **an index does not
resemble the thing it indexes**. A submesh carrying `3` matches nothing about
material 3. Searching for a *small integer in `[0, count)`* instead finds it
immediately - and the control group is what makes it evidence: nineteen offsets
in a `-0x60..+0x80` window are always in range and always zero on a
one-material model, and eighteen of them average 3.2 or fewer distinct values
per multi-material model where this one averages 80.3. Altima uses 525 of its
527 materials across 2,800 submeshes.

**It is measured, not read out of the executable, and that is what holds it at
90.** The Ghidra attempt is worth recording so nobody repeats it: all eleven
callers of `sceGxmDraw` (NID `0xBC059AFC`) in `vita-2048-eu-v104` turn out to
be debug primitives, sprite quads and the engine-flare effect. The mesh path
reaches the GPU some other way - a deferred command buffer, by hypothesis, not
read - so walking back from the draw call does not find it. No function was
named, so `names.tsv` gains no row.

Full evidence: [gxt.md](../../docs/formats/gxt.md) and
[2048-rcsmodel.md](../../docs/formats/2048-rcsmodel.md). Reproducers:
`crates/game/examples/vita_gxt_pvrtc_extent.rs` (the 16-byte level floor),
`vita_gxt_hd_texture_pairs.rs` and `vita_gxt_hd_oracle.rs` (the HD pairing),
`vita_gxt_pvrtc_smoothness.rs` (the metric that did not work, plus the
hard-transition and palette-path census),
`vita_rcsmodel_material_index_probe.rs` (the sweep and its control group) and
`vita_rcsmodel_material_index_check.rs` (the craft and circuit read out).


## What is left

None of it is a format any more; it is fields inside one, and presentation:

- **A 2048 model draws untextured, but it is lit off real normals now.**
  2026-08-27 found *where* the normal, tangent and texture coordinates are:
  section B carries a per-chunk vertex declaration structurally identical to
  HD's own
  ([2048-rcsmodel.md](../../docs/formats/2048-rcsmodel.md#section-b-carries-a-vertex-declaration-structurally-identical-to-hds)) -
  `normal@0x0c`, `tangent@0x10`, `Uv1@0x14`, `lightmapUV@0x18` on the common
  28-byte stride. HD's packed-normal encoding does not apply at that offset
  (chance-level dot product against HD ground truth), but the byte budget
  argued for a different shape - one signed byte per component, 4th byte
  padding - and it closed outright: **100% within 18° over a 1,504-vertex
  index-exact oracle, mean dot 0.994, and 33,335,682 of 33,335,682 normals
  unit-length across the full corpus**
  ([2048-rcsmodel.md](../../docs/formats/2048-rcsmodel.md#the-normal-is-cracked-three-signed-bytes-not-a-packed-word)).
  Implemented in `oag_rcs::rcsmodel::psp2::unpack_normal`, wired into
  `oag_mesh::mesh::rcs::psp2::build`, and `just play 2048 --race` now draws
  Altima lit off the file's own normals. What is still missing is the texture
  coordinate's own type nibble (`t8`) and the material/texture binding - that
  is what stands between this and a picture worth comparing.
- **The HD-derived roster is not reachable as a default on this title.** One
  `ship_dir` string cannot address both of 2048's trees; every file is there.
- No HUD art (2048's `.gxt` atlases sit under names this build does not ask
  for), no music, no front end. Zone has no hull here for the same `ship_dir`
  reason.

## Why nothing was wired up before that

Three things gate a race spawning at all, in `crates/raceplay/src/load.rs`:
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

## 2026-09-01: Race Remix's live team picker is the first thing to ever ask for a 2048 team by name, and none of the 17 it offers resolve

Found while a human tested Race Remix (`handover/race-remix-backend-lands-menu-wiring-is-next.md`)
interactively for the first time, with 2048 as the CRAFT TITLE: picking any
team failed to load, `cannot start a race: reading Data\HandlingStats\Auricom2048\handlingstats.xml
... has no entry at Data\HandlingStats\Auricom2048\handlingstats.xml`. Not a
Race Remix bug - `race::load` resolves a team the same way for every title,
`handling::entry_name_in(craft_title.race.handling_dir, &team)`, and that is
exactly what every other title's own RACE page already does. What changed is
reachability: 2048 has no front end, so before Race Remix the only team
`race::load` was ever asked for on this title was the hardcoded
`oag_2048::race::DEFAULT_TEAM` (`feisar2048\3`, `--race` with no `--team`) -
nothing before now ever tried a second one.

**The reproducer**, `crates/game/examples/remix_2048_roster_probe.rs`: of the
17 teams `crate::catalogue::teams` reads off 2048's plugin definition, **0
resolve** against `craft_title.race.handling_dir` joined directly with the id
- the same join that works for every other title. Two different reasons, by
roster half:

- **The 5 native teams** (`Feisar2048`, `Qirex2048`, `Piranha2048`,
  `AG_Systems2048`, `Auricom2048`) need a *second* path level this session's
  own earlier finding already named but did not wire anywhere reachable: "the
  team id is two levels (`feisar2048\3`)". The plugin definition's own
  `PI_Team` declares only the base name: `crate::catalogue::teams` has no way
  to produce `feisar2048\3` from it, because nothing in the definition states
  which of the four numbered variants a bare pick should mean. All 20
  (5 teams x 4 variants) resolve fine once the number is supplied by hand -
  confirmed variant by variant in the probe.
- **The 12 HD-derived teams** (`Feisar`, `Qirex`, `Assegai`, `Auricom`, ...)
  need [`HD_SHIP_DIR`](../../crates/2048/src/race.rs) instead of
  [`HANDLING_DIR`](../../crates/2048/src/race.rs) - the tree "What is left"
  above already named ("One `ship_dir` string cannot address both of 2048's
  trees"), now confirmed for every one of the 12, not just asserted for one.

**What would actually fix it:** `oag_title::RaceDefaults` carried exactly one
`ship_dir`/`handling_dir` pair per title, which was the right shape for every
title but this one - 2048 needs the choice to be a property of *which team*,
not of the title alone.

**2026-09-01, later: the 5 native teams are fixed.** `oag_title::TeamVariants`
(`crates/title/src/race.rs`) is the type design this needed: a `RaceDefaults`
carries an optional list of teams that offer more than one selectable
directory, plus how a variant's suffix joins with the team's own id -
measured on two different shapes rather than designed from one, since 2048's
own five (`feisar2048\3`, a numbered *subdirectory*) and HD/Fury's twelve
(`Auricom_c1`, a *suffix* on the same segment) disagree about more than the
label. A VARIANT row now appears on both the RACE and RACE REMIX pages
whenever the picked team is one `team_variants` names, defaulting to the
first variant offered. Entry resolution for all 20 native combinations
(5 teams x 4 ship types) is confirmed by
`crates/game/examples/team_variants_probe.rs`; a full race actually loading
on a combined id neither title's own `DEFAULT_TEAM` reaches - HD's
`Assegai_c1`, 2048's `Auricom2048\1` - is pinned in
`race_remix_ground_truth::a_team_variant_races_on_both_join_shapes`. **Only
through the menu path**: `race::load` itself has no notion of `team_variants`
- `combine_variant` lives in `session/menus.rs` and runs before `race_options.team`
is set, so `--race --team Auricom2048` still fails exactly as before; a
`--team` CLI user must spell the combined id (`Auricom2048\1`) directly. See
`race-remix-backend-lands-menu-wiring-is-next.md`.

**2026-09-01, later still: the 12 HD-derived teams are fixed too.**
`oag_title::GuestRoster` (`crates/title/src/race/variants.rs`) is the sibling
type `TeamVariants` needed for this - a *different* directory question, not
a suffix-join one, since HD_SHIP_DIR serves both the ship model and the
tuning in one tree where the native five split across two. `2048`'s own
`GUEST_TEAM_VARIANTS`/`GUEST_ROSTER` wire it; a full race loads on any of
the twelve, pinned in
`race_remix_ground_truth::a_guest_team_races_standalone_on_2048_without_a_craft_split`.
Race Remix additionally folds these twelve into "Wipeout HD"'s own CRAFT
TITLE entry rather than showing them under 2048 - see
`race-remix-backend-lands-menu-wiring-is-next.md` and
[ADR-0035](../../docs/architecture/adr/0035-a-craft-pick-may-fall-back-to-a-title-that-reships-the-same-roster.md).

## Open

- Section B's vertex declaration is found and its offsets are cross-checked
  (see "What is left" above). `normal`'s type (5), `Uv1`'s (8) and, as of
  2026-09-16, `tangent`'s (also 5, 4 components) are all decoded now -
  confidence 96, 96 and 76 respectively; `tangent`'s is lower because there
  is no Wipeout HD twin to check its content against (HD's own renderer has
  no decoded tangent frame either), so it rests on internal consistency
  (unit length, orthogonality to the already-cracked normal) rather than a
  cross-title oracle - see `docs/formats/2048-rcsmodel.md#tangent-is-cracked-too-at-a-lower-confidence`.
  Section B's *other* contents past this one declaration and
  the material table (see above) - the rest of the serialized object graph
  `RcsModel_Load` reads whole - remain unwalked, including
  the 64-bit hash beside a submesh's buffer pointers, and the three words
  beside the material index at `-0x20`, `-0x10` and `-0x08`. The
  submesh-to-material binding itself is no longer open - see the section
  above - but it was found by measurement, and the function that actually
  consumes it (which would also confirm `tangent`'s and the other two
  fields' SceGxm type codes symbolically) is still unlocated: a 2026-09-16
  pass searched for a vertex-declaration/attribute source tag near
  `System/Render/Model.cpp` and found only a texture-mipmap function at its
  one located caller, not vertex setup.
- ~~**The pixel format almost every 2048 texture is stored in is not
  decoded.**~~ Closed 2026-08-27 - `oag_texture::pvrtc`, confidence 92, see
  the section above. ~~What is *not* closed in that module: `UBC1`, `UBC3`
  and the two uncompressed format bytes~~ - also closed, 2026-09-16:
  `UBC1`/`UBC3` reuse `UBC2`'s twiddled block walk (confidence 88, no HD
  twin exists for either so a picture check carries it - see
  `docs/formats/gxt.md`), and the second uncompressed byte, `U8U8U8`
  (`0x98`, 13 files, confidence 70 - its channel order is unconfirmable,
  every shipped file being pure grayscale). Every format byte the corpus
  carries decodes now, 9,910/9,910. Still open in that module: PVRTC-II's
  local-palette path, implemented from the reference but reached by 0 of
  1,082,941,440 texels measured, so it carries no evidence either way.
- `normal`'s own type nibble is closed: two failed passes (HD's packed word,
  both byte orders; a reordered-fields-plus-derived-z scheme that looked
  confirmed on two hand-picked examples until checked at scale) were followed
  by an exhaustive per-byte search - one signed byte per component,
  `byte/127.0`, x/y/z in file order, 4th byte alignment padding - which
  scores 100% within 18° (mean dot 0.994) over the 1,504-vertex index-exact
  oracle and 33,335,682/33,335,682 unit-length across the full corpus. See
  [2048-rcsmodel.md](../../docs/formats/2048-rcsmodel.md#the-normal-is-cracked-three-signed-bytes-not-a-packed-word).
  Implemented and wired into the renderer. The index-exact correspondence
  itself (`crates/game/examples/vita_rcsmodel_exact.rs`) is a reusable oracle
  for `tangent`'s and the texcoords' type nibbles too, though a texcoord's
  *values* cannot be cross-title-matched the same way (a lightmap atlas is
  baked per platform) - only its encoding's plausibility can be checked.
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
- ~~The native roster's numbered subdirectories (`ag_systems2048/1..4`) and
  their relationship to speed class or ship variant.~~ Recovered further up
  this thread ("RaceDefaults needed a second directory"): fighter, agility,
  speed, prototype, in that order - what remained open was making that
  reachable through the generic catalogue/team-loading path, which the
  2026-09-01 section above is the fuller finding for.
- Whether the patch PSARCs (`data1.psarc`/`data2.psarc`) touch any of `altima`,
  `AG_Systems` or the global handling file at all - not checked, since base
  alone was sufficient for everything above.

## Next Steps

- **Engine trail and flare (read 2026-10-05, [2048-status.md](../../docs/formats/2048-status.md#engine-trail-and-flare---2026-10-05)).** Still open: the v1.04 patch archives are never opened by `oag_2048::open`, so the 24 patched flares are not the ones drawn (low priority: maintainer, 2026-10-05, v1.04 matters mostly for 3D SFX and the rest is VR, not planned); the flame's Vita `.gxp` rim/noise terms are unread; the ribbon, sprite flare and boost reveal are HD's laws, unmeasured on 2048; Omega's 69 flare models are unwired.

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
   ([rcsmodel.md](../../docs/formats/rcsmodel.md)) placed the same fields in a
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
5. ~~Section B's vertex declaration~~ - found 2026-08-27, offsets
   cross-checked, see "What is left" above.
6. ~~`normal`'s encoding~~ - cracked the same session, see "Open" above:
   three signed bytes, `byte/127.0`, x/y/z in file order, 4th byte padding.
   Implemented in `oag_rcs::rcsmodel::psp2::unpack_normal` and wired into
   `oag_mesh::mesh::rcs::psp2::build`; `just play 2048 --race` now draws
   Altima lit off real normals. The index-exact correspondence built to get
   there (`crates/game/examples/vita_rcsmodel_exact.rs`, 1,504 vertices, zero
   ambiguity) is a reusable oracle for the next guess.
7. ~~`Uv1`'s type nibble~~ - done 2026-08-27, see above: two little-endian
   `f16`s, confidence 96 against the same index-exact HD oracle that settled
   the normal. ~~`tangent`'s (also type 5, 4 components) is still open~~ -
   done 2026-09-16, and the same per-byte scheme *does* apply: one signed
   byte per component, all four bytes used (no spare padding byte the way
   `normal` has one), confidence 76 - lower than `normal`'s 96 because there
   is no HD twin to check content against, only internal consistency
   (orthogonality to `normal`, unit length, the fourth byte reading as a
   `+-1` handedness sign on the well-formed subset). See
   `docs/formats/2048-rcsmodel.md#tangent-is-cracked-too-at-a-lower-confidence`.
   `lightmapUV`'s offset is placed (same declaration, same type as `Uv1`)
   but its *content* still cannot be cross-title-checked, a lightmap atlas
   being baked per platform.
8. ~~**The submesh-to-material binding.**~~ Done 2026-08-27 - a `u32` index
   `0x18` bytes before the record, confidence 90, see the section above. The
   lesson generalises and is the reason to read that section before the next
   field hunt: **an index does not resemble the thing it indexes**, so a
   search keyed on a target's identity is structurally blind to one. One
   field in this container is still unplaced (the 64-bit hash beside a
   submesh's buffer pointers) and has so far been searched for by identity;
   `tangent`'s type nibble is no longer in this list - see item 7.
9. ~~**`PVRTII4BPP` decode.**~~ Done 2026-08-27 - `oag_texture::pvrtc`,
   confidence 92, see the section above.
10. **What is worth doing next, now that a race draws textured.** In order of
    what a screenshot would gain: ~~`.envsettings` (the sun direction, colour,
    ambient, fog and bloom blocks all fail to parse for this title, so the
    race lights off a stand-in rig and draws unfogged)~~ - the light rig is
    fixed, 2026-09-04, see below; `track.pvs` (header word 2 is `16777216`,
    not the `16` `docs/formats/hd-pvs.md` describes, so every chunk draws);
    and ~~the HD-derived roster, which one `ship_dir` string still cannot
    address alongside 2048's own five teams~~ - also done, see
    `oag_title::TeamVariants`/`GuestRoster` in the 2026-09-01 sections above
    (this bullet was never updated when that landed).

## 2026-09-04: the light rig is fixed - 2048 authors its own key names, not HD's, and one of its keys is a confirmed-live trap

Picked up from Next Steps item 10 above. `envsettings_light` was reading
every title's `.envsettings`/`.EnvSettings` against HD's own key constants
(`SUN_COLOUR = "Lighting.Sun color"`, `AMBIENT_COLOUR = "Lighting.Constant
ambient color"`), which is why 2048 always reported *"no usable sun
direction, colour and ambient; lighting with the stand-in rig"* even though
every circuit ships a `track.EnvSettings` in the identical syntax
`oag_tables::envsettings::EnvSettings::parse` already reads.

**Confirmed from the executable, not guessed from the file.**
`Environment_RegisterLightingSchema` (`0x810175fc` in
`vita-2048-eu-v104/eboot.elf`, confidence 85, named this session - see
[lighting-schema.md](../../docs/ghidra/functions/vita-2048-eu-v104/lighting-schema.md))
registers every key a `track.EnvSettings` can carry, in the file's own order,
and spells the ambient term `"Lighting.Constant ambient colour"` (British) and
splits HD's single sun colour into `"Lighting.Sun diffuse colour"` +
`"Lighting.Sun specular colour"` - genuinely different keys, not a
reformatting. `oag_tables::envsettings::PSP2_AMBIENT_COLOUR`/
`PSP2_SUN_DIFFUSE_COLOUR` are the new constants; `envsettings_light` and
`environment::staging` pick between HD's and 2048's key sets on a new
`GeometryKind` (replacing the `ps3_geometry: bool` that used to conflate
"has an `.rcsmodel`" with "is it HD's" - both HD and 2048 have one).
`just play 2048 --race` now reports the circuit's real sun/ambient rather
than falling back, and `data/shots/` (not committed, see the session's own
screenshot) shows Altima genuinely lit rather than flat.

**One trap worth carrying forward, not closed.** 2048's file *also* authors a
literal `"Lighting.Sun color"`, spelled exactly like HD's, bound by the
registrar to its own address - not a parser alias. It is deliberately **not**
wired as the diffuse term: five of the 14 circuits sampled carry the exact
value the registrar's own compiled-in default initialises it to, which reads
as an untouched, vestigial field rather than a real one, but a proper
consumer search needs the field traced from an actual call site's return
value forward (this binary accesses this address at a fixed *displacement*
off `Environment_RegisterLightingSchema()`'s own return pointer, not by
absolute address - the same trap `docs/formats/envsettings.md`'s HD notes
already record for a TOC-relative load, on a different binary). Full account,
including which search techniques already came back empty or inapplicable:
[lighting-schema.md](../../docs/ghidra/functions/vita-2048-eu-v104/lighting-schema.md).

**Fog and bloom are untouched, on purpose.** 2048's own fog block is
structurally unrelated to HD's (`Lighting.Fog colour`/`Fog Region Colour
Override %d`, no `Fog.*`-prefixed keys at all), so reading it against HD's
`FOG_COLOUR`/`FOG_DENSITY` would repeat the exact key-mismatch this session
fixed for the light rig. `environment::staging` still gates both readers to
HD's geometry only; modelling 2048's own fog/bloom schema is a fresh, unstarted
piece of work, not a continuation of this one.

## 2026-09-16: `tangent` is cracked (confidence 76, not `normal`'s 96), and its own gap - the vertex format's runtime consumer - is still the same one

Picked up from item 7 above: does `tangent`'s type-5, 4-component field take
the same one-signed-byte-per-component scheme `normal`'s type-5, 3-component
field does? Yes, extended to all four bytes (no spare padding byte the way
`normal` has one) - `x=byte[0] y=byte[1] z=byte[2]` (`i8/127`),
`w=byte[3]`. **Confidence 76, deliberately short of `normal`'s 96**: there
is no Wipeout HD twin to check content against for this field (HD's own
renderer has no decoded tangent frame either), so the evidence is internal
consistency across 6,653,653 vertices rather than a cross-title oracle - the
winning byte assignment is the unique best of the six permutations sharing
its own byte set (mean `|dot(normal)|` 0.219 against 0.39-0.49 for the other
five and 0.407 for a deliberately-wrong control), and restricted to the
58.1% of vertices whose `(x, y, z)` passes a unit-length band, the fourth
byte reads as a `+-1` handedness sign 82.6% of the time and the tangent
sits within 20 degrees of perpendicular to `normal` 86.5% of the time. Full
account, including why the other ~42% reads as genuinely degenerate tangent
data (a real state at UV poles/seams) rather than a second encoding:
[2048-rcsmodel.md](../../docs/formats/2048-rcsmodel.md#tangent-is-cracked-too-at-a-lower-confidence).
Reproducer: `crates/game/examples/vita_rcsmodel_tangent_bytesearch.rs`.

**The Ghidra route was tried and hit the same wall the material-index
section already recorded for a different field.** `search_strings` for a
vertex-declaration/attribute source tag near `System/Render/Model.cpp` (the
plausible runtime mesh class, found among 53 `System/Render/*.cpp` tags)
found one located caller, `FUN_81287fd4` - a texture mip-generation routine,
not vertex setup - and walking back from `sceGxmDraw` (NID `0xBC059AFC`) is
the documented dead end this thread's own material-index section already
hit. No function was named, so `names.tsv` gains no row this session either.
**`RcsModel_Load`'s actual consumer is still the standing gap** for all
three of `normal`, `Uv1` and `tangent` - none of `t5`'s or `t8`'s SceGxm
symbolic names are confirmed, only recognised by behaviour, and confirming
the consumer is what would take any of the three past its current ceiling.

**Wired, and confirmed to draw nothing new.** `oag_rcs::rcsmodel::psp2::unpack_tangent`
decodes it, `psp2::SubMesh::tangents` carries it per vertex on the same
declaration-lookup terms `texcoords` already uses, and
`oag_mesh::mesh::rcs::psp2::Report::decoded_tangents` counts it into the
load report (`just play 2048 --race` now says e.g. "260533 tangent(s)
decoded, unused (no normal-map consumer yet)" for Altima). **Confirmed
byte-identical before and after** (`cmp` on `data/shots/2048_tangent_before.png`
against the post-change render): nothing in this title's mesh path samples a
tangent-space normal map, and the shared `GpuVertex` vertex layout /
`mesh.wgsl` - used by every title's mesh path - was deliberately left
untouched rather than growing an attribute with no consumer.

**The GXT side of this thread's own "What is not decoded" list is also
closed, same session, unrelated to `tangent`**: `UBC1`/`UBC3` (BC1/BC3, 505
and 493 files) decode via the same twiddled block walk `UBC2` already uses,
confidence 88 - no HD twin exists for either so a picture check (a scanned
manual page, a front-end callout) carries it instead. `U8U8U8` (`0x98`, 13
files, confidence 70 - channel order unconfirmable, every shipped file being
pure grayscale) closes the last uncompressed format byte. Every format byte
the corpus carries decodes now, 9,910 of 9,910 `.gxt` files - see
[gxt.md](../../docs/formats/gxt.md).
