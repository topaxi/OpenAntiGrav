# A fired Cannon round: the body is drawn, the muzzle flash is not

2026-09-08. The thread's original premise - that the round is a
`Data\Psys\*.POB` effect whose trigger had not been found, with `Ship Muzzle`
(`0x3e2`) and `cannon_flash` (`0x3eb`) as candidates - was **wrong**, and both
candidates are dead. The Cannon names its own assets in plain strings in
`BOOT.BIN`, all three resolve in `Data.wad` on both PSP pressings, and the round
now draws as the mesh the original hangs on it. See
[`cannon-quake-leachbeam.md`](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
"What draws a Cannon round".

## What landed

- **The round has a body.** `oag_game::race::CANNON_MODEL_ENTRY` is
  `Data\Weapons\pulse_muzzleflash.vex` (`0x08a7c85c`), which
  `Cannon_Construct` (`0x088651d8`) loads into every round instance's own scene
  node. It goes through the same path the Rocket's, Mine's and Bomb's models
  already take, and `Race::cannon_model_matrices` puts one matrix on each live
  round. **The mesh is named `muzzleflash` and is the bolt**: 20 vertices, 18
  triangles, spanning `1.200 x 1.200 x 3.599` - a dart, longest along the +Z the
  matrix aims down the velocity.
  `the_discs_cannon_round_carries_its_own_model` measures both.
- **The directory layout, which the obvious guess gets wrong.** The mesh is in
  `Data\Weapons\`; the two textures are one level deeper in
  `Data\Weapons\Textures\`. `Data\Weapons\Cannon_bolt.mip` hashes to an entry
  that does not exist. Recorded on the evidence page with the hashes, entry
  numbers and the exact command, because the same split very likely holds for
  the other weapons.

## Open

- **The muzzle flash is still drawn by nothing, deliberately.** Each round
  builds two GU display lists of hand-written quads in its constructor -
  `FUN_08864cd0` draws two 4-vertex strips from `instance+0x100` and `+0x160`,
  `FUN_08864dc4` draws one from `+0x1c0` - textured from
  `Data\Weapons\Textures\Cannon_bolt.mip` and `Cannon_muzzle_flash.mip`
  (`Cannon_LoadTextures`, `0x08864b00`, into `g_cannon_bolt_texture` /
  `g_cannon_muzzle_flash_texture`). **Which list is which is unread**, so
  neither function is renamed and neither is drawn. The load order makes
  "`0x08864cd0` is the bolt" the obvious guess; it is on the page at confidence
  40 and must not be leaned on. The archive gives an independent cross-check
  nobody has spent yet: the two texture entries are **adjacent** (1057, 1058),
  so their order there should match the order `Cannon_LoadTextures` loads them.
- **The round's hit and damage path is still unread.**
  `oag_gameplay::projectile::cannon::direct_hit` applies `damage_per_bullet` off
  the schema's own shape rather than off a handler. `FUN_0886593c`
  (`0x0886593c`) is the place to start: it is what references
  `Data\Psys\WO_CANNON_SPARKS.POB` (`0x08a7c484`), the **only** `.POB` on the
  Cannon's path, and the strings `CANNONEXPLSHIP` (`0x08a7bf58`) and
  `CANNONEXPLWALL` (`0x08a7bf6c`) sit beside it. That effect *is* a `.POB` and
  `psys::Stage` can already play it once its trigger is read.
- **The per-class base speed is still chosen, not measured** -
  `func_0x00060af4` / `0x08864af4`, which
  `oag_gameplay::projectile::cannon::BASE_SPEED_KMH` stands in for.
  `Cannon_Init` calls it as `FUN_08864af4(param_2)` and adds the result to the
  caller's `speed_kmh`, so it is one decompile away.
- **Nobody has looked at this on a screen.** The mesh loads, decodes and is
  wired through a path three other weapons already use, and the tests measure
  its geometry and its axis - but there is no GUI on the build machine, so no
  frame of a round in flight has been judged the way a player would judge it.

## Next Steps

1. Look at it. A round in flight, several frames, at the size a player sees -
   the one check none of the tests above can make.
2. Read `FUN_0886593c` for the hit path and the `WO_CANNON_SPARKS` trigger.
   That is the last invisible half of this weapon and the effect mechanism is
   already generic.
3. Settle which display list is the bolt and which the flash, using the archive
   order of entries 1057/1058 as the cross-check, and then draw the flash at the
   barrel the round left from.
