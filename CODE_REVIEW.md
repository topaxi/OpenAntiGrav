# Code review

Reviewed at `a4c2862` on 2026-07-30. Findings are ordered by impact times
urgency: the first is a policy gap with a cheap fix, the middle band is
invariants that hold today but nothing keeps holding, the tail is debt and doc
drift.

**Provenance.** The tree was clean when the review started and did not stay that
way: `crates/physics/src/wall.rs`,
`crates/physics/tests/determinism.rs`,
`crates/trace/tests/wall_contact_ground_truth.rs`,
`docs/ghidra/functions/psp-pulse-usa/collision.md` and that binary's `names.tsv`
picked up uncommitted edits partway through (204 insertions, 130 deletions),
matching HANDOVER's "our hull meets the wall six ticks early" thread and its
named next step of reading `Collider_BoxSamplePoints`. That work is **in flight
and was left untouched**; every finding below is about the committed state at
`a4c2862` and none of them are in those five files. The measurements in "Verified
state" were taken before those edits appeared.

**What this review is not.** [`HANDOVER.md`](HANDOVER.md)'s open-threads table
already tracks the outstanding *reverse-engineering* questions (the six-tick hull
contact, the 13 % roll-stiffness gap, the exhaust ribbon, the grid's seven
unrecovered slots). Those are correctly recorded with their evidence and are not
restated here. This is a review of the code and the gates around it.

## Summary

| # | Finding | Impact | Urgency | Fix |
| --- | --- | --- | --- | --- |
| 1 | `--screenshot out.png` writes a reproduction to the repo root, and `png` is in no leakage layer; the movie cache's four formats are one layer deep | High | High | Cheap |
| 2 | Both "enforceable" dependency rules are enforced by discipline only | High if broken | Medium | Cheap |
| 3 | Determinism gate skips debug on Windows/macOS; toolchain unpinned; MSRV never built | Medium-high | Medium | Cheap |
| 4 | 3.6 k LOC of rendering sits in `oag-game`, where rule 2 forbids reuse | Medium | Low | Deferred |
| 5 | No fuzz target for the parsers that read untrusted disc images | Medium | Low-med | Moderate |
| 6 | `oag-view` has zero tests across 1,402 LOC | Medium-low | Low | Cheap |
| 7 | HANDOVER's "read this first" is stale on the image and trace inventories | Low | Low | Cheap |

Findings 1, 2 and 3 are all cheap and together they close the gaps between what
the docs claim is enforced and what CI actually runs; 6 is cheap too and covers
the one crate with no tests at all. That is the highest value per hour available
in this tree right now.

## Verified state

Everything below the findings was checked rather than assumed:

- `just test`: **1,049 passed, 65 skipped**, matching HANDOVER's recorded figure.
  The 65 skips are the `#[ignore]`d ground-truth tests.
- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets`:
  clean, no warnings.
- **Zero** `TODO`/`FIXME`/`HACK`/`todo!`/`unimplemented!` markers across 47 k LOC
  of Rust. The one grep hit is the string `XXXXXXXX` in a `--movie` doc comment.
- Both architecture dependency rules **hold**, verified on the feature-resolved
  graph via `cargo tree -e normal` rather than by reading manifests (`oag-game`
  enables `oag-formats/av1`, so the resolved graph is the one that counts):
  `oag-gameplay` and `oag-physics` have no edge to `wgpu`, `winit`, `oag-render`
  or `oag-input`, and `cargo tree --workspace -e normal -i oag-game` returns
  `oag-game` alone, so nothing depends on the composition root.
- The determinism rules are genuinely observed in the simulation crates: no
  `f64`, no `mul_add`, no `HashMap`/`HashSet`, no wall-clock read and no OS
  entropy anywhere in `crates/{core,physics,gameplay}/src`. Every grep hit is a
  comment explaining the ban.
- `crates/formats/tests/data/testsrc-64x64.ivf`, the one tracked binary, is 4 KB
  of synthetic `ffmpeg` `testsrc` and is documented as such at `av1.rs:423-429`.
  Correctly handled.
- **The parser path is defensive.** Compiling the parser crates' `--lib` targets
  (which excludes `#[cfg(test)]` modules) with `clippy::unwrap_used`,
  `expect_used` and `indexing_slicing` raises 294 warnings, but **zero**
  `unwrap_used`, and the sites sampled are all guarded: `ivf::Header::parse`
  length-checks before every fixed index and clamps with
  `header_len.min(data.len())`; `collision::triangle` indexes `vertices` with a
  file-supplied index, but `collision.rs:769-780` rejects any triangle index
  outside `vertices.len()` at parse time. The count is lint noise, not a defect
  count. See finding 5 for the part of this that is a real gap.

That is a healthy baseline. The findings are about what the gates do not cover,
not about a mess in the code.

---

## 1. The leakage gate does not cover the asset formats the project itself writes

**Impact: high (legal exposure is the project's stated first constraint). Urgency:
high, because a command CLAUDE.md documents drops a reproduction into a tracked
directory with no layer covering it. Fix: cheap.**

[`legal.md`](docs/overview/legal.md) promises three enforcement layers "because
one is not enough", and layer 2's stated job is that "a stray copy outside
`data/` is still caught". For the formats this repo *generates*, layers 2 and 3
are absent.

### The unprotected case: screenshots

`CLAUDE.md` documents these two invocations verbatim:

```sh
just view <image>:<path/to.wad> [--mesh ...|--track] [--screenshot out.png]
just play [--screenshot out.png]
```

`just view` and `just play` pass `*ARGS` straight through (`justfile:132-133`,
`justfile:90`) and run with the repo root as the working directory, so
`out.png` **is written to the repo root**. A screenshot of a decoded track or
ship is a rendered reproduction of the disc's models and textures, which
`legal.md` names explicitly ("no assets, no textures, no models ... A texture is
a reproduction").

`png` appears in **no** layer: not in `.gitignore`, not in `ci.yml:66`, not in
`justfile:251`. `git add -A` after taking a screenshot commits it and every gate
stays green. This is the one path here with zero defences rather than one.

(`just play-screenshots` is fine: it defaults to `out="/tmp"`, `justfile:100`.)

### The one-layer case: the movie cache

`crates/game/src/movie.rs` writes, per movie:

| Written at | Extension | What it is |
| --- | --- | --- |
| `movie.rs:913,955,1005` | `.ivf` | **Lossless** AV1 of the disc's video, so bit-exact picture |
| `movie.rs:977` | `.h264` | Raw demuxed video elementary stream |
| `movie.rs:929` | `.ipu` | Raw demuxed PS2 IPU stream |
| `movie.rs:1021,1120` | `.pss` | Raw demuxed PS2 program stream |

and `boot::default_cache_dir` (`boot.rs:822-831`) resolves to
**`data/cache/movies` inside the checkout** whenever a `data/` directory exists,
which is the normal developer case. Confirmed present:

```
data/cache/movies/71d3c1ec-5736448-480x272-av1ll-1200.ivf   # the intro, losslessly
data/cache/movies/54748_DATA_MOVIES_INTRO512.PSS-20905988.pss
data/cache/movies/18c51e58-1071104.h264
```

None of `ivf`, `h264`, `ipu`, `pss` appear in the `.gitignore` extension block,
in CI's regex (`ci.yml:66`), or in `just audit-leakage` (`justfile:251`). Nor do
the extracted asset formats the docs discuss as content: `mip`, `vex`, `fnt`,
`pmf`.

Here `/data/*` does cover the default location, so this is one layer deep rather
than none. What removes the remaining layer:

1. `--cache <path>` (`main.rs:303`) puts the whole set anywhere the user names,
   including a tracked directory.
2. A tracked `.ivf` already exists as a legitimate test fixture, so a second one
   does not look out of place in a diff or to a reviewer.
3. `.umd` and `.gcm` are in `.gitignore` but in **neither** audit regex. The
   three lists are hand-maintained copies and have **already drifted**, which is
   the failure mode this finding predicts, observed.

No leakage has actually occurred: every tracked file is text (`.rs`, `.md`,
`.toml`, `.py`, `.wgsl`, `.tsv`, plus the one synthetic `.ivf`). This is
prevention, not cleanup.

**Next step.** Add `png` first: it is the zero-layer case and it is one
`.gitignore` line plus one regex. Then put the extension list in a single file,
have `.gitignore`, CI and `just audit-leakage` all derive from it, and add
`ivf`/`h264`/`ipu`/`pss`/`mip`/`vex`/`fnt`/`pmf`/`umd`/`gcm`. While there,
consider asserting the audit covers every extension the code itself can write:
the list drifted once and will again.

Two caveats for whoever writes the patterns, both checked:

- The fix must keep `crates/formats/tests/data/testsrc-64x64.ivf` tracked, so
  `*.ivf` needs a negation for that path (or the fixture moves). It is
  legitimately ours and `av1.rs` `include_bytes!`s it.
- A blanket `*.png` **is** safe: no PNG is tracked today, and the AppImage icon
  is generated at build time by `scripts/appimage-icon.py` writing into
  `data/appimage/`, never committed. So `png` needs no negation.

---

## 2. Both "enforceable" dependency rules are enforced by discipline only

**Impact: high if broken (it is the project's core testability property). Urgency:
medium, both rules hold today. Fix: cheap.**

`CLAUDE.md` and `workspace-layout.md` state the two rules as "both enforceable
and worth checking before adding an import", and roughly ten source files cite
them in doc comments (`gameplay/src/input.rs`, `input/src/lib.rs`,
`game/src/race.rs`, `trace/src/script.rs`, ...). Nothing executes them: there is
no CI job, no test, no `deny.toml`, no `xtask`.

I verified both rules currently hold, on the resolved graph (see "Verified
state"). That is the point: the rules are correct now and are one `cargo add`
away from being silently wrong. A `wgpu` or `winit` edge reaching `oag-gameplay` would not fail
any gate, and the property it destroys, a simulation testable without a GPU and
comparable against a trace, is the one thing the architecture exists to protect.

**Next step.** One test over `cargo metadata` asserting (a) no gameplay crate has
an edge to `oag-render`/`oag-audio`/`oag-input`/`winit`/`wgpu`, and (b) nothing
depends on `oag-game`. Around 30 lines, and it converts a convention into the
gate the docs already claim it is.

---

## 3. The determinism gate is narrower than the invariant it is guarding

**Impact: medium-high (a determinism regression is expensive to find late).
Urgency: medium. Fix: cheap.**

[`determinism.md:24`](docs/architecture/determinism.md) requires bit-identical
state "on Linux, Windows and macOS, on x86-64 and AArch64, **in debug and
release**". Three gaps between that sentence and `ci.yml`:

- **Debug is only checked on Linux.** Both determinism asserts in the matrix job
  pass `--release` (`ci.yml:51,57`). Debug coverage comes solely from the
  `check` job's `cargo nextest run --workspace`, which is `ubuntu-latest` only.
  Of the 3 OS x 2 profile cells the doc claims, Windows-debug and macOS-debug are
  never executed. Architecture coverage is genuinely fine: `macos-latest` is
  AArch64.
- **The toolchain is unpinned.** `rust-toolchain.toml` says
  `channel = "stable"`. The concrete cost is CI stability rather than
  determinism: `clippy -D warnings` (`ci.yml:23`, plus `RUSTFLAGS: -D warnings`
  at `ci.yml:10`) means any new lint in a stable release reddens CI on code
  nobody touched. Secondarily, pinning makes "the toolchain moved" and "the code
  moved" distinguishable if the determinism gate ever does fire, which the
  standing rule "find the bug, never update the reference constants" does not
  otherwise cover. Note this is *not* an argument that the hashes are likely to
  drift: strict IEEE-754 `f32` without fast-math must round identically across
  rustc versions, and reassociation and fusion are exactly what the `mul_add`
  ban already forecloses.
- **The declared MSRV is decorative.** `Cargo.toml:23` sets
  `rust-version = "1.85"` and nothing in CI ever builds with it.

**Next step.** Add the debug asserts to the existing matrix (it is two more
`run:` lines, the job already has the OSes), pin `channel` to an explicit
version, and either add a 1.85 job or drop the `rust-version` claim.

---

## 4. About 3.6 k LOC of rendering sits where rule 2 forbids anyone reusing it

**Impact: medium (debt, no bug). Urgency: low, but it compounds.**

`oag-game` is 23.4 k LOC, the largest crate in the workspace, against
`oag-render`'s 4.4 k. The fat *library* is deliberate and fine: `lib.rs:3-5`
explains that everything but the window and event loop lives in the lib so it is
testable headlessly, `main.rs` is the window and event loop, and the crate
carries 294 tests. No complaint there.

The issue is specific: these are general rendering and text-layout subsystems,
not composition.

| Module | LOC |
| --- | --- |
| `game/src/hud.rs` | 1,691 |
| `game/src/font.rs` | 1,088 |
| `game/src/upscale.rs` | 606 |
| `game/src/sprite.rs` | 251 |

That is 3,636 LOC, plus `render.rs` (987) and `upscale.rs` both creating their
own `wgpu` render pipelines. Because rule 2 forbids depending on `oag-game`, none
of it is reachable from `oag-view` or from the `oag-ui` crate that
`workspace-layout.md` already anticipates. `oag-view` correspondingly builds its
own pipeline in `view/src/main.rs` rather than sharing one.

This is a planned migration coming due, not a new decision, and it is cheapest
before the HUD work in HANDOVER's first open thread adds to those files.

**Next step.** No refactor today. When `oag-ui` opens, move `font`, `sprite` and
`upscale` first: they have no front-end state and are the parts `oag-view` would
use immediately.

---

## 5. The parsers' safety rests on hand-written checks that nothing systematically probes

**Impact: medium (a panic on malformed input is a crash, not a vulnerability
here). Urgency: low-medium. Fix: moderate.**

The parser crates consume the one input this project does not control: a
user-supplied disc image, plus the movie cache it re-reads from disk. As recorded
above, the code is genuinely defensive and `oag-formats` runs 269 tests (plus 14
disc-backed ones skipped), truncation cases among them. Two structural gaps
remain.

**There is no fuzz target.** No `fuzz/` directory, no `cargo-fuzz`, and no
`proptest`/`arbitrary`/`quickcheck` anywhere in the workspace. Every malformed
input tested is one someone thought to write by hand, across the 20 modules of
`oag-formats`.

**Some safety is a cross-function invariant that no type expresses.**
`CollisionMesh::triangle` (`collision.rs:551-558`) indexes `self.vertices` with a
file-supplied `u16`, and cannot panic only because `parse_chunks`
(`collision.rs:769-780`) rejects out-of-range indices at the boundary. That is
the right design and it is documented (`collision.rs:54`). But the coupling is
implicit: a second construction path for `CollisionMesh` that skipped that
validation would turn `triangle`, whose signature promises `Option`, into a
panic, and no existing test would notice. The same shape recurs wherever the
`--lib` clippy run flagged indexing (`vex.rs` 35 sites, `iso9660.rs` 27,
`texture.rs` 22, `pmf.rs` 21).

**Next step.** One `cargo-fuzz` target per container entry point
(`wad::parse`, `ivf::parse`, `collision::parse_chunks`, `texture::parse`,
`pmf::parse`) seeded from the synthetic fixtures already in
`crates/formats/tests/data/`. This needs no disc image, so it can run in CI. If
that is too much, the cheaper half is a single test per format that feeds every
truncation of a valid fixture (`for n in 0..blob.len()`) and asserts `Err` rather
than a panic, which is a loop, not a framework.

---

## 6. `oag-view` has no tests at all

**Impact: medium-low. Urgency: low.**

1,402 LOC, **0** tests, the only crate in the workspace with none (next lowest is
`oag-tools` at 16 over 1,328 LOC). It is a developer tool, which caps the impact,
but it is also the instrument used to eyeball whether a format decoded correctly,
so format confidence partly rests on it.

HANDOVER's own font finding is the argument: the pre-outlined `.fnt` bug was
caught by a screenshot and "a screenshot caught this and no test would have, the
arithmetic was all correct". The viewer is that screenshot path, and it is the
one part of the chain with no coverage.

**Next step.** Not full coverage. Cover the CLI's `<image>:<path>` argument
parsing and the texture-browser selection logic, which are pure and are where a
silent mis-selection would make a correct decoder look wrong.

---

## 7. Doc drift in the two places a fresh reader is told to trust first

**Impact: low individually, higher than it looks because of where it sits.**

HANDOVER's "Read this first" is explicitly the orientation a new session runs on,
and two of its facts are now stale:

- It says `data/images/` holds three images. There are **four**:
  `hdfury-ps3-eu.iso` (2.2 GB, Wipeout HD/Fury, PS3) is also present, and is
  correctly recorded in
  [`source-images.md`](docs/reverse-engineering/source-images.md) and served by
  `just launch-hdfury-ps3`. HANDOVER is the file that disagrees.
- It says `talons-junction-time-trial-lap.csv`, `-lap-omega.csv` and
  `-standing-start.csv` "all gone". Two of those three are back
  (`-time-trial-lap.csv` and `-standing-start.csv`); only `-lap-omega.csv` is
  still absent. So the warning reads as current while describing a largely
  repaired situation.

The surrounding warning, that derived data under `data/` is not durable and the
scenarios that regenerate it are, remains exactly right and is worth keeping.

**Next step.** Restate both as "check, do not trust this list" rather than
enumerating a snapshot, since the enumeration is what goes stale.

---

## Note on one HANDOVER caveat

HANDOVER records that `upscale::tests::the_grade_moves_the_picture_in_the_direction_the_setting_names`
"skips with a note where there is no adapter, so a green CI run does not mean it
ran". On this machine it **did** run, taking 0.897 s of the 1.856 s suite. The
caveat is correct and machine-dependent; noting it so the next reader knows the
local gate does exercise the grade even though CI's may not.
