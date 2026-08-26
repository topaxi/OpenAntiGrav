# Wipeout 2048 status

The [ADR-0009](../architecture/adr/0009-multi-game-fanout.md) probe run a third
title on: what the existing tooling reads off *Wipeout 2048* unchanged, what it
refuses, and what `oag-2048` was filled in from. Measured against
`data/extracted/vita/PCSF00007` - the decrypted EU package, base plus both DLC -
on 2026-08-26.

**The short version.** 2048 is Wipeout HD's asset tree on a little-endian
console. The container, the plugin XML, the handling XML and the `.vex` class
table all read with no code change at all. Three *binary* formats changed
underneath it, and one of the three is now read.

## What reads unchanged

| Layer | Reads? | Notes |
| --- | --- | --- |
| [PSARC](psarc.md) container | **yes** | Version 1.4, flags 1, 18,430 entries in `PSP2/data.psarc`. HD's PS3 archives are version 1.3, flags 3; nothing asserts either. |
| [`.vex`](vex.md) class table | **yes** | Version 6, little-endian, the same class IDs HD's version 6 carries. |
| [Handling stats](handling-stats.md) | **yes** | Both the HD-derived roster's per-team files and the shared `<Global>` block at `Data\XML\handlingstats.xml`. The native roster's five teams parse too, `SUPERPHANTOM` class included. |
| [`WO Track`](track.md) spline | **yes, after a version gate** | See below. |
| Plugin definitions | **yes, but split three ways** | `Data\Plugins\teams\`, `tracks\` and `music\` each ship their own `Definition.xml` where every other title ships one file carrying all three node kinds. `oag_title::Title::plugin_definition` holds one name, so this build reads the teams one and sees no circuit or soundtrack list. |

## What changed, and what state each is in

**`WO Track` is version `0x107` and its control point shrank from 112 bytes to
96 - recovered, and the parser is version-gated for it.** Everything up to and
including `racing_line` stayed exactly where it was; only `section_id` and
`flags` moved, from `+0x60`/`+0x61` to `+0x5c`/`+0x5d`. The evidence is a
**pairing against Wipeout HD**: 2048's DLC re-ships twelve HD circuits with
identical path and per-path control-point counts, so HD's fully decoded
`0x106` record is ground truth for 2048's, 10,165 control points of it. Full
numbers and confidences are in
[track.md](track.md#wipeout-2048-authors-version-0x107-and-the-control-point-shrank-to-96-bytes).
All fourteen circuits the base package ships now parse with
`AiTrack::encoded_len()` landing on the payload length to the byte.

**`.rcsmodel` is not HD's container - unread.** `Data\art\published\environments\altima\track.rcsmodel`
opens `ed ad 5c ca` where [HD's](rcsmodel.md) opens `0x000a0000`, and is 17.4 MiB
for one circuit. `RcsModel_Load` (`0x812f15b2`) is named and the outer
three-section shape is confirmed by disassembly *and* by the arithmetic closing
on the real file - `189824 + 682966 + 16573144 = 17445934`, exactly the file
length. All three sections' interiors are unread. See
[track-and-collision-loaders.md](../ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).

**Collision moved out of the `.vex` into `track_col.col`, a k-d tree - outer
shape read, geometry unread.** `KdTree_Load` (`0x8118d134`) is named off its own
`"kdtr%04x"` format string; the magic, the ASCII version, the `"----"` section
tags, the 24-byte node array and the leaf index array all land where the
decompiled control flow predicts on the real 1,068,082-byte file. What is *not*
read is each node's trailing 16 bytes and the entire `KdTreeMeshShape.cpp`
trailer, which is where the triangles live. Same evidence page.

## What a race does today

`just play 2048 --race` opens the package, reads the spline, places the craft on
the start line and runs the simulation. What it does **not** do, and why:

- **The craft falls through the floor.** `collision::from_vex` returns zero
  colliders because 2048 authors none there, and `track_col.col` is not decoded
  - so there is no ground. This is the one thing between the current state and a
  race, and decoding the k-d tree's mesh trailer is what fixes it.
- **Nothing draws its authored surface.** Both the circuit and every craft have
  external geometry in `.rcsmodel`, so the circuit falls back to the derived
  ribbon and the craft draw nothing at all - the same honest half-picture a
  Wipeout HD race gives, for the same reason.
- **No front end, no music, no HUD art.** `oag_2048::TITLE` carries
  `front_end: None`, `loading: None` and `music: None`, and its `hud_art` is
  `Sights::Unread` with an empty always-on set. None of it has been read; a
  copy of HD's would compose to *something*, which is how a wrong picture
  survives review.

## The one axis 2048 forced into existence

`oag_title::RaceDefaults::ship_dir`. Pulse, Pure and Wipeout HD all keep their
rosters at `Data\Ships\<Team>\`, and `oag_title::race`'s own docs argued that
made it shared vocabulary rather than a title fact. 2048 disagrees: its fourteen
HD-derived teams are at `Data\art\published\hdships\<Team>\`, carrying the same
file set under the same names. That is the third-title disagreement
[ADR-0022](../architecture/adr/0022-title-packages.md) asks for.

**Its own five teams are not raceable here**, and the reason is shape rather
than spelling: `ag_systems2048`, `auricom2048`, `feisar2048`, `piranha2048` and
`qirex2048` live under `Data\HandlingStats\<team>\<1..4>\handlingstats.xml`, a
two-level directory this single-string axis cannot express, and what the
numbered level selects is unread.
