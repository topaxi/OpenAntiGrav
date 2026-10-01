# A wrecked craft draws its wreck, explodes, and the player is cut to the circuit's own camera

2026-10-01, lanes `pulse-wreck` and `pulse-wreck-2`. Evidence pages:
[ship-wreck-model.md](../../docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md),
[camera.md](../../docs/ghidra/functions/psp-pulse-usa/camera.md) ("The destroy camera"),
[screen-flash-callers.md](../../docs/ghidra/functions/psp-pulse-usa/screen-flash-callers.md).

**Landed.** `Ship_SetState` case 5 makes `shipwreck.vex` the live model
(`CraftState::Eliminated`; `oag_game::race::scene::wreck`), and `race::wreck_fx` throws
`WO_SHIP_FXNODE_EXPLO` and `WO_SHIP_DEATH_SPARKS` at each wreck locator on that edge,
then `WO_SHIP_EXPLOSION` 1.5 s later at the wreck's own model matrix moved
`4.0` rows along `-up` (`FUN_088407b0`, read and measured live: the matrix rows were 0.75
long and the translation `4.0 * 0.7499` below the craft). The player is cut to the
circuit's own authored `Camera` node nearest the wreck's aim point, zoomed to frame 35
units (`oag_render::camera::destroy`, pinned to the running original's per-frame
field of view and view matrix; `destroy_camera_ground_truth.rs` pins it on the disc). The
player's own state-5 wash is no longer skipped and the player's two explosion shakes are
armed. **The fireballs read white because twelve PSP sprites were 4 bits per pixel and the
reader refused them** (`oag_vex::pob::texture`); fixed, and the picture now matches the
original's orange textured fireball frame for frame at 480x272 (the opponent wreck,
`cap-opp` against ours, k 36 to 140).

## Open

- **A finished race freezes the world**, so a player's wreck in a Single Race or Zone is
  one frozen frame under the results panel: the destroy camera, but no state 6, no big
  explosion for the player and, by choice, no state-5 shake (it would freeze at its
  largest frame). The original keeps running. Only an Eliminator plays the sequence.
- **The Eliminator respawns the craft at the same tick the big explosion goes off**
  (`ELIMINATOR_RESPAWN_DELAY` is the state-5 dwell alone; the original adds state 8's
  timer), so the player never sees the explosion from the destroy camera. The blast
  is placed with the matrix the craft was wrecked with, so it still lands on the wreck.
- The explosion's smoke is thinner and its fire brighter and longer in ours at k 160 to
  180 (fire-coloured pixels 2.2x to 2.4x the original's). Not chased: the `0.75` rows of
  the matrix `Psys_Spawn_q` is given may scale emitter positions and speeds (unread), and
  the explosion's alpha-over emitters were not compared emitter by emitter. A GE dump
  (`scripts/psp-ge-dump.py`, vertex type `0x11e` batches) separates them.
- `FUN_088407b0` also builds a `Data\Weapons\Bomb_Shockwave.vex` object (`FUN_0885ecf0`
  under `DAT_08b34320`, non-zero live) - read, not drawn.
- The original hides the HUD by degrees from about 30 frames into state 5; ours keeps it.
- The camera lets go "when the craft is racing again" (chosen); the original's hand-over to
  another craft after ten seconds, and `Camera_RepickNearSubject_q`, are read, not ported.
  The focus rate `0.4` was seen on Venom only (Flash `0.5`, Rapier and Phantom `0.6` read).
- The wreck's two authored `Trail` nodes ([exhaust.md](../../docs/ghidra/functions/psp-pulse-usa/exhaust.md)):
  not looked for in the original's wreck frames.
- Under a wreck our blob shadow, shield shell and absorb overlays still draw as for a hull; the
  engine flare quad is hidden (seen, mechanism unread). `+0x2c` bit `0x1000000` appears on the wreck
  at state 6, unread.
- PS2 Pulse, Pure and HD: none of their wrecks or `Ship_SetState` is read; Zone's `zonewreck.vex`
  loads but no Zone craft was wrecked on the original.
- Not confirmed against a craft a race eliminated (`scripts/psp-wreck-capture.py` calls
  `Ship_SetState(entity, 4)`).

## Next Steps

1. Let a finished race keep stepping its cosmetics (shake, particles, the wreck's explosion)
   so a Single Race or Zone wreck plays out under the results; then arm the state-5 shake there.
2. Give the Eliminator the original's state-8 wait so the destroy camera holds through the
   explosion, and hide the HUD as state 5 goes on.
3. Compare the explosion emitter by emitter against a GE dump taken a few frames in
   (`scripts/psp-ge-dump.py`, `gestate`-style per-PRIM state), then read what scale the matrix applies.
4. `scripts/psp-wreck-capture.py --state 5` on a craft an Eliminator run really eliminated.
