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

## Landed 2026-10-14 (`h2h-port-titles`)

- ~~Grid places~~: a front-anchored field (`Mode::field_is_fixed`, Head to Head) keeps the racer
  index as the grid row (`oag_raceplay::spawn::grid_row`): one person AI 1 / player 2, two people
  person 2 on 1 / person 1 on 2. Every other short field (Multiplayer Time Trial) still packs to
  the back, chosen, not measured; a full grid is unchanged.
- ~~Row names~~ and ~~top and bottom cut~~: rows `1st <team>` / `2nd <team>` in the `Default`
  font, own row and gap coloured, `[40, 85]` clamp when two views are cut `Horizontal`
  (`HeadToHead::stacked`); `strip-head-to-head` removed.
- ~~RACE page entry~~: Head to Head is the last of `Mode::ALL` on every title, WEAPONS pinned
  `Off`, AI DIFFICULTY and KILLS greyed; 3+ seated falls back to a Single Race
  (`session::menus::mode_for_people`).
- ~~Laps~~: checked, the RACE page path runs the class default (3 on Venom).

## Open

1. **Records** (item 6). PS2's `Race_RecordResult` writes each person's place into one
   profile's best-place record; this build has no such record. Decide whether a two-person race
   writes anything (modern saves: no per-person profile exists).
2. ~~Pure, HD, Omega, 2048 readout~~ landed 2026-10-16 (`h2h-readout-titles`): the place stands, the
   gap sits under it (`docs/rendering/split-screen.md`), chosen, not measured. Still open: whether
   HD's `Hud_BindWidgets` mode-9 branch (`0x0009e0ec`) sizes its authored `HeadToHeadBar` at
   runtime (the per-frame writer is unread), which would give HD a measured bar.
3. The left and right cut's leader row at `y = 49` (this build keeps the authored place) and the
   one-person player row's profile tag.

Still unread in the original: the opponent's team rule (three samples, three teams), the
finish of a two-person race watched live, and the bar's height at a gap inside the clamps.
