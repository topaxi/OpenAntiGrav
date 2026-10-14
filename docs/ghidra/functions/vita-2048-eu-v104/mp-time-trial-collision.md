# 2048: the `MPTimeTrial` race manager is compiled in and clears the same collision byte (2026-10-14, `mp-collision-re`)

**Binary:** `eboot.elf` (2048, Vita, EU v1.04), ARM Thumb-2, image base `0x81000000`. Static
only. This **corrects** the earlier cross-check in
[../psp-pulse-usa/mp-time-trial.md](../psp-pulse-usa/mp-time-trial.md), which said 2048 has no
multiplayer Time Trial because its executable "names six `GameMode_*` classes" and its data
lists no `multiplayer_definition.xml`. Both facts hold, but the executable also still carries the
HD-lineage backend class.

## Answer

| Claim | Evidence | Conf |
| --- | --- | ---: |
| The class is in the executable | `Backend/General/MPTimeTrial_RaceManager.cpp` at `0x8147e6f8` (and `MPTimeTrial_ModeManager.cpp`), beside `MPArcade`, `MPElimination`, `MPNitro` and `MPTournament` race managers; the constructor `FUN_8116385c` stores that string to `+0x2c` and loads `Data\XML\Arcade_HUD.xml` and `Data\XML\MPTag_HUD.xml` with HUD flags `1, 2, 4, 8, 0x10, 0x40, 0x100`, the same set as Pulse's mode 17 | 80 |
| It can be built by mode id `0x13` (19) | `FUN_8115f3de`, a by-mode factory, `switch(DAT_8153fd24)`: case `0x13` calls `FUN_8116385c` (its one caller); cases `3`, `9`, `0xc` build `SpArcadeRaceManager_Construct`, case `2` `DemoRaceManager_Construct`. 19 is the id HD's `MPTimeTrial` has ([../ps3-hdfury-eu/mode-manager.md](../ps3-hdfury-eu/mode-manager.md)) | 78 |
| Its constructor clears a collision byte | `strb` of `0` to `*DAT_8151faec + 0x7448` (`movw r2,#0x7448` at `0x81163892`); `DAT_8151faec` is written by `FUN_81187abe`, which stores `1` to `+0x7448` (`movs r0,#1` at `0x81187ad4`, `strb r0,[r4,r2]` with `r2 = 0x7448` at `0x81187ada`) | 76 |
| The pair pass reads it | `FUN_8118b1c0` loads the byte through `movw r3,#0x7448 ; ldrb r3,[r2,r3]` and skips the pair when it is zero (`0x8118b204`, `0x8118b218`; seven `movw #0x7448` in the function, two read), the same per-shape-type switch as HD's `Collision_ProcessPairs` and Omega's | 74 |

So the byte, its writers and the gate are the same shape as HD and Omega, and 2048's Time Trial
class would play with craft passing through each other. **Whether any of it is reachable in 2048
is not shown:** nothing read offers mode 19 (no `multiplayer_definition.xml` in the archives,
`MP.xml` is the season ladder), and the factory is reached by whatever sets `DAT_8153fd24`, which
was not traced.

## Names

| Address | Name | Conf |
| --- | --- | ---: |
| `0x8116385c` | `MPTimeTrialRaceManager_Construct` | 80 |
| `0x8115f3de` | `RaceManager_ConstructByMode` | 76 |

## Cross-check

- **Omega: checked, differs** ([../ps4-omega-eu/mp-time-trial-collision.md](../ps4-omega-eu/mp-time-trial-collision.md)): the same byte, and a gate wider than craft against craft there; 2048's own gate count (seven `movw`) agrees with Omega's six-site shape, so 2048 probably gates the sphere pairs too (not read).
- **Port:** none. 2048's local split Time Trial takes Pulse's rule, **chosen**.
