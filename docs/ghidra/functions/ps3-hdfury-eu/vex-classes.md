# The `.vex` class table, read whole

**`g_VexClassTable` at `0x00921110`: 866 records, terminated by `id == -1` at
`0x009239a8`.**
Recovered 2026-08-18. This is the same table Pulse's PSP `BOOT.BIN` carries at
`0x08ab2370`; that read stopped short of the terminator and left the table's
extent unknown, and this one closes it.

Implemented as
[`oag_vex::vex::class_names`](../../../../crates/vex/src/vex/class_names.rs),
checked against every disc by
[`vex_class_ground_truth.rs`](../../../../crates/vex/tests/vex_class_ground_truth.rs).

## The record

```text
+0x00  u32     class id
+0x04  char *  name, NUL-terminated
+0x08  ptr     a runtime slot, `0x00aeeb80` on every one of the 866
```

Stride 12, identical to the PSP's. The third field is the same runtime slot
[`exhaust.md`](../psp-pulse-usa/exhaust.md) identified there - **not** a shared
vtable, which is what it looks like: it holds one value across the entire table,
which no vtable pointer would.

**Confidence 95.** It is not an inference: the ids that were already known from
unrelated evidence all agree, the terminator is present, no id repeats across
866 records, and every class any shipped `.vex` file carries on four discs
resolves. The one thing keeping it below 100 is that nothing here watched the
registrar walk it.

## How it was found, and the framing trap in it

Not through the decompiler. The name strings sit packed from `0x0079c000`
(`"Floor Collision"` at `0x0079c6d0`, `"cloudCube"` at `0x0079c860`), and
scanning the ELF for words pointing into that block finds 49 of them at a
regular 12-byte stride starting `0x00921114`.

**That stride is right and that start is wrong, in a way that produces a
plausible table.** Framed from `0x00921114` the records read
`{name, ptr, id}` - and pair `'Mesh'` with `0x6e`, `'Transform'` with `0x12c`,
`'AmbientLight'` with `0xf7`. Every one of those is a real class id and a real
class name, and every pairing is off by one entry. The tell is that `0x6e` was
already known to be `Transform` from node censuses; framed from `0x0092111c`
instead, as `{id, name, ptr}`, `0x6e` is `Transform`, `0x12c` is
`AmbientLight` and `0xf7` is `Camera`, all of which independent evidence
already said. **A one-field rotation of this record is self-consistent and
wrong**, which is the same shape as the TOC defect [`memory.md`](memory.md)
records: arithmetic that checks out against the wrong base proves
self-consistency, not correctness.

## What it says

**Maya's own class enumeration first**, `0x0000` `Invalid` through `0x03b8`
`Last`, then the **58 game classes**, `0x03b9`..`0x03ee`. That is why the game's
ids start where they do: `Last` is Maya's sentinel and the game numbers from the
next value up.

Nine game classes take Maya ids rather than game ones - `Transform` `0x6e`,
`World` `0xf4`, `Camera` `0xf7`, `NurbsSurface` `0x123`, `Mesh` `0x125`,
`AmbientLight` `0x12c`, `DirectionalLight` `0x131`, `PointLight` `0x132`,
`LodGroup` `0x2ee` - and each is listed once, in the game block. No id appears
twice in the whole 866.

### The 58 game classes

| id | name | id | name |
| --- | --- | --- | --- |
| `0x3b9` | Floor Collision | `0x3d5` | seaweed |
| `0x3ba` | Wall Collision | `0x3d6` | sea |
| `0x3bb` | WO Track | `0x3d7` | seareflect |
| `0x3bc` | Start Position | `0x3d8` | cloudCube |
| `0x3bd` | Speedup Pad | `0x3d9` | cloudGroup |
| `0x3be` | Weapon Pad | `0x3da` | weatherPos |
| `0x3bf` | Engine Flare | `0x3db` | Unused 1 |
| `0x3c0` | Anim Transform | `0x3dc` | animationTrigger |
| `0x3c1` | Texture | `0x3dd` | gridCamera |
| `0x3c2` | Dynamic Point Light | `0x3de` | lensflare |
| `0x3c3` | Dynamic Shadow Occluder | `0x3df` | textureBlob |
| `0x3c4` | ParticleSystem | `0x3e0` | blob |
| `0x3c5` | Airbrake | `0x3e1` | sound |
| `0x3c6` | **Skycube** | `0x3e2` | Ship Muzzle |
| `0x3c7` | Quake | `0x3e4` | exitglow |
| `0x3c8` | Trail | `0x3e5` | engine_fire |
| `0x3c9` | section | `0x3e6` | Mag Floor Collision |
| `0x3ca` | gate | `0x3e7` | Cage Collision |
| `0x3cb` | shadow | `0x3e9` | soundcone |
| `0x3cc` | speaker | `0x3eb` | cannon_flash |
| `0x3cd` | Reset Collision | `0x3ec` | **wingtip** |
| `0x3ce` | wospot | `0x3ed` | **Track Wall Collision** |
| `0x3cf` | wopoint | `0x3ee` | **absorb** |
| `0x3d0` | Ship Collision Fx | | |
| `0x3d3` | fogCube | | |
| `0x3d4` | MeshNode_Ghost | | |

`0x3e3`, `0x3e8` and `0x3ea` have no entry at all. The ids are **not** in
address order in the table - `0x3d0` sits between `0x3cd` and `0x3ce`, and
`0x3e9` between `0x3e1` and `0x3e2` - so a gap proves nothing about a missing
class.

### The three corrections it makes

Bolded above, and all three were live defects in
`oag_vex::vex::CLASS_NAMES`:

- **`0x3d5` and `0x3d6` were swapped** - `sea` and `seaweed`, adjacent ids with
  adjacent names, which is exactly the error a single-title read cannot catch.
- **`0x3ec` `wingtip`, `0x3ed` `Track Wall Collision` and `0x3ee` `absorb` were
  absent.** `0x3ed` is the one that mattered: Talon's Junction authors one, with
  a 99 KiB payload, and it read as an unnamed class in every census this project
  printed while the file's own node name `collision_trackwall` said what it was.

### And it forced the generic classes into the table

They had been left out of `CLASS_NAMES` on the reasoning that no shipped node
uses one. **Pulse's own version-6 `.vex` files carry 379 nodes of class
`0x0000`** - `ViewCompass` and other Maya scene furniture the exporter left in -
and one `0x0108` `NurbsCircle`. The ground truth is what caught it.

## This is version 6's id space, and there is at least one other

Pulse's **version-4** files use a different enumeration entirely. Seven ids live
there and are absent from this table: `0x372`, `0x373`, `0x375`, `0x378`,
`0x382`, `0x38f`, `0x397`. `0x378` is version 4's `skycube` - the node in
`Data\Defaults\Skycube.vex` is named `skycube1_nolightShape` - where version 6's
is `0x3c6`. So a version-4 file must **not** be read through this table, and the
ground truth asserts over version-6 files and reports the others.

## What this settles about the sky, and what it does not

**Skycube is `0x3c6` and Wipeout HD's Talon's Junction authors none.** That was
previously an inference from Pulse's borrowed table; it is now read from HD's own
executable, and the class it *does* author at the id that looked unknown is
`Track Wall Collision`, which is collision rather than sky.

So HD's sky is not a `.vex` node. `Skycube_Importer.cpp` is present in the
binary - `scripts/ps3-toc.py map` attributes `0x002db850`, `0x002db898` and
`0x006b4748` to it - but that codebase built Pulse too, and an importer for a
class no shipped circuit authors costs nothing. **Where HD's sky comes from is
still unread**; `sky.gtf` sits beside every circuit's `track.vex` as a named
file and is a 1024x1024 DXT1 cubemap, so the load is by path rather than by node
hash. See [`envsettings.md`](../../../formats/envsettings.md), whose
`Lighting.Sky colour` and `Sky rotation` are read and unused for the same reason (as a *lit-material* input; both are consumed by the sky draw, `Sky colour` as the cube's vertex colour since 2026-10-07 - see renderer.md).

## Open

- **The registrar.** Nothing here watched a walk of this table, which is the
  last step between 95 and what an executed branch would carry.
- **Version 4's own table**, if the executables carry one. Only seven of its ids
  are even observed.
- **`0x3db` `Unused 1`**, which is a name the table itself gives and not a gap.
- **What `wospot`, `wopoint`, `blob`, `textureBlob` and `Unused 1` are.** Named
  now, decoded not at all.

## See also

- [`exhaust.md`](../psp-pulse-usa/exhaust.md) - the PSP read of the same table
- [`memory.md`](memory.md) - the TOC defect, and the same self-consistency trap
- [`vex.md`](../../../formats/vex.md) - the format these classes belong to
