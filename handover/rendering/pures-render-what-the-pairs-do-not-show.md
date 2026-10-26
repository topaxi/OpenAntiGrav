---
status: ready-to-wire
titles: [pure, pulse]
updated: 2026-10-26
summary: "Pure render: canopy, sky, effect names, flare and circuit chrome-map pass closed; 250 light scale, Modesto light, HUD open"
---

# Pure's render: what the pairs still show, ranked

Lanes `pure-render-fidelity` and `pure-render-fidelity-2`, 2026-10-26. Closed so far:

- the hull takes its circuit's lights ([`psp-pure-usa/scene-light.md`](../../docs/ghidra/functions/psp-pure-usa/scene-light.md));
- the hull draws its `0x2000` extra pass, weighted by its texture's alpha;
- Chenghou's colourless sky cube draws unlit at white;
- the fifteen engine effect names Pure never requests are dropped.

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
2. **Hull, the rest.** Measured in the GE lists: the grid's light scale is `250` on all four pairs (`249/255`
   on every light byte). That is Pulse's live grid value, the double truncation of a blended `255` that ours
   reads as the nearest point. Wiring the blend in `oag_race::Course::light_scale_at` moves Pulse by the same 2 %,
   so it wants its own lane with Pulse pairs re-rendered. Modesto's list also carries a second directional
   `(208, 47, 1)` along `(-19.24, 7.19, 1.42)`, unnormalised; ours draws no second light there. Which wopoint
   it is, and why it is a directional, is unread (Pure's builder `0x08841648`).
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

## Next Steps

1. ~~Pulse circuit weighting~~ closed 2026-10-26: Pulse PSP's circuit ordinary pass is `0x2a` like Pure's, wired
   ([`psp-pulse-usa/circuit-shine.md`](../../docs/ghidra/functions/psp-pulse-usa/circuit-shine.md)).
2. To measure: record a GE list with `scripts/psp-ge-dump.py dump` on the pair's own save state
   (`data/saves/pure-psp/*-restart.ppst`, loaded through `scripts/psp-state.py load`). Replay an edited copy by
   opening the `.ppdmp` in PPSSPP itself; the edits that answered this lane are a register rewritten at a PRIM
   (`FIXA` 0, an opaque CLUT, vertex colours). Re-score with the region recipe: grey MAD over
   `hull (190..290, 175..272)`, `sky (0..480, 55..140)`, `track (0..480, 140..240)`.
