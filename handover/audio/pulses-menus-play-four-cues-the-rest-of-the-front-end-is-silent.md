# Pulse's menus play four cues; the rest of the front end's sounds are open

2026-10-08, lane `pulse-menu-sfx`. Evidence: [menu-sounds.md](../../docs/ghidra/functions/psp-pulse-usa/menu-sounds.md),
[psp-audio.md](../../docs/formats/psp-audio.md#the-front-ends-navigation-sounds-2026-10-08). The main menu, the
race-box selection screens, the campaign screens and the EndRace menu now play `UPDOWN`, `LEFTRIGHT`, `ACCEPT` and
`DECLINE` from `frontend.bnk` on Pulse and Pure, pad and pointer. A headless walk writes
a `menu-walk-<disc>.wav` (written by `menu_sfx_ground_truth.rs`).

## Open

1. **Not heard by a human, and not compared with the original.** The WAV is checked for energy on the tick of each
   press and silence before the first; a PPSSPP audio dump of the same walk was not made. `ACCEPT` and `DECLINE`
   saturate the 16-bit output on Pulse at the default level (the bank is authored hot and the mixer clamps as the
   hardware does).
2. **The session path's seam is partial.** `oag_game::sound::load_menu_sfx` (the one load `prepare.rs` and the
   ground truths share) and `oag_game::sound::play_nav` (what `Session::play_navs` does per nav, now also what
   `menu_sfx_ground_truth.rs` calls) are tested against the disc; every `take_nav` is `#[must_use]`, so a drive that
   drops the drain fails `-D warnings`. What no test reaches is `frame.rs` calling `play_navs` and `prepare.rs`
   storing the result: a `Session` needs a GPU and a window, and no headless driver of one exists. A windowed
   `oag-game --no-audio` run under Xvfb with xdotool (pointer moves and clicks) logged the `menu sound:` lines for the boot
   picker, the main menu, the options pages and the rename prompt on 2026-10-16.
3. ~~`TELETYPE`~~ **Closed 2026-10-11 (negative):** it is a per-menu-row cue off the `Menu` class, and 0 hits on PPSSPP across boot, main menu, single player and racebox; no authored `MenuDisplayType` is `TELETYPE` or `SCROLLING` (evidence in menu-sounds.md). Open only: a Menu with no authored type on an unwalked page, and ~~the silent `<Text DisplayType="TELETYPE">` type-in~~ **closed 2026-10-11 (lane `pulse-typein`, negative): the original's `Text` class has no type-in, it draws whole like the port** ([text-display-type.md](../../docs/ghidra/functions/psp-pulse-usa/text-display-type.md)); open only: HD/Omega's `Text` class (105 authored on HD) is unread and uncaptured.
   The place and medal lines landed 2026-10-11 (lane `pulse-endrace-voice`, [menu-sounds.md](../../docs/ghidra/functions/psp-pulse-usa/menu-sounds.md#the-end-of-race-voices)):
   Single Race and Eliminator on `Results` entry, Tournament at the first page flip off the standings rank,
   the medal on `Rewards` entry in a campaign cell. **Walked for real 2026-10-11** (lane `voices-ingame`: `ResultsThird` on
   Single Race and a campaign cell, `MedalBronze` on `Rewards`, in a WAV of the session's own mix, [menu-sounds.md](../../docs/ghidra/functions/psp-pulse-usa/menu-sounds.md#played-in-a-real-walk-2026-10-11-lane-voices-ingame)); the session wiring is
   guarded by `-D warnings` dead-code lints rather than a test, see that page. Open on them: Eliminator, Tournament and the gold/silver medals were not walked; not heard by a human; on the speech bus (chosen); whether
   `VIEW RESULTS AGAIN` speaking the place again (chosen, not measured); HD: wired 2026-10-11 (lane `hd-results-voice`, [endrace-voices.md](../../docs/ghidra/functions/ps3-hdfury-eu/endrace-voices.md)): place on `EndRace Results` entry in a free race, medal instead in a cell (played on Results because the port has no HD photo screen, chosen). Open on HD: the unread bytes `craft+0x7002` and `*0x009384e1`, which mode id a free Single Race carries (a cell logged 3), the Eliminator's word 0, the `*_shrt` medals (`0x000e4a30`) and `RACE_COMPLETE`/`TOURN_COMPLETE`/`SESS_COMPLETE`/`FINAL_LAP` (`RaceManager_Update`), and a live RPCS3 confirmation.
4. ~~**Not wired, selector unread:** `TMTX_NAME` and `TRTX_NAME`~~ **Landed 2026-10-11 (lane `pulse-name-voices`,
   [menu-sounds.md](../../docs/ghidra/functions/psp-pulse-usa/menu-sounds.md#the-name-voices)):** the guard reads the table
   `Sound_PlayNamedInSlot` writes, the value is the `soundregister` each `PI_Team`/`PI_Track` authors, and the voice
   sounds on Confirm of Team Selection and Track Select, pad and pointer. **Walked for real 2026-10-11** (Track 16_Track register 1, Ship Assegai register 4, same frame as the `ACCEPT`; a campaign cell speaks the team only). Open on it: not heard by a human; on the
   speech bus and dry (chosen, not measured); the four DLC teams' registers are inferred (2, 6, 11, 12), no pack was
   mounted on the capture; the PS2 trigger was not observed; HD's voice bank (plays on a team *change*, in a bank this
   disc's `frontend.bnk`/`speech.bnk` do not carry) is not located.
5. ~~**Confirm dialogs, the on-screen keyboard, the language picker, the boot front end and the pause menu's own
   `ConfirmButton`s** raise no sound~~ **Routed 2026-10-16 (lane `pulse-frontend-sfx`,
   [menu-sounds.md](../../docs/ghidra/functions/psp-pulse-usa/menu-sounds.md#text-entry-dialogs-and-the-boot-2026-10-16-lane-pulse-frontend-sfx)):**
   measured on PPSSPP with a breakpoint on `Sound_PlayNamedInSlot`. The name and tag entry is `TagInput_Update`
   (`0x088cae88`), and `TagEntry` now plays its cues (glyph scroll `UPDOWN`, a step `LEFTRIGHT`, a step it cannot take
   `DECLINE`, Cross on the slot `DECLINE` then `ACCEPT`), pad and pointer. The boot's movie skip and `Show Logo`'s press
   play `ACCEPT`; START pausing or resuming a race plays `ACCEPT`; the pause menu is a `Menu` with no confirm dialog
   (`QUIT RACE` is one `ACCEPT`). The yes/no `Confirm`, the key grid `Keyboard` and the language picker have no measured
   counterpart and borrow those cues (chosen, not measured; the picker by widget class, since Pulse USA never shows it).
   A cell the campaign cannot launch (`launch_campaign_cell`'s notice) plays `DECLINE` instead of `ACCEPT`.
   Open on it: the port's Cross on a `TagEntry` cell does not advance and its Circle cancels, where the original
   advances and steps back (behaviour, not sound, left alone); the `+0x129` flag that caps the advance one short of
   the slot (AllowBlank?) was not measured; the EU disc's picker and Pure's `TagInput` were not walked (the probe's
   address is the USA binary's; Pure is not open in the shared bridge); `Show Logo` takes CROSS in the original and
   this build reads START only; the PS2 executable was not read. The campaign grids still never move onto an empty
   cell, so the original's `DECLINE`-instead-of-`UPDOWN` move has no counterpart.
6. **Chosen, not measured:** pointer hover and click use the pad's cues; confirming a toggle row plays `LEFTRIGHT`;
   closing the root menu plays `DECLINE`; `Results` and `Rewards` advancing plays `ACCEPT`. The widget flag `+0x269`
   (silences `ACCEPT`) was not swept, so a few original buttons that stay silent here make a sound.
7. **A menu SFX volume row** was not added; the sounds ride the SFX bus (`Bus::Sfx`) and its setting.
8. **HD**: (2026-10-16: pausing a race with START, the rename/delete prompts and the boot screens now also play HD's
   `accept`/`navUp`/... through the shared path, unmeasured on HD; a lane may gate them to Pulse.) wired 2026-10-08 (lane `hd-menu-sfx`): `navUp`/`navDown`/`navLeft`/`navRight`, `accept`/`reject` and the
   `_fury` spellings, from Title data (`SoundBanks::frontend.cues`), pad and pointer. Evidence
   [menu-sounds.md](../../docs/ghidra/functions/ps3-hdfury-eu/menu-sounds.md). Open on HD: a blocked step plays `reject`
   in the original and the port's rows wrap; `unlockcell`/`unlockscreen` (cell unlock, `ShowUnlocks`), `~podiumticker`/
   `podiumtickend`, `wipe_med` (flyer selection), `TextBox01..04`, the team and circuit name voices (Fury also plays
   `accept_fury` after each); the `0x19` no-repeat rule; the rear-half whoosh is centred (chosen, not measured); not
   heard by a human. **2048**: a nameless v5 bank. **Omega**: the same eight cues as Wwise events, no SFX path.

## Next Steps

1. Dump PPSSPP's audio for the same walk and compare the cue moments and levels.
2. ~~Read the guard `FUN_0898dac0` feeds, then play `TMTX_NAME`/`TRTX_NAME`~~ done 2026-10-11 (item 4).
