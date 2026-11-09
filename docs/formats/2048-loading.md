# Wipeout 2048's loading screen

**2048 ships HD's loading-screen class, and the retail executable starts a different kind of it for a race load.**
Confidence 80 (a call-site census and a decompile, no running original: the machine has no Vita3K profile). The
evidence is [vita-2048-eu-v104/loading-screen.md](../ghidra/functions/vita-2048-eu-v104/loading-screen.md).
`oag_2048::TITLE.loading` is `None`, on purpose: the screen a Vita player sees is not built, and the one that is
authored in full is never started.

## Two screens in one class

| kind | what it is | reached |
| ---: | --- | --- |
| **4** | the race load: the mode's event card (or a controls still), a `Blue2048`/`White2048` percentage box, `FE_LOADINGDOT` and a `%d%%` readout | by every race start (`FUN_8115f09a`, `FUN_811494fa`) |
| **7** | HD's screen: caption naming the circuit, one of five illustrated features, a dotted progression bar | **by no call site** of the v1.04 executable |

Every call of the starter `LoadingScreen_Start` (`0x812845ce`) was censused: eight calls, with `0` (presents), `4` (race
start, three sites), `6` (boot and shutdown, three sites) and `8` (the epilepsy still). The argument is a literal at each.

**Why nothing is wired.** Kind 7 was wired in this lane (the layout is HD's operand for operand, minus the mode icon,
the ten illustrations and six marks resolve, a white ground) and taken out again: it is a screen no 2048 player has
seen, and a stand-in for the one they have. The evidence page keeps its operands. Kind 4 needs, and a lane that builds it
should start from:

1. The event card as a backdrop. The card's painter (`CampaignEventCard_DrawPanel`, `0x81055a16`, read in
   `campaign-event-card.md`) is `oag_ui::frontend::Frontend::draw_event_card`, which lives in the front end's state and needs
   the launched event's `EventCard`; `loading::Screen` has the front end's sprite sheet and none of that state, so the launch
   must hand the event over.
2. The mode test that picks the card or the still (vtable slot `+0x60`, unread) and which of our modes are on the HD
   lineage; the controls still sits at (-32, 144), 1024 by 256.
3. The percentage box: `Blue2048` (`0xff19295d`) and `White2048` (`0xffe1e4eb`), a 70 by 70 fill at (16, 16), a 338 by 4
   bar at (86, 82), `FE_LOADINGDOT` at (105, 30), `%d%%` at (51, 38) at scale 0.6, a meter at (790, 475). Its fraction
   is the bar's `+0x88 / 166`, ceiling `94.62 / 166` = 57 % until the loader raises the target (`FUN_812845b8`), which is the
   56 % a stalled Vita3K load sat at ([vita3k-capture.md](../reverse-engineering/vita3k-capture.md)).
4. A frame of the original. **None exists**: this machine has no Vita3K profile (no `~/.config/Vita3K`). The recipe is
   `scripts/vita-2048-pair.py --walk` (`docs/tools/compare-page.md`, "2048 (Vita) pairs"): its walk waits "out the load on the loading bar" until a
   dark frame, so grab frames every second between the mode card's tap and that frame.

## What ships (asserted)

All nine pictures kind 4 names resolve, and all sixteen kind 7 names (six marks, **both arrows included**, and ten
illustrations): `oag-2048` `loading_ground_truth::every_picture_the_race_load_names_is_in_the_package` and
`the_art_of_the_dormant_feature_screen_ships_whole`. The Omega page's "the two arrows were not listed" and the earlier
"four of six marks" were a census gap. The illustrations are 1024 by 512. The loader picks `_fury` on a byte that is
`0` in the image with no writer (confidence 70), so the base styling is the only one kind 7 reaches.

## Kind 7's operands, for the record

HD's, compared in `LoadingScreen_Draw` (`0x812830c8`): rules (160, 110) and (160, 975), 1600 by 8; caption arrow (160, 73);
caption in `Title` at (194, 62); labels at (160, 48), (160, 125), (960, 125), (160, 670), (1510, 670); four bracket sets; the
picture 896 by 480 at (180, 190) with uv 0.875 by 0.9375; name and heading rows at (175, 175) and (975, 175); the paragraph at
(975, 215), wrap 780, pitch 40; the brand is HD's literal `WIPEOUT(R) HD` (`0x814d0e24`); the mode icon's panel is a label and
brackets with nothing inside. The deck is HD's row for row (mode `6`: two, `0xe`: one, `0xd` and `0x15`: `0 1 3`, `8` and
`0x14`: five, else `rand() % 4`). 2048 reads `FE_HD_BG` and draws it nowhere in kind 7; the picture's ground is white, so the
ground that makes a base picture's overhang invisible is white. The kind draws the picture first and the rows over it.

## Against Omega and HD

- **Omega: checked, differs, and the check confirms Omega's wiring.** Omega's constructor `FUN_01708fe0` is called with
  `EDI = 8` by `ModeManager_ConstructByMode` (`0x0124fe44`) and `FUN_01277470` (`0x01277752`) and with `7` by `Game_Main`
  (`0x0163e71d`): the race start of Omega **is** the HD feature screen (its state 8), so `omega-loading.md`'s reading, which
  had been taken from the thread body alone, holds. 2048's race start is its kind 4, which is Omega's unwired
  "state 5" in shape (backdrop, `FE_LOADINGDOT`, a `%d%%` readout in a `Blue2048` box), so state 5 is **a different screen
  on Omega** (not its race load) and the same screen on 2048. Omega's state 5 was not wired here.
- **HD: ported.** Nothing changes in HD's loader.

## Looking at it

Nothing to look at: `oag-game data/extracted/vita/PCSF00007 --loading-screen ...` draws this build's generic counts screen.
