---
status: ready-to-wire
titles: [pulse, hd, pure, 2048, omega]
updated: 2026-11-11
summary: "Feed craft-to-craft contacts into the original driver's lane push and rail drop (walls out of the push); halves our field's contact time"
size: S (70)
difficulty: easy (65)
---

# The original driver separates touching craft; ours does not

Measured on Pulse PSP: [ai.md](../../docs/gameplay/ai.md#the-fields-contact-time-against-the-originals) (numbers,
method, reproducing commands) and [ai-controls.md](../../docs/ghidra/functions/psp-pulse-usa/ai-controls.md#ai_planlane-0x08854e7c-which-lane)
(the push loop, live). Our opponents (the original driver, the default AI) spend about twice the original's
contact ticks and ride side by side for 1-6 s several times a race; the original's longest episode in eight VENOM
boots was 69 frames.

## Open

- **The wire (simulation change).** In `oag_raceplay::original_ai`:
  1. `contact_summary` keeps walls for the `touching` flag only (the original's push loop skips kind `0`,
     `0x08855238`); the wall push it feeds today is not the original's.
  2. `Race::resolve_craft_pairs` (`crates/raceplay/src/field.rs`) notes each resolved pair on both slots: side
     `+1` when `dot(left, contact.point - position) >= 0` (left is `-body.right()`), else `-1`; `touching = true`.
     `PairContact::point` is already the midpoint the original records. It runs after `step_opponents`, so add to
     the summary there rather than overwrite it; the next tick's planner and rail read it.
  3. How many counts per pair-tick: the original appends about 1.96 kind-`1` records per (tick, hull) and pushes
     `0.2` per record; the second record's origin is unread (`contact-response.md`, Result 5). One count and two
     counts gave similar fields (`ai.md`); take two only with that read, else one, and say which.
  4. The player's driver: unchanged (its planner zeroes the push, its rail ignores contact).
- **Expected effect, from the uncommitted experiment:** per opponent over 3 min on Talon's VENOM 287.8 -> 160-189
  contact ticks (original 151.3); de Konstruct White 242.3 -> 143-163 (original 120.7). Gate on
  `ai_contact_duration_board` and the determinism/race ground truths; regenerate any golden only in a separate
  commit naming this behaviour change.
- **Titles.** HD's planner is the same loop by lineage (`ps3-hdfury-eu/ai-controls.md`); check its kind test
  before wiring HD, 2048 and Omega inherit from HD; Pure's sibling driver (`psp-pure-usa/ai-controls.md`) needs
  its own read of the contact loop.
- **What is left after the wire:** ours keeps some 60-200 tick episodes the original does not show. Next reads:
  the original's second ring record per pair-tick, and the lateral spacing at contact (`scripts/psp-ai-contact-field.py`
  logs poses; ours needs the same per-tick distance).
- **Not measured on the original:** de Konstruct Black, a confirmed faster class, Ace (`g_skill_level` 2).

## Next Steps

1. Wire 1-4 above; run `OAG_SWEEP=1 OAG_SEEDS=6 OAG_CLASSES=venom,rapier OAG_TICKS=10800 cargo nextest run -p
   oag-raceplay --run-ignored all --no-capture -E 'test(ai_contact_duration_board)'` before and after.
2. Pin it: a short race test that two opponents' lanes part after a forced side-by-side contact.
