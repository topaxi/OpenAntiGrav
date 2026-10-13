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
| Pulse PS2, `Data\Plugins\PI001\GUI\EndRace_Definition.xml` in `WADS2.WAD` | **Authors `EndRace Results Split` and `EndRace Rewards Split`** (the PS2 executable names both, beside `Split ScreenV`/`Split ScreenH` and `Kill Game Multiplayer`). Results Split is two per-player lap tables stacked in the 640x448 space: `<Item name="Player_0" OffsetY="-96">` and `Player_1` at `OffsetY="110"`, each its own `Line1_N` headline, `table_N`, eight `tablebg{r}_N` rows at `OffsetY` 152 + 25 (r - 1), `lap{r}.{c}_N` cells, `perfectlap`, `boostimg` and `tablehighlight` copies. Its `EndRaceRewardsRedirect` goes to `EndRace Rewards Split` (`PlayerRW_0` at -52 and `PlayerRW_1` at 176, a trophy, medal, loyalty row and bar each), then `Race End Save`, then the ordinary `EndRace Menu`, whose `IG_PAUSE_QUIT` row goes to `Kill Game Multiplayer`. | **Not yet**: found and read, left open (below). Two people only. |
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
- **No loyalty, no medal, no record, no ghost and no announcer for a split.**
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

## Open

- **The PS2's `EndRace Results Split` and `Rewards Split`** for two people on a PS2
  source: per-player lap tables and per-player rewards, the authored flow. Needs
  `lap{r}.{c}_{N}` cells and `<Item>` offsets read by `table_layers` (its row bands
  are the single table's), a per-player headline `Line1_N`, and the entry law.
  **Decided 2026-10-13 (maintainer)**: the PSP pressing borrows these two screens,
  built from its own assets and labelled "borrowed from the PS2 pressing", so
  Pulse ends a two-person split the same way on both pressings. The "no loyalty for
  a split" rule above is provisional until Rewards Split lands.
- **On a PS2 source every `EndRace Results` table's highlight is misplaced**, split or
  not: `table::highlight_y`, `row_of` and `with_row_offset` carry the PSP file's row
  geometry (first row 92, pitch 20) and the PS2 file authors its own (`tablebg1` at
  152, pitch 32.94 in 640x448). Seen on `--menu-page endrace-results-eliminator` (the
  eighth-placed row is highlighted on row 4) and on `endrace-results-people`
  (persons 5th and 7th highlighted on rows 2 and 3). The names and times are right.
  Found, not fixed: it is the same geometry the PS2 Split screens need.
- A live walk with two real pads: the end screens were checked through stills of
  the real draw functions and a headless race driven to its end, not through a
  windowed session with two devices.
- The HD podium for a split; a per-person `EndRace Rewards`; per-person records and
  ghosts (a split writes none).
- HD's `LOYALTY`/`TOTAL` blocks draw empty on a split (as on any HD race with no
  team behind it).
