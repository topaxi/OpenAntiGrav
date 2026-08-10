# ADR-0023: The boot sequence is a measured table per title

## Status

Accepted. Supersedes [ADR-0022](0022-title-packages.md) **item 4 in part**: that
item's exclusion of front-end screen names from having an engine-side type no
longer holds for the boot sequence specifically. Items 1, 2, 3 and 5 stand
unchanged - and **item 3's refusal of a `trait Game` is upheld here, not
weakened**. Item 4 continues to govern every other presentation axis it names:
HUD atlas, mode set, animated textures.

## Context

ADR-0022 item 4 confined `oag-title` to axes with two measured corpora, and
said presentation vocabulary - naming front-end screens explicitly - "stays as
plain constants in `oag-pulse` with no engine-side type, because
`pure-status.md` measured none of it".

That reasoning carried one unstated assumption: that the engine would never have
to *choose* between the two titles' vocabularies. It does, at eight sites in the
composition root, and it chose by probing screen names:

- `boot.rs:492` - `load_fmv_intro` returns early unless `FMV Intro` exists
- `boot.rs:910` - `default_boot_movie` picks Pure's movie if `Title Screen` exists
- `frontend.rs:964-969` - `language_confirm_target` tries `Show Logo`, then `FMV Intro`, then `Title Screen`
- `frontend.rs:1373` - the picker draws a parent's fills if `Title Screen` exists, a *rendering* decision made by a title guess
- `main.rs:3151` - a feed swap keyed on `Event::Enter(FMV Intro)`
- `boot.rs:640` and `boot.rs:462` - Pure's fallback globals and Pulse's backdrop name, both applied to every source unconditionally

A ninth was mid-write - `Frontend::booting`'s initial `transition_to` - when this
was caught. Three of those probes ask "is this Pure?" three different ways, and
each is a fresh chance for the pair to disagree.
[The roadmap](../../overview/roadmap.md) names WipEout 2048 and HD/Fury on this
same engine, at five new branches each.

**The corpus condition item 4 set is now met on this axis, and by a stronger
method than it anticipated.** Both titles' boot sequences have been cold-booted
under PPSSPP and read off their own front-end XML, and the two measurements
disagree in a way that settles the design:

- **Pure**: `Skin.xml` declares picker-first, and the runtime agrees. The chain
  is `Language Selection` -> `Developer Publisher Screen` -> `MemoryStickWarning`
  -> `FMV Intro` -> `Title Screen`, with no movie before the picker. Confirmed on
  both pressings. See [`pure-boot.md`](../pure-boot.md).
- **Pulse**: `Skin.xml` *also* declares picker-first - and the runtime does not.
  A cold boot opens straight into `LogoFMV` playing `Data\Movies\Intro.PMF` and
  runs on to `Show Logo`, with no picker between them. See
  [`frontend-boot.md`](../frontend-boot.md).

**So a front-end XML's declared entry point is not the runtime's.** It holds for
one title and fails for the other. That kills every derivation strategy: the
sequence cannot be read out of the data, cannot be inferred from a screen-name
probe, and cannot be shared between titles by analogy. What is left is a
measurement, per title - which is exactly the thing ADR-0022 says belongs in a
title package.

## Decision

**A title's boot sequence is a table of measured facts, held in `oag-title`,
filled in by each title package, and consumed by one generic mechanism in
`oag-game`.**

1. **The sequence is an ordered chain of steps, not a set of decision points.**
   `oag_title::boot::BootProfile` holds `chain: &'static [BootStep]`, each step
   naming a screen and optionally the movie that screen plays. Pure's chain is
   five steps, Pulse's three. The earlier design considered here - a boot leg
   plus an `after_language` target - could not express Pure's two intermediate
   screens at all, and was discarded once they were measured.

2. **Plain data, not a trait.** ADR-0022 item 3 stands: no `trait Game`, no
   vtable, nothing dispatches per call. A `BootProfile` is a struct of tables
   that the composition root selects once at boot, the same way `Title` already
   is.

3. **The table names no engine type, and in particular not `Screens`.**
   `oag_game::screen::Screens` lives in the composition root and
   [rule 2](../workspace-layout.md) forbids any crate depending on `oag-game`, so a
   table that mentioned it could not exist. Where a source's own screen list
   matters - skipping a step a given pressing lacks - the mechanism passes a
   `has_screen` predicate and the screen model stays where it is.

4. **The profile is selected once, by serial, and never probed for.**
   `oag_game::title::open_source` already identifies the title from
   `UMD_DATA.BIN` before any XML is parsed, then discards that knowledge.
   Screen-name probing is what this ADR *removes*; it is not what it formalises.

5. **What a state does stays behaviour in `oag-game`.** The table says which
   screens, in what order, and which movies. It never says what a button means,
   how long a hold lasts, or how a screen draws. Encoding that would be a
   state-machine interpreter in `oag-title` - ADR-0022 item 3's rejected trait
   wearing a data hat.

## Alternatives considered

**A `trait TitleProfile` in `oag-game`, implemented by the title crates.** The
shape originally proposed for this work. Illegal: rule 2 forbids
`oag-pulse -> oag-game`, and it revives exactly the per-title dispatch item 3
refused.

**The same trait in `oag-title`.** Legal on the dependency graph, but its
`recognises(&Screens)` method needs the composition root's screen model, and
`Screens` cannot move down - it is a parser over front-end XML built on
`oag-game`'s own widget types.

**Keep the probes.** The status quo: five new branches per title, at four
decision points that already ask the same question three ways. The measurement
above also makes the probes actively wrong-headed, since the thing they probe
for (a screen's presence) does not determine the thing they conclude (the boot
order).

**Derive the order from the front-end XML.** Ruled out by measurement, not by
taste: Pulse's XML declares an entry point its runtime does not use.

## Consequences

**Good.** A new title's boot is one table. The eight probes and two unconditional
Pulse literals go. Facts that were previously implicit in control flow -
"Pure ships no menu backdrop", "Pure plays no movie before its picker", "Pulse's
picker is not between its movie and `Show Logo`" - become citable data with
tests. And the picker/movie ordering question that `frontend.rs`'s module docs
have carried as "the order asked for" becomes a one-table edit, which is what
makes a faithfulness pass affordable at all.

**Bad.** `oag-title` now holds front-end screen names, which item 4 explicitly
ruled out. The line between "sequence" and "presentation" is drawn by what the
composition root branches on rather than by principle - defensible, and movable.
`Title` grows a field that `oag-assets` receives and never reads. And it is
still n=2: the chain being an ordered list with skippable steps is designed from
two titles, and the third may not fit.

**Unresolved.** `Developer Publisher Screen` is in Pure's chain as a measured
fact, but Pure's `Skin.xml` declares no content for it - the screen is drawn by
engine code, so the table can name the step while nothing can yet draw it. That
is Ghidra work, tracked in `HANDOVER.md`, and it is the first case of a step
whose *existence* is evidenced well ahead of its *behaviour*.
