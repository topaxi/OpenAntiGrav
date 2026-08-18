# OpenAntiGrav documentation

Documentation is a first-class deliverable. The aim is that a contributor
arriving in two years can understand the engine, and the original game, without
reopening Ghidra.

Two rules hold everywhere in this tree:

1. **Record the evidence, not just the conclusion.** A claim without evidence is
   a guess, and guesses that look like facts are worse than open questions.
2. **State the confidence.** Every reverse-engineering claim carries a score
   from the [confidence rubric](reverse-engineering/confidence-rubric.md).

## Reading order

New to the project? Read these in order:

1. [Project goals](overview/goals.md) - what this is and is not
2. [Roadmap](overview/roadmap.md) - milestones and their exit criteria
3. [Glossary](overview/glossary.md) - terminology, including Wipeout-specific terms
4. [Legal](overview/legal.md) - the no-copyrighted-content policy
5. [Workspace layout](architecture/workspace-layout.md) - the crates and their boundaries
6. [Determinism](architecture/determinism.md) - the rule the whole engine is built around

Then, depending on what you are here to do:

| I want to... | Start at |
| --- | --- |
| Understand a design decision | [ADR index](architecture/adr/README.md) |
| Reverse engineer something | [Methodology](reverse-engineering/methodology.md) |
| Set up Ghidra and the emulators | [Toolchain](reverse-engineering/toolchain.md) |
| Decode an asset format | [Format index](formats/README.md) |
| Know what is on the discs | [PSP](psp/pulse-disc-layout.md) / [PS2](ps2/pulse-disc-layout.md) |
| Use the tools | [Tool docs](tools/README.md) |
| Package it for a Steam Deck | [Packaging](tools/packaging.md) |
| Run the game | [Front-end boot](architecture/frontend-boot.md) |
| Add or change a menu | [Menus](architecture/menus.md) |
| Verify against the original | [Verification protocol](reverse-engineering/verification-protocol.md) |

## Tree

```
overview/               goals, roadmap, glossary, legal
architecture/           how the engine is put together
  adr/                  architecture decision records
reverse-engineering/    methodology, verification, confidence, source images
ghidra/                 conventions and per-function documentation
  functions/            one page per analysed function, per binary
  memory-maps/          address space layouts
formats/                one page per asset format, plus a status index
psp/  ps2/              platform-specific findings
comparisons/            PSP vs PS2, and cross-title comparisons
gameplay/ physics/ rendering/ ui/ networking/
                        subsystem documentation, filled in as each is built
future-2048/            what carries forward to later titles
reviews/                dated whole-workspace code reviews, findings graded
tools/                  command line tool reference, and packaging
```

## Status

**[`overview/roadmap.md`](overview/roadmap.md) is the authority**; this page
carries a one-paragraph summary and nothing per-item, because a status kept in
two places is a status that drifts in one of them. It already had: this section
still said "milestone M0, nothing about the game's runtime behaviour has been
established yet" on 2026-08-18, several milestones and one booted HD front end
later, which made the first page a new contributor reads the most wrong one
(finding I5 of that day's review).

Where it actually stands: **M0-M1 (foundation, asset archaeology) and M4
(playable core) are complete**, M2 and M3 are complete in every part the work so
far has needed, and **M5 (full race), M6 (rendering fidelity) and M7 (shell) are
in flight**. A race runs, with lap timing, opponents, pickups and three
single-ship modes; three titles' discs open, and Pure and HD both reach a front
end. Directories for subsystems that do not exist yet contain a scope statement
rather than speculation.
