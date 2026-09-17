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
- **Bomb**: `Pulse_Bomb.vex` draws; the detonation is **read and not built**
  (`Bomb_Detonate` -> `BombBlast_Construct`: `explosion_hemisphere.vex` +
  `WO_BOMB_SMOKERING` + `Bomb_Shockwave.vex` one unit below, ramp constants
  on [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md));
  `Race::blast_for(Bomb, ..)` returns `None`, so a bomb detonates with **no
  visual at all** - an honest absence, and the largest gap of the four.
  `BOMBLAUNCH`/`~BOMBRADAR` have no recovered trigger.
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
