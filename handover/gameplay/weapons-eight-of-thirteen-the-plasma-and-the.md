# Weapons: thirteen of thirteen, the Repulser last (2026-10-04), and the dispatch table read whole

**Retitled 2026-09-10 (thread audit note): the title and every index line
citing it said "nine of thirteen" through the Cannon (ten, 2026-09-07), the
Quake (twelve, same day) and the LeachBeam (thirteen less Repulser,
2026-09-08) landing in this same file without the title ever being updated -
the single most stale header found in this pass.**

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
`/ 3.6`. `oag_weapons::projectile::advance` already *was* that function, so the
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

**Two files split rather than baselined** (`crates/tables/src/weapons.rs`,
`crates/raceplay/src/field.rs`), and the second surfaced the *same* fault the
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
`0x0880d124`, 92); `oag_weapons::projectile::mine`; a drop that lays
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
every line of `oag_weapons::projectile::mine` except a count. Three
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
`oag_weapons::pickup::IMPLEMENTED`; `oag_tables::weapons::CannonStats`
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
  `oag_weapons::pickup::Held::advance_cannon_reload` reaching zero rounds.
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
decompiled this pass; `oag_weapons::projectile::cannon::BASE_SPEED_KMH` is
this build's stand-in, `400.0`, chosen and given no confidence score. This
is the one place `cannon-quake-leachbeam.md` is not sufficient to build the
Cannon from without a further Ghidra session, and its own "Not chased" line
already said so.

**Damage is direct-hit only, and that is read off the schema rather than
chosen.** The Cannon is the only projectile weapon whose block authors
neither `blastforce` nor `blastradius`, so `oag_weapons::projectile::step`
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
its own either. Nothing needed touching in `crates/raceplay/src/scene/
frame.rs` for the round to be visible.

Verified against `pulse-psp-usa.chd`: `WeaponStats_ParseCannon`'s block
decodes with authored `rounds`/`rate`/`damage_per_bullet` all above zero;
`--give cannon` on a real circuit fires a round with `SQUARE` never held,
which is the whole point of the weapon.

## Open

### 2026-10-04: the Repulser built (pulse-repulser lane)

Read whole on [repulser.md](../../docs/ghidra/functions/psp-pulse-usa/repulser.md)
(twenty functions, 60-90) and built: `oag_weapons::projectile::repulser`,
`Race::fire_repulser`/`advance_repulsers`/`advance_repulser_visual`. What is
still open, most player-visible first:

- ~~The field model is not drawn~~ **drawn 2026-10-04 (pulse-repulser-2)**:
  `race::repulser_field`, matrix read to the instruction and every ease
  confirmed live (88). It is a faint smooth ring; the original's bright
  **beaded** ring round the craft is the *blast psys*, not this model (proved
  live by holding the field alpha at zero, see repulser.md).
- ~~Not runtime-verified~~ **read live on PPSSPP 2026-10-04**: one update per
  60 Hz frame, cursors `+5`/`-2` a call, wave start at age 0.8008. One
  stationary Time Trial, so it lifts the steps-per-call law, not hardware
  pacing. The hit law (`+0x110` write) is still not watched live.
- **Open: the blast's bead ring.** Ours starts wide (about 13.6 units) and
  collapses by call 20; the original's stays compact round the craft from call
  10 to 40, then whitens. That is `WO_REPULSER_BLAST`'s own playback, and the
  next lever is its flag `0x200000` (evenly stepped ring angles) and its
  selector-5 record, still not played by `oag_fx::psys`
  ([pob.md](../../docs/formats/pob.md)). Frames:
  `psp-seq.png` against `oag-seq.png`.
- ~~**Open: the wave-start whiteout is confirmed as ours.**~~ Closed 2026-10-06: the `shazzam` quad is dropped by the GE's guard band, see particle-system.md.
- ~~The junction fork's third wave is not built~~ **built 2026-10-04**: the
  in-run fork (`Repulser_AdvanceWave`, 88) on `oag_race::Course::branches`
  (05, 07, 14, 23 carry one), off the forward wave. Not built: the init-tick
  variant (`Repulser_ForkAtJunction`, 72) for a firer already on a branch, and
  a backward-wave fork (`AiTrack_StepBackward` reads the exit junction's
  successors, unexplained). Open: whether 07/14/23's branches are real routes
  or duplicates. Not yet seen in-game: no capture fired one across a split.
- **HD plays Pulse's law now.** HD's Eliminator tables weight the Repulser
  (`ai=8 human=8`), and nothing gates it by title. HD's `Repulser_Construct` is
  read but its law is not.
- **Time-to-five moved** (`eliminator_finish_ground_truth`, parked player,
  `16_Track`), because opponents now fire it: seeds 13/16/5/9 went from
  71.4/105.8/64.3/53.2 s to 15.7/65.6/70.5/73.1 s. Seed 13's 15.7 s is one
  Repulser finishing five craft that were already at 8-15 of 95 shield - the
  wave reaches about 1,400 units ahead.
- **`~REPULSORTRAVEL` stays unwired, and that is measured** (2026-10-04): the
  cue is one opcode `0x14` grain with no sound on both PSP discs
  (`sfx_weapon_ground_truth::repulsortravel_binds_no_sound_on_pulse_psp`), and
  its emitter sits at the world origin (entity `+0x180`, never written, read
  live). PS2's bank was not checked.
- Screenshots were taken with `--race --mode single_race --give Repulser` and an
  `--input-script` that presses fire once: `--mode` has no Eliminator.

- **The Quake's road ripple is drawn (2026-09-24); five things around it are
  not.** See the section at the bottom of this file and
  [cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)'s
  "measured live" section. (1) **The hit is still ours**: the original's
  `Quake_SpanIntensityAt_q` reads the same span position, amplitude and
  half-width the ripple does, so its hit is very probably "how high is the bump
  under this craft" against `0.1` after smoothing; this engine's is the
  authored `radius` round the wave's `progress`. Matching it moves the golden
  hashes - its own change, with its own regenerate commit. (2) **The sim's wave
  sits 15 units behind the drawn bump**: `Quake_Init` arms the launch span
  `g_quake_launch_lead` (15.0) ahead of the craft, and the render adds that
  lead; the sim's `Wave::launch` does not, so the hit is measured from a point
  15 units short of where the bump is drawn. (3) **The launch screen tint is
  not drawn** - a strong orange-white flash for the first eighth of a second
  in every PPSSPP frame; `Quake_Update`'s kind-4 dispatch (`func_0x000ec0c0`)
  is the candidate, unread. (4) **The branch of a split (05, 07, 14)** is
  placed as one function of its own `t` pinned to the fork and the merge by
  the span table's links, chosen - whether both branches ripple together in
  play is unobserved. (5) **Draw-call bounds are not grown by the bump**, so a
  batch lifted 12 units near the edge of the view can be frustum-culled a frame
  early; not seen, not checked. (6) **The live wave ended before 5.0 s**: in
  the one PPSSPP run, the last armed frame was at age 4.788 and
  `Quake_UpdateSpans` was never called again (`live == 0`), while
  `oag_weapons::projectile::quake::LIFETIME_SECONDS` is the static read's 5.0
  and gates refire. One run is not enough to move a recovered constant; it is
  a measured discrepancy with no explanation yet (the amplitude is 0.5 units
  there, so perhaps every span had already run off its far end).
- ~~**`<Plasma charge_time>` is authored and nothing read spends it, and play
  says something should.**~~ ~~**A trap that will cost somebody an hour.**~~
  ~~**The Plasma draws no detonation.**~~ **All three closed 2026-09-09** -
  see the section at the bottom of this file and
  [plasma.md](../../docs/ghidra/functions/psp-pulse-usa/plasma.md). Short
  version: the wind-up is real (the from-play oracle was right), it is a
  hardcoded 1.0 s in both PSP titles, `charge_time` is dead data in both, and
  `WO_PLASMA_FLASH` is the detonation. The `+0x1ac` trap still stands and is
  still worth reading before hunting the Mine's round counter.
- ~~**`0x0885c5a4`, the Plasma's speed lookup, is deliberately unnamed.**~~
  **Decompiled and named 2026-09-09** (`Plasma_SpeedForClass`, 88). Its twin
  `FUN_0885c650` - the same four offsets, no ramp - inherits the old state and
  stays unnamed for the same reason.
- **The Plasma's launch-speed ramp is read and not implemented.**
  `Plasma_SpeedForClass` blends `entity+0x48` (the craft's own speed plus
  `launchspeed`) into the class speed over the bolt's first second of flight;
  this engine flies at `class + launchspeed` throughout, which is
  `oag_weapons::projectile::launch`'s shared choice. Deliberate, so the Rocket
  and the Plasma cannot drift apart - worth revisiting only alongside the
  Rocket's own.
- ~~**The Plasma blast's three models are recovered and not drawn.**~~
  **Drawn 2026-09-16** - see that date's section: `PlasmaBlast_Update` read
  in full (hardcoded 1.5 s single-stage retire, a dead keyframe-ramp
  mechanism, a per-model `Node_SetAnimTimeTree` scrub most likely playing
  each model's own baked animation) and `crates/raceplay/src/blast_models.rs`
  now draws all three, anchored on the impact point with a chosen
  "face the camera" billboard standing in for the original's own unresolved
  camera-vector read.
- ~~**The Plasma's launch-speed ramp is read and not implemented.**~~ **Ported
  2026-09-16** - see the dated section at the bottom of this file and
  [plasma.md](../../docs/ghidra/functions/psp-pulse-usa/plasma.md#plasma_speedforclass-0x0885c5a4-and-the-launch-ramp).
  Short version: `flight.rs`'s charging branch now reads the firing craft's
  velocity fresh at the tick the charge ends and blends it to the class speed
  over one second, sharing `missile::speed_kmh` with the Missile - the same
  linear form `Missile_SpeedNow` independently tests over the same second. The
  Rocket is untouched and is now the odd one out rather than a shared choice:
  nothing has re-read `Rocket_Update` for a ramp.
- **The Plasma blast's three models are recovered and not drawn.**
  `PlasmaBlast_Construct` (`0x0885fd90`) loads
  `Data\Weapons\pulse_plasma_halo1.vex` and two
  `pulse_plasma_hemisphere*.vex`, orients them to the located track surface and
  animates three ramps over them. This engine plays `WO_PLASMA_FLASH` and draws
  none of the three. Nothing is substituted for them.
- ~~**A plasma bolt that times out is reaped silently here and detonates in the
  original.**~~ **Closed and ported 2026-09-16** - see the dated section at the
  bottom of this file and
  [plasma.md](../../docs/ghidra/functions/psp-pulse-usa/plasma.md#the-expiry-settles-a-damage-question-nobody-had-read).
  Short version: `FUN_0886b898` is read in full and is not the blast sweep -
  neither it nor the teardown touches `damage`/`blastradius`/`blastforce` or a
  craft's pending impulse, on either ending. `Projectiles::advance` now emits
  an `Impact { blast: false, .. }` on the timeout, matching the wall-hit
  branch in every way but the blast flag. **That wall-hit branch's own
  `blast: true` is now a known conflict, not an oversight** - the same reading
  says the original spends nothing there either, and it is deliberately not
  touched by this port; see the dated section for why.
- ~~**The Plasma's wall-hit branch in `Projectiles::advance`
  (`crates/weapons/src/projectile/flight.rs`) still credits `blast: true`,
  which the 2026-09-16 reading below says the original never does either -
  wall or timeout, `Plasmas_Update`'s teardown is identical and spends no
  blast on a Plasma.**~~ **Closed 2026-09-16, later the same day** - see the
  dated section at the bottom of this file and
  [plasma.md](../../docs/ghidra/functions/psp-pulse-usa/plasma.md#a-craft-hit-is-the-third-ending-and-it-does-spend-a-blast).
  Short version: a direct craft hit is a **third** ending the earlier pass had
  not enumerated, and it does spend a blast - `Plasma_HitCraft` credits the
  struck craft directly, `Plasma_ApplyBlastForce` pushes everyone else in
  radius but the firer. The wall-hit branch now emits `blast: false` and the
  craft-hit branch routes to a new direct-hit rule; both are ported.
- ~~**What `Rocket_HitCraft`'s identical split-shape means for the Rocket's own
  `blast()` routing is a new lead, not fixed.**~~ **Read and ported
  2026-09-16**: `RocketPool_Update` runs `Rocket_SweepCraftHit`
  (`0x0886e7ac`, a 6-unit hull-cylinder sweep) whose hit calls
  `Rocket_HitCraft` - direct damage/slowdown credit to the struck craft - and
  then `Rocket_ApplyBlastForce` (`0x0886ee88`), the Plasma's force sweep
  twin; the pool's wall/expiry teardown touches no craft. The Rocket now
  takes the Plasma's two arms in `flight.rs`/`blast.rs`, and
  `crates/gameplay/tests/determinism.rs` moved its constants for it (with its
  target moved into the rocket's arc). See
  [rocket-visuals.md](../../docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md)'s
  "What a rocket hit spends" section. The 5.0 s lifetime and
  `Rocket_SweepProjectiles` (a rocket clearing mines and bombs it flies into,
  quietly) followed the same day.
- **A player mashing fire lays plasma bolts the same way they lay mines**, and
  the array fills: `--give plasma --press square` puts **128** bolts in the
  air inside 260 ticks and holds there, and in `single_race` the craft is dead
  by tick 100 against tick 582 on the same command with `--give rocket`. This
  is the `Race::spend_pickup` re-press bug this thread already records for the
  Mine (one shot per press-edge, no `is_dropping` guard) reaching a second
  weapon - **not** something the wind-up introduced, since each bolt still
  releases one second after its own press and they stay staggered. Recorded so
  the next person driving weapons with `--give` does not read it as new.
- ~~**The charging glow rides the nose already; its ramp does not.** Checked
  rather than assumed, and the first draft of this row had it backwards.
  `Race::flare_effect_for` maps `Weapon::Plasma` to `WO_PLASMA_HEAD` and
  `advance_projectile_flares` follows *every* live projectile's position - and
  a charging bolt is a live projectile reseated on the craft's nose every
  tick, so the glow sits on the nose for the wind-up and leaves with the bolt,
  which is what the original's re-parenting does, arrived at from the other
  end. What is **not** ported is `Plasma_UpdateCharge`'s
  `(1.0 - remaining) * 0.75` scale: the glow does not grow as the shot
  charges. That needs a per-instance scale on a *riding* flare, which
  `advance_one_flare` does not currently carry - `Stage::rescale` exists (the
  Quake uses it) but the flare path never calls it.~~ **Built 2026-09-16**:
  `weapons::visuals::plasma_flare_scale` computes
  `(CHARGE_SECONDS - charge.clamp(0, CHARGE_SECONDS)) / CHARGE_SECONDS * 0.75`
  and `advance_one_flare` now calls `Stage::rescale` alongside `follow` every
  tick, so a charging Plasma's flare grows. A launched bolt's flare freezes
  at `0.75` rather than resetting to neutral `1.0` - `Plasma_Launch`'s
  recovered body writes no severity field, so nothing resets it; an earlier
  draft that reset to `1.0` produced a visible unexplained pop at release in
  the screenshot probe, which is what caught it. The `* 0.5` craft flag stays
  unported (unread meaning). Verified by screenshot against
  `data/images/pulse-psp-eu.chd`, growth clearest in the back half of the
  1 s wind-up; see
  [`PLASMA_FLARE_EFFECT`](../../crates/title/src/engine_effects.rs)'s
  doc comment.
- ~~**`Data\Weapons\Bomb_Shockwave.vex` is a located string with no read call
  site.** It sits immediately before the plasma blast's own models in the same
  `.rodata` run (`0x08a7c190`), found while reading `PlasmaBlast_Construct`. A
  neighbour reading and nothing more - but it is the first concrete thing
  anyone opening the Bomb's unread teardown has to go on.~~ **Read 2026-09-15**:
  `BombBlast_Construct` (`0x08872078`) loads a second copy of that string
  (`0x08a7cbdc`) alongside `explosion_hemisphere.vex` and `WO_BOMB_SMOKERING`
  - see [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-15-the-bombs-teardown-read---its-own-blast-not-the-mines).
- ~~**How many mines a press lays is invented.** `oag_weapons::projectile::mine::CLUSTER`
  is `5`. Two independent sweeps for what writes the original's counter at
  `craft+0x1ac` found only the handler's own decrement, and no `<Stats>`
  attribute counts mines. It is simulation state, so changing it is a hash move.~~
  **Measured 2026-09-15: five, on the running original, and the writer found**
  (`WeaponPickup_ArmMine`, `0x0886759c`) - see the section at the bottom of
  this file and [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-15-the-cluster-is-five-measured-live-and-the-counters-writer-found).
  The hash did not move.
- **That a mine does not move once laid** is the conservative reading, not a
  read. `Mine_Init` copies the firing craft's velocity into the entity at
  `+0xa0` and nothing found integrates it.
- **That `trigger_radius` is what trips a mine** is this engine's reading of an
  authored attribute; the code path that spends it was not found.
- ~~**`MinePool_Update` is deliberately unnamed.**~~ **Named 2026-09-16**
  (`MinePool_Update` 85, `Mine_SweepCraftTrigger` 88), both read in full: the
  fuse raises only the destroy bit and the teardown spawns the explosion, so a
  mine that times out hurts nobody; the trip credits the *one* craft that
  tripped it, and only inside `blastradius`. The port follows both. See
  [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md)'s 2026-09-16
  section.
- ~~**Nothing dispatches bit `0x2000`**, the Cannon's own fire bit~~. **Read
  2026-09-07**: it never needed to be dispatched. The Cannon fires through a
  separate per-frame reload countdown gated on the held-weapon id, not through
  the request-bit system - see
  [cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).
  Bit `0x4000`, which *is* dispatched, goes to `0x088577ac` (**not**
  `0x088537ac`, an arithmetic slip corrected the same day), confirmed the
  Cannon's own burst spawn.
- ~~Two weapons are still unbuilt outright~~. **The Cannon landed
  2026-09-07** - see the entry below. ~~**One weapon is still unbuilt
  outright** (Repulser, deferred as Eliminator-only)~~ (built 2026-10-04), **and two more are
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
- ~~**`WO_SHURIKEN_TRAIL` is authored and unwired.**~~ Wired 2026-10-06 with the blade's own model: see shuriken.md's 2026-10-06 section.
- ~~**The Repulser's handler is read and it is not an instantaneous blast.**
  ... "the field is a state the craft is in".~~ **Wrong, and closed
  2026-10-04**: the four copies are never read; the weapon is a blast, then two
  track-following waves. Built - see the 2026-10-04 section under Open and
  [repulser.md](../../docs/ghidra/functions/psp-pulse-usa/repulser.md).
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
  two runs diverge. See `oag_weapons::projectile::mine::frozen_pose`'s doc
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
  cluster finishes. ~~**Not fixed here**~~ **Fixed, later and unlinked commit
  (thread audit note, 2026-09-10): `crates/weapons/src/pickup.rs`'s
  `begin_drop` is now a no-op while `is_dropping()` is already true - no
  `Race::spend_pickup`-side guard needed after all. Confirmed in current
  source; this bullet was never struck through when the fix landed.**

**Fixed 2026-08-26: a laid mine or bomb rode the Rocket's flare from the
moment it landed.** Reported from play as "the mines are animating the
explosion when dropped, instead of when detonating (only tested via
pulse)". `Race::advance_projectile_flares` (`crates/raceplay/src/weapons.rs`)
gated on `projectile.kind.is_none()` - true for *any* live projectile - to
decide who rides a [`ROCKET_FLARE_EFFECT`] (`WO_ROCKET_FLARE`) instance. That
effect's emitters are looping (see `mine.md`'s neighbour reading on
`Rocket_Init`), so a mine or a bomb got it attached the instant
[`Race::lay_mines`] placed it and kept it burning at that fixed point for the
charge's whole life - which reads exactly like an explosion that started at
the drop and never really ended. Invisible to `just test`: the headless
harness builds `Race` with an empty `psys::Library`
(`crates/raceplay/src/tests.rs`), so `self.effects.get(ROCKET_FLARE_EFFECT)`
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

## 2026-09-08: the LeachBeam built - thirteen of thirteen, less the deferred Repulser

**Every weapon a player can get now does something.** The LeachBeam was the
last, and the pass that built it started by finding that the reason the
2026-09-07 pass could not was **a wrong address, not a hard function.**

**The finding, and it is the same finding twice on one page.**
`cannon-quake-leachbeam.md` names the two drain-rate functions
`func_0x0006eedc` = `0x0886eedc` and `func_0x0006ef18` = `0x0886ef18`. Both
rebases are `0x4000` low: `0x08804000 + 0x0006eedc` is **`0x08872edc`**. The
stated addresses decompile cleanly - into `FUN_0886ee88`, a radial impulse over
every craft that has nothing to do with this weapon - which is exactly why the
error survived. **A wrong address that decompiles to a real function is the most
expensive kind**, and this page has now produced two of them, the first being the
Cannon candidate `0x088537ac` it corrected on 2026-09-07. Both were the same
magnitude as a bit mask being read nearby. **If a rebased address gives a
function whose shape does not match its call site, re-do the addition before
concluding anything about the function.**

**What the corrected addresses gave, in about the time the arithmetic cost.**
`LeachBeam_DrainRate` and `LeachBeam_RepairRate` are six lines each: an authored
rate times a one-shot `energy_multiplier` held on the instance's own byte. From
there the rest fell out in one sitting - `WeaponStats_ParseLeachBeam`'s whole
nine-attribute offset map, the lifetime (`LeachBeam_Advance` returns
`age < active_time`), the linger and fizzle constants, the ribbon geometry, three
cues and two authored effects.

**The item that had been called "the single largest remaining gap" was two
functions away from a function already named.** What consumes `entity+0x120` and
`entity+0x128` is `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`, *already in
`names.tsv` since the shield-pickup pass*) and its four-line neighbour
`FUN_0883f228`. Both are called from `FUN_0883f540`, the per-craft weapon update,
in the same statement list. Nobody had decompiled the caller. **When a field's
consumer is missing, decompile the function that calls the functions you already
know** - it cost one call here and closed the item outright.

**A correction the parser forced.** The page reads `LeachBeam_Drain`'s
`target+0x134 = <stats+0x110>` as a `pending_source` id. `+0x110` is
`slowShipFactor`, and `Ship_ApplyPendingWeaponDamage` passes it on to the
victim's handling record at `+0x31c` - which `engine.md` already identifies as a
**one-shot thrust scale**, not a slowdown. So the LeachBeam throttles its victim
to 80 % thrust for as long as the link holds, and *that* is why its block is the
only one authoring no `slowdown_time`. The absence was design, and this names the
mechanic that replaces it.

~~**Deliberately not wired, and it is the one gap:** that thrust scale.~~
**Wired 2026-09-16.** Not as a `ShipState` field after all: the scale is a
per-step *input* (`oag_physics::Environment::thrust_scale`, beside `pad_hit`
and `auto_speed`), held between the drain and the next step on
`oag_gameplay::world::Ship::pending_thrust_scale` the way `pending_slowdown`
is. That keeps the physics gate unmoved; only the gameplay constants moved,
isolated the documented way. `Beam::drain` arms it under the original's three
gates. See `cannon-quake-leachbeam.md`'s "`slowShipFactor` is why this block
authors no `slowdown_time`".

**By-catch.** The Quake's hit cue is literally `QUAKEHIT` -
`PTR_s_QUAKEHIT_08a7b740`, from decompiling `FUN_088418e0` whole. The page had
it as "a sound plays here, not this sound plays here".

**A defect found in the landed Quake, and not this pass's to have caused.** See
the section below.

## 2026-09-08: the Quake wave never retires

`Race::advance_quake` ends with `self.world.quake = Some(wave)` on every path and
nothing anywhere sets the field back to `None`;
`oag_weapons::projectile::quake::Wave` carries no age and no lifetime. Two
player-visible consequences, both live on `main`:

1. `Race::spend_pickup`'s Quake arm returns early on `world.quake.is_some()`, so
   **a Quake can be fired exactly once per race** and every later Quake pickup is
   kept and unusable.
2. The wave circles the ring forever, re-hitting the whole field once a lap.

**Fixed 2026-09-08, and the lifetime is measured rather than chosen.** The
prediction in this section - that the mechanism had no analogue in a
progress-along-a-ring representation and the fix would have to be a chosen bound
- was wrong, and pleasantly so. The original's terminator is not
`Quake_SampleSpan`'s validity bool; it is a **fixed 5.0 seconds from launch**,
and it is a single scalar that ports exactly.

The apparatus neither `Quake_Init` nor `Quake_Update` reaches is a table of
*road span* records. `Quake_UpdateSpan` (`0x0891cab8`) ages each armed span by
`dt` and sets its retire byte the frame `age + dt` exceeds `5.0` - the literal
is a `lui a0, 0x40a0` at `0x0891cafc`, read at instruction level. What makes
that one wave lifetime rather than one per span is that `Quake_ArmSpan`
(`0x0891b714`) *inherits* its caller's age into the new span rather than zeroing
it, and both propagators pass the parent's own. `Quake_Init` starts the chain at
`0`. `Quake_UpdateSpans` (`0x08874a30`) then clears the busy byte (`q+0x48`) on
the first frame no span rippled, which is what re-opens `Weapon_FireQuake`'s own
gate. Confidence 85; nine names landed with the evidence. See
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)'s
2026-09-08 section.

`oag_weapons::projectile::quake::LIFETIME_SECONDS` is that 5.0, `Wave::age`
accumulates it on every path including the degenerate-course one, and
`Race::advance_quake` drops the wave before applying hits on the tick it
expires.

**By-catch, and it is a correction to a documented conclusion**: the page's
"the Quake is a travelling impulse ..., not a deformation of the track" was
drawn from the three functions that pass the deformation by.
`Quake_UpdateSpan` rewrites the road mesh's own packed `short3` vertex
positions in place under a `vcos_q` profile. **The original does deform the
track.** The narrower claim the port rests on survives whole - the damage and
slowdown are a scalar impulse through the shared pending-hit channel with no
geometry in the path - but "does the Quake deform the track" now answers yes,
and this engine draws none of it. That is a new, unbuilt visual, not a
regression.

## 2026-09-08: adding a weapon to `IMPLEMENTED` reddened an AI test, and the test was the problem

**Landing the LeachBeam turned
`oag-game::ai_roll_ground_truth a_full_grid_still_rolls_and_a_higher_tier_rolls_no_less`
(six `a_full_grid_of_*_arms_no_fewer_rolls_than_*` tests since 2026-09-09)
red**, and nothing about the barrel roll had moved. Recorded here because the
next weapon to join `oag_weapons::pickup::IMPLEMENTED` will do the same thing to
whatever statistic is next-smallest.

The failure: `skilled armed 0 rolls total against novice's 1`, per-tier totals
novice 1, skilled 0, elite 1, ace 3. Cause confirmed by isolation, not guessed -
`Weapon::LeachBeam` in `IMPLEMENTED` makes it fail, the same line out makes it
pass, with nothing else changed. **A wider pickup pool widens the weighted walk
in `oag_weapons::pickup::draw`**, so every craft draws different pickups, so
every trajectory in the race differs, so the seven per-flight roll coin tosses
are simply re-rolled.

**The general shape, which is the part worth carrying forward.** Anything
downstream of `pickup::draw` is downstream of `IMPLEMENTED`'s *length*. A test
that measures a sparse, race-wide statistic - a count in the range 0-3, a
"did it ever happen" - is not measuring the mechanic it names; it is measuring
one particular sequence of draws. Adding a weapon is a legitimate change that
will reroll every such statistic, and the answer is a sample that survives the
reroll, not a re-baselined number.

**The maintainer's ruling, and what landed**: make the sample big enough to
assert on. The test now sums over **twelve forward circuits by two world seeds
by seven opponents** - 24 grid races and 168 flights per tier - rather than one
race on `09_Track`. `race::Options::seed` reaches `World::new`,
`oag_ai::Driver::for_slot` and `oag_ai::pilot_for_slot`, so a second seed is a
second grid of characters drawing from a second pickup stream, which is the
nuisance variable that had to be averaged out.

Measured 2026-09-08, armed totals:

| tier | seed `0x1` | seed `0x5eed0002` | total |
| --- | --- | --- | --- |
| novice | 5 | 0 | 5 |
| skilled | 13 | 8 | 21 |
| elite | 22 | 21 | 43 |
| ace | 46 | 36 | 82 |

Each seed reproduces the ordering on its own, which is the evidence the sample
is big enough rather than that it was grown until it passed. The bound is
unchanged - non-decreasing tier to tier, on the aggregate. **Two options were
rejected by the maintainer and are recorded so they are not re-proposed**:
dropping the monotonicity claim, and re-baselining the tier ordering.

**Cost**: 350 s in a debug build, against the 17 s the single-circuit version
took. `just test-data` runs debug, so that is the number that counts; it makes
this the longest single test in the suite.

## 2026-09-09: the charge is real, the detonation is found, `charge_time` is dead

**The three items above closed in one pass, and the pass was one function
long.** `get_xrefs_to Plasma_Update` resolves to two callers; the second,
`FUN_0886b490`, is the Plasma pool walker (`Plasmas_Update`, confidence 90),
and it holds everything the 2026-09-02 read had gone looking for in the fire
path. The lesson worth keeping: **the fire handler was never going to hold the
timer, because in this engine the pool walker is where a weapon's per-tick
state machine lives**. `MinePool_Update` is the Mine's equivalent and this thread
had already named it as the template - the mistake was reading it as "the
teardown" rather than "the update".

**1. The wind-up is real, one second, and hardcoded.** `Plasma_Init` sets
`entity+0x4c = 1` (charging) and `entity+0x50 = _DAT_08a7c098` = `0x3f800000`
= 1.0f. `Plasmas_Update` branches on `+0x4c`: a charging entity gets
`Plasma_UpdateCharge` (`0x0885c170`), which copies the firing craft's own
weapon-node world matrix onto the bolt every tick and scales the riding
`WO_PLASMA_HEAD` by `(1.0 - remaining) * 0.75`; `Plasma_Launch` (`0x0885bf84`)
fires it when the countdown reaches zero, reading the node matrix **fresh**, so
the shot leaves along the heading the craft has at release rather than at
press. Cross-checked from a path read for another reason: the netcode's
`Plasma_SpawnRemote` (`0x0886a920`) calls `Plasma_Init` then `Plasma_Launch`
back to back before fast-forwarding, which only makes sense if `Init` leaves
the bolt held.

**2. `charge_time` is still spent nowhere, and now that is measured rather
than "not found yet".** A calibrated sweep - every load and store at a stats
offset across all 69 functions in `psp-pulse-usa` that reach the weapon-stats
table - finds **zero** at `charge_time`'s `+0x9c` against **fourteen** at the
`venomspeed` control `+0xac`, including the known consumer. **Pure hard-codes
the same 1.0f in the same two stores** (`FUN_0885df98`), from a function whose
allocator tag is the original source path
`c:/Work/Wipeout/Code/Backend/Weapons/Plasma.cpp`. So the authored `3` is
three times the shipped wind-up in both titles.

**Both halves matter and they point opposite ways.** The from-play report was
right about the behaviour and would have been wrong about the number: a
three-second charge added to match the attribute would have tripled it. This
is the case the falsification clause exists for, and the way through it was to
find the consumer rather than to argue about the recollection.

**3. `WO_PLASMA_FLASH` is the detonation.** The walker's teardown pass calls
`Plasma_SpawnDetonation` (`0x0886ac88`) at the bolt's position;
`PlasmaBlast_Construct` (`0x0885fd90`) spawns the file with the fourcc `PLFL`
and **also loads three models** - `pulse_plasma_halo1.vex` and two
`pulse_plasma_hemisphere*.vex` - orienting them to the track. `Race::blast_for`
plays the particle system; **the three models are recorded and not drawn**, and
that is the honest partial rather than a substitute.

**Two incidental recoveries.** A bolt is reaped at a hardcoded **10.0 s**
(the walker, not an authored `timetodie`), and `Plasma_SpeedForClass`
(`0x0885c5a4`) - now decompiled, so no longer deliberately unnamed - blends the
craft's own launch speed into the class speed over the bolt's **first second**
of flight. This engine does not implement that ramp: it flies at
`class + launchspeed` throughout, which is `projectile::launch`'s shared
choice, kept so the Rocket and the Plasma cannot drift apart. Worth revisiting
only alongside the Rocket's own.

## 2026-09-16: the Plasma on HD/Fury and Omega, read from both binaries and set beside Pulse

**The 2026-09-09 findings hold on the two later ports, and the one Pulse
number the port got wrong is the one the ports changed.** Two new pages,
[ps4-omega-eu/plasma.md](../../docs/ghidra/functions/ps4-omega-eu/plasma.md)
(x86-64, read first because it decompiles clean) and
[ps3-hdfury-eu/plasma.md](../../docs/ghidra/functions/ps3-hdfury-eu/plasma.md)
(PPC, read second, constants pulled through the function family's own TOC),
24 names between them. The lesson of the 09-09 pass transfers unchanged:
the manager's vtable slot 3 is the pool walker and the walker holds the
state machine - on Omega with the flight step, the ship hit and the
detonation all inlined into one 2,400-line body, on HD as five separate
callees.

**The comparison, every cell read off its own binary:**

| | Pulse (PSP) | HD/Fury (PS3) | Omega (PS4) |
| --- | --- | --- | --- |
| wind-up | 1.0 s, hardcoded (`Plasma_Init`) | 1.0 s, hardcoded (`0x008a9f48`) | 1.0 s, hardcoded (`+0x88 = 0x3f800000`) |
| `charge_time` authored | yes, `+0x9c` | yes, `+0xbc` | yes, `+0x114` |
| `charge_time` read by | nothing (calibrated sweep) | nothing (`lfs 0xbc(r` sweep, `0xc0(r` damage as calibration) | nothing (`0x114]` in all four Plasma functions, `0x118]` damage as calibration) |
| release heading | node matrix re-read at release | same | same |
| launch speed | `|v| * 3.6 + launchspeed` | same | same |
| launch ramp | blend into class speed over first 1.0 s | same | same |
| speed classes | 4 | 4 (`+0xcc..+0xd8`) | **5**, `superphantomspeed` |
| km/h factor | `/ 3.6` | `0x3e8e2eb2 = 0.2777` (update), `3.6` (launch) | `0.2777` (update), `0.2777778` (launch) |
| surface probe | **12.0** | **6.0** | **6.0** |
| ride height | `g_ride_height` | 4.0 | 4.0 |
| open-air fall | `-surface * 50 * dt` | **world Y**, `-50` | **world Y**, `-50` (confirmed in disassembly) |
| wall = collision kinds | `0`, `4`; none = `0x7f` | `0`, `4`, `5`; none = `0x7f` | `4`, `5`, `6`, `8`; none = `0xe` |
| craft hit | (Pulse: unread this pass) | perpendicular distance in (-6, 6) along the travel segment | same, +-6.0 literals |
| damage | (unread) | `stats.damage` raw, `+0xc0` | `stats.damage * firer.handling.weapon_damage_multiplier` (`+0x118 * +0x4c4`) |
| slowdown / blast | (unread) | `+= slowdown_time`; blast loop `0x00146470` unread | `+= slowdown_time`; every craft within `blastradius` gets a `blastforce` impulse |
| timeout | 10.0 s hardcoded | 10.0 s (`DAT_008aa010`) | 10.0 s literal |
| timeout detonates? | yes, same path as a wall | yes, same path | yes, same path - **no damage, no impulse** either way; both live only in the craft-hit block |
| wall / timeout cue | `PLASMAHITWALL` | `PLASMAHITWALL` | `NGP_PlasmaHitWall` |
| craft-hit cue | (unread) | `PLASMAHITSHIP` | `NGP_PlasmaHitShip` |
| detonation psys | `PLFL` | `WO_PLASMA_LIGHTNING_EXPAND`, then `_COLLAPSE` at 1.3 s | same, HD asset root only |
| explosion models | `pulse_plasma_halo1` + 2 hemispheres | `HD_plasma_ring`/`_sphere`/`_halo` | same three (`HD_plasma_ball` is the **bolt head**, not the explosion) |
| explosion ramps | three, unread | `cur += (target - cur) * rate` per draw; targets **100 / 7.1 / 7.0**, rates 0.01 / 0.3 / 0.2, windows 1.7 / 1.3 / 1.3 s | same rates and windows; targets **50 / 2.0 / 3.0** |
| explosion lifetime | unread | 3.5 s | 3.5 s |
| explosion visible-only | - | one viewport must pass a visibility test or it never starts | same, and the slot recycles that tick |

**What this settles for the engine.** `PlasmaStats::charge` stays `1.0` and
`charge_time` stays unread - three binaries now, and on all three the
consumer search is calibrated against a sibling attribute that *is* read.
The 12.0-vs-6.0 probe that the 09-02 port bug was about is genuinely
title-specific: Pulse probes 12, both HD-lineage ports probe 6, so a
per-title `SURFACE_PROBE_LENGTH` is right and a shared one would be wrong
for one of them. Same for the fall axis: the HD engine drops a bolt along
world Y, the PSP along the carried normal, so an HD racebox would need the
other branch.

**What is not settled.** Which of Pulse's `PlasmaBlast_Construct` ramps
map onto which of HD's three (~~the PSP page never read the advance~~ -
**read 2026-09-16**, see the section below: they do not map at all in the
shape this table implies. HD/Omega's three explosion models each carry a
`(cur, target, rate)` recursive ease over a *scale*; Pulse's own advance,
`PlasmaBlast_Update`, animates a per-model anim-time value through
`Node_SetAnimTimeTree` instead - most likely scrubbing each `.vex`'s own
baked animation rather than computing a scale ease at all - and Pulse's own
`+0x90..+0x114` keyframe-curve mechanism, structurally the nearest thing
to HD's ease, ships dead in this binary. The two engines solve "the blast
grows" by different means, not by the same means at different rates); the
HD/Omega target-scale difference (100/7.1/7.0 vs 50/2/3 over identical
`.vex` names - probably a re-export scale, unverified); the 24-slot
descriptor pool both ports register lights or post-effects into
(`0x01606280`/`0x01605f50` on Omega, `FUN_00677c78`/`FUN_00677a88` on HD -
shape read, consumer not); and where Omega sets the `flags & 1` bit that
gates the craft-hit test (not in `Init` or `Launch` as decompiled).

## 2026-09-16, later: Pulse's own `PlasmaBlast_Update` read, and the three models now draw

Picking up the "Which of Pulse's `PlasmaBlast_Construct` ramps map onto
which of HD's three" item above, with a live Ghidra session against
`psp-pulse-usa`. Full evidence in
[plasma.md](../../docs/ghidra/functions/psp-pulse-usa/plasma.md#the-blast-objects-own-per-tick-animation-plasmablast_update);
two names landed (`PlasmaBlast_Update`, `Plasma_ClassSpeed`).

**The retire time is a hardcoded 1.5 s, and it is single-stage - no
HD-style collapse.** `PlasmaBlast_Update`'s own age check compares against
an immediate `0x3f800000`'s neighbour `0x3fc00000` = `1.5f` two
instructions before the branch, not a shared `DAT_` constant. HD and
Omega both retire at 3.5 s with a 1.3 s collapse partway through that hides
the models early; Pulse has no collapse step in this function at all - one
teardown call, at 1.5 s, full stop.

**The `+0x90..+0x114` keyframe mechanism `PlasmaBlast_Construct` spends
real effort precomputing rates for ships dead in the shipped binary** - the
same shape as the `charge_time` finding on this same page. Its own
keyframe count is read by `PlasmaBlast_Update` before it is ever written by
either function, so it is `0` for every constructed blast and the
interpolation loop never runs; what runs instead is a `count == 0` fallback
that reads one dword short of each model's own value array, landing on a
constant `0.07` for the halo and `0.0` for both hemispheres. Recorded as
observed, not resolved - whether that is authored (a flash confined to the
halo) or a shipped bug in a path nothing since exercised is open.

**What plays instead: a per-model anim-time scrub, most likely the
model's own baked animation.** `Node_SetAnimTimeTree(age * rate, model)` -
already a named, shared engine function - runs every tick with `rate`
`0.1`/`0.07`/`0.07` for the halo/hemisphere2/hemisphere1. Since this
project already has a working "sample this model's own `Anim Transform`
node at a given time" path (`oag_mesh::mesh::Model::sample_anim_nodes`,
wired for the PS2 boost plume's anchors), this reads as the *original*
scrubbing the same kind of baked animation this engine can already play
back - which is what the render side below does, rather than reproducing
HD's scale ease.

**One finding corrected the same pass it was written.** The camera-basis
read this page's first draft described as "look from the camera at the
blast" does not, on the actual disassembly, ever read the blast's own
position - it negates three floats read off the active camera struct and
nothing else. Which single axis that extracts (plasma.md's own hedge: it
may be one row of the camera's rotation reassembled, not any one semantic
axis) is not settled, so the render side below uses the ordinary "face the
camera" billboard as a stated substitute rather than this specific,
unresolved vector math.

**Drawn.** `crates/raceplay/src/blast_models.rs` (new module) tracks a
render-side pool of live blasts - position and age only, `RaceView` state,
never hashed - spawned from `Race::ignite_blast` on every Plasma
detonation and aged in `Race::tick`. `crates/raceplay/src/scene.rs` and
`crates/raceplay/src/scene/weapon_models.rs` load and draw
`PLASMA_BLAST_HALO_MODEL_ENTRY`/`_HEMISPHERE1_/_HEMISPHERE2_MODEL_ENTRY`
the same way the Rocket's, Mine's, Bomb's and Cannon round's own bodies
are, sized to `oag_weapons::projectile::MAX_PROJECTILES` but indexed
independently of the projectile pool since a blast outlives its bolt. One
transform serves all three models per blast (the per-model scale/offset
nudge `PlasmaBlast_Construct` reads is `0.0` for all three, measured), and
each model's own `write_node_anims` call is driven by that model's own
anim-time value - real infrastructure, no invented ease. The per-model
tint (`FUN_089122b4`) is **not** implemented: its values are constant, not
time-varying, as recovered above, and building a pipeline for a
barely-understood, possibly-dead code path would be exactly the kind of
plausible-looking invention `CLAUDE.md` forbids.

**Verified with the offscreen capture route** - `--race --give plasma
--press square`, screenshots after the bolt's own wind-up and flight;
paths and tick numbers in the report this thread's own commit history
carries. Not run against `just test-data`: this is a render-only change
that moves no simulation hash, checked directly against
`determinism_matches_the_committed_reference` and the full `oag-game`
suite (931 passed).

## Next Steps

- ~~**The Plasma's own three cues are unwired.**~~ ~~**Done 2026-09-16** - see
  that section. `PLASMA`, `~PLASMATVL` and `PLASMAHITWALL` all fire; the
  craft-hit ending plays `PLASMAHITWALL` too, chosen rather than measured,
  and the bank's own distinct `PLASMAHITSHIP` stays open for whoever reads
  the travel-segment sweep's craft-hit branch.~~ **Fully measured, same day,
  later still.** `Plasma_SweepCraftHit` (read the same day, see the section
  above) plays `PLASMAHITSHIP` and clears the bolt's own emitter before
  `PLASMAHITWALL` can also fire, so the two are mutually exclusive in the
  original. `Cue::PlasmaHitShip` is wired the same way here:
  `Impact::struck.is_some()` gets `PlasmaHitShip`, everything else gets
  `PlasmaHitWall`. No longer chosen, not measured - `plasma.md`'s own
  "chosen, not measured" language for this is gone in the same change.
- **HD/Omega Plasma, four unread pieces** (2026-09-16, detail in that
  section): where Omega sets the `flags & 1` bit gating the craft-hit test
  (not `Plasma_Init` or `Plasma_Launch`; likely the fire handler); the
  24-slot descriptor pool both ports register lights or post-effects into
  (`0x01606280`/`0x01605f50` Omega, `FUN_00677c78`/`FUN_00677a88` HD);
  HD's blast loop `0x00146470` and mine-clear `0x00145ef8`; and why the
  explosion targets are 100/7.1/7.0 on HD against 50/2/3 on Omega over
  the same three `.vex` names. Also `0x00126d80`'s `(1, 0xffff)` gate on
  HD, which is what writes two of the three explosion windows.

- ~~**Draw the two rear weapons' models.**~~ **Done 2026-09-05.**
  `MINE_MODEL_ENTRY`/`BOMB_MODEL_ENTRY` (`oag_raceplay`) load
  `Data\Weapons\Pulse_Mine.vex`/`Pulse_Bomb.vex` through the existing `.vex`
  path, drawn translation-only (a laid charge carries zero velocity - see
  `oag_weapons::projectile::mine::at_rest`). `Mine_Init`/`Mine_Construct`
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
  `crates/sound/src/sfx.rs`, wired from `crates/raceplay/src/weapons.rs`.
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
  `MinePool_Update`'s uniform teardown, reached from both the fuse timeout and
  `Mine_SweepCraftTrigger`'s trigger_radius sweep, and plays `WO_MINE_EXPLO` - confirmed
  by a direct memory read of the string, not inferred from a plausible name.
  See [mine.md](../../docs/ghidra/functions/psp-pulse-usa/mine.md#mine_spawnexplosion-plays-wo_mine_explo)
  for the full read, including the finding that fourcc `MIEX` is a shared
  "this is an explosion" tag rather than the Missile's own label -
  `Missile_SpawnExplosion` uses the identical tag for `WO_MISSILE_EXPLO`.
  `blast_for(Mine, ...)` now returns `MINE_EXPLO_EFFECT`.
- ~~**The Bomb is still open, and now the odd one out.** Every other rear/guided
  weapon that fires has a recovered detonation effect; the Bomb's own teardown
  is a distinct function from the Mine's (separate pool cursor, `+0xc4`/cap 32
  against `+0x164`/`+0x64`) and has not been read. `mine.md`'s new section
  names the two candidates - `WO_MINE_EXPLO` reused at a larger scale (which
  would match "the bomb should be static, like the mines, just a single big
  mine" extended to the visual) or its own `WO_BOMB_SMOKERING`. `MinePool_Update`
  is the template to read next: find the Bomb-pool equivalent (likely nearby,
  structurally similar) and see what it calls at teardown.~~ **Read 2026-09-15,
  not built.** `BombPool_Update` (`0x088638b8`) is that equivalent, and the
  teardown `Bomb_Detonate` (`0x088640c8`) is **neither** candidate: it builds
  `BombBlast_Construct`'s three-part blast (`explosion_hemisphere.vex`,
  `WO_BOMB_SMOKERING` at fourcc `BOSM`, `Bomb_Shockwave.vex` one unit below)
  and plays `BOMBEXPL`, or `BOMBEXPL_PC` with an `EAR_SWTNR` layer when the
  victim is the player. Also read: the fuse is `<Bomb timetodie>` counted up
  at `+0xc0` (so `Drop::bomb`'s "by analogy" is now recovered), the layer's
  exemption is `0.5 s` not forever, the blast is `damage` + `slowdown_time`
  on the tripper and a linear-falloff impulse on everyone in `blastradius`
  (`Bomb_ApplyBlast`, weapon kind `6`), `damageradius` still has no reader,
  and **a Quake wave detonates a bomb**. Building it means a `BombBlast`
  stage of two `.vex` models plus one `.pob` with an unread animator - the
  ramp constants are on the page; read the vtable at `0x08acafd8` first.
  Reusing `MINE_EXPLO_EFFECT` for the Bomb would be a stand-in.
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
  for the read and `oag_raceplay::weapons::missile_flare_anchors` for the
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
- ~~**The Cannon is the odd one left and is worth a session on its own.** Its fire
  bit `0x2000` is set by `Weapon_RequestFire` and dispatched by **nothing** in
  `Weapons_DispatchFire`. Bit `0x4000` on `world+0x58` (`0x088537ac`) is the
  obvious candidate for where it actually fires and was not chased. It is also
  the only weapon authoring `rounds` and `rate`, which is what made the Mine's
  handler look like it for months.~~ **Done 2026-09-07 and 2026-09-09** (the
  2026-09-07 sections above; `Cannon_BaseSpeedKmh` is a flat `500.0`, not a
  per-class table - re-verified by disassembly 2026-09-15, three
  instructions, `lui a0,0x43fa`). Stale row struck so nobody re-derives it.
- ~~Measure `CLUSTER` against the running original and retire the invented number.~~
  **Done 2026-09-15** - it is five; see below.
- **Re-read the five undefined `<Stats>` parsers** once the Ghidra bridge can
  read `.text` again - see Open.

## Answered 2026-09-07: Pulse's Quake is fire, **Pure's** is the concrete one

The maintainer's *"like a concrete wave"* is a real memory of a real effect -
just not Pulse's. Every colour below was parsed with `oag_pob`, not
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

- `oag_tables::weapons::QuakeStats` - the four authored attributes.
- `oag_weapons::projectile::quake::Wave` - the single travelling instance,
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

## 2026-09-15: the Mine's cluster measured live - five - and the counter's writer found

The one invented number on the Mine is retired without moving the hash: it
was already right. Measured on PPSSPP (`pulse-psp-usa.chd`, VENOM Single Race,
weapons on, audio off) with the new `scripts/psp-count-mines.py`, which breaks
on `Weapon_DropMines` every frame a cluster is out and reads the firing
record's `+0x1ac`, alongside a non-halting write watch on every record's
`+0x1ac` so PPSSPP's own stdout names the writer. Three clusters from three
different AI craft: `rounds` read **5** at the first hit of every one and
counted down to the frame the fire bit cleared; drops six frames apart (seven
once, on a `0.0002` reload residual). Nothing was written to game memory -
the raw-bit `psp-fire-weapon.py burst` path would have skipped the arm and
counted down from garbage, which is the trap the measurement had to avoid.

**The writer is `WeaponPickup_ArmMine` (`0x0886759c`, EU `0x088673f8`,
confidence 92/87)**, the function immediately before `Weapon_DropMines`, both
callers inside `WeaponPickup_Grant`: `li a0,0x5; sw a0,0x1ac(a1)` plus held
id `8` into `+0x1bc`/`+0x1c0`, fire word `&= ~0x1`, reload `0.0`. So the
count is armed **at pickup**, not at press; the first mine leaves on the press
frame (closes weapon-fire.md's "initial value of `+0x1b0`" item); and a
re-press mid-cluster is a no-op in the original, which turns
`Held::begin_drop`'s guard from chosen into recovered. The Bomb has no arm
function at all - the grant writes `9` inline and calls nothing - which is the
grant-side half of "a Bomb press lays one". The two earlier static sweeps that
reported "nothing else writes `+0x1ac`" were wrong, not incomplete: the store
is a plain `sw a0,0x1ac(a1)` and a third sweep on the relocated database lists
it. The runtime watch is what made the difference; PPSSPP's `CHK Write32 ...
PC=` log line is the whole method.

**The stretch item landed as a read, not a build**: the Bomb's pool walker,
trigger, fuse, blast and teardown are named on mine.md (seven functions,
`BombPool_Update` through `BombBlast_Construct`) and the Next Steps row above
is struck with the summary. Nothing in the engine's Bomb changed this pass;
what a build needs is written there. `MINERADAR`'s held per-projectile voice
is unchanged.

## 2026-09-16: does a timed-out plasma bolt damage anything? No - and neither does a wall hit

Closed the one item `plasma.md`'s own "not verified" list still carried:
whether `FUN_0886b898` - called per live entity in `Plasmas_Update`'s pass one,
unread until now - is the blast sweep that spends `damage`, `blastradius` and
`blastforce`, and therefore whether a bolt that times out at 10 s damages
craft near it.

**It is not, and the negative is well-evidenced, not assumed:**

1. `FUN_0886b898`'s full decompile references neither `+0xa0`
   (`damage`), `+0xa4` (`blastradius`) nor `+0xa8` (`blastforce`) anywhere.
2. It never writes `entity+0x110`, the pending-impulse slot every real blast
   function spends - checked directly against `Missile_ApplyBlastForce`
   (`0x08868ea4`, 88, `missile.md`), which does.
3. `Weapon_PostBlastImpulse` (`0x0886794c`) - the only function in the binary
   that reads a weapon's `damage`/`blastradius`/`blastforce` and posts an
   impulse - has exactly one caller in the whole executable
   (`get_xrefs_to`): `Mine_SweepCraftTrigger`, the Mine's own chain. The Plasma never
   reaches it.
4. `search_functions("Blast")` finds five names total and no
   `Plasma_ApplyBlast`-shaped one; the two `*_Construct` functions are visual
   only.
5. `Plasmas_Update`'s teardown - re-read in full, no elision on the `...` the
   page used to carry - is `Psys_Release_q`, `Plasma_SpawnDetonation`,
   `PLASMAHITWALL`, nothing else, identically for a wall hit and the
   `10.0 < age` timeout.

Full evidence: [plasma.md](../../docs/ghidra/functions/psp-pulse-usa/plasma.md#the-expiry-settles-a-damage-question-nobody-had-read).

**Ported**: `Projectiles::advance`'s Plasma-specific branch at the lifetime
timeout (`crates/weapons/src/projectile/flight.rs`) now emits
`Impact { blast: false, .. }` instead of reaping the slot silently, matching
the wall-hit branch in point, kind and owner - the same shape the Missile's
own self-detonation already uses. `Race::tick` plays the `PLASMA_BLAST_EFFECT`
for every impact regardless of `blast`, so the timeout now draws
`WO_PLASMA_FLASH` exactly as a wall hit does; `blast: false` keeps
`apply_impacts` from spending the blast. Tests: two unit tests against a hand
fixture (`crates/weapons/src/projectile/plasma/tests.rs`, mirroring the
Missile's own `an_unguided_missile_detonates_when_its_three_seconds_are_up`
pair) and one ground-truth test against a real disc
(`crates/game/tests/plasma_ground_truth.rs`,
`a_plasma_bolt_that_times_out_far_above_the_track_hurts_nobody_below_it` -
fired 10,000 units above the track so it cannot reach any real wall, since a
bolt fired down `Talons Junction`'s own racing line hit one in 1.37 s during
this pass, measured directly).

**A conflict this surfaces rather than fixes: `Projectiles::advance`'s
wall-hit branch for the Plasma still sets `blast: true`, and the same reading
says the original spends nothing there either.** That branch predates this
pass. Fixing it is a real behaviour change - a wall-hit Plasma currently
damages and pushes every craft in its `blastradius`, which the disc's own
teardown never does - with its own golden-hash and regression story, so it is
recorded as a new Open item above and left for whoever picks it up rather than
folded in here.

**What is genuinely open, not settled**: `FUN_0886b898` itself. Mechanically
it sweeps two entity lists shaped exactly like the Mine's own pool
(`+0x164`/`+0x64`) and the Bomb's own pool (`+0xc4`/`+0x44`) - both already on
this page's own pool-shape comparison - and sets the struck entity's `+0x3c`
bit `4`, the same "destroy" bit `mine.md` documents as `Mine_SpawnExplosion`'s
own trigger. Read at that strength, a flying Plasma bolt could chain-detonate
a nearby Mine or Bomb early - a third, undocumented way for either to die.
**Not claimed at that strength**: the two radii it reads (`stats+0x100`,
`stats+0xe0`) sit past the eleven offsets `WeaponStats_ParsePlasma` ever
writes, so they are unauthored for a Plasma specifically, and whether they are
a genuinely shared field or leftover memory is unread. Confidence on the
positive identity: under 50, so `FUN_0886b898` is not renamed. Reading
`WeaponStats_Parse`'s own common-attribute path, or a live breakpoint on
`stats+0x100` while a Plasma is airborne near a mine, is the next step -
nobody's task yet.

Regression gate before this change:
`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
- all twelve clean. After: unchanged (the gate does not fire a Plasma at any
craft), quoted verbatim in this pass's own commit.

## 2026-09-16, later the same day: a direct craft hit is a third ending, and it does spend a blast

The section above closed the wall and timeout endings and left the wall-hit
`blast: true` conflict open as a new Open item. This closes that item, and it
is not the fix the conflict implied - the wall-hit branch was right to be
flagged, but the fix is a **split**, not a flip: the Plasma has a genuine
third ending, a direct craft hit, that the earlier pass never enumerated
because nothing on `plasma.md` had traced the call chain to find it.

**The chain.** `Plasmas_Update`'s own `if (p->flags & 1)` call - read in the
earlier pass and prose-labelled `Plasma_NetSend_q`, a name that was never in
the database - is `Plasma_SweepCraftHit` (`0x0886afb8`, now named 82): a
per-tick hull-cylinder sweep against every craft, the same shape
`RocketPool_Update` runs for the Rocket via `FUN_0886e7ac`/`Rocket_HitCraft`.
On a hit it calls two more newly-named functions:

- `Plasma_HitCraft` (`0x0886ad60`, 88) - credits full `damage` and
  `slowdown_time` to the struck craft **unconditionally**, off
  `WeaponStats_ParsePlasma`'s own `+0xa0`/`+0xc4`. No distance test: the
  struck craft is already known.
- `Plasma_ApplyBlastForce` (`0x0886ae08`, 88) - pushes every craft but the
  bolt's own firer with a `(1 - d/blastradius) * blastforce` impulse, off
  `+0xa4`/`+0xa8` - the exact shape `Weapon_PostBlastImpulse` and
  `Missile_ApplyBlastForce` already carry. **The struck craft is not excluded
  from this sweep either** - it sits at or near the blast point and takes a
  near-full impulse on top of the direct credit above.

This corrects two things the previous section said, both for the reason
every such correction on this thread has had: `search_functions("Blast")`
cannot find a function that has not been renamed yet.

- ~~"the only function in the binary that reads a weapon's
  `damage`/`blastradius`/`blastforce` and posts an impulse"~~ - true only of
  `Weapon_PostBlastImpulse`'s own address; `Plasma_ApplyBlastForce` is a
  second, Plasma-owned copy of the same arithmetic.
- ~~"no `Plasma_ApplyBlast`-shaped function exists"~~ - `Plasma_HitCraft` is
  one; it just was not named when that line was written.

**Ported.** `flight.rs`'s two Plasma/Rocket-shared branches (the floor-probe
wall and the travel-sweep hit) now split by `kind` and, for the sweep branch,
by `struck`: a Plasma wall hit is `blast: false` on both, a Plasma craft hit
is `blast: true` routed through a new `blast::blast_direct_hit` rather than
the uniform `blast()` every other weapon still uses. `Impact::struck` and
`Impact::blast`'s own doc comments in `crates/weapons/src/projectile.rs`
carry the full reading. The Rocket is untouched throughout - see the new Open
item above for why `Rocket_HitCraft`'s identical shape is a lead, not a fix,
here.

Tests: three new unit tests in
`crates/weapons/src/projectile/plasma/tests.rs` (the `Impact` shape on a
craft hit, the struck craft's direct credit, and the bystander/firer split).
`weapon_slowdown_ground_truth.rs` needed no assertion change - it hand-detonates
through `blast()` directly rather than through a real `Impact`, so the new
routing does not reach it; its own doc comment now says so.

Regression gate:
`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
- quoted verbatim before and after in this pass's own commit.

## 2026-09-16: the Plasma's own three cues, and the per-projectile held-voice gap `MINERADAR`'s note left open is closed for the first weapon that needs it

All three of the Plasma's own recovered cues are wired: `PLASMA` at the press,
`~PLASMATVL` from release to whatever ends the bolt, `PLASMAHITWALL` at every
ending. Confirmed against a real disc: a real press on a real circuit put
`PLASMA` and `PLASMAHITWALL` 82 ticks apart (a 60-tick wind-up plus real
flight to a real wall on Talons Junction), and the mix rendered non-silent
PCM through the null backend the whole way - see
`crates/game/tests/plasma_cues_audio_ground_truth.rs`.

**Decompiling `Plasma_Init` in full is what settled the placement, and it
was not the reading this thread's own earlier passes assumed.** The bolt gets
its **own** `SoundEmitter_Init` record (`p->emitter`, `+0x5c`), pointed at the
bolt's own matrix (`p+0x70`) rather than the firing craft's node - so
`~PLASMATVL` and `PLASMAHITWALL`, both played on `p->emitter`, are heard from
the bolt, and `PLASMA`, played on the argument `Plasma_Init` is handed
directly (the craft's own emitter, before the bolt's is even constructed), is
heard from the craft. See `plasma.md`'s own updated sections for the
instruction-level evidence.

**The per-projectile held-voice gap is closed, for the Plasma.** The
`MINERADAR` bullet above names it precisely: "nothing in `CueEvent` or
`SfxVoices` can address that today; every held voice this engine plays is
keyed by grid slot". `~PLASMATVL` needed the same shape and now has it:
`SfxVoices::plasma_travel` is a `[Option<VoiceId>; MAX_PROJECTILES]`, read and
written directly off `race.sim.world.projectiles.slots` every tick in
`Audio::race_tick` rather than through a queued `CueEvent` - the same reason
`Engine` reads craft position directly. **This does not generalise
`MINERADAR` for free** - the array is Plasma-specific code, not a shared
abstraction - but the shape (an array of held voices keyed by projectile
slot, ticked directly off the world) is now proven out on real disc data and
is the one to copy rather than invent when someone next picks up `MINERADAR`
or `~BOMBRADAR`.

**`PLASMAHITWALL` also plays for a craft hit, and that is chosen, not
measured.** This engine's own Plasma can end on a craft
(`Impact::struck.is_some()`, the shared sweep-segment test in
`oag_weapons::projectile::flight`) as well as a wall or the 10 s timeout.
`plasma.md`'s own reading of `Plasmas_Update`'s teardown covers only the
latter two - `Plasma_Update`'s decompiled switch has no craft-hit case, and
the travel-segment sweep against a craft is the part of that function this
thread's own re-read left elided ("the same collision test again"). The bank
carries a **distinct** `PLASMAHITSHIP` cue (confirmed via `oag-wad sounds`,
alongside the other three, all in `weapons.bnk`) that this build does not
play for a craft hit, since nothing confirms it belongs there - reusing the
confirmed wall/timeout cue for the unconfirmed third ending is the smaller
invention of the two available, and is written down as one rather than
silently. Whoever reads the travel-segment sweep's own craft-hit branch next
either confirms `PLASMAHITWALL` there too or hands this its own cue.

**Superseded the same day, later still: the travel-segment sweep's craft-hit
branch is `Plasma_SweepCraftHit`, and it does hand the craft-hit ending its
own cue.** See ["a craft hit is the third ending"](../../docs/ghidra/functions/psp-pulse-usa/plasma.md#a-craft-hit-is-the-third-ending-and-it-does-spend-a-blast)
in `plasma.md`: the function plays `PLASMAHITSHIP` and clears the bolt's own
emitter before `Plasmas_Update`'s teardown would otherwise play
`PLASMAHITWALL`, so the original never plays both. `Cue::PlasmaHitShip` now
exists and `crates/raceplay/src/tick.rs` routes on `Impact::struck` the same
way. The paragraph above is left as the record of what was chosen before this
was read, not deleted.

A new placement had to be added for both: `Placement::Point`, carried on
`CueEvent::at_point` rather than a grid slot - the original always names an
*emitter* with a scene node, never a bare position, so nothing here is read
off a call site; it is the smallest honest way to place a sound that is not a
craft's own. `crates/sound/src/sfx.rs` and
`crates/raceplay/src/{weapons.rs,tick.rs}`.

## 2026-09-16, later again: the launch-speed ramp is ported, and a first attempt at it was wrong

Closes the `## Open` item above. `Plasma_SpeedForClass` (`0x0885c5a4`,
`plasma.md`'s "the launch ramp" section) was read 2026-09-09 and left
unimplemented: the bolt leaves at the firing craft's own speed plus
`launchSpeed` and blends to the class speed over its first second of flight,
and this engine flew at `class + launchSpeed` throughout instead, the choice
`oag_weapons::projectile::launch`'s doc comment made "so the Rocket and the
Plasma cannot drift apart."

**Ported**, in `oag_weapons::projectile::flight`'s charging branch
(`crates/weapons/src/projectile/flight.rs`): the tick the charge countdown
crosses zero, the bolt's `launch_speed_kmh` is set from the firing craft's
*current* velocity plus `stats.launch_speed`, read fresh rather than
whatever the craft was doing at the press - `Plasma_Launch` (`0x0885bf84`)
re-reads the craft's own node matrix and velocity at release for the same
reason the position is re-seated. The per-tick blend shares
[`missile::speed_kmh`](../../crates/weapons/src/projectile/missile.rs) with
the Missile rather than growing its own copy: `Missile_SpeedNow`
(`0x0885a038`) and `Plasma_SpeedForClass` each independently test `age < 1.0`
and blend with the identical operand order, `launch * (1 - age) + class *
age` - the same bar this project already used to fold the Rocket's,
Missile's and Shuriken's `12.0` surface probe into one constant. Age counts
from *release*, not from the press: the charging branch returns early every
tick of the wind-up, so `lifetime` (and the age derived from it) does not
move until the bolt is actually flying. **No launch floor carried over** -
`Plasma_Launch`'s listing has no `vmax_s` clamping a minimum, unlike
`Missile_Init`'s `14.4` km/h floor, so a standing-start bolt leaves at
`launchSpeed` alone. The Rocket is untouched and is now the odd one out
rather than a shared choice - `rocket.rs`'s own doc comment says so.

**A first attempt at this was wrong, and a real disc caught it, not a unit
test.** The release computation was gated on the weapon table being present
(`plasma: Option<&PlasmaStats>` being `Some`) but not on `projectile.charge
<= 0.0`, so it re-read the craft's velocity and overwrote the bolt's speed on
*every* charging tick rather than only the release tick. Every hand-fixture
unit test held the craft's velocity constant through the whole charge, so
recomputing a constant every tick looks identical to computing it once - the
bug was invisible to all of them.
`crates/game/tests/plasma_ground_truth.rs`'s
`a_plasma_fired_on_a_real_track_is_one_bolt_and_it_flies`, fired one tick
into an accelerating real race, caught it immediately: the bolt's speed one
tick after the press read the craft's speed *at that tick*, not the
charge-hold constant the assertion expected. Fixed by gating the release
block on `projectile.charge <= 0.0` as well.

**Tests.** Two new unit tests in
`crates/weapons/src/projectile/plasma/tests.rs`:
`the_launch_speed_reads_the_crafts_velocity_at_release_not_at_the_press`
(release formula, isolated) and
`a_flying_plasma_bolts_speed_blends_from_launch_to_class_over_one_second`
(the ramp through real flight, over a floor). Both were verified to actually
discriminate - deliberately breaking the release-time gate reproduces the
exact 156.3-vs-291.7 km/h mismatch the ground-truth run first surfaced, and
both new unit tests fail the same way, before the fix restores them.
`crates/game/tests/plasma_ground_truth.rs`'s
`a_plasma_fired_on_a_real_track_is_one_bolt_and_it_flies` is extended rather
than reassigned: its existing charge-hold-magnitude assertion (fired one tick
after the press, before any release can happen) is untouched and still
passes, now with a comment saying why it is unaffected by the ramp. The
"rides the track" loop widens from 60 to 240 ticks and now also measures,
against the disc's own table and never a hardcoded value: the release-time
formula (craft speed that tick plus `launchSpeed`, against `launch_speed_kmh`
- exactly the assertion that would have caught the bug above on real data),
the mid-ramp speed (strictly between launch and class, ~0.5s after release),
and the post-ramp speed (at the class speed, ~1.17s after release). All three
landed on this run: released tick 60, craft doing 431.2 km/h, bolt's own
launch speed 631.2 km/h (`launchSpeed=200`); detonated tick 131, so both the
mid and the post-ramp samples were captured before it hit the wall.

**No committed golden moved.** Both hash references in
`crates/gameplay/tests/determinism.rs` (`REFERENCE`, `REFERENCE_VOLLEY`) are
Rocket-only scenarios by their own doc comments and both still pass
unchanged - checked directly rather than assumed. No regen commit.

Regression gate,
`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`,
quoted verbatim before and after - byte-identical, all twelve circuits clean,
zero regressions, 01 still the only circuit with a respawn (one, standing-start
lap):

```
16_Track     clean lap  42.1s  laps 4   respawns 0   of 3448 lost at []
03_Track     clean lap  43.8s  laps 4   respawns 0   of 3560 lost at []
02_Track     clean lap  43.5s  laps 4   respawns 0   of 3112 lost at []
10_Track     clean lap  38.4s  laps 4   respawns 0   of 3376 lost at []
05_Track     clean lap  39.6s  laps 4   respawns 0   of 2976 lost at []
04_Track     clean lap  39.3s  laps 4   respawns 0   of 3344 lost at []
09_Track     clean lap  51.5s  laps 4   respawns 0   of 4016 lost at []
14_Track     clean lap  45.8s  laps 4   respawns 0   of 3684 lost at []
01_Track     clean lap  43.6s  laps 4   respawns 1   of 2836 lost at [794]
13_Track     clean lap  38.1s  laps 4   respawns 0   of 3084 lost at []
06_Track     clean lap  46.8s  laps 4   respawns 0   of 2852 lost at []
07_Track     clean lap  50.7s  laps 4   respawns 0   of 3216 lost at []
clean laps: ["16_Track", "03_Track", "02_Track", "10_Track", "05_Track", "04_Track", "09_Track", "14_Track", "01_Track", "13_Track", "06_Track", "07_Track"]
no clean lap: []
```

Expected: this scenario never fires a Plasma (solo Time Trial-shaped run, no
pickups), so byte-identical output is the predicted result, not a surprise -
recorded anyway, per this thread's own standing instruction to quote it
verbatim rather than summarise it as "unaffected."


## 2026-09-17: the LeachBeam ribbon's own body, drawn

~~Still open: the beam's own ribbon draws nothing, its geometry recovered but
its texture unlocated.~~ **Found and built.** The texture is
`Data\Weapons\Textures\pulse_leechbeam1_ADD.mip` (string at `0x08a7cc60`) -
under the Cannon's own `Data\Weapons\Textures\` directory, not the two
`Data\Tex\Weapons\` hull-overlay strings the earlier `(?i)leach` search
found; the disc spells this one "lee**ch**", which is why that search missed
it. `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`'s
"2026-09-17: the LeachBeam ribbon's own texture" section has the full
evidence chain: `LeachBeam_LoadTexture` (`0x088730d4`) loads it,
`LeachBeam_Construct` (`0x08872aa0`) is the only caller, and the loaded
handle is read by exactly one function, `LeachBeam_BuildStrip`
(`0x088739b0`) - not called by any `jal`, because it is a virtual draw method
at slot `0x44` of the vtable `LeachBeam_Construct` installs (`read_memory` on
`DAT_08acb048` finds the address sitting there directly). `LeachBeam_BuildStrip`
builds the strip and calls `LeachBeam_SubmitStrip` (`0x088731c4`), which
issues the actual `GU_TRIANGLE_STRIP` draw with the same blend
`exhaust::BLEND`'s own doc comment already cites
(`Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0, 0xffffff)`).

**Built in `oag_fx::beam`** (geometry half, wgpu-free, eleven unit
tests; `Pipeline` half modelled on `exhaust::Pipeline` but far smaller - one
texture, one blend, no trail/HD complexity), loaded in
`race::assets::leach_beam_texture` on the same "absence is reported, not
fatal" terms every optional asset here follows, and wired into
`Scene`/`RaceView`/`Race::advance_leach_beam_ribbon` the same way the
exhaust ribbon and its own `Rng` are. Verified as a player would: headless
`--race --mode single_race --give leachbeam --hold cross --press square`
captures at several tick counts show a locked beam's ribbon as a bright
additive streak between the two craft, brightest at its middle and fading to
nothing at both ends - exactly the recovered endpoint-alpha-zero taper.
Screenshots and exact commands are in the lane report,
`/home/topaxi/.cache/oag/drive/reports/leachbeam-ribbon.md`.

**Chosen, not recovered**: the two axes the crossed double-strip displaces
along (this engine's `Ship` carries no per-craft node basis the way the
original's scene graph does) and the amplitude bucket's re-roll cadence (the
original ties it to a ribbon-scroll cursor `oag_weapons::projectile::leach_beam`
already declines to model, for the reason given there). Everything else -
segment count, half-width, amplitude range and bucket span, base colour, the
disconnect fade, the endpoint taper, the crossed-strip structure and the
blend - is a recovered constant or a direct read.

**Not chased this pass**: whether the ribbon feeds the bloom glow mask
(`Gu_Enable(3)`/`Gu_StencilOp`/`g_bloom` inside `LeachBeam_SubmitStrip`
suggest it does, the way `Trail_BuildStateList` does for the exhaust
ribbon), the two other vtable slots beside `LeachBeam_BuildStrip`
(`0x08872c78`, `0x08872b68`), and the hull overlay pair
(`leachbeam_surface.mip`/`absorb_surface.mip`, drawn by `FUN_0890d828` ->
`FUN_0890e304`) - still undrawn, still honest absences.

Gate: full `just` and `OAG_REQUIRE_GAME_DATA=1 just test-data`, both watched
to completion - see the lane report for exact counts.

## 2026-09-17, later: the hull overlay pair, read in full

Prompted by a from-play report: the weapon-absorb effect does not appear to
play in any title here (only its `ABSORB` sound cue does). Reading
`leachbeam_surface.mip`'s draw path in full turned out to mean reading
`absorb_surface.mip`'s too - the two share every function but the one
texture constant.

**Four thin wrappers, one shared draw call.** `HullOverlay_DrawLeachBeam`
(`0x0890d828`) / `HullOverlay_DrawAbsorb` (`0x0890d744`) each walk one
entity's own sub-mesh chain; `HullOverlay_SubmitLeachBeamBatched`
(`0x0890e140`) / `HullOverlay_SubmitAbsorbBatched` (`0x0890e120`) are the
same pair's per-batch variant (confirmed at the disassembly level - each is
a bare tail call into `HullOverlay_Submit` with only the texture register
set, everything else already staged by its own caller). All four funnel
into `HullOverlay_Submit` (`0x0890e304`), which is the entire drawn picture
for both.

**The blend was findable, and it isn't a different equation.**
`HullOverlay_Submit` calls `Gfx_BuildBatchStateList` with a literal `0x282`
flag word rather than a direct `Gu_BlendFunc`; walking that function with
`0x282` (reusing `mesh-draw.md`'s own already-pinned GU state-index table
rather than re-deriving it) gives `GU_BLEND` on, additive -
`Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0, 0xffffff)`, the **same**
equation `exhaust::BLEND` and the LeachBeam ribbon's own
`LeachBeam_SubmitStrip` already use. Also on: `GU_CULL_FACE`, depth write,
`GU_ALPHA_TEST`/`GU_COLOR_TEST` (both cutout-shaped - discard transparent or
pure-black texels) and `GU_STENCIL_TEST` at `ALWAYS`/`REPLACE` - the same
shape `LeachBeam_SubmitStrip` uses to feed the bloom glow mask, so **both
hull overlays write the glow mask too**.

**The absorb half's timing is fully measured: a one-second, symmetric
triangle pulse.** `HullOverlay_AbsorbWindowActive`/`HullOverlay_AbsorbFade`
(`0x0883e904`/`0x0883e950`) both read the same `state+0x830 - state+0x878`
elapsed time against a `[0.0, 1.0]` second window; `HullOverlay_Submit`'s own
`param_1 >= 1.0 -> 2.0 - param_1` fold turns that into fade-in over the
first half-second and fade-out over the second.

**Still open**: what the shared `state` object actually is (both dispatch
sites - the per-entity one at `entity+0x74`, the per-batch one at a
batch-scoped `+0xc0` - read from the same kind of pointer, but its type,
size and writer were not found), and what the LeachBeam side's own fade
source is (`**(float**)(state+0x4c)`, a double pointer dereference into
some live float, plausibly the active beam's own age or
`LeachBeam_PulseStrength`, not confirmed by a write site). Neither blocks a
first build of the absorb half specifically, which this engine can trigger
off its own existing shield-hit event rather than the unidentified timer
pair - chosen, not measured, the same split every other wired-up trigger in
this codebase draws.

**Not built this pass** - the ask was a complete read so a follow-up lane
can build from it, and a finished read is worth exactly as much as a build
when the picture (geometry, colour, blend, UV mode, timing) is this settled.
Seven renames landed:
`HullOverlay_DrawLeachBeam`/`HullOverlay_DrawAbsorb`/`HullOverlay_Submit`/
`HullOverlay_SubmitLeachBeamBatched`/`HullOverlay_SubmitAbsorbBatched`/
`HullOverlay_AbsorbWindowActive`/`HullOverlay_AbsorbFade`, all in
`names.tsv` against `cannon-quake-leachbeam.md`'s new section.

Regression check: `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
re-run against the same seed after `git worktree add` off post-merge `main`
(this pass touched only docs and Ghidra's own database, no code) -
byte-identical to the ribbon lane's own baseline: all twelve clean,
`01_Track` one respawn at `[794]`, `07_Track` one respawn at `[687]`,
nothing else moved.

## 2026-09-24: the Quake ripples the road - read, measured live, drawn

`Quake_UpdateSpan`'s vertex block read at instruction level (the Allegrex
module's VFPU prefix decode is what makes it legible): a raised-cosine bump,
`H = amp * (1 + cos(pi d / w)) / 2`, peak 12.0 at 0.3 s falling linearly to 0
at 5.0 s (`Quake_SpanAmplitude`), half-width `25 + 10 age`
(`Quake_SpanHalfWidth`), along `-lerp(A, B, p)` - the span's authored "down"
axes, so up on level road. The span table is the track file's own `Quake`
node (`0x3c7`), loaded whole by `QuakeNode_Load`; every record is one whole GE
batch of a `Mesh` or pad node, on all 24 circuit files. Eight names landed.

A PPSSPP fork measured it: every vertex of every armed span matched to 0
shorts over 290 frames, **once shared batches were summed** - at a path
junction one batch belongs to two or three records, and they add. And a
stationary craft sat on a vertex that rose 10.8 units without its body
rising: **render-only, measured**, confidence 92.

Built: `oag_vex::quake` (the table), `oag_mesh::mesh::batch_placements` (batch
header to model vertices), `oag_render::ripple` (the bump, summing owners),
`oag_raceplay::SpanPlaces` (every vertex onto the course through its path's
own `t`, now `SplinePoint::progress`), and a `Scene::write_road` step before
the weapon pads' tint, which adds the displacement back. Ground truth:
`crates/vex/tests/quake_ground_truth.rs` and
`crates/game/tests/quake_ripple_ground_truth.rs` (vertices median 0.37 units
from where the course locates them; every seam, driven and branch, 0.00 at
p99 and 2.34 units at worst).

**The trap worth writing down**: mapping each span linearly between its own
two ends looked right on residuals (median 0.5) and left **26-unit steps at
seams** inside long curved spans - the bump would have torn at every one. The
fix is to send every vertex through the same function of its path's `t`,
which makes two spans meeting at one `t` meet at one distance. A seam check is
the test that catches this; a residual check is not. The same trap came back
on the split branches (38-unit steps on `07_Track` when each branch span was
placed from whichever neighbour reached it first, 18 at `05_Track`'s merge
when only the fork anchored it) and the same fix closed it: one `t` function
per branch, pinned at both ends.

**A second trap, in `scripts/psp-fire-weapon.py`**: `--index 0` is not the
player in a single race (record 7 of a full grid, measured). Its help text
said otherwise; it no longer does.

Regression check, before and after, byte-identical: all twelve circuits
clean, `01_Track` one respawn at `[794]` of 2836, `07_Track` one at `[687]`.

## From the HANDOVER.md index (moved 2026-09-25)

**2026-09-09: the Plasma's charge and detonation both closed, and the two findings point opposite ways.** The weapon *does* wind up before it fires - one second, held on the firing craft's nose, from `Plasma_Init`'s `+0x4c`/`+0x50` spent by the pool walker `Plasmas_Update` (`0x0886b490`) - so the from-play oracle was right for the third time. And `<Plasma charge_time>` is **still** read by nothing: the duration is a hardcoded `1.0f` in `.rodata`, not the authored `3`, and **Pure hard-codes the same second in the same two stores**. A three-second charge added to match the attribute would have tripled it. The negative is measured rather than assumed: across all 69 functions that reach the weapon-stats table, every access at `charge_time`'s `+0x9c` comes to zero against fourteen at the `venomspeed` control offset - **run the control offset or the negative is worthless**. `WO_PLASMA_FLASH` is the detonation, off `Plasma_SpawnDetonation` in the same walker's teardown. **The general lesson: in this engine a weapon's per-tick state machine lives in its pool walker, not in its fire handler** - the earlier pass read the whole press-to-flight path and the charge was never going to be in it. **the LeachBeam landed 2026-09-08, so every weapon a player can get now does something** and only the deferred Repulser is out. Two findings worth the row. First, **the reason the previous pass could not build it was a wrong address, not a hard function**: the page's two drain-rate functions were rebased `0x4000` low (`0x08804000 + 0x6eedc` is `0x08872edc`, not `0x0886eedc`) and the stated addresses decompile *cleanly into an unrelated function*, which is why it survived - the second such slip on that one page. Second, the item that page called "the single largest remaining gap" - what consumes `entity+0x120`/`+0x128` - was two functions away from one already in `names.tsv`: `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) and its four-line neighbour `FUN_0883f228`, both called from `FUN_0883f540`, which nobody had decompiled. **When a field's consumer is missing, decompile the caller of the functions you already know.** ~~Still open: the beam's own ribbon draws nothing, its geometry recovered but its texture unlocated~~ **Built 2026-09-16/17.** `slowShipFactor` (the one-shot thrust scale at `craft+0x31c`) wired 2026-09-16 through `oag_physics::Environment::thrust_scale`; the ribbon's own texture (`Data\Weapons\Textures\pulse_leechbeam1_ADD.mip`, spelled "leech" on disc) and its draw call (`LeachBeam_BuildStrip`/`LeachBeam_SubmitStrip`) found 2026-09-17 and built in `oag_fx::beam` - see `cannon-quake-leachbeam.md`'s "2026-09-17: the LeachBeam ribbon's own texture" section. **The hull overlay pair read in full the same day, prompted by a from-play report that the absorb effect never plays**: `HullOverlay_DrawLeachBeam`/`HullOverlay_DrawAbsorb` and their two batched siblings all funnel into one shared `HullOverlay_Submit`, additively blended (`exhaust::BLEND`'s own equation, decoded from `Gfx_BuildBatchStateList`'s `0x282` flag word) with a rotating `GU_TEXTURE_MATRIX` UV projection and a grey-alpha tint; the absorb half's timing is fully measured (a 0.5s-in/0.5s-out triangle over one second) but the shared trigger object and the LeachBeam side's own fade source are not - see the docs page's own section for the full read; the absorb half built 2026-09-23 (`oag_fx::hull_overlay`, on since a live PPSSPP probe the same day), the LeachBeam half's fade source read statically but not wired. **2026-09-24: the Quake ripples the road** (render-only, matched to the short on PPSSPP; the hit is still ours - see the thread's Open). **A separate defect found in the landed Quake: the wave never retires**, so a Quake can be fired once per race and then circles the ring hitting everyone every lap; see the thread for the mechanism and why the fix is a chosen lifetime rather than a measured one.
