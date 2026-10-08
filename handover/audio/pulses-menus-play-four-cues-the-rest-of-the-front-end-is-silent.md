# Pulse's menus play four cues; the rest of the front end's sounds are open

2026-10-08, lane `pulse-menu-sfx`. Evidence: [menu-sounds.md](../../docs/ghidra/functions/psp-pulse-usa/menu-sounds.md),
[psp-audio.md](../../docs/formats/psp-audio.md#the-front-ends-navigation-sounds-2026-10-08). The main menu, the
race-box selection screens, the campaign screens and the EndRace menu now play `UPDOWN`, `LEFTRIGHT`, `ACCEPT` and
`DECLINE` from `frontend.bnk` on Pulse and Pure, pad and pointer. A headless walk writes
`data/shots/menu-walk-<disc>.wav` (`menu_sfx_ground_truth.rs`).

## Open

1. **Not heard by a human, and not compared with the original.** The WAV is checked for energy on the tick of each
   press and silence before the first; a PPSSPP audio dump of the same walk was not made. `ACCEPT` and `DECLINE`
   saturate the 16-bit output on Pulse at the default level (the bank is authored hot and the mixer clamps as the
   hardware does).
2. **The session's play path has no test of its own.** `Session::play_navs` lives in the binary; the ground truth
   drives the same `Menu`, `take_nav`, `menu_cue` and `MenuSfx::play`. It was run once for real: a windowed
   `oag-game --no-audio` in a private Xvfb, keys injected with `xdotool`, the log showing the four `sfx:` load lines at
   boot and `menu sound: UPDOWN/ACCEPT/DECLINE/LEFTRIGHT (started: true)` for Down, Return on a submenu, BackSpace
   and Right on a choice. Dropping the call in `frame.rs` or the load in `prepare.rs` would only show there.
3. **Not wired, recovered:** `TELETYPE` (needs a text-reveal effect; the pitch argument is `(width/100 - 1) * 0.2 * 90`),
   the place lines (`FIRST_PLACE` ...) and medal lines (`gold_med` ...) from `speech_results.bnk` on EndRace, which are
   plain named cues in a bank the port does not load.
4. **Not wired, selector unread:** `TMTX_NAME` and `TRTX_NAME` in `FRONT_END_VO.bnk` (team and circuit name voices on
   the selection screens). The selector is `FUN_0898dac0(slot, value)` writing a 32-byte table; which guard reads it
   is not followed. A pick by index would be an invention.
5. **Confirm dialogs, the on-screen keyboard, the language picker, the boot front end and the pause menu's own
   `ConfirmButton`s** raise no sound: only the screens above were routed. A locked tier or cell on the campaign
   screens plays `DECLINE` (`Session::campaign_nav`, read-only on `handle_campaign`'s own refusal tests, the one line
   of `campaign.rs` this lane touched). A launch the campaign then refuses (`launch_campaign_cell`'s notice) is
   silent. The campaign grids never move onto an empty cell, so the original's `DECLINE`-instead-of-`UPDOWN` move has
   no counterpart.
6. **Chosen, not measured:** pointer hover and click use the pad's cues; confirming a toggle row plays `LEFTRIGHT`;
   closing the root menu plays `DECLINE`; `Results` and `Rewards` advancing plays `ACCEPT`. The widget flag `+0x269`
   (silences `ACCEPT`) was not swept, so a few original buttons that stay silent here make a sound.
7. **A menu SFX volume row** was not added; the sounds ride the SFX bus (`Bus::Sfx`) and its setting.
8. **HD**: `frontend.bnk` has 27 cues named `navUp`, `navDown`, `accept`, `reject` and so on; wiring is HD's own
   front-end reading. **2048**: a nameless v5 bank. **Omega**: Wwise.

## Next Steps

1. Dump PPSSPP's audio for the same walk and compare the cue moments and levels.
2. Load `speech_results.bnk` as a second front-end bank and play the place and medal lines off the EndRace models.
3. Read the guard `FUN_0898dac0` feeds, then play `TMTX_NAME`/`TRTX_NAME` on the selection screens.
