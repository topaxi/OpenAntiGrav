# Which sounds Pulse's front end plays, and when

Pulse PSP (USA) `BOOT.BIN` (`/pulse/BOOT-psp-pulse-usa.BIN`, image base `0x08804000`), 2026-10-08, lane
`pulse-menu-sfx`. Headless-equivalent reading through the Ghidra bridge (decompile and call-site scan, no live run).
One name recovered: `SoundManager_LoadBootBanks`.

## The path every menu sound takes

`Sound_PlayNamedInSlot` (`0x0893a768`, already named) reads a **bank slot** out of the sound manager
(`*(mgr + slot*4 + 0x124)`), stores two extra bytes into a 32-entry table (`FUN_0898dac0`, below) and calls
`Scream_PlaySoundByName(bank, 0, name, 0x400, ...)`. `0x400` is unity pitch and there is no emitter: every menu
sound is dry. A scan of its 79 call sites (decompiling each caller; list in this lane's scratch,
`play_sites.txt`) gives every cue the front end names and the slot it plays it from.

`SoundManager_LoadBootBanks` (`0x0893aba4`, called from the sound manager's constructor `FUN_08939fe0`) fills the
slots, in this order:

| Global | Bank | Hash entry (Pulse USA `Data.wad`) | Label |
| --- | --- | --- | --- |
| `DAT_08ac1de4` | `Data\Sound\frontend.bnk` | `#856 75a91641` | `FRNTEND` |
| `DAT_08ac1de8` | `Data\Sound\FRONT_END_VO.bnk` | `#857 a3aada5f` | none |
| `DAT_08ac1e00` | `Data\Sound\speech_results.bnk` | `#864 7c778043` | none |
| `DAT_08ac1dec` | `Data\Sound\hud.bnk` | `#859` | `HUD` |
| `DAT_08ac1df0` | `Data\Sound\generaltrack.bnk` | `#858` | `gentrak` |

Confidence **80**: the decompile is five `FUN_0893a654(mgr, path)` calls storing their result in five globals, and
every call site below passes one of them. The other slot globals (`1df4`, `1df8`, `1dfc`, `1e04`) are loaded elsewhere
and are race banks (`weapons`, `ship`, `speech`, ...).

## `frontend.bnk`: six cues, five of them played

`oag-wad sounds 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' --bank FRNTEND`:

| Cue | Waveforms (Pulse) | Waveforms (Pure) | Played by |
| --- | ---: | ---: | --- |
| `UPDOWN` | 3 | 1 | the cursor moving to another cell or row |
| `LEFTRIGHT` | 1 | 1 | a value stepping, and the grid controller's left/right |
| `ACCEPT` | 1 | 1 | a confirm that went through |
| `DECLINE` | 3 | 1 | back, a refused confirm, a move onto an empty cell |
| `TELETYPE` | 1 | 1 | a `Menu` row revealing with its cursor on; **no screen reaches it** (see `TELETYPE`) |
| `PAUSE` | 1 | 1 | **nothing recovered** |

Confidence **88** for the four navigation cues (several independent call sites each, strings in the pointer pool at
`0x08a7f068`, `0x08a7f074`, `0x08a81898`, `0x08a818a4`, ...). The mapping of event to cue, from the callers:

| Event | Cue | Where |
| --- | --- | --- |
| Cursor moves to a live cell | `UPDOWN` | `FUN_088a3b1c` (grid move, the `else` of the empty-cell test), `FUN_088a4e60`, `FUN_088a9db4`/`FUN_088a9e7c` (previous/next), `FUN_088b528c`, `FUN_088cae88`, `GridSelection_Update`, `TrackSelection_Update` |
| **Cursor move onto an empty cell** | `DECLINE`, *instead of* `UPDOWN` | `FUN_088a3b1c`, `FUN_088a3e8c` |
| Value stepped left or right | `LEFTRIGHT` | `FUN_088aaae4` (the grid controller's two buttons), `FUN_088cae88`, `FUN_088e4fd0` |
| Confirm resolved (the redirect evaluated non-zero) | `ACCEPT` | `ConfirmButton_Update` (`0x088c8e88`), `FUN_088ed05c` |
| Confirm refused (the button's own check `FUN_088b4fc8() == 0`, or the confirm's target not found) | `DECLINE` | `ConfirmButton_Update` |
| Secondary button (back) | `DECLINE` | `ConfirmButton_Update`, `GridSelection_Update`, `FUN_088cae88`, `FUN_088e4fd0`, `FUN_088ed05c` |
| Page transition by itself | **no sound** | see below |

`ConfirmButton_Update` skips `ACCEPT` and the refusal's `DECLINE` when the widget's byte `+0x269` is set (the XML
`Silent` property sits in the same string pool, `0x08a81860`); the back button's `DECLINE` does not read it. Which
widgets set it was not swept (open).

**A page transition makes no sound of its own.** `FrontEnd_TransitionTo` (`0x088e24fc`) calls `FUN_08965630`, which
the earlier `state-machine.md` called a front-end sound. It is three stores: `FUN_08965628(p, 0)` writes `*p = 0`
and `*(p + 4) = 0` on the singleton `FUN_0896565c` returns (a 0x140-byte object allocated on first use). Neither
reaches `Sound_*` or `Scream_*`. The cue rides the button that caused the transition. Confidence 85.

## `TELETYPE`

Corrected 2026-10-11 (lane `pulse-teletype`). The earlier reading here, "once per revealed character", was wrong, and
the cue is **never reached by any screen the shipped front end shows**. Nothing is wired, and that is the finding.

**The caller is `Menu_PlayTeletypeTick` (`FUN_088b6514`, `0x088b6514`), the second virtual slot of the `Menu` and
`MenuBitmap` widget classes** (vtables `0x08acd75c` and `0x08acd844`, slot at `+0x08` of the pair table; the class names
are the registered strings `Menu` at `0x08a8029c` and `MenuBitmap` at `0x08a8043c`). It indexes the menu's *entries*
(`<Entry IDString/String/Value>` children, `FUN_088b5a60`), not characters: while `+0xd0` (reveal complete) is 0, `+0xcc`
(revealed rows) is below the row count and `+0xf0` (cursor enabled) is set, it calls
`Sound_PlayNamedInSlot(slot, bank, "TELETYPE", 0x400, pitch, 0, 0)` at `0x088b669c` (pitch in `t0`) and parks the `_`
cursor widget (`+0xe0`, the string at `0x08a80138`) at the end of row `+0xcc`. The pitch is the formula below from the
row's measured text width `FUN_08919960(FUN_088cd21c(row text))`: `(width / 100 - 1) * 0.2 * 90`, `+360` when negative,
truncated to an integer. The reveal state machine is `FUN_088b6088`: `+0xcc` advances once per update through
`FUN_088883d4`, which is `jal FUN_088885ec; li v0, 1` and so **always reports the row done**, so the reveal-wait branch
(`FUN_08888418`, the `+0x100` width test) is dead code on this build. A press of input 0 completes the reveal
(`FUN_088b521c`); input 1 steps the selection and completes it (`FUN_088b528c`, which is also where `UPDOWN` plays).

**What `MenuDisplayType` does** (`FUN_088b5a60`, the `Values` element's attribute): `NORMAL` -> mode 3 and **clears
`+0xf0`**; `REFRESH_NORMAL` -> mode 2 (completes on the first update); `SCROLLING` -> mode 0; `TELETYPE` -> mode 1. The
constructor `FUN_088b49a4` defaults to mode 3 with `+0xf0 = 1`.

**Runtime check (PPSSPP v1.20.4, software, cold boot, USA).** A log-only breakpoint on the `jal` at `0x088b669c` and a
second on `0x088b6514` itself, armed through the whole first boot and then the walk Boot, `Main Menu`, `Single Player`,
`Racebox` (Time Trial, track select) into the race: **0 hits at the `jal`** against 212 hits at the entry. At the
entry every Menu object read mode `+0xdc = 3` and `+0xf0 = 0`; the main menu's object revealed its seven rows one per
frame (`+0xcc` 1..7 over seven consecutive updates) with `+0xf0 = 0`, so it is silent and the rows appear within seven
frames (about 0.12 s at 60 Hz), not as a visible type-in. Confidence that no screen on that walk plays `TELETYPE`: 85.

**Authored XML.** Across every entry of `Data.wad`, `FE.wad` and `FEData.wad` (USA), `MenuDisplayType` is authored 21
times: 19 `REFRESH_NORMAL` and 2 `NORMAL`; there is **no `TELETYPE` and no `SCROLLING`**. An unauthored Menu keeps the
constructor's `+0xf0 = 1` and mode 3, which is the one path that could tick; the walk above met none, and the
other screens were not walked (open: a Menu with no `MenuDisplayType` on an unvisited page).

**Other titles (checked 2026-10-11).** Pulse PS2: same `frontend.bnk` cue set (psp-audio.md), but its executable was
not read: not checkable here. HD/Fury: checked, differs - its bank has `TextBox01`..`TextBox04` in place of any
`TELETYPE`, trigger unread (`ps3-hdfury-eu/menu-sounds.md`). Omega: checked, applies, not wired - HD's front end, no SFX
path. Pure: `TELETYPE` is in the bank, its caller not read.

**The other `TELETYPE` is a different widget.** `DisplayType="TELETYPE"` on a `<Text>` widget (`<a ... DisplayType=
"TELETYPE">`, the attribute is tokenised to one letter in some `.cod` files, `o="TELETYPE"` in `01092`) is authored on
button prompts (`FE_CONFIRM_BUTTON`, `FE_CONFIRM`, `FE_BACK`), screen titles (`PRO_STATS`, `FE_ONLINE`) and
`BOOT_PRESS_ANY2`, in 13 of the `.cod` files. That type-in is the `Text` class's own and **no `Sound_*` call references
the `TELETYPE` string except `Menu_PlayTeletypeTick`** (xrefs to `0x08a80100..0x08a8011c` are `FUN_088b528c`,
`FUN_088b5a60`, `FUN_088b6514`, `FUN_088b49a4`, `FUN_088b4ae8` only), so the original types those in silently. The port
does not read `DisplayType` at all, so they appear whole; the type-in rate is unmeasured and open.

## `PAUSE`

No code reference. A byte search of `BOOT.BIN` for `PAUSE\0` has no hit, so the executable never spells it; the
pause menu (`InGame Pause`, `0x08a795ec`) is driven by the same `ConfirmButton_Update` cues as any other. Whether a
formatted name could reach it was not ruled out (confidence 60 that nothing plays it).

## The voice banks (not wired)

| Slot | Cue | Caller | Selector |
| --- | --- | --- | --- |
| `FRONT_END_VO` | `TMTX_NAME` | `FUN_088ea048` | `FUN_0898dac0(2, param_2)` before the play (the team) |
| `FRONT_END_VO` | `TRTX_NAME` | `FUN_088ed9a8` | `FUN_0898dac0(1, param_2)` (the circuit) |
| `speech_results` | `FIRST_PLACE` / `SECOND_PLACE` / `THIRD_PLACE` | `EndRaceResults_Update` | wired 2026-10-11, see "The end-of-race voices" |
| `speech_results` | `gold_med` / `silver_med` / `bronze_med` | `EndRaceRewards_OnEnter` | wired 2026-10-11, see "The end-of-race voices" |

`FUN_0898dac0(slot, value)` stores `value` in a 32-byte table at `0x08ac3247` for `slot` in `1..32`. The banks'
`TMTX_NAME` is a 24-waveform cue (21 s in all) and `TRTX_NAME` is 66 commands and no waveform of its own (child references): the table is the selector a
guard operand reads (`Scream_OpGuard`, per-voice and global variables, `sound.md`), but which global the guard reads
was not followed. Picking a waveform by team index would be an invention, so these stay unplayed. The place and medal
lines are the next section.

## The end-of-race voices

Decompiled headless 2026-10-11 (`/pulse/BOOT-psp-pulse-usa.BIN`, `EndRaceResults_Update` `0x088da530`,
`EndRaceRewards_OnEnter` `0x088dbbd4`). Both play with `Sound_PlayNamedInSlot(DAT_08ac1e10, DAT_08ac1e00, name,
0x400, 0, 0, 0)`: the `speech_results` slot, dry. `EndRaceResults_Update` keeps a once-flag at widget `+0xe8`.

**The place line** (`EndRaceResults_Update`; `+0xe4` points at `g_endrace_result`, word 0 the finishing place, word 1
the Tournament standings rank, `tournament.md`):

| `g_game_mode` | When | Place read |
| --- | --- | --- |
| 3 (`Arcade`, Single Race) and 8 (`Elimination`) | every update tick until the flag is set, so the first | word 0 |
| 4 (`Tournament`) | the first three-second page flip. `EndRaceResults_OnEnter` leaves the page flag `+0xe0` at 1, so the first flip takes the else branch, `ER_TOUR_STAN` | **word 1**, the standings rank, not the leg's place |
| 0x10 (multiplayer Tournament) | the same flip | the same word; not reachable here, this build has no split-screen |
| everything else (Time Trial, Speed Lap, Zone, `Head2Head` = 9) | never | - |

1 plays `FIRST_PLACE`, 2 `SECOND_PLACE`, 3 `THIRD_PLACE`; 0 and 4 or worse play nothing and **leave the flag clear**
(it is set only on a play). The earlier reading of this page and `endrace-screens.md` ("once per results screen, off
the finishing position") held for modes 3 and 8 only. Confidence 85 (a direct read of the branch; the once-flag's
reset on re-entry is not read: `OnEnter` never writes `+0xe8`. `VIEW RESULTS AGAIN` speaking the place again is
**chosen, not measured**).

**The medal line** (`EndRaceRewards_OnEnter`): only under `DAT_08b30ffc != 0` (a campaign cell in play), on entry,
off the medal ordinal at `result + 0x1160`: 0 `gold_med`, 1 `silver_med`, 2 `bronze_med`, otherwise nothing. The same
branch unpauses the matching trophy. Confidence 85.

**Wired** (`oag_game::endrace_voice`, the session's `tick_endrace`): `speech_results.bnk` is a second front-end bank,
`oag_title::FrontEndSounds::results` (`Some` on Pulse, `None` elsewhere), `oag_sound::sfx::BankName::SpeechResults`
and `Cue::RESULTS`, loaded by `MenuSfx` beside the navigation cues and only when the title names it. The bank rides
the speech bus, **chosen, not measured**. `FRONT_END_VO.bnk` (team and circuit names) can reuse the same shape: one
more optional path in `FrontEndSounds`, one more `BankName`.

**Cross-title** (census by `endrace_voice_ground_truth.rs`):

| Title | Result |
| --- | --- |
| Pulse PSP (USA, EU) | **ported**: all six names resolve, `FIRST_PLACE`.. 2 waveforms each, `gold_med`.. 4 each |
| Pulse PS2 (EU) | **ported**: same bank under the same name in `WADS2.WAD`, same counts |
| Pure PSP | **checked, differs**: no `speech_results.bnk` entry; `gold_med`/`silver_med`/`bronze_med` sit in `speech.bnk`, there is no place line there. Its call sites were not read, so nothing is wired |
| HD / Fury | **checked, applies, not wired**: `speech_results.bnk` ships with all six names (2 waveforms each) and the boot loads it (`sound-bank-loads.md`); HD's call sites were not read |
| 2048, Omega | **not checkable**: both load a `speech_results` bank (`game-boot.md`) but as a nameless v5 bank / Wwise events, with no SFX path to resolve a cue by name |

## Wired by this lane

`oag_sound::sfx::Cue::MenuUpDown`, `MenuLeftRight`, `MenuAccept`, `MenuDecline` (bank `BankName::Frontend`, named by
`oag_title::SoundBanks::frontend`, `Some` on Pulse and Pure only) and `oag_ui::menu::nav::Nav`, which the main menu,
the selection screens, the campaign screens and the EndRace menu raise. Chosen, not measured, and scored nowhere:
pointer hover and click use the pad's cues (hover sounds once per row change); `LEFTRIGHT` on confirming a toggle (the original has no confirm-steps-
value); `DECLINE` on closing the root menu. Pure's triggers are Pulse's read applied (Pure's executable carries the
same six names, minus `PAUSE`, but its call sites were not read).
