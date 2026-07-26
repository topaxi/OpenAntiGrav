# ADR-0005: Ghidra naming and documentation conventions

## Status

Accepted.

## Context

A Ghidra project accumulates thousands of renamed functions. Without
conventions, two failures follow, and both are hard to undo.

**Names become claims without evidence.** Someone renames `FUN_08831af0` to
`Ship_UpdateSteering` based on a plausible reading. Six months later that name
is treated as established fact by everyone who reads it, including the person
who wrote it. The confidence that was in someone's head at the time is gone.

**The knowledge lives in the database.** The Ghidra project is a large binary
artefact that is not committed, cannot be diffed, and cannot be reviewed. If it
is the only record, the project's understanding is one disk failure from zero,
and a new contributor must redo the analysis to learn anything.

## Decision

### Never rename without a documentation entry

Every rename gets a page under `docs/ghidra/functions/<binary>/`, using
[the template](../../ghidra/function-template.md). It records address, binary,
purpose, arguments, return type, related systems, confidence, and the evidence.

The documentation is the record of truth. The Ghidra database is a working copy.

### Naming scheme

`Subsystem_VerbNoun`, PascalCase:

```
Ship_UpdateSteering
Wad_OpenEntry
Race_ComputeLapTime
Render_SubmitShipMesh
```

Subsystem prefixes group related functions in Ghidra's symbol tree, which is the
cheapest possible navigation aid.

### Confidence is encoded in the name

Below 70 on the [rubric](../../reverse-engineering/confidence-rubric.md), the
name gets a `_q` suffix:

```
Ship_ApplyAirbrakeDrag_q     // 65: plausible, not verified
Ship_ApplyAirbrakeDrag       // 88: call sites consistent, decompilation clear
```

The suffix is visible at every call site. A name is a claim, and a claim that
might be wrong should look like one wherever it appears, not only on its own
documentation page.

**Below 50, do not rename at all.** Leave `FUN_08831af0` and write down the
hypothesis. A guess dressed as a name is worse than no name, because it stops
other people from looking.

### Data too

Structures, fields and enums follow the same rules. A recovered structure gets a
page with the evidence for each field's offset and type.

## Alternatives considered

**Free-form naming, confidence in the docs only.** Simplest. Rejected because
the name is what people read, and reading a call site does not prompt anyone to
go and check a separate document.

**Ghidra bookmarks and comments as the record.** Keeps the annotation next to
the code. Rejected: it lives only in the uncommittable database, and it cannot
be reviewed or diffed.

**A numeric confidence in the symbol name,** such as `Ship_UpdateSteering_c88`.
More precise at the call site. Rejected as noisy; the binary
verified/not-verified distinction is what actually changes how much you should
trust a name.

**Export the Ghidra database to a committed text format.** Attractive, and worth
revisiting. Deferred because a full export is large and noisy, and hand-written
pages carry the reasoning that an export cannot.

## Consequences

**Good.** Every name has traceable evidence. Confidence is visible where it
matters. A contributor can learn the binary from `docs/ghidra/` without opening
Ghidra. The documentation is reviewable in pull requests.

**Bad.** Renaming is slower, which creates a standing temptation to skip the
documentation "just this once". Names change when confidence crosses 70, so
`_q` suffixes churn and cross-references need updating. And the documentation
can drift from the database, with no automated check that they agree.

**Mitigation for drift:** treat the documentation as authoritative. If the
database and the docs disagree, the docs win and the database gets corrected.
