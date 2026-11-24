# The pause screen's music list (Pulse, PS2)

Addresses are `pulse-ps2-eu` `SCES_547.48`, image base `0x00100000`. Read 2026-11-24 (lane `pulse-pause-music-re`)
with the Ghidra decompiler against the PSP's reading, which was watched live
([psp-pulse-usa/music-list.md](../psp-pulse-usa/music-list.md)). Every function below has the PSP twin's body, with the
`List` fields 8 bytes further on (`+0x98` selection becomes `+0x9c`, `Wrap` `+0x102` becomes `+0x114`). Static on this
binary; the confidences lean on the two binaries agreeing and the PSP's live run.

| Address | Name | Conf | What it is | PSP twin |
| --- | --- | ---: | --- | --- |
| `0x0019b7a0` | `MusicList_RegisterClass` | 85 | Registers `MusicList` (`0x002af068`) with the vtable `0x00296e10` | `0x088bb4a0` |
| `0x0019b618` | `MusicList_Update` | 80 | Rebuilds the entries when the music set reports a change, then `ListWidget_Update` | `0x088bb264` |
| `0x0019b670` | `MusicList_RebuildEntries` | 80 | One entry per music node (its `+0x78` name), the selection the player's `+0x48`, clamped (`pmaxw`/`pminw`) | `0x088bb358` |
| `0x0019b578` | `MusicList_Enter` | 80 | `ListWidget_Enter`, `+0xdc = 1` (works without focus), `MusicList_RebuildEntries` | `0x088bb2c0` |
| `0x0019b5b0` | `MusicList_Leave` | 85 | `if (sel != player+0x48 && g_game_paused == 0) MusicPlayer_StartNextSong(player, sel)` (disassembly: `a0` the player, `a1` `+0x9c`) | `0x088bb2f4` |
| `0x001f9800` | `MusicPlayer_StartNextSong` | 80 | The PSP's law: a pick in range plays from its start and becomes `+0x48`; before the race runs it only stores the pick (`+0x38c`) | `0x08938e70` |
| `0x001942b0` | `ListWidget_Construct` | 85 | Step buttons `+0xc4..+0xd0` = 0, 1, 2, 3 (UP, DOWN, LEFT, RIGHT); arrows 25 x 28 at x - 32 and - 20; `Wrap`, `AutoRepeat` off | `0x088a9520` |
| `0x00194588` | `ListWidget_ReadElement` | 80 | The same attributes: `Wrap` to `+0x114`, `AutoRepeat` to `+0x11c`, `workWhenNotFocus` to `+0xdc`, `swapbuttons` | `0x088a9f3c` |
| `0x00195518` | `ListWidget_Enter` | 80 | The two arrow images, UV `(282, 68)` and `(306, 68)`, then `ListWidget_ShadeArrows` | `0x088aa580` |
| `0x00194ce8` | `ListWidget_Update` | 75 | The focus and value steps, the arrows | `0x088aaae4` |
| `0x00194b40` | `ListWidget_StepBack` | 85 | Selection - 1 with `UPDOWN`; at 0 `DECLINE` and no change unless `Wrap` | `0x088a9db4` |
| `0x00194bf8` | `ListWidget_StepForward` | 85 | Selection + 1 with `UPDOWN`; at the last `DECLINE` and no change unless `Wrap` | `0x088a9e7c` |
| `0x00195a18` | `ListWidget_ShadeArrows` | 85 | Each arrow half in `ArrowColor`, its alpha halved while the selection is at that half's end | `0x088a9ba4` |
| `0x0027a5c0` | `g_game_paused` | 70 | Read by `MusicList_Leave` where the PSP reads its paused flag (data) | `0x08ab0628` |

The cue names are packed into the calls as 8-byte constants: `0x4e574f445055` is `UPDOWN`, `0x454e494c434544`
`DECLINE`.

**So PS2 Pulse plays the PSP's law**: LEFT and RIGHT step the list whichever row the cursor is on, no wrap, `UPDOWN`
and `DECLINE`, nothing heard while paused, the pick started from its beginning on resume. What differs is the screen's
authoring alone (`y="270"`, `font="menu"`, `docs/ui/pause-menu.md`, "Pulse").
