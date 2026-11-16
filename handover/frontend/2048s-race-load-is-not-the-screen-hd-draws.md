---
status: ready-to-wire
titles: [2048]
updated: 2026-11-16
summary: "2048's race load (kind 4) is read and frame-confirmed: card, game tip, mode page, percent meter law; build it"
size: L (75)
difficulty: medium (65)
---

# 2048's race load is not the screen HD draws

HD's and Omega's loading screens are done (`docs/formats/hd-loading.md`, `docs/formats/omega-loading.md`). 2048 draws
none of its own (`oag_2048::TITLE.loading` is `None`). What the original draws is now fully read and confirmed against
three Vita3K captures: `docs/ghidra/functions/vita-2048-eu-v104/race-load.md` (operands, tables, the meter's law with
live numbers) and `docs/formats/2048-loading.md` (the summary). Omega is closed: its state 5 has no caller
(`docs/ghidra/functions/ps4-omega-eu/loading-screen-start.md`), so this is a 2048-only build.

## Open

Build kind 4. The recipe, in the order a player notices it:

1. **`oag_2048::TITLE.loading = Some(..)` with a 2048 screen kind.** `loading::Screen` today lays out HD's feature
   screen; 2048 needs a second layout: no caption, no feature, no rules. In the 960 by 544 grid: `Blue2048` box 70 by
   70 at (16, 16); `%d%%` (`White2048`, scale 0.6, centred on (51, 38)) of `floor(shown / 166 * 100)`;
   `FE_LOADINGDOT` (`default` face, scale 1.0, `Blue2048`) at (105, 30); a 338 by 4 `Blue2048` rule at (86, 82) that
   does not grow. Colours come from `Skin.xml`, as the event card's do.
2. **The meter** (`FrontEnd_DrawLoadingMeter`): seven triangles, centres `x = 895 - 30 i`, `y = 475` (odd `i`: rotated
   60 degrees, 14 lower), outer radius `28 u`, inner `14 u p - 1`, `p = clamp((1 - fraction) * 7 - i, 0, 1)`, pulse `u`
   from a 64-draw counter offset 7 per triangle (ramp 10 draws in, 10 out). New `Draw` geometry: three quads a triangle.
3. **The law**: `oag_title::loading::Progression` fits with `start: 94.62 / 166`, `milestones: &[0.75, 0.95, 0.98]`,
   `rate: 0.115`, `columns: 166.0`, **but two rules differ from HD's `Fill`** and need a per-title switch (`Title`
   data, not a title branch): a raised target multiplies the rate by `(old_gap + new_gap) / new_gap` (HD doubles it),
   and after the last milestone the rate is frozen as its own reset value (HD resets to `law.rate`). The step is one
   per 1/30 s, so two of our 60 Hz frames. The end of the load sets 100 %. Which of our load stages stands for which
   milestone is ours to choose (the original's loader steps behind the three calls are unread; addresses on the page).
4. **The backdrop is the launched event's card.** `Frontend::draw_event_card`
   (`crates/ui/src/frontend/event_card.rs`) is the original's painter; the load needs it callable with the event and
   its page state but without a live `Frontend` (the launch in `crates/game/src/main/session/load.rs` hands over the
   `MapEvent`, the page list and the page). The original keeps drawing the blurred campaign map behind it; ours may
   draw the card over the menu's last frame or its ground.
5. **Game tip, then the mode page.** First phase: the card where it was, with a tip panel in the photo's place
   (398 by 334 at (16, 92); the 37-row table, the pick law and the panel's operands are on the page; the art is
   `data/FE/NewImages/tips/*.gxt`, 36 files, all shipped; the strings are `TUT_TIP_HEADER`, `Tut_*`, `FE_*`). Second
   phase, from our "race built" stage: add `Page::Mode` (kind 6: the mode's name, class icon and `Tut_Event_*_1`
   body), turn to it, drop tip and photo, and slide the card `-(1 - (1 - s)^2) * 202` with `s += 0.08` a draw.
   The tip table goes in as data read from the executable's own strings (a `oag_2048` table with the evidence page
   cited), the texts from the string table; nothing is retyped.
6. **Modes without a campaign event** (racebox, the HD and Fury cells): the original draws the cell's own panel
   (`CampaignEventCard_DrawCellPanel`, `docs/ghidra/functions/vita-2048-eu-v104/circuit-name-key.md`) through the same
   route. Our racebox launches have no such card; draw the box and meter over the ground and say so in the loader
   report, never a made-up card.
7. **The attract demo** draws a still instead of a card (`Title_Screen.gxt`, or a controls still on an HD-lineage mode
   word, at (-32, 144), 1024 by 256). Ships; wire it with the demo's load.

Unread, none blocking (addresses and read recipes under "Not read" on the evidence page): the divisor `mode+0x188` of
the race manager's rate (probably the craft count; `8` on an eight-craft race keeps the rate), the controls-diagram tip
variant (`0x8106d3ce`, the first two loads of a profile), which loader step each milestone is, and a real Vita's draw
rate.

## Next Steps

1. Box, caption, readout, rule and meter with the law (items 1 to 3) over a plain ground: a player-visible screen in
   one lane, tested against the page's numbers (the three rescaled rates, the 20 % and 98 % triangle states).
2. The card backdrop with tip and mode page (items 4 and 5), pointer support unchanged (the load takes no input).
3. Compare against a fresh capture: `scripts/vita-2048-race-load.py` (about a minute; the docstring has the command).
