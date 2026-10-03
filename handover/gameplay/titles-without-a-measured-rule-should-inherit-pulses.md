# Where a title still runs a chosen rule while Pulse has a recovered one

2026-10-03, from the `fire-law-inherit` lane. The maintainer's rule: a title with no measured
law of its own runs Pulse's, never this project's chosen fallback, labelled **inherited from
Pulse, unmeasured on <title>**. The opponent fire law is done (all five titles ship a
`WeaponAIstats.xml`, `docs/gameplay/race-modes.md`, "Firing on the original's law"). The rest of
the survey is below; none was trivial enough to fix in that lane.

## Open

- **The start gantry's clock** (`crates/game/src/race/gantry.rs`, `clock_seconds`): Pulse starts
  its timeline at tick 92 (measured); every other title starts at `0`, chosen. HD's and 2048's
  timelines differ (2048's `GO` is frame 200, not 181), so inheriting means aligning `GO` to the
  release tick, not copying 92. Rendering lane.
- **Wreck effects, hit sparks, absorb burst** (`wreck_fx.rs`, `hit_sparks.rs`, `absorb.rs`):
  Pulse-only. Pure and 2048/Omega draw nothing; HD has its own mechanisms. An honest absence,
  not a chosen rule, but under the maintainer's rule an unmeasured title would play Pulse's
  emitters on its own locators. Needs each title's locator names first.
- **Pure's `VECTOR` lap count** falls back to Venom's `3` with a load warning (`race-modes.md`,
  campaign rungs): chosen, not measured. Pulse's rung table is the known rule.
- **Zone ship handling on Pure and HD** is unread; Pulse's rule (the player's own team numbers)
  is the known one (`race-modes.md`, Zone ship).
- **The absorb half and Mine/Bomb/Turbo/Cannon fire policy** of the opponent weapon AI are this
  project's own on every title, Pulse included: a Pulse gap, not a title one.
- **HD's `EliminatorAIStats` row** (flip, absorb and use scales by difficulty, in
  `weaponaistats.xml`) is authored and unread; Pulse's file has no such row, so this is HD-only.

## Next Steps

1. Draw the gantry clock first: it is the one a player sees on every non-Pulse race start.
