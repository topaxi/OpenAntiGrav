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

## Next address to try

- Translucency: the craft draw call for a non-local racer, checked for a `g_game_mode == 0x11`
  test (start from the `MPTag_HUD.xml` owner `FUN_0881b86c` and the craft render pass).
- The end-of-race order: `MPTimeTrial_Construct`'s vtable at `0x08ac8054` (its finish
  method), against `MPRace_Construct`'s at `0x08ac7e54`.
- A live check needs two PPSSPP instances on one ad-hoc link.
