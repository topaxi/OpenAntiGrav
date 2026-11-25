# HD's collision and grind sounds: what a contact has to be before it is heard

2026-11-15, lane `hd-impact`. Wipeout HD/Fury EU (`BCES-00664`), Ghidra program
`/hdfury/EBOOT-ps3-hdfury-eu.elf`, RPCS3 0.0.43-20574 for the live half.
[cue-variables.md](cue-variables.md) recovered how `.COLLISIONS` chooses between its children
(cue variable 1 the surface, variable 0 the severity) and left open what the game computes the
two from. This page is that half: the function that starts the cue, the fields it reads, and
who fills them.

## Bottom line

1. **The severity is the size of the contact impulse**, the largest `|p|` among the contact
   records the craft's body took this frame: over 50 is large, over 25 medium, otherwise small.
   Confidence 92.
2. **The surface is the record's first word**: a craft pair's record writes `1` there
   (variable 1 = 0), a track record `0` (variable 1 = 1). Confidence 92.
3. **A contact is heard only past a floor**: an impulse over 3, and over 18 unless the craft met
   the surface nose-on (the angle between its forward axis and the record's normal under
   0.7 rad). Then the cue cannot start again for 6 frames. Confidence 90.
   **A craft pair's record never writes its normal**, so the angle is taken against whatever
   that ring slot held before: zero (the angle `pi / 2`, the 18 floor) on a craft whose slot no
   track contact has used, the last track contact's normal otherwise. Confidence 88.
4. **The player's craft has a second cue, `.GRIND`**, for a contact that is not nose-on: any
   impulse over 0.5, every other frame, its own severity steps at 4 and 8. Other craft never
   grind. Confidence 92.
5. **A running Shield swaps the player's two cues** for `.COLLSHIELD` and `.GRINDSHIELD`; other
   craft play `.COLLISIONS` whatever their shield. Confidence 88.
6. **The volume is the impulse too**: `min(|p| / 28 + 0.2, 1)` for a hit and
   `min(|p| * 0.125 + 0.25, 1)` for a grind. Confidence 92.

## Names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x000d8bd0` | function | `Ship_UpdateCollisionCues` | 90 |
| `0x000d8910` | function | `Ship_RecordContactImpulse` | 78 |

Both carry TOC `0x008ad4d8` (`scripts/ps3-toc.py toc 0x000d8bd0`), so every displacement below
is against that base. `Ship_RecordContactImpulse` has an OPD entry (`0x00874a28`) and no caller
this pass found; the copy that runs is inlined in `Ship_UpdateEntity` at `0x000ec4a8` to
`0x000ec518` and `0x000ec640` to `0x000ec6d0`, instruction for instruction the same.

## The fields

Offsets from the ship entity (`r31` in `Ship_UpdateEntity`, `0x000eadb8`).

| Field | Type | Meaning | Written at |
| --- | --- | --- | --- |
| `ship + 0x6904` | i32 | frames until a collision cue may start again | `0x000d8fd8` (from tuning `+0x4c`), counted down at `0x000d8c44` |
| `ship + 0x6908` | i32 | the same for the grind cue | `0x000d8db4` (tuning `+0x50`), `0x000d8c60` |
| `ship + 0x690c` | f32 | the frame's largest contact impulse | zeroed at `0x000ebe44`, raised at `0x000ec640` |
| `ship + 0x6910` | u32 | word 0 of the record that set it: `0` for the track, `1` for a craft | `0x000ec660` |
| `ship + 0x6920` | vec4 | that impulse, normalised | `0x000ec6a0` |
| `ship + 0x6930` | f32 | `dot(body forward, record normal)` | `0x000ec6cc` |

The body is `*(ship + 0x6944)`; its forward row is `body + 0x1f0` ([physics.md](physics.md)).
A contact record is one `0x50`-byte entry of the body's ring (`body + 0x220`, count
`body + 0x4a0`, [wall-response.md](wall-response.md), "The shield drain is Pulse's"): the
other object at `+0x00`, the point at `+0x10`, the impulse at `+0x20`, the normal at `+0x30`,
the friction at `+0x40`, the other collider's class at `+0x44`.

The tuning block is `0x008c15e0` (TOC slot `0x008a8c6c`): `+0x48 = 0.7` (the angle, radians),
`+0x4c = 6` and `+0x50 = 1` (the two re-arms, frames). Read from the file and again live: the
same three values.

## A pair record has no normal

Two functions push a record, and they do not write the same fields:

| | `0x000f56d0`, a track contact (`Body_ResolveContact`, call `0x000f7b0c`) | `0x000f5780`, a craft pair (`Body_ResolveContactPair`, calls `0x000f8258` and `0x000f8270`, one per body) |
| --- | --- | --- |
| `+0x00` | `0` (`stwx r0` with `li r0,0`, `0x000f575c`) | `1` (`li r0,1`, `0x000f57d8`) |
| `+0x10` point | written | written |
| `+0x20` impulse | written | written |
| `+0x30` normal | written (`0x000f5760`) | **not written** |
| `+0x40` friction | written | written |
| `+0x44` class | written (`0x000f573c`) | **not written** |

So `ship + 0x6910` is exactly `0` or `1` (live: 34 and 105 of 139 triggers, no other value), and
`ship + 0x6930` after a craft contact is the dot of the craft's forward axis with a normal some
earlier track contact left in that slot. Both rows are in `names.tsv` under the names
[xfade.md](xfade.md) gave them before the ring was read (`Physics_QueueRayProbe`,
`Physics_QueueSegmentProbe`, 62); [wall-response.md](wall-response.md) already reads the first
as the body's contact record, and this is the second. The rows are another lane's and are left
as they are.

## Who fills them: the ring walk

`Ship_UpdateEntity` zeroes `ship + 0x690c` at `0x000ebe44` and then, for the player and the
opponents (role word `ship + 0x628c` is 0 or 2) whose state (`ship + 0x5fa4`) is not 6, walks
the body's ring from `0x000ec22c`. Per record, in this order:

```text
p = |record.impulse|                                       0x000ec2bc..0x000ec2fc
ship.contact_measure(+0x5f78) = max(|old|, p), signed by which side of the craft
                                the point is on (dot with body right, +0x1d0)   0x000ec304..0x000ec348
damage = p * 0.035 (TOC -0x43f0)                           0x000ec34c
if damage is the largest so far: keep it, and the point's offset along body forward
                                                           0x000ec60c..0x000ec638
unless the slot's shield bit is set (*(ship+0x5edc)+0x200 & 0x10):
    for each local view: if the point is within 100 units of that view's eye (squared
    distance under 10000 and over 0) and the craft passes a visibility test,
    Ship_SpawnContactFx(ship, record, min(p * 0.0125, 1))  0x000ec378..0x000ec4a4, call 0x000ec9fc
if ship.impulse(+0x690c) < p:                              0x000ec4a8..0x000ec518
    ship.impulse = p; ship.other(+0x6910) = record[0]
    ship.direction(+0x6920) = normalize(record.impulse)
    ship.dot(+0x6930) = dot(body.forward, record.normal)   0x000ec640..0x000ec6cc
if record.friction > 0 and the craft is racing (state 1):  0x000ec51c..0x000ec544
    shielded: ShipShield_Hit(*(ship+0x6f6c))               0x000ec5d0..0x000ec600
    else:     Ship_ApplyDamage(ship, damage, record[0] != 0, 0, 0) and flag bits 0x400020
                                                           0x000ec900..0x000ec960
if record[0] == 0 and record.class(+0x44) == 2: a reset surface, Ship_SetState(ship, 6)
                                                           0x000ec548..0x000ec55c, 0x000ec7a0..0x000ec8cc
```

After the loop the player's craft runs `Ship_UpdateContactRumble` (`0x000d10c8`, call at
`0x000ec8f4`) with the kept forward offset. `Ship_SpawnContactFx` (`0x000e6fd8`) is the
contact's spark: [ship-collision-fx.md](ship-collision-fx.md), "who owns the looping spark".

So the cue's input is the same impulse the shield drain spends, record by record, and the
frame's **largest** one, not their sum.

## `Ship_UpdateCollisionCues` (`0x000d8bd0`)

Called once a frame for every craft from `Ship_UpdateEntity` (`0x000ebeec` and `0x000ec758`,
straight after `Ship_UpdateEngineCrossfade`). Returns at once when the craft has no emitter
(`ship + 0x5ee0`).

```c
angle = acosf(fabsf(ship->dot));                    // 0x000d8c10..0x000d8c18, thunk 0x006776b8
pan = 0;
if (ship->role == 0) {                              // the player: a pan from the impulse
    v = listener_rows * ship->direction;            // *(0x00b6cc28)+0x50.., 0x000d8c98..0x000d8d30
    if (|v.x| > |v.y| && |v.x| > |v.z|)
        pan = (int)(v.x * (ship->cockpit ? -90.0 : -31.5));   // ship+0x5f42, 0x000d8d48..0x000d8d6c
}

if (ship->hit_wait > 0) ship->hit_wait -= 1;        // 0x000d8c34..0x000d8c44
else {
    p = ship->impulse;
    if ((angle < tuning->angle || p > 18.0) && p > 3.0) {     // 0x000d8f90..0x000d8fb8
        ship->hit_wait = tuning->hit_frames;        // 6
        volume   = min(p * (1.0 / 28.0) + 0.2, 1.0);          // 0x000d8fd0..0x000d8fe4
        severity = p > 50.0 ? 2 : p > 25.0 ? 1 : 0;           // 0x000d8fc0..0x000d9000
        surface  = ship->other == 0 ? 1 : 0;
        if (ship->role != 0) {                      // any other craft: its own emitter
            emitter->var[0] = severity;             // 0x000d9014
            emitter->var[1] = surface;              // 0x000d9028 / 0x000d9244
            SoundEmitter_PlayCue(volume, emitter, ship_bank, 0, ".COLLISIONS", 0, -1);   // 0x000d904c
        } else {                                    // the player: unplaced, with its own variables
            vars = { severity, surface, ship->perspective /* +0x5f14 */, 0 };   // 0x000d9100..0x000d9118
            name = slot_shielded(ship) ? ".COLLSHIELD" : ".COLLISIONS";         // 0x000d911c..0x000d913c
            if (race is running)                    // RaceManager+0x1970 == 2
                Sound_PlayBankCue(manager, ship_bank, name, (int)(volume * 1024), pan,
                                  0, 0, vars, floats);                          // 0x000d9220
        }
    }
}

if (ship->role != 0) return;                        // 0x000d8c48
if (ship->grind_wait > 0) { ship->grind_wait -= 1; return; }   // 0x000d8c50..0x000d8c60
p = ship->impulse;
if (p > 0.5 && angle >= tuning->angle) {            // 0x000d8d78..0x000d8d94
    ship->grind_wait = tuning->grind_frames;        // 1
    volume   = min(p * 0.125 + 0.25, 1.0);          // 0x000d8dac..0x000d8dc0
    severity = p > 8.0 ? 2 : p > 4.0 ? 1 : 0;       // 0x000d8d98..0x000d8ddc
    vars = { severity, ship->other == 0 ? 1 : 0, 0, 0 };      // 0x000d8e74..0x000d8e88, 0x000d9238
    name = slot_shielded(ship) ? ".GRINDSHIELD" : ".GRIND";   // 0x000d8eac..0x000d8ecc, 0x000d9230
    if (race is running)
        Sound_PlayBankCue(manager, ship_bank, name, (int)(volume * 1024), pan, 0, 0, vars, floats);
}
```

`slot_shielded` is `*(ship + 0x5edc) != 0 && (*(ship + 0x5edc) + 0x200) & 0x10`, the bit the
Shield pickup sets on the slot ([shield.md](shield.md)). The constants are single floats in the
TOC: `18.0` (`-0x468c`), `3.0` (`-0x4880`), `50.0` (`-0x4688`), `25.0` (`-0x46d0`),
`1/28 = 0.0357143` (`-0x46ac`), `0.2` (`-0x4724`), `0.5` (`-0x49d4`), `8.0` (`-0x4680`),
`4.0` (`-0x4884`), `0.125` (`-0x467c`), `0.25` (`-0x4678`), `1024.0` (`-0x4684`),
`-31.5` (`-0x4694`), `-90.0` (`-0x4690`). The strings: `.COLLSHIELD` `0x00782080`,
`.COLLISIONS` `0x00782090`, `.GRINDSHIELD` `0x007820a0`, `.GRIND` `0x007820b0`.

Four things follow from the code that the bank alone does not say:

- **The grind cue's perspective byte stays 0** (the grind block never stores to `vars[2]`),
  the collision cue's is the craft's (`ship + 0x5f14`: 1 in the chase view, 0 in the cockpit,
  [cue-variables.md](cue-variables.md)).
- **A hit and a grind start on the same frame** when a contact is both large and not nose-on:
  the two blocks are independent and read the same impulse.
- **The comparison is with the angle, not the dot**: `0x006776b8` is a thunk to `0x0041bc90`,
  the C library's `acosf` (its body is `atan` of `x / sqrt(1 - x^2)` with the quadrant offset,
  split at `0.7071`; Omega's twin calls a function Ghidra names `acosf`).
- **Mode 14 (Detonator, [mode-manager.md](mode-manager.md)) plays the shield names from another
  bank**: when `g_GameState.mode == 0xe` the player's two cues are `.COLLSHIELD` and
  `.GRINDSHIELD` from the bank at `*0x00b6cc50`, whatever the slot's bit (`0x000d925c`,
  `0x000d92f8`). Read, not run.

## Measured live

`scripts/rpcs3-hd-collision-cue.py` polls the six fields on all eight craft and the SCREAM
handler list about 200 times a second through the emulator's memory, with no breakpoint. A
craft's countdown seen risen is a cue started that frame; the handler that appears beside it
(within 80 ms) is the child `.COLLISIONS` or `.GRIND` replaced itself with, and carries the
variables the caller supplied. Three captures, Talon's Junction, Venom, eight craft:

| Capture | State | Input | Seconds | Triggers | Paired with a handler |
| --- | --- | --- | ---: | ---: | ---: |
| 1 | `hd-racebox-talons-venom-weapons-grid`, from the start | cross held, stick swung with a 2.5 s period | 150 | 34 | 24 |
| 2 | `hd-race-talons-venom-pack`, mid pack | none (the state's own Autopilot) | 200 | 76 | 76 |
| 3 | as 1, a Shield given and fired every 8 s | as 1 | 110 | 29 | 17 |

An opponent's trigger with no handler is one the emitter dropped for distance. Capture 2 is
flown by the Autopilot: it is evidence for the cue law, which reads a contact that happened,
and for nothing about the contact itself.

| Check | Result |
| --- | --- |
| variable 0 is the severity the impulse predicts | 115 of 117 paired starts; the two others are capture 3 pairing a grind with the `c_GS` child of a `.COLLSHIELD` that started on the same frame (its variable 2 reads 1, which only the collision block sets) |
| variable 1 is 0 when `ship + 0x6910` is non-zero, 1 when zero | 117 of 117 |
| the steps | grind: `3.9755` small, `4.0588` medium, `7.0022` medium, `8.7752` large; hit: `21.36`, `23.87`, `24.52` small, `30.51`, `46.17`, `49.33` medium, `52.15`, `57.93`, `139.9` large |
| the player's volume word | `vol16 / (int)(law * 1024)` is `0.627` to `0.630` on all 92 `c_GShip*` and `c_CShipWall*` starts (80 / 127, the cue's authored volume; the spread is the word's rounding), `0.708` on the one `c_CShipShipM` (90 / 127) and `0.944` on 8 of the 9 shield children (120 / 127; the ninth is one of the two mispaired rows above): the law times a constant per cue |
| perspective | the player's collision children carry variable 2 = 1 (chase view), its grind children 0, another craft's 2 |
| a craft pair's dot, the player | `0.0` on all 71 of the player's craft-contact triggers (capture 2, where its craft met no wall), so the angle is `pi / 2`: 70 grinds from `0.57` to `23.6` and one hit, at `23.61` |
| a craft pair's dot, opponents | exactly `0.0` on 11 of their 34 craft-contact triggers; the rest read a stale value, and 7 of the 34 are hits under 18 (`3.36` to `12.73`) that only a nose-on angle lets through. Captures 1 and 3, where the field had scraped the walls |
| the Shield | with slot bit `0x10` read set, the player's starts are `c_CS_*` and `c_GS_*` (`.COLLSHIELD`, `.GRINDSHIELD`), 8 of 8; with it read clear, `c_CShip*` and `c_GShip*` on 5 of 6 in capture 3 (the sixth a `c_GS_Large` as the Shield ran out between the frame and the poll) and on all 88 in captures 1 and 2 |
| both ends of a pair | the two craft of a contact arm on the same poll with the same impulse (`139.91` twice, `52.15` twice, `22.40` twice): one cue per craft |
| the tuning block | `0.7`, `6`, `1` in RAM |

A ship-to-ship hit at severity 0 starts `c_CShipShipM`, as cue-variables.md's table says
(`23.61`: variables `[0, 0, 1, 0]`, child `c_CShipShipM`).

Reproduce (the capture writes game memory readings to scratch, nothing tracked):

```sh
scripts/emu-env.sh <lane> <display> <gdb-port>; source data/scratch/<lane>/env.sh
scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid-rpcs3-20574.SAVESTAT.zst
python3 scripts/rpcs3-drive.py press cross        # once START RACE shows
OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- \
    python3 -u scripts/rpcs3-hd-collision-cue.py --out <dir> --seconds 150 --weave 2.5 [--arm 5 --arm-every 8]
python3 scripts/rpcs3-hd-collision-cue.py --analyse <dir>/collision-cues.jsonl --cues <ship-cues.json>
```

`<ship-cues.json>` maps a `shiphd.bnk` cue index to `[family, name]`; the indices are the name
table's: `c_GShipShipS/M/L` 42 to 44, `c_GShipWallS/M/L` 46 to 48, `c_CShipShipS/M/L` 52 to 54,
`c_CShipWallS/M/L` 56 to 58, `c_GS_Small/Medium/Large` 62 to 64, `c_CS_Small/Medium/Large` 65
to 67. `.GRIND` (cue 40) branches on variable 1 as `.COLLISIONS` does, to `c_GShipShip` (41) or
`c_GShipWall` (45).

## Wired

2026-11-16, lane `hd-impact`. HD/Fury only (`oag_title::contact_cue::ContactCues::Impulse`,
the numbers in `oag_hd::HD_IMPULSE_LAW`); Pulse, Pure, 2048 and Omega keep the re-armed
`.COLLISIONS` they had. **The player's grind is decided but not played** ("Grind held back").

- **Simulation side** (`crates/raceplay/src/cues.rs`): each tick keeps every craft's largest
  contact impulse, from the wall response's largest single contact
  (`WallResponse::impulse_largest`, with that contact's normal) and from each craft pair
  (`Race::resolve_craft_pairs`, both hulls, the pair's own impulse). Once every contact of the
  tick is in, `decide_contact_cues` runs the law above per craft and raises `Cue::Collision`
  (or `CollisionShield`) and, for a craft a person flies, `Cue::Grind` (or `GrindShield`),
  each carrying `ContactCue { severity, surface, volume }`. The nose-on test is
  `|dot| > cos(0.7) = 0.7648422`, the same comparison as `acosf(|dot|) < 0.7` without a
  platform transcendental.
- **Sound side** (`crates/sound/src/sfx/banks.rs`, `sfx.rs`): on the HD tick the loader walks
  `.COLLISIONS`, `.COLLSHIELD`, `.GRIND` and `.GRINDSHIELD` once per variable block the game
  can hand them (`[severity, surface, perspective, 0]`: 12, 6, 6 and 6 blocks) with the
  replacing child followed, and a contact event plays the block it names at the request's
  volume. A cue whose blocks do not all build plays nothing for a contact and says so in the
  report; a contact never falls back to the flat pick. The three new cues are loaded on HD
  alone (`Cue::HD_CONTACT`), so no other title's report gains a line.
- **Tests.** `crates/raceplay/src/tests/contact_cues.rs`: the law's floors, steps, volumes
  and waits with the captured impulses above (`3.9755` small, `4.0588` medium, `21.36`,
  `52.15`, the volume words `986` and `789`), and three race cases: a hard craft bump raises
  one `.COLLISIONS` on each hull with the craft surface and the law's severity and volume; a
  gentle one raises a grind for the person and nothing for the opponent; the re-armed law
  raises nothing from a craft bump. With the pair's hook removed the first two fail
  (`slot 0: []`, `left: [] right: [Grind]`). Ground truth on the Fury disc
  (`crates/raceplay/tests/sfx_timeline_ground_truth.rs`,
  `hd_s_contact_cues_play_the_child_their_severity_and_surface_select`, inside the one test
  that loads the banks): every block of every contact cue keys only commands of the child the
  table in [cue-variables.md](cue-variables.md) names (`[2, 0, 2, 0]` and `[1, 0, 2, 0]`
  `c_CShipShipL`, `[0, 0, 1, 0]` `c_CShipShipM`, the wall and grind children by severity, the
  `Small` pair for `.COLLSHIELD` at severity 0), and the census's flat list, `[".COLLISIONS"]`
  before, is empty.
- **Load report** (`oag-game <hd iso> --race --dry-run`, `RUST_LOG=debug`): `.COLLISIONS`
  12 contact blocks of up to 6 voices, `.GRIND` 6 of 1, `.COLLSHIELD` 6 of up to 9,
  `.GRINDSHIELD` 6 of 1.

**Chosen, not measured**, or not modelled:

- **A craft pair's dot is 0.** The original reads the stale normal of the ring slot ("A pair
  record has no normal"); zero is that slot before any track contact has used it. A craft that
  last met a wall nose-on would, in the original, let a craft hit through from 3 and grind
  nothing; here it needs 18 and grinds.
- **The cues ride the craft's emitter.** The original plays the player's four cues unplaced
  with a pan from the impulse's direction (`-31.5` or `-90` times the listener-space x).
- **No race-state gate** (the original's `RaceManager + 0x1970 == 2` for the player), no
  Detonator branch, no cockpit perspective (variable 2 is 1 for the player, as
  cue-variables.md chose).
- **The collision cage's contacts are not counted**: its response carries no normal here.
- **The pair's impulse is this engine's pair resolver's** (Pulse's `-1.1` law with the
  craft's masses); HD's own `Body_ResolveContactPair` is at confidence 60 and not ported.

**What a race sounds like, measured.** See "Grind held back" below: the player's grind is
wired in the law and not played.

## Grind held back

2026-11-16, lane `hd-impact-2`. `ImpulseLaw::grind_played` is `false` for HD
(`oag_hd::HD_IMPULSE_LAW`): `decide_contact_cues` still decides the grind and the banks still
build its blocks, and `RaceSim::raise_impulse_cues` drops the grind (and `.GRINDSHIELD`, the same
block) before it reaches the cue list. `.COLLISIONS` and `.COLLSHIELD` play. Setting the field
to `true` plays it again (`OAG_SWEEP_GRIND=1` in the sweep below).

**Why.** The retrigger rule is the original's, so it was not changed: with `grind_frames = 1`
the original starts a new `.GRIND` every other frame of a contact and holds no voice. Capture 2
(`scripts/rpcs3-hd-collision-cue.py`, craft scraping craft) shows runs of 23 starts in 0.73 s
and 8 in 0.26 s, about 31 a second, the rate this engine starts at. What differs is how much
contact there is: the same script (cross held, stick swung with a 2.5 s period) gave the
original 10 player grinds and 7 player hits in 150 s (capture 1), and this engine, a 60 s
headless Fury race (Venom, Talon's Junction, `crates/raceplay/tests/zz_contact_sweep.rs`):

| Run (60 s, full throttle) | Player `.GRIND` starts | Player `.COLLISIONS` starts |
| --- | ---: | ---: |
| this branch, grind played, stick swung (full amplitude) | **471** | 34 |
| this branch, grind played, stick centred | **1,577** | 2 |
| this branch, grind held back (as shipped) | 0 | 34 (stick swung), 2 (centred) |
| main's law (`ContactCues::Rearmed`; `WallResponse` gained report-only fields, so the trajectory is main's) | none: main has no grind cue | 47 (stick swung), 65 (centred) |
| the original, capture 1 (150 s) | 10 | 7 |

Two causes, both on the wall response and not on the cue: contacts last long (110 episodes in
the swung run, the longer ones 30 to 90 ticks; the original's 10 are single starts), and many
are chatter, one start every 4 to 6 ticks. Played, one `.GRIND` voice lasts 0.31 to 0.71 s, so
a sustained scrape holds 15 to 20 at once (HD's pool is 128). The script's
swing defaults to the full stick (`--amplitude 1.0`) as the sweep's does, but the captures'
command lines were not kept with amplitudes, so the ratio, about 100 times, is an order of
magnitude and not a measurement. `.COLLISIONS` starts do
not rise against main (34 against 47).

**To play it again:** make the contact the original's (`hd-wall-response-leftovers.md`'s dwell
and chatter), re-run the sweep until the grind starts in a swung-stick minute are in the
original's range, then set `grind_played: true`.

## Lineage

- **2048: checked, applies, not wired.** `FUN_811c5c3a` (`/2048/eboot-vita-2048-eu-v104.elf`,
  the only reference to `.COLLISIONS` at `0x814931dc`) is this function term for term: the
  same floors (`3.0`, `18.0`), steps (`25.0`, `50.0`, `4.0`, `8.0`), volume laws
  (`0.035714287 * p + 0.2`, `0.125 * p + 0.25`), the two countdowns, the variable bytes
  `{severity, other == 0, perspective}`, the shield swap on slot bit `0x10` and the mode-14
  branch. Two differences: the pan is `-90 * 0.35` out of the cockpit (the same `-31.5`), and
  the collision cue is named `.COLLISIONS_1` when a settings test fails
  (`FUN_81000930(&DAT_8153fc40, -1) == 0`). The executable carries the four names twice; the
  second copy (`0x814e2ffc`) is referenced from `0x812cf720`, in code Ghidra has not made a
  function of, unread. Decompile only, confidence 80. Not wired: 2048's
  race cues are still a flat pick ([cue-variables.md](cue-variables.md), "Lineage"), and a grind
  every other frame through a flat pick would be worse than the cue it has.
- **Omega: checked, differs in delivery.** `FUN_0130b100` (`/omega/eboot-ps4-omega-eu-v107.bin`)
  keeps the law (`acosf`, `0.7`, `3.0`, `18.0`, `25.0`, `50.0`, the countdown of 6; the grind's
  `0.5`, `4.0`, `8.0` and countdown of 1) and hands the two results to the sound engine as
  switches, `FUN_0177fe40(emitter, 0, severity)` and `FUN_0177fe40(emitter, 1, other == 0)`
  (the strings `VoiceReg__CollisionSize`, `CollisionSizeSmall/Medium/Large`,
  `VoiceReg__CollisionType`, `CollisionTypeShipToShip/ShipToWorld`), on the events
  `_COLLISIONS_1`, `_COLLSHIELD`, `_GRIND` and `_GRINDSHIELD`. **Only the player's craft plays
  them**: the function returns for any other role before the collision cue is posted. No volume
  law is visible (the event's own). Decompile only, confidence 78. Not wired.

## Not determined

- **`0x00307d00(volume, -1, 0)`**, called before each of the player's `Sound_PlayBankCue`s.
- **How the pan argument is applied** (`r7` of `Sound_PlayBankCue`, degrees by the look of
  `-90`).
- **The other craft's volume.** `SoundEmitter_PlayCue` takes the same `volume` in `f1`; what
  the emitter does with it beside its distance law is not read.
- **Which ring slot a craft pair's record takes**, and so which stale normal it reads. The
  count (`body + 0x4a0`) restarts each frame and records fill in resolve order; whether the
  track or the pairs are resolved first is unread. Modelling the stale normal needs it.

## 2026-11-23: which records carry no friction, and the spark they throw

Lane `hd-spark-row5-re`, static only (Ghidra and the file's bytes, TOC `0x008ad4d8` for every
HD function below, checked with `scripts/ps3-toc.py toc`). No RPCS3 run: the packaged build
refused HD save states. The question is the ring walk's other branch: `Ship_SpawnContactFx`
picks the spark kind from the record's friction (`+0x40`), so a record with none throws
`WO_SHIP_COLL_SPARK_NODAMAGE` instead of restarting the attached damage spark, and the same
zero skips `Ship_ApplyDamage` (`0x000ec51c`).

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x002eaba8` | function | `TrackCollision_AddMeshCollider` | 82 |
| `0x0098d5b8` | data | `g_ShipFloorFriction` | 78 |

### Bottom line

| The craft (material 0) against | Built by | Material | Class `+0x74` | Record friction | Kind | Confidence |
| --- | --- | ---: | ---: | --- | ---: | ---: |
| `Wall Collision` (`0x3ba`) | `0x002eaba8` | 3 | 0 | `0.035` (`0.025` in the masked modes) | 0, the attached damage spark | 88 |
| `collision_trackwall` (`0x3ed`) | `0x002eaba8` | 3 | 5 | the same | 0 | 85 |
| another craft's body | `Ship_Construct` `0x000dece0`, twin `0x000e0d24` | 0 | - | `0.035` (`+0x28`) | 0 (landed row 1 stands) | 85 |
| `Floor Collision` (`0x3b9`) | `0x002eaba8` | 2 | 1 | `g_ShipFloorFriction`, never written: **0** | **2** | 78 |
| `Mag Floor Collision` (`0x3e6`) | `0x002eaba8` | 2 | 3 | the same: **0** | **2** | 78 |
| `Reset Collision` (`0x3cd`) | `0x002eaba8` | 4 | 2 | the same: **0** | **2**, and the record sends the craft to state 6 | 78 |
| a wreck (state 6's compound collider) | `Ship_Construct` `0x000ded38`.. | 1 | - | `(0, 1)` is outside the table: 0, **but no such record is made**: a compound (type 6) is tested against meshes and compounds only ([collision.md](collision.md), "type 6 is a compound shape"), never against a craft's box (type 3) | none | 75 |
| anything of material 5 | not read | 5 | - | `(5, 0)` returns 0 | 2, if it ever touches a craft | - |
| `Cage Collision` (`0x3e7`) | none: `0x002eacc8` skips the node | - | - | no record | - | 82 |

So on HD the one-shot no-damage spark is what a craft throws when its box meets the floor, the
mag floor or a reset plane. Walls and live craft keep the attached damage spark. The cage makes
no collider, and a wreck's compound never meets a live craft's box, so neither makes a record.
Material 5 is unread: its restitution against 2 and 4 is a random pick (`Libc_Rand`), which reads
like debris; the place to start is the debris body `0x0010c958`, whose collider comes from its own
constructor. If that collider is a sphere (type 5), HD tests it against meshes only
([mp-time-trial-collision.md](mp-time-trial-collision.md)) and it never makes a craft record.

### The kind rule (`Ship_SpawnContactFx`, `0x000e6fd8`)

```text
r31 = 0                                                    0x000e72c4
f31 = 0.0   (TOC -0x49d8 -> 0x008a8b00, 0x00000000)        0x000e71fc
if record.friction(+0x40) <= f31: r31 = 2                  0x000e7348 lfs, 0x000e734c fcmpu, 0x000e7414 ble -> 0x000e7730
if ship+0x6a64 & 0x800: FUN_00054628 / FUN_00055e30(.., ship+0x6284) gate   0x000e7418..0x000e7424, 0x000e7610 (unread)
if ship.state(+0x5fa4) != 1: r31 = 3                       0x000e7428..0x000e7434
ShipCollisionFx_Trigger(locator, r31, ..)                  0x000e74dc
ship+0x7d80 = 1                                            0x000e74f0, every kind
```

`fcmpu` with `ble` takes the branch on less or equal only, so a NaN friction keeps kind 0.
Kind 3 overrides kind 2: a frictionless record of a craft that is not racing restarts the
attached spark like any other. The byte `ship + 0x7d80` is set after **every** kind, so a frame
whose only record is frictionless still keeps a running attached spark alive (the release at
`0x000ebf8c` sees the byte). Confidence 85.

### The friction table (`Collision_PairFriction`, `0x000be068`)

Read instruction by instruction. The function orders the pair so that a 0 goes first (`r0`),
then:

| Pair | Returns | Where |
| --- | --- | --- |
| `(0, 0)`, `(1, 1)` | `table + 0x28` = `0.035` | `0x000be118`, `(1, 1)` via `0x000be138`/`0x000be140` |
| `(0, 1)` | `0.0` (TOC `-0x50e8`, `0x008a83f0`) | `0x000be110` sends `r4 = 1` to `0x000be150` |
| `(0, 2)`, `(0, 4)`, `(1, 2)`, `(1, 4)` | `*(TOC -0x50e0)`, the float at `0x0098d5b8` | `0x000be168` |
| `(0, 3)`, `(1, 3)` | `table + 0x2c`, or `+0x38` under the mode mask `0x206040` with the net byte clear | `0x000be1d4`, `0x000be174` |
| `(5, 3)` | `table + 0x34` | `0x000be0f4` |
| `(5, 2)`, `(5, 4)`, `(5, 5)` | `table + 0x30` | `0x000be0c8` |
| `(5, 0)`, `(5, 1)`, and any pair with neither side 0, 1 or 5 | `0.0` | `0x000be0a4` (`bnelr`), `0x000be150` |

This corrects one row of [wall-response.md](wall-response.md)'s coefficient table, which gives
`+0x28` to `(0, 0)` and `(0, 1)`: the second pair is `(1, 1)`, and `(0, 1)` returns zero. The
row is that page's and is left as it is.

**`0x0098d5b8` is never written.** It lies in the data segment's zero-filled tail (file size
`0x0d6f80` from `0x00860000`, so it ends at `0x00936f80`). One TOC slot in the whole image holds
the address (`0x008a83f8`, file offset `0x8983f8`), and one instruction under this TOC loads that
slot: `0x000be168`. The seven other `lwz rX,-0x50e0(r2)` hits (`0x003fccdc`..`0x0040a2b0`) run
on the other TOC (`0x008bd3c4`) and read another slot. A scan of every TOC load in the image
that yields an address up to `0x8000` below it, followed by `addi` and every store and pointer
argument within the function, finds no write to `0x0098d5b8` and no pointer to it. The same
scan finds the known writes beside it (`0x000bdd7c` at `0x0098d494`, `0x000bddfc` at
`0x0098d5b4`, `0x000bf9d8` at `0x0098d5bc`), so it sees this kind of store. The weapon
`Stats` parser's `+0x188` store (`0x0002bdf0`, `venomspeed`) writes a `0x7a8`-byte heap object
(`0x000311e8` allocates three), not this float. Confidence 78: static, and the two later titles
agree (below). The live read that would take it past 90: any RPCS3 memory read of `0x0098d5b8`
in a race, or a breakpoint on `0x000be16c` reading `f1`.

### Which collider has which material

`0x002eaba8` builds one mesh collider (`0x00039a90`, `0xb0` bytes) per object of a collision
node (the loop `0x002ead7c`..`0x002ead90` runs the payload's object count), adds it
(`Collision_AddObject`, `0x002ead14`), then sets the material `+0x84` and the class `+0x74` from
the node's class id (`*(node + 0xc8)`):

```text
0x3e7 (Cage)          skipped before the allocation          0x002eacc8..0x002eaccc
0x3b9 (Floor)         material 2, class 1                    0x002eadd8, 0x002eade8
0x3e6 (Mag Floor)     material 2, class 3                    0x002eadd8, 0x002eae60
0x3ba (Wall)          material 3, class 0                    0x002eadf8, 0x002eae78
0x3ed (trackwall)     material 3, class 5                    0x002eadf8, 0x002eae6c
0x3cd (Reset)         material 4, class 2                    0x002eae88, 0x002eae8c
```

The class values are Pulse's surface types for the same four nodes (Wall 0, Floor 1, Reset 2,
Mag Floor 3, [collision.md](../../../formats/collision.md)), and class 2 is the one the ring
walk sends to state 6 (`0x000ec548`), so the reading is consistent. Confidence 82. Both ship
constructors give the body's box material 0 (`li r25, 0` at `0x000deb48`, stored at
`0x000dece0`; `0x000e0b8c`, `0x000e0d24`) and the seven-child compound at `ship + 0x694c`
material 1 (`li r26, 1`, stored at `0x000ded38` and on each child). That compound is the
wreck's collider, swapped in at state 6 ([ship-destroy.md](ship-destroy.md)), so a live craft
could meet material 1 only from a wreck, and the shape dispatch never pairs the wreck's compound
with a craft's box. Confidence 75: the stores and the dispatch are read, the swap is that page's.

A record carries `Collision_PairFriction` of the two materials either way: the track resolver
passes `contact + 0x2c` (`0x000f7afc`, call `0x000f7b0c`), the pair resolver the same field
for both bodies (`0x000f824c`, `0x000f8264`).

### What kind 2 spawns (`ShipCollisionFx_Trigger`, `0x002d9d18`)

```text
every kind returns at once when *(locator+0xe4)+0x5f42 != 0      0x002d97a0..0x002d97e8
name = net byte clear and mode in 0x206040 ? "WO_SHIP_SPARK_NODAMAGE_ZONE" (0x007a1ca8)
                                         : "WO_SHIP_COLL_SPARK_NODAMAGE" (0x007a1c88)   0x002d9d18..0x002d9d50
node = new 0x180-byte LinkObj ("System\LinkObj.h", line 0x21e), zeroed, node+0xc = locator
FUN_003238f8(locator, node)                    a child of the locator node
FUN_002c9108(node, name, 'DNPS', matrix = 0, flags = 0, 0)                  0x002d9e64
node+8 = 0x008745a0; if node+0x34 & 8: FUN_003265c8(node)                   0x002d9e70..0x002d9e88
```

With a null matrix and flags 0, `FUN_002c8c70` gives the node its own matrix, the identity
(`0x00769d70`), and hands the particle instance that matrix (`node + 0xf0`). So kind 2 is a new,
unattached, one-shot instance per record, a child of the nearest of the ten locators
(`ship + 0x79d0`). It takes no strength (the trigger's `f1` is not read on this path), no
carried velocity and none of the caller's basis vectors; its lifetime is the effect file's own.
The gate on `ship + 0x5f42` (the player's internal view or look-back, read 2026-11-25:
[particle-triggers.md](particle-triggers.md), "the camera-mode flags"; [cannon.md](cannon.md)) stops every kind, the attached spark included.

**One spark per record, whatever the number of views.** The ring walk's view loop
(`0x000ec3e4`..`0x000ec4a4`) calls `Ship_SpawnContactFx` from `0x000ec9fc` for the first view
that passes, then branches past the loop (`0x000eca00`, `b 0x000ec4a8`). A kind 2 node is
allocated per call, so a frame with three floor records makes three. Confidence 85. Confidence 80 for the branch; **60 that the spark sits at the locator**: whether
the scene update composes the parent locator's world matrix into `node + 0xf0` is not read.
Recipe: after a floor contact, read `node + 0x120` (the translation row) against the locator's
world position, or read the update reached through the node's vtable.

### Not read here

- How often the original's box reaches a floor at all (ours: "Wired", Rate). HD hovers on rays, and
  the box meets the floor only on a hard landing or scrape. The rate decides how often a player sees
  this spark.
  Recipe: RPCS3, a jump landing on a hover-heavy circuit, poll the ring's class word
  (`body + 0x264 + 0x50 * i`) for `1`, `3` and `2`.
- The `ship + 0x6a64 & 0x800` gate at `0x000e7610` (`FUN_00054628`, `FUN_00055e30`).
- When a kind 2 node is freed.

### Wired (`crates/raceplay/src/floor_sparks.rs`)

`oag_physics::wall::WallResponse::frictionless` keeps the first eight frictionless records of a
frame (point and impulse; reporting only, not hashed) and `oag_physics::reset::contact`'s point
is the reset plane's record. `RaceView::advance_floor_sparks` plays `Trigger::CollisionSparkNoDamage`
once per record, unattached (`Stage::play`), at the world position of the locator nearest the
record, scale `1.0`, no carried velocity, no cooldown and no `.COLLISIONS` cue (the kind 2 path
raises none). The kind rule is `floor_sparks::contact_kind` (`friction <= 0` is 2, a craft not
racing is 3, NaN keeps 0). Gates: a Shield, a craft no local view sees within 100 units (the
ring walk's reach, as `field_sparks`), a craft that is not racing, and a title with no
`Ship_DispatchCollisionFx` locators (`HitSparks::dispatch_anchors`, so Pulse and Pure throw
nothing). A frame of only frictionless records counts as touching for the attached spark's
release rule, for the seat and for the opponents.

- **Chosen, not measured:** the spark stays where it spawned instead of riding its locator (the
  confidence-60 value above stays open); the eight-record cap. The flag `ship + 0x5f42`
  is read (2026-11-25, [particle-triggers.md](particle-triggers.md), "the camera-mode flags"): the
  internal view or the look-back of the player's own craft, 0 for every rival; this engine has
  neither view, so it is always 0 here and the gate changes no frame.
- **Kind 3** (`NotRacing`) restarts the attached spark from the floor record's point and impulse
  (`floor_sparks::not_racing_record`, handed to `attach_rule` as a pair record, for the seat and
  the opponents; test `a_floor_record_of_a_craft_that_is_not_racing_restarts_the_attached_spark`). "Racing" is state 1: `CraftState::Racing` and off the grid (the countdown is state 0, so a floor record there is kind 3); a state 6 wreck makes none. The spark's place and the reach test are taken when the record is made, because a reset plane's record teleports the craft that tick.
  Not wired: the reset plane's record holding an attached spark (the craft is sent to state 6 the
  same frame).
- **Rate, measured on ours:** Chenghou Project, an Ace field with the seat on the autopilot, 6000
  ticks: seven sparks in four landings (the seat's at tick 3273 and 5466, three records each),
  so a player sees it on a hard landing, not a scrape. Frame: the seat's landing at tick 3273
  and 3275 to 3277 against the same ticks with the throw off shows the spark behind both rear
  wing tips.

### 2048 and Omega

- **Floor spark: ported by title data, not run.** Both carry the `WeaponSpark` entry that gates
  `dispatch_anchors`, both name `WO_SHIP_COLL_SPARK_NODAMAGE` (`particle-paths.md`, this
  directory's `ps4-omega-eu/ship-collision-fx.md`), and their races run the same physics and
  `oag-raceplay` path, so a floor record throws it there too. No 2048 or Omega race was run for it.
- **2048: checked, the frictionless floor applies.** `Collision_PairFriction`
  (`0x811a1870`, `/2048/eboot-vita-2048-eu-v104.elf`, [craft-laws.md](../vita-2048-eu-v104/craft-laws.md))
  has HD's shape with more materials: a craft (0 or 1) against a craft returns `0x8151fc18`
  (`0.035`), against walls 4, 5, 6 `0x8151fc1c`/`0x8151fc20` (`0.025`/`0.035` by the Zone test),
  and against 2, 3, 7, 10, 11, 12 the float at `0x818a5390`, past the data segment's file size
  (`0x8153fae0`) and read only by this function: zero, as HD's. `(0, 1)` gives `0.035` there;
  what 2048's material 1 is, is not read. Which 2048 surface is which material, and its contact-fx
  caller, are not read.
- **Omega: checked, the frictionless floor applies.** The pair tables are inlined
  into each narrowphase test (`FUN_012539b0` and its siblings, all readers of `shipWallCofR` at
  `0x0193fc54`). A craft (0 or 1) gets `0x3d0f5c29` (`0.035`) against a craft, `0.025`/`0.035`
  against 4, 5, 6, and the literal `0` against everything else: the compiler folded the
  never-written float to a constant. Same law as 2048's.
