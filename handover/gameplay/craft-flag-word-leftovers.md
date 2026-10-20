# The craft flag word's leftovers

2026-10-20, `craft-flags-sideshift`. Every writer of Pulse PSP's `craft+0x1c0` is named and
every bit read, with a live write watch over a countdown, two sideshifts and a Turbo:
[craft-flags.md](../../docs/ghidra/functions/psp-pulse-usa/craft-flags.md). Ported: the
sideshift gestures withheld on the grid (`0x2`), a running Turbo's (`0x200`) escape from the
stun/slowdown return, its speed-pad fade (`craft+0x2cc`) and its d-pad Up boost lift
(`<Special turbo_jump>`), and the Eliminator wrong-way kill (`0x2000`, a `CraftLaws` axis).
Dead in the original and left out on purpose: `0x10`, `0x20`, `0x40`, `0x80`, `0x100`.

## Open

- **`0x2000` live.** The wrong-way kill is a static read (82). Watch `craft+0x1c0` in an
  Elimination race on PPSSPP, turn the craft round, and check the bit, the zero thrust and
  the full airbrakes. Needs an Elimination save state (none under `data/saves/pulse-psp/`).
- **HD's flag word** (`craft+0x264`) numbers its bits differently: `Craft_UpdateThrottle`
  (`0x000efb50`) resets `craft+0x320` under `0x20` (Pulse's `0x200`/`0x2cc` shape) and zeroes
  `craft+0x30c` under `0x200`. Map every writer as on Pulse, then turn
  `eliminator_wrong_way_kill` from `inherited` to measured for HD (and Omega/2048 through it).
- **The barrel roll and a respawn** were not exercised live: the `0x400` stores are the static
  set, and nothing past `Ship_InitCraft` is known to write the word on a respawn.
- **`craft+0x2a4 == 1`** gates the turbo add and the lift; ours has no racing-state enum.
- **`Entity_SetGridState`'s other callers** (`FUN_08826044` through two wrappers, and
  `FUN_0889683c` in modes 14 and above) hold a craft in the grid state for a reason not read.
- **The AI gates moved, inside the noise bar, at its edge.** Failures among the 49
  `*_original_ai_ground_truth` tests per airborne pitch-torque perturbation (-1e-2 .. 1e-5):
  off `0 0 1 1 0 0 0 2 0`, on `1 2 0 1 4 2 0 0 0`; 10 against 4. Bounds moved in one
  commit (Omega Talon's and Pure Vineta K: one destroyed opponent; 2048 Square: two respawns a
  craft; 2048 Park: one). Per title the rise is Omega and 2048 (pad fade) and Pure (stun
  escape). Worth a seed-based sweep once the tool from `ai-gates-flip-on-noise.md` exists.
- **`speedup_pad`'s tilt** reads the pitch axis (`steer_y <= -0.5`) for the same d-pad Up
  bit the lift now reads from `ShipControls::dpad_up`; so our AI tilts its pad boosts and
  the original's cannot. Another lane's judgement call, left as it was.
- **Pure** carries the sideshift gate, the Turbo escape, the pad clock (`craft+0x284`) and the
  lift in the same shape, read but not watched live.

## Next Steps

1. Take an Elimination grid state on PPSSPP (`psp-drive.py menu`, then `psp-state.py save`),
   run `scripts/psp-craft-flags.py` with a scripted U-turn, and record the `0x2000` frames.
2. On RPCS3, put a write watch on HD's `craft+0x264` through a race with a Turbo and a
   sideshift, and name its bits.
