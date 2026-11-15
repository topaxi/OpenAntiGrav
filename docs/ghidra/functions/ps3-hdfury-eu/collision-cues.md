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
`.COLLISIONS` they had.

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

**What a race sounds like, measured.** A 60 s headless Fury single race (Venom, Talon's
Junction, the player on full throttle swinging the stick with a 2.5 s period) raised for the
player 34 wall hits (9 small, 11 medium, 14 large) and 471 grinds, and for the field 2 wall
hits and 2 craft hits. The same input on RPCS3 (capture 1, 150 s) gave the player 7 hits and
10 grinds: **this engine's craft stays on a wall for some thirty ticks after a hit, the
original's leaves within a frame or two**, so the grind, which the law restarts every other
tick of a contact, runs far longer here. That is the wall response's difference, heard
through a faithful trigger. Rendered offline (`--race --hold cross,left --ticks 1200
--dump-audio`, the craft held into a wall for 20 s): the mix's RMS is `0.154` against `0.116`
before the change, and the limiter works on 75,819 of 960,000 frames, never under a gain of
`0.7` (before: none).

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
