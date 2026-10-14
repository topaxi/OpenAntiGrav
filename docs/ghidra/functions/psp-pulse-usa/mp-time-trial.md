# Multiplayer Time Trial (mode 17), and no multiplayer Speed Lap (2026-10-14, `h2h-re`; collision and board order `mp-collision-re`)

**Binary:** `pulse-psp` `BOOT.BIN` (USA), image base `0x08804000`. Static only: no ad-hoc
capture path exists here (two PPSSPP instances on an ad-hoc link were not tried), so
nothing on this page is above 88, and that only for the collision byte, which four other binaries agree on.

## Answer

Pulse PSP has an ad-hoc **Multiplayer Time Trial**: 2 to 8 players race a time trial on
one track, each on **their own grid slot**, as **ordinary craft with name tags**, with
**craft-to-craft collision switched off**, weapons off, and the race clock and a place
counter on the HUD. Its record is the lowest **total race time**, the same as a solo Time
Trial. There is **no multiplayer Speed Lap**.

| Question | Answer | Evidence | Conf |
| --- | --- | --- | ---: |
| Mode number | **17** (`0x11`, `Multiplayer Time Trial`) | `g_game_mode_names` (`0x08ab0788`) index 17, constructor `MPTimeTrial_Construct` (`0x08822dd8`), [state-machine.md](state-machine.md) | 90 |
| How it is offered | the ad-hoc `Multiplayer` menu (`Multiplayer_Definition.xml` in `Data.wad`, global `MultiplayerMode`): `Create Single Race`, `Create Tournament`, `Create Time Trial`, `Join Game`. `Create Time Trial` has `Max Players` `2`..`8` (default 8) and `Downloaded Ships` `Allowed`/`Disallowed`, then `Team Selection` | the XML as shipped | 90 |
| Same grid spot or separate rows | **separate**: `MPTimeTrial_Construct` calls `Race_SpawnGrid` (`0x088247e0`), the same call `MPRace_Construct` makes, which gives each racer its own slot through `g_grid_orders` ([grid.md](grid.md)). Which slots a field of N takes was not measured | decompile | 75 |
| Ghosts or solid craft | **ordinary craft**: the racers are the craft `Race_SpawnGrid` spawns (no ghost object is built), and the race loads `MPTag_HUD.xml` (`FUN_0881b86c`), the name tags drawn over the other players' craft, as `MPRace_Construct` does. Whether they are drawn translucent was **not determined**: no mode test was looked for in the craft draw path | decompile | 60 |
| Craft-to-craft collision | **off**. The collision world's constructor (`FUN_088151bc`, stores itself at `0x08ab0824`) sets its byte `+0x5464` to `1`; `MPTimeTrial_Construct` clears it (`sb zero, 0x5464(a0)` at `0x08822e34`), the only other writer. `Collision_DispatchPair` (`0x08816eac`; its one reader is `lbu a0, 0x5464(s2)` at `0x08816ff0`) runs `Collision_BoxAgainstBox` for a pair of type-3 (craft) bodies only when the byte is set; craft against the track mesh is untouched. **Raised 82 to 88 on 2026-10-14**: Pure (USA and EU), HD, Omega and 2048 each have the same three parts (a constructor that sets a world byte, the `MPTimeTrial` constructor that clears it, a pair gate that reads it), found independently in each binary: [HD](../ps3-hdfury-eu/mp-time-trial-collision.md), [Pure](../psp-pure-usa/mp-time-trial-collision.md), [Omega](../ps4-omega-eu/mp-time-trial-collision.md), [2048](../vita-2048-eu-v104/mp-time-trial-collision.md). Not 95: no run | 88 |
| Weapons | off and locked: mode 17 is in `Race_ReadSetupOptions`' default-off and no-override sets (`5, 9, 10, 15, 17`), [state-machine.md](state-machine.md) | decompile | 85 |
| HUD | `Arcade_HUD.xml` (not `TimeTrial_HUD.xml`) with HUD flags `1, 2, 4, 8, 0x10, 0x40, 0x100`: the clock is shown (`PlayerStatus_Update` `0x0883b3b8` keeps a clock target in modes 5, 7, 10 and 17, [hud.md](../../../ui/hud.md)), `0x40` draws the place digits (`POS n / m`), and the per-craft place list (`0x800`) is **not** set. `MPRace_Construct` sets `0x800` (except in Head2Head) and `0x2000`; MP Time Trial sets neither (`0x2000`'s meaning unread) | decompile, both constructors side by side | 80 |
| Ranking | the profile record is the **total race time** (`Race_RecordResult` `0x0880ae54`, `case 2` / `0xe` = modes 5 and 17: keeps the lower `param_2`, `0` or `-1` meaning unset). The in-race place is the race-order place every race shows. **The end board orders the players by race place, which is total time** (read 2026-10-14, [below](#the-end-board-order)) | 75 for the record, 76 for the board |

## The end board order

Read 2026-10-14, static. The ad-hoc results table is `EndRaceResults_PopulateMultiplayerTable`
(`0x088d9588`, [endrace-screens.md](endrace-screens.md)): row `i` is record `i` of the array at
`+0xe4`, stride `0x110`, with the place column simply the row number `i + 1`, the name at record
`+0x35`, the player flag at `+0x34` and the time at record `+0x140` (`ER_DNF` for `-1`,
`ER_RACING` for `0`, otherwise the time). So the board's order is the order the records were
**filled** in, not a sort in the table code.

**Who fills the records.** `0x0882ad3c` (named `Race_BuildMultiplayerResults`) writes
`mgr + 0x7d8 + 0x110 * row` fields `+0x34`, `+0x35`, `+0x138`, `+0x13c` and `+0x140` (the
absolute offsets `0x80c`, `0x80d`, `0x910`, `0x914`, `0x918` in its body: `0x7d8 + 0x34` is
`0x80c` and `0x7d8 + 0x140` is `0x918`, so the two sides of the table agree). It is called every
frame once the manager's state is above 2 by `FUN_088244a8` (`0x088245e0`), the common update that **both**
`MPTimeTrial`'s (`0x0882314c`) and `MPRace`'s (`0x08822348`) per-state function call first. For
every mode except `0x12` (Elimination, which sorts by kills):

1. each craft gets its total race time, `Craft_TotalRaceTime` (`0x08826b18`), as a float key, and
   the keys are sorted ascending (`FUN_088265cc` compares `+4` as floats, smaller first; the
   default element `FUN_08826c74` is index `-1`, key `0x497423f0`, 999,999.0);
2. each craft, in that order, goes to the row `mgr + 0x58 + 4 * craftId` holds. That slot is
   written by this same function: when it is `-1` and the craft has finished (`craft + 0x912`),
   it latches the craft's race place `craft + 0xbd4 - 1` (`+0xbd4` is index plus one, distinct per
   craft, [race-progress.md](race-progress.md)); a craft with the slot still `-1` uses that
   place unlatched. So the row is the place at the moment of finishing;
3. the record gets the name, kills and deaths, and `+0x140` is `Craft_TotalRaceTime` when the
   craft has finished (`craft + 0x912` set), `-2` while still racing and `-1` when destroyed or
   retired (`Ship_State` 6 or 7).

`Race_UpdatePositions` (`0x08827f0c`) keys finished craft by total time in centiseconds, so the
race place is finishing order, which is total-time order from the common start. **The board is
ranked by total race time; a person still racing sits below every finisher.** Not found: who
resets the `mgr + 0x58` slots to `-1`, and what a tie in centiseconds does (the rows are
distinct places, so two equal times do not share a row). Confidence 76: both ends of the
record layout and the whole fill are read, no run.

This agrees with the port (`oag_race::places`, finishing tick, the same order) and needs no change.

**One difference from `MPRace`, observed.** The two update functions differ only in the racing
state's handler: `MPTimeTrial`'s (`0x08823270`) first tests the player's byte `+0x911` and, when it
is set, stores `4` to `*(player + 0x4c) + 0x1bc`; `MPRace`'s (`0x0882246c`) does not. The
finish-wait handlers (`0x08823300`, `0x088224dc`) are identical. What `+0x911` and the stored `4`
are was not determined (a controller mode on the player's control object, a guess), so the port
takes no rule from it.

## No multiplayer Speed Lap

The 19-entry mode table has `Speed Lap` (10) and five `Multiplayer *` modes (`Single Race`,
`Head2Head`, `Tournament`, `Time Trial`, `Elimination`), and no multiplayer Speed Lap; the
ad-hoc menu offers none either. Confidence 88. PS2 Pulse's table is the same 19 names
([../ps2-pulse-eu/head2head.md](../ps2-pulse-eu/head2head.md)) and builds no multiplayer
mode at all.

## The port

**Ported 2026-10-14 (`mp-time-trial-port`), on every title this build runs.** The mapping is
**chosen, not measured**: there is no ad-hoc play and no Multiplayer menu mode here, so a Time
Trial raced by two or more people (`--mode time_trial --players N`, `--window-seats`, the join
screen's seats) runs mode 17's rules, and one person is the ordinary Time Trial, byte for byte
(`Mode::is_multiplayer_time_trial`). The rules are the measured ones above and carry their
confidence; the wiring is ours.

| Rule | What this build does | Where |
| --- | --- | --- |
| Field | the people and nobody else: `people` craft, 2 to 8, whatever the `--opponents` hatch or a class with no driver says | `Race::start`, `Mode::is_multiplayer_time_trial` |
| Grid slots | each person a row of their own, the back rows in seat order (person 1 on the last). **Read as `Race_SpawnGrid` plus "a short grid packs to the back"** ([grid.md](grid.md), 82), which agrees with `oag_gameplay::rank_from_back`; which rows a field of N takes was not measured for any N below eight. [grid.md](grid.md) records Head2Head's field of two on the two *front* places, which conflicts with packing to the back; not resolved here, and mode 17 follows `Race_SpawnGrid` as `MPRace_Construct` does | `rank_from_back` |
| Collision | craft pass through each other, the track still stops them. **Derived state, not a stored byte**: `resolve_craft_pairs` asks `Mode::craft_collide(people)`, and both the mode and the person-flown slots are already in the state hash, so the rule is deterministic and a one-person race hashes exactly as before. The original's byte is `+0x5464` | `Race::resolve_craft_pairs` |
| Weapons | off, as Time Trial already is | `Mode::weapons_enabled` |
| HUD | the title's **arcade** layout, so the clock and `POS n / m` per view (Pulse measured, 80; no place list, no pickup widgets). Every other title takes Pulse's rule, chosen: HD and Omega draw their authored arcade split layout (`PositionPanel`), Pure's, Omega's and 2048's arcade layouts carry the clock and `POS` too, and no fallback layer was needed on any title | `hud_layout_for`, `split_assets` |
| Ranking | the board is `oag_race::places`, which orders finishers by finishing tick: the total race time from the common start. The board names every person by seat | `oag_game::results_tables::people_rows` |
| Records | **a multiplayer run writes no record and no ghost**, the same as every split race. The original keeps the lowest total time in the one profile; here a profile's Time Trial row is one person's, and writing a multiplayer total into it would change that person's solo records. Chosen | `Session::frame` |
| Speed Lap | several people in Speed Lap are left as they were (the hatch fields the AI): the original has none. A multiplayer Speed Lap, with the other players as ghosts on one shared start spot, is the maintainer's mod idea (2026-10-14), not built | - |

Tests: `oag_raceplay`'s `tests::multiplayer_time_trial` (field, rows, pass-through against a
Single Race contact, the time ranking) and the disc-backed
`crates/game/tests/mp_time_trial_ground_truth.rs` (the grid at 2, 5 and 8 people, two craft left
overlapping on Pulse's circuit passing through each other where Head2Head's separate, an
autopiloted two-person run ending on a two-row board in total-time order naming seats 1 and 2).
Looked at on Pulse PSP (EU) and HD at ticks 300 and 1200, on Pulse PS2, Pure, Omega and 2048
at tick 1200 (each shows the clock, the lap and `POS 1 / 2` and `POS 2 / 2`, one per view), and
at the tick two craft overlap under the autopilot (Pulse 362, 1.8 units apart; HD 1435, 2.6):
Pulse's hull draws inside the other's, HD's two hulls visibly interpenetrate.

**2048 / Omega / HD / Pure cross-check (CLAUDE.md), 2026-10-14, static:**

- **HD: ported.** The local rule is title-generic (`Mode::craft_collide`), and HD's own run shows the hulls interpenetrate ("The port" above). The online mode is not built. The `MPTimeTrial` race manager (`0x00041248`,
  mode 19) clears `collision + 0x17778`, which `Collision_ProcessPairs` reads only in the box-box
  case; craft against the track is not gated
  ([../ps3-hdfury-eu/mp-time-trial-collision.md](../ps3-hdfury-eu/mp-time-trial-collision.md), 84).
  `multiplayer_definition.xml` (`DATA02`) offers `Create Time Trial`, 2 to 8 players, Pulse's file
  shape. No online play here; the local split is Pulse's rule on HD's authored arcade split
  layout, **chosen**, and it now agrees with the original's collision.
- **Omega: ported, with a difference.** The rule is title-generic and weapons are off, so it is the same here; the online mode is not built. `multiplayer_definition.xml` (`data09`) is
  byte-identical to HD's. `MPTimeTrialRaceManager_Construct` (`0x0127ecd0`) clears
  `collision + 0x8890`; **checked, differs in width**: the byte also gates box-against-sphere and
  a third dynamic pair, not box-box alone, which changes nothing here because weapons are off
  ([../ps4-omega-eu/mp-time-trial-collision.md](../ps4-omega-eu/mp-time-trial-collision.md), 74).
- **2048: ported (corrected from "checked, differs").** The executable still carries
  `Backend/General/MPTimeTrial_RaceManager.cpp`; its constructor `FUN_8116385c` clears the same
  kind of byte (`+0x7448`) and the by-mode factory builds it for mode id `0x13`. What 2048 does
  not have is a way in that was found: no `multiplayer_definition.xml`, and `MP.xml` is a season
  ladder of XP and par-time levels ([2048-campaign.md](../../../formats/2048-campaign.md)). So the
  class exists and its reachability is **not shown**
  ([../vita-2048-eu-v104/mp-time-trial-collision.md](../vita-2048-eu-v104/mp-time-trial-collision.md), 74).
  The local split borrows HD's, **chosen**.
- **Pure: ported.** The rule is title-generic; ground truth runs on Pulse and HD only. The online mode is not built. `MPTimeTrial_Construct` (USA `0x08815e1c`, EU
  `0x08815c34`) clears `world + 0x3664`, and `Collision_DispatchPair` runs `Collision_BoxAgainstBox`
  only when it is set, the same as Pulse
  ([../psp-pure-usa/mp-time-trial-collision.md](../psp-pure-usa/mp-time-trial-collision.md), 82;
  [EU](../psp-pure-eu/mp-time-trial-collision.md), 80). Pulse's rules for the local split are
  now the original's own on Pure too, no longer assumed.

No title keeps craft collision on in its Time Trial, so no port follow-up note
is needed.

## Next address to try

- Translucency: the craft draw call for a non-local racer, checked for a `g_game_mode == 0x11`
  test (start from the `MPTag_HUD.xml` owner `FUN_0881b86c` and the craft render pass).
- Who resets the latched place slots `mgr + 0x58` to `-1`, and whether a tied centisecond total
  changes a row ([the board](#the-end-board-order)); and what `player + 0x911` and the `+0x1bc = 4` store in
  `MPTimeTrial`'s racing state are.
- A live check needs two PPSSPP instances on one ad-hoc link.
