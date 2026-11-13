---
status: ready-to-wire
titles: [omega, 2048]
updated: 2026-11-13
summary: "Omega authors the 2048 map pad focus in code (cones, repeat, scroll-to-focus, pulse, ripple); port to the shared map, both titles"
size: M (40)
difficulty: medium (45)
---

# The 2048 map takes Omega's pad focus law

Recovered from Omega's `eboot.bin` (static, confidence 72 to 84):
`docs/ghidra/functions/ps4-omega-eu/campaign-map-pad.md`. Maintainer rule: one shared map
(`oag_ui::frontend`), Omega's behaviour replaces our chosen one on **both** Omega and 2048.

## Open

- Port to `crates/ui/src/frontend/campaign_map.rs` (`nearest_event`, the d-pad handler):
  1. **Pick**: on raw canvas tile indices (`col = 55 + 4 M_X + tweak`, `row = 25 + 2 M_Y +
     tweak`; do **not** scale by `CELL_DX`/`CELL_DY`, the cones would change). Centre = corner
     + the per-type offset in the page's table (3 x 2 default: +1.5, +1; 7 x 4 types: corner
     col - 1, +2.5, +2). `a = atan2(dcol, drow)` in degrees, wrapped to [0, 360): right
     25..155, up 115..245 (smaller row), left 205..335, down 295..65. Nearest by plain
     Euclidean distance; none: stay. Replaces our `along + 2 * |across|` score. Check one
     frame that a smaller row is up on screen in our projection.
  1a. **Season filter, maintainer decision**: Omega shows one season per visit (picked on
     `Grid Selection 2048`) and the pick never leaves it. Our shared map draws all three
     seasons and 2048 has no season chooser, so porting the filter as is would trap the pad in
     one season. Options: drop the filter, or give the map a current season.
  2. **Input**: d-pad only (the left stick does not move focus on Omega); auto-repeat after 0.4 s
     held, then every 0.12 s; directions polled left, right, up, down, first press wins.
  3. **On focus change**: select the event (our card's selected event), play `newGUI_select` and
     `newGUI_trans03`, and scroll the view so the event is centred (smooth, speed 5.0 in the
     `TouchScroll`'s own units; the easing law of `TouchScroll` is unread, chosen until read).
  4. **Look**: every reachable event is a grey of level `b` (low byte of `DAT_01aeb688`,
     unread) and the focused one is `b + sin(5 t) * 32.5 + 32.5` (period 1.257 s), and a 2.5 s
     ripple on the lattice: per tile at distance squared `d2` within
     radius `r` (= half the button's size field: 3.5 by default, in canvas units; the tile pitch `DAT_01aeb850`/`DAT_01aeb854` that converts it to cells is unread), alpha `(sin(-5.5 t - pi +
     0.15 d2) + 1) * 128 * min(1, 4t) * (2.5 - t) / 2.5 * (r^2 - d2) / r^2`. Our lattice wave
     (`canvas_wave.rs`) is the likely host. This replaces the `hex_select` `Orange2048` marker
     as the focus look (decide with the maintainer whether the marker stays for pointer hover).
  5. **Start focus**: the season's remembered event, else the last raced event walked forward
     past completed ones, else the season's `Event 1` walked the same way.
  6. **Buttons**: Triangle opens the event's records (Omega); Circle back to the chooser; the
     card's left rect is Change craft (`team`, refused on a forced-craft event). Our
     right-stick arrow cursor has no Omega counterpart: keep it labelled ours or drop it
     (maintainer's call).
- Unread, with recipes on the page: Cross's launch path (the card's rects are tested only
  against touch lists with no writer found; next `0x013c5e70` and `pad+0x8e8`'s writer), the
  pulse's base colour `DAT_01aeb688`, the zoom limits, the `DifficultyButton` handler.
- Tests that fail if dropped: a pick test on a hand-made lattice where the cone law and our old
  score disagree (an event 60 degrees off-axis nearer than an on-axis one), a repeat-timing test,
  and a scroll-follows-focus test.

## Next Steps

1. Read `campaign-map-pad.md`, then `campaign_map.rs::nearest_event` and its callers (30 min).
2. Replace the pick and add the repeat with the three tests above (2 h).
3. Scroll-to-focus, pulse and ripple, judged on frames at player size (half a day).
