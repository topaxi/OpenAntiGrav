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
  (`oag_raceplay::bomb_blast`; see
  [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-23-the-blasts-own-per-tick-animator-read)
  for the full read, landed the same session). ~~The shockwave's own recovered
  alpha fade is not wired~~ (**wired 2026-10-01**, `Drawable::tint`; struck 2026-10-08 after a grep of `crates/raceplay/src/bomb_blast.rs`),
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
- *Pulse Mine's explosion, 2026-10-08 (pulse-weapon-look)* - **the cluster is the round counter, and our drop is a tick slow: fix prepared, not landed.** One round on the original is one blast, five rounds four bumps (the fifth mine's owner is out of the trigger radius); the "hue" gap was a single blast against a five-mine stack (cluster against cluster the mean RGB agrees within 3 %). Ours dropped the cluster at seven-tick spacing (a strict `> 0` on the `0.1` s reload at an exact `1/60` step), so only three blasts go off; at six ticks (`DROP_TIMER_SLACK`, chosen) they go at k=32/38/44/50 against 32/38/44/51. **Not landed: it raises three `ai_dekonstruct_black_ground_truth` cells over their "may only fall" ceilings** (the AI Aces play with weapons on; mine kills or chaotic per-seed divergence, not checked); patch `data/scratch/pulse-weapon-look/mine-drop-six-ticks.patch`, decision: land it and regenerate those ceilings in their own commit, or have the AI avoid its field's mines first. Left: ours' blasts 8-14 % brighter, the original's debris larger and tan. `mine.md`'s 2026-10-08 section.
- *Pulse Rocket glow, 2026-10-08 (pulse-weapon-look)* - **item (b) above is closed as stated, and a new episodic excess is left.** At fire+3..+8 ours is 2.20M warm light against the original's 2.46M/2.53M (two restarts in one boot), so "larger on the original" is false; from fire+9 ours is about 1.5x in spikes at fire+9/16/20/25. The flare pool (count, size, alpha, colour, position) and the blend equal the original's, so the excess is not the effect's law: the SHAZZAM draw's random sizes or the road's bloom, not isolated. The scenario `weapon-after-go.inputs` was ten ticks late (our start now reaches x 124.9 on tick 393): `weapon-after-go-matched.inputs`, ours at tick `391 + k`. `rocket-visuals.md`'s 2026-10-08 section.
- *Pulse Mine's explosion, 2026-10-01 (pulse-fx-3)* - **first picture of the original's**: a stationary craft trips its own Mine at fire+30
  (the arming delay; ours now exempts the owner for the same 0.5 s, 2026-10-07), and the burst runs about twenty frames of yellow-white wash, rays and orange debris
  where ours' lasts sixteen and leans green. The `ring` and `BANG` templates were drawn with the wrong sprite and are fixed
  (`particle-system.md`); the pools agree at first order. Open: the hue (red `215` against `194`), the debris colour and size, the flash's duration
  and whether the original's Mine cluster (several charges) is why the original's wash has three bumps. `mine.md`'s 2026-10-01 pulse-fx-3 section.
- *Pulse Mine* - **pictured**: the charges are visible for two frames as the craft
  leaves them (fire at speed, photograph every frame). Same model, same cadence
  (6-7 frames). **2026-10-01, second pass**: the original lays at the craft's own position
  (stationary probe to the hundredth; `Bomb_Init`'s drop point equals the body position to
  the last bit, stationary and at speed), and `mine::drop_point` now does too (its own
  commit, `mine.md`'s second-pass section). The Mine's own `Anim Transform` (a tilted spin, 2 s a turn) now plays too, by analogy with the Bomb and not seen in the original's two frames.
- *Pulse Bomb, pulse-fx-3 2026-10-01 - narrowed, not closed.* **`BombBlast_Update` steps its three eases `(int)(dt / (1/60))` times with no
  remainder** (read), so on PPSSPP the dome and ring ran about a third slow in every capture of the original taken so far
  (`ship-shockwave.md`); a frame-by-frame comparison must be at equal step count. The ring's `TEXLEVEL` bias (`2.875`) is the per-texture value
  `Texture_BuildBindList` computes (`mesh-draw.md`, "read 2026-10-01"): level 0 at the ring's 12-15 units either way, so it is **not** the dim ring.
  The dome's `0x13d` draws are unlit additive with the texel's alpha `80`; its yellow-opaque read is plausibly bloom (`pulse-hull-bloom`'s area).
  The launch ring and the +50 smoke were not re-looked at.
- *Pulse Bomb, third pass 2026-10-01* - the weapon bodies were missing from the glow-mask stamp
  list (the canister bloomed pale; now stamps `4`/`0xba` as the original, EDRAM-measured, pinned by
  `weapon_stamp_ground_truth`), and **the detonation has its first picture**: a laid Bomb moved
  120 units ahead by a debugger write (`psp-weapon-pair.py --detonate-bomb-at`). Ours matched the
  orange wash and timing; the smoke was a puff because psys shape 3 spawned at the anchor where the
  original spawns on a ring of the authored extent (fixed, `ParticleSystem_EmitRing`; also moves
  `WO_SHIP_EXPLOSION`, the Rocket's debris, the missile, the Repulser blast). **Still open**:
  the launch ring's brightness (the original's is a thick solid yellow band, ours three dim lines;
  GE state read, not isolated - `mine.md`'s third pass lists what matches), the top plate's hue,
  the dome reading yellow-opaque in the original and white-thin in ours, the smoke's density at
  fire+50. `0x200000` (evenly stepped ring angles, `WO_REPULSER_BLAST` only) is not played.
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
- *Bomb_Shockwave's fade, 2026-10-01 (pulse-fx-recheck)*: **wired.** `BombBlast_Update`'s `+0xf4` alpha ease goes through
  `Image_SetVertexColours` to the GE's ambient light alpha (`0x5d`, written before each strip: `0xf4`, `0xb8` on the ship
  explosion's ring, which is the same `.vex`), vertex colours the material: `Drawable::tint([1, 1, 1, alpha])`
  (`ship-shockwave.md`). The Bomb's own ring was not dumped. The ring, plate, dome and +50 smoke differences above were **not** re-looked at.
- *Bomb blast and Repulser field textures, 2026-10-05 (fx-age-clocks)*: **wired and measured.** Their texture tracks play on the object's own age
  (seeded `0` at spawn, rate 1; `anim-transform.md`, "A mesh's texture time"), uploaded through `write_anims` in `write_bomb_blasts`. Open: no `oag-game`
  scenario detonates the player's own bomb, so the Bomb has no before/after picture, only the live measurement and the draw-struct test.
- *Pulse Shield* - **matched 2026-10-01 (third pass)**, pixel for pixel at a pinned
  clock. The "ours dimmer, banding softer" gap was two defects, both fixed:
  `pulse_shield_test_ADD` (flags `0xe5`, pre-swizzled) was decoded linearly, and the
  shell's authored `u` scroll was never written. Against the original's own frame
  (hull-less difference, same craft and tick, clock pinned and one display frame
  allowed for) the per-channel correlation is **0.90 to 0.93**, ours about 10 % brighter.
  The shell's GE state is read (lighting on with the shield colour as the scene ambient,
  additive, no stencil so no glow stamp, cull off) and the `mesh+0x6c` candidate is closed.
  Onset (the frame timer) was resolved earlier. Still open: the cockpit sphere
  (`vr_shield_cockpit.vex`, `noise1_ADD`) is not compared and its texture transform is not
  written either; HD's steady-state colour (its own thread); the PS2 shell (its thread).
  Evidence: `shield-pickup.md`'s third-pass section. **The decoder fix also changes**
  every flagged version-6 `.vex` texture on Pulse PSP (75 nodes: all shield shells, `Pulse_Bomb.vex`'s
  three textures, the shuriken, cage and mag-effect textures, and seven ship-shaped models whose use is
  unidentified; `ship_FE.vex` and race hulls are unflagged): a Bomb comparison taken before 2026-10-01
  read them scrambled. `Pulse_Bomb.vex`'s three textures
  (`pulse_bomb`, `mine001_ADD`, `mine_flash_GLOW`) were sheared noise and now decode as a clean
  hatch plate and a glow flare (`data/scratch/pulse-shield-look/shots/bomb-tex-before-after.png`);
  `Pulse_Mine.vex` is flagged `0xe4` and did not change. **Re-look at the Bomb against the
  original**: the "bands" read off its canister on 2026-10-01 were this scramble.
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
- *Owner exemption, 2026-10-07 (bomb-owner)*: **landed.** The owner is exempt from its own Mine or Bomb for 0.5 s (`OWNER_EXEMPT_SECONDS`), then trips it; a stationary craft's own charge now goes off at about fire+30 frames as the original's does. HD's trip writer (flag `0x80` of `NormalBomb+0x40`) is not found, the 0.5 s rests on the film. The ground-truth ignition hook for a Pulse Mine comparison is no longer needed.

## Next Steps

1. ~~A gameplay lane takes rows 1-4 of the Rocket~~ - done 2026-10-01, second pass.
   Open instead: find why the original's first updates miss the floor (a break on
   `Collision_RaycastWorld`'s hit kind `local_38` and on what the rocket copies from its owner
   at `self+0x114`), then the flare's parameters.
2. ~~Mine/Bomb drop point; the Bomb's ring and shaft~~ - done 2026-10-01, second pass.
3. ~~What delays the shield's first visible frame~~ - the frame timer. ~~The shell's settled
   brightness and banding~~ - done 2026-10-01 (third pass): texture swizzle and scroll.
   Next: the cockpit sphere's own transform and a frame of it (the cockpit camera), and a
   re-look at the Mine and Bomb against the original now that their textures decode as the
   GE reads them.
4. Optional: re-run the Rocket probe on Flash to confirm 0.75 x class on a second
   speed class.
5. The Bomb's remaining differences (see the third-pass bullet): start from the ring's
   `TEXLEVEL` bias `2.875` against ours `1.0` (a per-texture value `Texture_BuildBindList`
   emits, read for this one texture only - read it for every `.vex` texture and apply it
   wherever it moves a level), then the dome's colour against its bloom.
6. (Pool census re-run 2026-10-01, pulse-fx-recheck: `WO_ROCKET_EXPLO`'s eight pools agree to 10 % in steady
   state and the `GLOW` is the same law; the matrix is identity so the ship explosion's `0.75` law does not apply. The
   picture series still wants a state where the player's Rocket meets a grid craft - fire at GO, not GO + 120.)
   Re-run the craft-hit Rocket blast's frame series (`rocket-visuals.md`, 2026-09-24): the
   pool particle's first draw at age 0 and the shape-3 ring/disc both move it, and it has
   not been compared since.
