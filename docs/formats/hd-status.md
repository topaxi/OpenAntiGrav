# Wipeout HD / Fury: what the Pulse format layer already reads

**This page started as a measurement, and part of it is now code.** It records
what happened when this project's existing format readings were pointed at
*Wipeout HD / Fury*'s disc, per
[ADR-0009](../architecture/adr/0009-multi-game-fanout.md)'s cheap probe - the
same probe [pure-status](pure-status.md) is the write-up of, one title further
along the lineage and one console generation across. Every success is a fourth
validation corpus for a format page; every failure is a finding about where the
parsers overfit to the PSP.

**A circuit draws, as of 2026-08-17:**

```sh
just view data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC \
    --track /data/environments/talons_junction/track.vex
```

The [roadmap](../overview/roadmap.md#a-circuit-draws-as-of-2026-08-17) is where
that is accounted for. Three things it is worth knowing here: the
[`.psarc` reader](psarc.md) is in `oag-formats` now; `oag_formats::ByteOrder`
exists and **neither `vex::nodes` nor `track::parse` takes it as an argument**,
because each file declares its own order in its own magic; and `--mesh` **now
draws**, because the container the geometry left for has since been read - see
[rcsmodel](rcsmodel.md), and [the geometry has left the
file](#the-geometry-has-left-the-file) below for what that section used to say.

Nothing on this page has been verified under an emulator, and no PS3 executable
has been read for any of it. Every score rests on static reading plus exact
agreement with shipped data, which
[caps it at 94](../reverse-engineering/confidence-rubric.md).

The disc is `hdfury-ps3-eu.iso`, serial `BCES-00664`; its SHA-256 is in
[source images](../reverse-engineering/source-images.md). Reading anything off it
needs the image [layer-1 decrypted](ps3-disc.md) first, and then the
[`.psarc` reader](psarc.md).

## The headline

**HD is not a new engine.** It is the PSP asset pipeline, big-endian, with the
render geometry lifted out of `.vex` into a PS3 container of its own.

| Layer | Verdict |
| --- | --- |
| Disc, ISO 9660, serial | Reads unchanged - `oag-unpack info` identifies it today |
| Archive container | **New.** [`.psarc`](psarc.md) replaces the [WAD](wad.md), and stores real paths rather than a name hash |
| `.vex` file header, node tree | Reads **byte-swapped**; version 6 and Pulse's own class IDs |
| `.vex` mesh batches | **Gone from the file.** A `Mesh` node is now a bounds pair and a reference |
| `WO Track` payload | Reads byte-swapped; version `0x106`, the same `0x70` control point |
| Collision soup | Reads byte-swapped, exactly - and `collision::from_vex` does |
| `section` PVS mask | Reads byte-swapped, with **one trap that a naive port walks into** - and `pvs` now avoids it |
| `Speedup Pad` / `Weapon Pad` volumes | Read byte-swapped, and `pads::volumes` does |
| Handling stats | Same schema, plain-text XML - **parses**, after making `headtilt` optional |
| HUD layouts | Same dialect and the same widget model - **all 18 compose**, once includes are followed and offsets composed. [hd-hud](hd-hud.md) |
| `.pob` particle container | Reads byte-swapped, exactly - and `pob` does, **all 88 parsed and their 249 emitters walked** |
| Textures | **New.** [`.gtf`](gtf.md), the PS3's own container - **read**, 7,333 of 7,333, 7,280 of them decoding |
| Sound bank | `.bnk` present, unexamined |
| [Music](#music-plain-mp3-declared-the-way-the-psp-titles-declare-theirs) | **New container, same declaration.** Plain MP3, named by the executable and declared as `PI_Music`; recovered and played |
| [Video](bik.md) | **New, and the one payload that is not byte-swapped.** `.bik` replaces `.PMF`; header read on all 37 files, pictures through the same cache |

**And most of HD's circuits are the PSP's circuits**, in the same world
coordinates - see [the circuits are the PSP's](#the-circuits-are-the-psps).

## Disc layout

`oag-unpack info` identifies the disc with no help at all: platform PS3, serial
`BCES-00664` out of `PS3_DISC.SFB`, ISO 9660 with 2048-byte sectors, 21 files in
5 directories, 2.1 GiB. That works against the *encrypted* image too, because
`PS3_DISC.SFB` lies in a plain region.

Seven `.psarc` archives hold everything: 11,664 entries, 3.83 GiB uncompressed.
Their per-archive split is on the [`.psarc` page](psarc.md#wipeout-hd--furys-archives).
By extension, whole image:

```sh
just psarc info data/images/hdfury-ps3-eu-dec.iso
```

| Count | Size | Extension | What it is |
| ---: | ---: | --- | --- |
| 7,333 | 2,426.6 MiB | `.gtf` | PS3 textures |
| 1,632 | 104.2 MiB | `.rcsmaterial` | Material/shader binding tables |
| 772 | 16.9 MiB | `.xml` | Authored tunables, plain text |
| 742 | 72.3 MiB | `.vex` | The scene format, unchanged in kind |
| 643 | 686.5 MiB | `.rcsmodel` | **All render geometry** |
| 88 | 0.9 MiB | `.pob` | Particle effects |
| 50 | 113.4 MiB | `.bnk` | Sound banks |
| 37 | 101.2 MiB | `.bik` | Bink video |
| 36 | 175.9 MiB | `.mp3` | Music |
| 33 | 24.0 MiB | `.fnt` | Fonts |
| 28 | 176.0 MiB | `.probes` | Lighting probes |
| 28 | 3.2 MiB | `.pvs` | Visibility, moved out of the `.vex` |
| 24 | 0.1 MiB | `.nnt` | See [the AI is not a line follower](#the-ai-is-not-a-racing-line-follower) |

The directory tree is the PSP's: `data/environments`, `data/ships`, `data/xml`,
`data/psys`, `data/weapons`, `data/hud`, `data/plugins`, `data/sound`. One node
name inside a shipped track file still reads
`z:\WipeoutPSP\HD\Data\Environments\VexDump\...`, and another
`Z:/WipeoutHD/Data/Source/Wip/DLC3/Environments/Talons_Junction/resource.ma` -
the exporter is Maya, as on the PSP, and its source tree was called `WipeoutPSP`.

## `.vex` is version 6 with Pulse's class table, byte-swapped

**Confidence 92**, on an exact arithmetic invariant across 40 files - three
invariants per file, in fact, and they are independent of one another.

The file header is [Pulse's](vex.md), read big-endian, with the magic in the
opposite byte order:

```text
+0x00  u32   version, 6 - the same word Pulse writes
+0x04  u32   size of the node tree
+0x08  u32   size of the embedded texture block
+0x0c  char  "XXEV", which is "VEXX" byte-reversed
```

The node header is unchanged too: `class_id`, a `u16` header length, a `u32`
data length, a **`u16`** child count at `+0x0c`, and the name at `+0x10`.

The checks, over every file under `data/environments/` whose name begins with
`track` - 28 circuits plus 12 front-end models, **40 of 40**:

1. `16 + tree_len + texture_len` equals the file length.
2. The node walk lands exactly on the tree end.
3. Immediate child counts sum to node count minus one - the same pre-order tree
   invariant that settled the `u16` reading on Pulse.

The third is the load-bearing one. It cannot come out right by accident, and it
fails immediately if the child count is read as a `u32` or the walk drifts.

```sh
just hd-survey data/images/hdfury-ps3-eu-dec.iso data/images/pulse-psp-usa.chd
```

### The class IDs are Pulse's, and one is not

Every class HD authors resolves against `oag_vex::vex::classes::V6` except
one. Over the 40 files, by count:

| Count | ID | Class | | Count | ID | Class |
| ---: | --- | --- | --- | ---: | --- | --- |
| 28,545 | `0x06e` | Transform | | 40 | `0x0f4` | World |
| 7,489 | `0x125` | Mesh | | 40 | `0x3c1` | Texture |
| 5,174 | `0x3cf` | wopoint | | 32 | `0x3c7` | Quake |
| 3,061 | `0x3c0` | Anim Transform | | 32 | `0x2ee` | LodGroup |
| 1,553 | `0x3e1` | sound | | 30 | `0x3d9` | cloudGroup |
| 722 | `0x132` | PointLight | | 28 | `0x3bc` | Start Position |
| 526 | `0x3c9` | section | | 28 | `0x3b9` | Floor Collision |
| 416 | `0x3bd` | Speedup Pad | | 28 | `0x3ba` | Wall Collision |
| 324 | `0x0f7` | Camera | | 28 | `0x3bb` | WO Track |
| 208 | `0x3be` | Weapon Pad | | 28 | `0x3cd` | Reset Collision |
| 170 | `0x3d8` | cloudCube | | 28 | `0x3ed` | collision_trackwall |
| 89 | `0x3da` | weatherPos | | 17 | `0x12c` | AmbientLight |
| 56 | `0x3ce` | wospot | | 17 | `0x131` | DirectionalLight |

plus `exitglow` `0x3e4` and `Mag Floor Collision` `0x3e6` at 10 each, `Skycube`
`0x3c6` at 9, `fogCube` `0x3d3` at 5, `soundcone` `0x3e9` at 3, and
`Dynamic Shadow Occluder` `0x3c3` and `gridCamera` `0x3dd` at 2.

`0x3ed` is [the barrier along the road](#0x3ed-is-the-barrier-along-the-road),
recovered on 2026-08-17. This page used to call it "unrecovered" and refuse to
name it; what changed is below.

### `0x3ed` is the barrier along the road

**Confidence 85, and the name in the code is still HD's own, not a table
entry.** Every circuit file on the disc authors exactly one node of this class,
under the node name `collision_trackwall` - all **28** of them, which is 16
forward circuits and the 12 `track_reversed.vex` that account for the rest of
the census row above. Four measurements settle what it *is*, taken over all 28
by
[`hd_trackwall_ground_truth.rs`](../../crates/game/tests/hd_trackwall_ground_truth.rs):

1. **Its payload is collision geometry.** All 28 carry the all-ones header word
   every [collision](collision.md) payload carries and parse to a clean end
   through the existing decoder with nothing left over - 2,174 to 4,974
   triangles each.
2. **Its triangles stand on end.** 97 % of them are more than 60 degrees off
   horizontal, median over the 28, worst **64.5 %** on `03_track/track_reversed`
   and 24 of 28 above 89 %. On the same files `Floor Collision` reads 0 - 19 %
   and `Wall Collision` 12 - 89 %. This is the facing statistic
   [collision](collision.md) settled Pure's classes with, and here it is
   calibrated against two classes on the *same file* rather than across discs.
3. **It occupies the road, not the environment.** Talon's Junction's barrier
   spans 1476 x 194 x 1088 against its floor's 1482 x 189 x 1095 - within 1 % on
   every axis - where `Wall Collision` spans 2185 x 511 x 2300. The test asserts
   the barrier stays within a quarter of the floor's extent on all three axes of
   all 28 files, and all 28 pass - the tightest being `12_sol_2` at 220 against
   258 in `y`.
4. **A craft stops at it.** Thrown at one of its triangles at 150 units/s, a
   hull penetrates 5.5 units and comes back - which is the point of the other
   three.

So it decodes as `SurfaceKind::TrackWall` and drives `oag_physics::Surface::Wall`.
The load report went from `4 collision node(s) -> 400 collider(s), 12737
triangle(s)` to `5 -> 530, 16883`, which is the barrier's own 130 meshes and
4,146 triangles exactly.

**Resolved 2026-08-26**: whether *Pulse's* class table names this ID at all was
open, and it does not. [`vex.md`](vex.md#node-types) now records the table read
to its terminator at `0x08ab4be4` - 863 real records, game-class ids `0x3b9`
through `0x3eb` only. `0x3ec`/`0x3ed`/`0x3ee` are the exact three missing
against HD's 866-record table, not a coincidence of count. Pulse authors no
node of this class either way - the survey in
[`collision_ground_truth.rs`](../../crates/vex/tests/collision_ground_truth.rs)
walks every `.vex` on both its discs and finds no sixth class carrying a
collision payload - so listing the ID in `classes::V6` changes nothing Pulse
decodes, and version 6 is a format generation rather than a title.

Two things this does *not* claim. HD's own loader has not been disassembled, so
the surface-type byte it would write is unread and `SurfaceKind::surface_type`
returns `None` for this class rather than guessing at a number. And its friction
is Pulse's `0.05` wall value by assumption, stated as one in
`SurfaceKind::friction`, because the alternative - `None` - would mean
*frictionless*, a barrier a craft grinds along losing no speed at all.

Two counts worth noticing against Pulse's own: HD authors **722 `PointLight`
nodes**, a class a
[Ghidra pass](../ghidra/functions/psp-pulse-eu/lighting.md) found has no
registration site at all on the PSP; and **17 `DirectionalLight`**, the class
that *is* registered there. Neither says anything about what HD does with them.

### The geometry has left the file

This is the one structural change, and it is large. On Pulse a `Mesh` node's
payload **is** its geometry - pre-batched GE draw calls with vertices inline. On
HD it is a small fixed record.

The same circuit, measured both ways:

| | `Mesh` nodes | Mesh payload | File | Geometry beside it |
| --- | ---: | ---: | ---: | ---: |
| Pulse `16_Track` | 602 | 3,487,280 B | 5,107,104 B | - |
| HD `talons_junction` | 126 | 90,128 B | 3,066,400 B | `track.rcsmodel`, 24,742,836 B |

90 KB of payload cannot hold what 24.7 MB of `.rcsmodel` does. What the payload
*does* hold is legible at the front, and one part of it is confirmed rather than
guessed: floats at `+0x10` and `+0x20` are a bounding-box pair, `min <= max`
componentwise on **1,638 of 1,638** `Mesh` nodes across the `DATA00` circuits.
There is a per-node 32-bit word after them that differs between otherwise
identical nodes, which this page called "the obvious candidate for the reference
into `.rcsmodel`" while that file was unread. **It is the reference**: it is the
chunk's own first word, and every one of Assegai's 15 mesh nodes resolves
through it. See [rcsmodel](rcsmodel.md).

So `oag-view --mesh` draws an HD craft and an HD circuit, as of 2026-08-17,
**lit off the file's own vertex normals** since later the same day - 23,888 of
them on Assegai and 531,904 on Talon's Junction. It is still untextured: the
texture coordinate is located but unconfirmed and the `.gtf` textures are
unread. `--track` still draws the ribbon from the `WO Track` spline, which is
authored data that never moved.

**One thing this section's own framing got wrong**, and it cost a render: the
geometry did not merely leave the payload, it left the *node tree*. All 126
`Mesh` nodes of `talons_junction/track.vex` are props; the road is among the 904
of 983 chunks no node references at all, carrying a world-space bias instead.

## `WO Track`: version `0x106`, the same control point

**Confidence 90.** [Pulse's layout](track.md) reads unchanged: a `0x20` header,
the `0x20` reserved block every version from `0x101` claims, then paths at `0x20`
bytes each, junctions at `0x10`, and control points at `0x70`.

**The magic is a byte-order discriminator, and this page used to say otherwise.**
It read "magic `WOtd`" for both games, which invites the conclusion that a `WO
Track` payload cannot say which way round it is and that a parser has to be told
by its container. Measured on the shipped files, `16_Track` opens `64 74 4f 57`
(`dtOW`) and `talons_junction` opens `57 4f 74 64` (`WOtd`) - the same word
`0x574f7464`, written on hosts of opposite endianness, exactly like
`VEXX`/`XXEV`. So `oag_vex::track::parse` sniffs it and reads a Wipeout HD
circuit with no argument and no caller change; `track::byte_order` is the sniff,
and `a_big_endian_payload_parses_to_the_same_spline_as_its_little_endian_twin`
pins it.

Across all 28 circuits:

- **23,874 control points**, and on every file the bytes after the path and
  junction arrays divide by `0x70` exactly and equal the summed path lengths.
- **0 of 71,622 frame vectors** are off unit length by more than `1e-3`. That is
  three vectors per point - tangent, down, lateral - and it is what says the
  field offsets are right rather than merely that the stride is.
- Control-point spacing runs **2.53 to 8.91** units, on every path of every file.
  Pulse's own range on the same check is 2.62 to 29.00, so HD's splines are
  sampled more evenly; nothing else separates them.
- Junction slots are 2-in, 2-out with `0x7fffffff` for null, as on the PSP.

### Two fields nobody reads, in either game

`oag_vex::track::SplinePoint` names fields up to `+0x61`. Two regions outside
it carry data:

| Where | Pulse | HD |
| --- | --- | --- |
| `+0x5c`, an `f32` | authored on all 862 points of `16_Track`, on 30 of 713 of `06_Track` | authored on all 776 of `02_track`, on none of `talons_junction` |
| `+0x62..0x70` | `ffffffff fd7f` then 8 bytes that vary per point | the same constant, then 8 zero bytes on most circuits |

Both are authored **per track rather than per title**, which is what rules out
"HD dropped them". They are recorded here because a reader comparing the two
games will find them and should know they are unread on both sides, not a
difference. Nothing before them shifts: every field the parser does read lines up
across the version bump.

### `racing_line` is zero, and that is not an HD finding

A first pass on this disc concluded that HD authors no racing line, from
`racing_line == 0.00` across all 862 control points of Talon's Junction. **That
reading is withdrawn.** The field is zero on all 862 points of Pulse's own
`16_Track` too, and on `01`, `05`, `06`, `13` and `14`. It is unauthored in both
games; `oag-ai` already drives the centre line when it is zero, so nothing about
this transfers badly.

The correction is kept rather than deleted because the mistake is instructive: a
field that is constant in the file you are holding says nothing until you have
looked at the same field in the file you are comparing against.

## Collision reads exactly

**Confidence 92.** [The chunked triangle soup](collision.md) - `0xffffffff`,
an object count, then three chunks per object in the order 1, 3, 2 with strides
`0x0c`, `0x04` and `0x06` - reads byte-swapped with nothing changed. **94 of 94**
collision nodes across the 28 circuits consume their payload down to its 16-byte
alignment padding, which is the invariant that settled the layout on the PSP.

**`oag_vex::collision::from_vex` reads it byte-swapped as of 2026-08-17**,
taking the order from the containing `.vex`'s magic - a collision payload cannot
declare its own, its header word being the palindrome `0xffffffff`. The counts
in the table below are reproduced exactly by that reader in
`crates/assets/tests/hd_psarc_ground_truth.rs`, which matters because they were
first measured by `scripts/hd-survey.py`: two independent implementations
agreeing on 246/7,588, 122/4,269, 14/357 and 18/627.

The geometry itself is **not** shared with the PSP even where the circuit is.
Talon's Junction against Pulse's `16_Track`:

| | floor | wall | mag floor | reset |
| --- | --- | --- | --- | --- |
| Pulse `16_Track` | 123 objects, 3,835 v | 66, 2,224 v | 7, 177 v | none |
| HD `talons_junction` | 246 objects, 7,588 v | 122, 4,269 v | 14, 357 v | 18, 627 v |

Close to a doubling on every class HD and Pulse share, and HD adds reset volumes
where Pulse's `16_Track` authors none - which is worth knowing next to
[the note in HANDOVER](../../HANDOVER.md) that three Pulse circuits author zero
`Reset` colliders and so have no net at all.

## The PVS mask, and the byte-order trap

**This is the one place a mechanical byte-order pass produces silent garbage**,
so it is worth more space than its size deserves.

[`oag_vex::pvs`](track.md) reads the `section` `0x3c9` payload's 64-bit
visibility mask as two `u32`s, `pvs_mask_lo` at `+0x08` and `pvs_mask_hi` at
`+0x0c`. On a little-endian file that is identical to reading the eight bytes as
one `u64`. **On a big-endian file it is not**, and swapping each word in place -
which is what a `from_le_bytes` to `from_be_bytes` sweep does - leaves the two
halves the wrong way round.

Measured, over 3,536 set bits in 526 `section` nodes on 24 HD circuits, by how
many bits name a section the file does not declare:

| Reading | Dangling bits | Share |
| --- | ---: | ---: |
| One big-endian `u64` at `+0x08` | 784 | 22.2 % |
| Two words swapped in place | 1,965 | **55.6 %** |

And the same script's control column, run over Pulse's own 24 circuits, where
the two readings are by construction the same thing: **223 of 10,597, 2.10 %** -
which independently reproduces the "one bit in fifty" figure
[the PVS ground-truth test](track.md) already records, so the measure is
measuring what it claims.

Under the correct reading, **15 of 24** HD circuits have *no* dangling bit at
all, where the word-swapped reading gives 100 % dangling on those same files. The
22 % overall is not spread out: it comes almost entirely from three circuits -
`01_vineta_k`, `12_sol_2` and `04_chenghou_project` - which are also the three
whose section IDs run all the way to 63, the cap the mask width implies and that
Pulse's `01_Track` hits exactly.

So: **HD authors real PVS data and `[graphics] pvs_culling` has something to
read**, provided the mask is read as one 64-bit quantity rather than as a pair.

**`oag_vex::pvs` reads it that way as of 2026-08-17.** The `pvs_mask_lo` /
`pvs_mask_hi` pair is gone, replaced by one `ByteOrder::u64` at `+0x08` -
identical on a little-endian file by construction, so the Pulse and Pure ground
truth is unmoved and the "one bit in fifty" figure above still holds. Talon's
Junction is one of the 15 clean circuits: **zero** dangling bits under the
correct reading, and over half dangling under the word-swapped one on the same
bytes, asserted against each other in
`crates/assets/tests/hd_psarc_ground_truth.rs` so the check needs no reference
answer.

## Pads and the grid read unchanged

`Speedup Pad` `0x3bd` and `Weapon Pad` `0x3be` carry [the same payload](pads.md)
they do on the PSP: the trigger volume is the mesh's own bounding-box pair at
`+0x10` and `+0x20`. Across the 28 circuits, **416 speedup pads and 208 weapon
pads**, with `min <= max` componentwise on all 624. Talon's Junction's pads
measure `(-4.66, 1.33, -4.66)` to `(4.66, 1.33, 4.66)`.

**Which is why the pads survive the change that stops `--mesh` drawing
anything.** A pad's payload *is* a `Mesh` payload, and the box pair is precisely
the part of a `Mesh` payload that stayed behind when the geometry left for
`.rcsmodel`. `oag_vex::pads::volumes` reads Talon's Junction's **18 speedup
and 9 weapon pads** off the PS3 disc today, every one within a half-width of the
spline horizontally - which is a check on the *transform chain* rather than on
the payload, since a pad's own box is in local space and only the parent chain
puts it on a circuit.

### Talon's Junction's own `Speedup Pad` chunks are a content donor its own file never baked

**Found 2026-08-25, wiring pad illumination.** The trigger volumes above are
one thing; the *drawable* chunk each pad node names at its mesh payload's own
`+0x30` is another, and on this one circuit the two diverge for one class
only. `oag_mesh::mesh::rcs::build_pads` (the PS3 counterpart of
`mesh::build_pads`, added the same day) walks all 18 `Speedup Pad` nodes,
reads a well-formed, non-zero chunk hash off every one, and finds **none of
the 18** in `talons_junction/track.rcsmodel` - `model.mesh(hash)` returns
`None` every time. `Weapon Pad`'s own 9 nodes, same file, same payload
layout, resolve **9 of 9**.

**Not a reader bug - checked against a second circuit.** `12_sol_2`'s own
`track.rcsmodel` resolves all ten of its `Speedup Pad` chunks and all eight
of its `Weapon Pad` ones, through the identical code path. So the class-id
resolution, the payload offset and the hash lookup are all sound; what is
absent is specific to Talon's Junction's own baked chunk set.

**Read as the same "content donor" pattern this page's `--mesh` section
already names**, not a new phenomenon: `mesh/rcs.rs`'s own doc comment
records that 56 of Talon's Junction's ordinary prop nodes name a hash found
only in *other* circuits' models (`tanker1aShape` among them), because the
hash is content-derived and this circuit's own bake simply never included
that donor's copy. The 18 `Speedup Pad` hashes read exactly the same way -
well-formed, structurally in the right place, absent from this one file. No
second circuit's `.rcsmodel` has been checked for a matching hash, so
whether Talon's Junction's speed pad plates are baked *somewhere* on the disc
is open; what is closed is that they are not in this circuit's own file.

**Consequence for the render**: `oag_raceplay::load` reports "0 of 18 mesh
node(s) drawn... 18 addressed no chunk" for Talon's Junction's speed pads and
leaves `Loaded::pad_model` at `None` - an honest absence rather than an
invented plate, the same choice `CLAUDE.md` names for every other unrecovered
surface. `Weapon Pad` draws fully on the same circuit (9 of 9, 4527
triangles) and both classes draw fully on `12_sol_2`, so pad illumination
(`oag_render::speedup_pad`, `oag_render::weapon_pad`) is exercised end to end
by `crates/game/tests/hd_pad_illumination_ground_truth.rs` against
`12_sol_2` rather than the default track for the speed pad half.

`Start Position` `0x3bc` is one 64-byte matrix per circuit, 28 of 28, with the
translation in row 3 and `w = 1.0` - the row-major layout every `.vex` uses.
Whether HD lays its grid out from that node the way
[Pulse's `Race_SpawnGrid`](../ghidra/functions/psp-pulse-usa/grid.md) does is
unread.

## The circuits are the PSP's

**Confidence 88**, and this is the most consequential thing on the page.

Ten of HD's sixteen environments have the *same spline, in the same world
coordinates*, as a circuit on Pulse's or Pure's UMD. Matched by mean distance
from each HD control point to the nearest control point of the candidate, which
is independent of how either side resampled the curve:

| HD environment | Is | Mean | Max | Nearest other candidate |
| --- | --- | ---: | ---: | ---: |
| `talons_junction` | Pulse `16_Track` | 0.67 | 5.5 | 1.4 |
| `amphiseum` | Pulse `09_Track` | 0.62 | 14.4 | 25.1 |
| `03_track` | Pulse `03_Track` | 0.64 | 8.5 | 2.3 |
| `02_track` | Pulse `02_Track` | 0.97 | 21.6 | 155.2 |
| `tech_de_ra` | Pulse `04_Track` | 1.46 | 8.6 | 2.2 |
| `01_vineta_k` | Pure `01_Vineta_K` | 2.06 | 20.2 | far |
| `modesto_heights` | Pure `03_Modesto_Heights` | 2.17 | 11.5 | 124.6 |
| `12_sol_2` | Pure `12_Sol_2` | 2.24 | 7.6 | 177.9 |
| `10_sebenco_climb` | Pure `10_Sebenco_Climb` | 2.42 | 7.4 | 106.1 |
| `04_chenghou_project` | Pure `04_Chenghou_Project` | 4.08 | 60.2 | far |

The separation is what makes it a match rather than a coincidence: a wrong
candidate scores in the hundreds. Where the runner-up is close it is the same
circuit's *other direction*, which is the answer being right twice.

Point for point on the identical-count pairs, Talon's Junction's first control
point is `(55.52, -53.63, -187.66)` on Pulse and `(55.52, -53.63, -186.80)` on
HD; its 288th is `(-333.98, -67.43, 51.82)` against `(-333.97, -67.41, 51.85)`.
Half-widths are retuned by about a unit and section IDs are renumbered wholesale
(Pulse 11..26 where HD uses 0..18), so the circuit was re-exported rather than
copied - but not re-laid-out.

**Six are unmatched, and only four of those are real negatives.** The four Zone
environments match nothing, as expected - HD's Zone tracks are its own. But
`05_ubermall` and `15_anulpha_pass` were **never in the candidate set**: the
candidate list is Pulse's 24 track files plus the seven Pure environments whose
entry name this project has recovered (`01_Vineta_K`, `03_Modesto_Heights`,
`04_Chenghou_Project`, `07_Blue_Ridge`, `08_Sinucit`, `10_Sebenco_Climb`,
`12_Sol_2`). Pure stores names as a [hash like every WAD does](wad.md#the-name-hash),
so a name that has not been mined cannot be opened. That is an absent candidate,
not a failed comparison, and `--match` prints the shortfall rather than hiding
it. Both are named after Pure circuits, so the expectation is that they match
once the names exist.

**Why this matters more than it looks.** The expensive part of a second title is
normally that nothing can be checked against anything. Here the project's own
reference scenario runs on `16_Track`, and HD ships that circuit's geometry. A
capture from Pulse and a run on HD's spline are comparable directly, on the same
coordinates, with no new harness.

## Authored data that is plain text

The XML is not [name-shortened](fexml.md) the way Pulse's is - it is ordinary
UTF-8 with a declaration and CRLF line endings, so it needs no dictionary.

**`ships/<team>/handlingstats.xml`** is the same schema
[`oag_tables::handling`](handling-stats.md) parses, element for element -
`easyshield` and `weight_distribution` included, both of which Pulse authors and
the parser already reads:
`<Misc height length shield easyshield width weight_distribution>`,
`<ExternalCameraFar>` and `<ExternalCameraClose>` with the identical seven attributes (the values differ from Pulse's, see
[handling-stats.md](handling-stats.md#the-two-externalcamera-blocks-across-titles)), `<AirbrakeGraphics>`,
and per-class `<Engine accelcap amount falloff gain turbo>`, `<Brakes>`,
`<Turning>`, `<Airbrake>`, `<Antigrav>`, `<Physical>` blocks for
VENOM/FLASH/RAPIER/PHANTOM.

**Airbrake flaps swing** (2026-10-05, `airbrake-flaps`). HD's `Ship.vex` authors the `Airbrake` tree (`Airbrake_L` hinge, an `Anim Transform` `0x3c0` with one static key, then `Airbrake_Left`, then the `Mesh`), the `.rcsmodel` supplies the triangles, and `mesh::rcs::build` now records the flap's vertex span (`Flap::collect`, shared with the PSP builder). The vertices are in the hinge's own space (the hinge is the anchor), so `Flap::hinge` is the identity.
The swing is the same `Flap::deflect` Pulse uses (`hinge * Rx(angle) * hinge^-1`,
about the hinge frame's local X), scaled by the title's own `<AirbrakeGraphics>`
`amount` and rates through `RaceView::airbrake_flaps`. **Which way it turns is
checked, not read:** the title's own `Airbrake` handler is unread, so
`hd_airbrake_flaps_ground_truth` asserts the physical claim Pulse's recovered axis makes (a positive
deflection raises the flap and flares it outward, both sides) on all twelve teams (Auricom's flap swings sideways like a door, `y` 0.000). Only
the player's craft swings, as on Pulse; a rival's flaps stay stowed.
Frames: `data/scratch/airbrake-flaps/` (`hd_*.png, strip_hd.png`).

**Tried, 2026-08-17: the parser accepts them after one change**, and the change
is one attribute. `handling::from_blob`'s existing dispatch already sends HD
down the right branch - the file begins `<?xml`, so it is read as plain text
rather than expanded against a [`<code>` dictionary](fexml.md) that is not
there, and its element names are the unshortened ones that expansion would have
produced. The extra blocks HD adds, `<FE>` and `<Tilt>`, are ignored like any
unknown element.

What did not parse: **`<InternalCamera>` has no `headtilt`** on HD's team files.
It is now `Option<f32>`, the same treatment `easyshield`, `sideshift`,
`weight_distribution` and `<pitch>` already had, and it costs nothing because
`oag_render::camera::internal` treats an absent `headtilt` as no tilt (it rolls
Pulse's cockpit view by `lean * headtilt` since 2026-10-02). The
absence is a fact about a generation of the schema, not a defect: **HD's own
mode ships still carry the attribute.**

Sweeping every ship on the image: **twelve racing teams** - AG Systems,
Assegai, Auricom, EGX, Feisar, Goteki, Harimau, Icaras, Mantis, Piranha, Qirex,
Triakis - each with Pulse's four rungs, plus a **`Test`** ship the retail disc
still carries.

### The mode ships author no speed class at all

A schema shape neither Pulse nor Pure has, and the reason the sweep above says
"team files". `/data/ships/detonator/handlingstats.xml` is 1,398 bytes against a
team file's ~3,500, names itself `team="ZoneMode"`, and hangs `<Engine>`,
`<Brakes>`, `<Turning>`, `<Airbrake>`, `<Antigrav>` and `<Physical>` **directly
off `<Stats>`** where a team file nests them inside four `<Class>` rungs. One
implicit speed class, for modes that have no speed selection.
`/data/ships/zone/` and `/data/ships/zone battle/` are the same shape.

It parses, and `Stats::classes` comes back empty, which is honest. **The trap:**
`oag_gameplay::handling_for` looks a rung up and `.expect()`s it, so handing it
one of these files panics. Nothing does today, and
`the_hd_mode_ships_author_no_speed_class_at_all` is what will fail first if an
HD boot path is written that does.

**`xml/handlingstats.xml`** is the global tunables file, a `<Global>` document
rather than a `<Stats>` one, so `handling::parse_global` reads it and
`from_blob` correctly refuses it. It authors **five `<GlobalClass>` rungs with
`VECTOR` first**, exactly as both Pulse pressings do - which matters because
[handling-stats](handling-stats.md) records that skipping `VECTOR` is only safe
while it is authored first. A third disc now corroborates a rule that had two.

**`environments/<track>/stats.xml`** carries target race and lap times per speed
class, Zone and Elimination targets, a circuit length and location string, and a
per-difficulty `SkillScaleValue` table.

**`plugins/grids/grid_*.xml`** is the campaign, cell by cell: each `PI_Cell`
names a track, a mode, a class, `AICount`, `Weapons`, `damage`, laps and medal
targets for three difficulties. **Measured in full**: split across four
archives that disagree on schema and on which eight grids they carry,
`oag_tables::race_campaign` reads all of it additively, and one grid's own
`<Values>` tag is broken on the disc - see
[`race-campaign.md`'s HD section](race-campaign.md#wipeout-hd-and-fury-the-same-schema-extended-split-across-four-archives).
`plugins/frontend/gui/*.xml` is the front end's own screen set.

**`xml/weaponstats_race.xml`** is [the weapon table](weapon-stats.md) under a
lowercase name, alongside `_detonator` and `_elimination` variants for modes
Pulse does not have.

**`ships/<team>/enginelightdata.xml`** is HD's own, two element-text numbers
and nothing else - `<Distance>` and `<Radius>`, spelled as C float literals
(`0.4f`, `1f`, `0.3`, `-0.4f` all occur) - and it is the one XML on the disc
`oag_tables::fexml` cannot carry, because that reader keeps attributes and
discards text. `oag_tables::enginelight` reads it on its own. **37 files, not
the 36 renderer.md's load-site entry counted**: the twelve teams and `zone`
across `DATA02`/`DATA03`, and all twenty-four `_c1`/`_n1` variants in
`DATA06`; `detonator`, `zone battle` and `test` ship none. `Distance` spans
`-0.4`..`1.5`, `Radius` `0.7`..`2.0`, asserted on the disc by
`crates/tables/tests/enginelight_ground_truth.rs`. What consumes it is
`EngineFlare_SubmitSpuLight` - one SPU vertex light per craft, `Distance`
behind the flare node along its Z axis, range `Radius`
([renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md), "The
captured buffer's producer is found", confidence 85),
and `oag_raceplay::engine_light` builds that record per frame from the
file, the locator and the boost blend - see the "SPU vertex lights" entry in
[`docs/rendering/README.md`](../rendering/README.md).

## `.pob` reads byte-swapped, and the parser reads it

**Confidence 93.** The [container](pob.md) is byte-swapped: the magic reads
`PSYS` where Pulse writes `SYSP`. `oag_pob` takes a byte order and
reads all 88 with no offset changed, checked against the disc by
`crates/pob/tests/pob_ground_truth.rs`. Over all **88** effects:

- magic on 88 of 88;
- the two constant words - `1` at `+0x0a` and `1` at `+0x0c` - on 88 of 88;
- a NUL-terminated ASCII name immediately after the slot table on 88 of 88,
  **predicting the file's own name on 86** of them, case aside
  (`WO_DAMAGE_ELECTRIC` for `wo_damage_electric.pob`). That is the boundary
  Pulse's page had to work to pin down, holding at a third set of slot counts.

The two that do not predict their filename are authoring, not parsing:
`stesparkstest.pob` carries `WO_SHIP_COLL_SPARK_DAMAGE`, and
`wo_ship_explosion_lightshafts.pob` carries `WO_SHIP_EXPLOSION` - a test copy
and a rename. A third, `wo_dustmotes.pob`, matches only case-insensitively
(`WO_DustMotes`), which is why the check is spelt that way.

Beyond the container: **636 of 636** real slots resolve through the same
two-hop pointer fixup to an in-bounds target, 526 of them onto developer
strings; and the emitter tree walks **249 records**, nine deep at its deepest,
with no record revisited. Every predicate the Pulse corpora satisfy - positive
schedules, in-range channel modes, a draw class the blend table has - holds on
records read the other way round. Two things HD *authors* differently are on
[pob](pob.md#two-things-hd-authors-that-pulse-does-not): a fourth
`blend_class`, and `LOOPING` being per-emitter rather than per-effect.

**Retracted - 2026-08-18: the length check never failed.** This section
previously read "with one field that no longer agrees" and reported `+0x04`
falling short of the file length by 48 to 208 bytes on all 88, with a
hypothesis about an appended string block. That was `scripts/hd-survey.py`
comparing the field to the *file* length. `+0x04` counts a payload that starts
after the 32-byte name, so it is short by `base + 16` on **every** platform -
which is both why the gap was always a multiple of 16 and why the same check
would have failed on all 76 Pulse files. The script is fixed and now reports
88 of 88; the identity is asserted per file on all three discs.

## A hull's locators are in a file of their own - 2026-08-18

**Confidence 90**, measured with `oag_vex::vex::class_world_transforms` on
the disc and asserted in
`crates/game/tests/livery_ground_truth.rs::an_hd_hull_takes_its_locators_from_the_file_beside_it`.

Pulse and the PS2 port put every locator node in the hull's own `.vex`. HD
splits a craft across a directory - `ship.vex`, `ship_lod.vex`,
`ship_deathshell.vex`, `engineflare.vex`, `locators.vex` and more - and the
locator nodes live in `locators.vex`. On `data/ships/detonator`:

| file | nodes | `Engine Flare` | `Ship Collision Fx` |
| --- | --- | --- | --- |
| `ship.vex` | 88 | 0 | 0 |
| `locators.vex` | 25 | **1** | **10** |
| `engineflare.vex` | 18 | 0 | 0 |

The class table is unchanged - all three files report version 6 and the same
class ids (`959` and `976`) that [`.vex`](#vex-is-version-6-with-pulses-class-table-byte-swapped)
already records. Only where the nodes are has moved.

**What this cost.** `WO_SHIP_ENGINEFLARE` parses, both its emitters are
`LOOPING`, and the game attaches one instance per craft - but only where the
hull yields a nozzle, and HD's yielded none, so the flare never drew. On screen
a missing asset and a missing anchor are the same picture. Ten is also the
number `Ship_DispatchCollisionFx` picks the nearest of, so HD authors the full
set the recovered Pulse trigger expects.

## The roster is declared, and under HD's own plugin name - 2026-08-18

**Confidence 90**, read off the disc and asserted in
`crates/game/tests/hd_livery_ground_truth.rs` and
`crates/game/tests/hd_boot_ground_truth.rs::the_menus_offer_the_twelve_teams_this_disc_declares`.

HD declares its teams, circuits and soundtrack in one plugin definition, exactly
the way the PSP titles do and with the same schema - one `PI_Team` node per team,
one `PI_Track` per circuit, one `PI_Music` per track, under the same attribute
names. What differs is only where it is: HD **names** the plugin where Pulse and
Pure **number** it.

| | Pulse / Pure | HD / Fury |
| --- | --- | --- |
| definition | `Data\Plugins\PI001\Definition.xml` | `Data\Plugins\Frontend\Definition.xml` |
| form | shortened, `<code>` dictionary | plain `<?xml` |
| teams | 8 | **12** |

That is now [`oag_title::Title::plugin_definition`], on the same footing as the
front-end root and the language plugins: a constant in `oag-pulse` that every
title reached for and that is wrong for one of three.

### It ships five times, and the copies disagree

The `ArchiveCandidates::extra` overlap, biting a file something actually reads:

| Archive | Bytes | `PI_Team` | `PI_Track` | `PI_Music` |
| --- | ---: | ---: | ---: | ---: |
| `DATA00` | 36,996 | 12 | 28 | 15 |
| `DATA02` | 11,133 | 8 | 8 | 9 |
| `DATA03` | 17,667 | 12 | 16 | 9 |
| `DATA05` | 19,355 | 12 | 16 | 9 |
| `DATA06` | 32,764 | 12 | 16 | 9 |

`oag_assets::Archives` serves `DATA00`'s, which is the fullest - and the one
whose fifteen `PI_Music` nodes the [music section](#music-plain-mp3-declared-the-way-the-psp-titles-declare-theirs)
already reports every expansion of as resolving. **That the precedence lands on
the fullest copy is a fact about that ordering, not a measurement of what a PS3
loads**; `DATA02`'s eight-team copy is presumably the base game's, from before
Fury added four. Same unresolved question as `skin.xml`'s six copies.

### What it cost: two bugs stacked, and one picture

A race on this disc fielded eight identical craft in the player's livery. Both
causes produce that picture and neither raised an error:

1. **`race::load` and `boot::definitions` asked for Pulse's path.** The read
   missed, the roster came back empty, and `livery::teams_for_slots` gave every
   slot the player's team - which is exactly what a working grid looks like on a
   source that genuinely declares one team.
2. **With the path fixed, `race::load` still called `fexml::expand`.** That
   refuses a file carrying no `<code>` dictionary, and HD's definition is plain
   XML, so the roster came back empty *again*, now as a swallowed `NoDictionary`.
   `fexml::text` decides from the blob - the trap that function's own docs
   already record for the PS2 in-race HUD.

The front end had the same first cause with a different ending: `load_teams`
used to fall back to `oag_tables::handling::TEAMS` (moved to
`oag_pulse::race::TEAMS`, 2026-09-01) when nothing was declared, so HD's
menus offered **eight** teams off a list this project held rather than twelve
off the disc. The fallback is gone, not moved - see
`roster_declared_ground_truth.rs`.

Both report the reason now rather than an empty count.

### The grid, off the disc

Eight slots, eight teams, eight different `.rcsmodel` hulls:

| slot | team | triangles |
| ---: | --- | ---: |
| 0 | Assegai | 19,914 |
| 1 | Feisar | 14,729 |
| 2 | Qirex | 27,883 |
| 3 | Piranha | 3,937 |
| 4 | AG_Systems | 22,666 |
| 5 | Triakis | 16,393 |
| 6 | Goteki | 20,204 |
| 7 | EGX | 22,163 |

Which team flies which slot is **this project's** and not the original's - the
same statement `oag_livery` carries for the PSP grid. All twelve declared
ids resolve to a `ship.vex`, a `ship.rcsmodel`, a `locators.vex` and a
`handlingstats.xml`; the ids are capitalised in the XML and lowercase in the
manifest, and the PSARC lookup's case folding is what joins them.

Every one of the eight resolves every mesh node and every stray, and none
reports an undecidable vertex stride - so a hull's count is the hull, not what
survived the decode. Piranha's 3,937 against a field of 14,000 to 27,000 is
therefore an authored difference and not a decode miss; it is a question for
[rcsmodel](rcsmodel.md) or for the artist rather than for the roster.

### `Unlock` is not read, on any title

Most of HD's `PI_Team` nodes carry `<Unlock purchase="1">` and most of its
`PI_Track` nodes carry `<Unlock grid="gridN">`; 69 `Unlock` elements in all.
`oag_raceplay::catalogue` reads none of them on HD (Pulse's `Grid` and loyalty rows are
read since 2026-09-29, `race-setup.md`), so everything HD declares is offered. That is a defensible answer for a build with no
progression and it is **not a measurement** of what the original gates - what
`purchase`, `grid`, `loyalty` and `MedalCount` select has not been read.

## What is genuinely new

- **`.rcsmodel`** - 643 files, 686.5 MiB, all render geometry. **Positions,
  triangles, vertex normals and the material table read**, see
  [rcsmodel](rcsmodel.md); the vertex
  stride is in no field of the file, and of the rest of a vertex a tangent
  (stride 22 only) and a texture coordinate are located but not decoded. A
  chunk's `+0x20` indexes the material table; the low two bits of a material's
  state word say whether the surface is see-through, and `+0x58` names the
  `.gtf` it paints with. Together those are a textured, blended HD circuit.
- **`.rcsmaterial`** - 1,632 files, wall-to-wall 32-bit hashes. A track authors
  around fifty by name (`track_surface`, `track_wall`, `glass_reflect`,
  `emissive_bloom`), and a second identical set under `materials_reversed/`.
- **`.gtf`** - 7,333 files, the PS3's own texture container and publicly
  documented. One sample header reads version `0x0105`, one texture, payload at
  `+0x80` behind an RSX texture descriptor. **Read**, and it is what put pixels
  on an HD craft and an HD circuit: a material names its `.gtf` at `+0x58`, and
  the alpha in that texture is where every see-through surface's coverage comes
  from.
- **`.pvs`, `.pvspatch`, `.probes`** - one of each per circuit direction, 28
  apiece. `talons_junction/track.pvs` is 114 KiB against 19 `section` nodes, so
  it is a finer structure than the mask in the `.vex`, not the same table moved.
- **`.xfx`** - no longer unknown: the per-team engine crossfade table, read by
  `oag_formats::xfx` on all 13 files and played as HD's engine note since
  2026-10-05 (`crates/sound/src/sfx/xfade.rs`); see [hd-xfx](hd-xfx.md).
- **`.stencilvolume`, `.svml`, `.points2`, `.effectsettings`,
  `.envsettings`** - the last of these is plain text, a key/value list starting
  `"Lighting.Constant ambient color"=0.403922 0.392157 0.509804`.
- **`.bik`** - the video, and no longer new to this build either: RAD's Bink 1
  container, read on all 37 files and transcoded through the same cache the PSP
  and PS2 movies use. See [bik](bik.md), and note the one thing on it that
  contradicts this page's headline - a `.bik` is **little-endian**, because the
  container is the PC authoring tool's rather than the console's.
- **`.mp3`** - the music, and the one item on this list that is no longer new to
  this build. See below.

## Music: plain MP3, declared the way the PSP titles declare theirs

**Recovered and played, 2026-08-17.** HD's music is the least exotic thing on
the disc: 36 plain MPEG-1 Layer III files under `/data/music/`, 48 kHz stereo,
no console container around them at all. The PSP titles wrap ATRAC3+ in RIFF and
the PS2 stores raw PCM; HD ships something any desktop player opens.

### Both halves of the path are in the executable

`strings` on the decrypted `EBOOT.elf` (see
[source-images.md](../reverse-engineering/source-images.md#hdfury-ps3-euiso---wipeout-hd--fury-ps3))
puts the templates a few bytes before `MusicManager.cpp`:

```text
793858  %s\%s_stereo%s
793868  music
793870  .mp3
793878  %s\%s_surround%s
793890  Data\Music\FEMusic\frontend%d_stereo_fury.mp3
7938c0  Data\Music\FEMusic\frontend%d_surr_fury.mp3
7938f0  Data\Music\FEMusic\frontend%d_stereo.mp3
793920  Data\Music\FEMusic\frontend%d_surround.mp3
```

A soundtrack track is a `PI_Music` location joined with `music`, `_stereo` and
`.mp3`. The **fifteen** `PI_Music` nodes in
`/data/plugins/frontend/definition.xml` all resolve - the same schema and the
same reader (`oag_raceplay::catalogue::music`) the PSP titles use, under a plugin
named rather than numbered.

### Two front-end axes, one of them unread

Four front-end candidates exist and only the channel-count axis is understood.
`frontend1_stereo.mp3` is 4,097,664 bytes and `frontend1_stereo_fury.mp3` is
1,057,536 - a 4x gap, so base and Fury are **different pieces of music**, not
two encodes of one. What the original selects on has not been read, so
`oag_hd::names::FRONT_END_MUSIC` picks the base stereo cut and
`FRONT_END_MUSIC_VARIANTS` records all four with the axis named. `FEship.mp3`
is a second front-end track, a literal rather than a template, and is recorded
and unwired. Its builder is now located -
[`Music_BuildFeshipTrackPath`](../ghidra/functions/ps3-hdfury-eu/sound.md#music_buildfeshiptrackpath-the-front-ends-second-track),
confidence 78 - but nothing calls it by a direct branch anywhere in the
binary, so what actually triggers it is still unread.

### What it cost, and what it found

Two things, neither of them about MP3:

- **A [PSARC](psarc.md) bug that dropped exactly one file.** `Exceeder`'s track
  is `DATA01.PSARC` entry 16 and its block 574 is a raw block beginning `0x78`,
  which the reader tried to inflate. Fourteen of fifteen tracks listed and the
  fifteenth vanished silently. Fixed and pinned; see
  [the block-layout section](psarc.md#a-block-size-of-zero-means-stored).
- **`symphonia` in the build, and `ffmpeg` out of this path.** ATRAC3+ has no
  Rust decoder and stays out of process per
  [ADR-0019](../architecture/adr/0019-atrac3plus-out-of-process.md); MP3 has
  several, so HD's music plays with no `ffmpeg` installed at all.

### Confidence

**88.** The templates and the declaration are each read off the disc and every
expansion resolves, but which of the four front-end cuts the *original*
plays, and which track it starts on, are still unobserved - the 2026-09-05
RPCS3 boot capture ([ADR-0025](../architecture/adr/0025-a-boot-chain-carries-its-provenance.md),
[hd-frontend.md](hd-frontend.md#this-front-end-is-wired-and-its-chain-has-now-been-watched))
watched the screen order, not the audio output, so it settled nothing about
which cut plays. What *is* now observed is our own boot: a headless
`just play hd --dump-audio` run picks `frontend1_stereo.mp3` and plays it
looping, confirmed 2026-09-08. `crates/game/tests/hd_music_ground_truth.rs`
asserts every claim above against the disc.

### Reproducing it

```sh
just psarc cat data/images/hdfury-ps3-eu-dec.iso \
    /data/plugins/frontend/definition.xml | grep -A3 PI_Music
just play hd --race --hold cross --ticks 600
cargo nextest run -p oag-game --test hd_music_ground_truth --run-ignored all
```

## The AI is not a racing-line follower

`data/netconfigs/` holds 24 `nnet_<team>_<class>.nnt` files and 24
`controlprm_<team>_<class>.txt` beside them - twelve teams by two speed classes
(`flash` and `venom`), the same twelve teams `data/ships/` carries. The naming
says HD's opponents are trained networks with a per-team, per-class parameter
file, which is a different architecture from
[the XML controller Pulse ships](../gameplay/ai.md) and from `oag-ai`'s own
lookahead-plus-cross-track follower.

Nothing has been read out of a `.nnt`. What this changes is scope, not code:
porting *HD's* opponents is a problem in its own right, where standing this
project's own driver in front of HD's geometry is not.

## What a real HD asset milestone would cost

Estimated from the [Pure fan-out](../architecture/pure-boot.md), which `git log`
puts at four days elapsed from the title-axis split to a Pure race, plus the two
things Pure did not need.

**The first two items are done, and the estimate for them was about right in
total and wrong in shape.** A `.psarc` reader in `oag-formats` plus an
`oag_assets::psarc::Archive` beside the WAD one was estimated at 1-2 days and
was closer to half of one - the layout is 60 lines and the only judgement call
was taking `miniz_oxide` for inflate rather than hand-rolling deflate.

The byte order was estimated at 2-4 days, on a count of 52 `from_le_bytes`
sites, and cost an afternoon: **the count was the wrong measure**. What mattered
was that the two files a ribbon needs each carry a magic that *is* the
discriminator, so `vex::byte_order` and `track::byte_order` sniff it and **no
call site changed at all**. Only `vex::transform` and `track::start_position`
take an order, because a bare matrix payload has no magic. The 52 sites are
still there, in the mesh, batch, vertex and embedded-texture decoders - and they
are correctly still little-endian, because a PS3 `.vex` has no geometry in it to
reach them.

What is left of the fortnight: an `oag-hd` title crate (2-3 days) and the boot
path, for a *driveable* circuit on Pulse's physics under a named stand-in, the
way [Pure races today](pure-status.md). Collision, both pad classes, the grid
node and the PVS mask are all measured to read byte-swapped already, so that
work is composition rather than format recovery.

**Something that looks like Wipeout HD - months, and mostly one unknown.**
`.gtf` is days. `.rcsmodel` is the variance: 686 MiB in an RSX vertex format
nobody here has read, whose closest calibration is the PS2 VIF-packet decode.
`.rcsmaterial` needs hash-to-name recovery. The lightmaps and `.probes` need a
lighting path the renderer does not have.

**HD's own simulation - not estimable.**
[ADR-0009](../architecture/adr/0009-multi-game-fanout.md) item 2 gates
second-title simulation work behind M4's exit criterion, which is open; and the
[verification harness](../reverse-engineering/verification-protocol.md) is
PPSSPP-shaped - a websocket debugger, a breakpoint on `Ship_UpdateCraft`, a
per-tick offset table - with no RPCS3 equivalent. That is a milestone, not a
task.

## Traps

- **The byte-order pass is wide, not hard, and its failures are silent.** The
  PVS mask above is the worked example: a mechanically correct per-field swap
  produces a mask that is 100 % wrong on 15 of 24 circuits and raises no error.
- **`0x3ed` earns its name from its geometry, not from its node name.** It is
  in `classes::V6` now, and what put it there is four measurements over 16
  circuits - see [above](#0x3ed-is-the-barrier-along-the-road). Naming it on the
  node name alone would have been exactly the guess-dressed-as-a-name the
  [rubric](../reverse-engineering/confidence-rubric.md) forbids, and the trap is
  still live for the *other* half: whether Pulse's table names this ID is
  unestablished, and the constant's name is HD's spelling of the node.
- **`.psarc` paths are lowercase, the digest is over the uppercase spelling.**
  See [psarc](psarc.md#the-check-the-confidence-rests-on).
- **A `.psarc` inside an encrypted image reads as noise.** Decrypt first;
  `scripts/psarc.py` reports the bad magic rather than guessing.
- **A material slot is not a texture, and treating it as one costs gigabytes.**
  Talon's Junction has **442 material slots over 175 distinct textures**, one
  lightmap atlas among them named by 275 slots at once. The slots are positional
  - a chunk names its material by ordinal - so filling them by value retained
  **1,858 MiB where 277 MiB of texels had been decoded**, and handed the GPU 884
  uploads of those 175 pictures. An HD race peaked at **2,577 MiB** on that
  alone, against 233 MiB for the same race on the PSP disc. Sharing one `Arc`
  per texture and keying the upload on its identity took it to 773 MiB with a
  **bit-identical capture**, and binding the disc's own DXT blocks (see
  [gtf](gtf.md#the-mip-chain-is-the-discs-and-a-box-filter-is-not-it)) to 530.
  The shape is general: any per-slot table over an HD circuit wants the same
  treatment.

## Reproducing this

```sh
python3 scripts/ps3iso.py decrypt data/images/hdfury-ps3-eu.iso <key> \
    data/images/hdfury-ps3-eu-dec.iso     # once; the key is never committed
just psarc info   data/images/hdfury-ps3-eu-dec.iso
just psarc verify data/images/hdfury-ps3-eu-dec.iso
just hd-survey    data/images/hdfury-ps3-eu-dec.iso data/images/pulse-psp-usa.chd
python3 scripts/hd-survey.py --match data/images/hdfury-ps3-eu-dec.iso \
    data/images/pulse-psp-usa.chd data/images/pure-psp-eu.chd
```

`hd-survey` prints every count on this page, and its second argument is what
produces the Pulse control column - the thing that says a check is measuring HD
rather than measuring itself. `--match` is the circuit table, and it needs all
three discs; it prints which Pure environment names it could not resolve rather
than quietly shrinking its candidate set.

## See also

- [PSARC](psarc.md) and [PS3 disc encryption](ps3-disc.md) - getting at the bytes
- [Pure status](pure-status.md) - the same probe, one title earlier
- [`.vex`](vex.md), [track data](track.md), [collision](collision.md),
  [pads](pads.md), [`.pob`](pob.md) - the pages this page is a fourth corpus for
- [Wipeout HD / Fury functions](../ghidra/functions/ps3-hdfury-eu/README.md) -
  the executable, which is a separate and much smaller effort so far
- [Roadmap M8](../overview/roadmap.md#m8---beyond-pulse) - where this sits
