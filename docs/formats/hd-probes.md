# `track.probes`: Wipeout HD's baked environment probes

**Status: understood.** The layout, the block format and the blend a hull is lit from
are decoded, reproduce the original's live maps to within 8-bit rounding, and are
consumed by the renderer.

Reader: [`oag_vex::probes`](../../crates/vex/src/probes.rs). Ground truth:
`crates/vex/tests/probes_ground_truth.rs` (every file on the disc, and 52 pauses of
the original). Consumer: `oag_raceplay::env_probes`, bound in the scene group and read
by `crates/mesh/shaders/shade.wesl`. Executable side:
[env-probes.md](../ghidra/functions/ps3-hdfury-eu/env-probes.md).

**This overturns the earlier reading** (`hd-ship-materials.md`, "Finding 4", 2026-10-20)
that `.probes` is a PVS structure and that the hull's environment maps are rendered at
runtime. They are disc data: the original blends a few of these probes by the craft's
position and writes the result, on an SPU, into the two textures the hull samples.

## Layout (confidence 85 for the header, position table and image sets; 85 for the block format; the 24 + 4 + 3 byte tail is unread, no score)

One file per circuit direction, 28 on the disc, big-endian:

```text
u32  count            862 on Talon's Junction
u32  16               stride of the position table
f32  position[count][4]            x y z z   (the fourth repeats z)
u8   irradiance[count][1536]       64x32, 12 bytes per 4x4 block
u8   reflection[count][6144]       128x64, same block format
u8   tail[count * 31]              24 + 4 + 3 bytes per probe, read by the loader, meaning unread
```

The length closes exactly on Talon's Junction; longer circuits carry more after the tail
(Sol 2 reversed, 123,632 bytes over). The loader (`0x003c1120`, [env-probes.md](../ghidra/functions/ps3-hdfury-eu/env-probes.md))
reads the arrays in this order and stops.

A block is **two `BGR888` endpoints (6 bytes) then sixteen 3-bit indices, least
significant first, row-major inside the block**; a texel is `c0 + (c1 - c0) * index / 7`.
Blocks run row-major across the image. The `/7` is measured: `/8` leaves a residual of
1.8 RMS against 0.4.

The probes come in left/right pairs (entries `2k`, `2k + 1`, about 20 units apart across the
track) and a section every 12-23 units along it. The first `3 * count` bytes of the
24 + 4 + 3 tail look like one average `RGB` per probe; none of it is read here.

## What the original does with it (confidence 85 that the maps are this blend; the weights are fitted, not read)

The live `paraboloidIblTex` (64x32) and `paraboloidReflectionTex` (128x64) of a craft are
a **weighted blend of the probes around it**, equal to this file's images:

- Harimau at rest on Talon's Junction: `0.652 * probe[852] + 0.340 * probe[854]`, RMS
  0.4-0.65 of 255 on both maps.
- 52 pauses of two drives round Talon's Junction (positions and the live maps, read over
  GDB): 50 of 52 are within 10 of 255 at the best of zero, one or two ticks of position
  lag, most within 2-4. Where the weights are fitted (non-negative least squares over the
  twelve nearest probes), 0.4-0.6 RMS.
- The weights are **bilinear over the four probes of the two sections the position lies
  between**: along the track by the projection on the line between the section centres,
  across it by the projection on the left-right axis, both clamped
  (`Probes::locate`). One craft on one side of the track blends two probes; one between
  them blends four. The position the original uses is **one tick old** (lag 1 fits best on
  41 of 52 pauses, lag 0 on 4 and lag 2 on 7).
- **The weighting rule is fitted to the live maps, not read from the SPU job** (its program was not read): label it chosen and fitted, no confidence score beyond the fit above.
- Two of the 52 pauses (tight corners) blend a section pair **two apart** where the law
  takes neighbours; held in the test as named exceptions. So the law is 70, not 85.

Each craft has its own pair of maps: the Metropia grid frame holds eight pairs of
addresses, one per craft, 0x8000 apart, and every draw of a craft binds its own.

## Not what the earlier lane thought

**Corrected 2026-10-21 (`hd-one-pair-moa`)**: the producer names below do exist on the running game, for the *scenery's* map: the
512x256 `paraboloidReflectionTex` at `0xc4065380` is the circuit's sky resampled
([hd-track-tone.md](../rendering/hd-track-tone.md), "The reflection map is the sky"). It is a different map from the hull's 128x64 pair this file
holds, and it is written once at load, which is why no frame shows a draw into it.

`liveRsxPanXForm_CubemapToDualParaboloide_fp` and `liveRsxPanXForm_CubemapConvolve_fp`
(names in the executable) are not on any frame of the running game: eight captures and a
draw-state poll at 100 Hz found no draw into either map. They read as the offline bake
that produced this file, which is the likely reason `.probes` carries a dual paraboloid.
Confidence 55: nothing here reads the bake.

## Cross-title

- **2048** (Vita, `data.psarc`): **checked, differs.** It ships 28 `.probes` (three DLC1 circuits
  among them) *and* 14 `skyParaboloid.gxt`, so the pair exists there too, but the file is not
  this one: little-endian, a header of `u32` count and a `u32` that reads `2` or `1`, then
  records of 1,112 bytes (Anulpha Pass: 1,146 probes, 1,274,368 bytes = 16 + 1,146 x 1,112) or of
  88 bytes (its reversed file: 858 probes, 75,520 bytes = 16 + 858 x 88). No `64x32`/`128x64`
  image set fits that. Not decoded: a 2048 race load names the file as a different layout
  (`probes::Error::Stride`, `hd_env_probes_ground_truth`) and binds no probes. Whether 2048's hull
  programs sample a probe or the authored sky paraboloid is unread: open.
- **Omega** (PS4): **checked, differs.** No `.probes` in any archive of the base package (`data00`-`data04`)
  or the patch (`data05`, `data07`-`data09`), so an Omega load binds none and its hulls keep the constant ambient: nothing was
  ported there by accident. Its two 2048-lineage circuits (`tower`, `mall`) carry a `track.shprobes` of 3.7 and 4.7 MB:
  spherical-harmonic probes by the name, not read. HD-derived hull programs on Omega would sample a probe all the same;
  not checkable without a PS4 emulator.
