# Weapons: seven of thirteen, and two doc pages had two of them swapped

2026-08-26, [mine.md](../docs/ghidra/functions/psp-pulse-usa/mine.md) and
[pickups.md](../docs/gameplay/pickups.md) - the pages to read, not this row.
Supersedes the four-of-thirteen thread. **Turbo, Shield, Rocket, Missile,
Autopilot, Mine and Bomb** now do something.

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

**The Bomb followed the Mine the same day and cost almost nothing**, which is
the point of recording it here rather than in a thread of its own: it shares
every line of `oag_gameplay::projectile::mine` except a count. Three
independent things say it is the Mine one size up - its `<Stats>` are the
Mine's six with every one larger on both shipped tables; `Weapon_FireBomb`
(`0x08863a20`) spawns once where `Weapon_DropMines` reloads a timer and spawns
again; and a maintainer who plays Pulse said so when asked, twice, unprompted
("a single big mine"). Measured on the disc: 15 damage against the Mine's 5, a
20-second fuse against 7, and it does not move - 0.000 units of drift while the
craft that laid it drove 118 away.

**And a third weapon's worth of work fell out of the Mine on the way**, which is
the finding most likely to matter to whoever reads this next.
`Weapon_PostBlastImpulse` (`0x0886794c`) had been read at instruction level a
week earlier and its `stats` pointer left unidentified - the thing holding it at
confidence 68. It is the **Mine's**: the four offsets it spends (`+0xe8`,
`+0xec`, `+0xf0`, `+0xfc`) are exactly `damage`, `blastradius`, `blastforce` and
`slowdown_time` in the block `WeaponStats_ParseMine` writes, in four matching
roles. The cross-check that makes it more than four numbers lining up is the
fourth: `+0x130`, the accumulator `+0xfc` feeds, was recorded across this tree
as `<Global slowdown_limit>`'s consumer *before* anyone knew `+0xfc` held
`slowdown_time` - the two identifications were made from opposite ends and met.
**So a blast's impulse falls off linearly (`1.0 - d/blastradius`) and its damage
does not**, and `projectile::blast` had both flat. Half of an invented rule
replaced by the original's, at the cost of one trajectory hash move.

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
- Six weapons are still unbuilt: Quake needs track deformation, LeachBeam a
  beam, and the remaining four - Cannon, Plasma, Repulser, Shuriken - need
  either the slowdown mechanic or a fire path nothing has read.
- **The Bomb's `damageradius` is authored and spent nowhere.** It is the only
  second radius any weapon has, and the one blast path read at instruction level
  spends `blastradius` for both damage and impulse. Left undecoded rather than
  wired on a guess about which half it governs.
- **`WeaponStats_ParseBomb` (`0x0880cef0`) resisted a read.** It is not a
  defined function in the Ghidra database, `create_function` refuses it, and the
  bridge cannot read bytes anywhere in `.text` (it reads `.rodata` normally).
  Its *name* is settled at 90 off the dispatch chain; its offsets are not, and
  nothing needs them. Whoever fixes the bridge should re-read that region -
  five of the fourteen `<Stats>` parsers are in the same state.
- **A mine and a bomb both draw nothing.** `Pulse_Mine.vex` and
  `Pulse_Bomb.vex` are both named and located and no renderer reads either, so a
  laid charge is invisible - which for a weapon whose whole point is that a
  rival does not see it coming is arguably fine, and for the player who laid it
  is not.

## Next Steps

- **Draw the two rear weapons.** `Data\Weapons\Pulse_Mine.vex` and
  `Data\Weapons\Pulse_Bomb.vex` through the existing `.vex` path, plus their
  own cues (`MINELAUNCH`/`MINERADAR`, `BOMBLAUNCH`/`~BOMBRADAR`,
  `BOMBEXPL`/`BOMBEXPL_PC`) - all six strings are located. This is now the
  biggest gap on both weapons: they work and are invisible.
- **The Cannon is the odd one left and is worth a session on its own.** Its fire
  bit `0x2000` is set by `Weapon_RequestFire` and dispatched by **nothing** in
  `Weapons_DispatchFire`. Bit `0x4000` on `world+0x58` (`0x088537ac`) is the
  obvious candidate for where it actually fires and was not chased. It is also
  the only weapon authoring `rounds` and `rate`, which is what made the Mine's
  handler look like it for months.
- Measure `CLUSTER` against the running original and retire the invented number.
- **Re-read the five undefined `<Stats>` parsers** once the Ghidra bridge can
  read `.text` again - see Open.
