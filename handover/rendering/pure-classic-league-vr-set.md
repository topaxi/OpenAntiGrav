---
status: ready-to-wire
titles: [pure]
updated: 2026-11-16
summary: "Pure's Classic hull is wired; the VrFx flag's other consumers (sound banks, vr_ weapons), Classic lighting and shadow shape are not"
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

- **The `<VrFx Type="1"/>` flag** (`TrackStartup.xml` of the four `Data\ClassicsNS` circuits; byte `0x08a8f604`, written by `FUN_08848bf8`). Its consumers read so far: `Data\Sound\ship_classic.bnk` and `weapons_classic.bnk` in place of `ship.bnk` and `weapons.bnk` (`FUN_08847a54`, branch on the byte), `WO_QUAKE_VR` and the dropped lift in the quake (`quake.md`), `vr_Mine.vex` (`weapons-gfx.md`), and the other `vr_` weapon models and `Data\Tex\vr_env.tga` by name, selector unread. Check each consumer's own read before wiring it; the hull flag is a different byte.
- **Classic worlds render darker than the original's** (`docs/tools/compare-page.md`, "The Classic hull"): whole-frame mean grey difference 17 to 39 on the four pairs against 9 to 27 on Beta. Lighting, not geometry; unmeasured per region.
- **The ground shadow under the Classic craft** is not the original's shape (a flat trapezoid in the original). `shadow::hulls` and `shadow::silhouettes` still read the plain `Ship.vex`; pointing them at the circuit's stem changed no pixel on the four pairs, so it is not the cause, and was left out.
- **Sagarmartha's hull correlation peaks at a 4 px shift** (0.28 at 0, 0.78 at +4): pose or the VR hull's origin, unmeasured.
- **The Classic craft draws no wreck model** (`Ship_LoadModel` zeroes the second instance on flag 2); Pure draws none today either, so nothing to do until Pure's wreck lands.

## Next Steps

1. Wire the Classic sound banks off the `<VrFx>` flag (read the flag once per circuit in `oag_raceplay::load`, a `Title` axis for the bank pair); verify with the load report, not by ear.
2. Re-make the four Classic compare pairs with the VR-hull caveat dropped from `psp-pure-pair.py --hull-differs` and check the hull offset gate (needs PPSSPP).
3. Measure the darkness per region on the four pairs (hull box, road, sky) before touching lighting.
