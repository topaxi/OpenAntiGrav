# The end of a split-screen race

What a race several people flew does after the first person crosses the line, when
the last one has, and on the screens that follow. **An enhancement of this
project's**: the original's split screen was never read beyond the authored data
below, so everything under "What is chosen" is **chosen, not measured** and carries
no confidence score. The race itself is [split-screen.md](../rendering/split-screen.md).

Code: `crates/raceplay/src/telemetry.rs` (`Race::finished_view`, the held readout),
`crates/raceplay/src/viewpoint.rs` (`Race::views_drawn`),
`crates/game/src/results_tables.rs` (the rows, `split_people`),
`crates/ui-screens/src/endrace/split.rs` (`Results Split`), `.../draw.rs`
(`split_rewards_draw_list`), `.../borrow.rs` (the PSP's borrowed copies),
`.../table.rs` (the grid geometry), `.../people.rs` (Pulse's one table),
`.../hd.rs` (HD's grid), `crates/game/src/main/session/endrace.rs` (the split branch of
`build_endrace`). Tests: `crates/raceplay/src/results/tests.rs`,
`crates/game/src/results_tables.rs`, the unit tests in `crates/ui-screens/src/endrace/`
(`split/tests.rs`, `table/tests.rs`), and the disc-backed
`crates/game/tests/split_end_race_ground_truth.rs` (the race to the end) and
`crates/game/tests/endrace_split_ground_truth.rs` (the screens against both discs). Stills:
`--menu-page endrace-results-split` / `endrace-rewards-split` / `endrace-results-people` on a
Pulse source (either pressing), `endrace-results-people` on HD.

## What the originals author

| Source | What it says | Used |
| --- | --- | --- |
| Pulse PSP, `EndRaceResults_PopulateMultiplayerTable` `0x088d9588` (78) | Fills the ordinary `EndRace Results` table with `PRO_POS` / `PRO_NAME` / `PRO_TOT_TIME` and a row per craft: place, name, finishing time (`ER_RACING` for a craft still racing, `ER_DNF` for -1), the player's row highlighted. Gated on the network-play modes (`g_game_mode > 0xd`), **not** on split screen. | **Yes, for every split race on a Pulse source.** Only the gate is ours. |
| HD, `EndRace Podium` (`ps3-hdfury-eu/endrace-podium.md`, 74) | The end of a race run by the `MPRaceManager` family (online), three places on plinths, `is_local_player` colouring the local names. | **No**: checked, applies, not wired. Entered from the multiplayer managers only and its portraits (`pod_img`) are unread. |
| Pulse PS2, `Data\Plugins\PI001\GUI\EndRace_Definition.xml` in `WADS2.WAD` | **Authors `EndRace Results Split` and `EndRace Rewards Split`** (the PS2 executable names both, beside `Split ScreenV`/`Split ScreenH` and `Kill Game Multiplayer`). Results Split is two per-player lap tables stacked in the 640x448 space: `<Item name="Player_0" OffsetY="-96">` and `Player_1` at `OffsetY="110"`, each its own `Line1_N` headline, `table_N`, eight `tablebg{r}_N` rows at `OffsetY` 152 + 25 (r - 1), `lap{r}.{c}_N` cells, `perfectlap`, `boostimg` and `tablehighlight` copies. Its `EndRaceRewardsRedirect` goes to `EndRace Rewards Split` (`PlayerRW_0` at -52 and `PlayerRW_1` at 176, a trophy, medal, loyalty row and bar each), then `Race End Save`, then the ordinary `EndRace Menu`, whose `IG_PAUSE_QUIT` row goes to `Kill Game Multiplayer`. | **Yes, since 2026-10-13, on both Pulse pressings** (the PS2 plays its own file, the PSP borrows the design, see "The two-person screens"). Two people only. |
| 2048, Omega | No split end screen found. 2048's pages are touch pages for one racer; Omega's are HD's. | see "Other titles" |

Reproduce the PS2 read: `oag-wad cat --expand "data/images/pulse-ps2-eu.chd:54748/WADS2.WAD" 'Data\Plugins\PI001\GUI\EndRace_Definition.xml'`.
What enters `Results Split` (which screen the race manager goes to, and on what
count) is in the executable and was **not read**.

## What a player sees

1. **Between their own line and the last person's**, a person's view holds: the
   lap counter stays at the target, the lap and race clocks stop at their crossing,
   the place readout shows the place the field gave them at that tick, and the
   layout's last message line (`Info4`) reads `FINISHED` (`OAG_SPLIT_FINISHED`, our
   own text, translated in the four project languages). The other views race on.
   Their craft is flown by the driver as a single-player craft is after the line
   (`Race::flown_for_the_player`, per slot), at the original's post-finish thrust.
2. **When the last person finishes** the race is over. One view is drawn
   (`Race::views_drawn`, not `viewport_count`, which stays the answer to "was this a
   split"), so `Race End Photo` and the panels sit over the one picture a
   single-player finish has: the spectator camera, starting on slot 0's craft.
3. **`EndRace Results`**: on **Pulse, with exactly two people**, `EndRace Results Split`:
   a lap table each, stacked, the first person's on top, each with its own headline
   (`5TH PLACE`, or what the mode says), laps, the speedup-pad column (the second person's is
   blank: the race keeps the tally for the first craft alone) and a highlighted totals row.
   **Pulse with another count** keeps the multiplayer table above, every craft in place
   order, each person named `PLAYER 1`, `PLAYER 2` by the seat they flew, each person's row
   highlighted, a craft still racing reading `RACING`. On HD the field grid with the same
   names in the column the original fills with a name, and each person's row highlighted; no
   single place is the headline. Pure and a source with no `EndRace_Definition.xml` keep the
   built-in table, which names `P1`, `P2`.
3a. **`EndRace Rewards Split`** follows `Results Split` (Pulse, two people): a panel each, `NO
   MEDAL AWARDED`, the person's team and what the race earned it, its running total and the
   loyalty bar. Confirm goes on to the menu: the file's `Race End Save` has nothing to do in a
   build whose progress persists by itself.
4. **`EndRace Menu`** as on any race: `RETURN TO MENU`, `RACE AGAIN` and `VIEW RESULTS AGAIN`.
   `RACE AGAIN` rebuilds the race with the same seats (`split_screen::seat_for_launch`
   reads `Controls::assignment().humans()`); leaving unseats (`unseat_after_split_race`).

## What is chosen, not measured

- That the Pulse multiplayer table answers a split race, and the names `PLAYER n`
  by seat (the original's `+0x35` name is a network profile's).
- The `FINISHED` line, what is held in a finished view, and the place readout
  staying up. The original's HUD vanishes at its one player's finish.
- One full view at the end rather than the split under the panels.
- **Loyalty for a split, resolved 2026-10-13 (`pulse-split-pages`):** with two people on a
  Pulse source each person **banks their own team's loyalty**, under the law a one-person
  race does (`Race_ComputeLoyaltyAward`, from their craft's laps and kills), and
  `Rewards Split` shows it: the authored panel is a loyalty row and bar per person, which has
  nothing to say unless it was earned. `Session::build_endrace` builds a `Rewards` per person
  (`medal: None, campaign: false`, so `NO MEDAL AWARDED` and no medal glyph, as a custom race's)
  and saves the records once (two people on the same team, which Ship Select allows, bank two
  awards to it). **Chosen, not measured**, as is showing `Rewards Split` at all:
  the one-person flow shows `Rewards` only after a campaign cell, and a split is never one, but
  `Results Split` authors the redirect to it as the file's only route and the panels exist for
  nothing else. **Still none for a split:** no campaign medal, no lap or race record, no
  ghost, and the results voice is not played, since no one place is the race's. With
  other counts, other titles or a source without the two screens: the generic table above, no
  loyalty. A split Tournament has no next leg row.
- **2048 keeps the built-in table** over its touch pages: they summarise one racer,
  which would be slot 0's alone (`build_endrace_touch`). It offers no `RACE AGAIN`.
- The HD `EndRace Podium` is not entered.

## Other titles

- **HD**: ported, with its own draw tests (`a_two_person_race_names_and_highlights_each_person`).
- **Omega**: shares HD's dialect and this code path (checked, applies, wired by
  the same function); no Omega race could be looked at on this machine's extraction.
- **2048**: checked, differs (touch pages, above). Its `EndRace` entry is not
  Omega's.
- **Pure**: no `EndRace` screen is read (`endrace_entry` is `None`), so the built-in
  table, with `P1`/`P2`.
- **Pulse PS2**: the same path as the PSP; the PS2's own split screens are open.

## The two-person screens: the PS2 plays its file, the PSP borrows its design (2026-10-13)

**Decided by the maintainer (2026-10-13):** where only the PS2 pressing authors a split
screen, the PSP pressing borrows the PS2's design, built from the PSP's own assets and
labelled "borrowed from the PS2 pressing". So `EndRace Results Split` and `EndRace Rewards
Split` are wired for **Pulse on both pressings** from one definition
(`oag_title::FrontEnd::borrows_ps2_split_screens`, true for Pulse alone, so Pure and the
other titles never borrow).

- **PS2 pressing**: `oag_game::endrace::load` reads both screens off `EndRace_Definition.xml`
  (`Layout::read`), and the loader's report says `Results Split read from the disc`.
- **PSP pressings** (`pulse-psp-usa`, `pulse-psp-eu`): `oag_ui_screens::endrace::borrow`
  builds them and the report says `borrowed from the PS2 pressing (this pressing authors none,
  placement not measured on it)`. **No confidence score for the PSP placement.**
  - `Results Split` is generated from `borrow::PS2_TABLE`, the numbers the PS2 file authors
    per person (item offsets -96 and 110, a 25 px pitch, row fill heights 28 and 23, the cell
    columns and rows, the header bar, the boost icon), scaled by the grid ratio
    (480/640 across, 272/448 down), with the PSP's own sprites, `Line1` and `ContinueButton`.
  - `Rewards Split` is the PSP's own single `EndRace Rewards` panel stamped twice at the PS2's
    offsets (-52 and 176) scaled, plus a header per panel: the PS2's single panel is the
    PSP's scaled by the same ratio (`BGgradient` at 91 against the PSP's 55, `MedalImg`
    43x53 against 32x32), so nothing about a panel is transcribed.
  - **Chosen for the PSP's smaller room:** its faces are as tall in 272 rows as the PS2's are in
    448, so the two headlines (and `Rewards Split`'s headers) are drawn at 0.85 and lifted to end
    just above the rule they sit over (`borrow::FIT_SCALE`).
- **The numbers stay re-derivable.** The borrow functions take the grid they express their
  layout in; `endrace_split_ground_truth.rs` builds them in the PS2's own 640x448 and asserts
  they draw exactly what the file draws (widget for widget, the file's one authored duplicate
  `topbarcenter_0` aside), and that the PSP's borrowed Rewards panels sit where the PS2's do
  scaled (within a pixel down the screen, 8 across: the PSP's single panel is hand-authored at
  about the PS2's ratio).
- **What the entry law is, unread.** What sends the race manager to `Results Split`, and on what
  count, is in the PS2 executable, which is not in a Ghidra project here. Two people is the
  file's (two `Player_N` items); the rest is chosen as above. The redirect chain the file
  authors is `Results Split` -> `Rewards Split` -> `Race End Save` -> `EndRace Menu`; the flow
  follows it with `Race End Save` as nothing (progress persists, no save slots), and a disc test
  asserts the file's gotos on both pressings.
- **Pointer and pad**: both screens answer a confirm anywhere, as `Results`/`Rewards` do; no
  row is new.
- **Not in the PS2 pressing's WAD under their names**: `hex_bg.mip` and the reward icons are
  stored as `.pct` there; the loader now reads them through `oag_pulse::read_image` like the
  campaign screens do, so the PS2's row tiles and `LoyaltyImg` draw (they drew nothing before).

`Team SelectionSplit` is borrowed the same way, see
[selection-screens.md](selection-screens.md#the-split-screens-team-selection-2026-10-13).

## Open

- ~~On a PS2 source every `EndRace Results` table's highlight is misplaced~~ **Fixed
  2026-10-13**: the grid reads each file's own geometry
  ([endrace-screens.md](endrace-screens.md#the-tables-rows-are-the-files-own-not-the-psps-2026-10-13)).
- **The PS2's highlight placement is chosen, not measured** (its executable is not in a Ghidra
  project): the row's fill top, which is the PSP's literal on the PSP's file.
- **The headline overlaps the top-left bar on the PSP's `Results Split`** (the borrowed
  headline ends at the table's bar, the screen's own top bar is bright): readable, not pretty.
- **The second person's pad tally is blank.** `RunStats::boosts_by_lap` is the first craft's.
- A live walk with two **real** pads: the end screens were walked live on the PSP pressing in
  an Xvfb with two virtual uinput pads (join, Track Select, the borrowed shared Ship Select, a
  race to its end, `Race End Photo`, `Results Split`, `Rewards Split`, the menu and `VIEW RESULTS
  AGAIN`), and the PS2's through stills and the disc tests; no walk on the PS2 pressing or on
  hardware.
- The HD podium for a split; per-person records and ghosts (a split writes none).
- HD's `LOYALTY`/`TOTAL` blocks draw empty on a split (as on any HD race with no
  team behind it).
