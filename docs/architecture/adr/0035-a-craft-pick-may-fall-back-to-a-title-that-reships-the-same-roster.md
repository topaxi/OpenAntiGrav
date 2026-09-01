# ADR-0035: A Race Remix craft pick may fall back to a title that reships the same roster

## Status

Accepted. Confined to Race Remix's own CRAFT TITLE/TEAM/VARIANT rows
(`crates/game/src/main/session/remix.rs`); does not touch `oag-title`, the
ordinary RACE page, or a direct `--race` launch.

## Context

Wipeout 2048's own team plugin declares seventeen teams: its own five, and
twelve spelled identically to Wipeout HD/Fury's roster, reshipped verbatim
under a tree of 2048's own (`Data\art\published\hdships`, confirmed against
the manifest for all twelve teams and their `_c1`/`_n1` reskins - see
`crates/2048/src/race.rs`'s `GUEST_TEAM_VARIANTS`). Race Remix's CRAFT TITLE
picker showed all seventeen flat under "Wipeout 2048" - indistinguishable
from the five that are genuinely this title's own, and unable to load at
all, since `oag_title::RaceDefaults` had one `ship_dir`/`handling_dir` pair
per title and 2048's pointed at its native tree only.

The request was to fold the twelve into "Wipeout HD"'s own CRAFT TITLE
entry instead, offered whenever a real HD/Fury source **or** Wipeout 2048 is
on this machine's search path, a real disc always preferred over 2048's
reused copy when both are present.

## Decision

1. **A real title always wins.** `Session::craft_backing`
   (`resolve_craft_backing` free function, unit-tested against hand-built
   `launcher::Candidate` values) resolves `remix.craft_title` against the
   real survey first; only when "Wipeout HD" is named and no real HD
   candidate exists does it fall back to Wipeout 2048's own candidate. This
   mirrors an existing precedent rather than inventing a new rule:
   `oag_assets::Archives::open_with_packs` already documents "the disc
   always wins a collision" for a downloaded pack against a shipped one -
   the more canonical source wins here on the same grounds.
2. **Confined to the CRAFT TITLE row, and to Race Remix.** TRACK TITLE's own
   resolver (`Session::remix_catalogue_for`) does not gain this fallback -
   2048's own circuits are not HD's, so no fallback applies there. A direct
   `--race --source <2048>` or the ordinary RACE page (unreachable for 2048
   today regardless, since it has no front end) sees exactly what it saw
   before this ADR: `oag_title::RaceDefaults::guest_roster` and
   `RaceDefaults::{team_variants_for, ships_for, handling_dir_for}` are
   additive infrastructure that makes the twelve teams *loadable at all*
   (needed under every design here, independent of this decision - see
   `crates/title/src/race/variants.rs`), but nothing about which title a
   direct launch opens changes.
3. **The reused roster is duplicated data, not a new crate dependency.**
   `oag_2048::race::GUEST_TEAM_VARIANTS` is 2048's own `const`, measured
   against 2048's own manifest, spelling the same twelve team ids and the
   same `""`/`_c1`/`_n1` suffix scheme
   [`oag_hd::race::TEAM_VARIANTS`](../../../crates/hd/src/race.rs) carries -
   confirmed to resolve identically under 2048's tree rather than assumed
   from the name. `oag-2048` does not gain a dependency on `oag-hd` to
   reference its table: no title crate depends on another today (only
   `oag-hd`'s *dev*-dependencies reach `oag-pulse`/`oag-pure`, for tests),
   and a runtime edge here would be a new precedent this feature does not
   need. Keeping every title package independent - "a title package is
   tables, measured off its own release" - was worth the duplication.

## Alternatives considered

**Merge the two into one title-wide axis inside `oag-title`.** Rejected:
`RaceDefaults` is shared vocabulary across four titles, and "which title
backs a CRAFT TITLE pick when the named one is absent" is a fact about Race
Remix's own UI, not about what a title *is* - the same reasoning
[ADR-0034](0034-a-race-may-open-two-titles-at-once.md) item 3 already
applied to `crate::remix::Remix` itself.

**Fold the directory-routing fix (`guest_roster`) and the picker fallback
(`craft_backing`) into one mechanism.** Considered and rejected once probed
concretely: the two answer different questions (`guest_roster` says *which
directory* a team's files live under; `craft_backing` says *which real
source* a menu pick opens), and 2048's own two rosters already disagree on
both the join (`Subdirectory` vs `Suffix`) and the tree count (two trees for
the native five, one for the guest twelve) - folding them would repeat the
single-example generalisation [ADR-0009] warns against, the same trap
`VariantJoin` was built to avoid.

## Consequences

**Good.** The guest-roster fix (`RaceDefaults::guest_roster` and its three
`_for` methods) is genuinely general: `--race --source <2048> --team
Assegai_c1` works with no Race Remix involved at all, and is exercised
standalone in `race_remix_ground_truth::a_guest_team_races_standalone_on_2048_without_a_craft_split`.

**Good.** No title crate gained a dependency on another; `just check-deps`'s
two rules are unaffected, and the picker-level fallback is entirely
`crates/game`-side, matching [ADR-0034](0034-a-race-may-open-two-titles-at-once.md)'s
own precedent that a composition-root-level concern needing more than one
title's data stays beside `crate::title`, not inside `oag-title`.

**Bad, unresolved.** The duplication ADR item 3 accepts is a real
maintenance seam: if HD/Fury's own roster or suffix scheme ever changed,
`oag_2048::race::GUEST_TEAM_VARIANTS` would not update with it, and nothing
enforces the two staying in step beyond the doc comment saying so and the
manifest probe that measured them equal once. Acceptable while there is
exactly one guest roster on one title; worth revisiting if a second one
ever needs the same shape.

[ADR-0009]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0009-multi-game-fanout.md
