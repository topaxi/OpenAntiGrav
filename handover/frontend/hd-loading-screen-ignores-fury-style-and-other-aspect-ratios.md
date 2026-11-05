---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-11-05
summary: "HD's style and fit landed; Omega draws no loading screen yet"
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
- Omega draws no loading screen (step 5 below).

## Next Steps

5. Follow-up: Omega draws no loading screen, reported from play, and its
   layout is expected to be close to HD's. Read what Omega's own disc ships before reusing HD's file,
   check it against 2048 per CLAUDE.md, wire it as `Title` data, then delete this thread.
