# Where the local players start on the grid (PS2 Pulse, 2026-10-13)

**Binary:** `SCES_547.48` (`/pulse/SCES_547.48-ps2-pulse-eu`), image base `0x00100000`. Static only: no
PCSX2 run was made for this page. The same question on HD, with the full account:
[`../ps3-hdfury-eu/grid-local-players.md`](../ps3-hdfury-eu/grid-local-players.md).

## Answer

**With two local players, person 1 starts on slot 8 (the back) and person 2 on slot 7; the AI take slots 1
to 6** - the same as HD/Fury. Confidence **85**, decompile only, two independent race-manager constructors
that agree and a second binary that agrees.

`FUN_0012ad78` and `FUN_0012b880` (two race modes; six more callers of `FUN_00128860` have the same
shape) branch on `*(DAT_002e0cc8 + 0xc7c)`, the split-screen setting (`0` one view, `1` and `2` two views):

```c
// N = DAT_002dab08, the ships on the grid
if (split == 0) {
    for (i = 0; i < N - 1; i++) Race_CreateAiRacer(mgr, i, 0, /*slot*/ i, team[i]);
    Race_CreatePlayer(mgr, N - 1, 0, /*slot*/ N - 1, name0);               // the back
} else if (split >= 0 && split < 3) {
    for (i = 0; i < N - 2; i++) Race_CreateAiRacer(mgr, i, 0, /*slot*/ i, team[i]);
    Race_CreatePlayer(mgr, N - 1, 0, /*slot*/ N - 1, name0);               // person 1: the back
    Race_CreatePlayer(mgr, N - 2, 1, /*slot*/ N - 2, name1);               // person 2: the row before
}
```

The layout routine `FUN_00127ad0` writes matrix index 7 at the authored node and steps forward `19.8`
(`0x419e6666`) per index, with the `10.0` lateral literal and the `50.0`/`5.0` raycast the PSP's
`Race_ComputeGridLayout` has: slot `N - 1` is the back. `FUN_00128158` places each ship on its matrix
(slot read from `mgr + 0x2be4 + 4 * i`, raycast 20 units down and `+2`, slot recorded at `ship + 0x9dc`).

## Functions

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x00128860` | `Race_CreatePlayer` | 78 | `(mgr, racer, local_index, slot, name)`, registers `"autopilot input%d"`; same arguments as the PSP's `Race_CreatePlayer`. |
| `0x00128be0` | `Race_CreateAiRacer` | 75 | `(mgr, racer, 0, slot, team)`, called for racers `0..N-2` / `0..N-3`; the PSP's `Race_CreateAiRacer` shape. |
| `0x00128158` | `Race_PlaceGrid` | 82 | Reads `0x00127ad0`'s matrices, drops each ship by a raycast (20 down, `+2`), records the slot; the PSP's `Race_PlaceGrid` (`0x08827c98`) step for step. |
| `0x00127ad0` | `Race_ComputeGridLayout` | 85 | `"start position"` (`0x002a1240`), `0x419e6666`, `10.0`, `0x42480000`/`0x40a00000`: the PSP's literals; matrix index 7 at the node. |

## Pulse PSP and Pure

The PSP has no local split in the original; the PSP binary's single-race spawn (`FUN_0882e57c`) is the
one-person branch above. Our Pulse PSP and Pure take this page's rule, labelled as borrowed from PS2.
