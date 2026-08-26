# Weapons: six of thirteen, and two doc pages had two of them swapped

2026-08-26, [mine.md](../docs/ghidra/functions/psp-pulse-usa/mine.md) and
[pickups.md](../docs/gameplay/pickups.md) - the pages to read, not this row.
Supersedes the four-of-thirteen thread. **Turbo, Shield, Rocket, Missile,
Autopilot and Mine** now do something.

**The Mine is the interesting one, and not because it was hard.** Its blocker
was that the project had it filed under the wrong function. `weapon-fire.md`
had the bit-`0x2` staggered-burst handler down as the **Cannon** at confidence
72, on the grounds that `rounds` and `rate` are the Cannon's `<Stats>` and
nobody else's; `contact-response.md` had the bit-`0x100` backwards spawn down as
the **Mine**; and `ai-stats.md` had noticed the two could not both be right and
recorded the conflict open, naming `FUN_08871ddc` as the thing that would
resolve it. Reading that function resolved it, and both attributions were wrong.

**The mechanical cause is worth carrying forward because it will bite again.**
`Weapon_RequestFire` (`0x08862d9c`) is not a comparison chain - it is an
indirect jump through a thirteen-entry table at `0x08a7c710`, and **entries 8
and 9 are out of address order** (`0x0005eef8` then `0x0005eed4`). Reading the
case bodies down the listing therefore transposes exactly the two weapons that
drop something out of the back. Read that table as a table.

**What actually settled it needed no ordering argument at all**: `Mine_Init`
(`0x08859ac8`), the spawn the bit-`0x2` handler calls, plays `MINELAUNCH` and
`MINERADAR`, and its constructor `Mine_Construct` (`0x08859930`) loads
`Data\Weapons\Pulse_Mine.vex`. The Bomb's handler sits in the group that loads
`Pulse_Bomb.vex`. A maintainer who plays the game said the same thing when
asked cold - mines are several, the bomb is one bigger one - which is the
second time on this subsystem that from-play knowledge has settled a decompiler
question.

**Three weapon enumerations, and they are not the same list.** This is the trap
the whole session was about. (1) The **pool order** at `0x08a78c00`, which is
also the `<Stats>` struct's own layout - verified by walking Mine's `+0xe8`
block back to the Missile's `+0x5c` with no gap. (2) The **shipped file's
element order**, which puts `Turbo` and `Shield` before `Cannon`; both
`WeaponAiStats_Load`'s thirteen explicit literals and the incoming-weapon
announcement table index by it, and neither is the weapon id. (3) The **weapon
id** at `craft+0x1bc`, which matches the pool at eleven of thirteen and reaches
the Mine at 8 and the Bomb at 9. `ai-stats.md`'s correction that "`Weapon::ALL`
is the one that is wrong about ids" is itself now corrected: it is right about
eleven of them, and the *AI stats file* is the thing indexed differently.

**Ported the same day**: `MineStats` (seven attributes, `WeaponStats_ParseMine`
`0x0880d124`, 92); `oag_gameplay::projectile::mine`; a drop that lays
`CLUSTER` mines one every recovered `0.1 s`; the recovered coupling that the
craft **keeps holding the pickup** until the last one is out, which is why
`pickup::Held` grew `dropping` and `drop_reload`; a mine that does not fly at
all and takes an early return at the top of `Projectiles::advance`; and a
behind-gated `Driver::wants_to_drop` for opponents, which is the forward
weapon's rule with the cone, the curvature and the minimum range taken out.

**Verified**: `just` green at **2,472**, and three disc-backed tests in the new
`crates/game/tests/mine_ground_truth.rs`. Measured on `pulse-psp-usa.chd`: five
mines over 111.5 units of track at 100 units/s, the first 4.87 units behind the
craft, a rival on one taking exactly the authored 5.0 damage inside the
authored 3.0 trigger radius. The world hash moved once and it is the **stream,
not the trajectory** - the two new `Held` fields are zero through both reference
scenarios, and with only their writes removed all eight previous constants
reproduce bit for bit.

**Two files were split rather than baselined** (`projectile/rocket.rs`,
`driver/weapons.rs`), and the second surfaced a pre-existing bug: `Driver::drift`'s
doc comment was stranded 270 lines above it, attached to `Driver::wants_to_fire`.
Put back.

## Open

- **How many mines a press lays is invented.** `oag_gameplay::projectile::mine::CLUSTER`
  is `5`. Two independent sweeps for what writes the original's counter at
  `craft+0x1ac` found only the handler's own decrement, and no `<Stats>`
  attribute counts mines. It is simulation state, so changing it is a hash move.
- **That a mine does not move once laid** is the conservative reading, not a
  read. `Mine_Init` copies the firing craft's velocity into the entity at
  `+0xa0` and nothing found integrates it.
- **That `trigger_radius` is what trips a mine** is this engine's reading of an
  authored attribute; the code path that spends it was not found.
- **`FUN_08867370` is deliberately unnamed.** It walks a pool with the exact
  `+0x164` cursor and `+0x64` base `Weapon_DropMines` uses, decrements the same
  `+0x48` the Mine's fuse lives at, and blast-sweeps on expiry - so "it is
  `Mines_Update`, and a mine detonates when its fuse runs out" is a strong
  hypothesis. It is not named because **no caller resolves** for it under either
  address rendering, and because `contact-response.md` records a live breakpoint
  reading its `+0x164` as `1` at race load with no weapon fired, which a mine
  count should not be. Under 50 by the rubric, so no rename.
- **Nothing dispatches bit `0x2000`**, the Cannon's own fire bit, anywhere in
  `Weapons_DispatchFire`. Where the Cannon actually fires is unread; bit
  `0x4000` on `world+0x58` (`0x088537ac`) is the obvious candidate and was not
  chased.
- Seven weapons are still unbuilt: Quake needs track deformation, LeachBeam a
  beam, and most of the rest the slowdown mechanic behind
  `<Global slowdown_limit>`, whose consumer (`weapon_record+0x130`) is known and
  unspent.
- **A mine draws nothing.** `Pulse_Mine.vex` is named and located and no
  renderer reads it, so a laid cluster is invisible.

## Next Steps

- **The Bomb is the obvious next weapon** and a maintainer has asked for it: one
  bigger charge out of the same rear anchor, bigger blast, longer `timetodie`,
  wider `trigger_radius`, all authored and all parsed the same way `MineStats`
  was. `Weapon_FireBomb` (`0x08863a20`) is read end to end in
  `contact-response.md` - the negated-forward spawn, the pool cursor, the
  owning-craft index. Most of `projectile::mine` is reusable as-is; what differs
  is one spawn instead of a cluster and a velocity that is *not* zero, since the
  Bomb is spawned along a direction the handler explicitly negates.
- Measure `CLUSTER` against the running original and retire the invented number.
- Draw a mine: `Data\Weapons\Pulse_Mine.vex` through the existing `.vex` path.
