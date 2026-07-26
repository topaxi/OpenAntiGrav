# ADR-0006: No copyrighted content in the repository

## Status

Accepted.

## Context

This project reads and documents the data formats of a commercial game. That
work is legitimate; distributing the game's content is not.

Projects of this kind fail in a predictable way: a test needs a real asset, the
asset is small, someone commits it "temporarily", and the repository is now
undistributable. By the time anyone notices it is in the history.

## Decision

**No game content in the repository, ever.** No assets, no executables, no
extracted data, no disc images, and no derived data that reconstitutes the
original.

Enforced in three layers:

1. `data/` is gitignored except its README.
2. Extension patterns (`*.chd`, `*.iso`, `*.wad`, `*.elf`, `*.bin`, and others)
   are gitignored globally, so a stray copy outside `data/` is still caught.
3. CI fails if any tracked file matches those patterns. Same check locally via
   `just audit-leakage`.

Three layers rather than one because the failure is effectively irreversible:
rewriting history after the fact is disruptive and often incomplete.

### Fine to commit

Format documentation with field offsets and structure layouts; parsers; SHA-256
hashes of source images; disc serials, volume identifiers and file listings;
reverse-engineering notes with addresses and signatures; and small test fixtures
we authored ourselves.

### Not fine

Any byte sequence taken from the game.

### Tests

Tests build the structures they need by hand. The ISO 9660 tests construct
synthetic directory records; they do not read a disc.

Tests that genuinely need a real image are marked `#[ignore]`, run only via
`just test-data` against images the developer supplies, and never run in CI.

## Alternatives considered

**Commit small fixtures and argue fair use / de minimis.** Would make some tests
much easier to write. Rejected: it is a judgement call with real downside and no
proportionate benefit, and hand-built fixtures test the parser better anyway
because they can be constructed to hit specific edge cases.

**Gitignore `data/` and rely on discipline.** Rejected. Discipline fails, and
this particular failure is expensive to undo.

**Git LFS or a submodule for assets.** Rejected. It relocates the distribution
rather than avoiding it.

## Consequences

**Good.** The repository is unambiguously distributable. Contributors know
exactly where the line is. The CI check makes accidents impossible rather than
merely discouraged.

**Bad.** Tests cannot use real data by default, so the parsers' first contact
with real-world messiness happens outside CI. Contributors must obtain their own
copies before they can do anything with the disc tooling, which raises the bar
for a first contribution. And the `#[ignore]` tests, being unrun by CI, will rot
unless someone runs them deliberately.

**Mitigation for the last point:** `just test-data` is documented in the README
and should be run before any release, and any change to `oag-disc` or
`oag-formats` should be validated against real images before merge.
