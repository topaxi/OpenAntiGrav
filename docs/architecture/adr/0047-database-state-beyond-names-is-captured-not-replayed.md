# ADR-0047: Database state beyond names is captured, not replayed

## Status

Accepted. Extends [ADR-0005](0005-ghidra-conventions.md), which governs names
and their evidence pages; this one governs everything else a Ghidra database
holds.

## Context

[ADR-0005](0005-ghidra-conventions.md) made function and data *names*
reproducible: `names.tsv` carries every one, `just apply-names` replays them
into a fresh database, and `just check-names` validates the table offline in the
`just` gate. Nothing covers the rest of what a database accumulates - struct and
enum definitions, function prototypes, local variable names, comments - and all
of it dies with the database, which is not committed and cannot be diffed.

The question was put directly: *"can we enhance names.tsv or create more files
to track things like struct and data types, function signatures/prototypes,
local variable names and potentially more?"* It arrived with a deadline. The PSP
databases were being replaced by fresh imports built with the Allegrex
relocation patch, so whatever those four held that `names.tsv` did not was about
to be deleted, and the working assumption was that it amounted to weeks of
struct work.

The deadline turned out to be the smaller half of the problem. The maintainer,
who works on two workstations, put the structural defect plainly:

> apart from `names.tsv`, anything else in ghidra will have been invisible
> between workstations

That is the real condition, and it has been running since the project started. A
reimport is merely the first event that made it legible. So the thing to design
is not a rescue but a **synchronisation channel**: something two machines can
converge through, which means it must be diffable and mergeable in git the way
`names.tsv` is, not an opaque dump.

### What the databases actually held

Before designing anything, all seven databases were measured
(`docs/ghidra/captures/README.md` has the table and the method). The assumption
was wrong, and by a wide margin:

- **Zero hand-recovered structs**, in any database. Every composite present is
  Ghidra's ELF loader (`Elf32_Ehdr`), a console SDK module table
  (`_scelibent_ppu32`, `SceProcessParam`), or a libc header a parse imported
  into `ps3-hdfury-eu` (`stat`, `tm`, `lconv`). None describes a game structure.
- **Zero hand-written prototypes.** All 10 on `psp-pulse-usa` are
  `IMPORTED`-source libc (`strlen`, `sinf`, `powf`). All 425 on `ps3-hdfury-eu`
  are libc or libm from Ghidra's signature database.
- **Zero hand-named variables.** All 707 named parameters in the project belong
  to functions whose signature Ghidra supplied (`__nptr`, `__s1`, `__x`).
- **52 plate comments** and **1,786 user labels** across all seven databases -
  the only material a person wrote that `names.tsv` does not carry.

And `names.tsv` had held completely where it mattered. The three PSP databases
under deletion pressure have **zero orphan function names** between them; the
table is ahead of the databases rather than behind. A second Ghidra project from
another workstation, eight days stale, contains **no name this machine lacks**
and exactly one disagreement in the whole corpus.

### How much actually diverged between the two workstations

The maintainer's account of the mechanism is exactly right, and the cost it
produced can now be counted. Comparing all six shared programs, machine against
machine:

| Axis | Diverged |
| --- | --- |
| Structs | 0 |
| Prototypes | 0 (425 on each, the identical libc set) |
| Variable names | 0 (707 on each, the identical libc set) |
| Function and data names | 0 - `names.tsv` carried every one |
| Plate comments | **34**: 1 on the other machine only, 33 here only |

So the channel was missing, and in the whole history of the project the traffic
it failed to carry was thirty-four comments. Two readings of that fit the
evidence and this ADR does not need to choose: either nobody wanted to record
structural knowledge, or - the maintainer's point - nobody bothered because
there was visibly nowhere for it to go. The second is the more likely, and it is
why item 3 lowers the bar rather than raising it.

**The one item that diverged onto the other machine is a wrong claim**, and that
is the most instructive result here. Its plate comment at `ps3-hdfury-eu`
`0x008b79cc` describes a "flat array of 73 const char\* key-name templates" and
cites `get_xrefs_to` as verification. A later pass on *this* machine established
the opposite and wrote it up in
`docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`: the region
is **not** an array, it is the module's TOC, `get_xrefs_to` on it returns
nothing, and nothing indexes it. The correction reached the docs tree, the docs
tree reached both machines, and the stale comment sat in the other database
regardless - because a comment has no way to be corrected remotely.

That inverts the usual argument for syncing comments. An unshared comment is not
only an unshared asset; it is an **uncorrectable wrong claim**, which is the
precise harm [ADR-0005](0005-ghidra-conventions.md) invokes to justify the
evidence rule for names.

So the priority order the question implied - structs first, as the expensive
part of RE - describes work not yet done rather than work at risk. Building
apply-and-check machinery for it now would produce exactly the museum piece the
`names.tsv` rules exist to prevent: a format with an empty file, a validator
that validates nothing, and a `just` recipe that costs time on every commit to
assert that zero rows are well-formed.

### The four candidates, judged on what they cost

- **Struct and data-type definitions** would be the most valuable thing to
  track, and there is nothing to track. A convention already exists in
  `docs/ghidra/structures/README.md`, including the `unk_0x20` naming rule and
  the requirement that field-level evidence be recorded separately from
  whole-struct confidence. It has never had an entry.
- **Prototypes** are, today, entirely Ghidra's own recognition of libc, and
  regenerate for free on reimport. A recovered prototype for a *game* function
  is a real claim, but none exists yet.
- **Local variable names** are the weakest candidate even in principle. A
  variable name is scoped to one decompilation of one function, and Ghidra's
  decompiler renumbers `local_*` freely between versions, so a name pinned to a
  stack offset in one import may land on a different value in the next. It is
  the piece of database state least able to survive the reimport this ADR is
  about.
- **Comments** duplicate the docs tree. All 11 plate comments in
  `psp-pulse-usa` cite a `docs/ghidra/functions/psp-pulse-usa/*.md` page, and 16
  of the 17 pages cited across the project exist. They are a convenience copy
  placed where the reader is, and the docs page is the record of truth per
  ADR-0005.

## Decision

### 1. A capture is a snapshot of a database, not a claim about a binary

`docs/ghidra/captures/<binary>/` records what a database held: user-defined
function names, user-defined labels, plate comments, and - when they ever exist
- non-library prototypes and variable names. Files are TSV, one row per fact,
addresses as the database reported them.

TSV specifically, and one row per fact specifically, because the format's job is
synchronisation: a capture from another machine must produce a readable `git
diff` and a mergeable conflict, exactly as `names.tsv` does. A serialised Ghidra
export would rescue a database and still leave two machines unable to converge,
which is the defect the maintainer identified and the larger half of the
problem.

A capture is an **observation of a working copy**, categorically different from
a `names.tsv` row, which is a **claim about the binary**. That difference
decides everything below.

### 2. Captures are not replayed

`just apply-names` does not read a capture, and no recipe writes one into a
database. A capture says "this is what the database contained on this date"; it
does not assert that a fresh database should contain it. Replaying one would
push names into a database that `names.tsv` never sanctioned, which is precisely
the defect `scripts/audit-ghidra-names.py` exists to find and `--prune` to
remove.

This is a deliberate departure from ADR-0005's replayability rule, and it has a
cost: a capture is a museum piece by that rule's own definition. The
justification is that the alternative is worse. A capture's value is as
*evidence about the past* - what was lost, what was never there, whether two
machines diverged - and evidence does not need to be replayable, it needs to be
preserved and readable. What keeps it from rotting is item 4.

### 3. Captures carry no confidence score, and no evidence page per row

ADR-0005 requires both for a name, and `scripts/apply-ghidra-names.py` refuses a
row whose address and name do not both appear on the page it cites. Neither
applies here.

A confidence score answers "how sure are we this is what the binary does". A
capture makes no such assertion - it reports, faithfully, that a database held a
possibly wrong name. Scoring it would be scoring somebody else's claim. Where
the captured name also has a `names.tsv` row, that row's confidence governs and
is not duplicated.

Demanding an evidence page per row would mean writing 3,821 pages to record
3,821 observations, so nothing would be recorded and the databases would be
deleted unmeasured. That is the failure mode this ADR was asked to avoid.

The bar is lowered deliberately and the reasoning is worth stating, because
ADR-0005 raised it deliberately too. Its rule exists because **a wrong name
actively misleads**: the decompiler answers to it, so a search lands on the
wrong function and the reader gets no signal that anything is off. A struct
field layout that is merely *incomplete* does not mislead that way -
`unk_0x20` announces its own ignorance, which is the whole point of the naming
convention in `docs/ghidra/structures/`. Applying the name bar to field layouts
would price recording out of reach for the thing it was never designed to
govern, and the measured cost of that is on the table above: two workstations,
the entire life of the project, and not one structural fact exchanged between
them.

**This relaxation is bounded to captures and does not travel.** The moment a
capture row is promoted into `names.tsv` it takes on the full ADR-0005 bar:
confidence, `_q` below 70, no name at all below 50, and a page that names the
address. A capture is where a name waits to be adjudicated, never a way around
the adjudication.

### 4. Captures are checked offline, in the gate

`just check-captures` (`scripts/check-ghidra-captures.py`) runs in the `just`
gate and in CI, without a Ghidra bridge. It asserts every file parses with the
right columns, every address is well-formed, every capture directory names a
binary the evidence tree knows, and - the part that earns its place - that the
measurement table in `captures/README.md` matches the files beside it, and that
every `docs/` page a captured comment cites exists.

That last check found a dangling reference on its first run: a plate comment in
`psp-pure-usa` points at `docs/ghidra/functions/psp-pure-usa/rocket-visuals.md`,
which was never written. `just check-docs` could not have found it, because it
walks prose in `docs/`, not comment payloads. The known-dangling set in the
script is a ratchet: it may shrink, and a new entry means someone cited a page
they did not write.

The check deliberately does **not** assert that a capture agrees with
`names.tsv`. A snapshot may legitimately disagree with a table that was
corrected afterwards, and failing the gate on that would force the snapshot to
be edited - and an edited snapshot is not a snapshot. `just audit-names` makes
that comparison against a live database, where a disagreement is actionable.

### 5. Structs, prototypes and variable names get a format, not machinery

The format is specified here so that the first recovery has somewhere to go, and
nothing is built until there is a row to put in it.

**Structs and enums** are recorded as a page under `docs/ghidra/structures/`,
following the conventions already written in that directory's README: PascalCase
type, `snake_case` fields, `unk_0x20` for an unknown field so the offset is
visible and no padding is claimed, a field table carrying per-field evidence, and
a whole-struct confidence. Field-level confidence is required and whole-struct
confidence is not sufficient, because a structure is recovered one field at a
time and `+0x00` is routinely far better established than `+0x3c`.

A struct page names its fields and their offsets. Under
[ADR-0006](0006-no-copyrighted-content.md) that is a description of a format and
is ours to write. **The values a shipped table holds are content and must not be
transcribed** - the same rule that forbids hand-copying a `.pob` emitter into a
`const`. A struct page describes the shape; the game's own data supplies what is
in it.

**Prototypes** become a `prototypes.tsv` beside `names.tsv`, columns
`address`, `prototype`, `confidence`, `evidence` - the same five-field shape and
the same evidence rule as `names.tsv`, because a prototype *is* a claim about
the binary and belongs on ADR-0005's side of the line. Only a prototype for a
function the project named is eligible; Ghidra's libc recognitions are not
recorded, they are re-derived.

**Local variable names are declined.** They are scoped to one decompilation,
Ghidra renumbers them between imports, and the reimport that motivated this ADR
is exactly the event that invalidates them. A variable name worth keeping is
worth writing into the function's evidence page as prose, where it is durable
and where the reasoning that produced it can sit beside it.

**Comments are declined as a tracked artefact.** They stay in the database as a
convenience, and are captured when a database is snapshotted, but they are not
maintained as a separate replayable record.

The `0x008b79cc` case above is why, and it cuts against the intuitive answer. A
comment is a *copy* of a docs page's reasoning, placed where the reader is. When
the page is corrected the copy is not, and there is no mechanism that could
correct it on a machine the author does not have. Syncing comments would
therefore propagate stale claims as readily as fresh ones, and give them the
appearance of having been checked. The docs page is the record of truth; a
comment that disagrees with it is a defect in the comment, and the fix is to
reread the page, not to ship the comment further.

What the capture *does* provide is detection: `just check-captures` verifies
that every `docs/` page a captured comment cites still exists, which is how the
dangling `rocket-visuals.md` reference surfaced.

## Consequences

The immediate question is answered with a measurement instead of a format: there
was no struct backlog, `names.tsv` lost nothing, and the PSP databases can be
deleted. `docs/ghidra/captures/` is the evidence for that, and is re-derivable
by `scripts/ghidra/DumpDatabaseState.java` plus
`scripts/capture-ghidra-state.py` against any database that still exists.

The real backlog moves into view: `ps3-hdfury-eu` holds 411 user-defined names
with no `names.tsv` row and `ps2-pulse-eu` holds two. Those two databases are
not protected by the discipline that saved the PSP ones, and the capture makes
the gap countable for the first time.

The `just` gate grows one check. It is pure-Python, offline, and reads about
3,800 TSV rows and one Markdown table, so it costs a fraction of a second.

A capture goes stale by design, and nothing detects that. It records a database
at a moment; the database moves on. This is accepted because a capture's purpose
is historical, but it means a reader must take the date in the README seriously,
and it means a capture must be re-run before it is used to argue about a
database's present state.

Structs remain untracked in practice. If the first one is recovered and the
convention in `docs/ghidra/structures/` turns out to need machinery after all,
that is a new ADR with a real row to design against - which is a far better
position than the one this ADR was asked to design from.
