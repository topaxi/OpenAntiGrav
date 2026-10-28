---
status: measure
titles: [pulse]
updated: 2026-10-28
summary: "Re-measure the finish frame on the Deck; the three small saves at the finish and the other frame-thread IO sweep are still on the frame thread"
categories: [rendering]
size: S (60)
difficulty: easy (70)
---

# Render-thread IO after the finish-frame fix

`build_endrace`'s disc open, screen read and GPU build now run on a worker during the race
(`docs/rendering/frame-thread-cpu.md`, "The finish frame"). Open:

1. **Deck re-measure** (the maintainer's device; blocked on hardware): the same race with
   `OAG_PHASE_SPIKE_MS=11 --frame-log`, audio on, vsync off and `[display] frame_limit =
   "unlimited"`; read the first `frame-spike` line after `race finished`; expect `prep` under 2 ms.
2. **Saves on the frame thread** (maintainer rule, 2026-10-27: IO off the render thread): the
   result capture (`records::save` 0.12 ms), the ghost write (0.5 ms at a lap), and the loyalty
   saves in `build_endrace` are plain `std::fs::write` calls. A queue of writes on an `oag-thread`
   worker must flush on quit and on a menu leave, or a quit right after the finish loses the
   record. Measure the same writes on the Deck's storage first; they may not matter there.
3. **Sweep** the remaining synchronous disc reads and parses on the frame thread (the campaign and
   picker screens open archives on the page's thread). Candidate scout.
4. **Language switch mid-race**: the prefetch uses the language live at race start.

Measuring command (desktop): see the doc section above.
