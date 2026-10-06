# `EndRace Podium`: the multiplayer race managers' end screen

2026-10-06, lane `hd-endrace-podium`. Static reading only (Ghidra bridge,
`/hdfury/EBOOT-ps3-hdfury-eu.elf`); no live RPCS3 run backs any row, so
nothing scores above 84. Every `lwz rX,disp(r2)` below was resolved with
`scripts/ps3-toc.py resolve <fn> <disp>` against these functions' own TOC
(`0x008ad4d8`, exact): Ghidra's `PTR_` names and its TOC xrefs are unreliable
in this cluster (its xref list for the screen name pointed at unrelated
audio code). **No names are recovered into `names.tsv`**: the three functions
below keep their `FUN_` names, since several fields they touch are unnamed.

## What the screen is

`EndRacePodium_Screen.cpp` (`0x00793ec0`) registers the type `EndRace Podium`
(`0x00793ee0`) through `FUN_0021f110`. Widgets: `pod_head.N`/`pod_text.N`/
`pod_img.N`/`pod_dots.N` (`0x00793f40`..`0x00793f80`), `~podiumticker`/
`podiumtickend`. It lives in `DATA05`/`DATA06` only and authors no
`NavigationController` and no `Redirect` except the network exception handler.

## Who enters it: the multiplayer race managers (confidence 74)

`scripts/ps3-toc.py attrib 0x0077b770` (the name string) names exactly one
function, **`FUN_000459d8`**, loading it at `0x00045e4c` (`lwz r4,-0x7144(r2)`)
and passing it to the goto helper `0x00058ab0` the next instruction. Its
object has `MPRaceManager_Construct`'s own field `+0x34ec`
(`race-manager.md`: the base class of the five multiplayer race managers), and
the string pool around the name holds `ONL_IGM_RTL_COUNTDOWN`,
`ER_RACE_COM2`, `Load Next Race` and `Kill Game Transition`. Reading
`0x00045a48`..`0x00045eac`:

- `+0x34ec` is a millisecond deadline; zero draws nothing. `FUN_00011d68`
  (`g_GameState+0xe0 == 0x11` and a counter `+0x14 < +0x10 - 1`, i.e. a
  mode-`0x11` series not on its last event; also gated by a byte at
  `DAT_008a5644`, unread) is the "mid-series" flag.
- Once the deadline has passed and the screen timer `+0x34f0` is at its
  initial `0.0`, and the series flag is **false**: `FUN_00051268` runs (it
  tallies the 50-slot award/badge counters over the records), the screen goes
  to **`EndRace Podium`**, and `+0x34f0` becomes now + `16.0` s (`-0x7128`).
- When that timer lapses: not mid-series -> `Kill Game Transition` (back to
  the front end); mid-series -> `Load Next Race`.

So the screen is the end of a **multiplayer** race (the whole family descends
from `MPRaceManager`), held 16 s with the three places on a podium, then back
out. This project has no multiplayer, so the live single-player flow stays
Results -> Menu; the screen is drawn by `--menu-page endrace-podium` only.
Not read: what starts `+0x34ec` (seven stores, in `0x000434b8`/`0x00043b60`/
`0x000442e8`/`0x00044780`/`0x00048728` and the constructor), and whether any
single-player manager reaches the same screen by another route (none found).

## The slot setter, `FUN_00220108` (78)

`setter(screen, slot, name_string, place, ship, is_local_player)`; `place 0`
hides `pod_head.<slot>`. For a place (`rise = 80 (4 - place)`, `col = 360 slot`):

| Widget (TOC slot) | Placed at |
| --- | --- |
| `pod_text` (`+0x2f44`) | `x = 70 + col`, `y = 532 - rise` |
| `pod_head` (`+0x2f40`) | `x = 120 + col`, `y = 485 - rise`; text `IG_HUD_1ST`/`2ND`/`3RD` (`+0x2f6c`/`70`/`74`, resolved) |
| `pod_img` (`+0x2f48`) | `x = 55 + col`, `y = 480 - rise`; a ship texture per record |
| `pod_dots` (`+0x2f50`) | `x = 60 + col`, `y = 565 - rise`, width `352.0`, height `rise`, sampled rectangle the same two numbers |

Colour: the local player's name and plinth are the screen's `HD_Blue` slot
(`+0x228`), anyone else `0xff646464`. The text is the record's string at
`+0x50`, the field `endrace-results-grid.md` leaves unidentified for
`Grid1.r`.

## The caller, `FUN_00220820` (76)

Fills three slots from the race manager (`RaceManager_GetInstance`, records
from `+0x1980`, stride `300`): first place to **slot 2**, second to slot 1,
third to slot 3, so the winner stands in the middle (slot 2's name `x` is
`790`, beside the authored `pod_head.2` `x="795"`). Two branches on
`g_GameState+0xe0 == 0x11` (and the `DAT_008b0458` byte, unread): in that mode
the places come from a rank pass over a second record array (from `+0x960`,
eight records, ties sharing a place); otherwise they are literally `1`, `2`,
`3` over the first three records at `+0x50`. Third place is hidden when the
field holds two or fewer craft. Eight `FUN_0021f9e8` calls then fill the
`b_b.N` panels from a badge list (`FUN_0004d7f8`, `BADGE_NOT_AWARDED`).

## What this project does with it

[`oag_ui_screens::endrace::hd::hd_podium_draw_list`](../../../../crates/ui-screens/src/endrace/hd/podium.rs)
draws the three places at the positions above, plinths included
(`dot.gtf`, brought by `oag_hd::endrace::EXTRA_TEXTURES`). Chosen, not
measured: only `HD_Blue`'s value and the title (not in the served string
table). See
[hd-endrace-screens.md](../../../formats/hd-endrace-screens.md#endrace-podium-read-and-drawn-multiplayer-only).
