# Zone's four-corner hover epilogue, and which handling a Zone craft flies

Pulse PSP (USA) `BOOT.BIN`, 2026-10-02, lane `pulse-zone-rest`. Headless decompile and disassembly plus a live PPSSPP
1.20.4 run. No new function names: `Ship_HoverFourCorner` (`0x0884ae90`) and `Ship_UpdateHover` (`0x0884870c`) were
already named.

**The mode was patched, not chosen.** A fresh profile cannot pick Zone (greyed), so every live read below is a Time
Trial on `16_Track` (Talon's Junction, Feisar-class Venom, one craft) with `g_game_mode` (`0x08b31048`) written to `6`
while the CPU was stopped. `Ship_UpdateHover` re-tests it every frame, so the four-corner variant runs from the next
frame; a native Zone race was not watched.

## `Ship_UpdateHover` picks the variant every frame

```c
void Ship_UpdateHover(int craft) {
  craft->flags_1c0 &= ~1;
  if (g_debug_mode_override == 0 && g_game_mode == 6) Ship_HoverFourCorner(craft);
  else                                              Ship_HoverTwoPoint(craft);
  Ship_UpdateMagLock(craft);
}
```

Confidence **95**: the decompile, and the live run: a breakpoint inside `Ship_HoverFourCorner` (`0x0884b7b0`) hit on
every frame once `g_game_mode` was `6`, and that function is reached nowhere else.

## The epilogue's bank-to-yaw gain is `50.0`, not `30.0`

`Ship_HoverFourCorner`, `0x0884b760 - 0x0884b7ac`:

| Address | Instruction | |
| --- | --- | --- |
| `0x0884b760` | `lui a0,0x8a9` / `lv.q C400,0xa00(a0)` | the addend vector starts as the constant at `0x08a90a00`: read `(0, 0, 0, 1.0)` |
| `0x0884b76c` | `lw a0,0x2a4(s0)` / `beq a0,zero,0x0884b798` | grid state (`craft+0x2a4 == 0`): skip the term, add the zero vector |
| `0x0884b77c` | `lui a0,0x4248` | `0x42480000` = **50.0** |
| `0x0884b778 - 0x0884b794` | `f12 = craft[0x174] * 50.0 * (1.0 - craft[0x280])` | stored to the addend's `y` |
| `0x0884b7a8` | `vadd.t C230,C230,C600` | `craft+0x340 += addend` (the body-local angular accumulator) |

`craft[0x174]` is `right.y` and `craft[0x280]` is `magLockBlend`. The two-point law (`Ship_HoverTwoPoint`,
`0x0884ad2c`) has the same shape with `30.0`, with one difference in structure: there the whole `vadd` sits inside the
`!= 0` branch, here the `vadd` runs every frame and adds the zero vector on the grid. The result is the same.

**Live**, 30 frames on a banked start (`right.y = -0.01934 ... -0.01816`, `magLockBlend = 0`, state 1): the addend's
`y`, read from the stack at `sp+0x170` at `0x0884b7b0`, equals `50 * right.y * (1 - magLockBlend)` to `1e-7` on every
frame (`-0.96689` against `-0.96689`; `-0.90815` against `-0.90815`); 30.0 would have read `-0.58`. A second run, 12
frames after the craft had been disturbed (`right.y` up to `0.101`), agrees to `1e-6` (`5.0503` against `5.0503`).
The grid branch (state 0) was **not** watched: writing `craft+0x2a4 = 0` over a running craft did not stick (the state
word reads `1` on the next frame), so the skip rests on the instruction read and the zero vector alone.

Confidence **90** for the gain and the formula (instruction read, live delta on two runs); **78** for the grid-state
skip (instruction read only, the same guard watched live on the two-point law, `docs/physics/grid-state.md`).

Ported: `oag_physics::hover::BANK_TO_YAW_GAIN_FOUR_CORNER`, selected by `ShipState::four_corner`, which the race writes
from the mode each tick. The effect is `50/30` on the same torque: a banked Zone craft yaws 1.67 times as hard.

## Not ported: the other differences in the four-corner epilogue

Read in the same disassembly (`0x0884b7b0 - 0x0884b81c`), unported because this engine has no four-probe layout to
hang them on and the difference is small where all four probes touch:

- **Downforce**: `-(track_gravity * mass)` along the contact-normal vector, with **no groundedness factor and no
  `(1 - magLockBlend)`**. The two-point law multiplies both. `track_gravity` is read as
  `*(craft+0x6c) + class * 0x80 + 0x100` here (`class` = `DAT_08b31040`) where the two-point law reads
  `*(craft+0x70) + 0x6c`. They are the same float: `Ship_InitCraft` (`0x08849354`) sets
  `craft+0x70 = *(craft+0x6c) + class * 0x80 + 0x94`. Live: `80.0` through both paths, `mass = 1.0`, class `0`.
  Groundedness is `0.25` a probe in the four-corner function (`craft+0x2b0 += 0.25` per contacting probe in the
  function's probe loop), so four contacts make `1.0` and the omission only matters with fewer than four.
- The probe layout, the alignment torque and the rest of the 2000-line function were not compared.

## Which handling does a Zone craft fly? The player's own team and class

`docs/gameplay/race-modes.md` and `oag_title::race::ZoneCraft` recorded that every title ships a `Data\Ships\Zone_01`
handling file (`<Stats team="ZoneMode">`, no `<Class>` block) that this engine does not read, and asked which block
Zone is meant to use. **On Pulse that file is not used by anything.**

| Read | Result |
| --- | --- |
| `Zone_01` and `ZoneMode` as byte strings in `BOOT.BIN` (headless `findBytes`) | **no hit** for either. The file is reachable only by a composed name, and no composition produces it, see the next two rows |
| `Data\Plugins\PI001\Definition.xml` (Pulse) | eight `PI_Team`s, all `type="Race"` (the `Values` default); a **`PI_TeamModel name="Zone"`** with `location="zone01"` per team, no `PI_Team` for `Zone_01`, no team of `type="Zone"` |
| `FUN_088c2a38` (the `PI_Team`/`PI_Track` `Values` parser) | `type` is matched against a two-entry name table at `0x08ab1ad0` (the second entry's string is `"Zone"`, `0x08a81018`) into `team+0xa0`, the attribute Pure uses to mark `Zone_01`. No Pulse team carries `type="Zone"` |
| `Ship_LoadHandlingStats` (`0x088c291c`) | the stats path is `<team+0x94>\handlingstats.xml`, the team's own `Location`; one parse per team, once |
| `Ship_LoadModel` (`0x08843258`) case 6 | builds `%s\Zone.vex` from the **same** `team+0x94`: the player's team directory again |
| `Ship_InitCraft` (`0x08849354`, line `0x70`) | `craft+0x70 = *(craft+0x6c) + DAT_08b31040 * 0x80 + 0x94`: the player's class block, in Zone as in every mode (no `g_game_mode` test on this line) |
| Live, mode patched to `6` | `craft+0x6c = 0x08f4c784`, `craft+0x70 = 0x08f4c818` = `0x08f4c784 + 0*0x80 + 0x94`; class `0`; `track_gravity` `80.0` |

So a Pulse Zone craft flies **its team's own `<Class>` block for the race's speed class**, which is what this engine
already does. `Zone_01` is data from the Pure branch of the engine that Pulse's disc still carries. Zone replaces three
things *inside* that handling and takes nothing from a Zone file: the engine (the auto-speed law), the brakes
(disabled) and the hover variant (the four-corner function above). Confidence **82**: the executable is read, the
definition is read, and the live craft matches; not higher because a native Zone race was not watched and the mode
factory's team choice was not traced end to end.

Two limits worth keeping: this is **Pulse only**. Pure's definition marks `Zone_01` as `type="Zone"` and a Pure Zone
race presumably reads it; HD's `/data/ships/zone` is the same shape. Neither executable was read.
