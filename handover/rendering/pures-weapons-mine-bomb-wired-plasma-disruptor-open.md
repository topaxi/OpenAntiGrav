---
status: measure
titles: [pure]
updated: 2026-11-03
summary: "Pure Plasma, Missile/Mine/Bomb, Disruptor effects wired from reads; needs a Pure capture, craft-hit count, Disruptor launch gate"
---

# Pure's weapons: all wired from reads; captures and a few gates open

2026-10-08. Evidence: `docs/ghidra/functions/psp-pure-usa/weapons-gfx.md`. Pure's table named Pulse's
weapon models (absent on its disc); it now names `Mine.vex`, `Bomb.vex` and the Bomb blast pair.

## Open

Wired 2026-11-03 (lane `pure-weapons-wire`, [plasma.md](../../docs/ghidra/functions/psp-pure-usa/plasma.md) and
[weapon-effects.md](../../docs/ghidra/functions/psp-pure-usa/weapon-effects.md), "Wired"): the Plasma blast (own key tables,
targets, 2.0 s life, 3.5 hemisphere drop, through `PlasmaBlastLaw`), the Missile/Mine/Bomb/Plasma triggers as `Measured`,
`WO_BOMB_GLOW`, the Bomb tumble axes (confidence 60), `WO_DISRUPTOR_HEAD`, `WO_DISRUPTOR_EXPLO` and `electric_halo2`. Still open:

1. **A Pure capture of the blast and the halo.** All of it is read, none matched: the Pulse-fitted model unit (`0.025`) is
   assumed for Pure's models, the halo's texture clock is the age by analogy, and the hemisphere drop's sign is chosen
   (against the track's up). A `scripts/psp-pure-weapon-fire.py` shot of Plasma and Disruptor settles all three.
2. **Craft-hit blast count on Pure** (ours spawns Pulse's two; Pure's callers were not walked, confidence 50).
3. **`WO_DISRUPTOR_LAUNCH`**: gated on `*(weapon+0x3c + 0xe0)` of the firing craft's weapon record; map the offset to a table
   field and add the trigger.
4. **The on-craft Disruptor effect** (`DisruptorFx_Construct` `0x08859b78`, confidence 55: `disruptor_effect`, `disruptor_cockpit`;
   read the update slot of the vtable at `0x08abc690`).
5. **Pulse's own hemisphere drop** (`PlasmaBlastLaw::hemisphere_drop` is `0.0` on Pulse's row though the GE dumps put its halo
   and hemispheres 3.49 apart), for a Pulse lane.
6. **The laid Bomb's glow** was not seen in a headless shot (the bomb had not left the craft); a detonating Bomb has not been
   seen on either side.
7. **Mine shading**: our Pure Mine draws darker than the original's light grey.
8. **Quake** (lane pure-leach-quake, [quake.md](../../docs/ghidra/functions/psp-pure-usa/quake.md)): wired. Still open: the road
   ripple (Vineta K carries one class `0x379` `Quake` node; Pure's ripple is unread and ours draws none), Pure's
   `ScreenFlash_Update`/draw (the wash is Pulse's law, inherited), the wave's speed (Pulse's 270, chosen), the second instance
   on a fork, the VR build (`WO_QUAKE_VR`, no lift), and a state-matched capture.
   Pulse-only names still in Pure's effect table report as absent on every load (`WO_CANNON_SPARKS`, `WO_REPULSER*`,
   `WO_SHURIKEN_*`, the Cannon textures).

## Next Steps

Capture a Pure Plasma blast and a Disruptor hit (`scripts/psp-pure-weapon-fire.py`: walk to Time Trial by hand, RESTART, run it) to
check the model unit, the drop and the halo, then map the Launch gate and read `DisruptorFx_Construct`'s update.
