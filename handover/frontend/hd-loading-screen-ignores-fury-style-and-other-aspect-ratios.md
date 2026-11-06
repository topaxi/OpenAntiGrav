---
status: ready-to-wire
titles: [hd, 2048]
updated: 2026-11-06
summary: "HD style palette follows the served archive; 2048 draws HD's screen but its race load is kind 4, unbuilt"
size: M (70)
difficulty: medium (60)
---

# HD's loading screen ignores the Fury style and other aspect ratios

Two reports from play. Under the Fury front-end style the loading screen's feature images are the
HD ones, on white. And the images are not scaled into their place when the screen has another size
or ratio. Omega's front end is HD's, so check both there.

## Open

Landed: an empty `display.front_end_style` takes the style the menus are drawn in, a style changed in
OPTIONS reaches the next load, the globals and the mode icon follow the same choice, and HD's pictures are
cut to the panel (`docs/formats/hd-loading.md`). Omega draws its loading screen (`docs/formats/omega-loading.md`).
2048's `Loading` table is wired (`oag_2048::loading::LOADING`, `LoadingScreen_2048.toml`, all sixteen `.gxt`
names resolve, `docs/formats/2048-loading.md`).

- **The white ground over the name row is the palette, not the draw order.** Read 2026-11-06: the picture is
  drawn before the name and heading rows in 2048's `LoadingScreen_Draw` and in ours, and the rows do draw over it.
  The HD styling's white pictures meet a black ground because the four globals are read from the served
  archive's `Skin.xml` (dark `DATA00`), not from the chosen style's copy. Fix: read the globals from the copy
  of `Skin.xml` whose ground matches the chosen style (HD's `DATA06` is white), the way the front-end colour template
  does. Check with `--loading-screen` on the HD disc and `front_end_style = "HD"`.
- **2048's race load is kind 4, which is not built.** The retail executable starts its kind 7 (HD's feature screen,
  the one wired) from no call site; every race start starts kind 4: the mode's event card (or a controls still) behind
  a `Blue2048`/`White2048` percentage box, `FE_LOADINGDOT` and `%d%%`
  (`docs/ghidra/functions/vita-2048-eu-v104/loading-screen.md`, "Kind 4"). Building it needs the event card drawn
  as a backdrop and a layout kind for the box; Omega's unwired "state 5" is the same screen. No Vita3K profile existed
  on the lane's machine, so no original frame was compared; one would also settle the ground colour of the wired screen
  (chosen: white, `HD_White`).
- 2048's bar ceiling is 94.62 at 0.115 a frame (HD: 149.4 at 0.1); how it maps onto the bar is unread.

## Next Steps

1. HD: take the four globals from the chosen style's `Skin.xml` copy, then re-shoot HD style on `data/images/hdfury-ps3-eu-dec.iso`.
2. 2048: capture a Vita3K race load (`scripts/vita-2048-pair.py --walk`, grab frames while the bar is up) and build kind 4 from it.
