# HD's Eliminator HUD: the race line, and the rows left unread (2026-11-06, `eliminator-hud`)

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), every function below on TOC
`r2 = 0x008ad4d8` (`scripts/ps3-toc.py toc` reports each `exact`). Original frames: RPCS3,
Wipeout HD Fury EU, Racebox Single Player, RACE TYPE Eliminator (`GetMode()==8` in `TTY.log`),
Vineta K, the default craft, weapons on. The behaviour and the picture
are on [`docs/ui/hud.md`](../../../ui/hud.md#hds-eliminator-race-line-2026-11-06-eliminator-hud).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x00089fa8` | `Hud_UpdateRaceLine` | 85 | Called from `Hud_Update` (`0x0009e3d0`) at `0x0009faec`, behind `hud+0x44 & 0x4000` (`rlwinm r0,r11,0,17,17` at `0x0009e944`/`0x0009fae0`). Moves `ShipLinePos0`..`7` and writes `ShipsInFront` / `ShipsBehind`; both reproduced frame for frame below. |

## What it does

`Hud_BindWidgets` (`0x0009bb30`, the `hud+0x44 & 0x4000` branch) binds the bar (`hud+0x490`,
`EliminatorPositions`'s image), `ShipLinePos0`..`ShipLinePos7` into `hud+0x3b0 + 4 * i` and
`ShipsInFront` / `ShipsBehind` into `hud+0x494` / `hud+0x498`. `Hud_UpdateRaceLine`, per race
manager craft `c`, in craft order:

```text
f = craft+0x7010 - RM+0x1948        // distance into the lap, less the grid's start offset
if f < 0: f += *(craft+0x6f64)      // wrap by the lap length
f /= *(craft+0x6f64)
marker = (c is the local player) ? hud+0x3cc (slot 7) : slot n++ (0, 1, ...)
marker.SetX(f * bar.width + bar.x)          // vtable +0x68, with +0x5c / +0x6c reading the bar
marker rotation (+0x12c) = (craft+0x6ad8 == 0) ? 0 : DAT_008a76bc   // 0x40490fdb = pi
marker flags |= 6                            // visible
```

Then `front = number of slots 0..6 whose x >= the player's x`, `ShipsInFront = front`,
`ShipsBehind = fieldSize - front - 1`, both written as plain decimal strings.

`craft+0x7010` is the distance into the current lap (`ai-controls.md`); `RM+0x1948` is the
start offset `AI_UpdateSpreadWander` subtracts. A craft still behind the line on the grid
therefore reads just under 1, so the whole pack starts at the bar's right end: both RPCS3 frames
from the grid show one chevron at the right and `0` / `7` at the ends. Ours takes
`Standing::distance` (the same lap-1 wrap) over the course length.

Confidence 85: the offsets and the counting are read off the instructions and the frames agree
(counts `0 / 7` on the grid, `1 / 6` and `0 / 7` mid-lap, markers where the counts put them).
**Unread:** what `craft+0x6ad8` is (the same byte gates `WeaponAi_DecideFireOrAbsorb`'s absorb
in [weapon-ai-fire.md](weapon-ai-fire.md)); ours draws every marker upright.

## Which design the original draws (settles hd-hud.md's open question for this file)

The executable's string table names `ShipLinePos%d` (`0x0077fd18`), `ShipsInFront`
(`0x0077fd28`), `ShipsBehind` (`0x0077fd38`), `ScoreFeedBack%d` (`0x0077fd48`),
`MPWeaponIcon%d` (`0x0077fbd0`), `PlaceString0/1`, `PosTag%d` and `HighlightBar`; **`KillsText`
is absent** (`search_strings` and the raw bytes `4B 69 6C 6C 73 54 65 78 74`, no match). Those
widgets are authored by `DATA00`'s `elimination_hud.xml` (the `HUD_Elim_*` fragments), and the
RPCS3 frames draw exactly them: the bottom race line, the `TARGET ( 200 )` header, the ranked
`NAME ( score )` rows with the player's row highlighted. `DATA02`'s copy (generic fragments plus
`KillsText`) has none of them. `DATA02`'s own Racebox lists no Eliminator at all ([head2head.md](head2head.md)), which fits the
Fury-layer reading. Confidence 90 that **Eliminator draws `DATA00`'s copy**, which is
what `Archives::holder_of` already returns, so no archive-order change and no per-file selection
is needed. Whether the original picks `DATA00` by its own precedence stays unread for the other
five duplicated roots.

## Unread: the rows, and the score they print

The frames show a score where Pulse shows kills: `TARGET ( 200 )` over `PLAYER ( 0 )`,
`EG-X ( 0 )` ... ranked 1 to 8, the screen's intro text reading "Score points for damaging and
eliminating opponents. Extra points for completing laps. After elimination you will respawn but
lose points". The pieces found, none read:

- `Hud_BindWidgets` formats the header as `"%s ( %d )"` (`0x0077fb50`) from `IG_HUD_TARGET` and
  `g_GameState+0x20` (`0x0009d494`, the `hud+0x44 & 0x200` branch).
- The rows' format is `"%s  ( %d )"` (`0x0077ed98`, two spaces) with writers `0x0008c6f8`
  (not decompiled) and, for the two-person list, `0x00095950` (read: it orders crafts by
  `craft+0x6f80`, a float score truncated to int, then `craft+0x6f7c`, then `craft+0x628c`).
- So the score is `craft+0x6f80`. What writes it (points per damage, per elimination, per lap,
  the loss at an elimination) is the scoring law and is **not recovered**; this build's Eliminator
  counts kills to a target of 5 to 25 (`Standing::kills`), a different rule.

`ScoreFeedBack0`..`15` (bound at `hud+0x3d0`, stride 12, for modes 8, 13, 14, 20, 21) are the
popups that scoring raises; `MPWeaponIcon0`..`7` and `VoiceCom0`..`7` sit in the same rows.
None is drawn in the frames taken (no score changed).
