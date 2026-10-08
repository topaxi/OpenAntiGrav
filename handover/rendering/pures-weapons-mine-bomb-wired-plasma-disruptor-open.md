# Pure's weapons: Mine, Bomb and the Bomb blast are wired; Plasma, Disruptor, Missile, Quake are open

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
   `WO_DISRUPTOR_*`, `WO_QUAKE*`, `disruptor_effect`, `electric_halo2`, `explosion_gaseous`.
5. **A detonating Bomb** has not been seen on either side (needs a craft entering the trigger radius).
6. **State match**: the pair is time-matched (race time `2.1 s`), not speed-matched; Pure's HUD km/h and our
   speed unit differ. Missile, Quake, Disruptor, Shield, Autopilot, Turbo were not captured.

## Next Steps

Capture a detonating Bomb and a Mine hit with `scripts/psp-pure-weapon-fire.py` (walk to Time Trial by hand,
RESTART, run it); then Plasma's key tables.
