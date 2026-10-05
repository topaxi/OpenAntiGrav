# A fired Cannon round: the body, its impact, its bolt streak and its muzzle flash are all drawn now

2026-09-17: **both hand-built quads landed** - see "What landed" below for the
new entry. The one thing still open is capturing the wall-hit spark's own
frame (open item 2); everything else this thread ever tracked is done. The
coordinator can delete this file and its `HANDOVER.md` index line once that
capture lands or is judged not worth chasing further.

2026-09-08/09. The thread's original premise - that the round is a
`Data\Psys\*.POB` effect whose trigger had not been found, with `Ship Muzzle`
(`0x3e2`) and `cannon_flash` (`0x3eb`) as candidates - was **wrong**, and both
candidates are dead. The Cannon names its own assets in plain strings in
`BOOT.BIN`, all three resolve in `Data.wad` on both PSP pressings, the round
draws as the mesh the original hangs on it, and (2026-09-09) its impact now
throws a spark. See
[`cannon-quake-leachbeam.md`](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
"What draws a Cannon round" and "`Cannon_UpdateRound` read".

## What landed

- **The two hand-built quads draw, 2026-09-17.** `Cannon_DrawRound`,
  `Cannon_BuildBoltList`/`Cannon_BuildMuzzleFlashList`, `Cannon_UpdateRound`
  and `Cannon_Construct` were re-read at instruction level for the actual
  vertex geometry - see "The two hand-built quads' geometry, read - and
  drawn" on the evidence page for the full recovery. In short:
  - **The bolt** is two camera-facing ribbons crossed at 90 degrees (a
    volumetric streak cross-section), between the round's position last
    tick and a point 20% of the way back from this tick's position toward
    last tick's, half-width `0.35`, fixed opaque white. Drawn every tick a
    round is alive.
  - **The muzzle flash** is one quad centred on the round, half-size and
    rotation freely rerolled every tick, fixed white with a randomly
    rerolled **alpha** (not RGB - correcting this thread's and the
    2026-09-08 entry's "randomly coloured" framing). Drawn only while the
    round's age is under `0.1` seconds.
  - Both textured from `Data\Weapons\Textures\Cannon_bolt.mip`/
    `Cannon_muzzle_flash.mip` (`Data.wad` entries 1057/1058) through a new
    `oag_fx::weapon_quads::Pipeline`, modelled on `exhaust::Pipeline`
    and reusing its recovered additive blend
    (`Gu_BlendFunc(0, 2, 10, 0, 0xffffff)` matches exactly). Depth write is
    **on** here, unlike the exhaust flare's - read directly at both list
    builders, kept as measured.
  - The flash's per-tick random roll is generated render-side
    (`oag_fx::weapon_quads::random`, seeded from the round's pool slot
    and the simulation tick), never from `world.rng`, so drawing it moves
    no committed determinism hash.
  - Two new functions recovered and named in the same pass:
    `Cannon_RotateFlashCorner` (`0x08864ea0`, confidence 82) and
    `Cannon_BuildRoundBasis` (`0x08864f54`, confidence 75 - the mechanism is
    read, one conditional offset's *purpose* is not).
  - **Judged on a screen**, real disc, several frames, at the size a player
    sees: the muzzle flash's warm yellow-orange glow (the texture's own
    authored colour, not invented) draws directly around the dart mesh at
    the moment of firing, older rounds further down the track correctly
    lose the flash once past the `0.1` s window and show only the small
    pale dart, and a whole round-robin burst is visible as a string of such
    dots trailing into the distance. See "Judged on a screen" below for the
    exact command and what was and was not distinguishable.
  - **Not read and not built**: `Cannon_UpdateRound` also drives a small
    extra particle-like spawn (`FUN_08945284`, the same function
    `Cannon_Init` calls once at spawn) inside the same age-under-`0.1s`
    window - reads as a separate spark/particle object at the muzzle, not
    part of either textured quad, and is out of this pass's scope. Left as
    an open item on the evidence page.
- **The round has a body.** `oag_raceplay::CANNON_MODEL_ENTRY` is
  `Data\Weapons\pulse_muzzleflash.vex` (`0x08a7c85c`), which
  `Cannon_Construct` (`0x088651d8`) loads into every round instance's own scene
  node, through the same path the Rocket's, Mine's and Bomb's models already
  take. **The mesh is named `muzzleflash` and is the bolt**: 20 vertices, 18
  triangles, spanning `1.200 x 1.200 x 3.599` - a dart, longest along the +Z
  the matrix aims down the velocity. Confirmed on a real disc, several frames,
  at the size a player sees - see "Judged on a screen" below.
- **The wall hit throws a spark; a craft hit does not.** `Cannon_UpdateRound`
  (`0x0886593c`, EU `0x08865798`) raycasts each round against the track's own
  collision mesh every tick and spawns `Data\Psys\WO_CANNON_SPARKS.POB` only
  on a world/track hit. The craft-hit path is a wholly separate three-function
  chain (`Cannon_TestCraftHit` -> `Cannon_MarkCraftHit` ->
  `Cannon_ApplyCraftDamage`) that applies `damage_per_bullet`/`slowdown_time`
  and a sound cue but never calls `Psys_Spawn_q` - confirming
  `oag_weapons::projectile::cannon::direct_hit`'s existing shape rather than
  changing it. Wired through `Race::blast_for`'s existing per-impact
  dispatch: `CANNON_SPARKS_EFFECT` plays when `struck` is `None`, nothing when
  it is `Some`.
- **Which display list is the bolt and which the flash is settled**,
  confidence 88, not the load-order guess this thread carried at 40.
  `Cannon_DrawRound` binds `g_cannon_bolt_texture` before every frame's
  `Cannon_BuildBoltList` call and `g_cannon_muzzle_flash_texture` before
  `Cannon_BuildMuzzleFlashList`'s, only while the round is under `0.1` seconds
  old - an instruction-level bind-then-call-list order, stronger evidence than
  the archive's entry-adjacency cross-check this thread proposed and never
  spent.
- **The base speed is measured, not chosen.** `func_0x00060af4`
  (`0x08864af4`/EU `0x08864950`) decompiles to an unconditional
  `return 500.0f` that never reads its own argument - not per-class, as the
  earlier framing assumed. `BASE_SPEED_KMH` corrected from the `400.0`
  placeholder to `500.0`.
- **The directory layout, which the obvious guess gets wrong.** The mesh is in
  `Data\Weapons\`; the two textures are one level deeper in
  `Data\Weapons\Textures\`. `Data\Weapons\Cannon_bolt.mip` hashes to an entry
  that does not exist. Recorded on the evidence page with the hashes, entry
  numbers and the exact command, because the same split very likely holds for
  the other weapons.

## Judged on a screen

`cargo run -p oag-game -- --race --mode single_race --give cannon --hold cross
--press square --no-audio --ticks 460 --screenshot /tmp/out.png` (headless
`--screenshot` renders without a display, no Xvfb needed) shows the round's
own dart-shaped mesh in flight ahead of the player's craft at ticks 320, 340
and 400 of a fresh single-race start - visually distinct from the ship's own
exhaust glow. This is the check none of the geometry/axis tests could make.

**2026-09-17 pass, same command family, ticks 60 through 460**: the muzzle
flash's warm yellow-orange additive glow is clearly visible around the dart
mesh on a freshly-fired round every time one exists in frame, and at tick 460
- once the ship is moving and a round-robin burst has built up - a whole
string of older rounds trails into the distance as small pale dots with no
flash, exactly the `age < 0.1 s` gate's predicted shape. **The bolt streak
was not separately distinguishable from the dart mesh/flash in any captured
frame.** The geometry is implemented per the recovered addresses and
unit-tested (the two ribbons are built perpendicular, at the right
half-width, between the right two endpoints - see
`oag_fx::weapon_quads::tests`), but every capture this pass took is from
the chase camera looking almost exactly down each round's own flight
direction, which foreshortens a streak built along that same axis to close
to a point - an honest gap in what a screenshot can show, not a sign the
code is wrong. A side-on or track-curve capture catching a round travelling
across the frame rather than away from it would be the next thing to try if
this matters again.

No frame catching the wall-impact spark itself was captured this pass either
- a straight-held throttle rarely steers a round into a wall inside a short
capture window - so the spark's *code* path is verified
(`a_cannon_round_sparks_on_a_wall_and_silently_on_a_craft`) but not yet its
own frame.

## Open

- **No frame has caught the wall-hit spark itself**, only the round in
  flight. The trigger is code-verified; a screenshot of the moment is not.
- **The bolt streak's own visual contribution is unconfirmed** (see "Judged
  on a screen" above) - not wrong, just not yet caught in a frame where the
  camera sees it side-on.
- **The muzzle's small extra particle spawn** (`FUN_08945284`) is unread and
  undrawn - see the evidence page's own new section.

## Next Steps

1. Capture a frame of the wall-hit spark itself - steer a round into a
   nearby wall inside the capture window, or extend `--give`/`--press` to
   aim rather than fire straight.
2. If the bolt streak's own contribution matters as a separate visual check,
   catch a round travelling across the frame (a wide track curve, or an
   external/side camera angle) rather than away from the camera.
3. Read `FUN_08945284` if the muzzle's extra particle spawn turns out to be
   visually significant - `Cannon_Init` and `Cannon_UpdateRound` both call
   it; neither call site's argument shapes have been read.

## From the HANDOVER.md index (moved 2026-09-25)

**2026-09-17: both hand-built quads landed.** The bolt is two crossed camera-facing ribbons between the round's last/current position, fixed white; the flash is one quad, randomly sized/rotated/faded under `0.1 s` of age, fixed white with only its **alpha** rerolled (not RGB - corrects the thread's earlier framing). Both drawn through a new `oag_fx::weapon_quads::Pipeline`, random roll seeded render-side so no determinism hash moves. Confirmed on a real disc screenshot: the flash draws on a fresh round and correctly vanishes on older ones. Open: the bolt's own contribution was not distinguishable in any capture (near-head-on foreshortening, not a code defect), no frame caught the wall-hit spark itself, and a small extra particle spawn (`FUN_08945284`) in the same window is unread.
