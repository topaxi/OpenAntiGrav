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
2. **The menu navigation sounds' session path has no test of its own** (`Session::play_navs`): the ground truth drives
   the same `Menu`, `take_nav`, `menu_cue` and `MenuSfx::play`, and a windowed `oag-game --no-audio` run showed the
   `menu sound:` lines for Down, Return, BackSpace and Right. Dropping the call in `frame.rs` or the load in `prepare.rs`
   would only show there. The voices below have a seam now; `play_navs` does not. (The 2026-10-11 walk also logged
   `MenuUp/Down/StepLeft/StepRight/Accept/Decline` through the whole front end, now into a WAV.)
3. ~~`TELETYPE`~~ **Closed 2026-10-11 (negative):** it is a per-menu-row cue off the `Menu` class, and 0 hits on PPSSPP across boot, main menu, single player and racebox; no authored `MenuDisplayType` is `TELETYPE` or `SCROLLING` (evidence in menu-sounds.md). Open only: a Menu with no authored type on an unwalked page, and the silent `<Text DisplayType="TELETYPE">` type-in the port draws whole (rate unmeasured).
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
8. **HD**: wired 2026-10-08 (lane `hd-menu-sfx`): `navUp`/`navDown`/`navLeft`/`navRight`, `accept`/`reject` and the
   `_fury` spellings, from Title data (`SoundBanks::frontend.cues`), pad and pointer. Evidence
   [menu-sounds.md](../../docs/ghidra/functions/ps3-hdfury-eu/menu-sounds.md). Open on HD: a blocked step plays `reject`
   in the original and the port's rows wrap; `unlockcell`/`unlockscreen` (cell unlock, `ShowUnlocks`), `~podiumticker`/
   `podiumtickend`, `wipe_med` (flyer selection), `TextBox01..04`, the team and circuit name voices (Fury also plays
   `accept_fury` after each); the `0x19` no-repeat rule; the rear-half whoosh is centred (chosen, not measured); not
   heard by a human. **2048**: a nameless v5 bank. **Omega**: the same eight cues as Wwise events, no SFX path.

## Next Steps

1. Dump PPSSPP's audio for the same walk and compare the cue moments and levels.
2. ~~Read the guard `FUN_0898dac0` feeds, then play `TMTX_NAME`/`TRTX_NAME`~~ done 2026-10-11 (item 4).
