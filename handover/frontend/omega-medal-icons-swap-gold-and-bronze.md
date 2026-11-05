---
status: ready-to-wire
titles: [omega]
updated: 2026-11-05
summary: "Omega draws gold and bronze medal icons swapped: HD's Hexmedal crop on Omega's differently sized atlas, cause unread"
size: S (60)
difficulty: easy (50)
---

# Omega's gold and bronze medal icons are swapped

Seen in play: gold and bronze change places on Omega; silver is not reported as wrong.
`docs/formats/omega-status.md` already records the three icons in the wrong order: `Hexmedal_HD` is
976x549 on Omega against HD's 1024x768, and the crop is HD's.

## Open

- `hd_medal_frame` (`crates/ui-screens/src/campaign/hd/medal.rs`) hard-codes HD's layout: tier `v` 0,
  61, 122 for gold, silver, bronze, block pitch 183, frame 60. It has no title axis.
- Cause unread. The swap of the two ends with silver fixed fits two readings: a different row order
  in Omega's file, or a vertical flip. `medal.rs` records that HD's `.gtf` raster reads upside down
  against its authored `v`, and Omega's `.gnf` goes through another decoder. A flip of the whole
  atlas would also swap the difficulty blocks (plain hex against swirl), so check those too.
- Omega's own `CellMode_Definition.xml` may author `Target0/1/2 Medal` crops as HD's does. Read them
  before measuring anything off the texture; 976x549 may also mean the 60 px frame and 183 pitch do
  not hold.
- Find every draw of the atlas: the `Target0/1/2 Medal` widgets, the hex `Medal_x_y` badge, and any
  end-of-race use. Check 2048 against the finding, per CLAUDE.md.

## Next Steps

1. Read Omega's authored crops and decode its `Hexmedal_HD.gnf`; state which reading holds.
2. Make the crop `Title` data (no title branching) filled from Omega's own numbers; HD stays as is.
3. Pair `omega/menu-cell-select`: icons match the original's order at every difficulty rung a frame
   shows. Correct `docs/formats/omega-status.md`, then delete this thread.
