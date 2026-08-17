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
because each file declares its own order in its own magic; and `--mesh` still
draws nothing, for the reason [the geometry has left the
file](#the-geometry-has-left-the-file) gives.

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
| Collision soup | Reads byte-swapped, exactly |
| `section` PVS mask | Reads byte-swapped, with **one trap that a naive port walks into** |
| `Speedup Pad` / `Weapon Pad` volumes | Read byte-swapped |
| Handling stats | Same schema, plain-text XML |
| `.pob` particle container | Reads byte-swapped; **one declared length no longer agrees** |
| Textures | **New.** `.gtf`, the PS3's own container |
| Sound bank | `.bnk` present, unexamined |

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

Every class HD authors resolves against `oag_formats::vex::classes::V6` except
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
| 170 | `0x3d8` | cloudCube | | 28 | `0x3ed` | **unrecovered** |
| 89 | `0x3da` | weatherPos | | 17 | `0x12c` | AmbientLight |
| 56 | `0x3ce` | wospot | | 17 | `0x131` | DirectionalLight |

plus `exitglow` `0x3e4` and `Mag Floor Collision` `0x3e6` at 10 each, `Skycube`
`0x3c6` at 9, `fogCube` `0x3d3` at 5, `soundcone` `0x3e9` at 3, and
`Dynamic Shadow Occluder` `0x3c3` and `gridCamera` `0x3dd` at 2.

**`0x3ed` is past the last entry anyone has read out of Pulse's table.** That is
a weaker statement than it looks and it is the accurate one: `vex.md` records
the table as read only as far as `0x08ab26a0`, with the terminator never
reached, so Pulse's own class space may well extend to `0x3ed` and nobody has
looked. HD authors it exactly once per track file, always under the node name
`collision_trackwall`, and its payload has not been read. The name and the count
both suggest a sixth collision class; that is a hypothesis, written down as one
rather than added to any table. **The cheap next step is not on this disc**:
read further in Pulse's own class table past `0x08ab26a0` and the ID may name
itself for free.

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
identical nodes, which is the obvious candidate for the reference into
`.rcsmodel` - unread, and named as unread.

So `oag-view --mesh` cannot draw an HD track, and the reason is not the `.vex`
layer at all. `--track` can, and does: the ribbon is built from the `WO Track`
spline, which is authored data that never moved.

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
`VEXX`/`XXEV`. So `oag_formats::track::parse` sniffs it and reads a Wipeout HD
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

`oag_formats::track::SplinePoint` names fields up to `+0x61`. Two regions outside
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

[`oag_formats::pvs`](track.md) reads the `section` `0x3c9` payload's 64-bit
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

## Pads and the grid read unchanged

`Speedup Pad` `0x3bd` and `Weapon Pad` `0x3be` carry [the same payload](pads.md)
they do on the PSP: the trigger volume is the mesh's own bounding-box pair at
`+0x10` and `+0x20`. Across the 28 circuits, **416 speedup pads and 208 weapon
pads**, with `min <= max` componentwise on all 624. Talon's Junction's pads
measure `(-4.66, 1.33, -4.66)` to `(4.66, 1.33, 4.66)`.

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
[`oag_formats::handling`](handling-stats.md) parses, element for element -
`easyshield` and `weight_distribution` included, both of which Pulse authors and
the parser already reads:
`<Misc height length shield easyshield width weight_distribution>`,
`<ExternalCameraFar>` with the identical seven attributes, `<AirbrakeGraphics>`,
and per-class `<Engine accelcap amount falloff gain turbo>`, `<Brakes>`,
`<Turning>`, `<Airbrake>`, `<Antigrav>`, `<Physical>` blocks for
VENOM/FLASH/RAPIER/PHANTOM. Whether the parser accepts them unmodified has not
been tried - that is a code change, and this page is a reading.

**`environments/<track>/stats.xml`** carries target race and lap times per speed
class, Zone and Elimination targets, a circuit length and location string, and a
per-difficulty `SkillScaleValue` table.

**`plugins/grids/grid_*.xml`** is the campaign, cell by cell: each `PI_Cell`
names a track, a mode, a class, `AICount`, `Weapons`, `damage`, laps and medal
targets for three difficulties. `plugins/frontend/gui/*.xml` is the front end's
own screen set.

**`xml/weaponstats_race.xml`** is [the weapon table](weapon-stats.md) under a
lowercase name, alongside `_detonator` and `_elimination` variants for modes
Pulse does not have.

## `.pob` reads, with one field that no longer agrees

**Confidence 85.** The [container](pob.md) is byte-swapped: the magic reads
`PSYS` where Pulse writes `SYSP`. Over all **88** effects:

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

**The length check does not hold on any of them.** On all 35 Pulse files
`+0x04` equals `HEADER_LEN + payload.len()` exactly; on HD it falls short of the
file length by 48 to 208 bytes, always a multiple of 16, and the excess is
readable developer strings (`Z:\WipeoutHD\Dat`, `a\Source\Common\`). The obvious
reading is that HD appends a string block the length field does not count, which
would fit Pulse's own finding that 43-80 % of slot-resolved records are developer
strings - but that is a hypothesis, and the field is recorded as disagreeing
rather than explained away.

## What is genuinely new

- **`.rcsmodel`** - 643 files, 686.5 MiB, all render geometry. Nothing read.
- **`.rcsmaterial`** - 1,632 files, wall-to-wall 32-bit hashes. A track authors
  around fifty by name (`track_surface`, `track_wall`, `glass_reflect`,
  `emissive_bloom`), and a second identical set under `materials_reversed/`.
- **`.gtf`** - 7,333 files, the PS3's own texture container and publicly
  documented. One sample header reads version `0x0105`, one texture, payload at
  `+0x80` behind an RSX texture descriptor. Not surveyed.
- **`.pvs`, `.pvspatch`, `.probes`** - one of each per circuit direction, 28
  apiece. `talons_junction/track.pvs` is 114 KiB against 19 `section` nodes, so
  it is a finer structure than the mask in the `.vex`, not the same table moved.
- **`.bik`, `.mp3`, `.stencilvolume`, `.svml`, `.xfx`, `.points2`, `.effectsettings`,
  `.envsettings`** - the last of these is plain text, a key/value list starting
  `"Lighting.Constant ambient color"=0.403922 0.392157 0.509804`.

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
- **`0x3ed` is not in any table.** Adding it to `classes::V6` on the strength of
  its node name would be exactly the guess-dressed-as-a-name the
  [rubric](../reverse-engineering/confidence-rubric.md) forbids.
- **`.psarc` paths are lowercase, the digest is over the uppercase spelling.**
  See [psarc](psarc.md#the-check-the-confidence-rests-on).
- **A `.psarc` inside an encrypted image reads as noise.** Decrypt first;
  `scripts/psarc.py` reports the bad magic rather than guessing.

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
