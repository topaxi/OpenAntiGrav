# ADR-0051: the composition root splits below itself, and the build-time argument does not survive a second time

## Status

Accepted; extends [ADR-0050](0050-format-crates-split-by-format-family.md).

## Context

[ADR-0050](0050-format-crates-split-by-format-family.md) split `oag-formats`
and closed by naming `oag-game` as "three times the size of `oag-formats` and
where the time actually goes". Read as a prediction that splitting it would make
the gate faster, **that is wrong, and this ADR is where the correction goes** -
ADRs here are immutable, so the record is amended by a new one rather than
edited.

`oag-game` was 106,859 source lines across ~49 top-level modules, plus 76
integration test binaries. Nothing depends on it: rule 2 of
[workspace-layout](../workspace-layout.md) makes it the composition root. So
every second it costs is its own.

## The measurement

Warm cache, after a semantic edit (a `pub const` appended to a source file, not
a `touch` - `touch` leaves metadata byte-identical and downstream rustc
short-circuits, understating by about 2x):

| scope | wall |
| --- | --- |
| `cargo build -p oag-game` - lib and bin only | **2-7s** |
| `cargo nextest run -p oag-game --no-run` - lib, bin and 76 test binaries | **30-59s** |

Repeated twice, consistent both times. **Linking the test binaries is 85-93% of
the cost of touching `oag-game`.** The library itself compiles in seconds.

That is the same shape ADR-0050 found for `oag-formats`, and it has the same
consequence: **a crate boundary does not move this number, because the test
binaries link wherever they live.** Extracting `oag-ui` creates
`oag-game -> oag-ui`, so an edit to `oag-ui` still invalidates everything that
stayed behind. The only saving runs the other way - an edit to `race` no longer
recompiles the UI's lines - and it is a saving in *library* compile time, which
is the small half.

The lever on the large half is the number of integration test *targets*, and it
is orthogonal to the split. It is also orthogonal to the earlier work that
divided slow matrix tests into more `#[test]`s: `cargo nextest` runs every test
in its own process regardless of which binary it was compiled into, so
consolidating binaries does not undo that.

## Decision

**Split `oag-game` downwards, and justify it on legibility rather than on build
time.** Three cuts, in dependency order:

1. **`oag-display`** - the picture's configuration vocabulary (`display`) and
   the coordinate space a source authors in (`space`). It was the most-named
   module in the composition root, and a shared vocabulary living inside one of
   its own consumers is not shared, it is borrowed. It had to go first: every
   other candidate names it.
2. **`RaceSim` / `RaceView`** - `Race` was a 96-field struct with 18 `impl Race`
   blocks across 18 files, interleaving `World` and `CollisionWorld` with fields
   its own doc comments labelled render-only, including an
   `oag_render::camera::shake::Shake`. It is now two structs. `state_hash` moves
   onto `RaceSim`.
3. **`oag-ui`** - the front-end and menu cluster: `frontend`, `menu`, `screen`,
   `font`, `anim`, `marquee`, `prompt`, `placeholder`, `language`, `strings`,
   `state_machine`, about 15,300 lines. `hud` is deliberately **not** in it;
   see below.

Together these take `oag-game` from 106,859 source lines to 86,496 - a fifth of
the composition root, and the fifth that had the least to do with composing.

**`oag-ui` is classified in `NOT_TITLE_PACKAGES`, not `GAMEPLAY_CRATES`.** The
cluster names `oag_title` about 73 times, and
`scripts/check-dependency-rules.py` requires any member that does to be in one
list or the other or it fails as unclassified. The choice is mutually exclusive
and permanent in effect: `NOT_TITLE_PACKAGES` lets `oag-ui` link a renderer and
forbids anything gameplay-side from depending on it. That matches what the code
already is - menus draw.

## Alternatives considered

**Split `oag-game` for build time.** Rejected on the measurement above. Stated
explicitly because ADR-0050 implied it and a reader who acts on that implication
will spend a week for two seconds.

**Consolidate the integration test binaries instead, and stop there.** This is
the lever that actually moves the measured cost, and it is not rejected - it is
*separate*, and it does nothing for legibility. A 106k-line composition root is
a reading problem whether or not it is a linking problem.

**Move `race` down into `oag-race`.** Its 27k lines are a quarter of the crate
and the obvious target. Rejected for now, and the reason is the God struct: no
file-level partition survives contact with a 96-field type whose `impl` blocks
are spread over 18 files and which holds a renderer's type as a field. Splitting
that struct is what this ADR does instead, and it is what makes the move
thinkable later. What did move down is what was already free-standing:
`race::sight` and `race::recovery`, into `oag-race`.

**Put `hud` in `oag-ui`.** Rejected. `hud/draw.rs` is real wgpu -
`RenderPipeline`, `BindGroup`, `CommandEncoder` - and `hud` was in a cycle with
`race` through `race::sight`. The documented M5 plan called `oag-ui` "HUD and
menus"; the measurement says those are two things, and only the menus half is
extractable as one move.

## Consequences

**Good.** `race.rs` went from 999 lines to 485. `state_hash` now lives on a
struct that cannot name a renderer, so the determinism boundary is enforced by
the type system rather than by a doc comment. Moving `race::sight` broke both
the `race <-> audio` and `race <-> hud` module cycles, which were almost
entirely that one type. `oag-display` is nameable from either side of the
simulation boundary without either side linking the other's machinery.

**Bad, and worth stating plainly.**

*The gate is not faster to build.* Same as ADR-0050, for the same reason, now
measured twice on two different crates: this project's *build* cost is dominated
by integration-test linking, and no crate boundary addresses that. **See the
amendment below - the gate did get faster, by a route this decision did not
anticipate.**

*The `Race` split is a redesign, not a move,* and it carries regression risk a
file move does not. The evidence it is behaviour-preserving is that
`race::hash`'s body has zero non-comment changes and still compiles against
`RaceSim` - which means the compiler, not a test, proves the hash reads only
simulation state. `crates/core/tests/determinism.rs` cannot guard this hash
directly, because rule 2 forbids the dependency that would let it.

*Rustdoc warnings are a bad ledger, in two different ways.* A link repointed at
a wrong-but-*existing* item is silent: thirteen became `[`RaceView::exhaust`]`
where the doc sat on `Scene`'s own `exhaust` field - resolvable, wrong referent,
no warning at all. And a link that *is* broken moves house with its file, so a
warning at `crates/game/src/language.rs:50` reappears at
`crates/ui/src/language.rs:50` and reads as a regression when it is
pre-existing debt at a new path. Comparing counts gets both wrong; comparing
warning *text* gets the second wrong. The only comparison that works is by
`file:line` **and** text together, and even that cannot see the first class -
for which there is no substitute for reading each rewritten link.

*A survey of forbidden dependencies is not a survey of coupling.* The module
list for `oag-ui` was assembled by checking which modules touch `wgpu`,
`winit`, `oag-render`, `oag-audio` and `oag-input`. That found nothing in
`frontend` or `menu` - and missed that `frontend` structurally *owns* a
`movie::Player` and that `menu::frame` needed a `sprite::Sheet`, neither of
which is a forbidden type and both of which sat in modules that were staying
behind. The fix was to hoist the two self-contained pieces (`Player`, a frame
counter with no decoding in it, and `Placed`, plain data) and to pass
`menu::frame::read` a slice of placements rather than the sheet. The general
form: "does this module name a renderer" and "can this module leave" are
different questions, and only the first one greps cleanly.

*The extracted crates each add a silent-failure surface,* the same three
ADR-0050 lists: a missing `[profile.dev.package.oag-*] opt-level = 2` entry, a
missing row in `check-transcendentals.py`'s `SCANNED_CRATES`, and a stale
path-keyed row in `check-file-size.py`'s `BASELINE`. Adding `display` to
`SCANNED_CRATES` immediately found a `tan` in `Fov::apply` that had never been
scanned - allowed, with the reason recorded, but it had been invisible for as
long as it lived in `oag-game`.

## Amendment, 2026-09-09: the splits made `just test-data` 15% faster, and the reason was in the build profile all along

The Consequences above say the gate is not faster, on the strength of two
compile-and-link measurements. That was the wrong measurement to stop at.
Running the full `just test-data` suite on an idle machine, everything
pre-built, either side of the day's work:

| commit | tests | suite wall clock |
| --- | --- | --- |
| `6120c207`, before the splits | 4,114 | **488s** |
| `7983eda3`, after them | 4,117 | **415s** |

**Three more tests, 73 seconds faster.** The mechanism is not subtle once seen,
and it is not the one this ADR or ADR-0050 reasoned about: `oag-game` is
deliberately left at `opt-level = 0` (see the build-profile table in
[workspace-layout](../workspace-layout.md)), and the roughly 18,000 lines that
moved out of it into `oag-ui`, `oag-display` and `oag-race` are all in crates
that are `opt-level = 2`. Code that leaves the composition root gets optimised.

So the general statement is sharper than either ADR made it. Splitting a crate
buys nothing in compile or link time, because the test binaries link wherever
they live. Splitting the **composition root specifically** buys test *runtime*,
in proportion to how much executable code leaves an unoptimised crate - and in a
suite that is tail-bound on a handful of long simulation tests, that is the cost
that was actually being paid.

This does not change the decision. It changes what the decision is worth, and it
means the "approximately zero" framing carried forward from ADR-0050 should not
be quoted for the composition root without this table beside it.

Two collateral corrections, both recorded in `scripts/check-test-budget.py`:

- **The 344s figure that CLAUDE.md and this project's ceilings were calibrated
  against is not reproducible.** The pre-split commit measures 488s idle on the
  machine in question. The ceilings now cite the two runs above.
- **Suite wall clock is contended and must be measured on an idle machine.** The
  same tree measured 456s, 402s and 415s at load averages of 21, 17 and 2, with
  four, two and one tests over the per-test ceiling and a *different* test each
  time. Two agents independently read a contended run as a regression on the
  same afternoon; the tests involved measure 76-79s in isolation, against the
  95-114s recorded when the ceiling was set.
