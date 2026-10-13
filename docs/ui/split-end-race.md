# The end of a split-screen race

What a race several people flew does after the first person crosses the line, when
the last one has, and on the screens that follow. **An enhancement of this
project's**: the original's split screen was never read beyond the authored data
below, so everything under "What is chosen" is **chosen, not measured** and carries
no confidence score. The race itself is [split-screen.md](../rendering/split-screen.md).

Code: `crates/raceplay/src/telemetry.rs` (`Race::finished_view`, the held readout),
`crates/raceplay/src/viewpoint.rs` (`Race::views_drawn`),
`crates/game/src/results_tables.rs` (the rows), `crates/ui-screens/src/endrace/people.rs`
(Pulse's table), `crates/ui-screens/src/endrace/hd.rs` (HD's grid),
`crates/game/src/main/session/endrace.rs` (the split branch of `build_endrace`).
Tests: `crates/raceplay/src/results/tests.rs`, `crates/game/src/results_tables.rs`,
the draw tests in `crates/ui-screens/src/endrace/`, and the disc-backed
`crates/game/tests/split_end_race_ground_truth.rs`. Stills:
`--menu-page endrace-results-people` on a Pulse or HD source.

## What the originals author

| Source | What it says | Used |
| --- | --- | --- |
| Pulse PSP, `EndRaceResults_PopulateMultiplayerTable` `0x088d9588` (78) | Fills the ordinary `EndRace Results` table with `PRO_POS` / `PRO_NAME` / `PRO_TOT_TIME` and a row per craft: place, name, finishing time (`ER_RACING` for a craft still racing, `ER_DNF` for -1), the player's row highlighted. Gated on the network-play modes (`g_game_mode > 0xd`), **not** on split screen. | **Yes, for every split race on a Pulse source.** Only the gate is ours. |
| HD, `EndRace Podium` (`ps3-hdfury-eu/endrace-podium.md`, 74) | The end of a race run by the `MPRaceManager` family (online), three places on plinths, `is_local_player` colouring the local names. | **No**: checked, applies, not wired. Entered from the multiplayer managers only and its portraits (`pod_img`) are unread. |
| Pulse PS2, `Data\Plugins\PI001\GUI\EndRace_Definition.xml` in `WADS2.WAD` | **Authors `EndRace Results Split` and `EndRace Rewards Split`** (the PS2 executable names both, beside `Split ScreenV`/`Split ScreenH` and `Kill Game Multiplayer`). Results Split is two per-player lap tables stacked in the 640x448 space: `<Item name="Player_0" OffsetY="-96">` and `Player_1` at `OffsetY="110"`, each its own `Line1_N` headline, `table_N`, eight `tablebg{r}_N` rows at `OffsetY` 152 + 25 (r - 1), `lap{r}.{c}_N` cells, `perfectlap`, `boostimg` and `tablehighlight` copies. Its `EndRaceRewardsRedirect` goes to `EndRace Rewards Split` (`PlayerRW_0` at -52 and `PlayerRW_1` at 176, a trophy, medal, loyalty row and bar each), then `Race End Save`, then the ordinary `EndRace Menu`, whose `IG_PAUSE_QUIT` row goes to `Kill Game Multiplayer`. | **Not yet**: found and read, left as the follow-up lane below (Pulse on both pressings: the PSP borrows this design). Two people only. |
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
3. **`EndRace Results`**: on Pulse the multiplayer table above, every craft in place
   order, each person named `PLAYER 1`, `PLAYER 2` by the seat they flew, each
   person's row highlighted (the one authored highlight drawn once per person), a
   craft still racing reading `RACING`. On HD the field grid with the same names in
   the column the original fills with a name, and each person's row highlighted; no
   single place is the headline. Pure and a source with no `EndRace_Definition.xml`
   keep the built-in table, which names `P1`, `P2`.
4. **`EndRace Menu`** as on any race: `RETURN TO MENU`, `RACE AGAIN` and `VIEW RESULTS AGAIN`.
   `RACE AGAIN` rebuilds the race with the same seats (`split_screen::seat_for_launch`
   reads `Controls::assignment().humans()`); leaving unseats (`unseat_after_split_race`).

## What is chosen, not measured

- That the Pulse multiplayer table answers a split race, and the names `PLAYER n`
  by seat (the original's `+0x35` name is a network profile's).
- The `FINISHED` line, what is held in a finished view, and the place readout
  staying up. The original's HUD vanishes at its one player's finish.
- One full view at the end rather than the split under the panels.
- **No loyalty, no medal, no record, no ghost and no announcer for a split. Provisional**
  (chosen, and to be revisited once `Rewards Split` lands: the PS2 authors a reward panel
  per person, trophy and loyalty bar included, so a split may well bank loyalty per person).
  `Session::build_endrace` banks no loyalty and builds no `Rewards`; the finish
  site writes no record (the earlier gate on `viewport_count`); the results voice is
  not played, since no one place is the race's. A split Tournament has no next
  leg row.
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

## Follow-up lane: the PS2's authored split end screens, for Pulse on both pressings

**Decided by the maintainer (2026-10-13):** where only the PS2 pressing authors a split
screen, the PSP pressing borrows the PS2's design, built from the PSP's own assets and
labelled "borrowed from the PS2 pressing" in the loader report. So this lane wires
`EndRace Results Split` and `EndRace Rewards Split` for **Pulse on both pressings** (the
same goes for `Team SelectionSplit`, which belongs to the ship-pick lane). It is not
PS2-only. Today the generic path above is what every Pulse split sees.

**Where the data is.** PS2 only: `data/images/pulse-ps2-eu.chd`, `54748/WADS2.WAD`, entry
`Data\Plugins\PI001\GUI\EndRace_Definition.xml` (the entry `oag_game::endrace::SCREEN_ENTRY`
already names; read it with `oag-wad cat --expand "<image>:54748/WADS2.WAD" '<entry>'`). The
screens are `EndRace Results Split` (type `EndRace Results`, about 800 lines) and
`EndRace Rewards Split` (type `EndRace Rewards`); the executable `SCES_547.48` names both.
The PSP file has no copy, so the PSP borrows the **layout numbers** (640x448 PS2 space,
rescaled to the PSP's 480x272 grid) and draws them with its own sprites and strings.

**What the screens are** (details in `docs/formats/endrace-screens.md`): two stacked
per-person lap tables, `Player_0` at `OffsetY` -96 and `Player_1` at 110, each with its own
`Line1_N` headline text, `table_N`, eight `tablebg{r}_N` rows (`OffsetY` 152 + 25 (r - 1),
height 28), `lap{r}.{c}_N`, `perfectlap{n}_N`, `boostimg_N`, `tablehighlight_N`; then
`EndRaceRewardsRedirect` -> `EndRace Rewards Split` (`PlayerRW_0` at -52, `PlayerRW_1` at
176: `BGgradient_N`, `BGhexgrid_N`, `TrophyPanel_N` with `g_/s_/b_trophy_N`, `MedalImg_N`,
`LoyaltyImg_N`, `BigPos_N`, `RewardLine1_N`/`RewardLine2_N`, `RewardLoyaltyActive_N`,
`loyaltynum_N`, `loyaltybg_N`/`loyaltybar_N`) -> `EndRaceRewardsMenuRedirect` -> `Race End
Save` -> the ordinary `EndRace Menu` (`IG_PAUSE_QUIT` -> `Kill Game Multiplayer`,
`ER_RACE_AGAIN` -> `InGame Restart`). Two people only; with other counts the generic table
stays.

**What has to change.**

1. **`table_layers` becomes suffix- and offset-aware.** `table::lap_slot` parses `lap3.1`;
   the Split names carry `_N` (`lap3.1_0`), so it needs to return the person suffix too.
   `row_of`, `with_row_offset`, `highlight_y` and `in_table` use the PSP file's constants
   (`FIRST_ROW_Y` 92, `ROW_PITCH` 20), which is also why **every PS2 `EndRace Results`
   table already highlights the wrong row** (see Open). They must read each table's own
   `tablebg{r}` offsets (and the enclosing `<Item OffsetY>`) instead. Fix that first, on
   the single-person PS2 file, with a `--menu-page` still beside the PS2 frame.
2. **Per-person content.** `Line1_N` is a per-person headline (the ordinal, as `Line1` is
   for one person; `headline_text(Headline::Position(place))`), and each table is that
   person's own lap table (`Standing::lap_splits`, `best_lap_ticks`; the data is per craft
   already, `build_endrace` just reads slot 0's). `Shown` per person.
3. **Rewards Split, and the question it raises.** Per person: a trophy (`campaign medal`,
   which a split never has), loyalty line and bar. **Open question for the maintainer:** does
   a split bank loyalty per person (to each person's team)? The rule today (no loyalty on a
   split) is chosen and provisional. Loyalty is per team in `records.toml`, and a split
   writes no record, so banking needs a decision on whose profile it is.
4. **Flow.** `Flow` is Photo -> Results -> (Rewards when a campaign cell was in play) ->
   Menu. The authored chain always goes Results Split -> Rewards Split. Whether the original
   shows the Rewards Split panels for a non-campaign split race is **not read**: the entry
   law (what sends the race manager to `Results Split`, on what seat count) is in the
   executable and unread. Read `FUN_`s referencing the string `EndRace Results Split` in
   `/pulse-ps2` Ghidra program if a PS2 project exists, else chosen.
5. **Pointer and pad.** Both screens answer a confirm anywhere (as `Results`/`Rewards`
   already do); no new row is interactive.
6. **Tests.** A disc-backed one over the PS2 file (the screen reads, 2 persons fill both
   tables) next to `split_end_race_ground_truth.rs`; a still per pressing, the PSP one
   labelled borrowed.

## Open

- **On a PS2 source every `EndRace Results` table's highlight is misplaced**, split or
  not: `table::highlight_y`, `row_of` and `with_row_offset` carry the PSP file's row
  geometry (first row 92, pitch 20) and the PS2 file authors its own (`tablebg1` at
  152, pitch 32.94 in 640x448). Seen on `--menu-page endrace-results-eliminator` (the
  eighth-placed row is highlighted on row 4) and on `endrace-results-people`
  (persons 5th and 7th highlighted on rows 2 and 3). The names and times are right.
  Found, not fixed: step 1 of the follow-up above.
- A live walk with two real pads: the end screens were checked through stills of
  the real draw functions and a headless race driven to its end, not through a
  windowed session with two devices.
- The HD podium for a split; per-person records and ghosts (a split writes none).
- HD's `LOYALTY`/`TOTAL` blocks draw empty on a split (as on any HD race with no
  team behind it).
