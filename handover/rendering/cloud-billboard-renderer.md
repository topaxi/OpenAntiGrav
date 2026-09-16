# A cloud renderer is real GPU-pipeline work, not yet done - the parser and the RE are

2026-09-16. Working the cloud/sea lane of `docs/rendering/README.md`'s
"cloud and sea classes are not implemented" line. **Decoded, not drawn.** The
parser (`oag_vex::cloud`) and the executable evidence
(`docs/ghidra/functions/psp-pulse-usa/clouds.md`) both landed; a renderer did
not, on a deliberate scope call rather than a decode failure - see below for
why, and what the next pass needs to actually draw one.

## What is already true, so a renderer does not re-derive it

- `oag_vex::cloud::clouds(data, nodes)` returns every `cloudCube` on a file,
  each with a world position (composed through both `Transform` **and**
  `cloudGroup` ancestors - `cloudGroup` carries its own `Transform`-shaped
  placement matrix, and `05_Track` nests one inside another) and a
  `CloudAttributes`: `overlap`, `seed`, `sprite_radius`, `sprite_radius_var`,
  `hi_colour`/`hi_alpha`, `mid_colour`, `lo_colour`/`lo_alpha`, `midpoint`.
  All real, measured values - not invented.
- The shared texture is `Data\Tex\Cloud\Wipeout_Clouds_D_128x64x4.mip`,
  confirmed present in `Data.wad` (11,920 bytes;
  `crates/vex/tests/cloud_ground_truth.rs::the_shared_cloud_texture_is_in_data_wad`).
  `oag_texture::texture` already decodes this exact shape - 128x64, the same
  mip-chain byte count as `grabbedEngineFlare128x64x8.mip` - so nothing new is
  needed on the texture-decode side.
- `05_Track` is the only Pulse circuit that authors any cloud node, on either
  PSP pressing (20 `cloudCube` leaves, 12 `cloudGroup` nodes, across
  `Data.wad`/`FEData.wad`/`BEData.wad`/`FE.wad` on both). A renderer only ever
  has to draw a handful of sprites, never a whole-track system.
- `cloudGroup` is the class that draws (`0x0893280c`, its own method-table
  slot); `cloudCube`'s draw slot is the inherited no-op. The original's
  billboard **rotates** - `vsin_s`/`vcos_s` of a per-instance running phase,
  transformed through a copy of the camera's own view-matrix stack
  (`vtfm4_q`) - not a static camera-facing quad. See `clouds.md` for the full
  read.

## Why nothing draws yet

The natural template is `oag_render::exhaust` - a camera-facing billboard
with its own texture, its own tiny `wgpu::RenderPipeline`, wired into
`race::Scene`/`race::load` the way the flare and its trail are. That module
is 1,425 lines plus a dedicated WGSL shader plus `FlareTexture` plus the
Scene/load wiring, and a cloud renderer would be the second thing built in
that shape - not a small addition to an existing one. Building it well
(texture bind group, uniform buffer, blend state, per-frame view-facing
update, Scene integration, a screenshot check on `05_Track`) is a genuine
multi-hour task in its own right, and doing it hastily risks exactly the
"stand-in that reads as legible" trap `CLAUDE.md`'s "Never invent what the
assets already author" section warns about - a flat, non-rotating billboard
would *look* like a cloud and would not be what the disc authors.

## Open

- **No GPU pipeline exists.** Needs a module in `exhaust::Pipeline`'s shape:
  own shader, own texture bind group (loading
  `Data\Tex\Cloud\Wipeout_Clouds_D_128x64x4.mip` via `oag_texture::texture`),
  a vertex buffer built from `cloud::clouds()`'s output, wired into
  `race::Scene`/`race::load` next to `build_sky`/`exhaust::Pipeline`.
- **The rotation is not pinned down.** `CloudGroup_Draw`'s phase rate and
  `vcst_s(5)`'s value were read as addresses, not as the seconds-per-radian
  constant a renderer needs - a live PPSSPP breakpoint capture (the same
  method `weatherpos.md` and `exhaust.md`'s flare-size correction used) would
  settle it in one session. A first renderer could reasonably ship a
  **static** billboard instead and say so plainly, rather than guessing a
  rotation speed - but should not silently drop the rotation without a note,
  since the original's is a real, measured behaviour.
- **GE state (blend, depth, fog) for the cloud draw is not read from a
  display list at all** - `CloudGroup_Draw`'s disassembly was read for the
  billboard/rotation shape, not its `Gu_*` calls. `exhaust.md`'s
  `ExhaustFlare_Draw` section is the worked example of pulling a blend
  function and depth state out of a display list; the same pass has not been
  done here.
- **`Seed`'s random draw is not reproducible.** Every shipped `cloudGroup`
  leaves it unset, so the original re-rolls `Psys_RandIntRange(1, 9999)` at
  construction; this project's parser reports the authored `0.0` (correct for
  a deterministic reimplementation) but a renderer that wants the *exact*
  jitter cannot get it from this data alone.
- **`cloudCube`'s `kind` (always `2`) and `scale` (always `1.0`) never vary**
  across the ten shipped instances, so neither field's meaning is testable
  against real data. Not blocking - a renderer can ignore both - but
  recorded so nobody later treats a hardcoded `2`/`1.0` as load-bearing.

## Next Steps

1. A live PPSSPP capture on `05_Track` to pin `CloudGroup_Draw`'s rotation
   rate and its display list's GE state - one session, same method as
   `weatherpos.md`'s constructor confirmation.
2. A `crates/render/src/cloud.rs` in `exhaust::Pipeline`'s shape: texture
   load, bind group, shader, Scene/load wiring. `cloud::clouds()` already
   hands it everything else - positions, radii, the colour ramp.
3. A screenshot check: `just view 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/DATA.WAD' --track 'Data\Environments\05_Track\track.vex' --screenshot out.png` (or `oag-game --play` on `05_Track`), read with the Read tool, described honestly against what `clouds.md` says the original does.
