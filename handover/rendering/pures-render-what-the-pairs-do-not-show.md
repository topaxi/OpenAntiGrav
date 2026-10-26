# Pure's render: the hull now takes its circuit's lights; what is left, ranked

Lane `pure-render-fidelity`, 2026-10-26. The hull-light gap that opened this thread is closed: Pure's
circuits author `AmbientLight`/`DirectionalLight`/`wopoint`/`wospot` under their own class ids and the
reader now finds them ([`psp-pure-usa/scene-light.md`](../../docs/ghidra/functions/psp-pure-usa/scene-light.md),
[`pure-status.md`](../../docs/formats/pure-status.md#pures-hull-takes-the-circuits-own-lights-2026-10-26)).
Hull MAD on the four compare pairs fell 50.3/39.7/36.8/16.3 to 33.8/27.2/20.5/15.7 (Vineta K, Chenghou,
Blue Ridge, Modesto). The 2026-10-26 normals change was bisected as the only commit that moved the Pure
pairs (hull only, the airbrake flaps, +0.17 to +0.27 MAD under the stand-in rig) and is faithful for Pure.

## Open

Ranked by what a player notices: hull and plume, then effects, then track and lighting, then HUD.

1. **Hull canopy.** Near white in the original on all four pairs, dark teal in ours. Not the hull's extra
   pass: Pure's hull has no `0x2000` batch, and turning `Looks::hull_shine` on changed no pixel (tried,
   reverted). Next: read the canopy batch's flags and texture in Feisar's `Ship.vex`, and check whether
   Pure's batch draw picks `TEXMAPMODE` 2 by another flag (the radial environment map `pure-status.md`
   mentions under "the swizzle bit").
2. **Hull, the rest.** Still 10 to 20 % dark on Vineta K and Chenghou. Pure's twin of
   `Craft_ApplyHullLightScale` (Pulse `0x0883e444`) is unread; ours assumes Pulse's copy. Whether the
   pairs' grid points carry a scale below 255 was not checked.
3. **Effects Pure ships that ours never loads.** Pure's executable names 25 `WO_*` effects
   (`search_strings` over `/pure/BOOT-psp-pure-usa.BIN`, `0x08a47f50`..`0x08a7a49c`). Ours loads 12 at race
   start. Not loaded: `WO_DISRUPTOR_LAUNCH`, `_HEAD`, `_EXPLO` (no Disruptor weapon in ours),
   `WO_BOMB_GLOW`, `WO_TRACK_ROCK_DEBRIS`, `WO_SHIP_COLL_SPARK_NODAMAGE`, and the ship-death set
   `WO_SHIP_FXNODE_EXPLO`, `WO_SHIP_DEATH_SPARKS`, `WO_SHIP_FXNODE_BIGEXPLO`, `WO_SHIP_EXPLOSION_DEBRIS`,
   `WO_SHIP_EXPLO_SMOKE` (the first two are Pulse's wreck pair in `oag_raceplay::wreck_fx`; check whether it
   fires on Pure). `WO_QUAKE_VR` is the VR build, not planned. Each needs its trigger read on Pure before
   it is wired. The weapon side continues in `pures-weapons-mine-bomb-wired-plasma-disruptor-open.md`.
4. **Pulse-only names in Pure's effect table.** Every Pure load reports `WO_SHURIKEN_*`, `WO_REPULSER*`,
   `WO_CANNON_SPARKS`, `WO_PLASMA_LIGHTNING_*`, `WO_ROCKET_EXPLO_TRACK`, `WO_SHIP_ENGINEFLARE`,
   `WO_TRAIL_HITSHIP*` and `WO_MAGSTRIP_*` as absent: inherited from Pulse, none in Pure's executable
   strings, so they could be `.without`ed in `oag_pure::effects::EFFECTS` as the LeachBeam ones were.
5. **Track and lighting.** On Vineta K and Chenghou the whole frame reads 13 to 28 grey darker in ours
   (sky band bias -28 and -14, track band -24 and -10), Blue Ridge and Modesto near level. A scene-wide
   term, not the hull: compare the sky and track textures' decode and the fog against the original.
6. **HUD.** Ours draws no speed gauge (the original has one) and prints no pilot message ("JAY TRIPWIRE
   ROOM 2", "FRIENDLY WE GOT JUICE" in the originals); both are visible on the pairs.
7. **Plume and shield.** Pure has no boost-plume model (`ship-models.md`) and no per-team
   `shipshield.vex`; ours draws the shared `Data\Weapons\shield.vex`. Not compared on a frame.

## Next Steps

1. Item 1: dump Feisar's canopy batch (`oag-view --nodes` on `Data\Ships\Feisar\Ship.vex`, then the batch
   flags) and read Pure's batch draw for its `TEXMAPMODE` choice.
2. Item 4 is a short cleanup in `crates/pure/src/effects.rs` with the executable's string list as evidence.
3. Re-score with the region script recipe: render each pair's recorded `ours.flags` and take grey MAD over
   `hull (190..290, 175..272)`, `sky (0..480, 55..140)`, `track (0..480, 140..240)`.
