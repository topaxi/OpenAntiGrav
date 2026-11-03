---
status: ready-to-wire
titles: [pure]
updated: 2026-11-03
summary: "Pure Plasma blast and Missile/Mine/Bomb/Disruptor triggers are read (plasma.md, weapon-effects.md): wire them; Disruptor on-craft effect open"
---

# Pure's weapons: Mine, Bomb, the Bomb blast and the Quake are wired; Plasma, Disruptor, Missile are open

2026-10-08. Evidence: `docs/ghidra/functions/psp-pure-usa/weapons-gfx.md`. Pure's table named Pulse's
weapon models (absent on its disc); it now names `Mine.vex`, `Bomb.vex` and the Bomb blast pair.

## Open

1. **Plasma blast: READ 2026-11-03, ready to wire** ([plasma.md](../../docs/ghidra/functions/psp-pure-usa/plasma.md)).
   Law (same mechanism as Pulse, Pure's own constants, confidence 85): per model a 3-key linear tint table
   (255 -> 0, the grey is the model's ambient): halo `0, 0.2, 0.5` s, noglow hemisphere `0, 0.3, 1.0` s, hemisphere `0, 1.0` s;
   scale eased per 60 Hz tick `cur += (target - cur) * rate` with targets `15, 8, 7.8` and rates `0.1, 0.07, 0.07`;
   animation time `age * 1.0`; life `2.0 s` (Pulse `1.5`); hemispheres in the world-fixed track frame at impact + `3.5`
   along the surface normal, halo at the impact point (camera-derived basis, confidence 55). Reuse Pulse's wired blast
   pass with these data and set `plasma_blast_pulse` for Pure. Open: craft hit blast count (Pulse spawns two), a live capture.
2. **Bomb tumble: READ, confidence 60** (weapon-effects.md, "Bomb tumble axes"): `-4.0 rad/s` about `(0, cos 0.25, sin 0.25)` and
   `0.5 rad/s` about `+Y`, then floor alignment and scale `0.4`. Ours yaws at `-3.5 rad/s`. The scale `0.4` and the rear anchor
   `4.875` are measured live on Feisar; other teams' anchors are assumed alike - measure one.
3. **Mine shading**: our Pure Mine draws darker than the original's light grey; compare textures/lighting.
4. **Triggers: READ 2026-11-03, ready to wire** (weapon-effects.md; same law as Pulse where Pulse is read): `WO_MINE_EXPLO` on each
   mine leaving the pool with its flag (fuse or proximity), `WO_MISSILE_HEAD` twice attached at spawn, `WO_MISSILE_BOUNCE` on each
   of the first four bounces (the fifth ends the flight), `WO_MISSILE_EXPLO` at the end of flight, `WO_BOMB_GLOW` as a child of the
   bomb node for its life, `WO_BOMB_SMOKERING` once at the blast, `WO_DISRUPTOR_LAUNCH` at fire (if the weapon record's `+0xe0` is
   set), `WO_DISRUPTOR_HEAD` attached to the bolt, `WO_DISRUPTOR_EXPLO` plus `electric_halo2.vex` (camera-facing, scale ease to `16`
   at rate `0.05`, grey fade `1 - age`, `1.0 s`) at bolt teardown (`10.0 s` or a hit). `explosion_gaseous` is the wreck's, not
   the Disruptor's. Still open: the on-craft Disruptor effect (`disruptor_effect`, `disruptor_cockpit`; `DisruptorFx_Construct`,
   update unread, address plus recipe in weapon-effects.md).
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

Wire Plasma first (data in plasma.md), then the Missile/Mine/Bomb/Disruptor triggers (weapon-effects.md), then read
`DisruptorFx_Construct`'s update. Capture a detonating Bomb and a Mine hit with `scripts/psp-pure-weapon-fire.py` (walk to Time Trial by hand,
RESTART, run it) to check the key times live.
