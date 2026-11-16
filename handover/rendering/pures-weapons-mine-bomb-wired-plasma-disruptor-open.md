---
status: measure
titles: [pure]
updated: 2026-11-11
summary: "Pure weapon effects wired from reads (Disruptor complete bar cockpit); open: Plasma/halo capture at a matched state, craft-hit count"
---

# Pure's weapons: all wired from reads; captures and a few gates open

2026-10-08. Evidence: `docs/ghidra/functions/psp-pure-usa/weapons-gfx.md`. Pure's table named Pulse's
weapon models (absent on its disc); it now names `Mine.vex`, `Bomb.vex` and the Bomb blast pair.

## Open

Wired 2026-11-03 (lane `pure-weapons-wire`, [plasma.md](../../docs/ghidra/functions/psp-pure-usa/plasma.md) and
[weapon-effects.md](../../docs/ghidra/functions/psp-pure-usa/weapon-effects.md), "Wired"): the Plasma blast (own key tables,
targets, 2.0 s life, 3.5 hemisphere drop, through `PlasmaBlastLaw`), the Missile/Mine/Bomb/Plasma triggers as `Measured`,
`WO_BOMB_GLOW`, the Bomb tumble axes (confidence 60), `WO_DISRUPTOR_HEAD`, `WO_DISRUPTOR_EXPLO` and `electric_halo2`. Wired 2026-11-11 (lane
`pure-disruptor-finish`, [weapon-effects.md](../../docs/ghidra/functions/psp-pure-usa/weapon-effects.md), "Disruptor"): `WO_DISRUPTOR_LAUNCH` at the hull's
`Ship Muzzle` node (the gate is the hull authoring one, not a table field) and `disruptor_effect.vex` on a disrupted craft. Still open:

1. **A Pure capture of the blast and the halo, at a matched state.** All of it is read, none matched: the Pulse-fitted model unit (`0.025`)
   is assumed for Pure's models, the halo's texture clock is the age by analogy, and the hemisphere drop's sign is Pulse's (against the track's
   up, read off Pulse's GE dumps). Lane `pure-disruptor-finish` captured a Disruptor fire (launch burst, bolt, wall spark) and the on-craft
   effect on PPSSPP (recipe on the evidence page, "Not settled") but not a Plasma blast or a halo against a wall at a state ours can match:
   a Time Trial has no opponent to hit and the craft that fires pins itself on the wall. Fire weapon 8 from the loaded
   `time-trial-vineta-vector-feisar-restart.ppst` (`psp-state.py --serial UCUS98612_2.00`) at the grid pose and run ours from the grid pose.
2. **Craft-hit blast count on Pure** (ours spawns Pulse's two; Pure's callers were not walked, confidence 50).
3. **`disruptor_cockpit.vex`**: the cockpit view's half of the on-craft effect (the model shows when `*(craft+0xe5)` is non-zero, which
   read `1` after the second SELECT press); its placement in the eye's frame is unread, and ours draws no on-craft effect on the player's craft
   in its internal view.
4. ~~Pulse's own hemisphere drop~~ done (plasma-dome-specks): `3.5` on Pulse's row, sign read off the dumps; the dark discs are `WO_PLASMA_FLASH` particles, not the shell texture.
5. **The laid Bomb's glow** was not seen in a headless shot (the bomb had not left the craft); a detonating Bomb has not been
   seen on either side.
6. **Mine shading**: our Pure Mine draws darker than the original's light grey.
7. **Quake** (lane pure-leach-quake, [quake.md](../../docs/ghidra/functions/psp-pure-usa/quake.md)): wired. Still open: the road
   ripple (Vineta K carries one class `0x379` `Quake` node; Pure's ripple is unread and ours draws none), Pure's
   `ScreenFlash_Update`/draw (the wash is Pulse's law, inherited), the wave's speed (Pulse's 270, chosen), the second instance
   on a fork, the VR build (`WO_QUAKE_VR`, no lift), and a state-matched capture.
   Pulse-only names still in Pure's effect table report as absent on every load (`WO_CANNON_SPARKS`, `WO_REPULSER*`,
   `WO_SHURIKEN_*`, the Cannon textures).

## Next Steps

Capture a Pure Plasma blast at a state ours can match (recipe in item 1) to check the model unit, the drop and the halo, then read the
cockpit view's `disruptor_cockpit.vex` placement.
