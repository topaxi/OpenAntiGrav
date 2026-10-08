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
| `TELETYPE` | 1 | 1 | a text line typing in, one cue per character |
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

`FUN_088b6514` plays it once per revealed character with a pitch argument (the fifth argument to
`Sound_PlayNamedInSlot`): `(width / 100 - 1) * 0.2 * 90`, `+360` when negative, truncated to an integer, where
`width` is the widget's measured text width. Not wired: no text-reveal effect in this port types characters in.

## `PAUSE`

No code reference. A byte search of `BOOT.BIN` for `PAUSE\0` has no hit, so the executable never spells it; the
pause menu (`InGame Pause`, `0x08a795ec`) is driven by the same `ConfirmButton_Update` cues as any other. Whether a
formatted name could reach it was not ruled out (confidence 60 that nothing plays it).

## The voice banks (not wired)

| Slot | Cue | Caller | Selector |
| --- | --- | --- | --- |
| `FRONT_END_VO` | `TMTX_NAME` | `FUN_088ea048` | `FUN_0898dac0(2, param_2)` before the play (the team) |
| `FRONT_END_VO` | `TRTX_NAME` | `FUN_088ed9a8` | `FUN_0898dac0(1, param_2)` (the circuit) |
| `speech_results` | `FIRST_PLACE` / `SECOND_PLACE` / `THIRD_PLACE` | `EndRaceResults_Update` | none; once per results screen, off the finishing position |
| `speech_results` | `gold_med` / `silver_med` / `bronze_med` | `EndRaceRewards_OnEnter` | none; on entry, when a medal was won |

`FUN_0898dac0(slot, value)` stores `value` in a 32-byte table at `0x08ac3247` for `slot` in `1..32`. The banks'
`TMTX_NAME` is a 24-waveform cue (21 s in all) and `TRTX_NAME` is 66 commands and no waveform of its own (child references): the table is the selector a
guard operand reads (`Scream_OpGuard`, per-voice and global variables, `sound.md`), but which global the guard reads
was not followed. Picking a waveform by team index would be an invention, so these stay unplayed. The place and medal
lines are plain named cues in a bank this port does not load; they are the next addition, not blocked on RE.

## Wired by this lane

`oag_sound::sfx::Cue::MenuUpDown`, `MenuLeftRight`, `MenuAccept`, `MenuDecline` (bank `BankName::Frontend`, named by
`oag_title::SoundBanks::frontend`, `Some` on Pulse and Pure only) and `oag_ui::menu::nav::Nav`, which the main menu,
the selection screens, the campaign screens and the EndRace menu raise. Chosen, not measured, and scored nowhere:
pointer hover and click use the pad's cues (hover sounds once per row change); `LEFTRIGHT` on confirming a toggle (the original has no confirm-steps-
value); `DECLINE` on closing the root menu. Pure's triggers are Pulse's read applied (Pure's executable carries the
same six names, minus `PAUSE`, but its call sites were not read).
