# 2048 and Omega XP: read, not kept

2026-10-16, lane `xp-loyalty`. 2048's XP is a per-profile rank system, not loyalty (falsified,
84): `docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`. Omega carries the same award table
and a halved rank table next to HD's loyalty: `docs/ghidra/functions/ps4-omega-eu/xp-and-loyalty.md`.
This project keeps neither: 2048 pays inherited HD loyalty, Omega pays loyalty only.

## Open

- XP is not kept or drawn on 2048 or Omega: no profile total, no rank, no event awards (about 30
  gameplay call sites of `Profile_AwardXP`), no `RaceXP`/`TotalXP`/rank-bar widgets.
- Nothing seen player-visible depends on the rank except the bar and two trophies (25, 50), and no
  `<Unlock>` row needing a rank was found (70). A keep-XP lane is only justified if the maintainer wants the rank bar.
- Omega's dispatch between its loyalty block and its XP block on `EndRace Results` is not traced (50).
  `FUN_0153ebf0` builds the loyalty ticker, `FUN_0149f910` the XP blocks; no caller was found for either.
- Omega's rank thresholds are exactly half of 2048's (15000 .. 1790000).

## Next Steps

1. Find Omega's end-of-race function that banks the XP total (xref the rank-table global the loader
   fills) and diff its caller against `Race_ComputeLoyaltyAward`'s callers.
2. If the maintainer wants the rank bar: a small per-profile XP store beside the team loyalty store, fed
   from the event awards, ranks from `XPRanks.xml` through the title's archive (never a `const`).
