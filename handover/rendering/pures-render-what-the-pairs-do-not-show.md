---
status: ready-to-wire
titles: [pure, pulse]
updated: 2026-11-27
summary: "Pure render: Sinucit adverts and strip, HUD lap and digits and scenery detail open; most else closed"
difficulty: medium (30)
---

# Pure's render: what the pairs still show, ranked

Lanes `pure-render-fidelity` and `pure-render-fidelity-2`, 2026-10-26. Closed so far:

- the hull takes its circuit's lights ([`psp-pure-usa/scene-light.md`](../../docs/ghidra/functions/psp-pure-usa/scene-light.md));
- the hull draws its `0x2000` extra pass, weighted by its texture's alpha;
- Chenghou's colourless sky cube draws unlit at white;
- the fifteen engine effect names Pure never requests are dropped;
- **the bloom** (`pure-sign-glow`): Pure runs Pulse's four-pass bloom over Pulse's stencil mask, composited before its HUD.
  Modesto's neon signs glow: sign box mean luma 75.5 to 111.3 against the original's 111.1, frame MAD 8.96 to 6.51
  ([`psp-pure-usa/bloom.md`](../../docs/ghidra/functions/psp-pure-usa/bloom.md), `Looks::psp_glow_mask`, `Looks::bloom_under_hud`).

- **the weather** (`pure-race-worst-two`): Sinucit and Sebenco Climb author a `<Weather>` (rain and lens, snow, and the
  `Mist.mip` screen overlay) that Pure never played. `Looks::circuit_weather` now plays it with Pulse's law: both pairs'
  whole frame read 23 grey dark, now +8.5 and +3 (mean grey 26.9 -> 15.4 and 25.7 -> 15.5; the other twelve pairs and a
  Pulse, HD, 2048 and Omega pair unchanged to the pixel; `docs/rendering/pure-weather.md`).

The last three: [`psp-pure-usa/hull-shine.md`](../../docs/ghidra/functions/psp-pure-usa/hull-shine.md).

Region MAD (grey, 480x272) on the four compare pairs, before this lane and after it:

| Pair | Hull | Sky band | Track band | Frame |
| --- | --- | --- | --- | --- |
| Vineta K | 33.79 -> 20.57 | 30.19 -> 30.19 | 25.60 -> 23.60 | 25.89 -> 24.86 |
| Chenghou | 27.17 -> 14.51 | 16.12 -> 14.21 | 12.86 -> 10.92 | 21.61 -> 13.12 |
| Blue Ridge | 20.49 -> 12.65 | 7.01 -> 7.01 | 8.25 -> 7.09 | 9.16 -> 8.57 |
| Modesto | 15.73 -> 5.63 | 10.79 -> 10.79 | 7.55 -> 6.02 | 9.96 -> 9.19 |

## Open

Ranked by what a player notices: hull and plume, then effects, then track and lighting, then HUD.

1. ~~**Hull canopy.**~~ Closed: the extra pass was never off in the data, the roster's `applies_everywhere()`
   gate was. The weight is `header_flags & 0x02` (`BLENDMODE 0x2a` in the recorded lists).
2. **Hull, the rest.** ~~The grid's light scale `250`~~ closed (`pure-light-scale`). ~~Modesto's second light~~ closed: `wopointShape2`, drawn
   (`hull-shine.md`, "Modesto's second light"). ~~The residual hull-box error~~ closed by `pure-modesto-hull`: **texture level**. Pure's
   `.vex` textures dropped their authored mip levels, so no Pure texture took the GE's slope rule (`TEXLODSLOPE` `1/256`, mode 2, on 430
   of 430 PRIMs); the hull, at view depth 6 to 17, reads level 0 in the original and a coarser sampler-chosen level in ours. Reading the
   levels moved the Modesto hull box 5.09 -> 2.74 (the canopy 16.9 -> 5.2, the whole high-pass gap closed) and ten of twelve race pairs'
   hulls (`hull-shine.md`, "The hull's texture level"). Open from it: the original's per-texture bias (hull 3.3 to 4.1; ours 1.0 on
   every texture, same as Pulse), the extra pass's `TEXMODE` level 0 only, and what the box still holds (plume glow, the ground
   shadow, canopy edge). The craft's own spline parameter only feeds the light scale; it is not the residual.
3. **Effects Pure ships that ours never loads.** Pure's executable names 24 `WO_*` effects (25 strings,
   one of them `WO_Game`; `search_strings` over `/pure/BOOT-psp-pure-usa.BIN`, `0x08a47f50`..`0x08a7a49c`).
   Not loaded:
   - `WO_DISRUPTOR_LAUNCH`, `_HEAD`, `_EXPLO` (no Disruptor weapon in ours);
   - `WO_BOMB_GLOW`, `WO_TRACK_ROCK_DEBRIS`, `WO_SHIP_COLL_SPARK_NODAMAGE`;
   - the ship-death set `WO_SHIP_FXNODE_EXPLO`, `WO_SHIP_DEATH_SPARKS`, `WO_SHIP_FXNODE_BIGEXPLO`,
     `WO_SHIP_EXPLOSION_DEBRIS`, `WO_SHIP_EXPLO_SMOKE`. The first two are Pulse's wreck pair in
     `oag_raceplay::wreck_fx`; check whether it fires on Pure.

   `WO_QUAKE_VR` is the VR build, not planned. Each needs its trigger read on Pure before it is wired. The
   weapon side continues in `pures-weapons-mine-bomb-wired-plasma-disruptor-open.md`.
4. ~~**Pulse-only names in Pure's effect table.**~~ Closed: `.without`ed, with the string list as evidence.
5. **Track and lighting.** On Vineta K and Chenghou the whole frame read 13 to 28 grey darker in ours (sky band
   bias -28 and -14, track band -24 and -10), Blue Ridge and Modesto near level. **Mostly the undrawn sun lens
   flare, now wired** (`pure-lens-flare-wire`, `docs/rendering/lens-flare.md`). Those two circuits author a
   `lensflare` node; Blue Ridge and Modesto have none in view. Masked mean diff against the original, flare off
   -> on: Vineta K -29.6/-26.2/-29.8 -> -3.3/-4.0/-6.7; Chenghou -25.9/-18.5/-14.5 -> -9.9/-10.0/-8.1; Modesto and
   Blue Ridge unchanged (-6.7/-2.2/-2.3 and -0.2/-0.8/-1.3). The original also saturates near the sun, where ours
   reads about 237.

   The recorded GE lists agree independently (`pure-render-fidelity-2`, `psp-pure-usa/hull-shine.md`, "The scene
   darkness"). The flare is the one camera-space additive PRIM those two circuits draw before the post chain:
   60 vertices (ten quads), `BLENDMODE 0xa2`, depth test off. With it removed, the replay matches ours (Vineta sky
   153.9 -> 128.8 against ours' 126.6). Vertex colours, CLUTs, fog ranges and the post composite were ruled out.

   Chenghou's -10 residual is mostly its sky cube, closed in `pure-render-fidelity-2`. The cube has no vertex
   colours or normals, and the GE draws it unlit at white; ours ran it through the stand-in rig (top band MAD
   49.8 -> 15.5). Re-score the pairs with both lanes merged. Left, from the same four GE lists:
   - ~~**The circuits' chrome-map pass.**~~ Closed for Pure by `pure-chrome-map` (`hull-shine.md`, "The circuits"; Vineta
     windows 20.7 -> 6.5 grey). Pulse's circuit weighting closed 2026-10-26 (`pulse-circuit-weighting`, weighted, wired). Original text: Pure draws 57, 36, 31 and 28 `TEXMAPMODE` 2 PRIMs off the craft
     (Vineta's blue windows are black in ours). Ours draws the circuit pass on Pulse's PSP only
     (`oag_raceplay::load`, `pulse_psp && track_shine`). The circuit batches carry the alpha-weight bit as well
     (Pulse PSP 3,446 weighted batches), and `oag_render::shine::build_track` ignores it. Wiring it on Pure is
     the faithful next step. Weighting Pulse's moves Pulse pairs: check against a Pulse GE list first.
   - **Material colour of an uncoloured draw.** The GE colours a colourless, unlit vertex with
     `MATERIALAMBIENT`: white on the sky and road, `(107, 216, 255)` and `(128, 128, 128)` on Vineta's layered
     water plane at `y = -50`. Its source is unread. `mesh::build` still gives such vertices a placeholder grey
     `(0.75, 0.78, 0.82)` and the stand-in rig outside the sky.
   - **Vineta's second fog.** 30 PRIMs carry fog end `1270`, start `500`, colour `(255, 252, 237)` beside the main
     `170..1400` one. Ours reports one volume. Which PRIMs those are was not read.
6. **HUD.** Ours draws no speed gauge (the original has one) and prints no pilot message ("JAY TRIPWIRE
   ROOM 2", "FRIENDLY WE GOT JUICE" in the originals); both are visible on the pairs.
7. **Plume and shield.** Pure has no boost-plume model (`ship-models.md`) and no per-team
   `shipshield.vex`; ours draws the shared `Data\Weapons\shield.vex`. Not compared on a frame.
8. **`TEXFUNC` 0 and the alpha weight.** The base pass reads `TEXFUNC 0` (`TCC` RGB), and the texture's alpha
   still reaches the `SRC_ALPHA` blend, measured by the CLUT edit. Recorded, not explained.
9. **Zone (2026-11-17, `zone-pair-pure`: Pro Tozo zones 8 and 9, `docs/tools/compare-page.md`, "Zone pairs: Wipeout Pure").** Ours' Zone
   camera is the Close rest rig (11.64 units from the body); the original's eye was 7.48 units from the body (2.2 up, 7.1 back) in both
   states (158 and 208 units/s), with the fov at 81.5 and 85.6 degrees where ours holds 60: a speed spring or a fixed Zone rig is open
   (no at-rest Zone read; confidence 55, one boot). Ours is brighter (mean grey 220/194 against 176/143: the original's wall bands are
   black with a pink-violet bloom), draws a shadow under the hull the original does not, and no blue engine glow. Whether the original
   changes palette between zones at one fixed place was not tested; ours' `--zone-stage` is a no-op on Pure. The Zone craft's hull in the
   original is not identified (ours flies Assegai, Pure's default; `Data\Ships\Zone_01` exists). Unpaired: `02_Zone`, `03_Zone`, `04_Zone`.
   The Zone counters' RAM addresses (zone, score, zone seconds) are not located. A second cold boot of the same Zone race has not been made.

10. **Sinucit's and Sebenco's remaining regions** (`docs/rendering/pure-weather.md`, "What is left"): Sinucit's advert
    screens (the original plays a cyan and yellow picture over three panels; ours three cyan strips, billboard slots drawn
    nothing), the blue strip across the track at the craft's height (wider and bloomier than the original's thin line, 8 to
    11 % of each pair's `>40` pixels), the mist's random UV pattern (the original rolls it each re-roll; a pair's RAM holds
    the offsets, ours a fixed table) and Pure's `FxScreenMist` draw function, unread (relocation trap, confidence 70).

11. **The circuit pairs' residual, split** (2026-11-27, `docs/tools/compare-page.md`, "The Pure circuit pairs' residual, split by region and cause"). In order of what to wire:
    (a) ~~The presented lag, 1 to 2 frames~~ wired 2026-11-27 (`docs/tools/compare-page.md`, "The presented lag, wired"); the Zone pairs inherit it unread. (b) The lap counter's `/3` sits 17 px right of the original's on all twelve; read Pure's `Hud` lap widgets.
    (c) The billboard slots (`adverts: None`): signs missing on Citta Nuova, Vineta, Sol 2, Blue Ridge, Sinucit; needs Pure's advert pass measured. (d) The largest share: background detail above the
    horizon (24 to 35 % of each score) has no identified cause; a GE list reading the scenery's `TEXLEVEL` bias is the next measurement.

## Next Steps

1. ~~Pulse circuit weighting~~ closed 2026-10-26: Pulse PSP's circuit ordinary pass is `0x2a` like Pure's, wired
   ([`psp-pulse-usa/circuit-shine.md`](../../docs/ghidra/functions/psp-pulse-usa/circuit-shine.md)).
2. To measure: record a GE list with `scripts/psp-ge-dump.py dump` on the pair's own save state
   (`data/saves/pure-psp/*-restart.ppst`, loaded through `scripts/psp-state.py load`). Replay an edited copy by
   opening the `.ppdmp` in PPSSPP itself; the edits that answered this lane are a register rewritten at a PRIM
   (`FIXA` 0, an opaque CLUT, vertex colours). Re-score with the region recipe: grey MAD over
   `hull (190..290, 175..272)`, `sky (0..480, 55..140)`, `track (0..480, 140..240)`.
