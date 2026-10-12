# Pure's weapons: Mine, Bomb, the Bomb blast and the Quake are wired; Plasma, Disruptor, Missile are open

2026-10-08. Evidence: `docs/ghidra/functions/psp-pure-usa/weapons-gfx.md`. Pure's table named Pulse's
weapon models (absent on its disc); it now names `Mine.vex`, `Bomb.vex` and the Bomb blast pair.

## Open

1. **Plasma blast**: `0x0885b2a0` loads `plasma_halo`, `plasma_hemisphere_noglow`, `plasma_hemisphere`
   over three 255-to-0 key tables (`+0x90`, `+0xc0`, `+0xf0`); read the keys and the placement, then wire.
   `plasma_blast_pulse` stays `None` until then (a billboard, not Pulse's baked track).
2. **Bomb tumble**: axes of the two turns in `Bomb_UpdateSpin` (`0x088583d8`, tables at `0x5420`/`0x5450`)
   are unread; ours yaws at `-3.5 rad/s` (chosen). The scale is `0.4` and the drop point the rear anchor
   (`4.875` behind the body), both measured live on Feisar; other teams' anchors are assumed alike - measure one.
3. **Mine shading**: our Pure Mine draws darker than the original's light grey; compare textures/lighting.
4. **Triggers unread** for `WO_MINE_EXPLO`, `WO_BOMB_GLOW`, `WO_BOMB_SMOKERING`, `WO_MISSILE_HEAD/EXPLO/BOUNCE`,
   `WO_DISRUPTOR_*`, `disruptor_effect`, `electric_halo2`, `explosion_gaseous`. (`WO_QUAKE` is wired, below.)
5. **A detonating Bomb** has not been seen on either side (needs a craft entering the trigger radius).
6. **State match**: the pair is time-matched (race time `2.1 s`), not speed-matched; Pure's HUD km/h and our
   speed unit differ. Missile, Disruptor, Shield, Autopilot, Turbo were not captured (the Quake was, 2026-10-12).
7. **Quake** (lane pure-leach-quake, `docs/ghidra/functions/psp-pure-usa/quake.md`): wired - the extent law
   for Pure's own `WO_QUAKE`, a world-up frame lifted 10 units, and the orange wash (`oag_title::QuakeLook`).
   Still open: **the road ripple** (Vineta K carries one class `0x379` `Quake` node; Pure's ripple is unread
   and ours draws none), Pure's `ScreenFlash_Update`/draw (the wash is Pulse's law, inherited), the wave's
   speed (Pulse's 270, chosen), the second instance on a fork, the VR build (`WO_QUAKE_VR`, no lift), and a
   state-matched capture (Pure's physics is unimplemented, so ours is further along at the same race time).
   Pulse-only names still in Pure's effect table report as absent on every load (`WO_CANNON_SPARKS`,
   `WO_REPULSER*`, `WO_SHURIKEN_*`, the Cannon textures); the LeachBeam ones are gone (Pure ships no LeachBeam).

## Next Steps

Capture a detonating Bomb and a Mine hit with `scripts/psp-pure-weapon-fire.py` (walk to Time Trial by hand,
RESTART, run it); then Plasma's key tables.
