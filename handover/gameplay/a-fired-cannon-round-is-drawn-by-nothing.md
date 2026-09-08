# A fired Cannon round is drawn by nothing

2026-09-07. The Cannon's *firing* is fixed and landed - see
[`cannon-quake-leachbeam.md`](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).
Holding fire now puts rounds in the air at the authored rate, and a headless
capture counts them: `--race --mode single_race --give Cannon --hold cross,square`
over 420 ticks has a round in the air on **335** of them, up to six at once,
against **0** with fire not held. As of 2026-09-08 opponents fire too - see
[the AI Cannon thread](the-ai-cannons-gate-is-an-uninitialised-byte.md).

**Nothing draws any of them.** `crates/game/src/race/weapons/visuals.rs` has no
Cannon arm at all, so the screenshots from that capture show a race with no
round in the picture. A player would still say the weapon does nothing: it is
now firing, and it is invisible. **This has not changed**, and drawing it is
still the whole of this thread.

## What changed 2026-09-08: the assets are located, and they are not a particle effect

The premise this thread carried - that the round is a `Data\Psys\*.POB` effect
whose trigger had not been found, with `Ship Muzzle` (`0x3e2`) and
`cannon_flash` (`0x3eb`) as candidates - is **wrong**. The Cannon names its own
assets in plain strings in `BOOT.BIN`, and the page above now records the read
in full ("What draws a Cannon round"). In short:

- `Data\Weapons\pulse_muzzleflash.vex` (`0x08a7c85c`), loaded per round instance
  by `Cannon_Construct` (`0x088651d8`) into that instance's node at `+0xc0`.
- `Data\Weapons\Textures\Cannon_bolt.mip` (`0x08a7c804`) and
  `Data\Weapons\Textures\Cannon_muzzle_flash.mip` (`0x08a7c82c`), both loaded by
  `Cannon_LoadTextures` (`0x08864b00`) into `g_cannon_bolt_texture`
  (`0x08b3bf80`) and `g_cannon_muzzle_flash_texture` (`0x08b3bf84`).
- Each round builds **two GU display lists in its own constructor** and replays
  them per frame: `FUN_08864cd0` draws two 4-vertex triangle strips from
  `instance+0x100` and `instance+0x160`, `FUN_08864dc4` draws one from
  `instance+0x1c0`. Same state on both - depth write on, func 6,
  `Gu_BlendFunc(0, 2, 10, 0, 0xffffff)`, vertex format `0x19f`.
- The **only** `.POB` on the Cannon's path is `Data\Psys\WO_CANNON_SPARKS.POB`
  (`0x08a7c484`), and it is the **impact**, referenced from `FUN_0886593c`
  (`0x0886593c`).

So `oag_render::psys::Library` is the wrong mechanism for the round. What this
needs is the `.vex` model and the two `.mip` textures out of `Data.wad`, drawn
the way `Race::rocket_model_matrices` already draws `Pulse_Rocket.vex`.

## Open

- **Nothing is drawn and nothing invented stands in**, deliberately, per
  CLAUDE.md. The rounds stay invisible until the assets are located in the WAD
  and wired. Do not add a billboard, a flare or a psys effect to fill the gap:
  the last two times a stand-in went in for a weapon it had to be reverted.
- **The WAD entries are not located yet.** `PSP_GAME/USRDIR/Data.wad` is
  hash-keyed - all 1142 entries list with an empty name - so the three assets
  have to be found by hashing their executable-side names. The exact commands
  are `just wad hash 'Data\Weapons\pulse_muzzleflash.vex'` and the same for
  `Data\Weapons\Textures\Cannon_bolt.mip` and
  `Data\Weapons\Textures\Cannon_muzzle_flash.mip`, then matching the hashes
  against `just wad list 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad'`.
  (Attempted 2026-09-08 and blocked by a sandbox rule on the subcommand name,
  not by anything about the data - the orchestrating session can run it
  directly.)
- **Which display list is the bolt and which is the muzzle flash is unread**,
  so neither `FUN_08864cd0` nor `FUN_08864dc4` is renamed. The load order makes
  "`0x08864cd0` is the bolt" the obvious guess; it is written down on the page
  at confidence 40 and must not be leaned on.
- **The round's hit and damage path is still unread.** No `Cannon_HitCraft`-style
  function has been located, and `oag_gameplay::projectile::cannon::direct_hit`
  applies `damage_per_bullet` off the schema's own shape rather than off a
  handler. `FUN_0886593c` (`0x0886593c`) is now the obvious place to start: it is
  what references `WO_CANNON_SPARKS`, and the strings `CANNONEXPLSHIP`
  (`0x08a7bf58`) and `CANNONEXPLWALL` (`0x08a7bf6c`) sit beside it.
- **The per-class base speed is still chosen, not measured** -
  `func_0x00060af4` / `0x08864af4`, which
  `oag_gameplay::projectile::cannon::BASE_SPEED_KMH` stands in for. `Cannon_Init`
  calls it as `FUN_08864af4(param_2)` and adds the result to the caller's
  `speed_kmh`, so it is one decompile away.

## Next Steps

1. Hash the three asset names and confirm they are present in `Data.wad` (one
   command each, above). If a name is absent, the executable-side spelling is
   wrong and that is worth knowing before any renderer work.
2. Extract `pulse_muzzleflash.vex` and view it with `just view --mesh` - that
   settles what the three quads actually are without reading any more assembly.
3. Add a Cannon arm to `crates/game/src/race/weapons/visuals.rs` alongside
   `rocket_model_matrices`, playing the real mesh and the real textures. Judge it
   as a player would: several frames, at the size a player sees, with a round
   actually in flight.
4. Read `FUN_0886593c` for the hit path and the `WO_CANNON_SPARKS` trigger. That
   effect *is* a `.POB` and `psys::Stage` can already play it once its trigger is
   read.
