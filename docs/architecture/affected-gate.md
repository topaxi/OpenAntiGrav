# The affected gate: what a change selects, and what it costs

A drive member gates its branch with `just gate-affected` and
`just test-data-affected`. Both run every check in full and only the tests
`scripts/affected.py` selects. This page records what was measured on
2026-11-10, the rules the selector follows, the case each rule could miss, and
what is still expensive. The lead's full `just` and `just test-data` per merge
batch run every test there is and are the safety net for every miss named here
([`workspace-layout.md`](workspace-layout.md), "Test-data suite measurements",
has the suite's own history).

Reproduce any table here from a full `just test-data` log:

```sh
python3 scripts/affected-replay.py target/test-data.log -n 50      # the last 50 merges
python3 scripts/affected-replay.py target/test-data.log --crates   # the per-crate table
```

## Where the seconds are

The log is this branch's full `just test-data --full` on 2026-11-10: 11,663 tests, 7,431 summed test seconds, 408 s of wall clock (load average 21 at the start, 8 at the end, with another member working), so about 18 tests ran at once.

Two packages hold two thirds of it, and they sit at the top of the dependency
graph, so nearly every change links into them:

| package | tests | summed s | share |
| --- | ---: | ---: | ---: |
| `oag-raceplay` | 2,301 | 3,306 | 44 % |
| `oag-game` | 1,877 | 1,662 | 22 % |
| `oag-mesh` | 378 | 585 | 8 % |
| `oag-texture` | 190 | 565 | 8 % |
| `oag-net` | 113 | 326 | 4 % |
| `oag-rcs` | 380 | 316 | 4 % |
| `oag-music` | 45 | 264 | 4 % |
| the other 51 | | 407 | 5 % |

`just test` (no disc data) ran its 8,795 tests in 38.8 s on the same tree, so
the affected gate's cost is the disc-backed half, and the numbers on this page
are that half's.

**Estimated wall clock.** A selection's wall clock is estimated as the largest
of three floors: its summed test seconds over the log's own parallelism (the
log's summed seconds over its wall clock), the `gpu-vram` group's summed
seconds over the group's cap of 4, and its slowest test. The estimate is for
comparing selections with each other, not a promise: wall clock on this
machine moves with what else is gating.

## Per crate: how often it changes and what a change selects

Commits are non-merge commits touching the crate's directory in the three
months to 2026-11-10. "Links it" is the number of other packages whose tests
link the crate. "Selects" is what a change to the crate's `src/` runs, in
summed test seconds, before and after this page's selector. Ranked by commits
times seconds selected after.

| crate | commits | links it | own tests | own s | selects before, s | selects after, s | after, est. wall s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `oag-raceplay` | 1,193 | 2 | 2,301 | 3,306 | 5,294 | 5,233 | 287 |
| `oag-game` | 2,580 | 0 | 1,877 | 1,662 | 1,662 | 1,628 | 195 |
| `oag-render` | 758 | 4 | 333 | 36 | 5,908 | 5,282 | 290 |
| `oag-title` | 468 | 30 | 74 | 1 | 7,394 | 7,191 | 395 |
| `oag-hd` | 413 | 12 | 87 | 2 | 7,393 | 5,328 | 293 |
| `oag-formats` | 285 | 39 | 238 | 25 | 7,431 | 7,338 | 403 |
| `oag-ui` | 359 | 8 | 522 | 9 | 7,393 | 5,278 | 290 |
| `oag-pure` | 343 | 13 | 50 | 12 | 7,393 | 5,311 | 292 |
| `oag-pulse` | 325 | 20 | 26 | 0 | 7,393 | 5,405 | 297 |
| `oag-2048` | 301 | 12 | 52 | 4 | 7,393 | 5,309 | 292 |
| `oag-assets` | 222 | 29 | 91 | 23 | 7,393 | 7,190 | 395 |
| `oag-mesh` | 263 | 7 | 378 | 585 | 5,923 | 5,902 | 324 |
| `oag-omega` | 270 | 7 | 23 | 1 | 7,393 | 5,279 | 290 |
| `oag-physics` | 217 | 16 | 431 | 3 | 7,431 | 5,347 | 294 |
| `oag-gameplay` | 217 | 9 | 94 | 1 | 7,431 | 5,281 | 290 |
| `oag-vex` | 153 | 26 | 367 | 99 | 7,393 | 6,217 | 341 |
| `oag-ui-screens` | 175 | 4 | 477 | 4 | 7,393 | 5,237 | 288 |
| `oag-tables` | 120 | 36 | 386 | 4 | 7,393 | 7,204 | 396 |
| `oag-ai` | 162 | 10 | 305 | 5 | 7,431 | 5,286 | 290 |
| `oag-rcs` | 119 | 9 | 380 | 316 | 7,393 | 6,218 | 341 |
| `oag-fx` | 121 | 6 | 370 | 8 | 5,916 | 5,301 | 291 |
| `oag-hud` | 116 | 3 | 335 | 8 | 5,302 | 5,241 | 288 |
| `oag-sound` | 114 | 3 | 146 | 17 | 5,311 | 5,267 | 289 |
| `oag-trace` | 89 | 3 | 157 | 6 | 5,300 | 5,239 | 288 |

What the table says:

- **The cost of a change is set by `oag-raceplay`, `oag-game` and `oag-net`.**
  Their tests are 5,294 of 7,431 summed seconds (71 %) of the suite, and a change to anything they
  link (which is nearly everything) selects all of it. No selection rule that
  follows the link graph goes below that floor for a change to a crate they
  link.
- **`oag-title` is hot and at the bottom.** It changed 468 times in three
  months (every per-title behaviour is a `Title` field), and the tests of 30
  other packages link it, the format and texture crates among them, because
  `oag-assets` takes a `&Title` to open a source and their tests open a disc
  through `oag-assets`.
- **The old selector over-selected through dev-dependencies.** It followed a
  dev-dependency edge like a normal one, so a change to `oag-hd` selected
  `oag-tables` (which dev-depends on it) and then everything that depends on
  `oag-tables`: 40 of 58 packages for any title crate.

## The rules, and what each could miss

Every rule narrows what runs. For each: why it is sound, and the case it could
miss. In every case the lead's batch gate runs the test that was skipped.

### 1. A dev-dependency edge ends at the package that has it

A test binary of `P` links `P`, `P`'s normal and build dependencies
transitively, and `P`'s dev-dependencies with theirs. A package that depends
on `P` links `P`'s library and never `P`'s dev-dependencies. So a change to
`X` selects `P` when `X` is in that set, and the edge is not followed past
`P`. Optional dependencies count when a feature switches them on, resolved as
`cargo test -p P` resolves them.

Checked against `cargo tree -p P -e normal,build,dev` for all 58 packages: the
script's set equals cargo's, except that it also follows target-specific
edges (`oag-disc` to `oag-thread`, a wasm-only edge), which only adds.

*Could miss:* a workspace-wide build unifies features, so in the lead's
`--workspace` run a package can link a feature of a dependency that it does
not link when built alone. A change reachable only through such a feature is
not selected for that package. The batch gate builds unified and runs it.

### 2. `oag-core` and the simulation crates select what links them

They used to select the full suite. Rule 1 already selects every test that
links the changed crate, which is every test that can observe a determinism
slip, and `check-determinism` and the determinism tests of the simulation
crates run on every such change. What is skipped is what cannot link the
change: for `oag-physics`, the format and texture crates' tests.

`scripts/affected.py` prints `core or simulation changed` for such a change,
and the lead runs the full gate at once after that merge instead of waiting
for the batch.

*Could miss:* nothing through the link graph. A test that reads a simulation
crate's source file by path would be missed, and none does.

### 3. A package is narrowed to the test modules that name a crate linking the change

Most crates link their integration tests into one binary, a module per file
(`tests/it.rs`). A test module can only call a crate its code names. So inside
a selected package, a module runs when its code, with comments removed, names
a crate that links the change: as `oag_x`, as `oag-x` in a string, through
`CARGO_BIN_EXE` (it spawns the package's binary) or through a path into `src/`
(it reads a source file). A module also carries what the binary's root and
the sibling modules it refers to (`super::x`, `crate::x`) name. The unit tests
of a package are one unit and always name the package itself.

This is what keeps a change to a hot crate off the tests that only sit beside
it: a test in `oag-texture` that opens a disc through `oag-assets` and never
names `oag_pulse` does not run for a change to `oag-pulse`, though the binary
links it.

*Could miss:* a macro of crate `A` that expands to a path into crate `B`,
used by a module that names only `A`, when `A` does not link `B` itself. No
macro in this workspace does that.

### 4. A change only under `tests/` stays in its package

No package links another package's test files. A changed test file selects
the modules that own it and the modules that refer to them; a changed binary
root selects the binary; a fixture or other data file under `tests/` selects
the whole package and, by the path search of rule 7, any other package whose
tests name it (`oag-schema` reads four crates' fixtures).

*Could miss:* a test file included from another package with `#[path]`. None
is, and a binary root that uses `#[path]` at all is kept as one unit.

### 5. A change only in a `#[cfg(test)] mod tests` under `src/` runs the unit tests

A file in a `tests` directory or a `tests.rs` under `src/`, whose parent
module declares it `#[cfg(test)] mod tests;`, is compiled into the unit-test
binary alone. It selects that package's unit tests and no dependent.

*Could miss:* nothing; a file not declared that way is treated as source.

### 6. A comments-only `.rs` change selects nothing

The file's text at the merge base and now are compared with comments removed
(line, block and doc comments; string, raw-string and character literals are
kept whole). If they are equal, no test can observe the change. `fmt-check`,
`lint` and `check-comment-history` still run in full. This is the comment
compact lane's whole diff.

*Could miss:* a test that asserts on a line number (`line!()`, a panic
location), since removing a comment moves lines; and a doc test, which no
gate runs (`cargo nextest` does not).

### 7. Files outside crates select who names them

As before: the path and its parent directories are searched for in every
package. New: a package that names the path only in its tests runs the test
modules naming it and selects no dependent.

### 8. Build inputs

| file | selects | why |
| --- | --- | --- |
| root `Cargo.toml`, `rust-toolchain.toml`, `.cargo/` | everything | profiles, lints and the compiler reach every test |
| `Cargo.lock` | the packages that link a locked package whose entry changed | a package's binary changes only if something in its resolved graph did; an entry the workspace cannot place selects everything |
| a crate's `build.rs` or `Cargo.toml` | that package in full and what links it | a build script runs for its own package |
| `.config/nextest.toml` | nothing | it schedules tests; `check-test-groups` runs in full |
| `justfile` | nothing | no test runs or reads a recipe (`oag-source` only asks whether the file exists); every check recipe runs in full |

*Could miss:* a `nextest.toml` change that makes a loaded run time out or run
out of GPU memory, and a recipe change that breaks how tests are invoked. Both
only show in a full run, which is the batch gate.

## The last 50 merges, replayed

Each merge's diff against its first parent through the old selector and the
new one, priced with the same log. The diffs are historical; the workspace,
the test files and the timings are today's.

| | before | after | change |
| --- | ---: | ---: | ---: |
| tests selected, median | 8,218 | 6,425 | -22 % |
| tests selected, mean | 6,587 | 5,277 | -20 % |
| summed test seconds, median | 6,795 | 5,949 | -12 % |
| summed test seconds, mean | 4,923 | 4,614 | -6 % |
| estimated wall seconds, median | 373 | 327 | -12 % |
| estimated wall seconds, mean | 274 | 258 | -6 % |
| worst merge (tests, summed s, wall s) | 11,663, 7,431, 408 | 11,663, 7,431, 408 | 0 |
| merges selecting the full suite | 5 | 1 | |
| merges selecting nothing (docs only) | 13 | 13 | |

By what the merge changed (summed test seconds, before to after):

| merge | changed packages | before | after |
| --- | --- | ---: | ---: |
| `4d5e00ae2` | mesh | 5,923 | 5,902 |
| `fe81bfc9b` | mesh, post, raceplay, vex | 7,409 | 6,233 |
| `82af4631d` | game, thread | 7,408 | 7,311 |
| `756cb6037` | 2048, ai, game, hd, omega, physics, pure, raceplay, title, trace | 7,431 | 7,203 |
| `31c22bbdc` | game | 1,662 | 1,662 |
| `833195e5f` | mesh, raceplay, vex | 7,393 | 6,217 |
| `85f7377fe` | mesh, post, shader-check | 5,946 | 5,922 |
| `f662ce468` | mesh, pulse, raceplay, render, title | 7,394 | 7,191 |
| `8a3e17ef4` | gpu, hd, mesh, pulse, raceplay, shader-check, title | 7,412 | 7,210 |
| `59900b483` | post | 5,928 | 5,253 |
| `95ca568b1` | fx, music | 6,197 | 5,599 |
| `2a244f1ac` | present | 5,298 | 5,237 |
| `5feb0a961` | fx, game, mesh, post, present, render, shader-check | 5,943 | 5,922 |
| `b199d56b6` | display, fx, game, hud, mod, pulse, raceplay, replay, schema, title, t | 7,394 | 7,194 |
| `388e7bdb3` | display, game, post, pulse, raceplay, title, ui | 7,410 | 7,207 |
| `970fe6fa1` | audio, disc, game, mod, modhost, raceplay, source, texture, thread | 7,431 | 7,431 |
| `5d5485015` | game, hud, post, pulse, title | 7,409 | 7,207 |
| `6bf65111d` | fx, raceplay, title | 7,394 | 7,191 |
| `3c64f0c5f` | 2048, hd, omega, physics, pulse, pure, raceplay, title, weapons | 7,431 | 7,203 |
| `00f49064e` | 2048, game, hd, hud, omega, post, pulse, pure, raceplay, render, title | 7,409 | 7,207 |
| `bba827728` | 2048, fx, game, hd, hud, livery, mod, omega, pulse, pure, raceplay, rep | 7,394 | 7,269 |
| `1b946feb2` | fx, pure, raceplay, title | 7,394 | 7,191 |
| `de6c29e52` | (test files only) | 5,923 | 3 |
| `373c562d1` | game, raceplay | 5,294 | 5,233 |
| `045b6b91b` | livery, mesh, pure, raceplay | 7,393 | 5,976 |
| `572a3fd9d` | fx, hd, pob, raceplay, title | 7,394 | 7,191 |
| `d77237d40` | 2048, ai, game, gameplay, hd, omega, pure, raceplay, title, trace | 7,431 | 7,199 |
| `4091cfd3e` | fx, game, hud, mod, pulse, raceplay, replay, schema, title, trackfile | 7,394 | 7,193 |
| `d015381b1` | 2048, game, mod, omega, pulse, raceplay, title, ui, ui-screens | 7,431 | 7,192 |
| `5ea385413` | game | 1,662 | 1,628 |
| `00f3b211f` | game, mod, pulse, raceplay, title, ui, ui-screens | 7,394 | 7,192 |
| `1ea6f3a07` | 2048, game, omega, pulse, raceplay, title, ui | 7,394 | 7,191 |

**The median merge selects 12 % fewer test seconds (22 % fewer tests) less, not the 40 % this work aimed
for.** The reason is the floor above: 26 of the 50 merges
changed `oag-raceplay`, a title crate, or something else `oag-raceplay` and
`oag-game` link, and for those the selection cannot drop below those two
packages' own tests. What the new selector removes is everything that was
selected without linking the change.

## Tests moved to the crate they test

73 of the 105 test modules in `crates/render/tests/` never named `oag_render`: they build and draw through `oag_mesh` alone, left behind when the mesh pipeline became its own crate. They are now `crates/mesh/tests/` (215 tests, 535 summed seconds, no test body changed; `cargo nextest list` over both packages names the same tests before and after, plus `oag-mesh`'s own `every_test_file_is_listed`). `oag-render` changed 757 times in three months and `oag-mesh` 262. The two helpers `archive_cache` and `track_albedo` are in both crates, because six modules that stay use them.

The selector finds such modules by rule 3: a module that does not name its own package. 32 stay in `oag-render` because they name `oag_render`, a title crate, `oag-tables` or `oag-post`, none of which `oag-mesh` may depend on.

## What would lower the floor

Measured, not built. Each needs a decision or a lane of its own.

- **`oag-assets` links `oag-title`.** With that one edge gone, a change to
  `oag-title` and the five title crates selects 5,681 summed seconds instead of 7,191 (`oag-texture`, `oag-rcs`, `oag-mesh` and `oag-vex` drop out). `Source::open`
  takes a `&Title` and reads its archive candidates, foreign serials and
  add-ons; ADR-0022 puts that knowledge there, so cutting the edge is a new
  ADR, not a refactor.
- **`oag-raceplay`'s simulation tests run for a change to the renderer.** The
  crate's `view` feature already splits its simulation half from its drawing
  half, and 80 of 220 (at most 921 summed seconds, from a first `cargo check -p oag-raceplay --no-default-features --tests`) of its integration test modules compile without it.
  Marking the others `#[cfg(feature = "view")]`, with a gate check that the
  rest still compile without the feature, would let a change reached only
  through `view` skip them.
- **The tests' own cost.** Everything else on this page selects among
  5,294 of 7,431 summed seconds (71 %); shrinking that sum is the larger lever, and
  [`workspace-layout.md`](workspace-layout.md) ("Test-data suite
  measurements") is how it has been done before.
