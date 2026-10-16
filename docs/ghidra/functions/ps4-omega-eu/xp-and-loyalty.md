# Omega carries both systems: XP (2048's) and loyalty (HD's)

2026-10-16, lane `xp-loyalty`. Ghidra `/omega/eboot-ps4-omega-eu.bin`, data from
`data00.psarc`. Static only, nothing live (no PS4 emulator). No rename comes of this
page; confidence for each claim is stated inline.

**Both are present and both are loaded.** (confidence 85)

- XP: `FrontendRoot_Construct` (`0x013c2eb2`) references `Data\XML\XPAwards.xml`
  (`0x0182eb7a`) and `XPRanks.xml` (`0x0182ec55`); the files ship in `data00.psarc`.
  `XPAwards.xml` is byte-identical to 2048's (`vita-2048-eu-v104/xp-award.md`); `XPRanks.xml`
  has 50 ranks at exactly half of 2048's thresholds (rank 1 = 15000, rank 50 = 1790000).
- Loyalty: `Race_ComputeLoyaltyAward` (`0x016544a0`, `endrace-loyalty.md`), HD's law.

**The end-race screen shows both** (confidence 75, layout only): `endrace_definition.xml`'s
`EndRace Results` has an HD-style `HDFuryLoyaltyItems` block (`ER_LOYSTAT`, `loyalty1.1`,
`loyalty1.2`, `loyalty2`, built by `FUN_0153ebf0`) and a 2048-style `Objective` +
`eventxp_block`/`totalxp_block` block (`FE_EVENT_XP`, `PASS_BONUS_XP`, `CO_TOTAL_XP`,
`xpbar`, built by `FUN_0149f910`), and `vita_EndRace_Definition.xml` is 2048's own
RaceSummary/ObjectiveSummary/Results set. Which family pays which is therefore by the
content's lineage, not a measured switch:

| Content | Pays | Evidence |
| --- | --- | --- |
| HD/Fury-derived races and campaign (mode ids with the HD class table) | loyalty | `Race_ComputeLoyaltyAward`'s mode classes are HD's; the ticker is the `HDFury*` block |
| 2048-derived campaign events (objective + pass/elite chain) | XP | `PASS_BONUS_XP` (the 10000 `Event complete`), `Objective` widgets |
| Online | not traced | |

**Not traced** (open, confidence 50 for the mapping above): the code that picks between
the two Results blocks (both big constructors, 14 KB for `FUN_0149f910`), and whether
Omega's loyalty-gated unlocks and its XP rank are both user-visible. **Not wired** in this
project: Omega races pay loyalty only; XP is not kept.

Cross-check: 2048 - ported (see `vita-2048-eu-v104/xp-award.md`, the same system, same
award table); HD - checked, differs (HD has no XP).
