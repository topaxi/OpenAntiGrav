# HD/Fury's Zone frame: the light, the bloom block and the resolve's feedback

`EBOOT.elf` (Wipeout HD/Fury, PS3), image base `0x00000000`. Read 2026-11-19
(`zone-grade-sky`): static disassembly, then two RPCS3 cold boots into a
Racebox Zone race on Vineta K (stage 1, Sub Venom), paused in the countdown.

**The question.** The pair `hd/zone-vineta-k-z5` read 28.77 mean grey with
three things unexplained: ours' sky about 17 % brighter than the original's,
the original's scenery whiter (ours 15 to 22 grey darker outside the sky, the
buildings grey-brown with their window texture), and the dome's band flatter
in the frame than `tan(e) / 0.75` ([zone-sky.md](zone-sky.md)).

**The answer.** `Scene_PrepareFrame`'s Zone arm does three things every frame
of a Zone race that no other race does, and ours did none of them:

| what | where | law | confidence |
| --- | --- | --- | --- |
| the light of every draw | `stb 0, 0x4b0(r22)` at `0x003ad8fc` | no lightmap, no baked vertex light: `light = Constant Ambient Colour + sat(N.L) * Sun colour` of the stage | 90 |
| the `HDR and Bloom` block | `0x003ad900`..`0x003ad980`, `0x003aec34` | written from the table at `0x008c2ae0`, over the circuit's own values | 92 |
| the resolve | `Bloom feedback` `-0.4` at `0x003ad908` | `lerp(scene, 2 * feedbackBuffer, -0.4)`: `0.6` of a flat surface's light | 92 |

The dome's band was never flat: the scene target shows `tan(e) / 0.75`, and
the resolve's `0.6` is what made the final frame read flatter (below).

## The Zone arm of `Scene_PrepareFrame`

The arm is taken when `g_ZoneEffectsActive` (`0x00d45f84`) is set (`lbz r9,
0(r17)` at `0x003abac8`, `bf` at `0x003abae8`). `r22` is the settings block
`EnvSettings_GetOrCreate` returns (`0x00c489f0` live), `r20` the table
`g_ZoneBloomTable`, TOC slot `0x008b7148` -> `0x008c2ae0`:

```text
003ad8e8  bl   0x3da540              ; Environment_UpdateStageBlend
003ad8f0  lwz  r20, -0x627c(r2)      ; -> 0x008c2ae0
003ad8f4  lwz  r24, -0x6178(r2)      ; -> 0x009384e1
003ad8fc  stb  r0(=0), 0x4b0(r22)    ; lightmap and baked-vertex sources off
003ad900  lfs  f13, 4(r20)
003ad908  stfs f13, 0x54c(r22)       ; Bloom feedback          = -0.4
003ad90c  lfs  f0, 8(r20)
003ad910  stfs f0, 0x554(r22)        ; Bloom horizontal size   = 1.0
003ad914  stb  r0(=1), 0x558(r22)    ; Radial bloom Enabled    = 1
003ad918  lfs  f13, 12(r20)
003ad920  stfs f13, 0x550(r22)       ; Bloom vertical size     = 1.0
003ad924  lfs  f0, 16(r20)
003ad92c  stfs f0, 0x534(r22)        ; Bloom from frame exponent     = 0.1
003ad930  lfs  f13, 20(r20)
003ad934  stfs f13, 0x538(r22)       ; Bloom from alpha contribution = 1.0
003ad938  bf   30, 0x3ad954          ; *0x009384e1 != 0: the common tail
003ad940  lwz  r0, 0xe0(r9)          ; the mode id
003ad944  cmpwi r0, 6 / bt 0x3aec34  ; Zone
003ad94c  cmpwi r0, 14 / bt 0x3aebb0 ; Detonator
003ad954  stfs T[0x1c], 0x530(r22)   ; Bloom from frame contribution = 0.5
003ad960  stfs T[0x18], 0x53c(r22)   ; Bloom adaption boost          = 5.0
003ad968  stfs T[0x2c], 0x560(r22)   ; Radial bloom rgb contribution = 0.1
003ad978  stfs T[0x30], 0x55c(r22)   ; Radial bloom rgb exponent     = 0.1
003ad980  stfs T[0x34], 0x564(r22)   ; Radial bloom distance scale   = 0.6
```

The Zone mode id's own arm (`0x003aec34`) writes the same `0x53c` and `0x530`
and then chooses the radial contribution (`0` or the table's `+0x20`, `0.04`,
on a byte of the player's craft; `0.04` live). The Detonator arm (`0x003aebb0`)
writes `0x560` from `+0x28` and **skips** the frame contribution and the
adaption boost, which stay the circuit's there. The ordinary arm writes the
feedback back: `stw r11(=0), 0x54c(r22)` at `0x003abb0c`, beside `stb 1,
0x4b0(r22)` at `0x003abaf8`.

The field offsets are the registrar's (`Environment_RegisterLightingSchema`,
renderer.md, "The chain runner, and every engine-fed parameter"). The table,
read off the ELF's `.data`:

| `0x008c2ae0 +` | value | goes to |
| --- | ---: | --- |
| `0x04` | `-0.4` | `+0x54c` `Bloom feedback` |
| `0x08` | `1.0` | `+0x554` `Bloom horizontal size` |
| `0x0c` | `1.0` | `+0x550` `Bloom vertical size` |
| `0x10` | `0.1` | `+0x534` `Bloom from frame exponent` |
| `0x14` | `1.0` | `+0x538` `Bloom from alpha contribution` |
| `0x18` | `5.0` | `+0x53c` `Bloom adaption boost` |
| `0x1c` | `0.5` | `+0x530` `Bloom from frame contribution` |
| `0x20` | `0.04` | `+0x560` radial contribution (Zone) |
| `0x28`, `0x2c` | `0.1`, `0.1` | `+0x560` (Detonator, the common tail) |
| `0x30`, `0x34` | `0.1`, `0.6` | `+0x55c`, `+0x564` radial exponent, distance scale |

These are `zone_1/track.envsettings`' own `HDR and Bloom` values (boost 5,
sizes 1.0, radial 0.04 / 0.1 / 0.6, enabled), compiled in, plus the feedback,
which no file authors.

### Read live

`scripts/rpcs3-hd-zone-frame.py`, the settings block at `0x00c489f0`, through
`/proc/<pid>/mem` every half second from the grid to 20 s after the release
(61 samples: 6 in phase 0, 16 in the countdown, 39 racing):

| field | circuit's file (Vineta K) | live, every sample |
| --- | ---: | ---: |
| `+0x52c` adaption rate | 0.025 | 0.025 |
| `+0x530` frame contribution | 0.5 | 0.5 |
| `+0x534` frame exponent | 0.1 | 0.1 |
| `+0x538` alpha contribution | 1.0 | 1.0 |
| `+0x53c` adaption boost | **6.709** | **5.0** |
| `+0x540`/`+0x544`/`+0x548` tone boost, clamp, maximum | 20, carried 3, 4 | 20, 3, 4 |
| `+0x54c` feedback | not authored | **-0.4** |
| `+0x550`/`+0x554` sizes | **0.5, 0.5** | **1.0, 1.0** |
| `+0x558` radial enabled | 0 | 1 |
| `+0x55c`/`+0x560`/`+0x564` radial | 0.1, 0.1, 0.6 | 0.1, 0.04, 0.6 |
| `+0x4b0` light-source byte | | 0 |

The exposure `maximum - min(adapted * boost, clamp)` is `4 - 3 = 1.0`
throughout (`adapted` 0.30 to 0.50), so the exposure is not what dims the
frame. The gate's patched constant in the same pause is `0.279453`, which is
`0.5 * (1 - min(0.353 * 5.0 * 0.25, 1))` with the live `adapted` of `0.3532`:
the fade runs on the Zone boost `5.0`, not the circuit's `6.709`.

## The feedback term

The resolve (`downsamplescaleaddfeedback_fp`, renderer.md, "The exposure is
read") is

```text
feedback = lerp(scene, 2 * feedbackBuffer, feedbackBuffer.a * scaleFeedback)
out      = saturate(feedback * scale + bloom * scaleAdd + tint)
```

and `Render_FinishFrame` binds `scaleFeedback = [block + 0x54c] + *0x00c51130`
(`lfs f13, 0x54c(r24)`, `lfs f3, 0(r9)`, `fadds f3, f13, f3` at
`0x003e37c0`..`0x003e37e8`; `r9` is TOC slot `0x008b7ce4` -> `0x00c51130`,
`g_FunkLayer + 0x250`). The added float read `0.0` in every sample. So a Zone
frame resolves `1.4 * scene - 0.8 * feedbackBuffer`.

**The feedback buffer is the quarter-resolution scene.** In the paused frame
the chain's draws on the scene target `0x01920000` (1300x1080 at that pause's
dynamic width) are followed by: the half copy (`0x02e60000`), the quarter copy
(`0x03140000`), a copy of that into `0x03280000` with the alpha write off, a
blended draw of `0x031e0000` over it (the zoom ring's history draw,
[funklayer-zoom.md](funklayer-zoom.md): weight `0.25 E`, zero outside a boost
or damage pulse), the adaptation ladder, the gate into `0x030a0000` and the two
blurs. The final draw binds `0x030a0000` and `0x03280000` beside the scene.
`0x03280000` read back equal to the scene at every probe and with alpha 255
on every texel, so the lerp weight is `scaleFeedback` alone.

**Measured.** With the scene target, the blurred bloom and the presented frame
of one pause, in linear light (sRGB decode of each byte):

| probe | scene (G) | bloom (G) | final (G) | `(final - bloom) / scene` |
| --- | ---: | ---: | ---: | ---: |
| sky | 135 | 71 | 126 | 0.601 |
| building | 215 | 127 | 207 | 0.607 |
| road | 77 | 35 | 70 | 0.598 |

A least-squares fit over the whole frame (unmasked, unclipped pixels) gives
`final = 0.61 * scene + 1.0 * bloom`, 3.2 bytes mean error; the same fit in
the encoded domain gives inconsistent scales (0.37 to 0.46). On a flat
surface `1.4 - 0.8 = 0.6`; across an edge the term is an unsharp mask, the
dark seams the original's Zone frame shows beside its bright panels.

## The Zone surface's light

`EnvSettings + 0x4b0` gates the chunk's ambient source bits (`IleVertex`,
`IleLightmap`; [zone-plain-chunks.md](zone-plain-chunks.md), "The lightmap
term is off in a Zone frame"), and the Zone arm clears it. Every draw of a
Zone frame is then the `Ambient` permutation, whose Zone program is `colour =
light * surface + glow` with `light = constantAmbientColour + sat(N.L) *
dirLight0Colour` ([zone-shader.md](zone-shader.md)).

- **Census.** All 1,442 `.rcsmaterial` files with a circuit-shape Zone block
  have a Zone block that declares `constantAmbientColour`, the families whose
  plain program is fed no scene light (the signs, the glow tubes) included
  (`crates/rcs/tests/zone_shader_census_ground_truth.rs`,
  `every_zone_material_has_an_ambient_lit_zone_block`). 7,772 of the 20,084
  blocks do not declare it: the other light-source permutations, which a Zone
  frame never selects.
- **Live.** The scene target reads `(192, 216, 223)` on Vineta K's buildings.
  `0.5 * 1.5 * Scene.Texture Colour (0.7216, 0.9098, 0.9647)` encodes to
  `(194, 215, 221)` at a white texel, within 2 on every channel: the stage's
  `Constant Ambient Colour` `1.5` and the programs' `/2`. Ours lit the same surface by the circuit's lightmap
  and baked vertex colours and read `(105, 126, 128)` there.

## The dome's band, re-read on the scene target

[zone-sky.md](zone-sky.md) left the band's shape at its static 82 because the
final frame rose less toward the zenith than `tan(e) / 0.75`. The scene target
does not: at the frame's top centre (24 degrees up) it reads `(7, 138, 164)`
where the law predicts about green 138 and blue 166 from Sub Venom's bytes, and
`(5, 135, 156)` at the top left. The final frame's `(6, 129, 153)` and
`(4, 126, 146)` are those values through the resolve above. Confidence 90 for
the band law (the static read and the live scene target at three probes).

## The port

- **Light**: `oag_title::ZoneFrame::lit_by_stage_alone`, carried to the mesh
  Zone uniform's `shape.w` by `ZoneGrade::with_stage_light`; `shade.wesl` gives
  a chunk on its Zone program `scene.light.ambient + sun * N.L`. Test:
  `crates/mesh/tests/zone_recolour.rs`,
  `a_zone_surface_is_lit_by_the_stage_ambient_and_no_lightmap`.
- **Bloom block**: `oag_hd::race::ZONE_FRAME`, applied by
  `oag_raceplay`'s `load::environment::bloom::for_mode` in `Mode::Zone`. Test:
  `a_zone_race_runs_the_titles_own_bloom_block`.
- **Feedback**: `oag_post::hd_bloom::Params::feedback`, `fs_encode` in
  `hd_bloom.wesl`; the vertical blur has its own quarter target so the
  quarter-res scene survives to the resolve. Test:
  `a_negative_feedback_darkens_a_flat_scene_to_its_share`.

`hd/zone-vineta-k-z5`, mean grey difference, re-rendered on a copy of the
pair:

| build | mean grey | RMSE |
| --- | ---: | ---: |
| before | 28.77 | 0.165 |
| the stage light alone | 17.04 | 0.104 |
| plus the bloom block and the feedback | **7.42** | 0.085 |

By region after: sky 0.10 (ours `(6, 126, 148)`, theirs `(6, 127, 149)`),
scenery 1.93 (bias -0.2), road and pads 1.50 (bias -1.8), the gantry board
1.32, the craft 1.92. Every other HD race pair (the thirteen circuits,
`talons-assegai-c1`, `hd-race/talons-pack`) is pixel-identical before and
after.

## Open

- **A pulse in a Zone race.** The feedback buffer is also the zoom ring's
  history target; ours resolves against the frame's own quarter-res scene, so
  a damage pulse's history is not in the feedback term. Unmeasured.
- **Radial bloom.** Enabled in a Zone race with contribution `0.04`; this
  port has no radial pass (renderer.md, "Is HD's glare under-reproduced?"; the
  block's `+0x558`..`+0x570`).
- **Detonator.** Its arm keeps the circuit's frame contribution and adaption
  boost and takes the rest; static only, and `ZONE_FRAME` is applied in Zone
  alone. Zone Battle (mode 13) takes the common tail, the Zone values but
  radial contribution `0.1`; not wired.
- **The hull.** The original's Zone hull reads `(60, 85, 98)` over the craft
  box against ours' `(33, 56, 62)`. Not a Zone-surface chunk; which light the
  hull's programs take in a Zone frame is unread (the same `+0x4b0` byte is
  the first address to check against the hull's ambient source).
- **`*0x009384e1`**, the byte that sends modes 6 and 14 to their own arms:
  read `0` live here and in [eliminator-flip.md](eliminator-flip.md); its
  meaning is still unread.

## 2048 and Omega

- **The bloom block and the feedback: checked, differs.** Neither executable
  (`eboot.bin` of Omega, `eboot.elf` of 2048) holds an `HDR and Bloom` key, a
  `scaleFeedback` parameter or a `downsamplescaleaddfeedback` program name;
  both name `wo_composite_zone_fp`, and Omega `wo_composite_zone_hdfury_fp`
  besides, as the Zone frame's composite. Those programs' Zone arithmetic is
  unread ([effectsettings.md](../../../formats/effectsettings.md), "on 2048
  the original grades the whole frame"), so both titles carry no `ZoneFrame`.
- **The Zone surface's light: not checkable here.** Their Zone surfaces are
  drawn by programs of their own (GXP, GNM), neither title authors
  `Lighting.*` keys per stage on its own table, and a live value needs a
  memory readout Omega has none of. Their frames are unchanged:
  `omega/zone-vineta-k-z5` and `2048-vita/zone-dlc-vineta-k-z5` render
  pixel-identical to the build before, and the two pairs on 2048's own
  circuits keep their scores (69.25, 112.95).

## Reproduce

```sh
scripts/emu-env.sh <lane> 94 23494 ; source data/scratch/<lane>/env.sh
export OAG_COMPARE_PLATFORM=hd-fury
uv run --with evdev --with pillow python3 scripts/emu-run.py --bg --total 0 \
    --status $W/zone/run.json --child-status $W/zone/logs/status.json -- \
    python3 -u scripts/rpcs3-hd-zone-frame.py --after-go 20 \
    --image <abs>/hdfury-ps3-eu-dec.iso --out $W/zone
python3 scripts/emu-run.py wait $W/zone/run.json
python3 scripts/rsx-draw-list.py $W/zone frame            # the frame's passes

ELF=data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf
llvm-objdump -d --mcpu=pwr6 --start-address=0x3ad8e0 --stop-address=0x3ad9b0 $ELF
llvm-objdump -d --mcpu=pwr6 --start-address=0x3aec34 --stop-address=0x3aeca0 $ELF
llvm-objdump -d --mcpu=pwr6 --start-address=0x3e37a4 --stop-address=0x3e3834 $ELF
python3 scripts/ps3-toc.py resolve 0x003e3268 9d84        # -> 0x008c2ae0
python3 scripts/ps3-toc.py resolve 0x003e3268 a920        # -> 0x00c51130
```

The walk presses RIGHT four times on RACE TYPE (Single Race, Time Trial,
Speed Lap, Eliminator, Zone) and repeats a press until the settled box
changes; with a fixed count and no check, four boots of six landed on another
race type (a press is dropped, or a mid-slide grab counts as a change). A pause can land inside the final pass: the screen
buffer of that frame then reads black, and the presented frame of the same
pause is the picture to measure against.

## Names applied

- `g_ZoneBloomTable` (`0x008c2ae0`, data, 90) - the floats `Scene_PrepareFrame`
  copies into the settings block's `HDR and Bloom` fields every frame of a
  Zone or Detonator race: `Bloom feedback` `-0.4`, both sizes `1.0`, the
  frame exponent `0.1`, the alpha contribution `1.0`, the adaption boost
  `5.0`, the frame contribution `0.5` and the radial set. Read at instruction
  level and matched against the live block in 61 samples.
