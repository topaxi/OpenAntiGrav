# Ship shadows: the two gates, the light they cast along, and what is still unread

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007`, patch v1.04), image
base `0x81000000`. Read 2026-10-19 (`omega-2048-original-shadows`) from the
two `Lighting.Debug.Draw Ship ...` keys outward. Names are applied from
[names.tsv](names.tsv); every one is below 70, so each carries `_q`.

| Address | Name | Confidence | Role |
| --- | --- | ---: | --- |
| `0x8101b70c` | `ShipShadow_BuildLightMap_q` | 65 | builds the ship-shadow light matrix and binds the two maps |
| `0x8102d9e8` | `ShipShadow_DrawCasters_q` | 60 | draws the craft into that pass |
| `0x8102d92a` | `ShipEnvShadow_DrawAll_q` | 62 | per ship, the environment-over-craft pass |
| `0x8102d748` | `ShipEnvShadow_RenderForShip_q` | 62 | one ship's environment map |

## The two gates are live, on, and authored by no circuit

`Environment_RegisterLightingSchema` (`0x810175fc`, [lighting-schema.md](lighting-schema.md))
registers `Lighting.Debug.Draw Ship Shadows` at `EnvInfo + 0x4a4` (string
`0x81508ad4`, call `0x81017cc4`) and `Lighting.Debug.Draw Ship Env Shadows` at
`EnvInfo + 0x4a5` (string `0x81508af8`, call `0x81017cdc`). The bytes are
`0x815cb9cc` and `0x815cb9cd` (`0x815cb528 + 0x4a4`/`0x4a5`).

- **Default `1` for both** (confidence 90): the registrar's `movs r0,#1` at
  `0x810177ac` is the register stored by `strb.w` at `0x81017862` (`+0x4a5`) and
  `0x8101786a` (`+0x4a4`) with nothing between them writing `r0`.
- **No shipped circuit authors either key.** Ten base circuits' `track.EnvSettings`
  (`altima arena bridge cathedral mall park sol square subway tower`) carry no
  `Debug.` line at all, so the default is what runs (confidence 85; the DLC
  circuits were not walked).
- **Read live, 2026-10-19:** a Vita3K guest read of `0x815cb9cc..0x815cb9cd`
  in a Metro Park time trial returned `01 01` (confidence 90 that both gates are
  on in a race, and that these are the right bytes: the read is what a changed
  default would break).
- **The only readers** are both in `FUN_8101bc7a`, the per-frame scene setup
  (`ldrb.w r0,[r0,#0x4a4]` at `0x8101ce14`, `[r0,#0x4a5]` at `0x8101ce24`, after a
  `bl 0x810175fc`). A whole-program `search_instructions` for both displacements
  finds exactly these two.

## Which light: `Ship shadow direction`, not `ShadowLight`

`Environment_RegisterLightingSchema` binds `0x815d13a0` to the string at
`0x815085d0` (`Lighting.Sun direction`, call `0x810178f0`) and `0x815d13b0` to the
string at `0x815085e8` (`Lighting.Ship shadow direction`, call `0x81017908`).
Confidence 90 on both identities (the strings were read from memory).

| Pass | Gate | Direction read | Address |
| --- | --- | --- | --- |
| ship map (`FUN_8101b70c`) | `+0x4a4` | **`Lighting.Ship shadow direction`** (`0x815d13b0`, read at `0x8101b732`) | confidence 80 |
| environment onto craft (`FUN_8102d92a`) | `+0x4a5` | `Lighting.Sun direction` (`0x815d13a0`, loaded at `0x8101ce36` and passed in `q0`) | confidence 70 |

**This corrects the lane brief and `shadows.md`'s "cast along `ShadowLight`".**
`Lighting.ShadowLight direction` is the `shadowLightDirection` uniform (global
slot `0x11` of the scene constant table built in `FUN_8101d068`), a *shading*
term the nova prelit shaders use; nothing in either shadow pass reads it
(`FUN_8101b70c` reads `0x815d13b0` only). The shipped values differ per
circuit: altima `0.45 0.60 1.20` (not unit length, so the magnitude is
discarded: the pass normalises), park `-0.545 0.5299 0.6496` (equal to its
`Sun direction`), square `-0.102 0.7245 0.6817`, sol `-0.217 0.348 -0.700`. The
default is `(0.099, 0.990, 0.099)`, nearly straight up (`DAT_815d13b0`,
`0x3f7d7a4f3dcac83f`, `0x3dcac83f`).

## What the passes do (read, not yet run against the original)

Confidence 60-70 each, from the decompile; the instruction-level numbers are
quoted so the next reader can check them.

- **`FUN_8101b70c` (`ShipShadow_BuildLightMap_q`, 65).** Builds a view from the
  ship-shadow vector with up `(0,1,0)` (`local_d8 = (0,1,0)`), takes eight
  frustum corners, scales by `0x42700000` (60.0) and fits a box to them, then
  writes one matrix to `DAT_815cbf40` that it uploads to **both** global slot
  `0x67` (`shadowMatrix`) and slot `0x6a` (`occlusionMatrix`), binds
  `DAT_816c46f0` to slot `0x66` (`shadowMap`) and `DAT_816c46b0` to slot `0x69`
  (`occlusionMap`) through `FUN_81042b04`/`FUN_81042afa`, and writes
  `(0.5 / w, 0.5 / h, 0, 0)` of the map to slot `0x68` (`shadowFactors`). The slot
  names are the rows of the table built in `FUN_8101d068`
  (`FUN_812f3f8c(&DAT_815cd380, 0x66, "shadowMap", ...)` and its neighbours). It
  also calls `FUN_8102d9e8` (below). **The fit is one map for the whole view,
  sized by the camera frustum, not one map per craft as HD's is.**
- **`FUN_8102d9e8` (`ShipShadow_DrawCasters_q`, 60).** Walks the racer table
  (`DAT_815d3980`, count `DAT_815d4f50`) and draws every entry whose flag word
  (`DAT_815d7e10 + i*0x250`) has bit `0x2` set, or whose mesh's material flag
  `+0x60` has bit `0x400`, into the pass the previous function opened. The
  casters are the craft (confidence 65).
- **`FUN_8102d92a` (`ShipEnvShadow_DrawAll_q`, 62) and `FUN_8102d748`
  (`ShipEnvShadow_RenderForShip_q`, 62).** For each racer with flag bit `0x2`,
  build a per-ship matrix at `ship + 0x88` from the *sun* vector and, for every
  track chunk whose bounding sphere satisfies `|(centre - ship) x dir|^2 <=
  r^2 + 100`, draw its sub-meshes whose flag word has bit `0x2`. Those are
  environment pieces over or beside the craft, drawn from the light into a map of
  their own, so this is the **environment-casts-onto-craft** half, not the craft's
  shadow on the road. Not traced further.

## What receives, and what the receiver does with it

- **Receivers are the track's own materials.** 2,075 fragment-program
  occurrences across `data.psarc`, `data1.psarc` and `data2.psarc` (base and
  patch copies counted separately; `gxp.md`'s 1,799 is the deduplicated count)
  bind `shadowMap` (a
  single-channel sampler at texture unit 3, beside `lightmap` at unit 1 and
  `occlusionMap` at unit 2); `scripts/vita-gxp.py grep` lists them. The global
  slots `0x66` and `0x69` above are what those samplers resolve to.
- **The receiver reads as a depth compare, not a plain coverage lookup**
  (confidence 70). Vita3K's `.dsm` log of `fc06_lambert_simple.rcsmaterial`
  fragment program #14 (the variant that binds `shadowMap`; a log's file name is
  the SHA-256 of the whole GXP container, which matched 5 of 5 programs tried)
  reads, next to the same material's unshadowed #16:

  ```text
  VPCKF16F32 pa0.x pa10.x                       ; fragment depth in the light, to f16
  VF16ADD    pa0.x -pa0.w pa0.x                 ; depth - stored sample
  VMOVC.f16  pa0.xy (pa0.xy < 0) ? pa6.xx : sa10.xx
  VF16MUL    pa0.xyz pa2.www pa0.xxx            ; scales the lit term by the chosen value
  ```

  The same subtract-then-select appears in a material with a different literal
  table, `diffuse_normal_specular` #14 (`VMOVC ... (pa0.xy < 0) ? pa14.xx :
  sa14.xx`), so the shape is not one compiler accident. It differs from HD,
  whose receiver block has no compare because the RSX compares inside the
  depth-texture sample.
- **What the two values are (confidence 65).** The select's constant register
  moves with the program's literal layout: `sa10` in `fc06_lambert_simple` #14
  (declared uniforms end with `fogColour` at words 6-9) and `sa14` in
  `diffuse_normal_specular` #14 (`fogColour` at words 10-13), in each case the
  first word past the declared uniforms, where the first literal record lives
  (index 2, `f16` pair `(1.0, 0.0)`). In `diffuse_normal_specular` the same
  `sa14.x` is also what `VF16ADD sa6.xyz -sa6.www sa14.xxx` subtracts from
  `shadowLightSpecularDiffuse.w` (a `1 - w`), which reads as a 1.0. So **one
  branch of the select is the literal 1.0 (no darkening)** and **the other is an
  interpolant, `pa6.x` / `pa14.x`, written by the vertex program**: the
  shadowed strength comes per vertex, not from a constant in the fragment
  program. **Which branch is the lit one is not read** (the depth orientation
  is unknown), and the vertex program that fills the interpolant is unread, so
  **no strength is claimed for 2048 or Omega.**

## What the original draws, on Vita3K

- **`Ship shadow direction` is live where the executable says** (confidence 90):
  a guest read of `0x815d13b0..0x815d13bb` on the Metro Park grid returned
  `(-0.545, 0.5299, 0.6496)`, `park`'s authored `Lighting.Ship shadow direction`
  to the digit. Nothing rewrites it within seconds of a write.
- **Both flags and the vector were driven live** (`W 0x815cb9cc`, `W 0x815d13b0`
  through `scripts/vita3k-mem.py`, a guest-memory daemon that owns Vita3K) on
  the sunlit Metro Park grid: the authored vector, `(1, 0.25, 0)`, `(-1, 0.25, 0)`
  and `(0.01, 1, 0.01)`, each with the gate on and off, six stills read by eye.
  **No streak appears, moves or disappears in any of them**: the road behind the
  craft is the same frame in every state (a dark ellipse right of centre is in
  all of them, and in the flag-off ones: baked). So Vita3K draws no craft
  shadow from this pass on the grid. **That is a blocker of its own and not
  "unmeasured"**: either Vita3K does not render the depth-compared projected
  map (the receiver's compare needs the map bound and the depth texture
  sampled as depth), or the pass does not reach this material in this view.
  Nothing here separates the two, and neither is a statement about a real Vita.
- **Earlier pixel comparisons are void.** A paused photo-mode scene gave an exact
  0-pixel difference across all four flag states, which means that frame was not
  being redrawn (the pause menu freezes the scene), so it says nothing about the
  flags. In a live drive and on the grid the camera moves between captures, which
  swamps a pixel difference. The vector experiment above is the test that
  does not depend on either.
- **Strength cannot be measured from stills for the same reason**: with no
  craft shadow in any frame there is no in-shadow against out-of-shadow
  luminance to take.

## Omega

Checked, differs (confidence 75), from `/omega/eboot-ps4-omega-eu.bin`:

- **Absent:** `Lighting.Ship shadow direction`, `Lighting.Debug.Draw Ship
  Shadows`, `Lighting.Debug.Draw Ship Env Shadows` and `Lighting.ShadowLight
  specular colour` return no string match. Omega's `ShadowLight` keys are
  `direction` (`0x01829a1e`) and `specular power scale` (`0x01829a3d`).
- **Present, not in 2048:** `shadowCoeffs` (`0x0182aa3e`, beside `shadowMap` and
  `shadowMatrix`), `CastShipShadows` / `CastNoShipShadows` (`0x0182ba32`, `0x0182ba42`,
  referenced from `FUN_015fb4c0`) and `Lighting.Ship diffuse brightness (shadow)`
  (`0x01829b21`, destination `R14 + 0x168`, registered in `FUN_015c1f20`).
- **`ShadowLight direction`** (destination `0x01e3de40`) is read and normalised at
  `0x015cb394` in `FUN_015c63a0` as a shading term, the same role as 2048's.
- **The cast axis for Omega is not recovered**: there is no `Ship shadow direction`
  key to read it from, so it is either `Lighting.Sun direction`, the shadow-light
  vector, or hard-wired. `FUN_015c63a0` is a 55 KB function; not read further.

## Open

- The vertex program that fills the shadowed branch's interpolant (`pa6.x` in
  `fc06_lambert_simple` #14), and which branch of the select is the lit one.
- Whether Vita3K can render this pass at all: read its handling of a
  depth-format render target sampled as a texture, or capture the original on
  a real Vita (none available). Until then no still can confirm the mechanism.
- `FUN_8101bc7a` (the per-frame scene setup) holds the dispatch for both passes
  and is unnamed.
- Omega's cast axis, from `FUN_015c63a0` or the `CastShipShadows` use at
  `0x016025e1`.
