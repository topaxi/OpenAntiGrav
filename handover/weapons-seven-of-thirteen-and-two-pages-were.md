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

**And a live gameplay bug came out of reviewing the wiring**, which is the one
finding here that is not about weapons at all. `Race::spend_opponent_pickup` is
an `&&`-chain and every armed weapon read `weapon == X && self.fire_x(..)`.
`fire_x` returns `false` for the ordinary "not this tick" answer - the trigger
is a *rate*, `0.05`, so it declines nineteen ticks in twenty - and a `false`
right-hand side made the condition false and fell through to the **absorb**
branch. The function runs every tick for every opponent, so a craft cashed in a
Rocket, a Missile or a Mine on the first tick it chose not to shoot: opponents
essentially never fired. **What hid it is worth carrying forward**: the comments
in that chain described the intended behaviour rather than the written one - the
Rocket's arm said "the pickup is kept rather than spent" in as many words - and
a comment that asserts the opposite of its code reads as documentation of a
decision, so nobody re-derived it. Fixed, with a test that drives a forward
weapon and a rear one against an empty field.

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
- **`WeaponStats_ParseBomb` (`0x0880cef0`) resisted a read**, and the *shape*
  of the failure is the useful part. `decompile_function` works fine on
  `0x0880d124` and `0x08862d9c`, while `inspect_memory_content` and
  `read_memory` fail on **those same addresses** - so it is not "the bridge
  cannot read `.text`", it is that the memory-read tools fail there while the
  decompiler works. The real blocker is narrower: five of the fourteen `<Stats>`
  parsers were never defined as functions in the database, `create_function`
  refuses to make one, and with the memory tools out there is no third way in.
  Anyone hitting this should try `run_analysis` over that range first rather
  than assuming the whole segment is unreadable. The Bomb's parser *name* is
  settled at 90 off the dispatch chain either way; its offsets are not, and
  nothing needs them.
- **The AI dodges laid charges but nothing else.** `oag_ai::Hazard` reaches a
  driver through `Field`, and `Driver::avoidance` leans away from the nearest
  one - but only mines and bombs are reported. A rocket or a missile in flight
  still cannot be seen or avoided, which is defensible (by the time a driver
  could react to one it has arrived) and is a choice rather than an oversight.
- **A driver notices a charge with no reaction latency at all.** `Reflex`'s
  clock is keyed on *which craft* is in a channel and a charge has no identity
  to track, so `Reflex::filter` passes the hazard straight through. The
  avoidance ramp starting gently at the far edge of its lookahead is what makes
  that look gradual rather than clairvoyant.
- **What a driver in the original does about a laid charge is unread.**
  `WeaponAi_Update` (`0x08851550`) is where it would live and only its authored
  table has been read. Everything in `driver/avoidance.rs` is this project's,
  and the reason it exists is fairness rather than fidelity - a charge the
  player can steer around and an opponent cannot is a worse wrong answer than a
  dodge nobody has verified.
- **A mine and a bomb both draw no model.** `Pulse_Mine.vex` and
  `Pulse_Bomb.vex` are both named and located and no renderer reads either.
  They were not, however, invisible - see the fixed bug below, which is what
  a maintainer who plays Pulse was actually seeing.

**Fixed 2026-08-26: a laid mine or bomb rode the Rocket's flare from the
moment it landed.** Reported from play as "the mines are animating the
explosion when dropped, instead of when detonating (only tested via
pulse)". `Race::advance_projectile_flares` (`crates/game/src/race/weapons.rs`)
gated on `projectile.kind.is_none()` - true for *any* live projectile - to
decide who rides a [`ROCKET_FLARE_EFFECT`] (`WO_ROCKET_FLARE`) instance. That
effect's emitters are looping (see `mine.md`'s neighbour reading on
`Rocket_Init`), so a mine or a bomb got it attached the instant
[`Race::lay_mines`] placed it and kept it burning at that fixed point for the
charge's whole life - which reads exactly like an explosion that started at
the drop and never really ended. Invisible to `just test`: the headless
harness builds `Race` with an empty `psys::Library`
(`crates/game/src/race/tests.rs`), so `self.effects.get(ROCKET_FLARE_EFFECT)`
is always `None` there and the attach never fires - only `just play` against
the real disc loads the asset. `docs/formats/pob.md` already listed
`WO_MISSILE_HEAD` as the Missile's own, separate effect, which is what said
this gate was wrong rather than merely coincidental. Fix: gate on
`kind == Some(Weapon::Rocket)` instead. A Mine and a Bomb now ride nothing,
which is the correct "not implemented" state.

**And wired the same day: the Missile rides its own `WO_MISSILE_HEAD`.**
Asked directly - "does the Missile have its own flare in Pulse, I know HD/
Fury/2048 do" - and it does: `WO_MISSILE_HEAD` is in the PSP disc's own
35-file name-hash list (`docs/formats/pob.md`), not an HD-only asset, and
`missile.md`'s reading of `Missile_Init` (`0x0885a160`, confidence 90) already
had it - "plays `WO_MISSILE_HEAD` at two anchors" is one of the three
independent things that settles bit `0x40` as the Missile. The gate above
generalised to `flare_effect_for(kind) -> Option<&'static str>` (Rocket ->
`WO_ROCKET_FLARE`, Missile -> `WO_MISSILE_HEAD`, everything else -> `None`),
unit tested the same way. **One simplification, stated rather than
discovered later**: the original attaches *two* instances, one per anchor,
and this engine rides *one*, centred on the projectile's own position - the
Missile carries no located model or locator set here the way the Rocket's
`Rocket.vex` does, so there is nowhere read to put a second one. The
`WO_MISSILE_EXPLO` and `WO_MISSILE_BOUNCE` reasons in
`crates/game/tests/psys_inventory_ground_truth.rs` were also stale ("the
Missile fires nothing yet" - it does) and are corrected to name the real gap:
`Race::ignite_blast` reuses the Rocket's own explosion pair for every
detonating projectile rather than reading which file each weapon authors.

## Next Steps

- **Draw the two rear weapons.** `Data\Weapons\Pulse_Mine.vex` and
  `Data\Weapons\Pulse_Bomb.vex` through the existing `.vex` path, plus their
  own cues (`MINELAUNCH`/`MINERADAR`, `BOMBLAUNCH`/`~BOMBRADAR`,
  `BOMBEXPL`/`BOMBEXPL_PC`) - all six strings are located. This is now the
  biggest gap on both weapons: they work and draw no model. **Read
  `Mine_Init`/`Mine_Construct` for a drop-time effect before wiring one** -
  the fixed bug above is evidence there may be a real, smaller one (an arming
  flash, a muzzle puff) rather than none at all; do not reuse `ignite_blast`'s
  detonation effect at lay time on a guess.
- ~~**Give the Missile its own detonation and bounce effects.**~~ **Done
  2026-08-26.** `Race::blast_for` now takes `kind` and maps `Missile` to
  `MISSILE_EXPLO_EFFECT` (`WO_MISSILE_EXPLO`, one file whatever it hit -
  `Missile_SpawnExplosion`, `0x08868d50`); `Race::ignite_missile_bounces`
  reads a before/after snapshot of `Projectile::bounces` (a bounce never
  reaches `Impact`, so there is nothing else to hang the trigger off) and
  plays `WO_MISSILE_BOUNCE` on every wall glance, per `missile.md`'s
  `Missile_Update` reading.
- **The Mine got the same fix, and further - its own detonation is now
  recovered, not just correctly silent.** `blast_for(Mine, ...)` was `None`
  for about an hour of this session (drawing nothing, replacing the
  Rocket-reuse bug), then a live Ghidra session found the actual spawn call:
  **`Mine_SpawnExplosion`** (`0x08867f1c`, confidence 90) is called from
  `FUN_08867370`'s uniform teardown, reached from both the fuse timeout and
  `FUN_08867b50`'s trigger_radius sweep, and plays `WO_MINE_EXPLO` - confirmed
  by a direct memory read of the string, not inferred from a plausible name.
  See [mine.md](../docs/ghidra/functions/psp-pulse-usa/mine.md#mine_spawnexplosion-plays-wo_mine_explo)
  for the full read, including the finding that fourcc `MIEX` is a shared
  "this is an explosion" tag rather than the Missile's own label -
  `Missile_SpawnExplosion` uses the identical tag for `WO_MISSILE_EXPLO`.
  `blast_for(Mine, ...)` now returns `MINE_EXPLO_EFFECT`.
- **The Bomb is still open, and now the odd one out.** Every other rear/guided
  weapon that fires has a recovered detonation effect; the Bomb's own teardown
  is a distinct function from the Mine's (separate pool cursor, `+0xc4`/cap 32
  against `+0x164`/`+0x64`) and has not been read. `mine.md`'s new section
  names the two candidates - `WO_MINE_EXPLO` reused at a larger scale (which
  would match "the bomb should be static, like the mines, just a single big
  mine" extended to the visual) or its own `WO_BOMB_SMOKERING`. `FUN_08867370`
  is the template to read next: find the Bomb-pool equivalent (likely nearby,
  structurally similar) and see what it calls at teardown.
- **The Missile's flare rides one instance where the original rides two.**
  `Missile_Init` plays `WO_MISSILE_HEAD` at two anchors; this engine centres
  one on the projectile's position for lack of a located missile model. Fine
  as a stated approximation, worth revisiting once the Missile has a drawn
  model with locators to read anchors off.
- **A trap worth carrying forward: `get_function_callers` missed
  `Mine_SpawnExplosion`'s only call site entirely** ("No callers found"),
  because the call is through the same image-base-relative-looking operand
  `weapon-fire.md` already documents for `jal` targets in this binary -
  `func_0x00063f1c(...)` decompiles as a bare offset rather than a resolved
  call, and only reading the caller's decompilation by hand turned up the real
  target (`0x63f1c + 0x08804000 = 0x08867f1c`). Static xref search is not
  enough to prove "nothing calls this" in this binary; read the caller by
  hand first. The same trick (adding the image base back to a small
  decompiler-printed constant, then a direct memory read to confirm) also
  resolved two "unreadable" string-pointer arguments to `Psys_Spawn_q` that
  looked like implausible addresses (`0x278904`, `0x278970`) until corrected.
- ~~**Teach the AI to see what is on the track.**~~ **Done 2026-08-26**, and
  the measurement is the point: over a minute of ordinary racing the field's
  *mean* energy barely moved (76 to 86 of 95) while the **worst craft went from
  34 to 75**. The damage was never spread evenly - it was concentrated on
  whichever craft sat behind a driver with a Mine, eating cluster after cluster
  on a line it could not see. Mines still land; one craft is no longer singled
  out for a punishment it had no way to avoid.
- **The Cannon is the odd one left and is worth a session on its own.** Its fire
  bit `0x2000` is set by `Weapon_RequestFire` and dispatched by **nothing** in
  `Weapons_DispatchFire`. Bit `0x4000` on `world+0x58` (`0x088537ac`) is the
  obvious candidate for where it actually fires and was not chased. It is also
  the only weapon authoring `rounds` and `rate`, which is what made the Mine's
  handler look like it for months.
- Measure `CLUSTER` against the running original and retire the invented number.
- **Re-read the five undefined `<Stats>` parsers** once the Ghidra bridge can
  read `.text` again - see Open.
