# Multiplayer Time Trial (mode 17), and no multiplayer Speed Lap (2026-10-14, `h2h-re`)

**Binary:** `pulse-psp` `BOOT.BIN` (USA), image base `0x08804000`. Static only: no ad-hoc
capture path exists here (two PPSSPP instances on an ad-hoc link were not tried), so
nothing on this page is above 84.

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
| Craft-to-craft collision | **off**. The collision world's constructor (`FUN_088151bc`, stores itself at `0x08ab0824`) sets its byte `+0x5464` to `1`; `MPTimeTrial_Construct` clears it (`sb zero, 0x5464(a0)` at `0x08822e34`), the only other writer. `Collision_DispatchPair` (`0x08816ff0`) runs `Collision_BoxAgainstBox` for a pair of type-3 (craft) bodies only when the byte is set; craft against the track mesh is untouched | three `search_instructions` hits on `0x5464(`, all read | 82 |
| Weapons | off and locked: mode 17 is in `Race_ReadSetupOptions`' default-off and no-override sets (`5, 9, 10, 15, 17`), [state-machine.md](state-machine.md) | decompile | 85 |
| HUD | `Arcade_HUD.xml` (not `TimeTrial_HUD.xml`) with HUD flags `1, 2, 4, 8, 0x10, 0x40, 0x100`: the clock is shown (`PlayerStatus_Update` `0x0883b3b8` keeps a clock target in modes 5, 7, 10 and 17, [hud.md](../../../ui/hud.md)), `0x40` draws the place digits (`POS n / m`), and the per-craft place list (`0x800`) is **not** set. `MPRace_Construct` sets `0x800` (except in Head2Head) and `0x2000`; MP Time Trial sets neither (`0x2000`'s meaning unread) | decompile, both constructors side by side | 80 |
| Ranking | the profile record is the **total race time** (`Race_RecordResult` `0x0880ae54`, `case 2` / `0xe` = modes 5 and 17: keeps the lower `param_2`, `0` or `-1` meaning unset). The in-race place is the race-order place every race shows. How the end-of-race board orders the players (finish order or time) was **not read** | decompile | 75 for the record, unread for the board |

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

**2048 / Omega / HD cross-check (CLAUDE.md), read 2026-10-14 off the shipped data:**

- **HD: checked, applies, not wired.** The executable has an `MPTimeTrial` race manager
  (mode 19, `0x00040f00`; [mode-manager.md](../ps3-hdfury-eu/mode-manager.md)), and
  `multiplayer_definition.xml` (`DATA02`) offers `Create Single Race`, `Create Tournament`,
  `Create Time Trial`, `Join Game`, with `Max Players` 2 to 8 on `Create Time Trial`: Pulse
  PSP's file shape, so the online mode is in HD's code and data. Its constructor was not read
  (does it also clear a craft-pair byte? unknown), and there is no online play here. The local
  split is Pulse's rule on HD's authored arcade split layout, **chosen**.
- **Omega: checked, applies, not wired.** `multiplayer_definition.xml` (`data09`) is
  **byte-identical** to HD's (18,226 bytes, `diff` empty), `Create Time Trial` included.
  Whether Omega's executable carries an `MPTimeTrial` class was not looked for here.
- **2048: checked, differs.** No multiplayer Time Trial: `MP.xml` is a season ladder of XP and
  par-time levels (`MP_S12_E02` and siblings, [2048-campaign.md](../../../formats/2048-campaign.md)),
  its executable names six `GameMode_*` classes and none is a Time Trial or a multiplayer one
  ([head2head.md](../ps3-hdfury-eu/head2head.md#census-pure-2048-omega)), and no
  `multiplayer_definition.xml` is listed in its archives. Which mode each MP level runs was not
  read. The local split borrows HD's, **chosen**.
- **Pure: checked, applies, not wired.** Its executable's race managers include `MPTimeTrial`
  beside `MPArcade` and `MPTournament` (string census, [head2head.md](../ps3-hdfury-eu/head2head.md#census-pure-2048-omega),
  85); the constructor was not read, so Pulse's rules are assumed for the local split,
  **chosen**.

## Next address to try

- Translucency: the craft draw call for a non-local racer, checked for a `g_game_mode == 0x11`
  test (start from the `MPTag_HUD.xml` owner `FUN_0881b86c` and the craft render pass).
- The end-of-race order: `MPTimeTrial_Construct`'s vtable at `0x08ac8054` (its finish
  method), against `MPRace_Construct`'s at `0x08ac7e54`.
- A live check needs two PPSSPP instances on one ad-hoc link.
