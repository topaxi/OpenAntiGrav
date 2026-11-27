# HD/Fury: the craft hull's light in a Zone frame

`EBOOT.elf` (WipEout HD Fury, PS3, EU, `BCES00664`), Ghidra program
`/hdfury/EBOOT-ps3-hdfury-eu.elf`. Read 2026-11-20 (`zone-craft-light`): static
disassembly, then three RPCS3 cold boots into a Racebox Zone race on Vineta K
(Feisar, stage 1 `Sub Venom`), paused 1.5 s into the countdown. The presented frame of
those pauses equals the adopted pair `hd/zone-vineta-k-z5` over the craft box to 4
bytes per channel.

**The question.** In `hd/zone-vineta-k-z5` the original's Zone hull is a light,
neutral grey and ours a dark, warm grey: craft box (`x 780..1080, y 600..880` of the
1920x1080 pair) original `(73.0, 92.6, 104.8)`, ours `(41.6, 60.3, 65.4)`, 38.56 mean
grey on the box.

**The answer, in part.** The hull's probe tint (`iblScalePower`, parameter
`0x7480de6d`) is not a constant of the mode: it is **twice the stage blend of
`Track.Base Colour Highlight`**, with `.w` 1. Wired. It turns the hull from warm to
neutral and moves the box 38.56 -> 33.58.

**The other half (2026-11-20, `zone-hull-fable`).** The factor of two that was left is
a term of one material's program, not of the mode: the Zone ship's pale panels are
`zoneship01`, whose `IBL` block adds a Schlick rim, `fresnelBase + (1 - fresnelBase) *
(1 - N.V)^5`, to the lit albedo, untinted. Wired; the craft box goes to 15.07 and the
pair to 6.57 (see "The rim term").

## The law: `EnvSettings + 0x510`

| item | value |
| --- | --- |
| writer | `stvx v31, r3, r0(=0x510)` at `0x003db260`, inside `Environment_UpdateStageBlend` (`0x003da540`) |
| `r3` | the settings block (`bl 0x003a9520` just before; `0x00c489f0` live) |
| blend | `v31 = lvx(r30 + 0x3560)` and `v13 = lvx(r30 + 0x3570)`, two stages' copies of one per-stage vector, cross-faded by the stage weight (`vmaddfp` at `0x003db228`/`0x003db22c`) |
| `.w` | `vsel` with a splat of `0.5` (`0x007b1f40`, TOC `-0x5794` of this function's TOC) at `0x003db230` |
| scale | `vmaddfp v31, v31, splat(2.0), v29` at `0x003db25c`; `2.0` is `0x007b1f50` (TOC `-0x5790`, `scripts/ps3-toc.py resolve 0x003da540 a870`) |
| reader | `Scene_PrepareFrame` copies `+0x510` to `block + 0x7c40` (`lvx` at `0x003ab100`), the hull programs' `iblScalePower` |

So `iblScalePower = (2 * blend(rgb), 1.0)`. The ordinary (non-Zone) arm writes
`(4, 4, 4, 0)` to the same field at `0x003abb10`..`0x003abb28`; the branch to
`0x003ad748` at `0x003abae8` is the Zone arm's entry, which goes on to call
`Environment_UpdateStageBlend` at `0x003ad8e8`.

**Which key.** `r30 + 0x3560` is identified by value, not traced to the loader's
record offset:

| stage, file | `Track.Base Colour Highlight` | twice it | live `iblScalePower` |
| --- | --- | --- | --- |
| 1 `Sub Venom`, `DATA02` `zonemode.effectsettings` (Vineta K) | `0.349020 0.501961 1.000000` | `0.69804 1.00392 2.0` | `{0.69804, 1.00392, 2, 1}` (this lane, both hull programs) |
| 3 `Sub Flash`, `zonemodedlc3.effectsettings` (`Zone_1`) | `0.349020 0.349020 0.349020` | `0.69804` grey | `(0.69804, 0.69804, 0.69804, 1.0)` (`hd-zone-hulls`, 2026-10-20, [hd-ship-materials.md](../../../rendering/hd-ship-materials.md)) |

The settings block of the same pause holds `(0.69804, 1.00392, 2.0, 1.0)` at
`0x00c48f00` (`+0x510`). No other key of either stage takes these values. Two stages
of two files agreeing on one mapping, with the writer's scale and lane read at
instruction level: **confidence 85**. What would raise it: tie the `lvx r9, r5` loads
at `0x003daa20`..`0x003daa30` that fill `+0x3560`/`+0x3570` to the loader's record
offset for `Track.Base Colour Highlight`
([zone-effectsettings-loader.md](zone-effectsettings-loader.md)).

## The hull's program in a Zone frame

`scripts/rsx-fp-match-material.py` against `data/materials/ships/*.rcsmaterial`
(`DATA02`): every hull draw is fragment block 17 (`HalfBrightIBLSunShadowMapSpot0SVC1`,
word `0x847`) of `diffuse_with_specular_from_alpha_n`, `zoneship01` or `diffuse`; no
`ZoneMode` permutation reaches the hull. `Ship_DrawModels` (`0x003ea368`) builds the
word as `SVC1 (0x800, a light touches the craft) | ShadowMap (0x40) | spots | IBL (6,
when the craft has a probe) | Sun (1)` and never reads `EnvSettings + 0x4b0`: the byte
the Zone arm clears gates only the seven track-chunk functions (an operand search for
`lbz ..., 0x4b0(` finds `0x003fc560`, `0x003ffb98`, `0x00400d40`, `0x00403e58`,
`0x00405c68`, `0x0040790c`, `0x004094c8`, nothing else). So the Zone surface's light
law ([zone-frame.md](zone-frame.md)) does not reach the hull.

The live program (`frame-c1116400.bin`, `scripts/ps3-fp-live.py`):

```text
light = luma(tex(unit 4)) * iblScalePower.rgb      @0x27..@0x30 (desaturated: .w = 1)
      + sat(N.L) * shadow * {0, 0, 0}              @0x1e: the stage's sun is 0 at Sub Venom
      + f[TC1]                                     @0x34, the SPU vertex light
out   = (fog(albedo * light + reflection terms)) / 2
```

Read live in the same pause:

- **Probe:** unit 4 (`0x03a94780`, 64x32 `A8R8G8B8`, gamma nibble 7) equals Vineta K's
  `track.probes` entry 210 to 2.2 of 255 RMS (`zone_1/track.probes`' nearest: 49):
  ours loads the same file. Raw mean `(73.3, 75.7, 72.9)`, luma 0.293.
- **SPU light:** the `SpuVertexColours` stream of each hull draw (`--small 2500`
  dump): zero on 25 of 31 chunks; the engine's blue on draws 616-620 and 639 only.
- **Sun:** `{0, 0, 0}` on every hull and Zone-surface program (`Sub Venom` authors
  `Sun colour` 0).

## The port

`oag_title::ZoneFrame::hull_probe_tint_scale` (`Some(2.0)` in `oag_hd::race::ZONE_FRAME`,
`None` elsewhere); `ZoneGrade::light` puts `scale * palette().track_base_colour_highlight`
into `mesh_render::Light::hull_probe_tint` (`.w` 1), and `shade.wesl`'s `probe_tint`
gives a hull with a probe layer `luma(probe) * hull_probe_tint.rgb` where `.w` is set.
Tests: `oag-raceplay` `zone_grade::tests::a_zone_hull_takes_twice_the_stages_track_highlight_as_its_probe_tint`
and `a_zone_hull_tint_cross_fades_with_the_stage`; `oag-tables`
`effectsettings_ground_truth::hd_files_parse_and_share_the_fifteen_stage_ladder` checks
both live values against the disc's two files.

`hd/zone-vineta-k-z5`, re-rendered on a copy:

| build | pair mean grey | craft box grey | craft box ours (rgb) |
| --- | ---: | ---: | --- |
| before (`1f516ac87`) | 7.42 | 38.56 | `(41.6, 60.3, 65.4)` |
| the tint | **7.27** | **33.58** | `(41.8, 66.4, 84.6)` |
| original | | | `(73.0, 92.6, 104.8)` |

Every other HD race pair (the thirteen circuits, `talons-assegai-c1`,
`hd-race/talons-pack`) keeps its score. Pixel for pixel against a build of `main` at
`9cc77d9b6` (after the Omega Zone ladder merged): `hd/vineta-k`, `hd/anulpha-pass`,
`omega/zone-vineta-k-z5`, `omega/zone-capital-reach-2048-z5`,
`2048-vita/zone-dlc-vineta-k-z5` and `2048-vita/zone-altima-z5` differ in 0 pixels.

## The rim term: `zoneship01`'s `fresnelBase`

**The law (confidence 90).** Fragment block 17 of `zoneship01.rcsmaterial` (`DATA02`,
block at `0xe7e0`, the `IBL` variant every hull draw resolves, word `0x847`), read with
`scripts/ps3-microcode.py fp-file zoneship01.rcsmaterial 17`:

```text
@0x15  DP3 R1.x, f[TC3], f[TC3]          @0x1d  DIVSQ R1.xyz, f[TC3], R1     V = normalize(TC3)
@0x25  DP3 R3.y, -R1, R2                                                       -N.V  (R2 = N, normal-mapped)
@0x28  ADD_SAT R1.w, R3.yyyy, {1}                                              sat(1 - N.V)
@0x2d  LG2 R1.w   @0x2e  MUL R1.w, R1, {5}   @0x33  EX2 R1.w                   (1 - N.V)^5
@0x38  MAD R1.w, R1, -{fresnelBase}.xxxx, R1                                   * (1 - fresnelBase)
@0x3c  ADD H1.w, R1, {fresnelBase}.xxxx                                        + fresnelBase
@0x47  MAD H0.xyz, H0, H1, H1.wwww                                             albedo * light + rim
```

`H1.xyz` at `@0x47` is the light of the page's first section (`luma(probe) * iblScalePower
+ sun * shadow + f[TC1]`), `H0` the albedo. The rim is one scalar added to all three
channels after the albedo multiply, so neither the albedo nor the probe tint colours it;
the specular (`@0x77`) and the reflection (`@0x7a`, `luma(tex(unit 5)) * iblScalePower *
albedo.a * reflectivity`) are added after it, then the fog and the `/2` (`@0x7d`). `V` is
the view vector: `@0x45` adds the sun direction to it for the half vector.

`diffuse_with_specular_from_alpha_n`'s block 17 (`0xdae0`), the hull's other paint, is
the same program without `@0x25`..`@0x3c`: its `@0x37` is `MUL H2.xyz, H2, H3`, no add.
That closes the page's old read 2 (the tail past `@0x5f`): the tail is the specular and
the reflection, both as ours already draws them.

**The parameter is `fresnelBase`.** Hash `0x743ea80b`, patched into slots `0x39` and
`0x3d`. `~crc32("fresnelBase") = 0x743ea80b`, and Omega's and 2048's copies of
`zoneship01.rcsmaterial` declare the name in the clear (`strings`: `fresnelBase` 8 times
in each). `oag_rcs::rcsmaterial::names::KNOWN_PARAMETER_NAMES` carries it.

**The value.** `/data/ships/zone/ship.rcsmodel` (`DATA02`) authors `0.1234` on its
`zoneship01` material (reflectivity `0xab31c2b1` `0.7`, specular power `0x4232e459`
`100`); `hd_material_dump` prints it. **Live** (one RPCS3 cold boot into the same Zone
countdown, `scripts/rpcs3-hd-zone-frame.py`): the patched program of the `zoneship01`
draws (`frame-c1117dc0.bin`) holds `{0.1234, 0, 0, 0}` at `@0x38` and `@0x3c`.

**Where it applies.** Of the 40 hull draws of the paused frame (draws 609..648, the ones
that bind the 64x32 probe at unit 4), `rsx-fp-match-material.py` gives:

| program (`fp`) | material, block | draws | indices |
| --- | --- | --- | ---: |
| `0x1115c00`, `0x1116b40`, `0x1117dc0`, `0x1119780` | `zoneship01` #17 (the rim) | 609, 636, 641..644, 648 | 7,605 |
| `0x1116400`, `0x1117340`, `0x11185c0`, `0x1118d00` | `diffuse_with_specular_from_alpha_n` #17 | 610..635, 637..639, 645, 646 | 35,553 |
| `0x1117a80`, `0x1119440` | `diffuse` #17 | 640, 647 | 1,182 |

Few triangles and most of the visible paint: the pods' and the nose's flat pale panels.
A census of all seven archives for the hash finds it in three materials: `zoneship01`
and two weapon glows (`hd_leachbeam_ball_glow`, `hd_detonator_cannonbolt_halo`, where it
is a different term, [hd-unlit-programs.md](../../../rendering/hd-unlit-programs.md)).
Only `ships/zone/ship.rcsmodel` authors it on a hull, so the port reaches the Zone ship
alone.

**Three independent reads agree**: the disc's program, the model's authored value and
the live patched constant; and the picture (below) lands within 1 byte on a flat panel.

### The page's other two reads, closed

1. **Unit 0 is the livery ours loads.** The live binding of unit 0 (`0x0a7c0100` this
   boot, 1024x1024 `DXT45`) equals `zone_tp_1024.gtf`'s payload byte for byte over the
   589,824 bytes dumped, and unit 1 `zone_tp_1024_n.gtf`'s. The file is identical in
   all thirteen `zoneship_*` directories. No part of the gap.
2. **Unit 5 is the probe file too.** The live 128x64 reflection map equals Vineta K's
   `track.probes` entry 210 to 3.8 of 255 RMS (next best 26.6), the 64x32 map to 2.5.
3. **The scene target is not where the hull is read.** `target-01920000.bin` of a
   paused frame holds the next frame's scene up to draw 608: the pause lands before the
   hull draws, so the dump shows the track without the craft. The hull was compared on
   the presented frame instead.

### The port

`mesh::rcs::probe_lit` puts the model's `fresnelBase` in `tint.y` of the hull material's
glow-table entry where the `IBL` block declares the parameter, and `shade.wesl` adds
`base + (1 - base) * (1 - N.V)^5` to the lit sum of a craft's probe-lit material.
Tests: `oag-mesh` `hd_hull_env_probe::a_fresnel_base_adds_an_untinted_rim_to_the_hull`
(grey, linear in the base, fifth power) and, on the disc,
`hd_zone_light_cells_ground_truth::the_zone_hulls_paint_binds_its_normal_map_and_its_chunks_carry_the_tangent`
(`check_fresnel_base`: only `zoneship01` carries one, `0.1234`).

`hd/zone-vineta-k-z5`, re-rendered on a copy, mean rgb of each box (`theirs`, then ours):

| box (`x0..x1, y0..y1`) | original | before (`6a79c7446`) | the rim |
| --- | --- | --- | --- |
| craft `780..1080, 600..880` | `(73.0, 92.6, 104.8)` | `(41.8, 66.4, 84.6)`, grey 33.12 | `(74.6, 95.5, 109.1)`, grey **15.07** |
| left pod `800..880, 640..780` | `(88.6, 98.7, 111.2)` | `(44.4, 60.9, 82.4)`, 44.05 | `(98.7, 110.3, 124.5)`, **16.95** |
| right pod `960..1040, 640..780` | `(77.8, 102.2, 111.8)` | `(45.4, 75.4, 92.6)`, 35.84 | `(82.5, 108.5, 121.2)`, **16.97** |
| left pod top `845..875, 650..700` | `(106, 109, 116)` | `(32, 41, 58)` | `(106, 110, 117)` |
| pair mean grey, rmse | | 7.27, 0.083 | **6.57**, 0.077 |

Pixel for pixel against a build of `6a79c7446`: `hd/vineta-k` (3.21),
`omega/zone-vineta-k-z5`, `omega/zone-capital-reach-2048-z5`,
`2048-vita/zone-dlc-vineta-k-z5` and `2048-vita/zone-altima-z5` differ in 0 pixels.

## Still open: three panels overshoot

The flat panels match; the left pod box overshoots by `(10.1, 11.6, 13.3)` because of
panels inside it that are too bright in ours:

| panel (`x0..x1, y0..y1`) | original | before | the rim |
| --- | --- | --- | --- |
| nose `940..972, 715..775` | `(81, 87, 102)` | `(96, 117, 159)` | `(130, 146, 183)` |
| left flap `828..850, 728..750` | `(86, 90, 100)` | `(37, 46, 63)` | `(104, 108, 117)` |
| left pod's outer side `795..820, 655..700` | `(54, 74, 86)` | `(42, 68, 88)` | `(67, 90, 107)` |

The nose was already too bright and too blue **before** the rim: that excess is tinted
like the probe terms (`0.7 : 1 : 2`), about five times the original's, so it is ours'
reflection on that panel, not the rim. The flap takes more rim in ours than in the
original (about `0.24` against `0.15` before the halving, read off the linear pixels;
the post chain is in that estimate). Both point at the same input, the direction of `N`
on those panels: the nose and the flaps are the tilted ones, the pod tops are flat and
match. Not read. Next, in order:

1. The live vertex streams of draws 641..644 (`va-0641-*.bin` and on, `--small 2500`
   dumps every hull draw's arrays): decode normal and tangent (`0x1244`, `0x1223`
   formats) and compare `N` on the nose with ours' `hull_normal` for the same vertices.
2. Whether the flaps (the airbrakes) carry their own node matrix in the frame: a normal
   left in model space on a rotated node would tilt exactly those panels.

**On the countdown grab (2026-11-27, `hd-zone-start`, ours at tick 238).** The same
boxes of `hd/zone-vineta-k-z5`: nose original `(77.6, 83.4, 96.8)`, ours
`(123.6, 139.0, 174.2)`; left flap `(82.9, 87.1, 98.3)` / `(104.8, 108.9, 119.4)`;
outer side `(58.2, 80.0, 92.0)` / `(70.4, 95.5, 113.0)`; left pod
`(87.3, 98.0, 111.0)` / `(97.7, 110.1, 124.9)`. Read side by side at 3x, **the
left-flap box lies on a flap the original holds raised on the grid** and ours draws
flush, so that box compares two different surfaces and says nothing about light
until the grid flaps are drawn. The nose and the outer side are the same shapes on
both sides. No vertex dump of draws 641..644 survives, so read 1 needs a new
capture (recipe under "Reproduce").

## 2048 and Omega

The probe tint (first section):

- **Omega: checked, differs.** Its hull ambient is a spherical-harmonic probe
  (`track.shprobes`, `oag_raceplay::sh_probes`), not the paraboloid map, so the
  hull's ambient never reaches `probe_tint`, and Omega carries no `ZoneFrame`. Where its
  Zone hull's light comes from is unread. `omega/zone-vineta-k-z5` and `omega/zone-capital-reach-2048-z5` render
  pixel-identical to the base build.
- **2048: checked, differs.** No `Track.Base Colour Highlight` key is in its table
  (`ZoneMode2048.effectSettings` reauthors the vocabulary,
  [effectsettings.md](../../../formats/effectsettings.md)) and 2048 carries no `ZoneFrame`;
  `2048-vita/zone-dlc-vineta-k-z5` renders pixel-identical to the base build.

The rim term:

- **Omega: checked, applies, not wired.** `Data/materials/Ships/zoneship01.rcsmaterial`
  (`data00.psarc`) declares `fresnelBase`, and `hdships/zone/ship.rcsmodel`
  (`data01.psarc`) authors it on `zoneship01` as `0.12269939` (reflectivity `0.7`,
  specular power `100`; `psp2_material_dump`). Its GCN program is unread, so the
  arithmetic is HD's by inheritance of the material, not by a read; ours' Omega hull
  does not go through `probe_lit` and draws no rim. Frames identical to the base build.
- **2048: checked, applies, not wired.** `Materials/Ships/ZoneShip01.rcsmaterial`
  (`data.psarc`) declares `fresnelBase` and `hdships/Zone/ship.rcsmodel` authors `0.1234`
  on it, HD's value. Its GXP program is unread and ours draws no rim there. Frames
  identical to the base build.

## Reproduce

```sh
scripts/emu-env.sh <lane> 94 23494 ; source data/scratch/<lane>/env.sh
export OAG_COMPARE_PLATFORM=hd-fury HOOK_TEX_RE='0\.69804'   # full dumps of the hull's textures
uv run --with evdev --with pillow python3 scripts/emu-run.py --bg --total 0 \
    --status $W/zone/run.json --child-status $W/zone/logs/status.json -- \
    python3 -u scripts/rpcs3-hd-zone-frame.py --small 2500 \
    --image <abs>/hdfury-ps3-eu-dec.iso --out $W/zone
python3 scripts/rsx-draw-list.py --samplers $W/zone frame       # units 4/5 of the hull draws
python3 scripts/rsx-fp-match-material.py $W/zone frame <ships/*.rcsmaterial> -- <fp...>
python3 scripts/ps3-fp-live.py $W/zone/frame-c<fp>.bin           # the tint constant

ELF=data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf
llvm-objdump -d --mcpu=pwr6 --start-address=0x3db1c0 --stop-address=0x3db268 $ELF
```

The settings block dump (`mem-00c489f0.bin`) holds the tint at offset `0x510`.

The rim term, static and live:

```sh
just psarc cat <image>:PS3_GAME/USRDIR/DATA02.PSARC /data/materials/ships/zoneship01.rcsmaterial > zoneship01.rcsmaterial
python3 scripts/ps3-microcode.py fp-file zoneship01.rcsmaterial 17
cargo run -p oag-rcs --example hd_material_dump -- /data/ships/zone/ship.rcsmodel <USRDIR>/DATA02.PSARC
python3 scripts/rsx-fp-match-material.py $W/zone frame <ships/*.rcsmaterial> -- 1117dc0 1116400
python3 scripts/ps3-fp-live.py $W/zone/frame-c1117dc0.bin    # {0.1234, 0, 0, 0} at @0x38 and @0x3c
```
