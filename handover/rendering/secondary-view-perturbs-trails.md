---
status: measure
titles: [hd, pulse]
updated: 2026-11-11
summary: "Drawing a portal's secondary view changes how an opponent's engine trail draws in the main picture"
size: S (40)
difficulty: medium (40)
---

# A secondary view changes an opponent's trail in the main view

`oag.window` (`assets/mods/examples/window`) asks for `portal-view`: the engine draws the scene from an exit
camera into a texture the window's shader reads. The mod "draws nothing that reaches the race", and the main
picture outside the window should be the same texel for texel as a run with no mod
(`crates/game/tests/mod_shader_frame_ground_truth.rs`, `check_portal`).

## Open

- **An opponent's engine trail is not.** HD/Fury, Single Race, `--autopilot`, tick 1022, 960x544, original
  driver with the pair-contact push: an opponent rides on the player's right, and with the window on its trail
  differs in a patch at about x 800-960, y 80-270 (outside the window's grown mask), 5.6 % of the texels outside
  the surface. The two runs share one simulation (same shield, same craft poses), so it is the drawing: a trail
  (or the stepped-visual state it reads) advanced or rebuilt by the second view's pass. Before that change the
  same scenario already read 0.7 % at tick 1010 (main `de8138d37`), so it predates it.
- The HD portal test now runs as a Time Trial (no opponents) so its bound holds the view alone; the Pulse portal
  tests still race a field and pass at their tick.

## Next Steps

1. Reproduce: in `check_portal`, race the HD test in `single_race` at tick 1022 and dump `with`/`without` and
   the outside-only difference (a few lines writing three PPMs next to the `gap` call). Then check whether
   trail emission or its between-ticks interpolation is fed per render pass rather than per tick
   (`oag_fx`'s trail code, the secondary view's draw path in `oag_raceplay::scene::portal_view`).
2. Once fixed, put the HD portal test back on `single_race` if it stays at `0.0005`.
