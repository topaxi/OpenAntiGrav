# Weapons: nine of thirteen, and the dispatch table read whole

2026-09-02, [plasma.md](../../docs/ghidra/functions/psp-pulse-usa/plasma.md),
[mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md) and
[pickups.md](../../docs/gameplay/pickups.md) - the pages to read, not this row.
Supersedes the seven-of-thirteen thread, which superseded the four.
**Turbo, Shield, Rocket, Missile, Autopilot, Mine, Bomb, Plasma and Shuriken**
now do something.

## 2026-09-02, later: the Shuriken, and an estimate that was wrong by a lot

**The finding is the estimate, not the weapon.** Judged from its fire handler,
its constructor and its bounce - four functions - the Shuriken looked like a
session of its own: a reflection unlike the Missile's, two damage numbers where
`blast_stats` returns one triple, a fuse, and a seeded coin flip. That
assessment was written into a `docs/` page and committed. Then
`Shuriken_Update` (`0x08877bdc`) - **the one function not read** - turned out to
be the *same floor follower* the Rocket and the Plasma already share, differing
only in that a wall calls `Shuriken_Bounce` where a rocket detonates. The
weapon cost about what the Plasma did.

**So: the trajectory is the expensive part of a weapon in this engine, and the
trajectory is the one thing you cannot infer.** It was inferred here from the
entity layout differing from the Plasma's (`+0x130` against `+0xf0`) and from
the bounce being unlike the Missile's, and both of those were true and neither
implied what it looked like it implied. The page carries the correction rather
than hiding it.

**A port bug came out of the same read**, and it had shipped hours earlier:
`Plasma_Update` probes `12.0` along its carried normal and this engine gave the
Plasma the *Rocket's* `6.0`. Noticing the constant a third time in
`Shuriken_Update` is what caught it. `SURFACE_PROBE_LENGTH` is now per-kind and
the comment says what it should have said all along - **the Rocket is the odd
one out at 6.0**; the Missile, the Plasma and the Shuriken all read 12.0, each
off its own function.

**What the Shuriken turned out to be**: one blade, thrown at a coin-flipped
`±0.349066` rad - **20.000 degrees**, a literal and its exact negation -
carrying the throwing craft's own speed in km/h on top of the class speed,
riding `WO_SHURIKEN_HEAD`, reflecting perfectly off walls (`v - 2(v.n)n`, no
damping, no bounce budget, pushed `0.1` out along the normal) with
`WO_SHURIKEN_BOUNCE` on each, until its authored `fuse` runs out. **Measured on
`pulse-psp-usa.chd`: a blade lived 119 of its 120 fuse ticks and bounced three
times off Talon's Junction.**

**The coin is drawn from `World::rng` and is hashed state**, and the ordering is
the original's: `Weapon_FireShuriken` draws *inside* its pool-space check, so
both fire paths here check for a free slot **before** drawing. Drawing and
discarding would advance the generator on a tick the original does not.

## 2026-09-02: the Plasma, and two by-catches worth more than the weapon

**The Plasma cost almost nothing and that is the finding, not an aside.** The
Bomb was cheap because it reused the Mine's whole module; the Plasma is cheap
because it reuses the **Rocket's** whole flight model. `Plasma_Update`
(`0x0885c6cc`) is `Rocket_Update`'s floor follower instruction for instruction -
the same 12-unit probe along the carried surface normal, the same
speed-preserving redirect, the same fall, the same detonate on a wall, the same
`/ 3.6`. `oag_gameplay::projectile::advance` already *was* that function, so the
port needed no flight code at all: a `PlasmaStats`, a one-shot `launch`, two
fire arms and a flare name. **The pattern to carry forward is that the
expensive part of a weapon in this engine is its trigger and its trajectory,
and a weapon that shares a trajectory with one already built is nearly free.**
That is worth checking for the Shuriken before assuming its ricochet is new
work - the Missile already bounces.

**By-catch 1: `Weapons_DispatchFire`'s full sixteen bits.** `weapon-fire.md`
printed five and truncated the rest with a `...` for three months. Reading all
sixteen and adding the image base back settles five weapons' handlers at once,
and **six of the rows are cross-checks against pages that already had the
answer - all six pass**. That is what makes the new rows trustworthy, and it is
also what caught this session's own arithmetic slip: the first draft of that
table added `0x08804000` without carrying, so *every* handler address was
wrong, and only the cross-checkable rows announced it. **Write the rows you can
check down first; a table of only-new addresses would have shipped.**

**By-catch 2: all fourteen `<Stats>` parsers, and a retired blocker.**
`WeaponStats_Parse`'s dispatch chain plus the type-string run at `0x08a78bf4`
names every parser in one read, four of them cross-checking against known
addresses. And `mine.md`'s "five of the fourteen parsers were never defined as
functions, `create_function` refuses, and the bridge cannot read `.text`" is
**no longer true** - `decompile_function` works on `0x0880cc2c` directly and
`inspect_memory_content` reads `.rodata` normally. Whatever that blocker was, the
route through the decompiler plus a `.rodata` string read is open, and it is how
the whole page was read.

**And the block landed exactly where arithmetic said it would.** `mine.md`
derived the layout of the `0xa4` unread bytes between the Missile's block and
the Mine's from nothing but attribute counts, putting the Plasma at
`+0x9c`..`+0xc4`. Measuring it there makes it the third checked anchor in that
run, after Turbo's `+0x84` and Shield's `+0x8c`. The arithmetic can now be
trusted for the Quake's four and the Cannon's five as well.

**Two files split rather than baselined** (`crates/formats/src/weapons.rs`,
`crates/game/src/race/field.rs`), and the second surfaced the *same* fault the
last split did: a stranded doc comment. `fire_opponent_rocket`'s paragraphs sat
above `fire_opponent_missile` and the Rocket carried none, exactly as
`Driver::drift`'s did. **Moving code is how these are found**, which is an
argument for splitting a file at the ceiling rather than baselining it.

**And two pre-existing ground-truth reds fixed on the way**, both in
`mine_ground_truth.rs`: their 120-tick warm-up went *inside* the measured
272-tick start-line countdown the day `b4bb23ee` landed, so the craft was
stationary and every "a moving craft" assertion failed for a reason unrelated to
mines. Both files now warm up through `oag_race::COUNTDOWN_TICKS`. **Worth
carrying forward: a landed countdown silently invalidates every disc-backed
test's warm-up premise, and those tests never run in CI.** Grep for a bare tick
count in `crates/game/tests/` before trusting any of them.

**Measured on `pulse-psp-usa.chd`**: one press is one bolt, it leaves ahead of
the craft at the disc's own Venom figure converted from km/h, and it covered
291.5 units of Talon's Junction still flying. `just` green at 2,765, and
**`just test-data` green at 3,342 of 3,342** - the first time the whole
disc-backed suite has been.

**The `<Stats>` struct's unread middle is now four-fifths closed.** `mine.md`
derived the whole `0xa4`-byte run between the Missile's block and the Mine's
from attribute counts alone; this session measured the Plasma's (`+0x9c`), the
Shuriken's (`+0x144`) and confirmed the Repulser's (`+0x128`) from a consumer.
With Turbo's and Shield's already measured that is five anchors, and only the
Quake's four (`+0x60`) and the Cannon's five (`+0x70`) are left - the two
weapons nobody has needed a parser for.

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

## 2026-09-07: the Cannon, the Quake and the LeachBeam checked and still blocked; the Mine's and the Bomb's pose built instead

**Checked all three of the remaining unbuilt weapons against the committed
docs alone, with no Ghidra session, per this pass's own constraint - and all
three are still genuinely blocked, not merely under-read.** `plasma.md`'s
sixteen-bit table (2026-09-02) already says so and nothing has been added to
it since: the Cannon's fire bit `0x2000` is dispatched by **nothing** in
`Weapons_DispatchFire`, and bit `0x4000`'s handler (`0x088537ac`,
`world+0x58`) - the one candidate for where it actually fires - is itself
unread past being reachable, and is not even confirmed to be the Cannon's;
the Quake's handler (`Weapon_FireQuake`, `0x0886c600`) and the LeachBeam's
(`Weapon_FireLeachBeam`, `0x08866658`) are each named only from the dispatch
table and a bit map, at confidence 82, with **no body read at all** -
`plasma.md`'s own words, "do not assume anything about what they do."
Addresses anyone picking this up next would need to read:

- `0x088537ac` (`world+0x58`) - the Cannon's own candidate, and whether it is
  the Cannon at all is itself unsettled.
- `0x0886c600` - `Weapon_FireQuake`'s body.
- `0x08866658` - `Weapon_FireLeachBeam`'s body.

**Checked `docs/ghidra/functions/ps3-hdfury-eu/weapons.md` for a cross-title
shortcut, per this pass's own instruction to look before declaring a block -
it is not one.** HD/Fury's PS3 binary does carry real, `__FILE__`-tagged
`CannonManager` and `LeachBeamManager` classes (`weapons.md`'s own finding,
independent of this pass), which is a pleasing confirmation that the Cannon
and the LeachBeam are real weapons in this engine lineage rather than cut
content - but it says nothing about how Pulse's PSP binary dispatches or
flies either one, being a different binary from a later console generation.
A lead worth naming, not a licence to port HD's shape onto Pulse's - doing
that would be exactly the plausible-reading-across-titles this pass was
told not to do.

**So the pass went to the best weapons-area improvement available without
Ghidra**: a laid mine or bomb now draws the pose it landed in rather than a
bare translation - see the "Open" list below, where that item is now marked
done, for the reading and its confidence split.

## 2026-09-07, later: all three fire bodies read, and the Cannon candidate above was wrong

**With a live Ghidra session this time**, per
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md) -
the page to read, not this row. Three findings that matter most to whoever
builds from this next:

- **`0x088537ac`, this thread's own candidate two entries up, is not the
  Cannon.** It decompiles as `Ai_Construct`. `Weapons_DispatchFire`'s bit
  `0x4000` really does go to a Cannon handler, but the correct address (found
  by decompiling the dispatcher directly and re-adding the image base by hand)
  is `0x088577ac` - `0x088537ac` was a `0x4000` arithmetic slip in `plasma.md`'s
  own table, ironically the same magnitude as the bit under discussion.
  `weapon-fire.md` had the right address all along, uncited by the later table
  that superseded it.
- **The Cannon does not use the fire-request-bit system at all**, which is why
  its own request bit `0x2000` being "dispatched by nothing" was never a sign
  of missing work. A separate per-frame function, gated directly on
  `craft+0x1bc == 3` (the held-weapon id), runs a reload countdown built from
  the Cannon's own authored `rate` and periodically arms bit `0x4000` itself.
  Fully buildable now.
- **The Quake does not need track deformation.** Its fire handler locates the
  firing craft on the track's own spline (the same `SplinePt` records
  `engine.md` already reads) and stores a travelling position - a segment, a
  parametric `t`, a direction sign - with no mesh, vertex or collision-geometry
  access anywhere in the chain. Its damage/slowdown application reuses the
  same shared `entity+0x130` pending-hit channel the Missile and Mine/Bomb
  already use. **Not fully buildable yet**: the function that advances that
  travelling position every frame, and the one that flags "the wave has
  reached this craft," were not found.
- **The LeachBeam reuses the Missile's own lock-on for victim selection**,
  unmodified - nothing new to build there. It drains continuously while
  connected (not a single hit), crediting one accumulator on the victim
  (`entity+0x120`) and a symmetric one on the shooter (`entity+0x128`) every
  tick a range/shield/state gate passes. **Not fully buildable yet**: the two
  rate functions and whatever consumes those two accumulators into an actual
  health/shield change were not decompiled this pass.

Eight names landed in `names.tsv` this pass, two of them raised confidence on
already-recorded ones (`Weapon_FireQuake`, `Weapon_FireLeachBeam`, both
82 -> 88): `Weapon_FireCannon`, `Cannon_UpdateReload`, `Cannon_Init`,
`Quake_Init`, `LeachBeam_InitLocked`, `LeachBeam_InitUnlocked`,
`LeachBeam_UpdatePool`, `LeachBeam_Drain`. `plasma.md` and `pickups.md` are
both corrected in the same change.

## 2026-09-07, later still: the Cannon built, ten of thirteen

Built straight off the same day's evidence page, with no further Ghidra
session - the page's own "buildable now" claim held. `Weapon::Cannon` joins
`oag_gameplay::pickup::IMPLEMENTED`; `oag_formats::weapons::CannonStats`
decodes the block whole (`absorb rounds rate damage_per_bullet
slowdown_time`); `Race::advance_cannons` is the per-craft, per-tick port of
`Cannon_UpdateReload` - every craft holding one fires itself, twin barrels
alternating by the low bit of its own remaining `rounds`, until the
magazine empties.

**Two things this weapon does that no earlier one does, both load-bearing
for whoever touches this next:**

- **It is the first weapon with no fire-request arm at all.**
  `Race::spend_pickup`'s Cannon arm and `Race::spend_opponent_pickup`'s both
  `return` unconditionally - a press (human or AI) is a recovered non-event,
  not a stub. The pickup is spent only by
  `oag_gameplay::pickup::Held::advance_cannon_reload` reaching zero rounds.
- **It self-arms rather than being armed at grant time.** A pad crossing,
  `--give` and a test setting `Held::weapon` directly are three different
  ways this project fills the slot, and only the first goes through this
  crate's own code at all. `advance_cannon_reload` reads `cannon_rounds == 0`
  as "not yet armed" on its first call after any of the three, rather than
  requiring a second call site to remember to initialise it - see that
  method's own doc comment for why zero is safe to read that way.

**One number in this weapon could not come off the disc and had to be
invented, flagged accordingly.** `Cannon_Init` adds a per-class base speed
(`func_0x00060af4`, `0x08864af4`) to the firing craft's own current speed,
and the Cannon's `<Stats>` authors no speed at all - not even a per-class
one, the shape every other projectile weapon here has. `0x08864af4` was not
decompiled this pass; `oag_gameplay::projectile::cannon::BASE_SPEED_KMH` is
this build's stand-in, `400.0`, chosen and given no confidence score. This
is the one place `cannon-quake-leachbeam.md` is not sufficient to build the
Cannon from without a further Ghidra session, and its own "Not chased" line
already said so.

**Damage is direct-hit only, and that is read off the schema rather than
chosen.** The Cannon is the only projectile weapon whose block authors
neither `blastforce` nor `blastradius`, so `oag_gameplay::projectile::step`
gives it its own arm rather than routing it through the shared
`blast`/`blast_stats` radius sweep: a round that struck a craft costs that
craft `damage_per_bullet` directly, through the same recovered
`apply_weapon` gate every other weapon's hit uses, and a round that struck
geometry costs nobody anything.

**The flight itself needed no new code.** No `Cannon_Update` was found on
the evidence page, so a round rides the shared floor-follower every other
unread projectile weapon here already gets as a placeholder - the same one
the Missile, the Plasma and the Shuriken fly - and it draws as the same
billboard-sprite fallback those three already use, since it has no model of
its own either. Nothing needed touching in `crates/game/src/race/scene/
frame.rs` for the round to be visible.

Verified against `pulse-psp-usa.chd`: `WeaponStats_ParseCannon`'s block
decodes with authored `rounds`/`rate`/`damage_per_bullet` all above zero;
`--give cannon` on a real circuit fires a round with `SQUARE` never held,
which is the whole point of the weapon.

## Open

- **`<Plasma charge_time>` is authored and nothing read spends it, and play
  says something should.** This is the one place the port is knowingly at odds
  with a from-play report, so it is first on this list. The whole press-to-flight
  path was read - `Ship_FireHeldWeapon` (`0x08844ae8`) -> `Weapon_RequestFire`
  (`0x08862d9c`) -> `Weapon_FirePlasma` (`0x0886a868`) -> `Plasma_Update`
  (`0x0885c6cc`) - and **none of the four holds a shot back**; `Plasma_Update`
  reads `+0x54` as a plain age. A maintainer who plays Pulse, asked cold, says
  the Plasma winds up before it fires, and that oracle has been right twice on
  this subsystem already. **Two places to look that this session did not
  follow**: the *second* fourteen-entry jump table at `0x08a7bc90`, which
  `Ship_FireHeldWeapon` dispatches through by `weapon_id + 1` **after**
  `Weapon_RequestFire` returns, and `WO_PLASMA_FLASH`'s absent call site.
  **The first of those two is now read and closed**: `0x08a7bc90` is a computed
  goto inside `Ship_FireHeldWeapon`, not a call table, and its Plasma arm is
  five instructions that increment a per-weapon "times fired" tally and branch
  to the shared exit. No arm holds a timer. What is left is
  `WO_PLASMA_FLASH`'s call site and the HUD - a charging weapon usually has a
  meter, and `Arcade_HUD.xml` is fully parsed. `PlasmaStats` carries no field
  for the attribute meanwhile, the same treatment `damageradius` gets.
- **A trap that will cost somebody an hour, left behind by that read.** The
  tally above sits at **`+0x1ac` on the object at `0x00057fdc`**, and
  `weapon-fire.md` and `mine.md` both record two failed sweeps for what writes
  **`craft+0x1ac`**, the Mine's round counter. The offsets collide exactly and
  the objects are different. Whoever finds this increment while hunting that
  writer will think they have it; they have not, and it is incremented here
  where `Weapon_DropMines` decrements.
- **The Plasma draws no detonation.** `Race::blast_for` returns `None`: the pool
  teardown that consumes `Plasma_Update`'s destroy bit was not followed, and
  `WO_PLASMA_FLASH` is authored with no located call site. `FUN_08867370` is the
  template - find the Plasma-pool equivalent (cursor `+0xa4`, array `+0x64`, cap
  16) and see what it calls at teardown. The damage and impulse land; only the
  picture is missing.
- **`0x0885c5a4`, the Plasma's speed lookup, is deliberately unnamed.** It was
  not decompiled; a name off a structural analogy with `Rocket_SpeedForClass`
  alone is what the rubric's 50-70 band is for.
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
- ~~**Nothing dispatches bit `0x2000`**, the Cannon's own fire bit~~. **Read
  2026-09-07**: it never needed to be dispatched. The Cannon fires through a
  separate per-frame reload countdown gated on the held-weapon id, not through
  the request-bit system - see
  [cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).
  Bit `0x4000`, which *is* dispatched, goes to `0x088577ac` (**not**
  `0x088537ac`, an arithmetic slip corrected the same day), confirmed the
  Cannon's own burst spawn.
- ~~Two weapons are still unbuilt outright~~. **The Cannon landed
  2026-09-07** - see the entry below. **One weapon is still unbuilt
  outright** (Repulser, deferred as Eliminator-only), **and two more are
  buildable in part**: the Quake for its hit/slowdown half but not its own
  per-frame travel along the track, and the LeachBeam for target selection
  and its connect/disconnect gate but not its actual drain amount. None of
  the three needs track deformation, a fact this thread had wrong as
  recently as the entry above.
- **What ends a Shuriken is a reading, not a recovery.** `Shuriken_Update`
  counts `+0x48` up - the offset the Mine's fuse lives at - and the pool
  teardown that would read it was not followed, so this build reaps a timed-out
  blade **silently**, which is what the Rocket's own pool does.
  `WO_SHURIKEN_EXPIRE` is authored and stays unwired for the same reason. The
  teardown is the first thing to read if anyone wants that closed.
- **`rhicochetdamage` and `rhicochetForce` are decoded nowhere.** The only
  *second* damage and force any weapon authors, and nothing read says when they
  are spent - a glancing hit off a craft is the obvious guess. There is
  currently no moment in this engine where they *could* be spent: a hull hit
  ends a blade and only geometry bounces it.
- **`WO_SHURIKEN_TRAIL` is authored and unwired.** It hangs off a *second*
  anchor whose basis the constructor rotates by -pi/2 and the update rebuilds
  every tick; a `Projectile` here carries a position and a velocity and no roll,
  so there is nowhere to put it.
- **The Repulser's handler is read and it is not an instantaneous blast.**
  `Weapon_FireRepulser` (`0x0886ce8c`) **copies four of its own `<Stats>` onto
  the firing craft** at `+0x170`/`+0x178`/`+0x17c` and *then* spawns a pool
  entity - a shape no other weapon has, and it reads as "the field is a state
  the craft is in" rather than a one-off push. Recorded in `shuriken.md`'s last
  section rather than a page of its own, because one function is not enough for
  one.
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
- ~~**A mine and a bomb both draw no model.**~~ **Done 2026-09-05** - see
  Next Steps.
- ~~**A laid mine's or bomb's own drop orientation is recovered and
  unwired.**~~ **Done 2026-09-07.** `Projectile` now carries `orientation:
  Quat`, set at drop from the firing craft's own body orientation and read
  back by `mine_model_matrices`/`bomb_model_matrices`. Kept out of
  `crate::hash`'s reference on purpose - it is presentation state nothing in
  the simulation reads back, so hashing it would only make the reference move
  the day the field started being set, for a value that cannot be the reason
  two runs diverge. See `oag_gameplay::projectile::mine::frozen_pose`'s doc
  comment for the two-part reading it carries: that a laid charge freezes a
  pose at drop is recovered from `Mine_Init` (`0x08859ac8`, confidence 90,
  `entity+0x60..0x9c`); that the pose is the craft's *body* orientation rather
  than the rear emitter's own local rotation is chosen, not measured, and
  carries no confidence score - `Mine_Init` actually copies `craft->anchor`
  (`craft+0xa0`), and this engine has no located rear-emitter transform to
  read instead. Applied to the Bomb too, on the same "one weapon in two
  sizes" footing this module already applies to `at_rest`, even though the
  Bomb's own spawn helper (`0x0885f188`) is unresolved and does not confirm
  the same matrix copy happens there.
- **Whether `Pulse_Mine.vex` needs a `MODEL_YAW`-style per-model axis
  correction, checked the same way `MODEL_YAW` itself was measured, and
  mostly closed rather than open.** `MODEL_YAW` (`race.rs`) exists because
  `Ship.vex`'s own hull was measured wider at one end than the other and is
  authored nose-along a different axis than `oag_physics::Body::forward`
  uses - a **per-model** fact, not a `.vex`-format universal. `oag-view
  --mesh` on `Pulse_Mine.vex` (`just view
  "data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad" --mesh
  'Data\Weapons\Pulse_Mine.vex' --screenshot ...`) shows a caltrop - three
  spikes at roughly 120° about one axis, confirmed from three yaw/pitch
  angles - not a directional hull like the ship's. A yaw error about that
  axis has no "wrong way round" to fall into: any of the three spikes reads
  as the front equally well, unlike a ship's nose and tail. **Left open, and
  genuinely unmeasured**: whether the caltrop's own axis - which way its
  points face - agrees with `Body::orientation`'s convention, a coarser and
  lower-stakes question than the ship's. A live-race screenshot was tried to
  settle even that and could not: the chase camera sits almost exactly where
  a charge is dropped, so a freshly-laid mine is occluded by the firing
  craft's own hull in every framing tried (stock chase view, `--camera-view
  far`/`close`, a dozen `--camera-pose` placements at the grid and moving).
  `Pulse_Bomb.vex` was not separately viewed - worth five minutes for whoever
  next touches this, on the same recipe.
- **A player mashing the fire button lays mines three times faster than the
  authored rate, and it was caught by this session's own `--give` telemetry
  rather than looked for.** `DROP_INTERVAL` is `0.1 s` and `CLUSTER` is `5`,
  which cap a *sustained* drop at one every six ticks - but a capture driven
  by `--press square` (one rising edge every two ticks) logged 30/60/90/120
  mines in the air at ticks 60/120/180/240: exactly one mine per press-edge,
  three times the authored ceiling. Also laid throughout the whole start-line
  countdown, while the craft cannot move. Likely cause, not confirmed:
  `Race::spend_pickup`'s Mine arm calls `begin_drop(drop.count)` on every
  press with no `pickup.is_dropping()` guard, so a re-press mid-cluster
  re-arms the drop and lays immediately rather than being ignored until the
  cluster finishes. **Not fixed here** - it moves mine positions and
  therefore the world hash, so it needs a change (and a hash regen) of its
  own; recorded so the next person driving weapons with `--give` does not
  mistake it for their own bug.

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

- ~~**Draw the two rear weapons' models.**~~ **Done 2026-09-05.**
  `MINE_MODEL_ENTRY`/`BOMB_MODEL_ENTRY` (`oag_game::race`) load
  `Data\Weapons\Pulse_Mine.vex`/`Pulse_Bomb.vex` through the existing `.vex`
  path, drawn translation-only (a laid charge carries zero velocity - see
  `oag_gameplay::projectile::mine::at_rest`). `Mine_Init`/`Mine_Construct`
  were read live first, per this item's own instruction: no call in
  `Mine_Init` resolves to `Psys_Spawn_q`, so there is no drop-time particle
  effect to wire, checked rather than assumed - see `mine.md`'s
  2026-09-05 section. Two real pre-existing bugs came out of adding a second
  and third model to the same code path: `Race::rocket_model_matrices`
  filtered `kind.is_some()` rather than `kind == Some(Weapon::Rocket)`, so
  any live projectile drew as a Rocket mesh whenever one was airborne
  alongside a loaded `Rocket.vex`; and `Race::projectile_sprites` gated its
  billboard fallback on one `bool` for every kind, so a live Missile,
  Plasma or Shuriken drew nothing at all (no mesh, and no billboard either)
  whenever `Rocket.vex` had loaded. Both fixed, both with a regression test.
- ~~**Their own cues are still unwired**~~ **One of six landed, 2026-09-06;
  five stay open and here is exactly why.** `Cue::MineLaunch` now fires
  `MINELAUNCH` off the firing craft's own emitter (`Placement::Craft`) every
  time `Race::lay_mines` lays a charge whose weapon is `Weapon::Mine` -
  `crates/game/src/audio/sfx.rs`, wired from `crates/game/src/race/weapons.rs`.
  Both `Mine_Init` (`0x08859ac8`) and its only caller, `Weapon_DropMines`
  (`0x088675cc`), were decompiled directly this session to settle *how* the
  cue plays, not only *that* it does: the emitter argument traces, through two
  pointer hops, to `craft+0x50` - the same offset `Collision` and `Shield`
  already read. Confidence 78 on that reading; see `mine.md`'s 2026-09-06
  section for the instruction-level evidence and `oag-wad sounds` output
  confirming all six cues live in `weapons.bnk`. A WAV of a real cluster
  drop, real disc, real mixer: `data/shots/mine-launch.wav`
  (`crates/game/tests/mine_launch_audio_ground_truth.rs`).
  **The other five stay silent, each for a distinct, checked reason:**
  - **`MINERADAR`** (the Mine's own second cue) anchors to a *new emitter
    `Mine_Init` allocates for the mine entity itself*, not the craft - a
    held, per-projectile voice. Nothing in `CueEvent` or `SfxVoices` can
    address that today; every held voice this engine plays (`Engine`,
    `Shield`, `Blowup`) is keyed by grid slot, and building a projectile-slot
    equivalent is real work of its own. Its single waveform also carries no
    loop bit (`oag-wad sounds ... --cue MINERADAR` says "0 looping"), so even
    what the held handle is *for* - a recurring ping, a one-shot the original
    can cancel, something else - is unread, not merely unimplemented.
  - **`BOMBLAUNCH`/`~BOMBRADAR`** do not have a recovered trigger at all.
    `Weapon_FireBomb` (`0x08863a20`) was decompiled end to end this session
    and it never calls the play function `Mine_Init` does - so unlike the
    Mine's pairing, which a direct decompile confirmed, the Bomb's cues are
    only known to sit in the same `.rodata` group as its model string. The
    call site is either inside `func_0x0005f188` (the spawn helper
    `contact-response.md` already records as unresolved past its prologue -
    two callers jump directly into it, past its real entry point) or is not
    on the fire path at all.
  - **`BOMBEXPL`/`BOMBEXPL_PC`** were already an open item before this
    session (see the Bomb's own explosion note above) and remain so: the
    Bomb's teardown function is a distinct one from `Mine_SpawnExplosion`'s
    and has not been read.
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
  See [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md#mine_spawnexplosion-plays-wo_mine_explo)
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
- ~~**The Missile's flare rides one instance where the original rides
  two.**~~ **Wrong framing, corrected and implemented the same session.**
  "Needs a located missile model" was never true: the two anchors are not
  hull locators at all, they are two points `Missile_Update` computes fresh
  every tick, orbiting the missile's own flight line - a `sin`/`cos` pair on
  the same angle, growing to a fixed `1.5`x`3.0`-unit ellipse over the first
  half second and rotating at `15 rad/s` for the whole ~3 second flight.
  Fully derivable from state this engine already tracks per projectile
  (`position`, `velocity`, `lifetime`), so it needed no model - see
  [missile.md](../../docs/ghidra/functions/psp-pulse-usa/missile.md#the-two-flare-anchors-orbit-the-missiles-own-flight-line)
  for the read and `oag_game::race::weapons::missile_flare_anchors` for the
  port. **Independently confirmed from play, asked before any of this was
  decompiled**: "the missile rotates with two trails" - matching the
  instruction-level reading of a rotation rather than the crossfade this
  session first assumed from the raw sin/cos terms alone. `Race` now carries
  a second per-slot instance array (`projectile_flare_orbit`) purely for this
  second anchor; the Rocket's single-instance path is untouched.
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

## Answered 2026-09-07: Pulse's Quake is fire, **Pure's** is the concrete one

The maintainer's *"like a concrete wave"* is a real memory of a real effect -
just not Pulse's. Every colour below was parsed with `oag_formats::pob`, not
transcribed by hand.

| title | `WO_QUAKE.POB` emitters | reads as |
| --- | --- | --- |
| **Pulse** (PSP `Data.wad` #524, PS2 `WADS2.WAD` #3758, byte-identical) | `WO_QUAKE` cream -> bright orange -> ash-brown (additive); `fireballs` near-white -> red-orange -> black (additive); `debris` near-white (alpha-over) | fire |
| **Pure** (PSP `Data.wad` #587) | the same, **plus `bobs`: grey `(102,102,102)` -> black, alpha-over** - occluding rather than glowing, one fixed shade per particle | **concrete** |
| **HD/Fury** (`DATA02.PSARC:/data/psys/wo_quake.pob`) | fire/molten/dust, no grey | fire |

**No Pulse emitter authors grey at all**, so our wave is not mis-tinted and
**was not tinted to match the memory** - which would have been exactly the
"plausible-looking stand-in" this file's own rule forbids.

**Our render was checked against the file and is faithful.** The whiteout in
the screenshots is dense *additive overlap* - track-width-scaled quads with a
real `dst_factor: One` summing past 1.0 and clipping to white - which is a
correct consequence of the authored blend mode, not an override. The whitening
term removed from `psys.wgsl` on 2026-08-12 is confirmed still removed; palette
RGB is sampled with alpha from the separate alpha channel rather than the
palette's own alpha byte; blend classes map 2 -> Additive and 3 -> AlphaOver;
and `Race::advance_quake_visual`'s `(right - left).length() / 50.0` matches
`Quake_Update`'s own recovered scale formula.

**One gap recorded rather than glossed:** at distance (`/tmp/quake-fix-t40.png`,
`t80.png`) the wave stays warm-white instead of resolving into individual
orange particles, so the saturation story is argued from the blend maths rather
than confirmed by eye. Minor, and open.

**If Pure's grey wave is ever wanted in Pulse**, that is the
[enhanced-rendering opt-in](../../docs/overview/goals.md) shape - another title's
authored effect played deliberately - and not a tint invented here.

## The Quake's "no track deformation" finding is about the fire handler only

**2026-09-07, from the maintainer's own play experience:** *"In the originals,
the wave at least looks like it does track deformation, like a concrete wave."*

That is a play observation of the original, and this project treats those as a
reliable oracle. It does **not** contradict
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)'s
reading, and the distinction matters for whoever builds this:

- What was actually read is **`Weapon_FireQuake` (`0x0886c600`)**, the *fire
  handler*. It stores a travelling spline position (segment, `t`, direction)
  and touches no mesh or vertex data. That is a claim about **one function**.
- The same pass recorded that **the wave's own per-frame travel is unlocated**.
  A deforming visual would live there, or in the render path that update
  drives - not in the code that spawns the projectile.

So the open question is not "does it deform" but **where the deformation is
produced**, and there are at least three shapes it could take, none of which the
fire handler would show:

1. A per-frame vertex displacement applied to track geometry near the wave's
   spline position, which is what "concrete wave" most directly suggests.
2. A separate authored mesh or effect drawn *over* the road and travelling with
   it, deforming nothing - visually similar, structurally unrelated.
3. A shader-side displacement keyed off the wave's position, with no CPU-side
   geometry write at all - which would explain the absence of mesh access in
   every function anyone has read so far.

**Do not build the Quake's visual on a guess between those three.** Per this
project's rule against inventing what the assets already author: locate the
per-frame travel first, see what it drives, and if it will not resolve, draw
nothing and say so rather than authoring a plausible-looking wave. An invented
stand-in that reads as legible is exactly how a wrong picture survives review
here - it has already happened twice.

The hit and slowdown remain buildable now: they reuse the shared channel the
Missile and Mine already use.

## 2026-09-07, later still: the per-frame travel is found - shape 2, `WO_QUAKE`

The three shapes above are resolved.
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)
now names `Quake_Update` (`0x0891d268`) - dispatched indirectly, not by a plain
`jal`, but readable at instruction level regardless - as the function that
advances the wave's spline `t` every tick, and `Quake_SampleSpan`
(`0x0891c028`) as the helper that turns `t` into two points across the track's
width.

**Shape 2**: `Quake_Update` uses those two points to build a transform and, the
first time a wave instance's node id is zero, spawns the disc's own
`WO_QUAKE` particle effect through `Psys_Spawn_q` - the effect name was read
directly out of memory, not inferred from the fourcc tag - alongside a
travelling positional sound (`Sound_Play`, `600.0`-unit falloff). Every frame
after that it re-positions (to the midpoint of the two current track-edge
points) and re-scales (to the track's own width, via an edge-to-edge distance)
the same effect, rather than re-spawning it. A call into
`AiTrack_LocatePosition` happens in the same block but before the orientation
basis is built and its result feeds nothing that basis uses, so - unlike
position and scale - **orientation tracking the track's own banking is not
established** by this reading. Nothing here writes a vertex or a mesh handle.

**Why that still fits "looks like it does track deformation"**: an authored
effect that tracks the road's own width and follows its spline every frame
reads as part of the road to a player, without the road's own geometry ever
changing. That is a plausibility argument for reconciling the play observation
with the reading, not a side-by-side comparison against the original - nobody
has looked at what `WO_QUAKE.POB` itself draws, so if it turns out to look
nothing like a wave,
that specific claim (not the trigger recovery) is what would need revisiting.
Shape 3 (shader-side displacement) isn't excluded by anything read in this
function, but the PSP's GE has no programmable vertex stage to put one in -
a hardware argument, not a code-reading one, against that shape on this
platform.

**Still not fully buildable**: the per-craft latch (`entity+0x860 & 0x40`) that
ties the travelling wave to the shared hit channel was not found this pass -
`field 0x860` alone is not a selective enough search (dozens of hits), and the
more selective `masked`/`andi` subcommands of `scripts/psp-relocate.py` were
not tried. The visual half and the hit-timing half are each independently
buildable now, but not yet wired to each other.

## 2026-09-07, later again: the latch found, the Quake built - twelve of thirteen

**The latch was inside the already-quoted function all along.** Reading
`FUN_088418e0`'s wave branch past what the earlier prose summarised as
"if (s2->0x860 & 0x40) goto apply" turns up the setter a dozen lines above:
a per-craft proximity value, smoothed at a fixed `8.0`/s rate, crossing a flat
`0.1` threshold. See
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)'s
"The latch setter, found 2026-09-07" section for the full mechanism and the
two new functions it named (`Quake_ProximityToCraft`, `Quake_SpanIntensityAt`).
**The Quake's own authored `radius` has no other consumer anywhere in the fire
chain**, which is the strongest evidence yet that `radius` is this mechanism's
authored gate rather than the unauthored `200.0`/`0.1` engine constants - so
that is what the port spends.

Built the same session, from that page and this thread in full:

- `oag_formats::weapons::QuakeStats` - the four authored attributes.
- `oag_gameplay::projectile::quake::Wave` - the single travelling instance,
  tracked as a plain `f32` distance-along-course in
  `oag_race::Standing::progress`'s own convention rather than the original's
  segment+parametric-`t` pair. The two are equivalent on a closed ring and
  this needed no new per-tick state beyond what standings already compute.
  Advances at the recovered `270.0` units/second, wraps at the course length.
- The hit/slowdown, wired to the shared pending-hit channel the Missile and
  Mine/Bomb already spend - a flat in/out-of-`radius` test rather than the
  original's smoothed debounce, edge-triggered the same way. **Chosen, not
  measured**, and said so in the type's own doc comment.
- `oag_race::Course::tangent` - new, small, what launch uses to pick the
  wave's own direction (dotted against the firing craft's forward, matching
  `Quake_Init`'s own dot product).
- The visual: the disc's own `WO_QUAKE`, attached once and followed/rescaled
  every tick to the midpoint and width of the track's own two edges nearest
  the wave's current position - read off `Spline`'s own sample fields
  (`pos`/`lateral`/`half_width_left`/`half_width_right`), the same ones
  `oag_render::track::build_model` already draws the ribbon's edges from.
  `psys::Stage::rescale` is new, mirroring `follow`. **Orientation is not
  established by anything read** (the original's own `AiTrack_LocatePosition`
  call is dead code in its own basis build, confirmed independently this pass
  by re-deriving `Quake_Update` rather than only quoting the earlier read), so
  the effect draws axis-aligned - **chosen, not measured, no confidence
  score**.
- `Weapon::Quake` added to `pickup::IMPLEMENTED` - a pad can hand one out now.

**Not wired**: the wave's own positional sound cue and its distinct hit cue.
This engine's `Cue`/`CueEvent` system is per-craft-slot only and has no
concept of an arbitrary moving 3D emitter; extending it was out of scope for
this pass. Silence is the honest state here rather than anchoring a cue to a
craft slot the original never emitted it from.

**Twelve of thirteen weapons now do something.** Only the LeachBeam remains,
blocked on its own two drain-rate functions and whatever consumes
`entity+0x120`/`+0x128` - see `cannon-quake-leachbeam.md`'s own Open list.

## 2026-09-07: the countdown weapon-gate call above is void - confirmed on pulse-psp-eu

This thread's mine drop-rate pass left weapons deliberately **ungated during
the countdown**, on the stated evidence that Time Trial's free Turbo was "the
one reachable case that demonstrably worked" - i.e. that the original itself
holds a Turbo through the countdown, so gating it here would diverge.

**That evidence was our own build's behaviour, not the original's.** The
maintainer tested `pulse-psp-eu` directly this session: the free Turbo is
granted **after** the countdown releases the craft, not at the start line -
so the original never holds anything to gate in the first place. See
`handover/frontend/hud-icon-turbo-timing.md` for the full trace (the green-hexagon
HUD comparison that surfaced this).

**So a blanket countdown weapon gate may now be the correct call**, where it
was rejected on the reasoning above.

**Landed 2026-09-07**: `Race::grant_free_turbo`'s call site moved from
`Race::start` (tick 0) to the release edge (`world.tick ==
oag_race::state::COUNTDOWN_TICKS`, i.e. the same tick
`RaceState::thrust_gated` first reads `false`) - see
`handover/hud-icon-turbo-timing.md`, now closed, for the exact change and
its test flips. **Still open here**: check whether any *other*
weapon-during-countdown path still needs a gate, or whether removing the
early Turbo grant was the whole gap. Not attempted this session - the mine
drop-rate pass this thread's own body covers is a separate concern from the
gate question.
