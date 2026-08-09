# Handover: the engine/title split

Scoped to one effort, and meant to be **deleted when it is finished**. The
durable record is [ADR-0021](docs/architecture/adr/0021-title-packages.md),
[workspace-layout](docs/architecture/workspace-layout.md) and the
"engine/title split" section of [HANDOVER.md](HANDOVER.md). This file is the
part that only matters until stages 4, 6 and 7 land.

Branch: `split/title-packages`, six commits, `just` green at each.

## The one rule that decides every question this raises

Two questions were being conflated. Keep them apart and most decisions answer
themselves:

| Question | Answered by | Lives in |
| --- | --- | --- |
| How does this byte stream decode? | **the file**, from its own version word | `oag-formats` |
| What does this title ship? | **a title package** | `oag-pulse`, `oag-pure` |

`oag-formats` must never depend on a title crate. The reverse edge exists, so
that one is a cycle, and the alternative - callers injecting a table - puts
"which disc is this" back at four call sites, which
`crates/game/src/race.rs:1421-1423` forbids in one line.

This is not a taste argument. `Data\Defaults\Skycube.vex` is a **version-4 file
on the Pulse PSP disc** using version-4 numbering (`0x373` texture, the same id
`pure-status.md` measured across 156 Pure files). Title-keying would have been
wrong on a file Pulse itself ships.

## Done

| Stage | What landed |
| --- | --- |
| 1 | ADR-0021 (supersedes ADR-0009 **item 3 only**); title/console/host vocabulary; workspace-layout crate table |
| 2 | `oag-title`, `oag-pulse`, `oag-assets::source` taking a `&Title`; 34 call sites; two byproduct defect fixes |
| 3 | `vex::classes` + `classes_of`; `fog`/`pvs`/`collision`/`track` read ids off the file; `handling.rs` accepts a five-rung ladder and four optional fields |
| 5 | `oag-pure`, and three ground-truth tests proving Pure's disc opens through the same mechanism |

Stage 5 was taken before 4 on purpose: it unblocked the real-file evidence the
handling change was missing.

## Left to do

### 0. Enumerate Pure's `handlingstats.xml` element set — do this first

**Cheapest task here and it prevents the most waste.** Pointing the parser at
Pure's real file found schema differences *one at a time*, each visible only
after the previous was fixed: five rungs, three absent attributes, no `<FE>`,
no `<pitch>`. There are probably more behind `<pitch>`.

Diff Pure's `<Stats>` and `<Class>` children against Pulse's in one pass and
write the result into [`pure-status.md`](docs/formats/pure-status.md). Do not
discover them one panic at a time.

The method lesson, already recorded on that page: its per-layer confidences came
from **surveys** - counting nodes, closing sizes - and **a survey cannot see an
element that is simply not there.** The 88s and 92s were never wrong about what
they measured; they could not measure absence.

### 1. Stage 4 - presentation tables to `oag-pulse`

Move as plain `pub const`s, with **no engine-side type in `oag-title`**:
`render::mesh::ANIMATED_TEXTURES` (`mesh.rs:171-190`), `render::loading`'s wave
model, `game/src/hud.rs` atlas + layouts, `frontend.rs:88-124` screen names and
`:79-83` reel frames, `loading.rs`, `language.rs`, `boot.rs` movie names,
`race.rs`'s `DEFAULT_TRACK`/`DEFAULT_TEAM`/ship-model templates.

`oag-title` stays scoped to the three axes with two measured corpora. Adding a
`HudAtlas` or `ScreenNames` type would be designing from one example -
`pure-status.md` measured none of Pure's presentation - which is exactly what
ADR-0009 item 3 named and ADR-0021 does not license.

Fold in the one named console leak: make `oag_game::frontend::SCREEN`
per-source rather than a hardcoded 480x272 (HANDOVER.md:452).

**Oracle:** determinism reference hashes in `crates/core/tests/determinism.rs`
must not change, plus `just play --screenshot` on a PSP and a PS2 source.

### 2. Stage 6 - the physics seam, and *not* the relocation

Define the params boundary; **leave every constant where it is.**
`crates/physics/src/params.rs:31-49` is most of the way there. Same for
`gameplay/src/handling.rs:59-101`, `race/src/zone.rs`, and
`race/src/course.rs:139` `START_LINE_OFFSET`.

ADR-0009 item 2 gates second-title simulation work behind M4's exit, and the
force law is that milestone's live blocker. `<pitch>` is queued behind this too.

**Oracle:** determinism hashes unchanged **and** `just trace-compare` against
`verification/scenarios/talons-junction-time-trial-lap.inputs` diverging at the
same tick. A shifted divergence point means the seam changed a value it was
meant to pass through.

### 3. Stage 7 - docs and tidy

- A **title** column in both status tables in `docs/formats/README.md`. They
  have a `Platforms` column and no title column, so every row is implicitly
  Pulse.
- Ten test call sites still spelling `n.class_id == vex::CLASS_WO_TRACK` by
  hand; `track::find_node(file, nodes)` exists for them. Cosmetic - they are
  Pulse-only tests - but a regex sweep over this needs care, see traps below.
- Delete this file.

## Traps, each of which cost something

**A table read from data collapses two different failures into one.** "Nothing
matched" and "I could not read the file" become the same empty result unless you
are deliberate. This bug shape appeared three times in stage 3:

- `collision::from_vex` returned `Ok(empty)` when `classes_of` failed, swallowing
  a malformed file as "authors no collision". `a_vex_walk_failure_is_propagated_rather_than_swallowed`
  caught it. It propagates now.
- The `pvs` fixtures in `oag-formats` and `oag-render` passed a bare run of
  payloads as `data`, which is documented as *the whole `.vex` file*. That was
  always a fiction; nothing read the header, so nothing noticed.
- `Stats::class` indexed by `SpeedClass` discriminant and returned Pure's
  `ALPHA` when asked for `Venom`. It matches on the rung's own name now. **A
  positional assumption is exactly what does not survive a second ladder.**

**Do not make `vex::textures` read `Texture` payload `+0x06` unconditionally.**
The precondition was checked and it failed: bit 0 is set on **88 of 5,375**
`Texture` nodes on the Pulse PSP pressing and **120 of 8,972** on the PS2 one,
and they are ship liveries and weapon effects, not font atlases. Gate on version
word <= 4, or settle what bit 0 means first. What a histogram cannot say:
whether `pure-status.md`'s Pulse claim is wrong, or whether bit 0 is not the bit
that claim means. Decoding one flagged Pulse texture both ways would separate
them; unstarted. See `crates/formats/tests/texture_swizzle_flag_ground_truth.rs`.

**`scripts/check-dependency-rules.py` validates every name in `GAMEPLAY_CRATES`
against `cargo metadata`.** A crate cannot be listed ahead of its own creation
the way `oag-audio` once was; add it in the change that creates it.

**Regex sweeps over call sites need a compile between steps.** Two went wrong
here: one produced `oag_assets::oag_assets::`, another `Layout::resolve(&image.display(, TITLE)`
by matching `[^)]*` across a nested call. Both were caught by the compiler, but
only because the sweep was small enough to check.

**Do not use `git add -A` while a background agent shares the worktree.** A file
written concurrently landed in two commits unreviewed and unmentioned;
`2628a80` is the correction. Stage the paths you touched.

## Decisions not to silently reverse

- **No `trait Game`, no `enum Title` dispatch, no plugin registry.** ADR-0021
  answers the n=1 objection for the format layer only; it does not repeal it.
- **No `oag-psp` / `oag-ps2` crates.** ADR-0004 stands. The console axis is
  per-blob discrimination, and turning it into a code axis forfeits the property
  that made the PS2 fan-out cost days.
- **`classes_of` errors on an unknown version rather than defaulting to V6.**
  The numberings share no id, so a fallback finds nothing and looks exactly like
  an empty file.
- **The foreign-serial lists rule a source out, never in.** Each title names the
  *others* it knows to reject. An allow-list would hard-reject a real player's
  own legitimate pressing.
- **A `None` in a class table means "not recovered", never "absent from the
  format".** Per CLAUDE.md's naming rules an unrecovered id gets no entry rather
  than a plausible one.

## Not part of this effort

**PS3 and Vita are paused on toolchain, not research.** Both are encrypted, in
two different ways - see
[`data/README.md`](data/README.md#the-ps3-and-vita-images-are-encrypted-and-nothing-here-decrypts-them-yet).
None of `PS3Dec`/`scetool`/`pkg2zip`/`psvpfsparser` is installed and there is no
RPCS3 install to borrow a decrypted copy from. The one architecturally relevant
fact came free: **HD/Fury and 2048 both ship PSARC, not WAD**, and 2048 arrives
as a PKG rather than a disc filesystem - which is why the content-source layer
is written against "archives resolved by name from a source's own file list".
