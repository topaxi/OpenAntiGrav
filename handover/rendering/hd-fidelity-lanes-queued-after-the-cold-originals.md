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
   details: **not started** (Tech de Ra carries 9 `scanlinebillboard` slots, now bound, none in the pair's view).

**Amphiseum's `hologram.rcsmaterial` board** (2026-10-22, `hd-blend-space`): drawn through the generic emissive
stand-in and +70 grey too bright over the start gantry (box `840,0 240x120`); the encoded store lifts it further, which is
the whole of `hd/the-amphiseum`'s +0.42 (10.09 to 10.51). With the material skipped the box is within 9 grey of the
original. Next: read its fragment program.

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
  open: `glass_reflect` (11 circuits), `ds_booth_glass` (11), `glass_reflect_opacity` (Chenghou 14 slots, Vineta K, `03_track`),
  `glasstest`, the water sheets of Vineta K (`rcs::water`, not started), the mag floors, the pipe family, the ice: each a row once its
  block is read, one per commit with all 13 pairs re-judged. Metropia's `glass_2nduv_reflect_glow` resolves to block `@0x18350`,
  assumed Chenghou's law, unread. The glass normal maps are the shared omission. The tint is `ProbeTint::SCENERY` (`{1, 1, 1, 0}`, read on Moa's glass only); a second circuit's
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
- ~~**The chevron's emissive pulse**~~ **wired** (2026-10-22, `hd-texture-slots`, item 2 above); what is left is the
  clock its curve runs on.
- **`m_diffuse_normal_constantspecular`'s normal alpha**: the RGB is a normal map
  (`docs/formats/rcsmaterial.md`); what its alpha carries is unread.

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
   `--attempts 4 --delay 0.1` and take `drive.screenshot` before the dump.
2. **`hd-tech-de-ra`** (opus, mesh): Tech de Ra's missing details by a slot-skip census, the glass road's mid and far
   brightness (the eye dump of `c[209]`), Modesto's background building texture, and Vineta's right-hand building
   (sunlit bands about 30 grey over the original; name the material with `OAG_TINT_MATERIALS=1` and `hd_slot_list`).
3. **Additive source strength** (mesh): Talon's grill source about 1.5x the original's, and Amphiseum's hologram
   stand-in about 70 grey over; then Chenghou's `glass_2nduv_reflect_glow` (reflection plus glow 4 grey over) and
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
