# The engine light: `EngineLightData.xml` feeds a vertex light per craft

Read 2026-11-16, lane `engine-glow-re`, on the **1.07 patch** eboot (Ghidra program
`/omega/eboot-ps4-omega-eu-v107.bin`; Omega addresses on this page are patch addresses unless marked
**base**, see [patch-v107-addresses.md](patch-v107-addresses.md)), on 2048's v1.04 executable
([its page](../vita-2048-eu-v104/engine-light.md)) and against HD's
`EngineFlare_SubmitSpuLight` ([renderer.md](../ps3-hdfury-eu/renderer.md), "the orange family's true
call site"). The producer was read statically (lane `engine-glow-re`); the receivers' GCN programs
and the live check on the native port are lane `engine-light-re`, the same day (see "Live check"). **Wired on Omega and 2048 the same day (lane `engine-light-wire`)**, see "Wired".

## Summary

| Question | Answer | Confidence |
| --- | --- | --- |
| What reads `Distance` / `Radius` (`ship+0xbc` / `+0xc0`) | `EngineFlare_SubmitVertexLight`, `0x0173e350` (**base** `0x016f5a00`), once per craft per frame | 85 |
| What it makes | one record in a 32-slot per-frame light table (`0x02047e60`, count `0x020482e0`): a point light with a position, a range and a colour | 85 |
| What the light is | not a sprite and not a material term of the flare: a **vertex light**. Each draw picks up to six table records that overlap its bounds and binds them as `DiffuseVertexLight<N>position` / `DiffuseVertexLight<N>diffuse` | 85 for the pick and the names, 90 for the falloff (Omega's own GCN hull program read, and the hull seen lit and unlit live) |
| Does it light the hull | **yes, and it flickers.** The range is redrawn as `Radius +- 1.0`, so an HD-era craft's light runs from nothing to twice its authored reach from one frame to the next; the housing is dark on some frames and washed warm on others. A single captured frame is one draw | 90 (two boots, 66 shipped frames against 36 with the producer switched off, and a forced-range sweep) |
| What the glow in the nozzle is | not this light: the flare model (`engineflare.rcsmodel`, material `2048_engine_additive`, `flame_01.gnf`). With the producer switched off the light table is empty and the flame is unchanged | 88 |
| Colour source | constants in the executable, picked by three ship facts: the team's `GameVersion`, the Fury skin name, and (2048 modes only) the mag-strip contact. No per-team or per-file colour | 85 |
| Strength at standstill | the base colour times `1.0` (the boost blend is `0`), range `Radius` plus a per-tick jitter | 92: read live on Omega, eight records of exactly `(40, 10, 4)` on the Modesto Heights grid and eight of `(3.0, 0.5, 0.1)` on Queens Mall |

The seven float readers of `[reg + 0xbc]` the previous lane listed are not it: `FUN_01083a10` and
`FUN_01083d80` are an audio filter coefficient (`x * -2 pi / sample rate + 1`), `FUN_0177e140` is the
music fade and spectrum, `FUN_0103c000`, `FUN_012cb090`, `FUN_012d9190` and `FUN_012de2d0` read their
own objects' `+0xbc`. The consumer loads `Distance` with `VBROADCASTSS` (`0x0173e4bc`), which a
`VMOVSS` search does not find.

## The producer: `EngineFlare_SubmitVertexLight` (`0x0173e350`)

`this` is the engine flare object (`E:\WOPS4\Wipeout\Code\System\Render\EngineFlare_Importer.cpp`);
`this+0x118` is its ship, `this+0x68` the flare node's world matrix. Constants are one block at
`0x01863abc` (**base** `0x018053fc`), read as bytes:

| Address | Value | Use |
| --- | --- | --- |
| `0x01863abc` | 3.0 | 2048-era red; mag-strip blue |
| `0x01863ac0` | 0.5 | 2048-era green |
| `0x01863ac4` | 0.1 | 2048-era blue; mag-strip green |
| `0x01863ac8` | 5.0 | 2048-era tint add |
| `0x01863acc` | 2.2 | 2048-era boost gain |
| `0x01863ad0` | 4.0 | HD-era low channel |
| `0x01863ad4` | 40.0 | HD-era high channel and tint add |
| `0x01863ad8` | 10.0 | HD-era green and boost gain |
| `0x01863adc` | 1.5 | mag-strip red |
| `0x01863ae0` | 0.2 | mag-strip gain scale |
| `0x01863ae4` | 1.0 | |
| `0x01863ae8` | 0.01 | least range submitted |

```text
ship  = this+0x118
if ship+0x8ba1:                         ; the team's GameVersion is "2048"
    base = (3.0, 0.5, 0.1); gain = 2.2; add = 5.0
elif ship+0x8ba0:                       ; Fury skin
    base = (40, 10, 4);     gain = 10;  add = 40
else:
    base = (4, 10, 40);     gain = 10;  add = 40

t      = this+0x260                     ; transition countdown
tint   = t > 0 || this+0x34c
strip  = mode id (0x0201280c) >= 0x17 && ship->vtable[0xd0](ship) && !tint
if strip:
    base = (1.5, 0.1, 3.0); gain *= 0.2

if !tint:
    blend  = this+0x13c + this+0x130    ; the boost blends, 0 at rest
    colour = base * (1 + gain * blend)
    calm   = 1 - blend
else:                                   ; the transition tint, per the HD page
    t >= 1:  colour = (base.r + add, base.g, 0)    ; (44, 10, 0) classic, (80, 10, 0) Fury, (8, 0.5, 0) 2048-era
    else:    colour = (low + add * t, base.g, high * (1 - t)), t clamped to 0..1
    calm   = 1

anchor   = node.row3 - node.row2 * Distance        ; ship+0xbc, behind the nozzle along the node's Z
off      = this+0x270                              ; vec4 (x, y, 0, w), redrawn every flare update
position = anchor + off.xyz * calm
range    = Radius + off.w * calm                   ; ship+0xc0
if range > 0.01 && count < 32:
    table[count++] = { position, 1 / range, colour, range, id = -1 }
```

`high`/`low` are the 40 and 4 of the HD-era bases (3.0 and 0.1 for the 2048 era), whichever skin.

Evidence, per term:

- **The read is unambiguous (90).** `*(float *)(ship + 0xc0)` and `*(float *)(ship + 0xbc)` are loaded
  off `*(this + 0x118)` at `0x0173e47e`-`0x0173e4bc`; the loader `FUN_01335030` stores `atof` of
  `Distance` and `Radius` there.
- **`ship+0x8ba1` is "the team's `GameVersion` is 2048" (90).** The ship loader `FUN_013354e0` writes
  `*(int *)(*(ship + 0x6e08) + 0x12c) == 1` (`0x013375c6`-`0x013375d4`); `ship+0x6e08` is the team
  record matched by name. `FUN_014ae370` parses the `GameVersion` attribute and stores
  `strcasecmp(value, "2048") == 0` at record `+0x12c`. `Data\plugins\teams\Definition.xml` authors
  `gameversion="2048"` on exactly the five `*2048` teams. The same byte picks the flare sprite:
  `FUN_01739980` loads `data/Tex/EngineFlare/Engine_Flare_Rich_red.gnf` when it is set and
  `Engine_Flare_Rich.gnf` otherwise.
- **`ship+0x8ba0` is the Fury skin (90).** `FUN_01334a30` sets it when the model name
  (`*(ship + 0x6e10) + 0xb8`) is `chrome_c1`, `nitro`, `concept1`, `vr_fury` or contains `detonator`.
  It is HD's `Ship_SetFuryTrailFlag` with `vr_fury` added.
- **The virtual at `+0xd0` is the mag-strip predicate (75).** All five ship vtables
  (`0x0197db90`, `0x01996970`, `0x01996bb0`, `0x01996d88`, `0x01997178`) hold `FUN_01310580` or its
  wrapper `FUN_017b7b10` there. Its body is the shape [ships-effects.md](ships-effects.md) records for
  the base build's `Ship_IsOverMagStrip` (slot kind 0 or 2 reads a byte on the controller object, here
  `controller+0x5e0`; a remote ship reads bit 1 of the replicated byte at `+0x41 + slot * 0x21`). The
  slot moved from `+0xc8` to `+0xd0` and the byte from `+0x5d0` to `+0x5e0` between the builds; the
  match is by shape, the 1.07 writer of `+0x5e0` was not read.
- **Mode ids from `0x17` up are the 2048 game modes (75)**, per
  [race-hud-selection.md](race-hud-selection.md).
- **The jitter (90: 75 static, then forced live, see "Live check").** `FUN_017366c0` (the flare's update) writes `this+0x270` at `0x017384f5` from
  three draws of the engine's lagged generator (`0x02102e50`), scaled by `0x02102e9c`, which
  `FUN_0167a020` sets from `1.0` (`0x018636e0`) and an exponent of `-32`: `x, y = r * 0.4 - 0.2`
  (`0x01863a18`, `0x01863a1c`) and `w = r * 2 - 1` (`0x01863a20`), so the position wanders 0.2 units
  either side and the range **one unit** either side. HD's are 0.1 each. The same instruction block
  decrements `this+0x260` by the frame time.
- **The tint is armed by the flare's own state change (60).** `FUN_0173c810` writes
  `this+0x260 = 3.3` (`0x40533333`) and clears `this+0x34c` at `0x0173e1a7`, on the path where
  `this+0xb4` differs from a three-way class of its float argument (thresholds `0x01863ab4`,
  `0x01863ab8`), and sets `this+0x34c` at `0x0173cec9`. Which input that argument is was not read.
  This is the arming the HD page left open.
- **Who calls it (70).** Only `FUN_01307d40` (`0x0130843a`), slot 5 of the ship vtables, when the ship
  has a flare (`ship+0x69c8`) and its state value is below 5.

### At standstill

`blend` is `0` (`this+0x130` is zeroed by `FUN_01739980`; HD's live capture reads the shared branch
as exactly `(40, 10, 4)` on the grid), so the colour is the base and the jitter runs at full size:

| Craft | Colour | `Distance` / `Radius` authored | Range per tick |
| --- | --- | --- | --- |
| HD-era, classic skin | `(4, 10, 40)` | -0.4 to 1.2 / 1.0 to 2.0 | `Radius` +- 1.0 |
| HD-era, Fury skin (`concept1`, `nitro`, `chrome_c1`, `vr_fury`, `detonator`) | `(40, 10, 4)` | same | `Radius` +- 1.0 |
| 2048-era (`GameVersion="2048"`) | `(3.0, 0.5, 0.1)` | 3.0 / 30.0 (all twenty `ships\<team>2048\<n>` files) | 29 to 31 |
| any craft over a mag strip in a 2048 mode | `(1.5, 0.1, 3.0)` | its own | its own |

This is why HD's law washed 2048's circuits white: a 30-unit light drawn at `(4, 10, 40)` instead of
`(3.0, 0.5, 0.1)`. The 2048-era light is a wide, dim red one three units behind the nozzle that
reaches the floor (ride height is about 4 units); the HD-era one is a tight, strong one at the nozzle
that reaches only the housing.

## The table and who receives it

- **Record, `0x24` bytes (85):** `+0x00` position xyz, `+0x0c` `1 / range`, `+0x10` colour rgb,
  `+0x1c` range, `+0x20` an owner id (`-1` = every view). 32 records at `0x02047e60`, count at
  `0x020482e0`, a zero record at `0x020482e4`. `FUN_0162a390` clears the count once a frame
  (`0x0162d3ef`).
- **Other producers.** 21 more functions append to the table (`get_xrefs_to(0x020482e0)`: the
  `0x0134e670`-`0x013a03e0` weapon and effect range, `FUN_012ecc90`, `FUN_01322370`, `FUN_01767a00`,
  and `FUN_01307d40` itself, which appends one more light per ship before the flare's). None was read.
- **The pick (85).** `FUN_0165b890(sphere, &count, view_id)` walks the table in order and keeps the
  records whose owner is the view or `-1` and whose axis-aligned box (`position +- range`) overlaps the
  draw's bounding sphere, stopping at six; `FUN_0165b760` is the same over a box and takes only owner
  `-1`. A count of 3 or 5 is padded to 4 or 6 with the zero record. Callers: `FUN_0127a130`,
  `Scene_PrepareFrame`'s 1.07 twin `FUN_015f2d30`, `FUN_01606410`, `FUN_01610fa0`, `FUN_01618bb0`.
- **The bind (80).** The caller copies the first `0x20` bytes of each picked record into the draw's
  constant block as two vectors, `(position, 1 / range)` and `(colour, range)`
  (`FUN_0127a130`, after `0x0127b6b6`). The executable names twelve constants,
  `DiffuseVertexLight0position` to `5position` and `0diffuse` to `5diffuse` (`0x0188e249`-`0x0188e475`).
- **Which draws (88).** The caller's loop takes a submesh only when `(submesh+0x24 & 7) != 0`, which
  reads as the number of lights its program declares. The hull's own material declares them (next
  section), and the hull is seen lit live. On 2048 the uniform is declared by 24,291 programs: every
  environment's materials and the ship materials
  ([2048's page](../vita-2048-eu-v104/engine-light.md)). So track and hull both receive it.
- **The falloff (90).** `diffuse * max(0, N . L) * max(0, 1 - d / range)` per vertex, read on
  Omega's own hull program below and on 2048's.

## The receiving programs on Omega (GCN)

`Data\art\published\hdships\feisar_c1\ship.rcsmodel` (the grid craft of the Omega HD-mode pairs) names
five materials: `data/materials/ships/diffuse_with_specular_from_alpha_n_vcol`, `carbonfibre`,
`diffuse_vcol`, `glass_texture_clamped` and `detonator_emissive_bloom`. Its `engineflare.rcsmodel`
names one, `data/art/published/ships/materials/2048_engine_additive`, with `flame_01.gnf` and
`facing_gradient.gnf`: the flame is an additive model of its own, not a lit surface.

`gcn_dump data08.psarc data/materials/Ships/diffuse_with_specular_from_alpha_n_vcol.rcsmaterial
--summary` lists 21 programs. The vertex programs come in a family by light count: none (`0x1964`,
96 instructions), one (`0x28e4`, 119), two (`0x39c4`, 140), four (`0x42e4`, 184) and six (`0x4de4`,
227), each declaring `DiffuseVertexLight<N>diffuse` and `<N>position` for its lights, and the same
family again with the shadow matrices (`0x98c4`, `0xb224`, `0xbc64`, `0xc884`). That is the
`submesh+0x24 & 7` count and the pad of 3 to 4 and 5 to 6. `carbonfibre.rcsmaterial` has the same
family.

**The vertex program, one light (`--at 0x28e4`), 90.** `s[0:3]` is `DiffuseVertexLight0position`
(constant dwords 44 to 47), `s[64:67]` is `DiffuseVertexLight0diffuse` (40 to 43), `v1..v3` the world
position, `v4..v6` the world normal (`world` times the vertex normal, **not normalised**):

```text
 b4  v_sub_f32 v24, s0, v2            L = light.position - world   (c0, c8: y, z)
 bc  v_mul_f32 v19, v24, v24  ...     d2 = L . L                    (d0, e0)
 f0  v_max_f32 v25, 0.01, v19
114  v_rsq_f32 v10, v25               1 / d
118  v_min_f32 v8, 65504, v19
124  v_mad_f32 v14, -v10, v8, s67     diffuse.w - d2 / d = range - d
158  v_max_f32 v13, 0, v14
15c  v_mul_f32 v18, v10, v24          (N . L) / d                   (v24 = N . L from f8, 120, 13c)
180  v_mul_f32 v13, s3, v13           position.w * max(0, range - d) = max(0, 1 - d / range)
184  v_max_f32 v18, 0, v18
1a0  v_mul_f32 v13, v13, v18
1e8  v_mul_f32 v24, s64, v13          * diffuse.rgb                 (1ec, 1f0: g, b)
224  exp param3 v24, v25, v13, v0     the vertex light, interpolant 2
```

So `diffuse.rgb * max(0, N . L / d) * max(0, 1 - d / range)`, with `range` taken from `diffuse.w`
and `1 / range` from `position.w`: the bind's two vectors, `(position, 1 / range)` and
`(colour, range)`, used as bound. No other scale, no per-material weight, no divide. The one
difference from a textbook Lambert term is that `N` is the world matrix times the vertex normal with
no renormalisation, so a scaled hull matrix scales the light by the same factor.

**The pixel program that takes it (`--at 0x3104`, 172 instructions; its unlit twin is `0x2064`, 166),
88.** The six extra instructions are the three interpolations of the vertex light and its add:

```text
28c  v_add_f32 v6, s0, v7             constantAmbientColour.r + vertex light.r   (2b4, 2e8: g, b)
29c  v_mac_f32 v6, s8, v16            + liveLighting0diffuse.r * N.L
230  v_mul_f32 v2, v2, v14            Diffuse_Texture.r * vertex colour.r
2a4  v_mac_f32 v13, v6, v2            specular + (ambient + vertex light + sun) * texture * vertex colour
2cc  v_mad_f32 v6, v7, v13, s24  ...  fog
```

`out = fog((constantAmbientColour + vertexLight + liveLighting0diffuse * N.L) * texture * vertexColour
+ specular)`. The light joins the ambient, unscaled, and is multiplied by the hull's texture and its
vertex colour: a dark panel takes little of it. The cube-map variants (`0x8c04`, `0xa224`) carry the
same extra interpolant; their bodies were not read line by line.

## Wired

`oag_title::flare::EngineLightLaw` carries the table above as `Title` data (`Authored::engine_light`,
`Some` on HD, Omega and 2048); `oag_raceplay::engine_light::record` is the arithmetic and binds
through HD's path unchanged (`Race::hd_craft_engine_lights`, the `spu_lights` scene block, track and
hull). Per craft: the 2048 era by the craft's roster directory against `Era2048::roster_dir`
(Omega `Data\art\published\ships`, 2048 `Data\art\published\Ships`; the team's
`GameVersion="2048"` is only authored on that roster), the Fury skin by `RaceView::hd_trail_red`,
the strip by `mag_floor_fx::Shown::on` (this tick's mag-floor contact) on a gated circuit (Omega: a
`circuit_lineages` circuit; 2048: every circuit). Wander 0.2 / 1.0 scaled by `1 - blend`; a range at
or under `0.01` submits nothing. Not wired: the six-light cap (ours sums every light), the
transition tint, the 21 other producers. Omega's own fragment combine of the light was not read, so
the receivers' shader is HD's `SVC1` combine **inherited**.

Boxes, ours against the original on the stored pairs. **MAE** is the mean absolute difference of the
8-bit R, G and B values over the box's pixels (0-255). Ours is the pair's `ours.flags` re-run on the
branch's `oag-game` (`--screenshot`, into a scratch directory; the shared reference is not written),
before and after the wiring. Rectangles are `x0,y0 - x1,y1` in the pair's own pixels (1920x1080 Omega,
960x544 2048); craft poses differ by a few pixels between the two sides and our hulls are paler than
the original's before any light, so MAE moves with brightness more than with hue. The Amphiseum
floor box sits beside the craft (right of it), not behind it: the original's flame fills the floor
under the nozzle there.

| Pair (craft) | Housing box | MAE | Floor box | MAE | What the crop shows |
| --- | --- | --- | --- | --- | --- |
| `omega/the-amphiseum` (Feisar_c1, Fury) | 880,690 - 1020,810 | 72.0 -> 77.3 | 1140,630 - 1240,760 | 53.4 -> 53.5 | the original's orange rim on the housing, vents and fins now appears on ours; housing R/B 0.86 -> 0.97 against the original's 1.04 |
| `omega/modesto-heights` (Feisar_c1, Fury) | 890,690 - 1010,800 | 41.9 -> 50.5 | 1030,940 - 1140,1020 | 71.6 -> 72.8 | warm rim appears; stronger than the original's, whose housing is near-black (R/B 0.84, ours 0.90 -> 1.12) |
| `omega/queens-mall-2048` (Feisar2048\3) | 920,800 - 1000,880 | 88.3 -> 88.7 | 1140,650 - 1260,800 | 25.4 -> 24.6 | the dim red light barely moves the housing (R/B 1.15 -> 1.17 against 1.81); the floor red rises 162 -> 167 against 179 |
| `2048-vita/moa-therma` (Feisar, `hdships`) | 440,385 - 510,440 | 39.3 -> 48.5 | 460,450 - 560,500 | 19.7 -> 20.7 | blue glow on the nozzle housing and engine pipes as the original has, but stronger and wider (B/R 1.02 -> 1.61 against 1.16) |
| `hd/the-amphiseum` | - | - | - | - | pixel-identical to before (`magick compare -metric AE` 0) |

So the colour and place are right and the strength is not yet proven: all four housing boxes
read worse in MAE because ours is brighter than the original's there. Not tuned. The six-light
cap (ours sums every light) is not expected to explain it: an HD-era light reaches at most 3 units, so a
hull draw on a grid overlaps few records (not counted); it can matter for the 2048 era's range 30.
Open: Omega's and 2048's fragment combine, and a frame's own wander (the range is `Radius` +- 1, so one
original frame may be anywhere from no light to twice the authored one). The wander flickers on
ours: four consecutive ticks of the Amphiseum housing box differ by a mean 3.5 to 6.1 (0-255) with
the box mean at 129 to 133, a few percent. The stored pairs hold one frame of the original, so
whether it flickers by that much is not checkable from them; the jitter stays as read (confidence
75).

## Cross-title

- **2048: ported** (same law, `crates/2048/src/lib.rs`, its own roster directory and a mag-strip gate on every circuit; wander inherited from Omega). The receivers too: 2048's copy of the same hull material has the same
  light-count family and the same combine ([its page](../vita-2048-eu-v104/engine-light.md), "The
  hull's own programs"). Not checked live on 2048 (no run of its executable in this lane), and its
  jitter sizes are still unread, so the flicker on 2048 is Omega's by inheritance. The same function is in 2048's own executable with the same
  constants, the same three colour branches and the same record
  (`EngineFlare_SubmitVertexLight`, `0x812726d6`, [page](../vita-2048-eu-v104/engine-light.md)).
  One difference: the mag-strip branch is gated by a call (`FUN_81000930(0x8153fc40, -1) == 0`) where
  Omega tests the mode id; that call was not read. 2048 names no `Engine_Flare_Rich_red` sprite.
- **HD: checked, differs.** HD has the HD-era half only (`EngineFlare_SubmitSpuLight`,
  `0x0029ff28`): the two bases, gain 10, the tint, jitter 0.1 on every axis, and a different receiver
  (the SPU vertex-light list). No `GameVersion` branch and no mag-strip branch. Whether HD scales the
  jitter by `1 - blend` was not re-read. HD's is wired (`oag_raceplay::engine_light`).
- The live HD capture (40 records, all `(40, 10, 4) * (1 + 10 * blend)`) is the check on the branch
  the three executables share.

## Live check (2026-11-16, `engine-light-re`): the hull is lit, and it flickers

`scripts/wipeout-pc-engine-light-live.py` on the native port (`wipeout_pc` `e56904d`, the 1.07 eboot,
image base `0x800000000`, `/proc/<pid>/mem` from the launching process): four cold boots on a fresh
save, Race Box single race, walked as the pairs are. Three on the Modesto Heights grid (WipEout HD
mode, `FEISAR_C1`: `Distance -0.4`, `Radius 1.0`), one on Queens Mall (WipEout 2048 mode,
`Feisar2048\3`: `3.0` / `30.0`). The producer's first 24 bytes read back at the mapped address on
every boot, and the constant block reads `3.0, 0.5, 0.1, 5.0, 2.2, 4.0, 40.0, 10.0, 1.5, 0.2, 1.0, 0.01`.

**The table (92).** Sampled 600 times a boot (the frame clears and refills it, so about one read in
twenty is full). On the Modesto grid, eight records, one per craft, 20 units apart down the grid:

| Field | Read |
| --- | --- |
| colour | `(40.0, 10.0, 4.0)` on all eight during the countdown; `base * (1 + 10 * blend)` once the field leaves (`(74.4, 18.6, 7.4)`, `(360, 90, 36)`, and so on), the player's still `(40, 10, 4)` |
| range | `0.05` to `1.87` on the player's record across 27 full reads (`Radius 1.0 +- 1.0`); `0.56` to `2.86` over the field (other craft author up to `2.0`) |
| `1 / range` | the reciprocal of the range on every record |
| owner | `-1` on every record |

On Queens Mall, eight records of `(3.0, 0.5, 0.1)`, range `29.00` to `31.00`. That is the standstill
table above, both eras, measured.

**The hull is lit, on some frames (90).** Frames grabbed about 0.45 s apart with the player's craft at
rest after the field has left, then the same with the producer switched off (`c3` written over its
first byte: the table reads empty on 300 of 300 samples). Counted per 1920x1080 frame: pixels of the
craft region `(640, 560)-(1280, 960)`, less the flame's box `(900, 840)-(1020, 1000)`, with
`R > 1.35 * max(G, B)` and `R > 50` (`scripts/engine-light-warm-census.py`):

| Frames | n | Warm pixels: mean | median | 10th to 90th percentile | max |
| --- | --- | --- | --- | --- | --- |
| shipped, boot 2 | 60 | 4,712 | 2,700 | 368 to 12,458 | 16,232 |
| shipped, boot 1 | 6 | 6,202 | 4,585 | | 12,856 |
| producer off, boot 2 | 30 | 447 | 438 | 276 to 516 | 1,387 |
| producer off, boot 1 | 6 | 309 | 300 | | 347 |
| the stored pair's `theirs.png` (`omega/modesto-heights`) | 1 | 115 | | | |

The shipped frames range from the unlit count to thirty times it, frame by frame; 37 of 60 are over
1,500. With the producer off none is. The lit frames show the warm cast on the grilles either side
of the nozzle, the nozzle's rim, the inner faces of the rear fins and the bay's side walls. **The
stored pair's frame is a draw near zero range**: its count is below the unlit mean. The flame in the
nozzle and its glow on the floor are in every frame of both sets, so that glow is the flare model's
and not the light's.

**The flicker is the range draw (90).** `EngineFlare_Update` (`0x017366c0`) computes the jitter's `w`
as a product plus the constant at `0x01863a20` (`-1.0`, one reader). With the product removed
(`VMULSS xmm1, xmm3, xmm1` at `0x017384cf` rewritten to `VXORPS xmm1, xmm1, xmm1`) and the constant
written, every craft's record reads exactly `Radius + w` (the player's: `0.25, 0.5, 0.75, 1.0, 1.25,
1.5, 1.75, 2.0`; at `w = -1.0` the player has no record, its range being under `0.01`). Four frames
at each:

| Forced range (`Radius 1.0 + w`) | Warm pixels, mean of 4 | Summed `R - max(G, B)` over them |
| --- | --- | --- |
| none (`w = -1.0`) | 496 | 25,120 |
| 0.25 | 454 | 22,901 |
| 0.5 | 921 | 37,380 |
| 0.75 | 1,707 | 63,696 |
| 1.0 | 3,275 | 106,450 |
| 1.25 | 7,672 | 287,589 |
| 1.5 | 9,245 | 355,657 |
| 1.75 | 12,548 | 519,447 |
| 2.0 | 13,182 | 550,931 |

A uniform draw over that curve has a mean near 5,500, against the 4,712 measured over 60 shipped
frames: the shipped flicker is this curve sampled at random. How often the vector is redrawn per
second was not measured (frames 0.45 s apart never repeat).

**Ours against it, on the one frame there is.** The held `engine-light-wire` render of this pair
(tick 100) counts 18,577 warm pixels with a summed excess of 1,230,348, against 1,524 and 60,785 for
the same render without the light. That is over the original's count at its largest range (13,182)
and 2.2 times its excess there (550,931). Ours' range at that tick was not recorded, so what this
says is bounded: ours' brightest is brighter than the original's brightest. Ours' unlit hull is
already 1.5 to 1.9 times the original's in the housing box (`(66, 63, 73)` against `(35, 43, 42)`),
and the light multiplies the same texture and vertex colour, so the excess is the size the hull's
own over-brightness predicts; it is not shown to be that and nothing else (confidence 55).

**Queens Mall, the 2048-era light (85).** 20 shipped frames against 10 with the producer off, floor
box `(1140, 650)-(1260, 800)`: `(176.1, 125.0, 84.9)` against `(172.5, 125.2, 85.6)`, a rise of
`+3.6` red and nothing else (frame-to-frame deviation 0.7 to 0.9). With the three bases times four,
`(189.8, 126.2, 83.6)`: `+17.3`, linear. The held render's floor box moves `+4.8` red
(`162.2` to `167.0`). So the 2048-era light is a dim red pool on the floor, and ours' matches it
within the measurement. The housing box there moves more between frames (deviation 10, the camera
bobs) than the light moves it.

**The other producers, as far as the nozzle needs.** On the grid the table holds the eight engine
lights and nothing else, so no other light is at the nozzle at rest. `FUN_01307d40`, the ship's
pre-draw (the engine light's one caller), appends one more record itself when `ship+0x770c > 0`:
position the ship's own, colour `(100, 50, 50)` (`0x01a85418`), range `133.33 * ship+0x770c`; it was
never in a grid table. Seen in passing once the field raced: a pair at one position, `(500, 200, 50)`
range 40 and `(20, 5, 0.5)` range 100 shrinking to 0 over half a second, and a `(0.21, 0.3, 1.0)`
range 15. None was traced to its producer.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x016f5a00` (**base**; `0x0173e350` on 1.07) | `EngineFlare_SubmitVertexLight` | 82 | Decompiled on 1.07 with every constant read as bytes; 2048's executable carries the same function; the shared branch is HD's live-captured law. The base address is the function whose first 24 bytes equal 1.07's (`55 48 89 e5 41 57 41 56 41 54 53 48 83 ec 20 49 89 fc 49 8b bc 24 18 01`) and whose RIP-relative loads hit the base constant block `0x018053fc` at the same offsets on both builds (`+0x37`, `+0x74`, `+0x7c`, `+0x9f`, `+0x10c`); its body was not decompiled on the base build, hence 82 and not 85. |

| `0x016eeb10` (**base**; `0x017366c0` on 1.07) | `EngineFlare_Update` | 85 | Writes the jitter vector `this+0x270` (`0x017384f5` on 1.07) and counts `this+0x260` down. Live on 1.07: with its `w` product removed at `0x017384cf` and its addend `0x01863a20` written, all eight craft's table ranges read `Radius + w` for nine values of `w`. The base address is the function holding the same instruction bytes (`c5 e2 59 c9 c5 f2 58 0d` at `0x016f09a1`, addend `-1.0` at `0x0180537c`, then the same `VMOVUPS [r12 + 0x270]`), entry `55 48 89 e5 41 57 41 56 41 55 41 54 53 48 81 ec 68 01 00 00`; its body was not decompiled on the base build. |

Unnamed, with their readings above: `FUN_0165b890` and `FUN_0165b760` (the pick), `FUN_01334a30`
(the Fury skin flag), `FUN_01335030` (the loader; the base build's is `FUN_0130e090`), `FUN_01307d40` (the ship's pre-draw, the producer's caller), `FUN_0173c810` (arms the tint). They are 1.07 addresses with no base twin located,
which `names.tsv` does not key on.
