# HD's shadowed track redraw: how dark, and how big the per-ship map is

2026-10-15 (`hd-shadow`). Read because `docs/rendering/shadows.md` carried two
numbers of this project's own in the `original` tier on Wipeout HD:
`MAP_STRENGTH = 0.5` (how dark a covered texel draws) and `shadow::map::SIZE =
1024`, with the original's side of the table reading "the colour that variant
computes still unread" and "`shadowMapTexSize`, value unread".
[`shadow-model-maps.md`](shadow-model-maps.md) had already found the accumulate
half of `RenderModelShadowsOnTrack` (`Shadow_CompileShadowedTrackRedraw`,
`0x004053e0`) and that it binds the `ShadowToAlpha` permutation; this page
reads what that permutation computes and who writes `shadowMapTexSize`.

Static only, so every score caps at 84 per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md). Code was
disassembled from the image with `capstone` over `scripts/ps3-toc.py`'s `Image`
(PPC64 big-endian, `skipdata` on - without it the decoder stops at the first
data word); each TOC-relative load was resolved with `scripts/ps3-toc.py
resolve` against the function's own TOC, which for `0x004053e0` is
`0x008bd3c4` (`exact`). Fragment microcode is `scripts/ps3-microcode.py`'s.

## The names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x004053e0` | function | `Shadow_CompileShadowedTrackRedraw` | 82 |
| `0x003e9660` | function | `Ship_RankAndAssignMapSlots` | 62 |
| `0x006d1a98` | function | `Ship_CompareDistanceSqLess` | 78 |
| `0x00d1c838` | data | `g_ShipMapSlotInUse` | 78 |
| `0x00d1c8e0` | data | `g_ShipMapRankList` | 78 |
| `0x00c8b218` | data | `g_ShipDistanceSq` | 78 |

`Shadow_CompileShadowedTrackRedraw` was 75 on `shadow-model-maps.md` (the
parameter slots, the blend state and the chunk cull were read; what it *draws
with* was not). It is 82 here: the permutation word it asks for is read to the
last bit and the program that word selects is decoded. The three data names are
the ship record array's neighbours at `0x00c86880 + 0x95fb8`, `+ 0x96060` and
`+ 0x4998` (the base is `TOC[-0x5624]` = `0x008b7da0` -> `0x00c86880`, the same
array `shadow-model-maps.md` walks). The slot allocator itself lives inside
`ModelRecord_Load` (`0x003eeb08`, 65 on `renderer.md`).

## How dark: the redraw is the road lit without its sun

### The permutation the redraw asks for

`Shadow_CompileShadowedTrackRedraw` builds one twelve-bit permutation word per
track chunk and looks it up through `0x003f0ff8` (the table
[`rcsmaterial.md`](../../../formats/rcsmaterial.md#the-permutation-word-is-read-and-so-is-the-lit-race-pass-2026-08-20)
decodes). Every term, from `0x00405c60` to `0x004060d0`:

| Instruction | Term | Bits |
| --- | --- | --- |
| `0x00405c64..0x00405c98` | the chunk's own ambient source, from `rldicl`/`rlwinm` on `material+8`, stored at `0x154(r1)`; zero when `0x4b0(r10)` is clear | `IleVertex` (`0b010`), `IleLightmap` (`0b100`), or neither (`Ambient`) |
| `0x00405d48` `li r9,0x20` ; `0x00405d54` `or r12,r10,r9` | **`ShadowToAlpha`** | bit 5 |
| `0x00405d70` `slwi r0,r11,3` ; `0x00405d74` `or r12,r12,r0` | the spot-light count from the light object's `+0x78`, only when byte `0x59b` is set | bits 3-4, `Spot0`..`Spot3` |
| `0x00406070` `or r3,r7,r12` | `0x160(r1)`, which is **0**, or `0x400` (`NoAlbedo`) when the debug byte at `0x53a4` is set (`0x00406018..0x00406048`) | bit 10 |
| `0x004060ac` `ori r3,r3,0x800` | `SVC1`, for a chunk with an SPU light list | bit 11 |

**Bit 0, `Sun`, is not set anywhere.** There is no `li`/`ori`/`or` of `1` into
the word on any path, and the two sources of extra bits (`0x154` and `0x160`)
are fully enumerated above. So the redraw selects `ShadowToAlpha` + the chunk's
ambient source + `Spot0` + `SVC0`/`SVC1`, and no `Sun`.

**The disc agrees from the other side.** Every `.rcsmaterial` on
`hdfury-ps3-eu` (1,632 of them, `DATA00`-`DATA06`) was swept for variants whose
word has bit 5 set: **16,554 variants, and `Sun` is clear on all 16,554**.
They split by chunk source (Ambient 7,014 / IleVertex 3,234 / IleLightmap
6,306, never `IBL`) and by spot count (`Spot0` 10,638 / `Spot1` 3,210 / `Spot2`
2,706, `Spot3` never), and none sets `ShadowMap` or `FalseLight`. The compiler's word and the shipped library have no
intersection with `Sun`, which two independent readings (code and data) agree
on: a variant it asked for had to exist.

### What that program computes (Talon's Junction `track_surface`, blocks #8 and #9)

`data/environments/talons_junction/materials/track_surface.rcsmaterial`,
variants #8 `ShadowToAlphaAmbientSpot0SVC0` (fp block `0x6100`) and #9
`ShadowToAlphaIleLightmapSpot0SVC0` (fp `0x6450`); their `SVC1` twins (#13,
#14) share the same fragment blocks. `scripts/ps3-microcode.py fp-file`:

```text
; #8  Ambient            params: fogColour (0x3dc31258), constantAmbientColour (0x81db67ea)
@0x00  MUL R0.x, f[FOGC], fogColour.w
@0x02  MUL R0.w, -R0.x, R0.x                      ; -(density * view_depth)^2
@0x03  ADD H2.xyz, f[TC1], constantAmbientColour  ; light = TC1 + ambient
@0x05  MUL R1.z, R0.w, 1.44269
@0x07  TEX H1.xyz, f[TC4] unit0                   ; albedo
@0x08  MAD R2.xyz, H2, H1, -fogColour             ; light * albedo - fog
@0x0a  EX2_SAT R0.x, R1.z                         ; exp(-(density * depth)^2)
@0x0b  TXP R1.x, f[TC0] unit1                     ; shadowMapTex, compared by the RSX
@0x0c  ADD H0.w, -R1.x, 1                         ; alpha = 1 - sample
@0x0e  MAD /2 H0.xyz, R0.x, R2, fogColour END     ; fog lerp, output scale /2
```

Block #9 is the same with the light `= pow(lightmap, prelitBias) * prelitScale +
f[TC1]` (`TEX unit1`, `LG2`/`EX2`) in place of the constant ambient. The paired
vertex program (`vp-file ... 8`) writes `o[TC1].xyz = c[208].xxxx` where `c[208]`
is the literal `(0,0,0,0)`, so `f[TC1]` is **zero**: the light is the ambient
(or lightmap) alone.

What is *not* in either program: no `directionalLight0*` parameter, no `N.L`, no
half-vector, no specular, and **no shadow-strength parameter of any name** - the
two blocks' parameter lists are exactly `fogColour` + `constantAmbientColour`
(+ `prelitBias`, `prelitScaleSpecular`). `ShadowAlpha`, `shadowFactors` and
`ShadowColour`, the names `shadows.md` listed as candidates, are not in them
(`ShadowColour` is 2048's technique, below). The frame's main pass for the same
chunk (`HalfBrightAmbientSunSpot0SVC0`, block #5) ends on the same `ADD /2`
output scale and the same fog lerp, so the two outputs are in one convention and
blend meaningfully.

### The law

The redraw blends `SRC_ALPHA / ONE_MINUS_SRC_ALPHA` (`0x8006`, `0x302/0x303`,
`0x004054a0..0x004054c8`) over the already-drawn frame, so for a fragment with
map sample `s` (1 lit, 0 shadowed - the RSX compares a depth texture in the
`TXP`) and `F` the sunlit, fogged colour already in the frame:

```text
alpha  = 1 - s                                 (coverage)
shadowed = fogged(albedo * (ambient + prelit))  (no sun, no specular)
result = alpha * shadowed + (1 - alpha) * F
       = F - alpha * (F - shadowed)
```

**So there is no darkness constant.** A covered texel does not darken the road
by a fraction; it shows the road lit as it would be with the sun off, in
proportion to coverage. How dark that is depends on the circuit's own sun
against its own ambient. On Talon's Junction the sun is `(-0.7776, 0.5815,
-0.2391)` at `directionalLight0Colour` and the ambient `(0.404, 0.392, 0.510)`.
**Confidence 82** that this is the colour the redraw writes (static: the word,
the two fragment programs and the blend are read; nothing has been watched under
RPCS3). The doubt left is the stencil volume's own contribution - the redraw is
clipped to the ship's shadow volume, which only restricts *where* the blend
lands, and `Shadow_DrawOccluderStencilVolumes` is read on
[`shadow-stencilvolume.md`](shadow-stencilvolume.md).

## How big: `shadowMapTexSize`

### Who writes it

Engine parameter 42 (`renderer.md`'s table). Slot arithmetic is
[`ship-sun-occlusion.md`](ship-sun-occlusion.md)'s: value pointer at `0x18 +
0x20 i`, count at `0x1c + 0x20 i`, so slot 42 is `0x558` / `0x55c` of the
parameter block `*(ctx+0xd8)`. A sweep of every load/store with `0x558(`/`0x55c(`
(`capstone`, whole `.text`, 105 hits, almost all other objects' fields) has one
pair in the shadow code, inside the redraw compiler. The sweep cannot exclude a
second writer behind a different base, but this one is slot 42's by the slot
arithmetic and by what it stores:

```text
0x00405588  lwz   r11,0(r31)           ; the ship's maps[1] (the depth map's view)
0x004055b0  lwz   r0,0x38(r11)         ; TEXTURE_IMAGE_RECT: (width << 16) | height
0x004055b4  rldicl r9,r0,0x30,0x30     ; width
0x004055bc  clrldi r0,r0,0x30          ; height
0x004055c8..4055f8                     ; f12 = float(width), f13 = float(height)
0x00405590  lfs   f0,-0x504c(r2)       ; = 2.0   (TOC slot 0x008b8378 holds 0x40000000)
0x004055f4  fdivs f13,f0,f13           ; 2.0 / height
0x004055f8  fdivs f0,f0,f12            ; 2.0 / width
0x00405604..40561c                     ; lanes x = 2/width, y = 2/height, z,w kept
0x00405624  stvx  v13,r7,r0            ; the static vec4 at 0x...53b0
0x00405628  stw   r8,0x558(r9)         ; params[0x558] = &that vec4
0x00405634  stw   r9,0x55c(r11)        ; params[0x55c] = 1   (one vec4)
```

So **`shadowMapTexSize` = `(2/width, 2/height, 0, 0)` of the ship's map**,
recomputed for every ship the redraw compiles (the NDC width of one texel, not
the size itself). Its first-ever value is `0x3b800000` = `2^-8` in both lanes
(`0x00405558`: `lis r0,0x3b80`), which is `2/512` - the largest map in the pool
below, so the default is the standard case. `Texture_BuildGcmRegisters`
(`0x005a9998`, `renderer.md`) writes `(width << 16) | height` as the descriptor's
image-rect word at `+0x18` of the view's `+0x20` block, the RSX
`NV4097_SET_TEXTURE_IMAGE_RECT` shape (`0x005a9b64`, `slwi r10,r26,0x10 ; or
r10,r23,r10` with `r26` the width and `r23` the height `Texture_View`
(`0x005aa1d8`) passes; the other dimension path at `0x005a9ab0` stores the same
word).

**Nothing on the disc reads it**: a raw scan for `~crc32("shadowMapTexSize")`
(`0x86a174d7`) over all 1,632 `.rcsmaterial` files finds **0** (`shadowMapTex`
`0x730df9ee` is in 381). The engine publishes it and no shipped material
declares it, so it has no visible effect of its own; what it tells is the map's
real resolution. Confidence 80 for the formula; 84 for "no material reads it"
(a census, not an inference).

### The real resolution

`Shadow_AllocShipMapPool` (`0x003e5e58`, [`ship-sun-occlusion.md`](ship-sun-occlusion.md))
makes a pool of eight records whose depth maps run, by pool slot,
**512, 512, 256, 128, 64, 32, 32, 32** texels square (`g_ShipMapSizes`,
`0x0076a080`, read off the image there; `Texture_View`'s `+0x1c`/`+0x20` width
and height are used as-is when the texture's MSAA field `+0x28` is 0, which the
pool passes). Which ship gets which slot, read here:

1. **Creation order** (`ModelRecord_Load`, `0x003eec30..0x003f0010`): eight
   in-use bytes at `0x00c86880 + 0x95fb8..0x95fbf` (`g_ShipMapSlotInUse`); a
   ship record with flag bit `0x40` set takes the first free one (`7` if all
   are taken), stored at `ship+0x130` and the byte set. `Ship_FreeRenderSlot`
   (`0x003e55d0`) clears it.
2. **Per frame, by distance** (`Ship_RankAndAssignMapSlots`, `0x003e9660`,
   called through a job trampoline at `0x006cdde4` with `job+0x40` and a vec4 at
   `job+0xa0`): for every ship with `+0x140` set and `+0xe4 & 3` non-zero it
   computes the squared distance from the ship record's `+0xd0` position to that
   vec4 (`0x003e9b5c..0x003e9b90`), stores it at `0x00c86880 + 0x4998 + 4*i`
   (`g_ShipDistanceSq`) and the ship index in a list at `+ 0x96060`
   (`g_ShipMapRankList`), sorts the list with `0x006d1fc8` and the comparator
   `Ship_CompareDistanceSqLess` (`0x006d1a98`: `1` when `dist[a] < dist[b]`, so
   **ascending, nearest first**), then writes `ship[list[k]] + 0xe0 = pool +
   16*k` for `k = 0..7` (`0x003e9bc8..0x003e9d00` and the second copy at
   `0x003e7fc0`).

So the two ships nearest the job's reference point get 512 x 512, the third 256,
the fourth 128, the fifth 64 and the rest 32. **What the vec4 at `job+0xa0` is
is not read** (the camera's eye is the obvious reading and it is only that);
confidence 85 that the size is ranked by distance and 62 that the reference is
the eye. The ship map pool is allocated with `0x20`/`9` format codes
`Texture_Create` owns; the `+0` map (the one the redraw samples) is the `0x20`
depth format.

### Texel density against this project's one map

Each ship's box is `Shadow_BuildShipShadowMatrices`' ortho box: its model bbox
through the world matrix, or the `(-6)..(6)` cube when the model has none, so
12 to 21 units across (the cube's face to its space diagonal). The original's
density is therefore 512 / 12..21 = **24 to 43 texels per world unit** for the
two nearest ships, 12 to 21 for the third, 6 to 10 for the fourth. This
renderer fits one map to the whole grid; measured on Talon's Junction's grid
(eight craft, `--race --opponents`) the fit radius was **233.6 units** because
the fit added each drawable's own radius (50 to 200 for an HD `Ship.vex`), which
made the 1024 map **2.2 texels per world unit**. Wiring: see below.

## What was wired

- **Darkness.** `mesh`'s `shadowed` (`crates/mesh/shaders/shadow.wesl`) takes the
  sun back out of the lit colour in proportion to coverage (`colour - coverage *
  sun_delta`), where `shade.wesl` records the sun's share of what it returned
  (`surface * sun_diffuse + specular`, in the output's own space, linear or
  encoded). No darkness constant remains for an HD race. `MAP_STRENGTH` (0.5)
  survives only for a scene with no authored light rig and for the `mapped`
  tier, which is this project's own, and says so.
- **Map size.** `shadow::map::SIZE` 1024 -> **2048**, and the grid fit's per-craft
  term from the drawable's radius to `6 * sqrt(3)`, the bounding sphere of the
  original's own fallback box. At the grid (casters' spread 70 + 10.4 + the 6
  margin, ~172 units across) 2048 texels is **11.9 texels per unit**, the
  original's third ship. **Chosen, not measured**: the original's 512 is per
  ship and a shared map has no equivalent. 4096 would reach the nearest ships'
  density at 16 MiB; one map per craft, each sized by distance rank, is the
  faithful version and is the open alternative.

## Cross-title check

- **2048: checked, differs.** Its executable carries `ShadowColour` and
  `shadowFactors` but neither `ShadowToAlpha`, `shadowMapTexSize` nor any of
  HD's four `Job *Shadow*` names (`strings` over `PCSF00007/patch-v104`), which
  is `shadows.md`'s earlier measurement that 2048 has none of HD's shadow-map
  pipeline; its `original` is the proximity/env system. Nothing here applies.
- **Omega: not checkable.** The PS4 `eboot.bin` carries no readable strings and
  the PS4 material container has no parser, so neither `ShadowToAlpha` nor the
  parameter can be looked for; `Title::original_shadows` keeps Omega `Unbuilt`
  and it draws no coverage map, so the change cannot reach it.
