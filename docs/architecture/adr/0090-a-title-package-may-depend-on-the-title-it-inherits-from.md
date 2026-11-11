# ADR-0090: A title package may depend on the title it inherits from

## Status

Accepted (2026-11-11). Amends the "title packages are independent tables" reading of
[ADR-0022](0022-title-packages.md).

## Context

Omega's HUD layouts are HD's by lineage (`docs/formats/omega-status.md`), and its always-on
widget list was a copy of HD's. The copy drifted: HD added `LapPanel2` (the Eliminator lap
hexagon) and Omega, which authors the same widget, never drew it. Until now no title package had
a normal dependency on another; `oag-hd` was only a dev-dependency of `oag-omega`, and some
comments said a title package depends on no other.

## Decision

1. A title package may take a normal dependency on the one title it inherits from by lineage, to
   read that title's data instead of copying it. Omega depends on HD (its front end and HUD are
   HD's). Today that is the only such edge: `crates/omega/Cargo.toml` lists `oag-hd` under
   `[dependencies]`, first used by `oag_omega::hud::ART`'s `always_on` (`oag_hd::hud::ALWAYS_ON`).
   Omega's front end is HD's `PI001` plugin.
2. Never the reverse, and never a cycle. The inherited title knows nothing of its descendant.
3. Only data the descendant's own files confirm is shared: Omega reads `oag_hd::hud::ALWAYS_ON`
   because its layouts author the same widgets (`omega_hud_ground_truth`). A value the
   descendant measured differently stays its own, as `HD_ERA_TEAMS` does.
4. Generic crates still reach titles through `oag_title::Title` and the registry; this edge
   changes nothing for them.

## Consequences

- One definition of a shared list: a widget added for HD reaches Omega, and a test pins the
  two equal.
- A change to HD's data can now move Omega. The pinning test and the ground-truth tests are the
  guard; an Omega difference is made by owning a list again, not by patching HD's.
- Linking `oag-omega` links `oag-hd`; both are tables-only crates, so the cost is small.
- Dependency rules 1-6 are unchanged and `check-deps` needed no edit.
