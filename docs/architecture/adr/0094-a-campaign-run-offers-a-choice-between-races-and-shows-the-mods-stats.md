# ADR-0094: A campaign run offers a choice between races and shows the mod's stats

## Status

**Proposed** (2026-11-20). Split out of
[ADR-0093](0093-a-custom-campaign-keeps-a-bounded-run-state-and-its-mod-sets-bounded-modifiers-at-race-start.md)
by the maintainer, so the run state can be accepted and built without waiting for the
frontend world. Nothing is built. **This ADR stays editable until it is merged into
`main`, and becomes immutable only then.**

**Challenged, unresolved:** objections 4, 14 and 15 of
[`wasm-extensions/progression-challenge.md`](../wasm-extensions/progression-challenge.md)
propose a host-drawn offer page with a third sim export and guest-reported stat rows,
which would remove this ADR's dependency on ADR-0080. The two decisions below are the
first draft's, moved here unchanged until those objections are answered.

**Would narrow**, if accepted as drafted:

- the "a disc screen never gains behaviour a mod invented" clause of
  [ADR-0066](0066-the-menu-is-the-disc-plus-mods.md) decision 4, ADR-0079 decision 6
  and [ADR-0080](0080-a-wasm-mod-may-own-a-screen-and-save-a-small-blob.md) decision 2
  (decision 2);
- ADR-0080 decision 6, "the only output is one blob" of a screen (decision 1).

**Builds on** ADR-0093 (the run, its seed and ordinal, `race-end`, the commit on exit)
and ADR-0080 (the frontend world, itself *Proposed*). The surface is
[`wasm-extensions/progression.md`](../wasm-extensions/progression.md) section 8.

## Context

ADR-0093 gives a custom campaign a run state that `race-end` writes on the exit from
each race. A campaign can already pick its reward inside `race-end`. Two things need
more than the sim world: letting the player choose between offers, and showing the
mod's own numbers (engine level, shield level) beside the title's bars. The frontend
world that would host both is a Proposed design, and nothing of it is built.

## Decision

1. **A choice screen** is an ADR-0080 screen named by `[run] choice_screen`, opened by
   the host after an exit whose `race-end` asked for it (`END_OFFER`). It receives the
   committed run bytes and may return replacement bytes, as a second allowed output
   beside `save` (narrowing ADR-0080 decision 6). The offers are rolled in `race-end`
   from the run's seed and ordinal, because the frontend world has no seeded RNG.
   Whether the player may skip an offer is the mod's choice, per offer (maintainer,
   2026-11-19).
2. **Stats.** `[[run.stat]]` rows name a byte offset, a kind and a style in the run
   bytes. The host draws them and interprets nothing else. A campaign that declares
   them may add **display-only** rows and bonus segments to the stat panel of the
   Ship Select it launches, for that campaign only (the one allowance that narrows
   "a disc screen gains no mod behaviour"). A title bar is never recomputed from scaled
   handling.

**Not decided here:** the host-drawn offer page and `oag_run_pick` (challenge objection
14), guest-reported stat rows (objection 15), the page that shows stats when the craft
is preset (`progression.md` section 12, question 10), and ADR-0080's own fate
(question 15).

## Alternatives considered

- **A screen that returns only the pick, with `race-start` applying it.** Keeps the
  transition in deterministic code, but the Ship Select rows must show the pick before
  the next race starts, and the pick would be a second replay input. Rejected in the
  first draft; reopened by challenge objection 14.
- **A screen that rolls the offers.** No seeded RNG on the UI side. Rejected.
- **Stats on a mod screen's canvas only.** The player would see the craft and the
  title's bars on one screen and the mod's stats on another, and the canvas would need
  disc data a mod may not yet read (README open question 15). Rejected as the only
  path; kept as the fallback if the maintainer refuses the allowance in decision 2.

## Consequences

**Good.** ADR-0093 no longer depends on ADR-0080. A campaign ships with `race-end`
choosing its reward before anything here is built.

**Bad.**

- **Blocked on ADR-0080 as drafted.** If the frontend world is rejected, both
  decisions have no home unless objections 14 and 15 are taken.
- **Three screen integrations for the stats** (Pulse's Team Selection, HD's and
  Omega's Ship Select), and a fourth place for preset craft.
