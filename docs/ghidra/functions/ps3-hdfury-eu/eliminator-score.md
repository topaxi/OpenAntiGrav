# HD's Eliminator score law (2026-11-07, `hd-score-law`)

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), every function below on TOC
`r2 = 0x008ad4d8` (`scripts/ps3-toc.py toc` reports `exact`). **Live:** RPCS3, Wipeout HD Fury EU,
Racebox Single Player, Eliminator (`GetMode()==8`), Vineta K, Venom, Novice, target 200, weapons on.
Captured with `scripts/rpcs3-hd-eliminator-score.py` (one row per change of a craft's score, kills,
deaths, shield, lap or state, read off the race manager's craft list `RM+0xe8`). The player flew
under Autopilot only to get round; no physics law is read from these runs.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x00069130` | `SPElimination_UpdateTarget` | 90 | Ends the race for every craft when any craft's truncated score reaches the target, below; run 4 ended there live. |
| `0x0008c6f8` | `Hud_UpdateEliminatorRows` | 88 | Writes `PosTag0`-`7` as `"%s  ( %d )"`, ranked, below; the frames agree row for row. |
| `0x0008c568` | `Hud_PlaceHighlightBar` | 80 | Moves `HighlightBar` behind the row it is given; the frames show it on the player's row at every rank. |
| `0x00083ba0` | `Hud_RaiseScoreFeedback` | 75 | Formats an amount into a free `ScoreFeedBack` widget and raises its alpha. Read, never seen in a frame. |
| `0x000d9d60` | `Ship_RefillLapShield` | 88 | `Ship_AddShield`-shaped: `0.2 * max` then `Ship_PlayAbsorbFeedback`; live, the player's shield rose 28.0 of 140 on each lap. |

The two score blocks in the state and lap code (`0x000e2ddc`, `0x000e5b1c`) are not inside a Ghidra
function, so nothing there is renamed.

## The law

`score` is the float `craft+0x6f80`, `0` from `Ship_Construct` (`0x000de6ac`). `kills` is `+0x6f7c`,
`deaths` `+0x6f78`. Everything below is offline mode 8 (`g_GameState+0xe0 == 8`).

`scale(craft)` multiplies every award: `1.0` for a craft whose role `+0x628c` is `0`, the person,
and for role `2`, an opponent, the float `RM+0x193c + difficulty * 4`. That is
`WeaponAIStats.xml`'s `<EliminatorAIStats>` `easyScoreScale`, `mediumScoreScale`, `hardScoreScale`
(the loader is [weapon-ai-stats.md](weapon-ai-stats.md); the parse fills `RM+6240+176..+228`,
which puts the three score floats at `RM+0x193c`). Read live at Novice: `RM+0x193c..+0x1944` =
`1.15, 1.25, 1.4`, the file's own three.

| Event | Law | Instruction and constants | Confidence |
| --- | --- | --- | ---: |
| Weapon damage to another craft | attacker `+= 0.1 * damage * scale` | `0x000e7bb4`, `0.1` at TOC `-0x4a1c` (`0x008a8abc` = `0x3dcccccd`); `damage` is the blow as dealt, not capped by the victim's shield | 90 |
| A kill by a weapon | attacker `+= 20 * scale`, `kills += 1`, on top of the fatal blow's damage points | `0x000e83d8`, `20.0` at TOC `-0x47a4` (`0x008a8d34` = `0x41a00000`) | 90 |
| A lap | craft `+= 5 * scale` | `0x000e5b68`, `5.0` at TOC `-0x4890` (`0x008a8c48` = `0x40a00000`); gate at `0x000e52a4` (`mode == 8`, network byte clear) | 90 |
| Any destruction (state 4) | victim `-= min(score, 5)`, `deaths += 1`, not scaled | `0x000e2df4`, reached from `0x000e2180` on `mode == 8`; `fsel` floors at `0` (`deaths` `0x000e2104`) | 90 for the `-5`; **84 for the floor** |
| Ending | every craft to state 2 the tick any `trunc(score) >= target` | `0x00069130`; target `g_GameState+0x20`, or `cell+0xd4 + difficulty * 4` when `g_GameState+0x90` is set | 90 Racebox; **84 for the cell branch** |

The damage award runs only when the source is a weapon (`param_4 == 2`) and the recorded attacker
`*(craft+0x5edc)+0x13c` is not the craft itself. It sits past `Ship_ApplyDamage`'s early returns, so
a craft under a Shield (`+0x6a64 & 0x1000`) or still protected (`+0x6a84 > 0`) gives no points
(static, 84). The kill award runs from the zero-shield branch, weapon source only, `kills` always,
the points when the attacker is not the victim.

**Not scoring in mode 8:** `0x000cf660` (a floored subtract) and `0x000cf6e0` (the scaled add) are
the out-of-line copies of the two blocks above. No `bl`, `b` or data word reaches them (raw scan of
the text segment), so they are dead here; hypothesis only. `0x00043168`, `0x00047888` and
`0x000d1e98` move a score through `g_GameState+0x4e4[...]+0x2e0`, the online record, read `0` for
all eight slots in these runs. `0x000e88e4` is the online `-5`, mode `0x14`.

## Live evidence

Racing rows from `data/reference/hd-eliminator-score/` (local, gitignored), every one a single
tick's change:

| Row | Reads | Law |
| --- | --- | --- |
| AI `c3`, `32.2 -> 41.4`, on `c1`'s shield `135.9 -> 55.9` | `+9.2` | `80 * 0.1 * 1.15` |
| AI `c6`, `0 -> 0.805`, on `c3`'s shield `53.7 -> 46.7` (LeachBeam tick) | `+0.805` | `7 * 0.1 * 1.15` |
| AI `c6`, `1.2075 -> 24.61`, `kills 0 -> 1`, `c3` down | `+23.4025` | `23 + 0.4025` (`20 * 1.15` and a `3.5` blow) |
| AI `c3`, `41.4 -> 36.4`, `deaths 0 -> 1`, state `1 -> 4` | `-5.0` | not scaled |
| Every AI at each lap, `lap n -> n+1` (21 rows in run 2) | `+5.75` | `5 * 1.15` |
| Person, three laps, `0 -> 5 -> 10 -> 15` | `+5.0` | scale `1.0` |
| Person, Quake kills, `15 -> 41`, `41 -> 67`, `148 -> 177` | `+26`, `+26`, `+29` | `20 + 6`, `20 + 6`, `20 + 9` |
| Person, Quake hits, `67 -> 70` | `+3.0` | `30 * 0.1` |
| Person's shield at each lap, `36.2 -> 64.2` | `+28.0` | `0.2 * 140`, `Ship_RefillLapShield` |
| Run 4 end, AI `c6`, `196.54 -> 202.29` on a lap | `+5.75` | every craft to state 2 in the same row |

**Not seen live:** a death below 5 points, the person's own death (the person died 0 times), Skilled
or Elite, a campaign cell, an absorbed hit. Reproduce: `scripts/rpcs3-hd-eliminator-score.py`
against a `serve` on the grid, `--script "1:down:cross;2:give:6;3:tap:triangle"`.

## The rows and the results

`Hud_UpdateEliminatorRows` (`hud+0x350 + 4 * rank`, bound from `"PosTag%d"`, `0x0009d870`) gives each
craft `rank = count - 1 - (crafts with a strictly lower truncated score)` and walks up from a taken
rank, crafts in array order: the rule `oag_hud::kill_tags::ranked` already carries. Text is
`"%s  ( %d )"` (`0x0077ed98`, two spaces), the truncated score. Scale is `1.2` for the person
(`0x008a7620`), `1.0` for an opponent (`0x008a75bc`), both written to `+0xb8`/`+0xbc`. The name is
the team's for an opponent and the profile's for the person (`PLAYER` on these frames). The person's
row then calls `Hud_PlaceHighlightBar`. The rank digits are the layout's own `PlaceString0`-`7`, the
bracket its image.

The header is `Hud_BindWidgets`' `"%s ( %d )"` of `IG_HUD_TARGET` and the target (`eliminator-hud.md`).

The results screen ranks by the same score: `HARIMAU 202`, `FEISAR 177`, `GOTEKI 45 176` ... under
`POINTS`, headed `2ND PLACE - KILLS: 6 DEATHS: 0` (`results.png`). 195 Elimination loyalty was
paid on that race.

**`MPWeaponIcon0`-`7` are not drawn in a single-player Eliminator.** `Hud_BindWidgets` binds them and
clears their visible bits (`0x0009d988..`); no frame of four runs shows one. That half of the brief's
claim is false for single player.

**`ScoreFeedBack0`-`15` are authored (`hud_scoring_feedback.xml`, loaded by `elimination_hud.xml`)
and were never caught.** `Hud_RaiseScoreFeedback` is called for the person's damage and kill awards
(`0x000e7c2c`, `0x000e8458`). Frames 0.05 to 1.3 s after six of the person's awards show no popup, so
either it is shorter than that or it draws somewhere the sampling missed. Not drawn by ours, 75 for
the name. Next: a frame burst across one of the person's hits, reading `hud+0x3d0 + 12 * i`.

## What ours does

`oag_title::Title::eliminator` states the rule; `oag_raceplay::score_law` resolves it once at load
into a plain `oag_race::ScoreLaw` on the race's setup, and nothing during a race reads a title. The
HUD follows that value too (`Readout::score_rows`), so the rule is free to move off the title later.
A Racebox target is `<Slider name="Eliminations">`'s `minSlide` off `DATA00`; a campaign cell's is
its `NitroElim*` number for the rung, on `Options::eliminator_score_target`. A score race ignores the
`KILLS` pick, which another title may have stored. The law is `None` outside an Eliminator, as the
original's awards all sit behind `mode == 8`. `Standing::score` enters the state hash only once it is not
zero, so a kill race hashes as before.

Three departures, all stated: a kill's points and the `-5` land when ours first sees the craft down,
about half a second after the original's tick (`Race::credit_kill`, its existing path); a blast's
points go to the first impact whose radius reached the craft (`credit_blast`'s owner, chosen);
`HighlightBar` keeps its authored width, because the HUD's draw list has no font to measure the
row with, where `Hud_PlaceHighlightBar` fits its left edge to the text.

**Open:** the results screen (HD's Eliminator headline is still `Headline::Unresolved` and its
table is `oag_raceplay::scoreboard`'s places, not `POINTS`); the Racebox's TARGET row (`race_setup`
is `None` on HD, and the disc's row is a `Slider`, not a list); the lap refill for an opponent:
ours refills every craft, HD's reads as the person's alone (an AI's shield did not move on any lap);
`ScoreFeedBack` above; the person's name in a row is the team's, not a profile's, and `--race` with
no `--team` keeps the lowercase default id, upper-cased, because the HUD has no roster to look it up
in.

## Other titles

- **2048:** **checked, applies, not wired.** The executable names `IG_HUD_TARGET`, `ScoreFeedBack%d`,
  `MPWeaponIcon%d`, `%s ( %d )` and the three score scales, and its `WeaponAIStats2048.xml` carries
  the row. Whether its awards are HD's is unread: the writer of its score was not looked for, and
  its Elimination campaign objective reads kills (`crates/2048/src/campaign.rs`).
- **Omega:** **checked, applies, not wired.** The same strings, including the row's two-space
  `%s  ( %d )` (`FUN_0125e9b0`). Its named `weaponaistats.xml` has no `<EliminatorAIStats>` row and
  its loader defaults every scale to `1.0`, so an opponent there would score flat if the law holds.
- **Fury against the base game:** the `<EliminatorAIStats>` row is in `DATA00`'s copy alone
  ([race-modes.md](../../../gameplay/race-modes.md)). `DATA02`'s Racebox has no Eliminator.
