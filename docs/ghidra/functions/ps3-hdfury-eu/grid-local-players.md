# Where the local players start on the grid (HD / Fury, 2026-10-13)

**Binary:** `/hdfury/EBOOT-ps3-hdfury-eu.elf` (TOC `r2 = 0x008ad4d8`, below `0x260000`). Static only: no
RPCS3 run was made for this page, because the capture needs a second virtual pad and `scripts/rpcs3_pad.py`
drives one (open item in the lane report, not a negative). The same question on PS2 Pulse, read the same way
and agreeing: [`../ps2-pulse-eu/grid-local-players.md`](../ps2-pulse-eu/grid-local-players.md). The PSP's
single-player grid is [`../psp-pulse-usa/grid.md`](../psp-pulse-usa/grid.md).

## Answer

**With two local players, person 1 starts on the back row (grid slot 8) and person 2 on the row before it
(slot 7); the AI take slots 1 to 6.** Confidence **85**: a full decompile of the race manager that spawns the
race, the same shape in a second binary (PS2 Pulse), and the single-player half of the same code matches a
live measurement (the player last on a full grid). Not watched with two pads on a running HD.

The race manager's constructor (`SPArcade`, `0x00061770`, and its twin `0x000623a8`, `Elimination` and
`Nitro` the same shape) spawns by the number of local players, `byte(g_GameState + 0xe4)`:

```c
// N = g_GameState[0], the ships on the grid (8 on a full one)
if (local_players < 2) {                       // bVar1
    for (i = 0; i < N - 1; i++) Race_CreateAiRacer(mgr, i, 0, /*slot*/ i, team_entry[i], -1);
    Race_CreatePlayer(mgr, N - 1, 0, /*slot*/ N - 1, entry[+0x60], /*local index*/ 0);
} else {
    for (i = 0; i < N - 2; i++) Race_CreateAiRacer(mgr, i, 0, /*slot*/ i, team_entry[i], -1);
    Race_CreatePlayer(mgr, N - 1, 0, /*slot*/ N - 1, entry[+0x60], 0);   // person 1: the back
    Race_CreatePlayer(mgr, N - 2, 0, /*slot*/ N - 2, entry[+0x5c], 1);   // person 2: the row before
}
```

- **The slot is the racer index.** There is no permutation table on this path (the `g_grid_orders`
  table at `0x00765938` is read only by the online `MP*` managers, `0x000453f8`).
- **Slot `N - 1` is the back row.** `Race_ComputeGridLayout` fills slot 8 at the authored node and steps
  forward 19.8 per slot ([grid.md](../psp-pulse-usa/grid.md)); PS2's copy of that routine writes matrix
  index 7 at the node and index 0 furthest forward.
- **The AI count is `N - 2` with two people, not `N`:** the field stays `N` craft.
- **Person 1 is local index 0** (`Ship_Construct`'s last argument, `ship + 0x7a60`) and person 2 index 1;
  the `0x5c` and `0x60` entries are racers 6 and 7's picks, so the order of the team picks follows the racer
  index.

## Functions

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x0005cb40` | `Race_CreatePlayer` | 80 | `(mgr, racer, 0, slot, entry, local_index)`: stores `slot` at `mgr + 0x2d78 + 4 * racer`, builds `Ship_Construct(ship, role 0, racer, ..., local_index)`, its input and camera. The PSP's `Race_CreatePlayer` (`0x0882821c`) has the same arguments and the same job; the AI spawner `Race_CreateAiRacer` (`0x00058e38`) passes role 2. |
| `0x00058600` | `Race_PlaceGrid` | 78 | After the layout, loops `g_GameState[0]` ships: reads each ship's slot from `mgr + 0xe8 + 0x2c90 + 4 * i` (`0x00058798`), drops the slot's matrix by a raycast, `Ship_SetState`, records the slot at `ship + 0x7004`. The PSP's `Race_PlaceGrid` does the same. |
| `0x00057ae0` | `Race_ComputeGridLayout` | 72 | Looks up the `"start position"` registry entry (TOC slot `0x008a6958`, the `lwz` at `0x00057ba8`) and is the only producer of the matrix buffer `Race_PlaceGrid` reads. The body was not read in full. |

## Checked against Omega and 2048

Both ship the same race manager classes (`SPArcade_RaceManager.cpp` and the others, found by string in each
binary), so the rule is expected there: **checked, applies, not decompiled** (their spawners were not read).
Our Omega and 2048 take HD's rule, labelled as borrowed.

## Not read

- A live two-pad run on RPCS3 (the way to raise the 85).
- `SPTournament` (the campaign), which spawns by a different shape, and the modes with one `Race_CreatePlayer`
  call (Time Trial, Free Play, Zone, Detonator), which have no split.
- What sets `g_GameState[0]` for a split race (the rule above assumes the field stays 8).
