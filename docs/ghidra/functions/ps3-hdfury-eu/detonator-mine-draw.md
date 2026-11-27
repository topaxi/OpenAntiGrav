# Detonator: the matrices a laid mine is drawn with, and why the near mines looked displaced

`EBOOT.elf` (WipEout HD Fury, PS3, EU, `BCES00664`), Ghidra program `/hdfury/EBOOT-ps3-hdfury-eu.elf`. Read 2026-11-24
(`det-mine-look-re`). Bodies read off `llvm-objdump -d --mattr=+altivec,+vsx` over their real extent; TOC slots through
`scripts/ps3-toc.py resolve` (every function here declares TOC `0x008ad4d8`). The live legs are the push-buffer dumps and
frames of two `det-mines` grabs of one boot (RPCS3, Talon's Junction, Feisar, stage 1, state
`hd-detonator-talons-feisar-grid-late`; shot A at lap clock 2.70 s, push-buffer eye `199.27, -41.48, -194.98`; shot B a few
ticks later, eye `206.47, -41.15, -194.76`) and the three Detonator pairs' `theirs.png`. The mine's objects are
[detonator-mine.md](detonator-mine.md), its shader constants [detonator-mine-material.md](detonator-mine-material.md).

## Result

| Part | Law | Where | Confidence |
| --- | --- | --- | ---: |
| Football matrix | rows `R(axis m+0xe0, angle m+0x150) * scale`, translation `m+0x80` (the object's position, unmodified) | `DetonatorMine_AnimateToMusic` `0x001396f0`-`0x001399a4` | 78 |
| Spikes matrix | the same rotation and translation, rows `* spikeScale` | `0x001399bc`-`0x00139a78` | 78 |
| Football size | `scale = clamp(2.3 s, 1, 2.3)`, model bounding radius `1.00` | `detonator-mine-material.md`; oag-view | 85 (law), 80 (radius) |
| Spikes size | `spikeScale = clamp(4 t, 1, scale + 0.95)`, model bounding radius `1.04` | same | 85, 80 |
| Dark mine | football and spikes at the rotation, rows unscaled (`scale` and `spikeScale` not applied) | `0x00139b78`-`0x00139cec` | 72 |
| Visualiser | a **camera-facing** basis at `m+0x80`, rows `* (1 + 0.4 (m+0xd8 - 1))` live, `* 0.9` dark; model bounding radius `2.45` | `DetonatorMine_PoseVisualiser` `0x00139308` | 65 |
| Near-mine displacement | **not the draw**: the RPCS3 picture is 1.5 to 4 ticks older than the push buffer's camera the pair renders from | "The displacement", below | 70 |

## `SceneNode_SetLocalMatrix(node, m, flag)` (`0x00327500`)

```text
if node+0x80 != m: copy the four rows of m to node+0x80..+0xbf           ; 0x00327500-0x0032754c
node+0x34 = (node+0x34 & 0x0fffffff) | (flag == 0 ? 0x80000000 : 0x90000000) ; 0x00327550-0x0032757c (lis -1792, addis 2048)
if node+0x34 & 0x1000 was clear: node+0x34 |= 0x1000;
    for c = node+0x14; c; c = c+0x10: 0x00324bc0(c, 0x1000)               ; mark every child dirty
```

Every mine part reaches the renderer through this call: the matrix handed in is the node's local matrix; nothing on the
way adds a view-dependent term. Confidence **72** (the body is read end to end; `0x00324bc0` is read only as "set this flag
on a child"; what the `0x80000000`/`0x90000000` bits select is not read).

## The football and the spikes (`DetonatorMine_AnimateToMusic`, `0x001395f0`)

```text
M := copy of the object's rows m+0x50, +0x60, +0x70 and position m+0x80      ; 0x00139624-0x00139694
phase m+0x150 = wrap(m+0xc0 * K)                                             ; 0x0013969c-0x001396ec
0x006772d8(&M, axis = m+0xe0 (v2), angle = phase (f1))                       ; 0x001396f0-0x00139710
rows of M &= mask (w lane cleared, TOC -0x2bd0); translation = m+0x80 (v31)  ; 0x00139718-0x00139770
v30 = 0                                                                      ; vxor at 0x001398fc
football rows = M.rows * scale + v30  -> SceneNode_SetLocalMatrix(m+0xcc, &M) ; 0x00139938-0x001399a4
spikes   rows = M.rows * spikeScale   -> SceneNode_SetLocalMatrix(m+0xd0, &M) ; 0x00139a20-0x00139a78
```

`0x006772d8` is a TOC stub (`r2 += 0xfeec`, then `b 0x006ca538`). `0x006ca538` multiplies `f1` by a TOC constant, takes
`Math_CosPoly` (`0x00390c68`) and `Math_SinPoly` (`0x00390ca0`) of it and builds the axis-angle terms `x*x*(1-c)+c`,
`x*y*(1-c) - z*s` and so on from the three lanes of `v2` into `r3`'s rows. Read as "rotation matrix from an axis and an
angle", confidence 60, so not renamed. Whether it writes all three rows (replacing the object's basis) or multiplies
into them is the open part: the stores after `0x006ca63c` were not read. Either way **the translation of both parts is
`m+0x80` exactly**, and the only scale is the uniform `scale` / `spikeScale` on the three rows. Confidence **78**.

The dark branch (`0x00139b78`) builds the same rotation (`0x00139bf8`) and submits the football and the spikes
(`0x00139ccc`, `0x00139cec`, the models at `m+0xcc` and `m+0xd0`) with unscaled rows. **The visualiser (`m+0xd4`) is not
submitted by this function on either branch**, contrary to the reading on
[detonator-mine-material.md](detonator-mine-material.md) ("Both models and the visualiser are re-submitted").

## The visualiser: `DetonatorMine_PoseVisualiser` (`0x00139308`)

`DetonatorMine`'s vtable `0x008642f0` slot `+0x14` holds the OPD `0x00876cb0`, whose code is `0x00139308` (one of the
shared scene-object slots, so the scene calls it, not the mine manager). It loads the mine's rows and position, a
pointer behind TOC `-0x2b8c` (`0x00987884`, a `.bss` word: the current view, read as the camera) and its `+0x70`, then
builds an orthonormal basis from the direction between the camera and `m+0x80` (two `vrsqrtefp` normalisations, two
cross products by `vperm`/`vmaddfp`/`vnmsubfp`, `0x001393bc`-`0x001394ac`), keeping the translation. Then:

```text
if m+0x160 (live):  rows *= (m+0xd8 - K[0]) * K[4] + K[0]  = 1 + 0.4 (footballScale - 1)   ; 0x00139568-0x001395d0
elif byte K[8] == 0: rows unscaled                                                       ; 0x001394d4-0x001394e0
else:               rows *= K[12] = 0.9                                                  ; 0x00139500-0x00139548
SceneNode_SetLocalMatrix(m+0xd4, &M, 0)
```

`K` is the mine's tuning block at `0x008c19dc` (TOC `-0x2b78`), the same block
[detonator-mine-material.md](detonator-mine-material.md) reads its envelope constants from: `K[0] = 1.0`, `K[4] = 0.4`,
byte `K[8] = 1`, `K[12] = 0.9` in the shipped `.data`. So **the visualiser is a billboard that faces the camera, centred
on the mine, `1` to `1.52` times its model size while live (`1.52` at `scale = 2.3`) and `0.9` dark**. Its model,
`Electric_Visualiser`, has a bounding radius of `2.45` (oag-view), more than twice the football's. Confidence **65**:
the basis is read as camera-facing from its shape (normalise, two crosses) and the pointer, not from a frame;
`0x00987884+0x70` being the eye is not read to its writer.

This is the violet ring that circles every lit mine in the original's frames, a circle at any angle. Ours does not load it
(`oag_raceplay::load::weapon_models::load_detonator_objects`: "electric_visualiser is music-driven ... and is not drawn").

## The displacement

`det-mines` measured the original's near mines drawn about 30 % closer to the screen centre than their RAM objects
project with the pair's camera ([detonator-mine.md](detonator-mine.md), "Where a mine stands"). **The draw does not
displace them** (above: translation `m+0x80`, no view term). **The camera is from a later frame than the picture.**

1. **No mine matrix is in the dumped regions.** The pair's camera comes from the push buffer
   (`rpcs3-drive.py`'s `PUSHBUFFER_REGIONS`: `0x40010000` 16 KiB, `0x40060000` and `0x40080000` 128 KiB each). A walk of
   their `TRANSFORM_CONSTANT_LOAD` packets (`0x1efc`, both emitters) in shot A finds 369 draws,
   `c[256]`/`c[260]` loads that decompose to the camera, the hull (scale `0.63`, five translations), HUD and screen
   quads, and **no matrix whose translation is within 3 units of any of the 80 mines** (also as a 16-float window and as
   `M * viewProj^-1`). The world's draws (track, scenery, mines) are elsewhere in the command stream, so the frame the
   camera belongs to is not shown by these regions. Clean negative; live recipe for the mine's own matrix: a PPU
   breakpoint at `0x001399a4` reads 64 bytes at `r4` (the football's matrix as it is submitted).
2. **Two shots of one boot disagree by the camera's travel.** Shot B's push-buffer eye is 4 ticks on from shot A's, yet
   B's picture fits **A's** push-buffer camera moved back only 1.15 ticks (centroid fit below, rms 20 px; 0 by eye), so
   B's picture is about 3 ticks behind its own push buffer, A's 2.15. The same RAM objects, the same draw: only the
   camera moved, by a different lag on each shot.
3. **A camera moved back along the craft's velocity fits every pair.** Projecting the RAM objects with the push-buffer
   `viewProj` shifted by `k` ticks of the craft's velocity, against the centroids of the original's lavender pixels
   (`b > 180`, `r > 150`, `r - g > 12`) in a window scaled by distance:

   | Pair | Near mines | Best `k` (ticks) | rms after | rms at `k = 0` |
   | --- | --- | ---: | ---: | ---: |
   | `mines-ahead` (lap 2.67 s) | 3, at 12 to 18 units | 3.55 | 11.9 px | 135 px |
   | `cannon-volley` (2.63 s) | 3, at 19 to 25 units | 2.30 | 9.4 px | 107 px |
   | `pad` (2.02 s) | 4, at 83 to 89 units | 1.55 | 1.4 px | 5 px |
   | shot A (2.70 s) | 3 | 2.15 | 11.7 px | 93 px |

   `pad` is why "a constant 7 units rejects pad" held: at 85 units a tick moves a mine about 3 px, so its lag is small
   and loosely bounded, and a fixed world offset would not depend on the grab. The lag is not a whole number because
   the camera does not move exactly along the craft's velocity.
4. **The scenery shows the same lag.** A mine-only fit cannot separate "the camera is older" from "the mine is pushed
   along the track" (a mine object even carries a velocity, `+0x90`, 100 along the track). Ours was re-rendered from
   the `mines-ahead` pair's flags with the camera and the craft moved back `k = 0..6` ticks: the left edge of the
   violet overhead panel at the top right, a feature across the line of travel 15 to 40 units away, stands at
   x `1565, 1554, 1540, 1524, 1512, 1501, 1491` for `k = 0..6` against the original's `1518`, so `k` about 3.5, the same
   as the mines, on x (its lower edge does not match on y at any `k`: ours' panel is drawn differently). Along-track
   features (the red lines, the floor dashes, the right wall's panels) do not move under this shift and cannot decide
   it, which is why `det-mines` saw the strips agree. **The rest of the scenery does not improve:** outside the near
   mines and the hull the plain mean absolute grey goes `16.67, 16.73, 17.52` at `k = 0, 2, 4`, and the edge correlation
   of the right wall peaks at `k = 0` (`0.565` against `0.396` at 4) while the upper-left structure peaks at 4 (`0.192`
   against `0.141`). The re-render moves the eye only, not its orientation, which is one candidate for why the larger
   regions do not follow; one cross-track edge is the scenery evidence, not a whole-frame fit.
5. **The size law leans the same way.** At `k = 0` the `mines-ahead` middle mine (18.2 units) shows a football of
   0.68 units against the law's minimum (`scale >= 1`, bounding radius `1.00`); at `k = 3.55` (24.5 units) 0.92. The
   visible rim is not the bounding sphere, so this is directional support, not a discriminator.

**Confidence 70** that the displacement is the camera's frame, not the draw: five grabs, the draw's own code (no view
term), and one cross-track scenery edge; held below 80 by the scenery regions that do not follow and by the earlier
measurements below. **Not reconciled with what the project measured before.** [compare-page.md](../../../tools/compare-page.md),
"The skew is the pause, not the original's draw order", found the push buffer's hull and camera to be one frame, 1 or 2
ticks behind RAM, and the pairs record the presented picture as 0 to 3 ticks older than RAM (`hd-pair-lag-scan.py`) and
`hud_clock_ticks = 3`. Here the picture is `craft_ticks + k` = 5.6 to 6.3 ticks behind RAM. The hull and camera agreement
is blind to this skew (both ride the camera). `hd-pair-lag-scan.py` scores whole-frame edge correlation with the eye
rewound and orientation kept, which this page's own regions show is dominated by along-track features that do not move
under the shift; that it is blind is likely but not shown. Which clock the HUD lap time reads at a pause is unread.

## One near mine, predicted and seen

`mines-ahead`, the middle near mine (object `217.595, -44.266, -192.347`), the pair's push-buffer camera
(`fov_y 68.10`, focal 799 px at 1080 lines) moved back `k = 3.55` ticks (`1.81` units a tick):

| Quantity | Predicted | Original's frame |
| --- | --- | --- |
| Centre | `1046, 497` (at `k = 0`: `1075, 545`) | lavender centroid `1042, 504` |
| Distance | 24.5 units, 32.7 px a unit | |
| Football radius | `33 * scale` px, `scale` in `[1, 1.24]` for the pair's football bands (`0.28` to `0.54`, band `0..3`) | about 30 px (the shell's visible edge) |
| Spike tips | `34 * spikeScale` px, `spikeScale = min(4 t, scale + 0.95)`, the spikes' bands `11..14` at `0.65` to `0.99`: 64 to 74 px | about 70 px |
| Visualiser | billboard ring, bounding `80 * (1 + 0.4 (scale - 1))` px | violet ring, about 57 px |

The ring's own radius inside its 2.45-unit bounds was not measured here (the model's arcs reach beyond it); it is now,
and ours matches it ("The ring's radius", below). Measured by eye on
a 3x crop with rings at 20 to 60 px: shell about 30 px, ring about 57 px, spike tips about 70 px. **So the original's lit
mine reads about twice ours' because of the ring and the long spikes around a football of ordinary size**, not a bigger
football (confidence 55: one mine, by eye).

## Ready to wire

| Row | What | Source | Confidence |
| --- | --- | --- | ---: |
| 1 | **Pair tooling, every HD pair:** render ours from the picture's camera, not the push buffer's: move `--camera-pose`'s eye and `--pose` back by `k` ticks of motion. `k` varies by grab (1.15 to 3.55 measured); measure it per grab (in Detonator from the near mines' centroids; elsewhere from one cross-track feature) by extending `hd-pair-lag-scan.py` with a cross-track or static-object score, and record it in the existing `original.presented_lag` block; refuse a grab whose fit does not converge. Settle the disagreement with the earlier lag measurements first | "The displacement" | 70 |
| 2 | **Wired on HD** ("The visualiser, wired"). Draw `Electric_Visualiser` at each mine: camera-facing, centred, scale `1 + 0.4 (football - 1)` live, `0.9` dark, colour `ElectricityColour` and the music binds already read | `0x00139308`, [detonator-mine-material.md](detonator-mine-material.md) | 65 |
| 3 | Football and spikes translation stays the object's position; orientation is an axis-angle spin about `m+0xe0` by `m+0x150` (ours spins about its own choice; the axis's writer is unread) | `0x001396f0` | 60 |
| 4 | Dark mine parts at scale 1 | `0x00139b78` | 72 |
| 5 | Carry each mine's `s` (`+0xf4`) and `t` (`+0xf8`) into a pair, so a pair's mine has the original's size: open since `det-mines` | [detonator-mine-material.md](detonator-mine-material.md) | 85 |

Row 1 decides how much of the `mines-ahead` and `cannon-volley` gap is real: re-rendering `mines-ahead` with
`k = 4` moved the whole-frame mean absolute grey of this tree's render (not the page scorer's number) from 22.99 to 20.74.

## The visualiser, wired (2026-11-24, `det-mine-visualiser`)

`oag_raceplay::Race::mine_visualiser_matrices` poses the ring and `oag_mesh::mesh::rcs::electricity` draws it with
`electricity.rcsmaterial`'s own program (`PROGRAM_ELECTRICITY`, `shade.wesl`); HD only.

- **The program, read** (`ps3-microcode.py fp-file`/`vp-file`, fragment and vertex block #1; instruction offsets in the
  module's doc): `rgb = overlay(uvd(uv1.x, uv1.y + 0.3 m).rgb * VertexColour1.w, ElectricityColour) * |N.V|^P`,
  `alpha = VertexColour1.w * |N.V|^P`, `m` the music-driven distortion built from three taps (`electricity_bass_treble_hits`
  unit 0, `electricity_pitch` unit 1, `electricity_uv_dist` unit 2). `P` is the material's authored `0x5cfb25e3` (1.0),
  `0x906b67ba` the engine clock. Confidence 75 (read end to end; the gamma nibbles and the blend factors are not).
- **The model**, 132 vertices, 192 triangles, an undeclared stride-26 chunk (`VertexColour1` at `+0x0a`, the pair
  `0xac7c75d0` at `+0x0e`, `Uv1_Uv2` at `+0x12`), is a flat disc in its own XY plane, radius 2.45. The lightning ring is the
  texture's, at the disc's rim.
- **Chosen, not measured:** which model axis takes the camera direction (Z, the disc's normal) and the roll about it (the
  mine's own up); the scale of ours' whole-spectrum level behind `MasterVolume` and `PitchVolume` (the clamps are the
  original's, [detonator-mine-material.md](detonator-mine-material.md), "The visualiser's level"); `PitchOffset` and `TrebleHit` stay 0 (accumulator step and `.bss` writer unread); the colour tap is decoded.
- **On the three published Detonator pairs the cell and frame barely move** (pairs as published, no picture-lag fit):
  `pad` 8.82 -> 8.82, `mines-ahead` 16.38 -> 16.40, `cannon-volley` 15.32 -> 15.33 mean grey. Ours' near mines are
  smaller and nearer the screen edge than the original's there (the picture lag the pairs do not carry), so the ring lands
  on the wrong pixels; by eye the original's ring is two smooth concentric circles, ours a ragged band at the disc's rim.
- **Omega: ported** (2026-11-25, `det-visualiser-2048-omega`). The ring is drawn by its own program on Omega's Detonator
  mines: `oag_mesh::mesh::rcs::psp2` (`electricity`) binds `electricity.rcsmaterial`'s three pictures and two glow-table
  entries and `shade.wesl`'s mode 25 draws it, HD's path unchanged. The program, read with `oag_rcs::gcn`
  (`gcn_dump <data00.psarc> Data/weapons/Materials/electricity.rcsmaterial`, pixel `Shdr @0xf84`, 139 instructions):
  two taps of `Bass_Treble_HitsTexture` (slot 2, at `Uv3` and at `uv2`), `s = uv1.y + time + n0 + n1` scaled by `Tempo`,
  one tap of `ElectricityTexture` (slot 0, alpha) minus the two noise taps, one of `PitchTexture` (slot 1) times
  `PitchVolume`, times `MasterVolume`, the colour tap at `uv1.y + 0.3 m` (the literal read off a `v_madmk_f32`), the
  overlay against `ElectricityColour` and `|N.V|^FacingPower`: **the law above, instruction for instruction**. The
  samplers are named, not numbered the same: Omega's slot 0 is HD's unit 2, slot 2 is HD's unit 0 (bound by name). The
  vertex program (`Shdr @0x984`) writes `Uv3` as the bass and treble pair, `Uv1_Uv2.xy` as `uv1`, `.zw` minus `BassHit`
  as `uv2` and `VertexColour1.w` as the alpha, as HD's chunk does. The lit variant (`@0x29a4`) differs only in the
  attribute numbering, and the shipped program ends in a fog lerp the scene's own fog replaces. **Confidence 80** (read end
  to end; no PS4 emulator has drawn it, the units' gamma is inherited from HD). Census: `electricity.rcsmaterial` in
  `data00` (`weapons/`, `weapons2048/`) and the patch's `data05` and `data08` (both folders): six files, twelve pixel
  programs, all read as this program with the shift `0.3` (`every_omega_electricity_material_reads_as_the_visualiser_program`);
  `electric_visualiser.rcsmodel` in `data00` under both folders builds with the ring bound
  (`both_omega_visualiser_models_bind_the_electricity_program`). `oag_omega::race::DETONATOR_OBJECTS` names HD's entry.
  Frame: ours on Omega's Tech De Ra, Detonator, a laid mine with its violet ring.
- **2048: ported** (2026-11-25, `det-visualiser-2048`). The ring is drawn by 2048's own USSE program on its
  Detonator mines, through the same `oag_mesh::mesh::rcs::psp2` (`electricity`) plan as Omega's; HD and Omega unchanged.
  **The program, read** (`vita-usse.py disasm <data.psarc:Weapons/Materials/electricity> 1`, fragment program #1, 48
  primary words; `vita-gxp.py dump` for its interface). It has **five** fetches, not three: the fragment input records
  name two **prefetched** texels of `Bass_Treble_Hits` (resource 2), at interpolant 2 (`pa4`, two words of `f16`) and
  interpolant 3 (`pa8..pa11`, four words of `f32`), so only three `smp` words are in the code. Uniforms are `sa` words by
  their `resource` (`time` `sa0`, `Tempo` `sa1`, `MasterVolume` `sa2`, `PitchOffset` `sa3`, `PitchVolume` `sa4`,
  `ElectricityColour` `sa6`, `FacingPower` `sa8`, `fogColour` `sa10`); the literals and the dependent samplers sit in
  container 19, whose base is `sa12`: literal records 10, 11, 12 are `sa22` (`f16` 0.5), `sa23` (`f32` 0.3) and `sa24`
  (`f16` 2.0); the dependent-sampler table (`+0x84`, four records a sampler, `resource * 4 + k` low, `sa` offset high) puts
  `Electricity` (0) at `sa14` and `Pitch` (1) at `sa18`. On 2,577 of 2,577 fragment programs of `data.psarc`'s materials that
  carry the table, every record names a declared sampler and every `smp` reads a word it lists. Word by word:

  ```text
   3  smp2d  pa8 = Pitch(uv1.x, PitchOffset)           ; 1-2: pa8 = pa12 (uv1.x), pa9 = sa3
   4  mul.f16 pa4.x = n0.r * n0.r                       ; the bass tap, squared
   9-16  N = normalize(interp 1.xyz), V = normalize(interp 0.xyz)
  17-21  q = exp2(log2|N.V| * FacingPower)              ; pa8.z
  22  mul.f16 = pit.r * pit.r                           ; the pitch tap, squared
  24  add.f32 pa6 = time + uv1.y
  25-26  s = n1.a + pa6 + n0.r^2                        ; n1 = pa11, the f32 texel's alpha, unsquared
  27  pa13 = Tempo s
  28  smp2d.f32 pa0..3 = Electricity(uv1.x, Tempo s)    ; its alpha is pa3
  30-31  d = uvd.a - n0 - n1
  32-33  m = (pit^2 PitchVolume + d) MasterVolume
  34  mad.f32 pa0 = m * sa23 (0.3) + pa6                ; uv1.y + time + 0.3 m
  35  add.f32 pa13 = -time + pa0                        ; uv1.y + 0.3 m
  36  smp2d.f16 pa0 = Electricity(uv1.x, uv1.y + 0.3 m)
  38  mul.f16 c = rgb * rgb                             ; the colour tap, squared
  39-40  c *= VertexColour1.w; pa4 = 2 E c              ; sa12 = 2 E, sa6 = 2 (1 - E) (secondary)
  41-44  over = c - 0.5 < 0 ? 2 E c : 1 - 2 (1 - E)(1 - c)
  45-46  rgb = fog + fogvis (over q - fog)              ; the scene's own fog replaces this
  47  alpha = VertexColour1.w q
  ```

  **HD's law instruction for instruction, with the three colour-bearing taps squared in code** (the bass tap's red, the
  pitch tap's red, the colour tap's rgb; the two alpha taps are not), where HD leaves the decode to its units' gamma
  nibbles. The vertex program (#0, 35 words) hands over interpolant 0 = `(eye - position, VertexColour1.w)`, 1 =
  `(N, fog visibility)`, 2 = `Uv1_Uv2.zw` minus `(0, BassHit)`, 3 = `Uv3.xy` minus `(0, TrebleHit)`, 4 = `Uv1_Uv2.xy`:
  HD's routing. The model's declaration also names `Uv1` and `Uv2` at `Uv1_Uv2`'s second pair (`vertex_decl::UV1_UV2_HASH`), so
  the ring reads `Uv3` by its own hash (`SubMesh::uv3_texcoords`); today the diffuse pick holds the same values. The other five fragment programs of a file (#2, #4, #9, #11, #13) differ only in register allocation and
  an `f16` normalize. Wired: `oag_rcs::gxp::electricity` reads the taps by sampler table, the shift by data flow (the
  colour tap's `v` is an `add` of `-time` to an `f32` `mad` of a literal, whose addend is an `add` of `time`) and the
  squares; the second glow-table entry's scale carries the squares to `shade.wesl` (mode 25), which squares the bass and
  pitch taps, the colour tap's square being 2048's own `decode`. The Vita model routes `VertexColour1` through its own
  array (`vita_vertex_colour1`). **Confidence 80** (read end to end; no live Vita read of the ring). Census:
  `electricity.rcsmaterial` under `data/Weapons/` and `data/Weapons2048/` in `data.psarc`, `data1.psarc` and `data2.psarc`,
  EU and USA: twelve files, 72 fragment programs, every one read with the shift 0.3 and the squares
  (`gxp_electricity_ground_truth`); `dlc1.psarc` and `dlc2.psarc` ship none (`vita-gxp.py list`, both regions); `electric_visualiser.rcsmodel` ships in the base `data.psarc` alone (both folders) and
  builds with the ring bound, its declaration carrying every attribute the vertex programs read (`position`, `normal`,
  `VertexColour1`, `Uv3`, `Uv1_Uv2`; `both_2048_visualiser_models_bind_the_electricity_program`). `oag_2048::race::DETONATOR_OBJECTS`
  names HD's entry. Frame: ours on Altima, Detonator, three mines laid ahead (`--detonator-mines`), tick 320: a faint
  pale lavender ring, two thin concentric bands round each mine, absent with the title data reverted. Against HD's (a
  ragged violet band at the disc's rim) and Omega's (small saturated violet circles on its red Zone grade) it is paler and
  more transparent; the capture runs with `--no-audio`, so the frame's band levels are 0.
- **Omega against 2048, the squares: checked, differs.** Omega's GCN program (`Shdr @0xf84`) feeds its `r` tap
  (`image_sample ... dmask:0x1` at `0x16c`) and its `rgb` tap (`0x1f0`) straight into multiplies: no square in code, its
  decode is the texture unit's (`srgb_texels`). The rest of the law is shared.

## The spikes in a pair (2026-11-26, `det-mine-spikes`)

**Claim:** the `mines-ahead` pair's near mines were dark, about half the original's size and without spikes because the pair
carried no music envelope, not because ours' spikes draw differently. **True, confidence 75.**

- **The pair was older than the envelope carry.** `hd/detonator-talons-mines-ahead` was made from a shot record without
  `football_s`/`spikes_t` ([compare-page.md](../../../tools/compare-page.md), "The mines' music envelopes"), so ours ran its own
  envelopes. Its raw bands 11 to 14 (the spikes') are `0.12` to `0.28` and bands 0 to 3 under `0.23`: no rise of more than
  `0.3` ever attacks an envelope that starts at `0`, so every mine held `s = t = 0`: football and spikes at scale 1
  (bounding radii `1.00` and `1.04`, so the spikes sit inside the shell) and `MineBrightness` 0. That is the dark, small,
  spikeless ball of the old pair.
- **The original's envelopes at the grab.** Three restores of `hd-detonator-talons-feisar-grid-late` read the near mines at
  `s` 0.36 to 0.70 and `t` 0.45 to 0.81 (lap 2.68 to 2.73 s), so `spikeScale = clamp(4 t, 1, scale + 0.95)` reaches 1.8 to
  2.1 against a football scale of 1.0 to 1.6: long spikes, as the original's frame shows.
- **Same `theirs.png`, ours without then with the envelopes** (whole-frame mean grey, page scorer, this tree's binary):

  | Grab | Without | With |
  | --- | ---: | ---: |
  | `mines-ahead` re-grab, lap 2.68 s | 15.44 | 13.93 |
  | `pad` re-grab, lap 2.05 s (mines 80 units out) | 10.98 | 10.88 |
  | a cannon volley re-grab, lap 2.63 s (press 2.20 s, mid-blast) | 17.11 | 16.73 |

  By eye on the `mines-ahead` re-grab (crop x 700..1400, y 300..750) ours' two near mines are lit, ringed and spiked, at the
  original's place and size; ours' spikes read a little longer on the left mine. A placeholder envelope (`0.9, 0.65` on
  all 80 mines) on the old pair lit them as well but scored 14.85 against 13.62, so the values have to be the original's.
- **The picture lag is carried, and is not the cause.** Each pair carries its own fitted lag (`original.presented_lag`): 2.03
  ticks on the old pair (1.2 px rms), 2.33 on the re-grab, and ours' near mines stand within 4 px of the original's centroids
  on the re-grab. The 3.5 ticks in "The displacement" is a different fit method; the two are not reconciled.
- **Near-ball 60x60 box, re-grab** (middle mine, centre `1050, 510`, mean RGB): original `(145, 124, 200)`, ours without the
  envelopes `(158, 132, 233)` (this grab's raw bands are high, so ours' own envelopes attack and over-size every mine), ours
  with them `(97, 80, 156)`. White pixels (`min > 200`) between 1.2 and 2.0 shell radii: original 1,611, ours with the
  envelopes 536, without 2,374. With the envelopes the reach and size match by eye, but ours' spikes and body are dimmer
  (a third of the original's white there, the box 50 grey darker): a brightness gap left open, not a size one.
- **Residual, open (confidence 50 for the reading below).** The two near mines read `(s, t)` `(0.36, 0.62)` (left) and
  `(0.37, 0.81)` (middle): the law gives both spike scale 1.95 and football scale 1.0. The original draws the middle one with
  long spikes and the left one with **short spikes and a larger, brighter body**; ours draws both alike. The picture is about
  6.3 ticks older than the RAM the envelopes are read from (`picture_total_ticks`). Decay alone, rewound over that skew
  (`s, t / 0.95^6.3`), would make the body larger and brighter, as seen, but the spikes longer, not shorter; it scores 13.98
  against 13.93, no better. So the left mine's short spikes need an attack of its `t` inside that window (the picture before
  the attack), which one RAM read cannot show. ~~Not wired~~: settled below ("The envelope at the picture's tick"): an attack
  inside the window, read live.
- **Not tuned.** No spike scale, count or map changed: `spikeScale` is the law above (85), fed the original's `t`.
  `BassHit`, `TrebleHit` and `PitchOffset` feed the visualiser ring only (`electricity`), not the football or the spikes.

**Wired:** `rpcs3-hd-pair.py` refuses a Detonator shot record with live mines and no envelopes
(`check-compare-environment.py`, `check_detonator_mine_envelopes`), and `mines-ahead` is re-grabbed with them. `pad` keeps
its old grab: the re-grab's music-lit floor differs from ours' (10.88 against the old grab's 7.60, a floor gap, not the
mines) and the envelope moves it 0.1. `cannon-volley` has the same missing envelopes; its re-grab landed 0.33 s late on the
press, mid-blast, so it is not a like-for-like replacement and the pair is kept.

**2048 and Omega: not checkable.** The fix is HD pair tooling; ours' envelope code is shared and runs on their mines, but no
memory readout of their mine objects exists (no 2048 or Omega Detonator pair carries envelopes).

## The envelope at the picture's tick (2026-11-26, `det-mine-spikes`)

Two boots of `hd-detonator-talons-feisar-grid-late`, `rpcs3-hd-detonator-shots.py` with three stops each and two reads a stop
(12 reads, lap 2.533 to 2.750 s; every read keeps each mine's draw words, `rpcs3_hd_field.MINE_DRAW_WORDS`).

- **The envelope steps once a tick.** The left near mine's `s` reads `0.324, 0.307, 0.292` at lap `2.633, 2.650, 2.667`:
  `x 0.95` a tick exactly. `MineBrightness` (`+0x100`) reads `2 s` and the football scale (`+0xd8`) `clamp(2.3 s, 1, 2.3)` on
  every read. Confidence 85 (12 reads, the law of [detonator-mine-material.md](detonator-mine-material.md) to four digits).
- **An attack inside the picture's lag.** At lap 2.667 every near mine's `t` went `0.57 -> 1.0` in one tick (first boot);
  on the second, the middle mine's `t` reads `0.675` at 2.567 and `1.0` at 2.583. The picture is 5 to 6.3 ticks older than its
  pause's RAM (`picture_total_ticks`), so a pause's own read can hold an envelope the picture never drew. This is the left
  mine's short spikes above. Confidence 75.
- **Wired: the pair takes each mine's draw words from the read nearest the picture's clock** (`rpcs3-hd-pair.py`,
  `picture_read`, within 1.5 ticks, else the pause's own read with a caveat; `original.detonator.mine_read` records which).
  `mines-ahead` is re-grabbed so (lap 2.667, picture clock 2.582, read 2.583). Same `theirs.png`, near-ball 60x60 box (left
  mine / middle mine) and whole frame:

  | Ours fed | Left box | Middle box | Mean grey |
  | --- | --- | --- | ---: |
  | original | `(170, 145, 231)` | `(180, 157, 230)` | |
  | the read at the picture's clock | `(161, 136, 226)` | `(161, 138, 228)` | 13.17 |
  | the pause's own read | `(120, 101, 186)` | `(139, 118, 202)` | 13.15 |

  White pixels (`min > 200`) in a 160 px box: original 6,686 and 3,798, ours 4,967 and 3,106 at the picture's read, 3,217 and
  2,027 at the pause's. **Most of the "dimmer bodies and spikes" was the envelope's tick**: brightness is `2 s`, and the
  pause's read had decayed. The whole frame does not move (13.17 against 13.15): the mines are a small part of it.
- **Residual: the middle mine's centre**, radial means (original / ours, 10 px rings from the centre): `204/131`, `153/168`,
  `201/180`, `208/198`; 24 % against 4 % white in the first ring. The left mine is within 10 grey at every radius. A spike
  pointing at the camera in the original's centre: the spin below, not a brightness term (confidence 55).
- **The ring's words.** `TrebleHit` (`+0x114`) read `0` on all 12 reads, as ours holds it. `PitchOffset` (`+0x108`) read 8.255 to
  8.375, rising 0.005 an update; **wired as a pin**: `--detonator-mine-envelopes S,T,P` holds it per mine
  (`detonator_look::hold_pinned`, `a_pinned_live_mine_holds_the_originals_envelope_whatever_the_bands_are`), and the pair
  passes it. 13.1715 -> 13.1721: neutral on the frame. Its step's writer (`0x00677538`) is unread, so ours' own stays `0`.
  `MasterVolume` read `1.0` and `PitchVolume` `0.32` at the picture's read where ours' band reading gives `1.0` and `0.03`:
  ours' chosen band 1 for `PitchVolume` is off on this frame; not wired (one frame). Read since: both are one level,
  `clamp01(5 L)` and `clamp(L, 0, 0.4)`, six live reads, and pinning them costs `mines-ahead` 0.01
  ([detonator-mine-material.md](detonator-mine-material.md), "The visualiser's level").

### The spin, read

`DetonatorMine_AnimateToMusic` (`0x0013969c`-`0x001396ec`): `phase m+0x150 = 4.0 * m+0xc0` (`K` at TOC `-0x2bcc`, `4.0`),
wrapped by `2 pi` (`-0x2bd4`) against `pi` (`-0x2bd8`). `m+0xc0` is the mine's age ([detonator-mine.md](detonator-mine.md)).
Live: age 55.7836 on every mine of the grab, phase 3.2228 = `4 * 55.7836 mod 2 pi`. `0x006ca538` (TOC `0x008bd3c4` through
the stub's `r2 += 0xfeec`) takes `angle = (int)(phase * 2^24/pi)` sign-extended from 25 bits `* pi/2^24` (constants
`0x4aa2f983`, `0x34490fdb`), so the angle is `phase` itself, and **writes all three rows from scratch**: row 0
`(x^2 (1-c) + c, xy (1-c) - z s, xz (1-c) + y s)`, row 1 `(xy (1-c) + z s, y^2 (1-c) + c, yz (1-c) - x s)`, row 2
`(xz (1-c) - y s, yz (1-c) + x s, z^2 (1-c) + c)`, the axis `(x, y, z)` = `m+0xe0` (live: the object's up row). So a mine's
basis is the world axes turned about its up by `4 * age` radians, not a frame built on its up; translation `m+0x80`.
Confidence 75 (read end to end; the phase reproduced from the live age; the rows' sense in the row-vector convention not
checked against a frame; settled below). Wired in "The spin, wired".

## The spin, wired (2026-11-26, `det-mine-spin`)

**Claim:** ours' HD Detonator mines turn about their up axis at 4 rad/s of the mine's own age, from world axes, as the original
does; the `mines-ahead` pair carries the grab's mine ages, and its mean grey drops. **True.**

- **The law** (`oag_raceplay::detonator_look`, `spin_phase` and `spun_basis`): `phase = (4 age) mod 2 pi`, and the football and
  spikes are drawn on the world axes turned about the mine's survey up by it, the scale applied after. The original's rows are the
  model's axes, so each row is a column in ours; about `+Y` row 0 is `(c, 0, s)`, a turn of `-phase` in ours' right-handed sense.
  Dark mines turn too (the dark branch builds the same rotation, `0x00139bf8`). The rate is `Title` data,
  `oag_title::Detonator::mine_spin`: `Some(4.0)` on HD, `None` on 2048 and Omega (below), where the mine keeps its upright frame.
- **The age is view state**, not `World` state: `MineEnvelopes` records the race clock a mine is first seen at, and the age is
  the clock since. `DetonatorMine_Place` zeroes `m+0xc0` and `BaseMine_Update` steps it ([detonator-mine.md](detonator-mine.md));
  the simulation reads no age, so nothing enters the state hash. Ours' own age counts from ours' laying, so it cannot reproduce the
  original's: every mine of the grab is 53.1 s older than the lap clock (age 55.70 at lap 2.583, the grid wait included), and only
  a pin matches it.
- **The pin.** `--detonator-mine-ages "A;A;..."` holds each carried mine's age (live or dark) in `--detonator-mines` order;
  `rpcs3-hd-pair.py` records `age_s` per mine in `original.detonator.mines[]` from the same read as the envelope, the one nearest
  the picture's clock. On `mines-ahead` that is lap 2.583, age 55.7002 on all 80 mines (phase 2.89); the pause's read is lap
  2.667, age 55.7836 (phase 3.22), 19 degrees on.
- **The sense, checked against a frame.** Same `theirs.png`, ours rendered with each choice (whole-frame mean grey, page scorer):

  | Ours | Mean grey |
  | --- | ---: |
  | no spin, upright frame | 13.17 |
  | `-phase` (the rows as read), the picture's age 55.7002 | **12.51** |
  | `+phase`, same age | 13.16 |
  | `-phase`, the pause's age 55.7836 | 13.01 |
  | `-phase`, ours' own age (from ours' laying) | 12.97 |

  Only the read sense at the picture's age moves the frame; by eye (crop 780..1300, 330..690) both near mines then show the
  original's face pattern and spike layout. The middle mine's centre, radial means and white share in the first 10 px ring:
  original 181 grey / 22 %, ours before 135 / 7 %, after 165 / 18 %. The rows' sense is **confidence 80** now (read end to end,
  and one frame picks it over its mirror by 0.65 grey).
- **The other pairs** carry no ages (older shot records) and run ours' own: `pad` 7.61 -> 7.61, `cannon-volley` 14.32 -> 14.32,
  `cannon-volley-hot` 11.47 -> 11.48.
- **Tests:** `detonator_look::tests::a_mines_basis_is_the_world_axes_turned_about_its_up_by_the_originals_rows` (the sense and the
  world-axes start), `a_mine_turns_four_radians_a_second_of_its_age_and_the_live_phase_reproduces`,
  `a_mines_age_counts_from_its_laying_and_a_pin_holds_it_live_or_dark`; disc-backed
  `tests::detonator_craft::an_hd_mine_turns_about_its_up_by_four_times_its_age`; the pair's age from the picture's read in
  `check-compare-environment.py` (`check_detonator_mine_envelopes`).

## The ring's radius (2026-11-26, `det-mine-ring-radius`)

**Claim:** the ring's own radius is read from the original, ours draws it there, and the `mines-ahead` pair's mean grey
drops. **False: ours already draws the ring at the original's radius**, so there is nothing to wire and the pair cannot
move by it.

- **Nothing in the program sizes the ring beyond the model and the pose.** `electricity.rcsmaterial`'s vertex block #1
  (`ps3-microcode.py vp-file electricity.rcsmaterial 1`, 15 instructions) builds the position as
  `R0.xyz = v[0] * c[210] + c[211]` (`MUL` @6, `ADD` @7), `c466`/`c467` being `positionScale`/`positionBias` (`0x9cc5ab3a`,
  `0xa4972b78`), the model's dequantisation every HD vertex program applies ([renderer.md](renderer.md)), then
  `o[POS] = R0 * c[0..3]` (@10..@14, the `float4x4` at `c256`). No music term, no volume and no per-instance scale touches
  the position. So the ring's size is the model's disc times `DetonatorMine_PoseVisualiser`'s `1 + 0.4 (football - 1)` live
  (`0.9` dark), already wired ("The visualiser, wired"). Where the lightning sits inside the disc is the fragment program's
  colour tap at `uv1.y + 0.3 m`, also wired; `m` carries the volumes, which a measured re-landing moved by `0.01`
  ([detonator-mine-material.md](detonator-mine-material.md), "The visualiser's level"). Confidence **80** (both programs
  read end to end; the pose's scale at 65, unchanged).
- **Measured on the pair, three mines.** Same `theirs.png`, ours rendered at `main` (`d28fc9db9`, the spin merge's render
  unchanged). Each near mine projected with the pair's own `--camera-pose` and `--camera-fov 68.0996` (focal 798 px), its
  ring scale `1 + 0.4 (football - 1)` from the pair's `football_scale`. Violet mask `b > 150, r > 100, b - g > 60, r - g > 30`;
  for the middle mine, centre searched over +-10 px and the 3 px annulus with the highest violet share kept:

  | Mine | Depth, px a unit, ring scale | Original | Ours |
  | --- | --- | --- | --- |
  | middle, projected `1047, 507` | 23.97, 33.3, 1.097 | centre `1049, 507`, radius 57.5 px | `1045, 501`, 55.5 px |
  | left, projected `758, 536` | 18.10, 44.1, 1.072 | on a 74 px circle, by eye | on the same circle, by eye |
  | right, projected `1521, 518` | 23.62, 33.8, 1.123 | on a 60 px circle, by eye | on the same circle, by eye |

  The middle mine's radial profile (violet share, 4 px rings) peaks at 56 px in the original (0.34) and 52 px in ours
  (0.34); 2 px of 57 is 3.5 %. Its ring stands at `57.5 / (33.3 x 1.097) = 1.57` model units, 0.64 of the 2.45-unit bound
  (the bound is not the rim). That radius predicts 74 px for the left mine and 60 px for the right; circles of those radii on
  2x crops lie on the ring in both frames (the original's left ring is fainter and more diffuse). On the side mines an
  annulus fit locks onto the violet body or the spikes in both frames (31 to 34 px, or 52 to 55 px with the centre at the
  search's edge), so the circle check is the evidence there. Confidence **75** that ours' radius is the original's (three
  mines, one frame, one by fit and two by eye; `cannon-volley` and `cannon-volley-hot` have no like-for-like near mine,
  ours' mines there stand far off, and `pad`'s are 80 units out).
- **Where `mines-ahead`'s gap is** (page scorer's grey difference, share of the whole-frame 12.49): the three near mines'
  boxes (`650..870 x 420..650`, `960..1135 x 420..600`, `1420..1625 x 420..620`, background included) carry **2.49**
  (0.82, 0.54, 1.14). The score panel (`700..1225 x 35..240`, HUD) carries **2.10**. The scenery in the upper right
  (`1100..1920 x 0..400`, the panel left out) carries **3.26**: an overhead structure ours draws lit grey where the
  original is black, and the violet panels beside it. So no ring change can bring the pair under 10.
- **Not wired, so no test added.** The ring's scale law is pinned by
  `tests::detonator_craft::a_laid_mines_visualiser_ring_faces_the_camera_and_follows_the_footballs_size`.
  `5fac71320` (the volumes) was not re-landed: the radius is already right, so the brief's condition for it did not hold.
- **Pairs** (this tree, unchanged): `mines-ahead` 12.51, `pad` 7.61, `cannon-volley` 14.32, `cannon-volley-hot` 10.31.
- **2048 and Omega: not checkable.** Both ship `electric_visualiser` and draw it ("The visualiser, wired"), but no 2048 or
  Omega Detonator pair has a lit near mine to measure a ring radius on.

## Still open

- The writer of the spin axis `m+0xe0` (ours takes the survey's up; live it read as the object's up row).
- `+0xfc = +0xc0 * k` in `DetonatorMine_Update` ([detonator-mine.md](detonator-mine.md)): a second turn word, its reader unread.
- The eye behind `0x00987884+0x70` (the visualiser's camera), read to its writer.
- A live read of the mine's submitted matrix (`0x001399a4`, `r4`) would lift rows 3 and 4 above 80.

## Lineage (2048 and Omega)

- **2048 v1.04: ported** (the visualiser ring; "The visualiser, wired"). The mine parts: `data.psarc` ships `data/Weapons/` and `data/Weapons2048/`
  `Mine_Rita_Football`, `Mine_Rita_Spikes`, `Electric_Visualiser` (`.vex` and `.rcsmodel`) with
  `Detonator_Mine_Normal_Alpha.rcsmaterial` and `electricity.rcsmaterial`, the same three parts and materials as HD;
  `eboot.elf` names `mine_rita_football`, `mine_rita_spikes`, `Electric_Visualiser`. The function bodies were not read.
  **The spin: not checkable** yet: 2048's mine animate function is not located, and no 2048 Detonator pair reads a mine's age;
  `Detonator::mine_spin` is `None` there, so its mines keep the upright frame (2026-11-26, `det-mine-spin`).
- **Omega: checked, differs.** `data00.psarc` ships the same three models plus parts HD has not:
  `Detonator_Mine_Platonic_A`/`_B`/`_C`, `Detonator_Mine_Glow_Sprite`, `Detonator_Mine_Explosion_Kaleidoscopic`, the
  textures `detonator_mine_glow_red`, `detonator_mine_panel_black`, and the spikes get their own
  `Detonator_Mine_Spikes{,_1,_2}.rcsmaterial` beside `Detonator_Mine_Normal_Alpha{,_1,_2}` (also in `data05` and
  `data08`). `eboot.bin` names the three HD models too. Which parts Omega's mine draws is not read.
  **The spin: not checkable** yet: Omega's mine animate function is not located and no Omega mine object has been read live;
  `mine_spin` is `None` (2026-11-26, `det-mine-spin`).
