# A fired Cannon round is drawn by nothing

2026-09-07. The Cannon's *firing* is fixed and landed - see
[`cannon-quake-leachbeam.md`](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).
Holding fire now puts rounds in the air at the authored rate, and a headless
capture counts them: `--race --mode single_race --give Cannon --hold cross,square`
over 420 ticks has a round in the air on **335** of them, up to six at once,
against **0** with fire not held.

**Nothing draws any of them.** `crates/game/src/race/weapons/visuals.rs` has no
Cannon arm at all, so the screenshots from that capture show a race with no
round in the picture. A player would still say the weapon does nothing: it is
now firing, and it is invisible.

## Open

- **The round has no model and no effect.** Every other projectile this project
  fires has at least a flare. The Cannon has neither, and `vex.md`'s class table
  names two candidates that are almost certainly its own -
  `Ship Muzzle` `0x3e2` and `cannon_flash` `0x3eb` - neither of which has a
  recovered trigger. `Weapon_FireCannon` (`0x088577ac`) picks between two
  emitter anchors on the firing craft's entity (`+0x64`/`+0x68`), which is
  presumably where a muzzle flash is anchored, but that is an inference and not
  a read.
- **Whether an AI should be able to fire one is a maintainer question**, not an
  RE one. The original gives an opponent no way to: `Cannon_UpdateReload` gates
  on the *fire-held* byte `+0x16` of the craft's control record, and
  `WeaponAi_Update` (`0x08851550`) writes that record's `+0x15` and `+0x17` and
  never `+0x16` - an exhaustive operand scan finds zero byte stores at `+0x16`
  in the whole image. `Race::advance_cannons` copies that and fires for slot 0
  only. The barrel roll got an explicit ruling to deviate on 2026-09-06; this
  has not.
- **The round's hit and damage path is still unread.** No `Cannon_HitCraft`-
  style function was located, and `oag_gameplay::projectile::cannon::direct_hit`
  applies `damage_per_bullet` off the schema's own shape rather than off a
  handler.

## Next Steps

- Find what `Weapon_FireCannon`'s two emitter anchors feed, and whether
  `cannon_flash` (`0x3eb`) is bound to them. A trigger read is what this needs;
  an invented muzzle flash is exactly what `CLAUDE.md`'s "never invent what the
  assets already author" rule forbids.
- Ask the maintainer whether an AI opponent should fire a Cannon. If yes, the
  gate in `Race::advance_cannons` becomes `oag_ai::Driver::wants_to_fire` for
  slots 1..8, the same shape the Rocket and the Plasma already use.
- Chase the per-class base speed a round carries (`func_0x00060af4`,
  `0x08864af4`), which `oag_gameplay::projectile::cannon::BASE_SPEED_KMH` still
  marks as chosen rather than measured.
