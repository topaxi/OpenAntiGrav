---
status: parked
titles: [all]
updated: 2026-11-19
summary: "ADR-0093 and progression.md draft a campaign run state, race-start modifiers, pickup parameters, a choice screen and stats; waiting on review"
---

# A campaign run state and race-start modifiers are drafted, not accepted

[ADR-0093](../../docs/architecture/adr/0093-a-custom-campaign-keeps-a-bounded-run-state-and-its-mod-sets-bounded-modifiers-at-race-start.md)
(*Proposed*, editable until merged) and
[progression.md](../../docs/architecture/wasm-extensions/progression.md) describe how a
mod builds a custom campaign whose races change the craft and the pickups. No crate,
no ABI revision and no mod author skill text changed. The branch was not merged by the
lead: the maintainer reviews it first.

## Open

- The maintainer's review of the ADR. Every decision in it, and the sixteen open
  questions at the end of `progression.md`, are unsettled. The ones that change the
  design most: clamp or fault at race start (1), the commit rule that stages the medal
  with the run state (section 5), the Ship Select allowance for mod stats (section 8.2),
  and whether the AI may be scaled (11).
- The frontend world (ADR-0080) is unbuilt, so the choice screen and the stat rows have
  no home until it lands. The README's `race-start`, `race-end` and `setup.tune` are not
  in ABI revision 6 either.
- `TopSpeed` has no field: scaling `engine.accelcap` is unmeasured. The measurement is a
  held-throttle terminal speed at accelcap 1.0, 1.25 and 1.5 on one circuit and class.
- Omega's Ship Select stat panel was not read (HD's front end or the 2048 panel);
  `team-grid.md` records it as checked, applies, not wired.

## Next Steps

1. Review ADR-0093; change it in place or reject it. Accepting it also decides whether
   ADR-0080 proceeds.
2. If accepted, build in the order of `progression.md` section 11: the ABI and the run
   record, then modifiers (with a disc-backed ground truth), then pickup parameters and
   `give-autopilot` (with weapon-lineage rows), then the frontend world and the screen.
3. When the ABI changes, update the mod author skill in the same change.
4. Delete this file once the ADR is merged and the build steps have their own threads.
