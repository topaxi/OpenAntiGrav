# HD fidelity lanes queued behind the cold originals

2026-10-21, the render-fidelity drive's queue at wind-down, written down so it survives the session. Each
item has its evidence in a `docs/` page; this file only says what is left and in what order. Every
HD item is judged against the HD originals re-made cold at 1920x1080, Resolution Scale 100
(`docs/reverse-engineering/rpcs3-capture.md`), not the thirteen stored at scale 150, and none may raise
another HD pair's mean grey difference by more than about 0.1-0.2.

## Open

**The maintainer's eight scenery observations (2026-10-22, `hd-scenery-materials`)**, judged on the cold pairs by a box away
from the craft (mean grey difference); 1-4 done, 5-8 not started. Evidence pages are named per item.

1. ~~Talon's white grills~~ **wired**: `scanlinebillboard` (renderer.md, "`scanlinebillboard` is the fourth shape"). Box
   51.08 to 43.57. Left: the original adds in gamma space, ours in linear (the panel reads greyer); the add is also about 3x
   the program's `overlay * tint / 2`, source unread.
2. ~~Anulpha's black chevron panel~~ **wired** (2026-10-22, `hd-texture-slots`; hd-track-tone.md, "Six material textures"):
   route 2, group 1 carries six textures; the emask rides the fifth binding and the wave the sixth, the normal map stays
   third, the offset is the material's curve. Left: the phase (at the pair's clock ours shows the outline the re-capture
   does not, box 8.84 to 16.75; off-panel phases 5.25; the scenery clock is chosen, not measured) and the two units'
   decode (chosen). **Blocker for merge, see below.**
3. ~~Moa Therma's cut-off trees~~ **wired** (2026-10-22, `hd-tree-depth`; material-state.md, "The race pass sets pass bits
   3, 5 and 6"; hd-track-tone.md, "Moa Therma's trees"). A live frame shows the trees unblended with alpha-to-coverage and
   depth write; `Looks::material_render_state` (HD measured) moves state-bit-7 draws to the cutout list at a chosen 0.5 and
   makes bit-2 blends write depth. Box 25.62 to 16.70 on the re-captured pair; no pair worse than +0.004. Left open: the
   crowns' edges are hard where the console's two-sample coverage softens them (with `--msaa` on, real
   `alpha_to_coverage_enabled` could replace the threshold; unbuilt); Omega and 2048 author the same `0xbd` trees and
   bit-2 blends but their `psp2::transparency::route` ignores both bits (checked, applies, not wired).
4. ~~Sebenco's bare rocks~~ **wired** (2026-10-22, `hd-texture-slots`; rcsmaterial.md, "Sebenco's snow-capped rocks"):
   the rock is mixed in by the painted mask, not a slope (confidence 85); twelve slots, both lit variants. Box 112.65 to
   9.72, frame 13.11 to 8.65. Left: `pale` raw here and decoded in `ice` (both chosen), block #5's light, and
   `sebenco_snow_just_snow` (a different program, unread).
5. Modesto glass / banners / building texture; 6. Vineta's light and sky; 7. Chenghou's windows; 8. Tech de Ra's missing
   details: **not started** (Tech de Ra carries 9 `scanlinebillboard` slots, now bound, none in the pair's view).

**The six-texture layout on a 16-texture adapter** (2026-10-22, `hd-texture-slots`): the mesh fragment stage now binds 18
sampled textures; an adapter limited to 16 (Intel WebGPU, GL/WebGL2, wgpu's default limits) fails the layout outright.
Either a layout per device with the chevron and snow claims gated on it, or two group-2 bindings freed (the Zone outer
pair folded into a 2-layer array with the inner). Table of limits: hd-track-tone.md, "Six material textures".

Tooling found on the way: `OAG_ONLY_SLOT`, or an `OAG_SKIP_SLOT` list covering a whole draw list, panics with
"buffer slice can not be empty" (an empty list reaches `wgpu`); skip in chunks of about 40 slots.

- **Moa Therma pair** (`hd/moa-therma`): 15.05 to 10.32 mean grey (2026-10-21, `hd-one-pair-moa`; the other twelve HD pairs 0.00).
  The glass cause is closed: `paraboloidReflectionTex` is the circuit's `sky.gtf` as a 512x256 dual hemisphere map and
  `dc_windows_a` reads it (`docs/rendering/hd-track-tone.md`, "The reflection map is the sky"). Left on this pair: the cast
  shadow under the craft (the original's is harder), the teal advert wall at the top right (ours 28 grey brighter in its cell,
  slot not found by bisection), texture sharpness, and the glass program's normal map.
- **The other families that declare the sky map** (`glass_reflect`, `ds_booth_glass`, `glasstest`, `mt_windows_a` on Anulpha Pass,
  the water sheets of Vineta K, the mag floors, the pipe family, the ice): the map and its binding exist
  (`mesh::sky_paraboloid`, group 2 binding 17); each family needs its program block read and a `rcs::window_glass`-style tag, one
  per commit with all 13 pairs re-judged. The tint is `ProbeTint::SCENERY` (`{1, 1, 1, 0}`, read on Moa's glass only); a second circuit's
  read would settle whether it is per circuit. The producer program itself is still undecoded.
- **HD's exposure adaptation law** (`docs/rendering/hd-track-tone.md`, "Decide the adaptation law"): the
  live readback matches the mean of the *encoded* per-pixel luma within 1.3 % on three circuits, ours
  encodes the linear mean (15-43 % high). The patch is six lines in `crates/post/shaders/hd_bloom.wesl`
  (`fs_reduce_first` returns the encoded value, `fs_adapt` drops its `pow`); held until judged on the
  cold originals, and the static read of `FUN_003b4690` says linear, so the console's rule is still open.
- **HD's `-1.5` material LOD bias** (`material_lod_bias: Rule::UNREAD`): **re-judged 2026-10-22 on the cold originals, still held off**
  (`docs/rendering/hd-track-tone.md`, "Re-judged on the cold originals"): 12 of 13 pairs' mean grey up, 8 by over 0.2, the Modesto
  tower box 3.2x the original's Laplacian variance already. It does bring back the glass grid's lines (the section's blurred metric
  0.0304 to 0.0410 against 0.0379). **Open: a bias on the glass sheen's grid tap alone** (a chosen scope), and whether RPCS3 applies
  the filter word's bias at all. Anisotropy: the original asks for none, the pair profile draws ours with none, the default is 16x.
- **Glass road mid and far brightness** (Tech de Ra, Talon's): the original's pixel implies the facing
  ramp read at `u` about 0.36-0.5 where ours reads 0.10-0.20. Next: dump draw 354's vertex constant
  `c[209]` (`eyePositionWorldSpace`, `0x3466fc0e`) at the cold pair's tick against our view eye, then
  the chunk's packed normal (same page, same section).
- **The chevron's emissive pulse** (`cf_chevron_pulse`, `docs/rendering/hd-track-tone.md` item 3): the
  pulse's colour and phase (`emask.a * colour * wave`) and its `+ emask.rgb` diffuse term are unwired.
- **`m_diffuse_normal_constantspecular`'s normal alpha**: the RGB is a normal map
  (`docs/formats/rcsmaterial.md`); what its alpha carries is unread.

## Later, not queued

- **Pure's trail queue position**: Pulse PSP queues the trail at `0x4d000000` before list B
  (`docs/rendering/draw-order.md`); whether Pure does the same is unread.
- **The HD race pair's lean**: the PSP race pairs are re-made with the drawn lean (`--pose-lean`); the HD
  race pair (`hd-race/talons-pack`) is not.

## Next Steps

1. Adopt the cold HD originals, run the provenance scan, `just compare-rerender-ours --only /hd/`.
2. Spawn the Moa Therma lane against them.
3. Then, in order: the exposure law, the glass eye dump, the chevron pulse.
