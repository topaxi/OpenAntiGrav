# ADR-0009: Fan out to other games by verified layer, not by date

## Status

Accepted.

## Context

This project is a Wipeout Pulse reimplementation first, and a foundation for
the wider Studio Liverpool lineage second (see
[goals](../../overview/goals.md) and roadmap M7). That raises a sequencing
question: when does work on a second title (Pure first, per the roadmap's
release order) begin? Two intuitions pull against each other:

- "Finish Pulse first" - a second title multiplies unverified surface, and
  the verification method has to be proven end to end on one game before it
  is worth replicating.
- "Architecture must be considered early" - retrofitting multi-game support
  onto a single-game codebase is the expensive version of the same work.

The 2026-07-28 sessions produced direct evidence about both. The PSP/PS2
split was effectively a dry run for a second game: because nothing branches
on platform in any decode path (discrimination is per-batch, archives
resolve by name against the source's own file list), adding the PS2 asset
path and even booting the PS2 front end cost days, not months. Meanwhile the
physics work demonstrated the opposite property on the simulation side:
every recovered law rests on constants that are code literals in one
executable (the inertia box, the friction pair, the damping gains) - none
of it transfers to another title. Only the *method* transfers: the trace
harness, the autopilot, the verification protocol, the confidence rubric.

Pure's disc is already in `data/images/` and already serves as a passive
cross-validation corpus (it identified the boot dev/pub reels as shared
assets, and format pages cite it).

## Decision

**Each layer fans out to a second game when that layer is verified for
Pulse - formats opportunistically now, the simulation only after M4's exit
criterion.**

Concretely:

1. **Container/format layer: fan out opportunistically, starting now.**
   Pointing `oag-unpack`/`oag-wad`/`oag-view` at Pure costs little and pays
   twice: every format claim gains a third validation corpus (the same
   mechanism by which the PS2 binary raised confidence scores across the
   docs), and any place Pure breaks a parser is a place the parser had
   silently overfit to Pulse. This is architecture testing, not scope
   creep. Failures are findings, recorded per the usual rules.
2. **Simulation layer: no second-game work before Pulse passes trace
   comparison for a full lap (M4's exit criterion).** The laws do not
   transfer; the harness does. Until the method is proven end to end on
   one game, replicating it multiplies unverified surface.
3. **No speculative game abstraction at n=1.** No `trait Game`, no
   per-game plugin architecture, until a second title's real shape forces
   one. Game-specific constants stay visibly namespaced as Pulse's.
   Interfaces designed from one example encode that example.
4. **The cheap hygiene that keeps the door open**: per-game data in the
   capture harness (breakpoint address, memory-offset table) stays
   config-shaped rather than inline, since that is the first thing a Pure
   effort would fork; and the two dependency rules in
   [workspace-layout](../workspace-layout.md) continue to apply unchanged.

## Alternatives considered

**Finish Pulse completely (M6) before touching any other title.** Rejected
as stated, because it forfeits free cross-validation: the format layer is
already effectively multi-game, and Pure data has already caught real
findings in Pulse work. The strict version survives for the simulation
layer only.

**Start Pure as a parallel first-class target now.** Rejected. The
verification harness exists for one emulator and one binary's offset
table; the physics method is mid-proof (M4 open). Doubling targets now
doubles the unverified surface exactly where the project is weakest.

**Design a game-abstraction layer up front.** Rejected per the n=1
argument above; this is the classic speculative-generality failure. The
PS2 experience shows the codebase grows the right seams when a real second
corpus arrives.

## Consequences

**Good.** Format code gets adversarial input early and cheaply. The Pure
fan-out cost becomes measurable long before anyone commits to it (a
smoke-test of Pure assets in `oag-view` is the cheap probe). The
simulation keeps a single, provable target. No premature interfaces to
maintain.

**Bad.** Pure support will look "started but stalled" from the outside -
assets browsable long before anything drives. The roadmap's M7 phrasing
("a second title boots and plays") stays the real gate, and this ADR does
not move it. And opportunistic format testing against Pure adds a second
corpus to keep in mind when writing format claims - pages should say which
discs a claim was validated against.
