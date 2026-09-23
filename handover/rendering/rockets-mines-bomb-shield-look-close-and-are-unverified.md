# Rockets, mines, the bomb and the shield look close, and none is verified frame-by-frame

2026-09-17. The user, from play: "rockets, mines, bomb, shield look close, but
not fully verified." A verification thread, not a build thread: each of the
four has a recovered law and a screenshot pass at the time it landed, and
none has been compared side by side with the original at player size over
more than one frame since.

What each is known to leave out, from the source and the docs, so the pass
checks these first rather than rediscovering them:

- **Rocket**: `Rocket.vex` draws with the recovered forward alignment and a
  quarter-turn that is *not* recovered (`Race::rocket_model_matrices` doc
  comment); the `WO_ROCKET_FLARE` glow draws with a procedural falloff, not
  its sprite (the psys-sprites lane of 2026-09-17 is closing that). No
  launch-speed ramp has been read for it (the Plasma's was).
- **Mine**: `Pulse_Mine.vex` pose at drop is the craft's *body* orientation,
  chosen not measured (`mine::frozen_pose`); whether the caltrop's axis
  agrees with `Body::orientation` is unmeasured; `MINERADAR` is unwired
  (per-projectile held voice).
- **Bomb**: `Pulse_Bomb.vex` draws at rest; the detonation - `Bomb_Detonate`
  -> `BombBlast_Construct`/`BombBlast_Update`: `explosion_hemisphere.vex` +
  `WO_BOMB_SMOKERING` + `Bomb_Shockwave.vex`, both models eased per
  `BombBlast_Update`'s own three ramps - **built 2026-09-23**
  (`oag_game::race::bomb_blast`; see
  [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-23-the-blasts-own-per-tick-animator-read)
  for the full read, landed the same session). The shockwave's own recovered
  alpha fade is not wired (`bomb_blast`'s own module doc comment says why),
  the basis crosses this engine's `Vec3::Y` and the frozen orientation's
  forward axis rather than the executable's own unlocated rear-emitter row
  (chosen, not measured, same footing `mine::frozen_pose` already carries),
  and `BOMBLAUNCH`/`~BOMBRADAR` still have no recovered trigger.
- **Shield**: PSP law recovered; HD's own colours/constants landed 2026-09-16
  but the steady-state colour's source is untraced
  (`hds-shield-hit-flash-is-amber-and-the-target-colour-is-a-parameter.md`);
  the PS2 source renders it solid and static (its own thread).

## Open

- A side-by-side, three-frames-each capture per weapon per title against a
  PPSSPP (`docs/reverse-engineering/ppsspp-debugger.md`) / RPCS3 capture of
  the same moment, judged as a player would.

## Next Steps

1. Capture: `cargo run -p oag-game -- --race --give <rocket|mine|bomb|shield>
   --press square --ticks N --screenshot ...` at three ticks each, Pulse PSP
   and HD; same moments on the emulators.
2. For each visible difference, name the recovered-vs-chosen row above it
   falls under, and open a build thread only for a difference that is not
   already listed.
