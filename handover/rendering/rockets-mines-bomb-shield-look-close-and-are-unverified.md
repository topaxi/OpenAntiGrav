# Rockets, mines, the bomb and the shield look close, and none is verified frame-by-frame

2026-09-17. The user, from play: "rockets, mines, bomb, shield look close, but
not fully verified." A verification thread, not a build thread: each of the
four has a recovered law and a screenshot pass at the time it landed, and
none has been compared side by side with the original at player size over
more than one frame since.

What each is known to leave out, from the source and the docs, so the pass
checks these first rather than rediscovering them:

- **Rocket**: **2026-09-24** - the model's basis is measured live on PPSSPP
  (`n x f`, `n`, `f`, a rotation) and drawn so; ours was a reflection. The
  quarter-turn is the `WO_ROCKET_FLARE` frame's, not the model's, and the
  flare now rides it (`psys::Stage::orient`: its `+Y` is the velocity). See
  `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`'s 2026-09-24
  section. No launch-speed ramp has been read for it (the Plasma's was).
- **Mine**: **2026-09-24** - the pose is measured live and is not the
  craft's at all: `Mine_PoseNode` re-poses every laid mine each tick as
  `0.6 x Rot(axis, -4 x fuse)` about a per-mine random axis. Pulse draws
  that now (the axis values are a render-side hash, ours). HD keeps the
  frozen craft pose, chosen. `MINERADAR` is unwired (per-projectile held
  voice). See `mine.md`'s 2026-09-24 section.
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
  and `BOMBLAUNCH`/`~BOMBRADAR` still have no recovered trigger. **The laid
  Bomb's own pose is measured 2026-09-24**: `Bomb_Init` squares it to world
  `+Z` about the craft's up, scale 1, never re-posed - Pulse draws that now.
- **Shield**: PSP law recovered; HD's own colours/constants landed 2026-09-16
  but the steady-state colour's source is untraced
  (`hds-shield-hit-flash-is-amber-and-the-target-colour-is-a-parameter.md`);
  the PS2 source renders it solid and static (its own thread).

## Open

- A side-by-side, three-frames-each capture per weapon per title against a
  PPSSPP (`docs/reverse-engineering/ppsspp-debugger.md`) / RPCS3 capture of
  the same moment, judged as a player would. **Where it stands 2026-09-24:**
  - *Pulse Rocket* - one volley, three frames each (5/15/30 frames after
    firing), ours against PPSSPP on the same start straight and team. Not a
    matched state: the PPSSPP craft was at 140-237 km/h after `psp-drive.py
    restart`, ours at 48-100 km/h, so our volley met an opponent and
    detonated where the original's flew clear. The darts are a few pixels
    at player size on both; the flare is now a glow down the flight path on
    both. Frames: `data/scratch/weapon-pose/side-rocket-*.png`.
  - *Pulse Mine/Bomb* - **no capture of the original's picture**: the
    player's own charges land behind the chase camera, and setting the fire
    bit on an AI record (indices 0 and 6) is never consumed. The poses were
    measured from memory instead (above); a picture still wants a viewpoint
    behind the craft, or the AI's own drop path.
  - *Pulse Cannon* - the round's own basis measured live
    (`cannon-quake-leachbeam.md`, 2026-09-24); the node that draws it was
    not, and no frame compared.
  - *Missile* - nothing to pose: neither title draws a Missile model (Pulse
    rides `WO_MISSILE_HEAD` alone; HD's `HD_missile_ball_bloomring` is
    unwired).
  - *HD, every weapon* - no RPCS3 capture: firing on the emulator needs the
    held-weapon slot, which is unread (the same blocker the Plasma thread
    records).
  - *Shield* - not touched.

## Next Steps

0. For the Rocket, match state before comparing: drive both from one
   `.inputs` script (`psp-trace.py --script` / `--input-script`) instead of
   `psp-drive.py restart` plus a held button, so the two crafts fire at the
   same speed.
1. Capture: `cargo run -p oag-game -- --race --give <rocket|mine|bomb|shield>
   --press square --ticks N --screenshot ...` at three ticks each, Pulse PSP
   and HD; same moments on the emulators.
2. For each visible difference, name the recovered-vs-chosen row above it
   falls under, and open a build thread only for a difference that is not
   already listed.
