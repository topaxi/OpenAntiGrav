---
status: ready-to-wire
titles: [pure]
updated: 2026-11-25
summary: "Classic hull, sound banks and the disc's song text are wired; the hull box grey was ours' invented song bar; vr_ weapons unwired"
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

- **The `<VrFx Type="1"/>` flag** (`TrackStartup.xml` of the four `Data\ClassicsNS` circuits and the four Zone circuits; byte `0x08a8f604`, written by `FUN_08848bf8`). **Wired: the sound banks** (`ship_classic.bnk` and `weapons_classic.bnk` in place of `ship.bnk` and `weapons.bnk` on a non-Zone race; test `pure_race_ground_truth::a_circuit_that_authors_vrfx_loads_the_classic_sound_banks_and_no_other_does`; `ship-models.md`, "What selects the VR hull"). Not wired: `WO_QUAKE_VR` and the dropped lift in the quake (`quake.md`), `vr_Mine.vex` and the other `vr_` weapon models (`vr_Bomb`, `vr_shield`, `vr_shield_cockpit`; `weapons-gfx.md`), `vr_engine_noise.mip` (`0x08a4a094`) and `vr_fxgrid1.mip` (`0x08a7ae04`); each has its own read to check before wiring, the hull flag is a different byte. `Data\Tex\vr_env.tga` is embedded in each team's `VR\Ship.vex` (no selector, no `Data.wad` entry, no load-report miss); `FUN_08848bf8` walks `LevelFx` only while the byte `*(_DAT_0028a960 + 0x5ded)` is zero, unread.
- **The Classic pairs' remaining grey** (`docs/tools/compare-page.md`, "The Classic hull"): no whole-frame darkness is left; the scenery phase is closed on all twelve Pure race pairs (the four Classic read 4.20, 6.73, 6.24, 9.06 at main `41d94285f`). **The hull box's "original brighter" is mostly ours' now-playing bar, not the hull**: its rows 247 to 272 read +8 to +15 grey signed and hold the bar's dark backing; its hull pixels read +7.2, +4.6, -2.5, -1.6 (one sign does not hold, so no fixed VR-hull material offset). What is left: the HUD, the ground shadow (ours 28 to 48 grey darker over its 400 pixels), Sagarmartha's hull at -4 px, and a circuit-dependent hull light term nobody has named.
- **Pure's music text** (wired; song pinning and timing open) (read 2026-11-25, `docs/formats/now-playing.md`, "Pure's own music text"): the disc's `InGame` screen authors `InGameArtistText`/`InGameTrackText`/`InGameLabelText` (`HUDSmall`, left, `x 80`, `y 240/250/260`) and the original draws two plain lines (artist over track, label never shown), fading in over 1.8 s from GO, holding about 4.9 s and fading out over 1.8 s (Pulse's 5-of-255 step at 30 a second; measured on Karbonis and Sebenco Climb, confidence 60 for the timing, seen once). **Wired** (`Show::Lines`, hold 150 updates of 30 a second, the fallback marquee file retired): the Classic hull box's mean absolute grey difference falls 11.24 -> 5.39, 8.44 -> 6.04, 8.21 -> 5.52, 25.40 -> 24.78; three pairs whose original plays another song than ours rose by 0.06 to 0.20 (the words differ), so the open item is pinning the original's song in a pair (`--hud-song` needs `--hud-state`) and a pair script that records it. Open before wiring: whether the hold times from GO or from the song's start, and what the next song does (the frames at 16.7 and 17.8 s show no text, but they are inside the first song; measure across a song change).
- **The ground shadow under the Classic craft** is not the original's shape (a flat trapezoid in the original). `shadow::hulls` and `shadow::silhouettes` still read the plain `Ship.vex`; pointing them at the circuit's stem changed no pixel on the four pairs, so it is not the cause, and was left out.
- **Sagarmartha's hull sits 4 px left of ours** (0.481 at (-4, 0), 0.093 at (0, 0); pose and camera identical, the other three Classic pairs at (0, 0)): pose or the VR hull's origin on that circuit, unmeasured.
- **The Classic craft draws no wreck model** (`Ship_LoadModel` zeroes the second instance on flag 2); Pure draws none today either, so nothing to do until Pure's wreck lands.

## Next Steps

1. Pin the original's song in the Pure pair script (read by eye or from RAM), so the wired text's words match; measure the display across a song change and whether the 4.9 s hold times from GO or from the song's start.
2. The hull's own residual (+7.2, +4.6, -2.5, -1.6 by circuit): record one GE list of the original's Karbonis hull and compare its light words with ours' `HullLights`, as `hull-shine.md` did for Modesto; `hull_shine_lit` is unread on Pure.
3. Wire the remaining `<VrFx>` consumers one at a time, each against its own read.
