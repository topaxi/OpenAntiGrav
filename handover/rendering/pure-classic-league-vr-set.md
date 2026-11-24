---
status: ready-to-wire
titles: [pure]
updated: 2026-11-24
summary: "Pure's Classic hull and sound banks are wired off their flags; vr_ weapons, the hull box's brightness and 8 pairs' world clock are not"
categories: [rendering, audio]
size: M (50)
difficulty: medium (40)
---

# Pure's Classic league: the VR set past the hull

The hull is done: a `PI_Track` `type="Classic"` circuit flies `VR\Ship.vex` on every slot
(`oag_title::RaceDefaults::hull_by_track_type`, `docs/ghidra/functions/psp-pure-usa/ship-models.md`,
"What selects the VR hull"; test `pure_race_ground_truth::a_classic_circuit_flies_the_vr_hull_on_every_slot_and_an_alpha_circuit_does_not`).
What is left is read but not wired.

## Open

- **The `<VrFx Type="1"/>` flag** (`TrackStartup.xml` of the four `Data\ClassicsNS` circuits and the four Zone circuits; byte `0x08a8f604`, written by `FUN_08848bf8`). **Wired: the sound banks** (`ship_classic.bnk` and `weapons_classic.bnk` in place of `ship.bnk` and `weapons.bnk` on a non-Zone race; test `pure_race_ground_truth::a_circuit_that_authors_vrfx_loads_the_classic_sound_banks_and_no_other_does`; `ship-models.md`, "What selects the VR hull"). Not wired: `WO_QUAKE_VR` and the dropped lift in the quake (`quake.md`), `vr_Mine.vex` and the other `vr_` weapon models (`vr_Bomb`, `vr_shield`, `vr_shield_cockpit`; `weapons-gfx.md`), `vr_engine_noise.mip` (`0x08a4a094`) and `vr_fxgrid1.mip` (`0x08a7ae04`); each has its own read to check before wiring, the hull flag is a different byte. `Data\Tex\vr_env.tga` has no string in `BOOT.BIN`: the model data names it.
- **The Classic pairs' remaining grey** (`docs/tools/compare-page.md`, "The Classic hull"): there is no whole-frame darkness left (signed -1.2 to +1.9). The background was the scenery's animation phase; re-captured with the world clock the four pairs read 4.31, 6.86, 6.36, 9.04. What is left: the hull box (the original 2 to 6 grey brighter on all four), the HUD, Sagarmartha's hull at -4 px.
- **The other eight Pure race pairs have no world clock** (`original.world_clock_offset_s` null): re-capture each state with `psp-pure-pair.py` as the four Classic pairs were.
- **The ground shadow under the Classic craft** is not the original's shape (a flat trapezoid in the original). `shadow::hulls` and `shadow::silhouettes` still read the plain `Ship.vex`; pointing them at the circuit's stem changed no pixel on the four pairs, so it is not the cause, and was left out.
- **Sagarmartha's hull sits 4 px left of ours** (0.481 at (-4, 0), 0.093 at (0, 0); pose and camera identical, the other three Classic pairs at (0, 0)): pose or the VR hull's origin on that circuit, unmeasured.
- **The Classic craft draws no wreck model** (`Ship_LoadModel` zeroes the second instance on flag 2); Pure draws none today either, so nothing to do until Pure's wreck lands.

## Next Steps

1. Re-capture the other eight Pure race pairs with the world clock read; the lead adopts the four Classic pairs already made.
2. Trace the hull box's brightness (the original 2 to 6 grey brighter on all four Classic pairs): the VR hull's own textures and env map.
3. Wire the remaining `<VrFx>` consumers one at a time, each against its own read.
