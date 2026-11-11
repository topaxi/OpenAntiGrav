---
status: re-first
titles: [pulse, hd, pure, 2048, omega]
updated: 2026-11-11
summary: "Pair contacts now push the original driver's lane and drop its rail (wired, field contact time halved); left: the second ring record per pair-tick, ours' remaining long episodes, Omega's kind test"
size: S (60)
difficulty: medium (50)
---

# The original driver separates touching craft; what is left after the wire

Measured on Pulse PSP and wired: [ai.md](../../docs/gameplay/ai.md#the-fields-contact-time-against-the-originals)
(the two rules, the before/after board, other titles' reads). `oag_raceplay::original_ai` feeds each resolved
craft pair into the planner's push (side by the midpoint, one count per pair-tick) and the rail drop; walls drop
the rail only. Pinned by `ai_pair_contact_push_ground_truth`.

## Open

- ~~The wire (simulation change).~~ Landed: contact summary, `resolve_craft_pairs` noting both slots, one count
  per pair-tick, the player's driver unchanged. Board on Pulse, VENOM, seeds 1-6, 3 min per opponent: Talon's
  287.8 -> 188.6 (original 151.3), de Konstruct White 242.3 -> 142.8 (original 120.7).
- **The second ring record per pair-tick.** The original appends about 1.96 kind-`1` records per (tick, hull);
  what makes the second is unread (`contact-response.md`, Result 5). Read it (recipe there: count `0x0884f630`
  hits for one pair between two `Ship_UpdateCraft` stops); if it is a second resolver pass or manifold point,
  take two counts. The uncommitted two-count run gave Talon's 160.8 and de Konstruct White 162.9 (3 min).
- **Ours keeps some 60-160 tick episodes** the original does not show (VENOM, 0-8 per race), and Talon's
  RAPIER one 739-tick episode on one seed. Next read: lateral spacing at contact on both sides
  (`scripts/psp-ai-contact-field.py` logs poses; ours needs the same per-tick distance).
- **Omega's kind test is inherited, not read**: the loop is inlined in `AI_UpdateCraft`, and the v107 import's
  function boundaries differ from the documented `0x012c0480` (`ps4-omega-eu/ai-controls.md`).
- **Pure's rail drop on a pair contact is static** (confidence 75): not checked live that the pair resolver's
  bodies are `craft+0x8b4` (`psp-pure-usa/ai-controls.md`, "The rail: position only").
- **Not measured on the original:** de Konstruct Black, a confirmed faster class, Ace (`g_skill_level` 2).

## Next Steps

1. Read the second kind-`1` record's origin on PPSSPP (recipe above); rerun `OAG_SWEEP=1 OAG_SEEDS=6
   OAG_CLASSES=venom,rapier OAG_TICKS=10800 cargo nextest run -p oag-raceplay --run-ignored all --no-capture -E
   'test(ai_contact_duration_board)'` with the count it implies.
2. Lateral spacing at contact, both sides, for the remaining long episodes.
