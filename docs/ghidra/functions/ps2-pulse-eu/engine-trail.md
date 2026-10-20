# The PS2's engine trail: the centred spike at rest

**Binary:** `SCES_547.48` (Pulse PS2, EU). Read 2026-10-20 statically and on PCSX2 (Moa Therma White, player slot 8, GS dumps at rest and under thrust).

## Answer

The centred four-fin spike the original draws at rest is **the `Trail` ribbon**, not `shipboost.vex` (confidence 88). `shipboost.vex` is the ship object's `+0x214` model; `FUN_001d4728` (per-frame) sets bit `0x4` of its flags only when the boost timer `+0x19c > 0.2`, clears it again after 1.5 s, and the constructor `FUN_001d4310` clears it: hidden at rest, exactly as ours.

The ribbon is the object at ship `+0x144`: built by `FUN_001d4310` (`FUN_001efbc0(.., 2)`), vtable `0x0029afb8`, preset filled by `FUN_001f0090(sub, 2)` (10 samples, 3 layers, half-widths 1.0 / 0.7 / 0.5, colours blue `(0,8,128)`, magenta `(155,0,124)`, white `(210,210,210)`: the PSP's preset 2, `psp-pulse-usa/exhaust.md`). It queues at sort `0x7d100000` (`FUN_001efeb8`) and draws in `FUN_001f0a08`.

## Why it shows at rest on PS2 and not on the PSP (confidence 85)

- The PSP's `Trail_DrawRibbon` multiplies the baked vertex colours by the per-frame grey ramp through `sceGuAmbient`; at rest the ramp is 0, so the ribbon adds nothing.
- `FUN_001f0a08` never reads the ramp (`obj+0x1e8/0x218/0x248`, written by `FUN_001d4728`, read live as 0.33/0.21/0 at thrust). Evidence: under thrust the head vertex is still `210` with the ramp at 0.35; a GS dump at rest holds grey `210,147,99,63,38,20` = `210 x (1 - 0.1k)^(1/0.3)` (our `trail_fade`) and magenta `155,0,124` fading the same way.
- Its submit loop only submits layers with `index != 0` (the `iVar15 != 0` test before the per-segment `FUN_0020df40` loop). Layer 0 (blue, widest) is the PSP's stencil-stamped layer; the PS2 has no blue group in the dump. Grey (layer 2) is drawn first, magenta (layer 1) second, as dumped. The PSP loop (`Trail_DrawRibbon`, `0x0892acc8`) draws layer 0 too (`iVar21 == 0` takes the stencil path), so this is PS2 only.
- Colours are on the GS scale: `TEX0.TFX = 0` (modulate), `128 = 1.0`, so `210` is 1.64 x texel (blend `ALPHA 0x7f00000068` = additive). `FRAME FBMSK = 0xff000000`: the ribbon stamps no glow mask.
- Width is `intensity * 0.35 + 0.2` at the object's `+0x164` (read 0.2 at rest), same as the PSP.

## Wiring

`Looks::ps2_trail` (Pulse, PS2 only, Measured) -> `Race::trail_law()` -> `oag_fx::exhaust::TrailLaw::Baked`: layers from `PS2_FIRST_LAYER`, colours x `255/128`, no ramp, no glow stamp. PSP and every other title keep `Ramped`. Tests: `the_ps2_law_draws_the_baked_ribbon_at_rest_without_the_widest_layer`, `title_effects` rule test. Sheets: `data/reference/ps2-plume/` (not committed).

## The engine flare is one quad, and `WO_SHIP_ENGINEFLARE` is never played (2026-10-20, `ps2-flare-halo`, confidence 85)

The PS2 draws **one textured quad per craft** for the flare, not the particle effect the disc ships as `Data\Psys\WO_SHIP_ENGINEFLARE.POB`.

- **Draw census.** Every GS draw group with a vertex in the player's nozzle box, in the rest dump (both fields) and in dumps at 0.2 s and 4 s of thrust: the hull, the ribbon's 64x64 groups (grey `210,147,...`, magenta `155,0,124`), a 32x32 engine-plate group, and exactly one `128x64` group (`ALPHA 0x48`, `TEST 0x7000d`, `TEX0.TFX 0`, four vertices `rgba 255`). Nothing else with a violet or multi-particle shape. The `.POB` has two looping emitters (`interval 4`, `life 20` ticks and a cone of `Flares`), so it would add several billboards per field.
- **The quad's size follows the PSP law.** At rest it is 81 x 115 px, 110 x 157 px and 103 x 147 px of the 512 buffer in different fields; after 4 s of thrust 151 x 217 px. The PSP's half-size is `(intensity * 0.6 + 0.4) * 2.5` with a 0.75-1.25 flicker, a 2.5x span rest to thrust. Ours at the same pose is 79 px at rest against a hull 138 px wide (0.77 of the span); the original's is 0.63-0.86 of its own hull width (128 px). All four vertices are `rgba 255` in every dump: the PSP's alpha flicker (200-255) does not show on the PS2.
- **The executable never loads the effect.** `SCES_547.48` lists every effect it plays as a `Data\Psys\<name>.POB` path (`0x1a90f8`-`0x1a93e0`, `0x1bebb8`-`0x1bec40`) and by name; `WO_SHIP_ENGINEFLARE` is in neither, and neither is its hash `0x0afe819a` in either byte order. The earlier wiring ("the trigger needs no recovery, an engine flare is on while the craft is") was an assumption, and it drew violet rays several times the plate's width that the original does not show.

**Wired:** `oag_pulse::EFFECTS` no longer carries `Trigger::EngineFlare`, so a PS2 race draws the procedural flare quad as the PSP does (`Race::engine_flare_effect` is false). HD dropped the same trigger earlier. The `.POB` stays on the disc, unplayed.

**Not matched yet.** The original's quad shows a violet halo round the plate that ours, now a quad too, draws far fainter. Tried and ruled out as the cause on a matched-pose sheet: a larger quad would overstate the extent (sizes agree), and four copies of the quad at the GS's `255/128` colour gain changed nothing visible. The GS law on this quad is `Cd + min(Cs * 255/128, 1) * min(As * 255/128, 2)` with `COLCLAMP 1`, up to about four times a texel; ours is `Cd + Cs * As` with `As <= 1`. A depth offset of the quad toward the camera (the quad's `z` 29862 sits above the plate group's 27418-27932 in the dump) brings glow out below the plate but not round it. Open: where the PS2 puts the quad in depth, and whether its alpha test (`ATST NOTEQUAL 0`) hides the ring.

## Open

- The third term of the draw gate, `DAT_0027e7f0 != 0` (with ring full and count > 1): writers not read. It holds at rest in the state.
- The ribbon's colours exceed 1.0 before the texel multiply (PS2 clamps per fragment before the add); ours does not clamp, so the core may read slightly hot. Judged on the pair sheet only.
- `FUN_001f0780` (the colour bake) not read; unnamed.
- Our ribbon is more speckled than the original's at the nozzle (not compared; the ribbon's colour clamp above is its likely cause).
- The violet halo round the plate is the flare quad, not the `.POB`: see "The engine flare is one quad". Other titles: Pure ships no PS2 pressing here and keeps the engine table's trigger, which finds no asset in its archive; 2048 and Omega name no such effect. Checked, not applicable.
- Other titles: Pure has no PS2 pressing here; 2048/Omega not applicable (no `Trail`): checked, not applicable.

## Names

| Address | Name | Conf |
| --- | --- | ---: |
| `0x001f0a08` | `Trail_DrawRibbon` | 80 |
| `0x001f0090` | `Trail_InitPreset` | 80 |
