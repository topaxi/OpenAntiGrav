# ADR-0022: Title packages, and title as a data axis rather than a code axis

## Status

Accepted. Supersedes [ADR-0009](0009-multi-game-fanout.md) **item 3 only**;
items 1, 2 and 4 stand unchanged.

**Item 4 below is itself partly superseded by
[ADR-0023](0023-boot-sequence-as-title-data.md)**, which gives the boot sequence
an engine-side type in `oag-title`. Item 4 continues to govern every other
presentation axis it names.

**Item 3's "once" is clarified, not weakened, by
[ADR-0034](0034-a-race-may-open-two-titles-at-once.md)**: it was always a rule
against dispatch, not against count, and ADR-0034 is where that reading is
made explicit for a feature (Race Remix) that holds two title packages live at
once with no dispatch through either.

## Context

ADR-0009 item 3 refused a game abstraction "until a second title's real shape
forces one", on the ground that "interfaces designed from one example encode
that example". That was the right call at the time, and its own precondition has
since been met. The ADR-0009 probe ran, and
[`pure-status.md`](../../formats/pure-status.md) is the measurement: Pure's
`.vex` class-ID space is renumbered wholesale and keyed by a version word of 4 or
3 against Pulse's 6, its v3 batch header is shorter, its model textures ship
pre-swizzled, its `WO Track` payload is version `0x103` against Pulse's `0x105`
with header `+0x18` zero rather than one, its handling stats carry nine teams and
five speed classes against Pulse's eight and four and lack three Pulse-era
fields, it ships three archives rather than four, and it has no `SBlk` at all.

So the second example exists, and the places the format layer overfits to Pulse
are countable rather than hypothetical.

Two further facts shape what the abstraction should be.

**The console axis already solved this problem, and solved it by not being an
axis.** [ADR-0004](0004-asset-pipeline.md) decided "normalise at the boundary;
nothing downstream branches on platform", and the code honours it:
`oag-formats` has no `Platform` in its API at all, and every PSP/PS2
discrimination is made per blob - a WAD entry is compressed or it is not, a
`.vex` batch header is a VIF packet or it is not, a font carries an embedded
atlas or the next entry is its atlas. ADR-0009's own context records the payoff:
"adding the PS2 asset path and even booting the PS2 front end cost days, not
months". `race.rs` states the rule in one line: *"'which disc is this' is never
the right question to ask."*

**Pulse's fingerprints are nonetheless spread across nine crates** - archive
paths and entry names, `TEAMS[8]`, HUD atlas and layout paths, front-end screen
names, the animated-texture table, Zone scoring, `START_LINE_OFFSET`, and around
150 RE-derived physics literals. A contributor starting Pure work today has
nowhere to put anything, and no way to avoid editing Pulse's constants to do it.

## Decision

**Title is a data axis, not a code axis. Two questions are separated, and each
is answered by the thing that actually knows the answer.**

1. **"How does this byte stream decode?" is answered by the file.** Class-ID
   tables, header shapes and schema variants stay inside `oag-formats`, selected
   from the artifact's own version word - `vex::classes::{V6, V4, V3}` and the
   equivalents for `collision`, `track` and `handling`. The decoder never learns
   which title a file came from, and no caller has to tell it. This is
   `pure-status.md`'s own phrasing ("keyed off the file's own version word")
   taken literally, and it is what keeps the format crate free of a dependency on
   any title package.
2. **"What does this title ship?" is answered by a title package.** Archive
   candidate lists, entry names, name hashes, team lists, HUD and front-end
   names, language plugins: `oag-pulse` and `oag-pure`, sibling crates of plain
   tables. `oag-title` holds the engine-side *types* those tables fill in.
3. **No `trait Game`, no `enum Title` dispatch, no plugin registry.** The
   n=1 objection is answered for the format layer, not repealed in general. A
   title package is a struct of tables that a composition root selects once at
   boot; it is not a vtable that decode paths dispatch through.
4. **`oag-title` covers only axes with two measured corpora** - archive
   candidates, entry names, handling schema version. Presentation vocabulary
   (HUD atlas, front-end screen names, mode set, animated textures) stays as
   plain constants in `oag-pulse` with no engine-side type, because
   `pure-status.md` measured none of it. Extending `oag-title` there would be
   exactly the failure ADR-0009 item 3 named, and this ADR does not license it.
5. **No `oag-psp` / `oag-ps2` crates, ever.** ADR-0004 stands. The console axis
   is per-blob discrimination and asset normalisation, and turning it into a code
   axis would forfeit the property that made the PS2 fan-out cheap.

Item 2 of ADR-0009 is untouched: no second-title *simulation* work before Pulse
passes a full-lap trace comparison. Accordingly the physics, race-rule and
gameplay constants get their seam defined - a params boundary through which
every RE-derived constant enters the solver - but do not move crates until that
gate clears. The force law is the live blocker; relocating its constants
mid-investigation is the one part of this change with real downside and no
compensating benefit.

## Alternatives considered

**Keep ADR-0009 item 3 as written and wait longer.** Rejected. Item 3's
condition was a second title's real shape, and it now exists in measured form.
Waiting further does not buy more evidence; it buys more Pulse constants
scattered into more crates, each one a merge conflict for the Pure work the
roadmap already commits to.

**Move the class-ID tables into the title packages.** Rejected, and this was the
plan's first draft. It forces one of two broken shapes: `oag-formats` depending
on both title crates, which is a dependency cycle since they depend on it; or
every caller injecting a table, which means `oag-view`, `oag-render`,
`oag-assets` and `oag-trace` each have to know which title a file came from -
the forbidden question, reintroduced at four call sites. A class-ID table is a
property of a file format version, and the version is in the file.

**A `trait Game` with per-title implementations.** Rejected for the reason
ADR-0009 gave and this ADR does not disturb: the seams that matter are tables
selected by data, not behaviour selected by type. A trait would make every
decode path generic over a parameter that only two constants ever differ in, and
would put a dispatch boundary exactly where the console axis proved none is
needed.

**Split by console instead of by title** (`oag-psp`, `oag-ps2`). Rejected. The
survey found the console axis is already clean below `oag-game`, and the user's
own reading - that Pulse differs by assets rather than by platform - is a
measured property of this codebase, not an aspiration.

## Consequences

**Good.** Pure work and Pulse work stop competing for the same files. A Pure
contributor's day one is "fill in a table", and `just check-deps` stops them if
they reach for the renderer. The format layer's overfitting becomes visible as
missing table entries rather than as silent wrong answers. The two defects the
survey turned up - a PS2 music path constant that is right for exactly one
pressing, and a `DEFAULT_TRACK` duplicated between the game and the trace tool -
are fixed on the way past, because consolidation is what exposed them.

**Bad.** Three new crates to keep in the dependency rules, and
`scripts/check-dependency-rules.py` has a hardcoded crate set that must be
updated or rule 1 silently stops covering them. The version-word dispatch adds
an indirection to every decode path that previously read a `const` directly, for
a benefit that is invisible until a second title is loaded. `oag-pulse` will for
a while be a grab bag of constants that have nothing in common except the disc
they came from - which is honest, but reads as a dumping ground. And the physics
deferral means the split is visibly half-done until M4 closes: the seam exists,
the constants are still in the engine, and anyone reading `oag-physics` will see
Pulse addresses in a crate the layout table calls engine.

**Unresolved.** Whether HD/Fury and 2048 fit this shape at all. Both ship PSARC
archives rather than WAD, and 2048's content arrives as a PKG rather than a disc
filesystem, so the content-source layer must not assume "disc image plus WAD".
That much is designed for. Whether their geometry and handling formats are
recognisable descendants is unmeasured, and this ADR does not guess.
