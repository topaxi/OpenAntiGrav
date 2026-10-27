---
status: ready-to-wire
titles: [hd]
updated: 2026-10-27
summary: "HD fidelity queue: bitangent sign wired 2026-10-27 (2048/Omega read open), Bomb, weapon taps, Amphiseum scene, scenery clock, exposure"
difficulty: medium (35)
---

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
   51.08 to 43.57. ~~Left: the original adds in gamma space, ours in linear~~ **wired 2026-10-22 (`hd-blend-space`)**:
   HD's scene stores encoded values (`Looks::blends_encoded`, confidence 85) and the holo taps are decoded on fetch
   (confidence 80); docs/rendering/hd-track-tone.md, "HD's blends combine encoded values". Left: the grill's source is
   still about 1.5x the original's (scanline phase and layer scroll are the candidates).
2. ~~Anulpha's black chevron panel~~ **wired** (2026-10-22, `hd-texture-slots`; hd-track-tone.md, "Six material textures"):
   route 2, group 1 carries six textures; the emask rides the fifth binding and the wave the sixth, the normal map stays
   third, the offset is the material's curve. Left: the phase (at the pair's clock ours shows the outline the re-capture
   does not, box 8.84 to 16.75; off-panel phases 5.25; the scenery clock is chosen, not measured) and the two units'
   decode (chosen). **Open: measure the clock the curve runs on** (a live read of the original's scenery
   animation time beside the HUD clock on a cold Anulpha frame); until then the outline's phase in any pair is chance.
   The fill, where the outline is off, matches the original's (5 grey levels brighter).
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
5. Modesto ~~glass~~ (**wired 2026-10-22, `hd-glass-families`**: `glass_texture_customr` is a pane, `Texture1` its picture
   and coverage and the sky through `1 - a`; glass box 146.5 to 123.5 grey against 120.0; hd-track-tone.md, "The sky-reflecting
   families") / banners / building texture (**banners at the turn located 2026-10-22, `hd-blend-space`**: slot 773,
   `uvanim_diffuse_emissivealpha` with `egx_lcd.gtf`, an LCD the original draws as a pale scanline grid; its program is
   unread, next is its fragment block and a live Modesto frame's unit gamma nibbles; the encoded store moved the right
   banner box 35.1 to 24.7 only); 6. Vineta's light and sky; 7. ~~Chenghou's windows~~ **wired 2026-10-22, `hd-glass-families`**
   (`glass_2nduv_reflect_glow`: the window picture, the sky through `1 - a` and the glow through the second-UV mask; the green
   strips are back, but the tower box reads 58.9 grey against 54.7, the panes' reflection too bright: open); 8. Tech de Ra's missing
   details: **census done 2026-10-27 (`hd-tech-de-ra-2`)**: the black booth window behind the right-hand railing was
   ~~`ds_booth_glass`~~, **wired** (hd-track-tone.md, "`ds_booth_glass` reflects the sky through its normal map's alpha"; on 12
   of 13 pairs, booth boxes 13-35 grey to 4-7). The rest of the pair's gap is clock phase, not material (see "HD's scenery
   clock" below).

**~~Amphiseum's hologram board~~** (`hd-hologram-board`, 2026-10-27, wired): the surface is
`dc_hologramwithstatic.rcsmaterial` (slot 637), not `hologram.rcsmaterial` (the 2026-10-22 needle
`hologram` was a substring that also matched it). Program read live and wired as glow mode 7
(hd-track-tone.md, "The Amphiseum hologram board"). Box `840,0 240x120`: per-pixel mean grey
difference 90.5 to 26.2 (difference of means +90 to +23.5), the pair's whole frame 9.61 to 8.59, the other 17 HD
pairs unchanged. The bar (within 10) is **not met**: the box now matches the program's own
arithmetic (+13/+23/+24 lift against the expected +15/+23/+24) and the original's strength (scanline stripe
energy ratio ours/theirs 0.87) and the picture reads like the
original's (faint scanline-striped rings), but the box mean grey still sits above the original's
because **the scene behind the board is about 20-25 grey brighter in ours than in the original**
(ours with the board skipped reads `62, 81, 89` against theirs `54, 77, 90` with the board
drawn): next lead is that pillar/wall behind the board, not the material.

**HD's unlit programs read their taps raw** (2026-10-22, `hd-blend-space`; three closed 2026-10-26,
`hd-render-fidelity`): with the scene stored encoded, a tap the console decodes on fetch (gamma nibble 7 in
`SET_TEXTURE_ADDRESS`) and ours reads raw draws too pale. Read live and wired: the engine flame (rest "jewel" now blue
with pink, no rays; nozzle box down 1.5-5.9 grey on 12 of 13 pairs), the trail tube's two colour taps
(`hd-race/talons-pack` whole 46.47 to 46.12) and the Bomb's fireball (yellow where it was near white); the Bomb's
shockwave ring reads raw on the console too (nibble 0), so its white burn is the console's law. Evidence:
engine-trail.md, "The flame's colour tap is decoded on fetch" and "The trail's colour taps are decoded on fetch";
hd-unlit-programs.md, "The Bomb's units, read live". **Still unread**: the Missile blast's three programs, the
LeachBall and its ring, the Plasma trio, the rocket smoke, the absorb shell, the light cone. Method: one
`rsx-draw-list.py --samplers` of a frame that draws them (the stored dumps under `data/reference/hd-flame-tap/` hold
none of these), read unit 0's nibble, decode in `shade.wesl` on `encoded_out` only.

**Fragment-stage texture budget** (2026-10-22, `hd-texture-slots`): the per-slot mesh fragment stage is at 16 sampled
textures (group 1's six, group 2's ten) after the Zone Inner/Outer fold, which is WebGPU's and GL's ceiling. A seventh material
texture or another group-2 texture needs another fold first: candidates are the shadow coverage and depth pair (a tier reads
one), or the sky map as a layer beside the probes (different sizes today, 512x256 against 128x64).

- **Moa Therma pair** (`hd/moa-therma`): 15.05 to 10.32 mean grey (2026-10-21, `hd-one-pair-moa`; the other twelve HD pairs 0.00).
  The glass cause is closed: `paraboloidReflectionTex` is the circuit's `sky.gtf` as a 512x256 dual hemisphere map and
  `dc_windows_a` reads it (`docs/rendering/hd-track-tone.md`, "The reflection map is the sky"). Left on this pair: the cast
  shadow under the craft (the original's is harder), the teal advert wall at the top right (ours 28 grey brighter in its cell,
  slot not found by bisection), texture sharpness, and the glass program's normal map.
- **The other families that declare the sky map**: ~~`mt_windows_a`~~, ~~`glass_texture_customr`~~ and
  ~~`glass_2nduv_reflect_glow`~~ **wired 2026-10-22, `hd-glass-families`** (rows of `rcs::window_glass`'s family table;
  hd-track-tone.md, "The sky-reflecting families"; Anulpha's strip box 2 grey brighter with the normal map still left out). Still
  open: `glass_reflect` (11 circuits), ~~`ds_booth_glass` (11)~~ (**wired 2026-10-27**), `glass_reflect_opacity` (Chenghou 14 slots, Vineta K, `03_track`),
  `glasstest`, the water sheets of Vineta K (`rcs::water`, not started), the mag floors, the pipe family, the ice: each a row once its
  block is read, one per commit with all 13 pairs re-judged. Metropia's `glass_2nduv_reflect_glow` resolves to block `@0x18350`,
  assumed Chenghou's law, unread. The glass normal maps are the shared omission. The tint is `ProbeTint::SCENERY` (`{1, 1, 1, 0}`, read on Moa's glass only); a second circuit's
  read would settle whether it is per circuit. The producer program itself is still undecoded.
- **HD's exposure adaptation law** (`docs/rendering/hd-track-tone.md`, "Decide the adaptation law"): the
  live readback matches the mean of the *encoded* per-pixel luma within 1.3 % on three circuits, ours
  encodes the linear mean (15-43 % high). The patch is six lines in `crates/post/shaders/hd_bloom.wesl`
  (`fs_reduce_first` returns the encoded value, `fs_adapt` drops its `pow`); held until judged on the
  cold originals, and the static read of `FUN_003b4690` says linear, so the console's rule is still open.
- **HD's `-1.5` material LOD bias is on** (2026-10-27, `hd-texture-bias`; `docs/rendering/hd-track-tone.md`, "Texture crispness"). What is left
  of it: the 0.5-level gap against the RPCS3 fit (-0.75 matched, the filter word says -1.5; RPCS3's LOD computation or our
  `textureSampleBias` with derivative scaling), the warm light pool on the Vineta K kerb, and the Tech de Ra glass grid (missing content,
  not blur). Open: a bias on the glass sheen's grid tap alone. Anisotropy: the original asks for none, the default is 16x.
- **Glass road mid and far brightness** (Tech de Ra, Talon's): the original's pixel implies the facing
  ramp read at `u` about 0.36-0.5 where ours reads 0.10-0.20. Next: dump draw 354's vertex constant
  `c[209]` (`eyePositionWorldSpace`, `0x3466fc0e`) at the cold pair's tick against our view eye, then
  the chunk's packed normal (same page, same section). **Measured 2026-10-27 (`hd-tech-de-ra-2`), small, not a lane on its
  own**: the `etched_glass` slots (`OAG_SKIP_MATERIAL=etched_glass`, mask = pixels the skip moves) cover 233,622 px on
  `hd/tech-de-ra` at a blur-7 error of 2.8 and luma bias -0.1 (0.66 M), 4,551 / 4,571 px on the two Talon's pairs (7.5, 9.6;
  bias -7, -10) and 7,685 px on Modesto (14.4, bias -12): 0.85 M over the 13 pairs, against 3.83 M the booth glass cost. The
  eye half is already settled on Talon's (rcsmaterial.md, "The glass floor's gamma, curve and sun weight", item 5: `c[209]` is
  the camera matrix's eye to 1e-4, and the pairs replay that camera), so the open read is the chunk's normal: the vertex
  stream of draw 354 from `hd-draw-dump.py` without `HOOK_LIGHT` (`HOOK_TGT` the scene target), its `normal` attribute against
  `oag_rcs`'s decode of the same chunk.
- ~~**The chevron's emissive pulse**~~ **wired** (2026-10-22, `hd-texture-slots`, item 2 above); what is left is the
  clock its curve runs on.
- **HD's scenery clock** (2026-10-27, `hd-tech-de-ra-2`; one measurement, several symptoms): ours runs every world mesh on the race
  tick over 60 (`Race::world_clock_seconds`, HD pairs record no `--world-clock-offset`), the original on a clock whose offset over the
  HUD clock is unread. On `hd/tech-de-ra` the camera droid sits over the left wall in the original (box `400,400 120x120`, 24 grey,
  luma -23) and over the right-hand stands in ours (+15 there); the `nr_twinblend` GO banner shows another phase of its wipe; on
  `hd/modesto-heights` an airship crosses the tower top right in ours only (box `1300,0 220x240`, 21 grey). The Anulpha chevron's
  outline phase is the same input. **Recipe**: on a cold RPCS3 frame, read the scenery animation time the engine hands
  `Anim Transform` nodes beside the HUD clock (find the writer in `/hdfury/EBOOT-ps3-hdfury-eu.elf` from the node evaluator's time
  argument), store it in `pair.json` as `world_clock_offset_s` the way the Pulse PSP pairs do (`scripts/psp-world-anim-clock.py`) and
  let `compare-rerender-ours` pass it. Never fit the offset to the picture.
- **`m_diffuse_normal_constantspecular`'s normal alpha**: the RGB is a normal map
  (`docs/formats/rcsmaterial.md`); what its alpha carries is unread.

- **Ours is 11-18 % brighter: READY TO WIRE** (2026-10-27, `hd-brightness`; `docs/rendering/hd-track-tone.md`, "The dark-end lift is the final encode"). Cause: `fs_encode` ends on `pow(x, 1/2.2)` where the original's ROP writes the true sRGB curve. Change: swap that one line for an sRGB encode (linear segment below 0.0031308, else `1.055 * x^(1/2.4) - 0.055`). Dark-end (original luma < 60) mean diff on 13 pairs goes from +3..+9 to -2..+1 (Moa +5, Modesto +1.8 stay); box luma 1.18/1.18/1.11 -> 0.95/0.96/1.00. Held on purpose: whole-frame rmse rises 1-3 % because the lift had masked the open bloom deficit (Anulpha mean luma 0.97 -> 0.91), and the `-1.5` bias then reads sharp (fine 1.27) so the crispness call needs a re-check after it lands. The adaptation-law patch does not move it. Size S (15), difficulty low (80).

## Later, not queued

- **Pure's trail queue position**: Pulse PSP queues the trail at `0x4d000000` before list B
  (`docs/rendering/draw-order.md`); whether Pure does the same is unread.
- **The HD race pair's lean**: the PSP race pairs are re-made with the drawn lean (`--pose-lean`); the HD
  race pair (`hd-race/talons-pack`) is not.

## Next Steps

The queue for the next drive, in order (written at the 2026-10-22 wind-down; each item's evidence is in the
section above or the doc page it names). At most one lane in `crates/mesh` at a time.

1. **A WCB-on picture of the Bomb blast** at about 0.7 s and 1.8 s (the unit nibbles are read, 2026-10-26; what is
   missing is the picture to judge the fireball's yellow and the rings' white against). The draw-list capture
   (`rpcs3-hd-whiteout.py --bomb`) stalls reading vertex spans under load and its retries add 0.5 s each: pass
   `--attempts 4 --delay 0.1` and take `drive.screenshot` before the dump. The bomb lands behind the chase camera:
   holding `triangle`, `r3` or `l3` on the virtual pad in a Time Trial does not turn the view (2026-10-26), so the shot
   needs another camera (a rival's bomb ahead of the player, or the player placed behind the dropped bomb).
2. **`hd-tech-de-ra`** (opus, mesh). **2026-10-27 (`hd-bitangent-sign`): slot 384's wash is ~~open~~ wired** (bitangent sign per vertex program; hd-track-tone.md, "Wired"). **Still open from it**: the 2048 and Omega vertex-shader read of the tangent's `.w` (the usse reader classifies words only; `vita-usse.py` and Omega's `eboot.bin`), and `pad_normal`'s derived frame for a chunk without an authored tangent. **Done 2026-10-26 for the glow banner only**: the `glow` needle's +30 on Tech de
   Ra was `nr_twinblend` (slot 133, the banner behind the gantry), now drawn by its program (rcsmaterial.md,
   "`nr_twinblend`: two pictures, a scrolling wipe and bars"; mask bias +1.13). **2026-10-27 (`hd-tech-de-ra-2`)**: the
   missing-details census found `ds_booth_glass` (wired). **Measured, not wired (2026-10-27)**, each by area x error on the cold
   pairs (blur sigma 7, mean abs RGB): the glass road 0.85 M (above, "Glass road mid and far brightness"); **Vineta K's right-hand
   building: the "sunlit bands about 30 grey over" claim is false on today's tree** (box `1300,0 620x540` luma -1.4, no 60x30 block
   over +7.7; the no-dot-power specular went with `hd-top-right-band`), and what is left there is the windows' hue, slot 147
   `jd_simplespecular` (`and_metalshine2.gtf`, no sky sampler in any of its 41 blocks), 6,853 px at 5.6 (0.04 M); **Modesto's
   background**: the tower box `1300,0 220x240` (21 grey) is sky traffic in another phase (an airship in ours, a balloon in the
   original: the scenery clock below), and the left-hand steps behind the crowd are slot 349 (`diffuse`, `steps1.gtf`, no lightmap)
   at +18 luma over 9,812 px (0.17 M). Also found on Tech de Ra: warm highlights along the pit box's top edge (slot 408,
   `track_surface` with `ds_pit_box_cs.gtf`, exponent 50.5) that the original does not show, box `1050,735 400x40` 12.9 grey,
   gone with `OAG_ALBEDO_ONLY=1`, so a light term (about 0.2 M); and **the near-left wall below the left pylon, slot 384**
   (`diffuse_normal_specular_emmissive`, `ds_sf_cs.gtf`), a yellow wash over the original's: box `480,600 240x120` 13.1 grey,
   luma +10.3, the slot's mask 131,478 px at 6.8 (0.90 M, as large as the glass road). **WIRED 2026-10-27 (`hd-bitangent-sign`: sign read per vertex program, Tech de Ra frame 7.47 -> 7.06, Modesto 9.13 -> 8.09, raw circuits unchanged; hulls read raw; 2048/Omega open) - was: READY TO WIRE (`tech-de-ra-wash`): the cause is the bitangent's sign, not a light or glow term.** The original's vertex program scales `B` by `2w - 1` (`w = 0` gives -1) in the four original circuits' compile (Amphiseum, Modesto, Talon's, Tech de Ra, Zone 1 to 4); ours uses raw `w` and drops `ny`. The eight numbered circuits' compile is raw and right. A local edit in `shade.wesl`'s tangent branch (`in.tangent.w * 2 - 1` for `pad_ne` slots and track receivers) took the box 13.08 -> 2.74 (luma +10.3 -> -1.81), the slot's mask 0.917 -> 0.507 M, the Tech de Ra frame 4.205 -> 3.840, Modesto 9.004 -> 7.975 (but Vineta K and Sol 2 worse, because they are raw: **read the sign off the vertex program, never the circuit**). The exact change, the census and the ruled-out terms are in [hd-track-tone.md](../../docs/rendering/hd-track-tone.md), "Tech de Ra's near-left wall"; the hulls' own vertex programs are to be read with the same reader before `hull_normal.wesl` changes, and Omega/2048's shaders are an open check. Slots are `hd_material_probe_dump` ordinals; the bisection is
   `OAG_SKIP_SLOT` halving on one pixel.
3. **Additive source strength** (mesh): Talon's grill source about 1.5x the original's, (~~Amphiseum's hologram
   stand-in about 70 grey over~~, wired 2026-10-27); then Chenghou's `glass_2nduv_reflect_glow` (reflection plus glow 4 grey over) and
   Modesto's `uvanim_diffuse_emissivealpha` LCD banners (program unread).
4. **Vineta K's water sheets** through `rcs::water` and the sky map (family 4 of the glass lane, not started).
5. **`hd-render-state-ports`**: Omega and 2048 state bits 7 and 2, real alpha-to-coverage under `--msaa`, Zone's
   pass bits; the Anulpha chevron curve's clock (a live read beside the HUD clock).
6. **`hd-hud-leftovers-2`** (HUD crates): the retro skins (one RPCS3 draw capture with HUD Style 2097), the empty
   Lap1/Lap2 rows' rule, the `0xCC` writer by a PPU write-watch, and a test that fails if `hud_overlay` drops
   `half_unit_glyph_quads`.
7. **Pure** (`docs/formats/pure-status.md`): the hull, flat in ours where the original's follows the scene; an
   unlock for circuits past the Alpha league so more pairs can be made.
8. **2048 and Omega** (`docs/tools/compare-page.md`): 2048's Empire Climb draws in flat colours and is far brighter
   than the original, Queens Mall's grid spot differs, Metro Park's Time Trial hangs Vita3K; Omega's gantry board
   shows a static 8 where the original counts down, its lighting differs, Anulpha's road reads white, and the idle
   exhaust flame is missing.
