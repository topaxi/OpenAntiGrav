# Pure's weapon table, its Disruptor and its fuse-less Bomb

Reads the three things about Pure's weapons that Pulse's pages cannot answer
for it: the `Data\XML\weaponstats.xml` parser and the struct it fills (ten
weapons, not thirteen; a `Disruptor` Pulse never had; a `Bomb` with no fuse),
the whole Disruptor from grant to victim, and the Bomb's pool. Every reading is
on `psp-pure-usa` at decompiler level with `get_xrefs_to` working (the
2026-09-07 relocation fix; nothing here needed the `lui`/`addiu` workaround of
[`psp-pure-eu/string-anchors.md`](../psp-pure-eu/string-anchors.md)).

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pure USA, UCUS-98612), image base `0x08804000` |
| **Related** | [`psp-pulse-usa/weapon-fire.md`](../psp-pulse-usa/weapon-fire.md), [`psp-pulse-usa/mine.md`](../psp-pulse-usa/mine.md) (Pulse's dispatch and Bomb reads, the structural template), [`../../../formats/weapon-stats.md`](../../../formats/weapon-stats.md) (the schema, with this page's Pure dialect section), [`psp-pure-eu/weapons.md`](../psp-pure-eu/weapons.md) (the same functions on the EU pressing) |

Confidence follows the [rubric](../../../reverse-engineering/confidence-rubric.md).
Everything here is decompilation-level reading, so 84 is the ceiling; the
rows at 84 are the ones with a unique string anchor **and** a call-graph
neighbour that agrees with the reading. Names went into
[`names.tsv`](names.tsv) in the same change.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x088087c4` | `WeaponStats_Load` | 80 |
| `0x088096d8` | `WeaponStats_Parse` | 84 |
| `0x088087e0` | `WeaponStats_ParseRocket` | 84 |
| `0x088089cc` | `WeaponStats_ParseMissile` | 84 |
| `0x08808be4` | `WeaponStats_ParseQuake` | 84 |
| `0x08809d9c` | `WeaponStats_ParseDisruptor` | 84 |
| `0x08808d44` | `WeaponStats_ParseTurbo` | 84 |
| `0x08808e44` | `WeaponStats_ParseShield` | 84 |
| `0x08808f44` | `WeaponStats_ParseAutopilot` | 84 |
| `0x08809044` | `WeaponStats_ParsePlasma` | 84 |
| `0x08809230` | `WeaponStats_ParseBomb` | 84 |
| `0x0880941c` | `WeaponStats_ParseMine` | 84 |
| `0x08809608` | `WeaponStats_ParseGlobal` | 84 |
| `0x0880a500` | `WeaponStats_ParsePickupOdds` | 84 |
| `0x08b177a0` | `WeaponStats_Table` (data) | 84 |
| `0x0884d19c` | `WeaponPickup_Grant` | 80 |
| `0x0884f9ec` | `WeaponPickup_ArmDisruptor` | 80 |
| `0x0884fa10` | `Disruptor_RollEffect` | 82 |
| `0x0884fb18` | `Disruptor_PrimeStall` | 80 |
| `0x0884fb30` | `Disruptor_PrimeMirror` | 80 |
| `0x0884fb4c` | `Disruptor_PrimeNoAirbrakes` | 80 |
| `0x0884fb68` | `Disruptor_PrimeAutopilotSlow` | 80 |
| `0x0884fb8c` | `Disruptor_PrimeAutopilotFast` | 80 |
| `0x0884fbb0` | `Disruptor_PrimeDrunk` | 80 |
| `0x0884fbcc` | `Disruptor_PrimeDrunkCamera` | 80 |
| `0x0884fbf0` | `Disruptor_PrimeRubberShip` | 80 |
| `0x0884fc14` | `DisruptorPool_Fire` | 82 |
| `0x08859010` | `Disruptor_Init` | 84 |
| `0x0885930c` | `Disruptor_Update` | 82 |
| `0x088592d8` | `Disruptor_SpeedForClass` | 82 |
| `0x088506c4` | `DisruptorPool_Update` | 82 |
| `0x08850274` | `Disruptor_TestHit` | 80 |
| `0x08850ec8` | `Disruptor_ApplyEffect` | 84 |
| `0x0892935c` | `Ship_UpdateWeapons` | 78 |
| `0x0892f1f0` | `Ship_UpdateEngine` | 74 (was 62, by fuzzy match alone; raised here from its own body) |
| `0x0892edfc` | `Ship_ApplySteeringTorque` | 72 |
| `0x088580d8` | `Bomb_Init` | 84 |
| `0x088583d8` | `Bomb_UpdateSpin` | 78 |
| `0x0884e968` | `BombPool_Update` | 80 |
| `0x0884ef78` | `Bomb_UpdateTrigger` | 82 |
| `0x0884f420` | `Bomb_ApplyHit` | 82 |
| `0x0884f214` | `BombPool_Detonate` | 82 |

Two functions read but **not** named, because a slice of each was read and
not the whole: `FUN_0892d214` (zeroes both airbrake inputs under the No
Airbrakes bit, and copies the frame's `dt` and input record onto the body) and
`FUN_0892dea8` (the hover-force pass that scales one damping term by `0.2`
under the Rubber Ship bit). Both are cited by address below.

## The parser is one function per weapon, and Pure has ten of them

`SystemRoot_Create`'s Pure counterpart (`FUN_088990f4`) calls
`WeaponStats_Load(&WeaponStats_Table)` with `0x08b177a0`, right before it
loads `Data\XML\HandlingStats.xml` - a 7-instruction thunk onto
`WeaponStats_Parse`. That function opens `Data\XML\weaponstats.xml`
(`0x08a445a0`), walks `<Weapon type=...>` and dispatches on the class-name
strings at `0x08a445d0..0x08a44628` - **`Rocket`, `Missile`, `Quake`,
`Disruptor`, `Turbo`, `Shield`, `Autopilot`, `Plasma`, `Bomb`, `Mine`,
`Global`** - one parser each. No `Cannon`, `LeachBeam`, `Repulser` or
`Shuriken` string exists anywhere in the binary, and no `DisturberOdds`
either: those are Pulse's additions, not Pure's omissions. Ten weapons agrees
with `oag_pure::hud`'s ten icons and with the ten `absorb` values the table
authors.

`<Pickupodds class=...>` is matched against **`Vector`, `Venom`, `Flash`,
`Rapier`, `Phantom`** and stores 0..4 in `table+0x470`; every odds array is
five slots deep and the totals loop runs `while (i < 5)`. That is the
opposite of Pulse's parser, which tests `Vector` and discards the result
([`../../../formats/weapon-stats.md`](../../../formats/weapon-stats.md#the-fifth-class-the-code-knows-the-name-the-data-does-not-use-it)),
and the shipped file authors all five blocks. It is evidence about the
*weapon table's* class axis, not a claim that Vector is a playable rung.

### The struct, offset by offset

`WeaponStats_Table` at `0x08b177a0`, every field a float, in parser order:

| Offset | Field | Offset | Field |
| --- | --- | --- | --- |
| `+0x00` | Global `slowdown_limit` | `+0x58` | Disruptor `Stall.time` |
| `+0x04` | Rocket `damage` | `+0x5c` | Disruptor `Mirror Left Right.time` |
| `+0x08` | Rocket `slowdown_time` | `+0x60` | Disruptor `No Airbrakes.time` |
| `+0x0c` | Rocket `speed` | `+0x64` | Disruptor `Autopilot Slow.time` |
| `+0x10` | Rocket `blastradius` | `+0x68` | Disruptor `Autopilot Slow.speed_percent` |
| `+0x14` | Rocket `blastforce` | `+0x6c` | Disruptor `Autopilot Fast.time` |
| `+0x18` | Rocket `absorb` | `+0x70` | Disruptor `Autopilot Fast.speed_percent` |
| `+0x1c` | Rocket `spread` | `+0x74` | Disruptor `HUD Flicker.time` |
| `+0x20` | Missile `damage` | `+0x78` | Disruptor `Drunk.time` |
| `+0x24` | Missile `speed` | `+0x7c` | Disruptor `Drunk.amount` |
| `+0x28` | Missile `blastradius` | `+0x80` | Disruptor `Rubber Ship.amount` |
| `+0x2c` | Missile `blastforce` | `+0x84` | Disruptor `Rubber Ship.time` |
| `+0x30` | Missile `lock_min_dist` | `+0x88` | Disruptor `Drunk Camera.amount` |
| `+0x34` | Missile `lock_max_dist` | `+0x8c` | Disruptor `Drunk Camera.time` |
| `+0x38` | Missile `absorb` | `+0x90` | Disruptor `Trippy.time` |
| `+0x3c` | Missile `slowdown_time` | `+0x94`/`+0x98` | Turbo `time`/`absorb` |
| `+0x40` | Quake `damage` | `+0x9c`/`+0xa0` | Shield `time`/`absorb` |
| `+0x44` | Quake `radius` | `+0xa4`/`+0xa8` | Autopilot `time`/`absorb` |
| `+0x48` | Quake `slowdown_time` | `+0xac`..`+0xc4` | Plasma `charge_time damage blastradius blastforce speed absorb slowdown_time` |
| `+0x4c` | Quake `absorb` | `+0xc8`..`+0xe0` | Bomb `damage damageradius blastradius blastforce absorb slowdown_time trigger_radius` |
| `+0x50` | Disruptor `absorb` | `+0xe4`..`+0xfc` | Mine `damage blastradius blastforce timetodie absorb slowdown_time trigger_radius` |
| `+0x54` | Disruptor `speed` | `+0x100`..`+0x41c` | `<Pickupodds>`: per weapon, four columns of five classes |

The order is the parser's `if` chain, not the file's attribute order, and it
is **not** Pulse's layout - Pure keeps the Turbo/Shield/Autopilot pairs as
`time` then `absorb`, Pulse the other way round, and Pulse's Quake sits at
`+0x60` where Pure's Disruptor does. The two struct layouts have nothing to do
with each other beyond `slowdown_limit` at `+0x00`.

The weapon **id** the craft carries at `craft+0x1d8` is a third order again,
recovered off `Ship_UpdateWeapons`'s absorb switch, which reads one `absorb`
per case straight out of this table: **1 Rocket (`+0x18`), 2 Missile
(`+0x38`), 3 Quake (`+0x4c`), 4 Disruptor (`+0x50`), 5 Turbo (`+0x98`), 6
Shield (`+0xa0`), 7 Autopilot (`+0xa8`), 8 Plasma (`+0xc0`), 9 Mine (`+0xf4`),
10 Bomb (`+0xd8`)**. Ten cases, ten offsets, every one landing on the `absorb`
its parser wrote - which is also what pins the table base at `0x08b177a0`
from a consumer rather than only from the load call.

### Four authored `<Effect>` blocks have no parser branch

`WeaponStats_ParseDisruptor` reads `<Stats absorb speed>` and then walks
`<Effect type=...>` children, matching **ten** names - `No Airbrakes`,
`Stall`, `Mirror Left Right`, `Autopilot Slow`, `Autopilot Fast`,
`HUD Flicker`, `Drunk`, `Rubber Ship`, `Drunk Camera`, `Trippy` - and reading
each one's `<EffStats time|amount|speed_percent>` into the slots above. The
shipped file authors **fourteen**. `Fire Weapon`, `Turbo Now`, `Steal Weapon`
and `Adjust Gravity` appear in no parser branch and as no string anywhere in
the executable (two regex sweeps of the string table, plus the contiguous
parser-string run at `0x08a445a0..0x08a44734` read byte for byte). They are
authored and dead. Confidence 90 on the absence: a string sweep is exhaustive
in a way a decompile is not.

## The Disruptor, grant to victim

### Grant: one of eight, uniformly, rolled when the pad hands it over

`WeaponPickup_Grant` (`0x0884d19c`) is the Pure counterpart of Pulse's
`0x08861d20`, same shape: a human draws `human + front * f + back * (1 - f)`
with `f = (place - 1) / field`, an AI draws `ai` flat, `rand() % total` walks
the cumulative weights, and a draw equal to the craft's last grant
(`craft+0x23c`) rolls again. Its Disruptor arm calls
`WeaponPickup_ArmDisruptor` (`0x0884f9ec`), which writes weapon id **4** into
`craft+0x1d8` and `craft+0x1dc` and calls `Disruptor_RollEffect`
(`0x0884fa10`): **`rand() & 7`** (sign-corrected, so it is C's `rand() % 8`)
into an eight-way switch onto the eight `Disruptor_Prime*` functions, each of
which writes the effect kind into `craft+0x19c`, its `time` into `craft+0x1a0`
and its `amount`/`speed_percent` into `craft+0x1a4`/`craft+0x1a8`:

| Kind | Primed by | Reads |
| --- | --- | --- |
| 0 | `Disruptor_PrimeStall` | `+0x58` |
| 1 | `Disruptor_PrimeMirror` | `+0x5c` |
| 2 | `Disruptor_PrimeNoAirbrakes` | `+0x60` |
| 3 | `Disruptor_PrimeAutopilotSlow` | `+0x64`, `+0x68` |
| 4 | `Disruptor_PrimeAutopilotFast` | `+0x6c`, `+0x70` |
| 5 | `Disruptor_PrimeDrunk` | `+0x78` |
| 6 | `Disruptor_PrimeDrunkCamera` | `+0x8c`, `+0x88` |
| 7 | `Disruptor_PrimeRubberShip` | `+0x84`, `+0x80` |

So the effect is decided **at grant, not at hit**, it is uniform over eight,
and **`HUD Flicker` and `Trippy` are parsed but never rolled** - their two
slots have a writer and no reader. That is the second dead layer under the
four dead blocks above.

### Fire: a floor-following bolt that homes if the craft had a lock

`Ship_UpdateWeapons` (`0x0892935c`) fires whatever `craft+0x1d8` holds
through `FUN_0884e3a8(craft, matrix, target, -1)`, whose Disruptor arm is
`DisruptorPool_Fire` (`0x0884fc14`, `WO_DISRUPTOR_LAUNCH` anchor). The pool
holds **48** bolts (`pool+0x120 < 0x30`). It calls `Disruptor_Init`
(`0x08859010`; `WO_DISRUPTOR_HEAD`, `"DISRUPTOR"`, `"~DISRUPTORTVL"` and the
`Disruptor.cpp` allocation site all in the body) with the craft's matrix, the
owner, the craft's **lock target** (`craft+0x194`) into `bolt+0x110`, and the
**primed kind** (`craft+0x19c`) into `bolt+0x114`, then clears `craft+0x1d8`.

`Disruptor_Init` seeds the velocity as `forward * 500.0 / 3.6` - a literal
500 km/h, not the authored `speed`. `Disruptor_Update` (`0x0885930c`) then
each tick:

1. `age += dt`.
2. Sweeps `position -> position + velocity * dt`; a hit whose collider's
   `+0x6c` is zero (a wall) sets bits `0x14` (dead, wall); any other hit
   pushes out along the normal by `4.0`.
3. Probes `position -> position - surface * 12.0`. Nothing: `velocity.y -=
   50.0 * dt`. A floor: aims at `hit + normal * 6.0` and rescales to
   `Disruptor_SpeedForClass() / 3.6`, where `Disruptor_SpeedForClass`
   (`0x088592d8`) is **`speed + 80.0 * class_index`** - the one weapon on the
   disc whose single authored speed *is* made per-class, by the code rather
   than the file. `class_index` is `DAT_08b173e0`, 0..4.
4. If `bolt+0x110` is set, blends the heading toward the target by `1.0 * dt`
   (normalised) and rescales to the same speed - so a Disruptor fired with a
   lock **homes**, gently.

`DisruptorPool_Update` (`0x088506c4`) runs `Disruptor_Update` then
`Disruptor_TestHit` (`0x08850274`) per live bolt, and reaps any with `age >
10.0` or the dead bit, spawning `DisruptorExplo` (`FUN_0885a2c8`) and playing
`DISRUPTOREXPWAL` (bit `0x10`) or `DISRUPTOREXPSHP` (bit `0x20`).

`Disruptor_TestHit` is a swept-cylinder test against every craft but the
owner: the craft's centre must project within `[-1.0, +1.0]` of the tick's
segment ends and lie within **`6.0`** of the segment line; on a hit it sets
bits `0x24` and calls `Disruptor_ApplyEffect(pool, 0, craft, bolt)`. Both
literals are hardcoded; nothing here reads the table.

### Hit: one flag, one timer, one switch on the craft, another on the ship

`Disruptor_ApplyEffect` (`0x08850ec8`) refuses if the victim is already
disrupted (`craft+0x134 == 1`) or **shielded** (`craft+0x178 == 1`). Otherwise
it spawns the `DisruptorFx` on the ship (`FUN_08924428` -> `FUN_08859b78`,
`disruptor_effect.vex` / `disruptor_cockpit.vex`), writes `craft+0x138 =
kind`, `craft+0x134 = 1`, and switches on the kind to write the **time** into
`craft+0x13c` and the amount/percent into `craft+0x140`/`craft+0x144` - the
same table slots the `Prime*` functions read, re-read here from the table
rather than copied off the firing craft.

`Ship_UpdateWeapons` (`0x0892935c`) applies it on the victim's side every
tick while `craft+0x134` is set, on the body's flag word `body+0x1c0`
(`body = ship+0xc8`):

| Kind | What the switch does each tick | Where the bit is spent |
| --- | --- | --- |
| 0 Stall | `flags \|= 0x10` | `Ship_UpdateEngine` (`0x0892f1f0`): with `0x10` set and the craft not AI-driven it zeroes the engine output at `body+0x270` and returns before the thrust law - **no thrust for `time` seconds** |
| 1 Mirror Left Right | `flags \|= 0x40`, `ship+0x991 = 1` | `Ship_ApplySteeringTorque` (`0x0892edfc`): yaw torque is **negated** while the timer at `body+0x2a0` is above `1.0`, and scaled by `1.0 - 2.0 * timer` through the last second, so it blends back from `-1` to `+1` |
| 2 No Airbrakes | `flags \|= 0x100` | `FUN_0892d214`: both airbrake inputs (`input+0x08`, `input+0x0c`) are **zeroed** before the airbrake law reads them |
| 3 Autopilot Slow | autopilot on (`FUN_0892c620(body, 1)`), `flags \|= 4`, `body+0x258 = 0.7`, `ship+0x9a4 = 1` | `Ship_UpdateEngine`: thrust `*= body+0x258` while bit 4 is set - **the literal `0.7`, not the authored `speed_percent`**, which is written to `craft+0x144` and read by nothing found |
| 4 Autopilot Fast | the same with `body+0x258 = 1.3` | as above, literal `1.3` |
| 5 Drunk | `flags \|= 0x20` | `Ship_ApplySteeringTorque`: with probability `1/16` a tick (`rand() & 0xf == 0`) a new yaw offset `((rand() & 0xff) - 128) * 0.01 * Drunk.amount` is drawn into `body+0x29c` and **added to the steering input** - this one does read its `amount`, straight from the table at `0x08b1781c` |
| 6 Drunk Camera | `ship+0x992 = 1` | camera code, unread; `amount` is carried at `craft+0x140` |
| 7 Rubber Ship | `flags \|= 0x80`, `(ship+0x8b4)+0x3c8 = 1.0` | `FUN_0892dea8`'s hover pass scales one damping term by `0.2`; the `+0x3c8` parameter is restored to `0.4` on expiry |

Then `body+0x2a0 = craft+0x13c`, `craft+0x13c -= dt`, and at or below zero
the whole thing is torn down: `DisruptorFx` removed, bits `0x10 0x20 0x40 0x80
0x100` cleared, `+0x3c8 = 0.4`, autopilot off, `body+0x258 = 0`, bit 4
cleared.

**What is not read**: how `Drunk Camera` moves the camera, what `HUD Flicker`
and `Trippy` would have drawn (nothing rolls them), and the AI-side caller of
`Disruptor_SpeedForClass` (`FUN_08931270`, presumably the opponents' aim).
**What is recovered without a reading**: `Ship_UpdateWeapons` adds `0.5` to
`body+0x248` when a blast lands on a craft, and `Ship_UpdateEngine` takes the
same no-thrust exit while `+0x248 > 0` - so every weapon hit on Pure stalls
the engine for half a second by the Stall's own mechanism. Recorded for the
next reader; not this page's subject.

## Pure's Bomb has no fuse, and `damageradius` has no reader

`WeaponStats_ParseBomb` (`0x08809230`) matches `damage damageradius blastforce
slowdown_time blastradius absorb trigger_radius` - **no `timetodie` branch at
all** - into `+0xc8..+0xe0`. `WeaponStats_ParseMine` (`0x0880941c`) is the
same function with `timetodie` where the Bomb has `damageradius`, into
`+0xe4..+0xfc`. So on Pure the two blocks differ by exactly one attribute, and
it is the fuse.

`Bomb_Init` (`0x088580d8`; `WO_BOMB_GLOW`, `BOMBLAUNCH`, `~BOMBRADAR`,
`Bomb.cpp` allocation) drops the charge, probing `10.0` below the craft for
the floor. `BombPool_Update` (`0x0884e968`) per live bomb adds `dt` to
`bomb+0x64`, calls `Bomb_UpdateSpin` (`0x088583d8`) - which spends that age on
**nothing but the model's two spin angles** (`rate * age mod 2pi`, twice) -
and then `Bomb_UpdateTrigger` (`0x0884ef78`). That one walks every craft but
the owner, box-tests then distance-tests against **`DAT_08b17880`, which is
`+0xe0 = trigger_radius`**, and on a hit sets the detonate bit and calls
`Bomb_ApplyHit` (`0x0884f420`). `BombPool_Update`'s third pass detonates
anything with that bit through `BombPool_Detonate` (`0x0884f214`;
`BOMBEXPL`, `BombExplo`) and swaps it out of the pool.

**Nothing compares the age to anything.** The only other path to the detonate
bit is the second pass, gated on `DAT_08b173e4 > 8` (the network-session
state) and an owner slot that has gone inactive. So in a single-player race a
Pure Bomb **sits until tripped**, for the whole race if nothing trips it.
Confidence 86 - three functions read end to end, plus the parser's own absence
of the attribute.

`Bomb_ApplyHit` reads `DAT_08b17868` (`+0xc8 damage`) into the tripping
craft's `+0x120`, `DAT_08b1787c` (`+0xdc slowdown_time`) into its `+0x130`,
and then pushes every craft but the owner within `DAT_08b17870` (`+0xd0
blastradius`) by `direction * (1 - d / blastradius) * DAT_08b17874` (`+0xd4
blastforce`) - the same falloff Pulse's `Weapon_PostBlastImpulse` has, read
independently here. **`DAT_08b1786c` (`+0xcc damageradius`) has no
cross-reference in the program**: `get_xrefs_to 0x08b1786c` returns nothing,
where the six slots either side of it each resolve to exactly the consumer
named above. It is authored, parsed, stored and never read, on Pure as on
Pulse. Confidence 88 on the absence.

## Ship-side names

`Ship_UpdateWeapons` (`0x0892935c`) is named at 78 rather than 84 because the
946-instruction body was read for its disruption switch, its fire dispatch
(`ROCKET`, `QUAKELAUNCH`, `~AUTOPILOT` cue strings; `FUN_0884e3a8` as the
weapon fire; `FUN_08923b9c(absorb, ship)` as the absorb credit, whose ten
cases pin the id map above) and its lock scan, and not for its telemetry
counters. `Ship_UpdateEngine` (`0x0892f1f0`) was already in `names.tsv` at 62
from a fuzzy match against Pulse; reading its body - throttle input at
`input+0x04`, per-class accelerate/decelerate rates off the handling table,
the autopilot thrust scale, the no-thrust exit - raises it to 74 with this
page as its evidence. `Ship_ApplySteeringTorque` (`0x0892edfc`) is named at 72
off its Drunk and Mirror arms alone; the rest of the function is the yaw
torque law and was not read in full.
