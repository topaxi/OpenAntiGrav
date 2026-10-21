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
- ~~**HD's flag word**~~ mapped 2026-10-21 (`hd-throttle-and-flags`):
  [ps3-hdfury-eu/craft-flags.md](../../docs/ghidra/functions/ps3-hdfury-eu/craft-flags.md).
  From `0x20` up HD's bits are Pulse's shifted right by four; the sideshift grid gate, the
  Turbo laws, the doubling and the drag are measured on HD; **HD has no wrong-way kill**
  (`0x200` has no writer; live, an Eliminator craft flipped backwards keeps its engine), so
  HD, 2048 and Omega turned it off. Left open from that read:
  - **HD's Eliminator flip** (L1, "Flip your ship", about `0.15 s` for a half turn) is a
    player control ours does not have. Its code is unread.
  - **HD's `T += 100`** in `Craft_UpdateThrottle` while `craft+0x2e0 > 0` and grounded
    (`0x000efd2c..0x000efd54`), ahead of the doubling; `craft+0x2e0`'s writers are listed on
    the page, its meaning is not. Ours has no such term.
  - **`Craft_ApplyQuadraticDrag`'s `r4 != 0` arm** (velocity `x0.99`, or `x0.95` when
    `body+0x210 == 2`): no caller path read reaches it.
  - **HD's `0x400`** is read by `Craft_IntegrateHull` and two AI functions and has no
    setter; **2048's `0x400` and `0x800`** have setters HD lacks (five for `0x400`, one for
    `0x800`). 2048's `0x800` overrides the control record (`0x811cfb2c`, beside Pulse's
    forced-airbrake reader of `0x200` at `0x811cfb0c`), set by `FUN_812bda96`: read what
    sets it before trusting 2048's inherited "no wrong-way kill".
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
2. HD's flip: find the L1 binding's reader in an Eliminator (`scripts/rpcs3-hd-air-hold.py
   --script` logs the button words `+0x24`/`+0x28`), read what it writes to the body, then
   decide whether ours offers it.
