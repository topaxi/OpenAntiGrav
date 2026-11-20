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
neutral and moves the box 38.56 -> 33.58; the hull is still about half as bright as
the original's in red, and that term is not found (see "Still open").

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
`hd-race/talons-pack`) keeps its score; the five checked pixel by pixel against the
base build (`anulpha-pass`, `modesto-heights`, and the three Zone pairs below) differ in
0 pixels.

## Still open: the hull is about half as bright

After the tint the hull's red is `44` against the original's `88` (left pod box).
Ruled out: the probe data (above, the same entry), the SPU light (zero on most chunks),
the sun (0), the tint (read). Not the probe decode either: on the non-Zone pair
`hd/vineta-k` (same `.probes`, tint `(4, 4, 4, 0)`) ours' hull box reads `(96.6, 92.5,
82.1)` against the original's `(93.0, 89.5, 79.4)`.

A diagnostic build that fed the hull a flat probe of luma `0.293` (the live map's raw
mean, as if undecoded) matched red to 0.4 and overshot blue by about 30: a one-channel
fit, not a law, and not wired. Next reads, in order:

1. Unit 0's binding (`0x0a780100`): match its 64-byte heads against
   `zoneship_faisar/team.gtf` and `zone_tp_1024.gtf`, as
   [hd-ship-materials.md](../../../rendering/hd-ship-materials.md) "Finding 5" did; a
   different livery or a brighter albedo map would explain a uniform gap.
2. The live hull program's tail past `@0x5f` (the dump stops there): the reflection
   and specular adds, with this pause's constants.
3. The scene target under the hull (`target-01920000.bin`) against ours' scene before
   the resolve, to split the gap between the hull term and the post chain.

## 2048 and Omega

- **Omega: checked, differs.** Its hull ambient is a spherical-harmonic probe
  (`track.shprobes`, `oag_raceplay::sh_probes`), not the paraboloid map, so the
  hull's ambient never reaches `probe_tint`, and Omega carries no `ZoneFrame`. Where its
  Zone hull's light comes from is unread. `omega/zone-vineta-k-z5` and `omega/zone-capital-reach-2048-z5` render
  pixel-identical to the base build.
- **2048: checked, differs.** No `.probes` ships and no `Track.Base Colour Highlight`
  key is in its table (`ZoneMode2048.effectSettings` reauthors the vocabulary);
  `2048-vita/zone-dlc-vineta-k-z5` renders pixel-identical to the base build.

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
