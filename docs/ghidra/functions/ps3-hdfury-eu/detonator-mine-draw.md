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

The ring's own radius inside its 2.45-unit bounds is not measured (the model's arcs reach beyond it). Measured by eye on
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
  mine's own up); `MasterVolume` and `PitchVolume` read bands 0 and 1 (the original's level readers at `0x304cc8` are
  unread); `PitchOffset` and `TrebleHit` stay 0 (accumulator step and `.bss` writer unread); the colour tap is decoded.
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
- **2048: checked, applies, not wired.** 2048 has the Detonator mode (mode `0xe`, `vita-2048-eu-v104/detonator-hull.md`) and
  ships the same model and material, but the material is GXP and **its pixel program is not HD's listing**: the USSE
  program (`vita-usse.py disasm <data.psarc:/materials/electricity> 1`, 48 primary instructions) carries three texture
  fetches where Omega's GCN has five, so the taps are packed differently. Not read to the end, so
  `DetonatorObjects::mine_visualiser` stays `None` there and the loader says so.

## Still open

- `0x006ca538`'s stores (whether the object's basis survives the spin) and the writer of the axis `m+0xe0`.
- The eye behind `0x00987884+0x70` (the visualiser's camera), read to its writer.
- The ring's own radius inside `Electric_Visualiser` (HD's material program is read: "The visualiser, wired").
- A live read of the mine's submitted matrix (`0x001399a4`, `r4`) would lift rows 3 and 4 above 80.

## Lineage (2048 and Omega)

- **2048 v1.04: checked, applies, not wired.** `data.psarc` ships `data/Weapons/` and `data/Weapons2048/`
  `Mine_Rita_Football`, `Mine_Rita_Spikes`, `Electric_Visualiser` (`.vex` and `.rcsmodel`) with
  `Detonator_Mine_Normal_Alpha.rcsmaterial` and `electricity.rcsmaterial`, the same three parts and materials as HD;
  `eboot.elf` names `mine_rita_football`, `mine_rita_spikes`, `Electric_Visualiser`. The function bodies were not read.
- **Omega: checked, differs.** `data00.psarc` ships the same three models plus parts HD has not:
  `Detonator_Mine_Platonic_A`/`_B`/`_C`, `Detonator_Mine_Glow_Sprite`, `Detonator_Mine_Explosion_Kaleidoscopic`, the
  textures `detonator_mine_glow_red`, `detonator_mine_panel_black`, and the spikes get their own
  `Detonator_Mine_Spikes{,_1,_2}.rcsmaterial` beside `Detonator_Mine_Normal_Alpha{,_1,_2}` (also in `data05` and
  `data08`). `eboot.bin` names the three HD models too. Which parts Omega's mine draws is not read.
