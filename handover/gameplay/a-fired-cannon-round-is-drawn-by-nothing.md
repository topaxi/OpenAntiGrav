# A fired Cannon round: the body and its impact are drawn; the two hand-built quads are not

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

- **The round has a body.** `oag_game::race::CANNON_MODEL_ENTRY` is
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
  `oag_gameplay::projectile::cannon::direct_hit`'s existing shape rather than
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
--press square --ticks 340 --screenshot /tmp/out.png` (headless
`--screenshot` renders without a display, no Xvfb needed) shows the round's
own dart-shaped mesh in flight ahead of the player's craft at ticks 320, 340
and 400 of a fresh single-race start - visually distinct from the ship's own
exhaust glow. This is the check none of the geometry/axis tests could make.
No frame catching the wall-impact spark itself was captured this pass - a
straight-held throttle rarely steers a round into a wall inside a short
capture window - so the spark's *code* path is verified
(`a_cannon_round_sparks_on_a_wall_and_silently_on_a_craft`) but not yet its
own frame.

## Open

- **The two hand-built display lists are still drawn by nothing**, and that
  remains a deliberate, honest absence rather than a regression: both are raw
  GU quads textured from `Cannon_bolt.mip`/`Cannon_muzzle_flash.mip`, not
  `Data\Psys` effects, and drawing them needs a textured-billboard path this
  project's mesh/texture-upload machinery does not yet expose outside
  `oag_render::mesh_render` - out of a Cannon-focused pass's own file
  ownership when another member holds `mesh/`. See
  `oag_game::race::cannon_model_matrices`'s doc comment for the exact
  addresses and offsets a `mesh_render`-owning pass would need.
- **No frame has caught the wall-hit spark itself**, only the round in
  flight. The trigger is code-verified; a screenshot of the moment is not.

## Next Steps

1. Build a textured-billboard draw path (needs `mesh_render`-adjacent
   ownership) for `Cannon_BuildBoltList`'s streak and
   `Cannon_BuildMuzzleFlashList`'s randomly sized/coloured flash, bound to
   `Cannon_bolt.mip` (`Data.wad` entry 1057) and `Cannon_muzzle_flash.mip`
   (entry 1058) respectively, gated the way `Cannon_DrawRound` gates them
   (bolt every frame, flash only under `+0xc8 < 0.1` seconds).
2. Capture a frame of the wall-hit spark itself - steer a round into a
   nearby wall inside the capture window, or extend `--give`/`--press` to
   aim rather than fire straight.
