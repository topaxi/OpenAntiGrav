---
status: parked
titles: [all]
updated: 2026-11-20
summary: "ADR-0093 (run, modifiers, pickups, unlocks) and ADR-0094 (choice, stats) reviewed 2026-11-20, all answered; awaiting merge"
---

# A campaign run state and race-start modifiers are drafted, not accepted

[ADR-0093](../../docs/architecture/adr/0093-a-custom-campaign-keeps-a-bounded-run-state-and-its-mod-sets-bounded-modifiers-at-race-start.md)
(*Proposed*, editable until merged), its split-out
[ADR-0094](../../docs/architecture/adr/0094-a-campaign-run-offers-a-choice-between-races-and-shows-the-mods-stats.md)
(the choice screen and stats, *Proposed*) and
[progression.md](../../docs/architecture/wasm-extensions/progression.md) describe how a
mod builds a custom campaign whose races change the craft and the pickups. No crate,
no ABI revision and no mod author skill text changed. The branch was not merged by the
lead: the maintainer merges it. The review walked every challenge objection and
section 12 question with the maintainer on 2026-11-20; each answer is recorded in the
ADRs, `progression.md` and under its objection in `progression-challenge.md`.

## Open

- ADR-0093 decision 11 names the atomic records write by description: the general ADR
  is being written by the drive's `profile-atomic-writes` lane. Link its number before
  the merge, or accept the unlinked reference.
- The modded records table (personal bests for a modified race): later, outside
  ADR-0093 (maintainer, 2026-11-20); already owed in
  [custom-race-modes-and-campaigns.md](../gameplay/custom-race-modes-and-campaigns.md),
  "Still open from this slice".
- Mod stat rows on a title's own Ship Select (Pulse Team Selection, HD and Omega Ship
  Select): a follow-up of ADR-0094 decision 2 (maintainer, 2026-11-20).
- Replay compression: a replay header now carries a run of up to 4 KiB (ADR-0093
  decision 1, 2026-11-20); the maintainer expects it to compress well and favour
  compressing replay files again.
- A per-leg `race-end` for tournaments (rewards between legs): later, by the
  maintainer's answer to challenge objection 9 (2026-11-20).
- The frontend world (ADR-0080) is unbuilt, so ADR-0094's choice screen waits for it
  (its stat rows on the campaign page do not). The README's `race-start`, `race-end`
  and `setup.tune` are not in ABI revision 6 either.
- `Launch` (`accelcap`) and `TopEnd` (`engine.amount`) replace `Thrust` and `TopSpeed`
  (maintainer, 2026-11-20); each needs a held-throttle ground truth at factors 1.0 and
  2.0 on one circuit and class before it ships. Neither applies in Zone.
- Omega's Ship Select stat panel was not read (HD's front end or the 2048 panel);
  `team-grid.md` records it as checked, applies, not wired.

## Next Steps

1. The maintainer decides ADR-0093's status and merges the branch.
2. Build in the order of `progression.md` section 11: the ABI (revision 7, SDK, mod
   author skill), then the run and its lifecycle (the first playable slice), then
   modifiers (`Launch` and `TopEnd` after their ground truths), pickups, own unlocks;
   then ADR-0094.
3. When the ABI changes, update the mod author skill in the same change.
4. Delete this file once the ADRs are merged and the build steps have their own threads.
