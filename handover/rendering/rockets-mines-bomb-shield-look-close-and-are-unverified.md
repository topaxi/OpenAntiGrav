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
  section. **2026-10-01**: measured - 0.75 x class for four frames, then the class speed alone (see Open).
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

**2026-10-01 pass (Pulse PSP only), matched state.** Method, numbers and
evidence are in `rocket-visuals.md`, `mine.md` and `shield-pickup.md` (each has a
2026-10-01 section); the harness is `scripts/psp-weapon-pair.py` with
`verification/scenarios/weapon-after-go.inputs` on our side. Frames stay under
`data/scratch/pulse-weapons/`.

- *Pulse Rocket* - **flight matched 2026-10-01, second pass; the look is not.** The four rows
  landed one commit each (cruise 222.22 u/s class alone; 0.75 x class until the first surface
  hit; spawn at the craft's position; and the two laws the re-measure found: the riding normal
  is seeded from the craft, and the probe-hit arm steers toward the ride point). A volley
  fired from the original's measured pose detonates at ticks 49/60/69 and 177/218/251 units
  against the original's 51/61/70 and 185/221/252
  (`crates/game/tests/rocket_launch_ground_truth.rs`, disc-backed). Evidence and the arm's
  read in `rocket-visuals.md`'s 2026-10-01 second-pass section. **Still open**: (a) the
  original's first two or three updates are a probe miss (`Collision_SweepSegment` returns
  `0x7f` with the floor four units below, so the rocket flies 166.67 u/s and falls for them);
  ours finds the floor on its first update. Cause unrecovered, see the page; (b) the wide
  orange glow at fire+3..+8 is still larger on the original than ours (480x272 pair,
  `data/scratch/pulse-weapon-laws/rocket-pair-after.png`); `WO_ROCKET_FLARE`'s parameters are
  unread; (c) our standing start puts the craft 2.4 units and 1.7 degrees off the original's
  at the same place, which alone moves a detonation by 30 ticks (spawn/handling, not this
  thread's).
- *Pulse Mine* - **pictured**: the charges are visible for two frames as the craft
  leaves them (fire at speed, photograph every frame). Same model, same cadence
  (6-7 frames). **2026-10-01, second pass**: the original lays at the craft's own position
  (stationary probe to the hundredth; `Bomb_Init`'s drop point equals the body position to
  the last bit, stationary and at speed), and `mine::drop_point` now does too (its own
  commit, `mine.md`'s second-pass section). The Mine's own `Anim Transform` (a tilted spin, 2 s a turn) now plays too, by analogy with the Bomb and not seen in the original's two frames.
- *Pulse Bomb* - **pictured**, same method; **the launch look is fixed 2026-10-01**
  (second pass). The wide flat ring was `Pulse_Bomb.vex`'s `orbit` node drawn at its
  time-zero pose: both `orbit` and `bomb` are keyframed `Anim Transform`s on the one
  animation clock (`orbit` turns about its own `X`, once in 3 s), so from behind the ring is
  always edge-on - a thin vertical shaft or a tilted arc by the clock's phase. The laid pools
  now write their node-animation table (`bomb_orbit_ground_truth.rs` pins it). The phase is
  our tick clock, not the original's session clock (`g_ingame->0x40`, 49-92 s at a launch),
  so a frame-exact ring pose is not claimable. Drop point as for the Mine. No detonation is in
  view for a moving craft on either side; the detonation animator itself still has no original
  picture (the owner trips its own charge only when stationary, and then at the craft, so a
  picture of it needs the craft stationary and the camera is inside the blast).
- *Pulse Shield* - **pictured**. Matches: violet to blue, size against the craft.
  **Onset resolved 2026-10-01 (second pass)**: a live `ShipShield_Update` probe shows nothing
  delays the shell - the object, its models and its colour, swell and clock match
  `ShipShield::advance` frame for frame on the same `dt` values (a test pins 30 live frames).
  The original's later onset is its jittered frame `dt`: `(int)(dt/substep)` is 0 on 47 of 200
  frames, so it took 0.765 substeps a frame against ours 1.0 (`shield-pickup.md`). Not
  reproduced and not chosen. Still differs: **brightness and banding**, ours dimmer at settled
  state (about a third of the original's mean pixel change by one rough measure); unexplained,
  and a frame-for-frame comparison needs the animation clock pinned on both sides (the shell's
  texture scrolls on it). Candidate: how `mesh+0x6c` (the colour `Image_SetVertexColours` writes)
  reaches the draw against ours multiplying the authored vertex colours.
- *Resolved 2026-10-01 (camera lane)*: the original's craft looked about 1.4 x
  larger because its fresh profile flies `OPT_CLOSE` and ours defaulted to `far`;
  the default is now `close` and a three-tick native comparison agrees. Pass
  `--camera-view far` only when the original capture was taken on `OPT_FAR`.
- *Pulse Cannon* - the round's own basis measured live
  (`cannon-quake-leachbeam.md`, 2026-09-24); the node that draws it was not,
  and no frame compared.
- *Missile* - nothing to pose: neither title draws a Missile model.
- *HD, every weapon* - no RPCS3 capture: firing on the emulator needs the
  held-weapon slot, which is unread.

## Next Steps

1. ~~A gameplay lane takes rows 1-4 of the Rocket~~ - done 2026-10-01, second pass.
   Open instead: find why the original's first updates miss the floor (a break on
   `Collision_RaycastWorld`'s hit kind `local_38` and on what the rocket copies from its owner
   at `self+0x114`), then the flare's parameters.
2. ~~Mine/Bomb drop point; the Bomb's ring and shaft~~ - done 2026-10-01, second pass.
3. ~~What delays the shield's first visible frame~~ - nothing does; the frame timer
   (above). Open instead: the shell's settled brightness, with the animation clock pinned on
   both sides (`--anim-seconds` on ours, `clock` in the harness rows) and `mesh+0x6c`'s path
   into the draw (`mesh-draw.md`).
4. Optional: re-run the Rocket probe on Flash to confirm 0.75 x class on a second
   speed class.
