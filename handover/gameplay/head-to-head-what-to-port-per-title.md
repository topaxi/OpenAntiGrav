# Head to Head: what to port, per title, against what the originals do

2026-10-14, `h2h-re`, for the queued `h2h-port-titles` lane. The RE is done; everything
below is either measured (address and capture on the evidence pages) or labelled ours.
Evidence: [`docs/ghidra/functions/ps2-pulse-eu/head2head.md`](../../docs/ghidra/functions/ps2-pulse-eu/head2head.md)
(the two-person mode, PCSX2 live), [`docs/ghidra/functions/psp-pulse-usa/head2head.md`](../../docs/ghidra/functions/psp-pulse-usa/head2head.md)
(single player, the grid places measured on PPSSPP), [`docs/ghidra/functions/ps3-hdfury-eu/head2head.md`](../../docs/ghidra/functions/ps3-hdfury-eu/head2head.md)
(HD, and the Pure / 2048 / Omega census).

## Which originals have it

| Title | One person | Two people |
| --- | --- | --- |
| Pulse PSP | Racebox `Head to Head` and 23 campaign cells | none (mode `0xf` is the ad-hoc network race, not ported) |
| Pulse PS2 | the same | **yes**: SPLIT SCREEN VERTICAL / HORIZONTAL -> RACEBOX `RACE TYPE` `Head to Head` (mode 9, the split flag) |
| Pure | none (no mode, string or race manager) | none |
| HD/Fury | in the executable, offered nowhere (`<aEntry>` in every menu, no cell) | none |
| 2048 | inherited name only, offered nowhere | none |
| Omega | HD's case | none |

So only Pulse has anything to measure against. Everything on Pure, HD, 2048 and Omega is
this project's own extension: label it **chosen, not measured**, and mirror the closest
relative (Pure from Pulse, Omega from HD, 2048 from HD via Omega).

## Open: differences between this build and the original (Pulse)

1. **Grid places.** The original starts a Head to Head on the **two front places** of the
   eight-place grid: one person, AI on place 1 and the player on place 2 (PSP, measured);
   two people, person 2 on place 1 and person 1 on place 2 (PS2, measured, the slot is the
   racer index with no compaction). This build puts both on the back two
   (`crates/raceplay/src/start.rs`, the `pose_start` / `rank_from_back` block, whose own
   comment calls it extrapolated and cites a note now struck). The general law, from PS2:
   **the absolute slot is the racer index in the eight-place layout**, with people at the
   highest indices; that gives the back rows for a full grid and the front two for a field
   of two, so no Head to Head special case is needed. Pinned today to the back rows, to
   change on purpose: `crates/game/tests/window_seats_grid_ground_truth.rs`'s
   `head_to_head_across_two_windows_seats_both_people_at_the_back` (`assert_eq!(rows, [8, 7])`).
   Changes the one-person hash; regenerate in a separate commit that names it.
2. **Row names.** In a split each row is `1st <team>` / `2nd <team>` with the craft's team
   name (`Qirex`, `Assegai`), the same two rows in both views; only the view's own
   person's row and the gap are coloured (green leading, red trailing). One person: the
   opponent's team and the profile tag (`AAA`). This build draws ordinals only, and the
   `Default` font is still missing (`head2head.md`, "Not drawn").
3. **Top and bottom cut.** `Split ScreenH` clamps the half gap to `[40, 85]` (not
   `[60, 180]`) and moves the leader row onto the bar's top (`FUN_001371c8`). This build's
   `strip-head-to-head` rule scales the place group to 0.8 instead. The left and right cut
   keeps `[60, 180]` and the leader row at `y = 49`.
4. **Where the mode is chosen.** Pulse's RACE page (both pressings) has no Head to Head
   entry (`Mode::ALL` omits it), but the PSP Racebox authors one (`race-setup.md`) and the
   PS2 split screens' list is `Arcade`, `Tournament`, `Head2Head`. Add it, greying WEAPONS
   to `OFF` and AI DIFFICULTY as the original does (seen live on PS2), with the two-person
   cap at join/START (`Mode::max_people`).
5. **Laps.** A Racebox Head to Head ran 3 laps on Venom (PS2 split, PSP one person), the
   class default; the 4/5 laps belong to campaign cells. Check this build's RACE page path
   uses the class default.
6. **Records.** PS2's `Race_RecordResult` (`0x001054c8`) writes each person's place into
   the one profile's best-place record. Decide whether a two-person race writes records
   here (modern saves: progress persists automatically).

Same as the original, nothing to do: two people and no AI; weapons locked off; the race
ends when both people have finished; the gap is `|d1 - d2|` in metres; the end board
has two rows with places 1 and 2 (PS2 shows `EndRace Results Split`).

## Next Steps

1. Grid places (item 1), with a test that fails on the back two.
2. The Pulse RACE page entry and its greying (item 4), pointer support included.
3. Team names on the rows and the `[40, 85]` clamp in the top and bottom cut (items 2, 3).
4. Pure: our extension mirroring Pulse. Pure's HUD authors no `HeadToHeadBar`, so the
   readout needs a fallback layout (labelled ours) or a place-only HUD.
5. HD, Omega, 2048: our extension. Their HUD files (`hud_positions.xml`,
   `splitscreen_hud/splitscreen_positions_screen.xml`, 2048's `HUD_positions.xml`) do author
   `Position`, `Position Outof` and `HeadToHeadBar`, but no `PositionOf` gap label was seen
   (only the names were grepped); read the widget set before building the readout. The
   strings (`Head2Head` = "HEAD TO HEAD", `MSC_EVENT_HTH`) and the `head2head*.gtf` icons
   ship on HD and Omega. Their RACE pages need the entry as our own row, since the disc's
   is a switched-off `<aEntry>`.

Still unread in the original: the opponent's team rule (three samples, three teams), the
finish of a two-person race watched live, and the bar's height at a gap inside the clamps.
