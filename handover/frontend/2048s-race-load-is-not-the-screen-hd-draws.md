---
status: measure
titles: [2048, omega]
updated: 2026-11-06
summary: "2048's race load is its kind 4 (event card or still, percent box), read and unbuilt; needs a Vita3K frame"
size: L (80)
difficulty: hard (70)
---

# 2048's race load is not the screen HD draws

The report that started this thread (HD's loading screen under the Fury style, and scaling) is closed: HD's style
pick, picture cut and style-matched ground landed (`docs/formats/hd-loading.md`), and Omega draws HD's screen
(`docs/formats/omega-loading.md`, its race load confirmed as state 8 by the constructor's callers).

## Open

2048 draws no loading screen of its own (`oag_2048::TITLE.loading` is `None`). Its executable carries HD's
`LoadingScreen` class, but the screen a race load starts is **kind 4**, not the HD feature screen (kind 7, started by no
call site). `docs/formats/2048-loading.md` and `docs/ghidra/functions/vita-2048-eu-v104/loading-screen.md` hold the
call-site census, the draw operands and the names; all nine pictures kind 4 names ship
(`crates/2048/tests/loading_ground_truth.rs`).

What a build needs:

1. The launched event's card as a backdrop. `CampaignEventCard_DrawPanel` (`0x81055a16`) is
   `oag_ui::frontend::Frontend::draw_event_card`'s body; `loading::Screen` has the front end's sprite sheet but not the
   event, so the launch (`crates/game/src/main/session/load.rs`) must hand it over and the card's panel half must be callable
   without a `Frontend`.
2. The mode test that picks the card or a controls still (vtable `+0x60`, unread) and our modes' place on the HD lineage.
3. The percentage box and its meter (positions in the evidence page), driven by the load stage; its ceiling is 57 % until
   the loader raises it.
4. An original frame, to settle all of it: no Vita3K profile existed on the lane's machine. `scripts/vita-2048-pair.py
   --walk` reaches the load; grab frames between the mode card's tap and the dark frame.

Omega's unwired state 5 is the same screen in shape; its constructor's callers pass 8 (race), 7 and one value not read
(`0x01252b86`), so what starts Omega's state 5 is open.

## Next Steps

1. Capture the original's race load on Vita3K (a campaign event, and an HD-lineage mode if the save reaches one).
2. Read vtable slot `+0x60` of the mode objects and `FUN_8105d4a8`.
3. Build kind 4 in `loading::Screen` and give `oag_2048::TITLE` a `Loading` for it.
