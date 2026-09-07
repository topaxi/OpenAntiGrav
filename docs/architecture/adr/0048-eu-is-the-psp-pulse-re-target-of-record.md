# ADR-0048: `psp-pulse-eu` is the Ghidra target of record; `psp-pulse-usa` corroborates

## Status

Accepted.

## Context

**The decision this ADR records already exists in the repository, just not as
an ADR.** On 2026-08-05, commit `c4b9786f` flipped the Ghidra target of record
from `psp-pulse-usa` to `psp-pulse-eu`, "per explicit direction" from the
maintainer. It recorded the flip in three places: a dated notice at the top of
[`docs/ghidra/functions/psp-pulse-eu/corroboration.md`](../../ghidra/functions/psp-pulse-eu/corroboration.md),
a header comment in `psp-pulse-eu/names.tsv`, and a `HANDOVER.md` row. The
commit message says explicitly: *"No ADR conflict - ADR-0005 names
`docs/ghidra/functions/<binary>/` generically and never hardcoded a primary
target."* That reasoning is correct as far as it goes - no *existing* ADR
contradicted the flip - but it is also why the decision had nowhere permanent
to live once its `HANDOVER.md` row was deleted, which happened per this
project's own rule that a thread's index row is removed once its work lands
(see `CLAUDE.md`, "Documentation tree"). The corroboration page's dated notice
and the `names.tsv` header comment survive, but neither is a place a reader
looking for "why does this project prefer one region" would think to check
first, and a project-wide policy is exactly the kind of decision ADRs exist to
make findable.

This was surfaced again independently: the maintainer recalled "this has been
documented in the past" when asking for tooling to migrate, and a search of
`docs/reverse-engineering/`, `docs/ghidra/`, `CLAUDE.md` and the ADR index did
not find it on the first pass - only a full-repository search, including the
project's own work-in-flight tracking outside `docs/` and its git history,
did. The maintainer's memory was right, but the finding was scattered exactly
the way this ADR exists to fix.

**A second, older document actively disagrees with the flip and was never
updated**: [`docs/reverse-engineering/source-images.md`](../../reverse-engineering/source-images.md)
still called `pulse-psp-usa.chd` "the primary reverse-engineering target" and
`pulse-psp-eu.chd` "not the reverse-engineering target of record" until this
change corrected it. That page is a living reference, not an ADR, so it is
edited directly rather than superseded.

### The maintainer's stated reasoning

> *"have tooling migrate to the EU version where possible, most RE shall be
> done there moving forward, this has been documented in the past, but I think
> doing RE on pulse USA was more convenient because we had more named
> functions etc. there. that is a cap to be closed at some point, so that we
> can do EU first, and use USA as a second 'opinion'"*

The 2026-08-05 flip's own rationale, unchanged by this ADR:

- **Pulse PSP's DLC shipped on the EU disc only** - the USA release never got
  any (`data/README.md`, DLC section) - so the EU disc is the more complete
  artefact of the two, for the game itself and not only for its executable.
- **Pulse PS2 is EU-only; no USA PS2 disc exists.** `ps2-pulse-eu` was already
  this project's sole PS2 reference by necessity. Preferring `psp-pulse-eu` on
  PSP too keeps both platforms' primary targets in the same region rather than
  split, which also **resolves the region asymmetry**
  `source-images.md`'s own "Region asymmetry" section flagged: before this
  flip, the PSP reference was US and the PS2 reference was EU, confounding
  every platform comparison with a region comparison. With both PSP and PS2
  targets now EU, that confound is gone for anything reasoned from the target
  binaries themselves (the USA disc remains available as the explicit second
  opinion when a region-specific difference is exactly what is being tested).

## Decision

**`psp-pulse-eu`/`BOOT.BIN` is the Ghidra target of record for new PSP Pulse
reverse-engineering. `psp-pulse-usa`/`BOOT.BIN` (and `ps2-pulse-eu` where
relevant) are the corroboration sources** - a second binary that either
confirms a finding or surfaces a real regional difference, per the confidence
rubric's existing "a second binary is worth more than a second reading of the
first" principle. This is the same policy `c4b9786f` already put in force; this
ADR is what makes it discoverable and durable rather than re-deciding it.

Concretely, for new work:

- A fresh Ghidra investigation starts in `/psp-pulse-eu/BOOT.BIN`, documents
  under `docs/ghidra/functions/psp-pulse-eu/`, and cross-verifies against
  `/psp-pulse-usa/BOOT.BIN` with `find_similar_functions_fuzzy`/
  `diff_functions` once a name exists - not the reverse.
- Tooling that has to pick one PSP Pulse binary by default should now default
  to EU, **except** where something else already ties it to USA specifically
  (see Part 3 below) - migrated case by case, not by a single global switch.
- This does not retroactively invalidate anything already documented against
  `psp-pulse-usa` at that binary's own confidence. A finding stays valid
  evidence for its own binary regardless of which one is preferred for new
  work.

### The cost, honest and current

**This policy is aspirational until the name gap closes**, and that gap is
real: measured directly (`grep -vc` over comment/blank lines) on 2026-09-07
before any of this change's own work, `psp-pulse-eu/names.tsv` carried **301**
rows against `psp-pulse-usa/names.tsv`'s **656** - a 355-row gap, not the
~364 estimated before measuring. **This same change also closes part of it**:
an exact opcode-hash match (`get_bulk_function_hashes` on both binaries,
unique on both sides, `diff_functions`-verified zero-diff body on every one of
115 candidates before applying anything - see
[`exact-hash-transfer.md`](../../ghidra/functions/psp-pulse-eu/exact-hash-transfer.md))
brought `psp-pulse-eu/names.tsv` to 416 rows, narrowing the total-row gap to
240. **182 named functions remain with no exact-hash EU counterpart** (plus 25
`data`-kind rows this technique cannot touch at all), and those need a live
Ghidra session running `bulk_fuzzy_match` and the structural/positional
techniques `docs/ghidra/functions/psp-pulse-eu/corroboration.md` already
proved out - designed, not executed, in the handover thread this change also
updates. Doing RE "on EU first" is still less convenient today than the
USA-side habit it replaces, precisely because USA still has substantially more
named functions to search, cross-reference and build on - just measurably less
so than before this change.

## Alternatives considered

**Do nothing beyond what `c4b9786f` already recorded.** Rejected: that is the
status quo this ADR is fixing. The decision demonstrably was not findable
without a full-repository search, which is the same failure mode ADRs exist to
prevent for every other hard-to-make decision in this project.

**Retroactively rewrite `psp-pulse-usa`'s pages to read as corroboration-only
from the start.** Rejected: it would erase the historical record of which
binary a finding was actually derived against, and this project's own rule for
ADRs (immutable, superseded rather than edited) applies in spirit here too -
`corroboration.md`'s own four sweeps are kept verbatim as history for the same
reason.

## Consequences

- New Ghidra work on PSP Pulse defaults to `psp-pulse-eu`; `psp-pulse-usa` is
  the explicit second opinion, named as such.
- Tooling migration (Part 3 of the task this ADR grew out of) is scoped
  file-by-file against this decision, not applied uniformly - some tooling is
  correctly USA-pinned for reasons unrelated to which binary is preferred for
  naming, and forcing those to EU too would break them. See the accompanying
  change's report for the file-by-file list.
- The name gap narrowed from 355 to 240 total rows (115 functions, via exact
  opcode-hash matching) in this same change, once Ghidra access was granted
  mid-task; the 182 functions and 25 data rows still remaining are designed,
  not executed, in the handover thread this change updates.
- `docs/reverse-engineering/source-images.md` is corrected in the same change,
  since a reference page giving the opposite answer to this ADR would be worse
  than the page saying nothing at all.
