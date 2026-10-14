# Two-person Head2Head on PS2 Pulse (2026-10-14, `h2h-re`)

**Binary:** `SCES_547.48` (`/pulse/SCES_547.48-ps2-pulse-eu`), image base `0x00100000`.
The single-player mode is on the PSP page,
[`../psp-pulse-usa/head2head.md`](../psp-pulse-usa/head2head.md); this page is what the
PS2 adds when two people race it in a split screen.

## Answer

**The PS2's two-person Head to Head is mode 9 (`Head2Head`) with the split flag set, not
mode `0xf` (`Multiplayer Head2Head`).** Two people, no AI, weapons off and locked, each
view reads the gap to the other person, the race ends when both have crossed the line,
and each person's result is recorded. Mode `0xf` builds no race at all on this binary; on
the PSP it is the ad-hoc network race (`MPRace_Construct`), and it only shares the HUD
swap. Confidence per claim in the tables below; everything here is **decompile only**,
one binary (the PSP has no local split to corroborate), so nothing is above 84.

## How the split race gets mode 9

1. `Split ScreenV` and `Split ScreenH` (`Data\Plugins\PI001\GUI\RaceBox_Definition.xml` in
   `54748/WADS2.WAD`) each carry `<List name="Mode" global="Mode" default="Arcade">` with
   three entries, `idstring` `Arcade`, `Tournament`, `Head2Head`, and no `value`, so the
   global `Mode` holds the entry's own name. The list's `GSDisableEntriesBitField="0xFA"`
   leaves bit 2 (Head2Head) clear under the "bit N gates entry N" reading of
   [race-setup.md](../../../formats/race-setup.md#gsdisableentriesbitfield).
2. Entering either screen sets **`g_split_mode`** (`0x002dab3c`): `FUN_001c10a8`
   compares the screen name, `Split ScreenV` (`0x002b68b0`) gives `2`, `Split ScreenH`
   (`0x002b68c0`) gives `1`; `FUN_001be2b0` does the same for a second pair of screen
   names. `FUN_001bf550` clears it (`0`) and sets it from `DAT_0027fb30` (`1 -> 2`,
   `2 -> 1`). Demo mode clears it (`FUN_00123cc0`, `case 2`).
3. **`Race_ReadSetupOptions`** (`0x001868b8`) reads `Mode` through
   **`GameMode_FromName`** (`0x00100600`: `strcasecmp` over the 19 names at
   **`g_game_mode_names`** `0x0027a5e8`, the PSP's table in the PSP's order) and, only
   when the index is below `0xd`, stores it through `FUN_00100e60` at
   `0x002dab08 + 0xc8` = **`g_game_mode`** (`0x002dabd0`). `FUN_00100e60` has this one
   caller and Ghidra finds no other write to `0x002dabd0`, so **a `Multiplayer *` mode
   (`0xe..0x12`) is never set on this binary.**
4. **`Race_CreateModeObject`** (`0x00124f80`) has cases `1..10` and nothing for
   `0xe..0x12` (no race object), and passes `g_split_mode` to `FUN_00147040`, which stores
   it at `+0xc7c` of the object `DAT_002e0cc8` points at (`0x00147298`, `sw s4` with
   `s4 = a1`). That `+0xc7c` is the split value `ArcadeRace_Construct` branches on, which
   [grid-local-players.md](grid-local-players.md) left unread. The InGame state's own
   switch (`FUN_00123cc0`) lists `0xe..0x12` as an explicit empty case.

| Claim | Evidence | Conf |
| --- | --- | ---: |
| The split screens' `Mode` list sets `Head2Head` by name | the XML above; `Race_ReadSetupOptions` reads global `"Mode"` (`0x002ac4d8`) | 82 |
| `GameMode_FromName` = `strcasecmp` over 19 names, index or `-1` | `0x00100600`, the PSP's `GameMode_FromName` (`0x08805628`) instruction for instruction in shape, same table order | 85 |
| `g_game_mode` is written only from the `Mode` name, and only below `0xd` | one setter, one caller, `if (index < 0xd)` at `0x001868b8` | 80 |
| Mode `0xf` builds no race on PS2 | `Race_CreateModeObject` and `FUN_00123cc0` have no case for it | 82 |
| `g_split_mode`: `0` one person, `1` Split ScreenH, `2` Split ScreenV | `FUN_001c10a8`, `FUN_001be2b0`; the HUD and the race read `== 1` / `!= 0` consistently | 80 |
| `+0xc7c` (the race's split branch) is `g_split_mode` | `FUN_00147040` stores `a1` there; `Race_CreateModeObject` passes `DAT_002dab3c` | 82 |

This corrects [camera.md](camera.md) and the README's hypotheses table, which read
`0x002dab3c` as an attract/demo switch at 40: it is the split mode, which is why
`0x0014f108` picks a per-player view setting when it is non-zero.

## Who is on the grid

**`Race_ReadSetupOptions` sets the grid size from the mode:**

| Mode | Grid (`g_racer_count`, `0x002dab08`) |
| --- | --- |
| 3, 8, 12, 14, 16, 18 | 8 (then `Opponents + 1` if the XML sets `Opponents` and the grid read back as 8) |
| 4 (`Tournament`) | by `SkillLevel` (`DAT_002dabcc`): `0` -> 4, `1` -> 6, `2+` -> 8 (**PS2 only**; the PSP's table gives 4 a flat 8) |
| 9 (`Head2Head`) | **2** |
| everything else, `0xf` included | 1 |

The write goes through **`GameSetup_SetGridSize`** (`0x00100e70`), which caps the value
at `DAT_0027ddc8` and forces `1` when `DAT_0027a5c4` is set. The `Opponents` override
cannot fire for Head2Head (the grid is 2, not 8).

**`ArcadeRace_Construct`** (`0x0012ad78`, the case for modes 3, 9 and 12) then branches
on the split value:

```c
// N = g_racer_count
if (split == 0) {
    for (i = 0; i < N - 1; i++) Race_CreateAiRacer(mgr, i, 0, i, team[i]);
    Race_CreatePlayer(mgr, N - 1, 0, N - 1, team0_name);
} else if (split == 1 || split == 2) {
    for (i = 0; i < N - 2; i++) Race_CreateAiRacer(mgr, i, 0, i, team[i]);   // none when N == 2
    Race_CreatePlayer(mgr, N - 1, 0, N - 1, team0_name);                    // person 1
    Race_CreatePlayer(mgr, N - 2, 1, N - 2, team1_name);                    // person 2
}
```

`Race_CreatePlayer` stores its slot argument at `mgr + 0x2be4 + 4 * racer` with no
compaction, and `Race_PlaceGrid` (`0x00128158`) reads it back as the matrix index. So in
a two-person Head2Head **person 1 starts on grid matrix 1 and person 2 on matrix 0**: the
two front positions of the eight-place layout (matrix 7 is the authored `start position`
node, the back, and each lower index steps `19.8` further up the grid,
[grid-local-players.md](grid-local-players.md)). The one-person Head2Head on this binary
puts the AI on matrix 0 and the player on matrix 1, the same two places. The PSP's
`Race_SpawnGrid` instead bubbles assigned slots toward the high end (the back,
[grid.md](../psp-pulse-usa/grid.md)); **PS2 and PSP differ here**, and the PS2's
`Race_PlaceGrid` was read in full for it.

| Claim | Evidence | Conf |
| --- | --- | ---: |
| Mode 9 sets a grid of 2, mode `0xf` a grid of 1 | `Race_ReadSetupOptions` `0x001868b8`, the PSP's table row for row except mode 4 | 85 |
| Two people, no AI | `ArcadeRace_Construct`'s split branch with `N = 2` | 82 |
| Person 1 on matrix 1, person 2 on matrix 0, no compaction | `Race_CreatePlayer` writes the slot, `Race_PlaceGrid` reads it | 78 |
| Matrix 0 is the front | rests on [grid-local-players.md](grid-local-players.md)'s "matrix 7 at the node, the back" (85) | 72 |
| Each person flies their own team | `Race_ReadSetupOptions` reads `Team%d` (`0x002ac4b0`) once per person (`(g_split_mode != 0) + 1` times); `team0_name`/`team1_name` are `DAT_002dabd4[0/1] + 0x78` | 75 |

## Weapons and damage

Same switches as the PSP's: mode 9 is in the weapons default-off set (`5, 9, 10, 15, 17`)
and in the set where the XML's `Weapons` cannot turn them back on (`5, 8, 9, 10, 15, 17`),
so a split Head2Head has **no weapons, whatever the split screen's own `Weapons` row
says**. Damage defaults on (off only for `5, 7, 10`) and the XML `Damage` may change it;
the split screens author no `Damage` row, so it stays on. Confidence 84
(`0x001868b8`, the PSP's [state-machine.md](../psp-pulse-usa/state-machine.md) table
agrees row for row).

## The HUD: each view reads the gap to the other person

**`Hud_BindWidgets`** (`0x00139ea8`, one HUD per view, `hud+0x2b8` the view's person
index, `hud+0x2b4` the mode) does the PSP's swap for `mode == 9 || mode == 0xf`: hides
`PositionTxt`, puts `Position Outof` on `Position`'s `x` and alignment at scale `0.8`,
binds `PositionOf` (the gap label) and `HeadToHeadBar` (`0x002a3b50`, one xref). The
per-tick update `FUN_001371c8` is left unnamed for
the same reason as the PSP's `FUN_0881d458` (it also carries the `0x800` position-list
branch). Its two-person branch:

```c
// pair = the first two craft (FUN_001286f0)
gap  = abs(pair[0]->+0xb98 - pair[1]->+0xb98);
half = gap * 0.5;
if (g_split_mode == 1) {           // Split ScreenH: two short views
    half = clamp(half, 40, 85);
    row1.y = bar.GetY();           // the leader row sits on the bar's top
} else {                            // one person, or Split ScreenV
    half = clamp(half, 60, 180);
    row1.y = 49;
}
row2.y = half;
if (g_split_mode != 0) {
    me    = the pair member whose +0x3f4 == hud+0x2b8;   // this view's own person
    other = the other member;
    leading = me->+0xc94 == 1;
    names: me->+0x828, other->+0x828
}
// leading:  row1 = "1ST <me>"   green 0xff30ff30, throb flag 0x200; row2 = "2ND <other>" white
// trailing: row1 = "1ST <other>" white; row2 = "2ND <me>" red 0xffff3030, throb flag 0x200
// gap label "%s%3.0fm": "-" when this view's place is 1, else "+", coloured like my row
// gap label y = (bar.GetY() + half) / 2; bar height = half - bar.GetY()
```

| Claim | Evidence | Conf |
| --- | --- | ---: |
| The split race takes the same Head2Head swap (mode 9) | `Hud_BindWidgets` reads `hud+0x2b4` at `0x0013acc4` and tests `9`/`0xf`; `hud+0x2b4` is `g_game_mode` stored at `0x00139f00` | 84 |
| Each view finds its own person by `craft+0x3f4 == hud+0x2b8` and reads its own lead | `FUN_001371c8`, split branch | 80 |
| `Split ScreenH` clamps the half gap to `[40, 85]` and lifts the leader row onto the bar's top; `Split ScreenV` keeps `[60, 180]` and `y = 49` | `FUN_001371c8`, literals `0x42200000`, `0x42aa0000`, `0x42700000`, `0x43340000`, `0x42440000` | 82 |
| The row text is `"%s %s"` = `IG_HUD_1ST`/`IG_HUD_2ND` + the craft's `+0x828` name, in a split for both rows | `0x002a3810`, `0x002a3818`, `0x002a3828`; one-person: the player's row is the profile tag `g_settings + 0x457` | 75 |
| What `+0x828` holds for a person's craft | the name `Race_CreatePlayer` was given, `DAT_002dabd4[person] + 0x78` (the team record); the HUD passes the trailing name through the string table (`FUN_00184520`) when the mode is below `0xe` | 65 |
| Colours and the sign strings are the PSP's | `0xff30ff30`, `0xffff3030`, `"-"`/`"+"` at `0x002a3848`/`0x002a3850` | 85 |

## How it ends and what it writes

**The race ends when both people have finished.** The Arcade race's per-person update
(**`ArcadeRace_UpdateFinish`**, `0x0012b2c0`) waits for its own craft to finish (state
`2` or flag `0x1000` at `+0x8f0`); with two people (`mgr+0x2c5c == 2`) it also requires
the other person's craft (`racer ^ 1`) to have finished before it calls the result step
(`FUN_00126028` through `FUN_00126e40`) and moves the race to state `3`. Nobody's race
ends early: a person who finishes first keeps watching until the other crosses.
Confidence 78.

**The result step (`FUN_00126028`) runs once per person** (`mgr+0x2c58` = the person),
takes that person's place from `craft+0xc98`, and calls:

- **`Race_RecordResult`** (`0x001054c8`): for modes `3, 9, 0xe, 0xf` (the PSP's shared
  arm) it writes the finishing place into the track's best-place record with the profile
  name at `g_settings+0x478` when it beats the stored one, then the medal and the
  gold/silver/bronze counters. The arguments carry the person's index but the record is
  the one profile (`g_settings`): **both people's places land in the same profile's
  record.** Confidence 70 (the person index is passed and not used in the body).
- `FUN_00104c88`: a points total for the person's team (`DAT_002dabd4[person]`), weighted
  for modes `3, 4, 9` by `+0x11e8 * 15 + +0x11ec * 25`, then multiplied by a factor from
  `+0x1204` (`1 -> 3`, `2 -> 4`, else `2`). Read, not interpreted; the fields are unnamed.
  Confidence 55, no name.

After the last person's step (`mgr+0x2c58 == people - 1`) it arms `Race End Save`
(`FUN_00129c00`); the end-of-race screens with a split are `EndRace Results Split` and
`EndRace Rewards Split` (`FUN_00125e78` picks them when `g_split_mode != 0`;
[split-end-race.md](../../../ui/split-end-race.md)). **Who wins** is the place order, as
in any race: `IG_HUD_1ST` goes to the craft with `+0xc94 == 1`. No campaign cell is
involved, so the gold-or-nothing medal rule of a Head2Head *cell* does not apply.

## Functions

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x00100600` | `GameMode_FromName` | 85 | `strcasecmp` over `g_game_mode_names`, 19 entries, the PSP's `GameMode_FromName` |
| `0x00100e70` | `GameSetup_SetGridSize` | 75 | the grid-size writer: `min(value, DAT_0027ddc8)`, `1` when `DAT_0027a5c4` is set |
| `0x001868b8` | `Race_ReadSetupOptions` | 82 | reads `Team%d`, `SkillLevel`, `Livery`, `Mode`, `Class`, `Track`, `Opponents`, `Laps`, `Tournament`, `Weapons`, `Damage`; the PSP's `Race_ReadSetupOptions` switches |
| `0x00124f80` | `Race_CreateModeObject` | 80 | the mode switch building each race object; no case for `0xe..0x12` |
| `0x0012ad78` | `ArcadeRace_Construct` | 80 | the case for 3, 9 and 12; one-person and two-person grid branches |
| `0x0012b2c0` | `ArcadeRace_UpdateFinish` | 72 | the per-person finish test, waiting for both people |
| `0x00139ea8` | `Hud_BindWidgets` | 80 | binds every HUD widget by `HUD->` name, the mode 9/`0xf` swap; the PSP's `Hud_BindWidgets` |
| `0x001054c8` | `Race_RecordResult` | 78 | switches on the mode with the PSP's `3/9/0xe/0xf` arm writing the best place |

| Address | Data | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x0027a5e8` | `g_game_mode_names` | 88 | 19 pointers, `Debug` .. `Multiplayer Elimination`, the PSP's table |
| `0x002dabd0` | `g_game_mode` | 85 | `0x002dab08 + 0xc8`, written by `Race_ReadSetupOptions` only; 150 readers |
| `0x002dab08` | `g_racer_count` | 82 | the grid size every constructor loops over; first word of the setup block |
| `0x002dab3c` | `g_split_mode` | 80 | `0`/`1` (`Split ScreenH`)/`2` (`Split ScreenV`), see above |

**Not renamed:** `FUN_001371c8` (the per-tick place and gap update), `FUN_00126028`
(the per-person result step: "store result" fits, but it also does the Elimination and
Tournament tables), `FUN_00147040` (constructs the object that keeps the split value at
`+0xc7c`), `FUN_00104c88` (team points).

## Not verified live

No PCSX2 run reached a two-person Head2Head on this pass. What a live run should read at GO in
a `Split ScreenV` Head2Head: `0x002dab08 == 2`, `0x002dabd0 == 9`, `0x002dab3c == 2`; and
the two craft on the two front grid places.
