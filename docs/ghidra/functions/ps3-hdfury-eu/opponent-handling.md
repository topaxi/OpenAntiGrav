# Whose handling an opponent flies, and which Fury variant (2026-10-12)

**Binary:** `/hdfury/EBOOT-ps3-hdfury-eu.elf`. Static only: no RPCS3 run was
made for this page. TOC-relative strings were resolved with
`scripts/ps3-toc.py resolve <function> <disp>`, never with Ghidra's own
TOC (see [memory.md](memory.md)).

The same question for Pulse is answered and measured live in
[grid.md](../psp-pulse-usa/grid.md#each-craft-flies-its-own-teams-handling-measured-2026-10-12):
each craft flies its own team's file. HD keeps the shape.

## Answer

- **Each AI craft is built from its own team name, and its handling is looked
  up by that name.** Confidence **75**: decompile only, two spawners and the
  constructor consistent, and the same shape Pulse was measured to have.
- **An AI's Fury variant (`normal`, `concept1`, `nitro`) is chosen per AI**,
  and in the single-player path it follows the variant the human entrants
  chose when they all chose the same one. In single player that table has
  one entry, so the test always passes: **the AI flies the player's variant
  class** (player `concept1` gives AI `concept1`, `nitro` gives `nitro`,
  `normal` gives `normal` or a team model outside the named variants), and `Team_LoadHandlingStats` reads that variant's
  directory, so the AI flies **its own team's file in the player's variant**.
  Confidence **55**: what the entrant table at `g_GameState + 0x134` holds is
  inferred, not read.
- **A team's handling file is loaded once per team, from the first craft's
  variant.** Confidence **70**. If two crafts of one team carry different
  variants, the second one flies the first one's file. Whether a race ever
  fields that was not checked.

## The constructor: `Ship_Construct` (`0x000ddd58`)

Already named on [particle-triggers.md](particle-triggers.md). Arguments
`(ship, role, slot, ?, team_name, variant_model, ?)`; the AI spawner passes
role `2`, `0x00044a00` passes role `1`. In order:

```c
ship->team /* +0x6294 */ = Team_FindByName(team_name, 0);          // 0x0015de70
if (!ship->team) { team_name = "Feisar"; ship->team = Team_FindByName("Feisar", 1); }
if (variant_model) ship->variant /* +0x6298 */ = <the team's model whose name matches variant_model>;
Team_LoadHandlingStats(ship->team, ship->variant ? ship->variant->dir /* +0x1b4 */ : 0);  // 0x001e6ff8
engine = new ShipEngine;  ship->engine /* +0x5fac */ = engine;
sprintf(key, "%s HANDLING STATS", team_name);                     // -0x4544(r2) -> 0x008a8f94 -> 0x007823f0
engine->stats /* +0x78 */ = registry_lookup(key);
Ship_ReloadModelForSkin(ship, team_name);
```

`-0x454c(r2)` resolves to `"Feisar"` (`0x007823e8`): the fallback is Feisar,
as on Pulse, not the player's team.

`Team_FindByName` (`0x0015de70`) walks the team list and returns the first
team whose `+0x78` name compares equal to the argument.

`Team_LoadHandlingStats` (`0x001e6ff8`) does nothing twice for one team
(`team+0xb8` is a loaded flag, set at the end). The first time, it registers
the team's stats block under `"%s HANDLING STATS"` with the team's `+0x78`
name, then picks the file: Detonator mode (`0xe`) reads
`Data\Ships\Detonator\handlingstats.xml`, Zone Battle (`0xd`)
`Data\Ships\Zone Battle\handlingstats.xml`, the three Zone modes (mask
`0x206040`) `Data\Ships\Zone\handlingstats.xml`; otherwise
`"%s\handlingstats.xml"` formatted with the variant's directory when the ship
has a variant, else with the team's own `+0xa4` location.

## The spawners

`SPArcadeRaceManager_Construct` (`0x00061770`; its `+0x30` is the
`SPArcade_RaceManager.cpp` `__FILE__` string) builds the field from a per-slot
array of team names on the session object (`*piVar13`):

```c
for (i = 0; i < racers - 1; i++)
    Race_CreateAiRacer(mgr, i, 0, i, session->team[i] /* +0x44 + 4*i */, -1);   // 0x00058e38
FUN_0005cb40(mgr, racers - 1, 0, racers - 1, session->team_last /* +0x60 */, 0);   // the player
```

That is Pulse's `ArcadeRace_Construct` loop, slot for slot, with the player
last. Two-player splits the last two slots (`+0x60` and `+0x5c`). Nine other
callers of `0x00058e38` (`0x000623a8`, `0x00069780`, `0x00069ee8`,
`0x0006e738`, `0x0006ee38`, `0x00073428`, `0x00074600`, `0x0007d4a0`,
`0x0007d8e0`) are the other modes' race managers and were not read. **Who
fills `session->team[]` was not traced** on HD; on Pulse it is
`RaceSession_DrawAiRoster`, which excludes the player's team.

`Race_CreateAiRacer` (`0x00058e38`) names the input `AI_input_%d`, calls
`Ship_Construct(ship, 2, slot, ?, team_name, variant, ?)` with the team name it
was given, and registers the ship with `AiManager_AddDriver`
([ai-controls.md](ai-controls.md)). The variant it passes, strings resolved
through its own TOC (`-0x6aec` `"concept1"`, `-0x6ae8` `"nitro"`, `-0x6af0`
`"normal"`, `-0x6af4` `"chrome_c1"`, `-0x6af8` `"chrome"`, `-0x6afc` `"zone"`,
`-0x6b00` `"detonator"`):

- Outside the networked case and outside mode `0xd`, it first looks for a
  model of the AI's own team whose name is none of those five special names,
  with a per-slot divisibility gate (`slot_field % n == 0`, `n` counting from
  6) that was not understood.
- Then it counts the entries of a table at `g_GameState + 0x134` (stride
  `0x54`, count byte at `g_GameState + 0xe4`) by name: `concept1` or
  `chrome_c1`, `nitro`, other. **If every entry is `concept1`-like the AI gets
  `concept1`; if every entry is `nitro` it gets `nitro`;** if every entry is
  the other kind it keeps the model found above unless that is `concept1` or
  `nitro`, in which case `normal`. A mixed table keeps the model found above,
  or `normal`.
- Mode `2` with no model found picks `concept1` or `normal` on `Libc_Rand() & 1`.

Reading the `+0x134` table as "the human entrants' chosen variants" fits the
count byte (1 in single player, 2 in split screen at `0x00061770`'s
`(byte)g_GameState[0xe4] < 2` test), but its writer was not read. That is why
the variant rule sits at 55.

## What would falsify this

A live HD race (RPCS3, the patched watchpoint build) where an AI ship's
`engine+0x78` equals the player's, or holds the player's team's values. The
cheap check is the same as Pulse's: read each ship's `+0x6294 -> +0x78` name
and `+0x5fac -> +0x78` stats pointer on the grid and compare one stat that
differs between two teams' files.

## 2048 and Omega

**Not checked**: no 2048 or Omega executable was opened in this lane. Omega
runs HD's front end, so the race-manager shape is the likelier carry-over; it
is unverified.

## Names landed

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0015de70` | `Team_FindByName` | 80 |
| `0x001e6ff8` | `Team_LoadHandlingStats` | 80 |
| `0x00058e38` | `Race_CreateAiRacer` | 75 |
| `0x00061770` | `SPArcadeRaceManager_Construct` | 78 |

`0x00044a00` (role `1`) and `0x0005cb40` (the last slot, called with the
player's string) are left unnamed: neither body was read past its call into
`Ship_Construct`.
