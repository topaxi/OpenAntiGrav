---
status: ready-to-wire
titles: [hd, 2048]
updated: 2026-11-05
summary: "HD's style and fit landed, and Omega draws its loading screen; HD's white ground over the name row and 2048's loading table are open"
size: S (60)
difficulty: easy (50)
---

# HD's loading screen ignores the Fury style and other aspect ratios

Two reports from play. Under the Fury front-end style the loading screen's feature images are the
HD ones, on white. And the images are not scaled into their place when the screen has another size
or ratio. Omega's front end is HD's, so check both there.

## Open

Landed: an empty `display.front_end_style` takes the style the menus are drawn in (the first reading
proved: it took the first entry, HD's white art, while the menus follow the served archive's ground), and a
style changed in OPTIONS reaches the next load (`apply_setting` re-reads the assets, so the second reading
was fine). The globals and the mode icon follow the same choice. The scale report was HD's opaque white ground
overhanging its panel at every screen size, not a ratio problem: Fury's pictures sat inside their panel on
1280x800, 2400x1080 and 2560x1080 already. HD's pictures are cut to the panel (chosen, not measured). See
`docs/formats/hd-loading.md`.

- The white ground of the HD styling also covers the lower half of the feature's name row. Draw order or the
  art's own placement; the original's HD loading screen is unread.
- 2048 draws no loading screen: its executable carries HD's string run and `data.psarc` ships the ten
  illustrations and four of the six marks as `.gxt` (`docs/formats/omega-loading.md`, "Against 2048 and HD"). `oag_2048::TITLE.loading` is
  `None`; wiring is a `Loading` table of `.gxt` names and a layout, read against 2048's own draw function.

## Next Steps

1. Read how HD's `LoadingScreen_Draw` orders the name row against the white ground (the original's HD frame).
2. Wire 2048's `Loading` table once `2048-focus`'s front end lands, off `docs/formats/omega-loading.md`.
