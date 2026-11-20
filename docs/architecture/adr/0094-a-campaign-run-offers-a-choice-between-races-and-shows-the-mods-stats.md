# ADR-0094: A campaign run offers a choice between races and shows the mod's stats

## Status

**Proposed** (2026-11-20). Split out of
[ADR-0093](0093-a-custom-campaign-keeps-a-bounded-run-state-and-its-mod-sets-bounded-modifiers-at-race-start.md)
by the maintainer, so the run state can be accepted and built without waiting for the
frontend world. Nothing is built. **This ADR stays editable until it is merged into
`main`, and becomes immutable only then.**

**Challenged, being answered:** objections 14 and 15 of
[`wasm-extensions/progression-challenge.md`](../wasm-extensions/progression-challenge.md)
proposed a host-drawn offer page with a third sim export and guest-reported stat rows.
Objection 14 is answered: the choice stays a mod screen (decision 1). Objection 15 is
answered: the guest reports its stat rows, Ship Select rows follow later (decision 2).

**Narrows** ADR-0080 decision 6, "the only output is one blob" of a screen
([ADR-0080](0080-a-wasm-mod-may-own-a-screen-and-save-a-small-blob.md); decision 1).
It does **not** yet narrow "a disc screen never gains behaviour a mod invented"
([ADR-0066](0066-the-menu-is-the-disc-plus-mods.md) decision 4, ADR-0079 decision 6,
ADR-0080 decision 2): rows on a title's Ship Select are a follow-up (decision 2).

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
   2026-11-19). **Settled (maintainer, 2026-11-20):** the choice is a mod screen as
   drafted, not a host-drawn page, so a campaign owns its reward screen's art and
   layout; it waits for the frontend world. Its reply may also request a new run, which goes
   through the host's NEW RUN confirm (ADR-0093 decision 6; maintainer, 2026-11-20).
2. **Stats, reported by the guest** (maintainer, 2026-11-20). `race-end`'s reply and
   the choice screen's reply may carry up to 8 rows `stat{string id, value, max}`.
   The host stores them beside the run as a display cache and draws them on pages it
   owns for the campaign: a stats block on the campaign page, which also serves an
   event with a preset craft. The host reads no offsets in the run bytes, and a stale
   cache shows the last true values. A title bar is never recomputed from scaled
   handling. `race-end` is ADR-0093's sim export, so the rows on the campaign page
   need no frontend world. **Follow-up:** display-only rows and bonus segments on the
   title's own Ship Select for that campaign, which would narrow "a disc screen gains
   no mod behaviour"; decided when it is built.

**Not decided here:** the Ship Select rows (decision 2's follow-up) and ADR-0080's own
fate (`progression.md` section 12, question 15).

## Alternatives considered

- **A screen that returns only the pick, with `race-start` applying it.** Keeps the
  transition in deterministic code, but the Ship Select rows must show the pick before
  the next race starts, and the pick would be a second replay input. Rejected in the
  first draft; reopened by challenge objection 14.
- **A screen that rolls the offers.** No seeded RNG on the UI side. Rejected.
- **A host-drawn offer page and a third sim export, `oag_run_pick`** (challenge
  objection 14). No frontend world needed and all run logic in one deterministic
  module, but no custom art or layout. Rejected by the maintainer (2026-11-20): the
  reward screen is the campaign's own.
- **Stats on a mod screen's canvas only.** The player would see the craft and the
  title's bars on one screen and the mod's stats on another, and the canvas would need
  disc data a mod may not yet read (README open question 15). Rejected as the only
  path.
- **`[[run.stat]]` rows naming byte offsets in the run bytes** (the first draft). The
  layout would live in the rules module, the screen and the campaign file, and a mod
  update that moves a field draws wrong numbers until the next write (challenge
  objection 15). Rejected for guest-reported rows.

## Consequences

**Good.** ADR-0093 no longer depends on ADR-0080. A campaign ships with `race-end`
choosing its reward before anything here is built.

**Bad.**

- **The choice is blocked on ADR-0080.** If the frontend world is rejected, decision 1
  has no home. The stats on the campaign page do not wait for it.
- **Stats are not beside the craft yet.** Until the Ship Select follow-up, the player
  sees the mod's numbers on the campaign page and the reward screen, and the title's
  bars on Ship Select. The follow-up costs three screen integrations (Pulse's Team
  Selection, HD's and Omega's Ship Select).
