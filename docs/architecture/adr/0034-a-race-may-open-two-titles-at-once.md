# ADR-0034: A race may open two titles at once; nothing dispatches through either

## Status

Accepted. Clarifies [ADR-0022](0022-title-packages.md) rather than superseding
it - ADR-0022 never ruled on how many `Title`/`Archives` values may be alive in
one process, only on how a title is *reached*.

## Context

Race Remix (`crates/game/src/remix.rs`) lets a race load its track,
collision, environment, pads and weapon tuning from one title and its
livery, HUD, exhaust/flare, handling stats and grid roster from another - the
feature's own request was "pick race track of Wipeout 2048, pick craft from
Wipeout Pure".

The first reading of ADR-0022 made this look forbidden: "a `&'static Title`
is chosen once, by the composition root" reads, on a fast pass, like a
process-wide cardinality limit. Reading the ADR's actual decision rather than
a paraphrase of it settles the question the other way. Item 3 is explicit
about what it rules out - "no `trait Game`, no `enum Title` dispatch, no
plugin registry... a title package is a struct of tables that a composition
root selects once at boot; it is not a vtable that decode paths dispatch
through" - and every word of that is about the *shape of dispatch*, not
about a count. "Once" there is contrasted with *per call*, not with *per
process*.

Two facts already in the codebase before Race Remix confirm the reading:

- `Title` (`crates/title/src/lib.rs`) is a plain `Copy` struct with no global
  state anywhere in `oag-title`, `oag-assets` or `crate::title` - no
  `OnceLock`, no `thread_local`, no `static mut`. Two `&'static Title`
  references coexisting is exactly as unremarkable as two `&'static str`
  references coexisting.
- `crates/game/src/prefetch.rs` already holds a `Vec<oag_assets::Archive>` -
  multiple opened archive handles of *one* title - alive at once, with no
  special-casing anywhere in the archive-handle machinery. The mechanism
  that would need to forbid a second `Archives` simply does not check for
  one.

## Decision

1. **Any number of `&'static Title` values, and any number of opened
   `Archives`, may be alive in one process at once.** Nothing about "one" in
   ADR-0022 constrains count, and this ADR records that reading explicitly
   rather than leaving the next reader to re-derive it under a feature
   request.
2. **What stays forbidden, unchanged from ADR-0022 item 3: no code path
   chooses at runtime which title's *behaviour* to run through a shared
   abstraction.** No `trait Game`, no `enum Title` dispatch, no registry.
   Every `Title`/`Archives` pair a feature holds is still resolved to a
   concrete, statically-typed value by the composition root or something it
   calls (`crate::title::open_source`, or `crate::remix::Remix::open` for
   Race Remix), and every call site names explicitly which one it wants -
   `archives`/`title` for track content, `craft_of(&mut craft, &mut
   archives)`/`craft_title` for craft content in `race::load` - never
   "whichever one is current" through an implicit, thread-local or
   mutably-shared slot.
3. **A composition-root-level concern that needs more than one title stays
   beside `crate::title`, not inside `oag-title`.** `crate::remix::Remix`
   (an enum of one or two `Opened` values) lives in `crates/game`, the same
   layer `crate::title::Opened`/`open_source` already do, because it needs
   the same concrete knowledge of every title package that they do.
   `oag-title` stays exactly as thin as ADR-0022 item 4 already committed
   it to: it "covers only axes with two measured corpora", and "how many
   titles does this feature mix" has no measured corpus to generalise
   from - it is a fact about the feature, not about what a title is.

## Alternatives considered

**Treat ADR-0022 as forbidding this and design a new axis** - either a
"remix" pseudo-title inside `oag-title`, or an `enum Title` big enough to
carry a mix. Rejected on both counts: the pseudo-title route reintroduces
exactly the n=1 overfitting problem ADR-0009 named (a type designed from one
feature's shape), and the `enum Title` route is the literal dispatch
mechanism ADR-0022 item 3 already rejected by name.

**Wait for a fresh ADR before writing any code.** Considered during the work
itself and rejected once the actual text of ADR-0022 was read rather than
assumed - the question it raised had a direct answer already on the page,
and writing a new ADR before reading the one already governing the area
would have been process for its own sake, deciding a question nobody had
actually looked up yet.

## Consequences

**Good.** A feature that needs two titles' data live at once - Race Remix
today, something else tomorrow - has a precedent to build from without
relitigating ADR-0022 each time: "how many" and "how reached" are now two
separate questions, the same way `oag-formats`' console axis already
separated "which console" from "how a byte decodes" (ADR-0004).

**Good.** No engine-side type had to grow a field, a variant or a dependency
to carry this. `crate::remix::Remix` is entirely outside `oag-title`'s own
contract, so removing or reshaping Race Remix later costs `oag-title`
nothing.

**Bad, unresolved.** Nothing here says how many is *too many* for a given
feature. Race Remix uses two (track, craft); a hypothetical feature wanting
eight - one title per grid slot - is not ruled out by this ADR, but it is a
much larger change than this one licenses by itself: it would need a
per-`Ship` provenance field reconciled against
`crates/gameplay/src/hash.rs`'s exhaustive `write_ship` destructure, which
this ADR does not decide anything about.

**Bad.** The cost of opening N titles is real and unmeasured here.
`crate::remix::catalogue` opens a title's archives afresh on every menu
title-pick - fine at menu-interaction cadence, and would not be at any
higher frequency. Nothing in this ADR requires a caller to think about that
cost; a future feature that reaches for "just open another title" pays
whatever that title's archive-open cost is, silently, unless it caches
deliberately - see `crates/game/src/main/session.rs`'s `Session.titles`
doc comment for the one place Race Remix itself made that trade-off
explicit rather than hiding it.
