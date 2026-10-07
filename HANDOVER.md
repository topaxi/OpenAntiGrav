# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); open,
in-flight work lives in [`handover/`](handover/), one file per thread, sorted
into five subdirectories by category - `rendering/`, `gameplay/`,
`frontend/`, `tooling/`, `audio/`; this file is the index plus what a fresh
reader would otherwise have to rediscover before touching either.

**Keep it that way.** A finding belongs on a docs page with its evidence and
its confidence score; an open thread belongs in `handover/`; this file points
at both rather than restating them. Rewrite a section in place rather than
appending a dated pass to it - `git log -- HANDOVER.md` is the history, this
file does not need to carry its own.

## Read this first

- **"What's built vs. read vs. verified, per title" is now one page**:
  [`docs/overview/status.md`](docs/overview/status.md), not a question to
  answer by reading four prose pages.
- **The disc images are in place** - all six, in `data/images/`:
  `pulse-psp-usa.chd`, `pulse-psp-eu.chd`, `pulse-ps2-eu.chd`,
  `pure-psp-usa.chd`, `pure-psp-eu.chd` and `hdfury-ps3-eu.iso`. Hashes match
  [`source-images.md`](docs/reverse-engineering/source-images.md), so
  `just test-data` works. Earlier sessions have believed them lost and spent the
  session unable to verify anything - check before concluding a ground-truth
  test "cannot run".
- **`rg` and `fd` cannot tell you that.** Both respect `.gitignore` and `data/`
  is gitignored, so both return nothing with exit code 0 whether the directory
  is full or empty - no error, no warning, and it reads exactly like "there is
  nothing here". Use `--no-ignore`, or `/bin/ls` / `find` / `grep`. Note plain
  `ls` may be shell-aliased (`ls -la` failing with `ls:1: command not found:
  -la` reads like a broken argument and is not), and a spawned agent has a
  second, independent reason to see `data/` as empty: being gitignored, it does
  not travel into a worktree or a restricted sandbox. **In a worktree, fix that
  with `just link-data`** before concluding anything - it symlinks every `data/`
  subdirectory from the main checkout, and until it is run the DLC and trace
  suites skip silently (or, under `OAG_REQUIRE_GAME_DATA=1`, fail in a heap
  naming files that were in the main checkout all along). Added 2026-08-26 after
  exactly that happened: five `dlc_ground_truth` failures that were a worktree
  with no `data/dlc` in it and nothing else.
- **Derived evidence under `data/` is not durable; the scenarios and scripts
  that regenerate it are.** A 2026-07-29 pass found three reference traces gone,
  leaving `force-balance-ground-truth.md`, `contact-response.md` and three
  citations inside `wall.rs` pointing at files that were not there. Anything
  under `data/traces/`, `data/shots/` or `data/cache/` can be absent; the
  committed `verification/scenarios/*.inputs` are what reproduce it.
  **Five are absent as of 2026-09-05** (corrected from "three" recorded
  2026-09-02, which missed a pair), confirmed by `/bin/ls data/traces/` rather
  than inferred: `pad0-boost.csv` (cited from `race/scene.rs` and
  `race/effects.rs`), `talons-junction-time-trial-lap-omega.csv`,
  `talons-junction-standing-start.csv` - which `crates/trace/tests/wall_contact_ground_truth.rs`
  names as its `CAPTURE` - and **`talons-junction-steer-left.csv` and
  `talons-junction-steer-right.csv`**, `yaw_authority_ground_truth`'s own two
  captures, missing the same way and not previously listed here. Beware
  `talons-junction-time-trial-lap.csv`, which *is* present and is a different
  file from the `-omega` one.
- **Four numbers about this suite look green and are not, and each hid the next.
  Read all four before believing any red count.** Every one was believed by
  somebody on 2026-09-02 - two people, working from opposite ends - before being
  checked. They are one mistake with four faces: a green number trusted without
  the check that would have refuted it, where the check is always *did the thing
  I think I measured actually run*:
  1. **`just test-data` reported one failure and there were six.** The recipe
     had no `--no-fail-fast`, and the two `oag-formats` reds sort early enough
     to stop the run at 138 of 3,300. Fixed in the `justfile`; a truncated run
     reads as a far worse tree than it is *and* hides everything after it.
  2. **`0 skipped` is not evidence any data was exercised.** nextest counts a
     test that early-returns on a missing file as **passed**, so the count says
     only that nothing was *filtered out*. Reading it as coverage is what
     produced a since-deleted claim in this file that the missing traces were
     back, when `ls` says they are not.
  3. **A filter can select a different set of tests than you meant and report a
     confident green on it.** `-E 'test(...)'` matches a test's own name and
     module path, **not** the binary it lives in. So `test(rcsmodel)` selects
     `oag-formats`'s `rcsmodel::tests::*` and `rcsmodel::vertex_decl::tests::*`
     unit tests and none of the ground-truth ones; and `test(race_finish)`
     selects exactly one test - `race::results::tests::the_board_is_taken_on_the_tick_the_race_finishes`,
     whose *name* contains "race_finishes" - which passes. **That is worse than
     selecting nothing**, because `0 tests run` is visibly wrong on the console
     and one green test is not. A re-run of "the failures" this way came back
     **39 passed**, of 39 tests, none of which was a failure.

     **Use `binary(...)`, not `test(...)`, to re-run a ground-truth failure**:
     `-E 'binary(race_finish_ground_truth)'` lists all three of that binary's
     tests including both reds. Keep `--run-ignored all` with it - all six are
     `#[ignore]`d, so an exact-name filter *without* that flag is a second,
     independent route to the same false green, reporting
     `0 tests run: 0 passed, 3304 skipped` as success.

     **The habit that catches this is `cargo nextest list` on the filter before
     trusting the run.** It is one cheap command, and this face of the mistake
     is the one that most needed it: the other three were each caught by
     somebody simply re-running the thing, while this one survived *two* people
     reasoning forward from it - one to a shared-state hypothesis, the other all
     the way to going and looking for the shared state. Neither asked what the
     filter had selected.

  **So a red count means nothing without the variable's state attached.** The
  same tree, same commit, same day: **6 failed / 0 skipped** plain, and
  **17 failed / 0 skipped** under `OAG_REQUIRE_GAME_DATA=1`. The extra eleven
  are the derived-data suites - `chase_camera` (3), `wall_contact` (3),
  `yaw_authority` (2), `pure_dlc` (2), `maglock` (1) - failing because that
  variable turns "silently pass on missing data" into a failure. They are
  missing captures and DLC packs, not broken code.
- **The six reds above are fixed, 2026-09-02, all six premise expiry rather
  than a code bug** - each was a test whose assumption stopped being true
  under a real, separately-verified improvement, and none of the six needed
  its underlying subsystem touched:
  - **Both `race_finish_ground_truth` ones**: the disc's own results-screen
    field list gives a destroyed craft "Ship destroyed" instead of a place
    (confidence 75, `0x08a82dfc`), so a single race ending by elimination is
    the disc's second legitimate ending, not a bug - already recorded above.
    Both tests now branch on which ending happened and assert the matching
    one; see `crates/game/tests/race_finish_ground_truth.rs`'s module doc.
  - **`stall_rescue_ground_truth::a_craft_that_stops_on_the_disc_is_put_back`**:
    only *half* premise expiry, and worth reading carefully rather than
    trusting the first pass on it - an earlier version of this entry claimed
    the disc no longer beaches a craft anywhere and that was wrong, caught by
    re-tracing the fix's own claim rather than trusting it. `docs/gameplay/ai.md`'s
    hover-probe sweep (the "last three" fix) closed the *sustained*-stall
    shape of the failure disc-wide - swept all twelve circuits at all four
    difficulties, no cell reaches `STALL_TICKS` by sustained dwell any more
    (closest: `07_Track` novice at 46 of 120) - which is real and is what the
    original test measured. **It did not close the bounce-in-place gap**
    `docs/ghidra/functions/psp-pulse-usa/grid.md` already had open: re-traced
    2026-09-02, `05_Track` at novice sits frozen near `(-717, 19, -512)` from
    roughly tick 5,000 to 12,000 while its speed keeps flicking 0-13 units/s,
    never sustained low enough to trip the dwell counter - one lap in 12,000
    ticks, zero respawns. `07_Track` novice shows the same shape, milder.
    Renamed to `no_circuit_sustains_a_stall_past_the_rescue_threshold`
    (split per difficulty into `..._at_novice`/`_at_skilled`/`_at_elite`/`_at_ace`
    on 2026-09-09), scoped to the claim that actually holds, and now prints laps rather than
    hiding the ones that stay low. The dwell-counter mechanism itself moved to
    a direct unit test,
    `crates/raceplay/src/tests/respawn.rs::a_stopped_opponent_is_flagged_after_the_dwell`,
    which proves the counter fires exactly at its threshold - it was never in
    question, the open part is a craft that never describes the state it
    watches for. `grid.md`'s "bounce in place" open item is re-confirmed open,
    not closed.
  - **`difficulty_ground_truth::every_difficulty_is_quicker_than_the_one_below_it`**:
    the same AI-gained-weapons change as the race-finish pair, one level up -
    `ace`'s opponents now shoot each other (3 of 5 seeds put one out of the
    race), which the old `wrecked == 0` assertion read as a bug. It is not:
    the leader-distance metric this test actually measures is unaffected
    either way (`Standing::distance` on a wrecked craft just stops advancing
    and falls out of contention), the ordering still holds on the same runs,
    and a wreck is now printed rather than asserted against.
  - **Both `oag-formats` ones**: leftovers from `3ce4e5ac`'s `STRIDES` fix
    (2026-08-25, three widths to the disc's own seven) that the commit landing
    the fix did not update. `the_disc_declares_more_widths_than_the_search_looks_for`
    asserted the pre-fix gap by name and could not survive it closing -
    deleted, since `rcsmodel_stride_ground_truth.rs` (the same commit) already
    pins the correct invariant with more rigor. `a_third_of_a_circuits_chunks_are_see_through`'s
    pinned count moved `(257, 560)` to `(251, 553)`, confirmed rather than
    assumed as `STRIDES`'s doing - setting it back to `[14, 18, 22]` locally
    reproduces `(257, 560)` exactly. Likely mechanism, from reading rather than
    tracing every changed chunk: `solve_stride_without_a_box`'s three rules
    each pick a winner by its margin over the runner-up among `STRIDES`, and
    four more candidates give a few chunks that used to win by default a
    closer contender to tie or lose against. The share barely moved (31.4% to
    31.2%) - same disc, more carefully measured.

  Verified together and against the rest of the disc-backed suite: `cargo
  nextest run --workspace --run-ignored all --no-fail-fast -E
  'binary(race_finish_ground_truth) or binary(stall_rescue_ground_truth) or
  binary(difficulty_ground_truth) or binary(rcsmodel_decl_ground_truth) or
  binary(rcsmodel_material_ground_truth) or binary(rcsmodel_stride_ground_truth)'`
  - 29 of 29 pass, 76s.
- **Check `git status` before assuming the tree is clean.** A whole milestone's
  work once sat uncommitted for a day.
- **Gate status:** last measured green at **2,949 tests (2026-09-04)** in 13.8s,
  with `fmt`, `clippy`, `check-docs`, `check-deps`, `check-determinism`,
  `check-size`, `check-names` and `check-handover` all clean (575 skipped -
  the `#[ignore]`d disc-backed ones). Re-measure rather than trusting the number here -
  `git stash && just test` is how the drift was caught last time.
- **Plain `just test-data` is 2 failed / 0 skipped; `OAG_REQUIRE_GAME_DATA=1`
  is 13 failed / 0 skipped - both measured 2026-09-05 against local `main`.**
  Say which one a number means, always: this file has been burned by exactly
  this ambiguity before (see the 6-vs-17 entry below, 2026-09-02). Plain
  `just test-data` (486s) is the command CLAUDE.md documents and the one the
  next agent will actually run; it shows only the 2 deterministic reds below,
  because the other 11 are absent-derived-data tests that silently pass
  without the flag. The 13-under-the-flag number was **reproduced twice**, at
  load 22-29 (10:51) and on a quiet machine (6:49) - both runs named the
  **exact same 13 tests**, with byte-identical assertion values on the two
  that assert a value, so contention cost wall time on this measurement, not
  correctness. Re-measure the same way rather than trusting either list past
  the next batch of merges; it is exactly the kind of number three sessions
  disagreed on this same day before this was pinned down.
  - **11 of 13 are environment-dependent, not code bugs**: absent derived
    data that `OAG_REQUIRE_GAME_DATA=1` turns into a hard failure instead of a
    silent early return. `chase_camera_ground_truth` (3, needs
    `data/traces/pad0-boost.csv`), `wall_contact_ground_truth` (3, needs
    `data/traces/talons-junction-standing-start.csv`), `yaw_authority_ground_truth`
    (2, needs the `-steer-left`/`-steer-right` pair), `maglock_ground_truth`
    (1, needs `data/traces/talons-junction-time-trial-lap-omega.csv`) - all
    five files confirmed absent, see the trace-absence bullet above - and
    `pure_dlc_ground_truth` (2, needs `data/keys/pure-dlc-keys.txt`, a
    maintainer-only decryption key table; the DLC zips themselves *are*
    present under `data/dlc/`). Neither cause got its own `handover/` thread:
    the trace-capture gap is the already-documented, already-actionable
    situation two bullets up (`verification/scenarios/*.inputs` regenerates
    them; the recorded absence there is now the whole account) and the DLC
    key table is a maintainer-supplied credential this repo cannot regenerate
    at all, not an investigation with a next step.
  - **1 of 13 *was* deterministically red and is now green. There is no
    inherited red in this suite.** The diagnosis is kept below because it is
    the useful part, and because of this file's own standing rule: a red
    ground-truth test usually encodes a real disagreement, so "make it green"
    is the wrong instinct without understanding why first. **Re-confirmed
    2026-09-09** against `main` at `1328d108`, unmodified - `PASS [0.812s]`,
    plus two independent full `test-data` runs green the same day on unrelated
    branches. **Do not hand a member "expect one inherited failure" as a
    baseline**: this row's opening line said "not fixed" for a day after its
    own body said "Resolved", and a member given that figure will dismiss a
    real regression as inherited. The test and its history:
    `shuriken_ground_truth::a_thrown_blade_bounces_off_a_real_circuit_and_dies_on_its_fuse`
    (one press throws zero blades - **not** `0c78c477`, that attribution is
    refuted; bisected to `cc395862`, the countdown-hold fix, which correctly
    leaves the field grid-tight at throw time and a blade clips a grid-mate -
    **deferred 2026-09-06**: the Shuriken's race-table odds are zero in every
    class, so this scenario - a blade thrown off a standing start in a single
    race - is one no implemented mode can produce a weapon pad for; it needs
    `Mode::Eliminator`, which does not exist yet, so chasing `hull_radius`
    further is parked rather than a next step). **Resolved 2026-09-08**, once
    `Mode::Eliminator` landed: the diagnosis above was exactly right and the
    engine's behaviour was never the bug - a hull hit correctly detonates a
    Shuriken by design, the confound was a grid-mate in the blade's path. The
    test now runs in Eliminator (the mode the weapon is actually reachable
    in) with the other seven grid slots switched off before the throw, which
    is what its own doc comment already claimed to be testing - a blade
    against track geometry, not a craft's hull. Green: 119 of 120 fuse ticks
    alive, 3 bounces. The `hull_radius` question this thread left open is
    therefore moot for this scenario and was not otherwise chased.
    `stall_rescue_ground_truth::a_healthy_craft_never_looks_stalled_for_a_single_tick`
    was the other one - **fixed 2026-09-07**: instrumenting `(tick, thrust,
    speed)` around the countdown release confirmed the standing hypothesis
    exactly (thrust 0 -> 100 at `COUNTDOWN_TICKS`, speed still ~0.57 <
    `STALL_SPEED`, then >1.0 the very next tick, identically on `16_Track`,
    `03_Track` and `06_Track`), so the test now excludes that one understood
    tick from the stall count rather than the threshold being loosened.
  - `ps2_source_ground_truth`'s transcode test - historically flaky under load
    per the entries below - **passed cleanly in both runs today**, including
    at load 22-29 (298s), the same contention level the older flake reports
    were attributed to. That weakens rather than confirms "ordinary contention
    triggers it"; whatever the older failures needed, this session's load
    wasn't it.
  - This supersedes the 2,488-of-2,494 count below, which is over two weeks
    stale (2026-08-18) and against a workspace that has nearly tripled in
    test count since; the mechanism notes below it (the ffmpeg flake's shape,
    the build-profile fix) are still accurate and worth reading.
- **`just test-data` takes about 3:20, not 11:27 and not 9:47.** If it takes
  eleven minutes you are on a tree from before the build-profile fix:
  `[profile.dev.package."*"]` never matched a workspace member, so our own
  decoders and the sim compiled at `opt-level = 0`, and the suite additionally
  ran its multi-minute tests last. Both are fixed in `Cargo.toml` and
  `.config/nextest.toml`; the measurement and the reasoning are in
  [workspace-layout.md](docs/architecture/workspace-layout.md#build-profiles).
  **Do not "simplify" either file back:** the per-crate `opt-level` lines look
  redundant next to the `"*"` glob and are not, and the nextest priorities look
  cosmetic and are worth half the win.
  **If it takes nine or ten minutes you are on a tree from before 2026-09-09**,
  when the suite had drifted back to 587s with a *single* test accounting for
  525s of it - the six tail tests were matrices in one `#[test]` each, and
  `cargo nextest` parallelises across tests, so each ran on one core. They are
  split now, and `just test-data` runs `scripts/check-test-budget.py` over its
  own log afterwards so the next one fails at review: **450s for the suite,
  300s for any single test** (both under-load durations - see that script).
  The split took it to 379s, and sharing one open `DiscImage` across a
  source's archives to **344s**. When that fires, re-profile the tail rather than
  adding a `BASELINE` row - CLAUDE.md's "Commands" section has the four-step
  recipe and the worked examples of which splits keep which assertions. **The four `oag-trace` failures this bullet used to warn
  about are gone**: the disc-backed `oag-game` and `oag-formats` suites are
  **1,269 of 1,269, 0 skipped** under `--run-ignored all` (re-measured
  2026-08-17 after the HD HUD and `.gtf` passes; `oag-game` alone read 739, then
  656, then 652). **`oag-trace`'s five `data/traces/` failures are fixed
  (2026-08-19), and it is skips now, not reds**: `wall_contact_ground_truth` and
  `yaw_authority_ground_truth` used to fail with `No such file or directory` on
  `data/traces/talons-junction-standing-start.csv` and `-steer-left.csv`/`-steer-right.csv`
  whenever `data/traces/` did not exist, because - unlike every other
  ground-truth test in the tree - they read those files with a bare `expect`
  instead of the usual skip-if-absent guard. Both files now gate on a
  `capture()`/`capture_path()` helper matching the pattern `image()` already
  uses elsewhere (`OAG_REQUIRE_GAME_DATA` forces a hard failure, otherwise a
  missing capture prints the `verification/scenarios/*.inputs` command that
  regenerates it and the test returns early). Verified both ways: all five skip
  cleanly with `data/traces/` absent, and fail loudly under
  `OAG_REQUIRE_GAME_DATA=1`. **Two of them were flaky rather than solid, both in
  `ps2_source_ground_truth`, and both are fixed as of 2026-09-09
  (`33913eb9`).** The cause, undiagnosed across four separate investigations
  between 2026-08-16 and 2026-09-09: the two transcode tests shared one
  deterministic scratch directory, so `nextest`'s own parallelism had them
  writing and reading a single set of files. Every observation those
  investigations recorded follows from it - a failure in a full sweep then a
  pass alone, `movie.frames` being `None`, `ffprobe` "invalid float literal"
  and "Invalid data found", and the correlation with machine load, which only
  changed the interleaving. See `transcode_scratch`'s doc comment in
  `crates/game/tests/ps2_source_ground_truth.rs`. **The lesson worth keeping is
  the shape**: a red that moves with load, passes alone, and resists diagnosis
  is a shared-path race before it is a starvation story. Separately, and still
  true, **one failure is the machine rather than the code**:
  machine rather than the code**:
  `oag-disc::ground_truth listing_matches_chdman_and_the_reference_iso` shells
  out to `chdman extractdvd` into `/tmp/oag-ground-truth.iso`, `/tmp` here is a
  6.8 G tmpfs, and the extracted PSP ISO does not fit in what is free. It fails
  at "Extracting, 29.4% complete... Error writing to file; check disk space",
  which `nextest` reports as `chdman extractdvd failed` two frames up and reads
  like a broken invocation. Free `/tmp` or point the test elsewhere; do not go
  looking for a disc bug.
- **Rendering fidelity gaps are mostly not bugs.** Bloom, colour grading and
  every authored light class (`AmbientLight`/`DirectionalLight`/etc.) are still
  unchecked M6 roadmap items - the flat, dark look that produces is a known,
  documented gap, not something to guess-fix from a screenshot.
- **And a second one was, in the other direction: the disc authors the light
  rig and nothing read it.** `mesh.wgsl`'s two invented light directions were a
  stand-in and its own header said so; Wipeout HD states a sun direction, a sun
  colour and a constant ambient in a plain-text `track.envsettings` beside every
  circuit. Read and drawn on 2026-08-18 -
  [`envsettings.md`](docs/formats/envsettings.md). **The trap in the file**:
  `"Lighting.Sky colour"=128 128 128 0` is a byte quadruple where every other
  colour is normalised, and the only thing that says so is the missing decimal
  point; and `Sun direction` is **not** a unit vector on most circuits, with
  four degenerate. **The hold**: its magnitudes are authored for a linear HDR
  pipeline with a tonemapper and this one has neither, so the hue is drawn and
  the scale is not. That the file authors magnitudes above 1.0 at all is
  evidence that ADR-0020's gamma-authoritative model is wrong *for HD
  specifically* - recorded on the page, not acted on, because superseding an ADR
  needs a reference frame from the running original.
- **But one of them was, and the shape of it is worth carrying.** On 2026-08-18
  a screenshot of HD's Talon's Junction showed a quarter of the circuit smeared
  into horizontal streaks. Two of `.rcsmodel`'s readings were *solving* for
  something the file states: a chunk's `+0x58` points at a **vertex-attribute
  declaration** giving the stride, and per attribute a `~crc32` name, an RSX
  type and a byte offset. The texture coordinate was being read from the last
  four bytes of a vertex, which on the commonest layout is `lightmapUV`.
  `docs/formats/rcsmodel.md`. **The trap that cost the first sweep**: `+0x58` is
  the index count on a `LAYOUT_INLINE` chunk, so a disc-wide sweep that does not
  gate on the `+0x06` layout byte comes back as convincing noise - 606
  declarations with a stride of zero, type codes spread over all sixteen values.
  **The lesson worth more than the fix**: the retired reading had a
  texel-density discriminator that really did separate two groups, and a sweep
  of every header byte that really did find no field separating them. Both were
  correct and both were about the wrong thing - the sweep was run against labels
  the discriminator produced, so it could not find the field one word away from
  the bytes it swept. When a heuristic stands in for a missing field, check the
  field is missing.

## Where the project stands

### The 2026-08-18 review's findings

[`docs/reviews/2026-08-18-code-review.md`](docs/reviews/2026-08-18-code-review.md)
lists the findings and the headline fixes. The do-now batch, the next-session
batch and the P2/P3 sweep are all landed, one commit each, full `just` green
throughout; what was not done is below.

**Not done, and why**: S4/S5 (a per-asset `(Archives, &Title)` split and a
title-probe table - racebox groundwork the review itself grades L/M and puts
last); S6 (a naming axis on `Title`, waiting on the 2048 corpus that would
give it shape, per ADR-0022's own rule); I8 (splitting this file - M-sized,
orthogonal to a findings pass); U3 (physical- vs logical-key bindings -
investigated and rejected: AZERTY needs physical scancodes throughout, and a
hybrid mapping some keys physically and some logically would be worse than
either pure scheme); U4 (a `Vec` resize plus an `Arc<Sound>` drop on the
audio thread - needs a return channel to fix honestly).

**One trap worth keeping**: G3's "validate the chain's first step" check also
fired on `--reel`, whose state is off-path by construction (the dev/pub reel
has no screen in the set), and silently turned the flag into a no-op.
`boot_ground_truth::the_reel_leg_still_runs_its_frame_holds` is the only
thing keeping that path from rotting, and it is `#[ignore]`d, so `just`
stayed green throughout. A validity check applied to a state the *operator*
named rather than a chain led to is a check against the wrong question.

### M4 and M5

M4 (shield/energy, Zone's ending, the grid, weapon pads, the weapon table,
pickups, projectiles, the race-level determinism hash, weapon-fire dispatch,
`Mode::SingleRace`) and M5 (six AI stages, per-craft liveries, respawns,
mistake injection, per-opponent lap times) are both landed. Each has its own
docs page with the evidence - **do not requote from this file**:

- Shield/energy pool, contact damage, Zone's ending:
  [shield.md](docs/ghidra/functions/psp-pulse-usa/shield.md),
  [zone-mode.md](docs/ghidra/functions/psp-pulse-usa/zone-mode.md).
- The grid, weapon-fire dispatch, pickups, projectiles, the Missile (the
  first weapon whose whole behaviour - lock, guidance, speed ramp, bounce -
  is recovered rather than half-invented):
  [grid.md](docs/ghidra/functions/psp-pulse-usa/grid.md),
  [weapon-fire.md](docs/ghidra/functions/psp-pulse-usa/weapon-fire.md),
  [pickups.md](docs/gameplay/pickups.md),
  [missile.md](docs/ghidra/functions/psp-pulse-usa/missile.md).
- A race ends and shows a scoreboard, off a captured snapshot rather than a
  live tick (`Race::tick` itself is unchanged):
  [race-modes.md](docs/gameplay/race-modes.md#what-happens-when-a-race-ends).
- The player is recovered when they leave the circuit
  (`Race::lost_off_the_track`, `PLAYER_RESCUE_HALF_WIDTHS`/`_TICKS`,
  deliberately not extended to a human-held slot 0):
  [ai.md](docs/gameplay/ai.md#and-nothing-recovered-the-player-either).
- Six AI stages, liveries, respawns, mistake injection, per-craft lap times -
  the AI no longer blocks M5, and **this paragraph is why a summary here ages
  faster than the row it summarises**: re-read the AI rows under Open threads
  before trusting this line. [ai.md](docs/gameplay/ai.md).
- Wipeout HD reaches the menus and starts a race, off a *declared* rather
  than measured boot chain, with four independent format bugs fixed on the
  way (`.fnt` byte order, no `.gtf` decode path, a Latin-1 language file, the
  480x272-authored menu grid at HD's 800 offset):
  [hd-frontend.md](docs/formats/hd-frontend.md),
  `crates/game/tests/hd_boot_ground_truth.rs` (13/13).
- Rocket visuals (`Data\Weapons\Rocket.vex`, not a billboard, with two
  separately authored explosions), floor-following flight and the km/h fix:
  [rocket-visuals.md](docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md).
- Race sound effects on all three titles, off the discs' own `.bnk` banks:
  six cues on Pulse and Pure, four of the six on HD.
  [psp-audio.md](docs/formats/psp-audio.md#a-cue-owns-a-run-of-the-command-table).
- The start gantry's `3`, `2`, `1`, `GO` on all four titles: Pulse walks four
  per-vertex UV cells across a shared palette staircase (90); Pure ships no
  track-side gantry at all (85); HD ships four glyph files (plus `fx350.vex`)
  whose geometry moved to a sibling `.rcsmodel` and whose own five UV cells
  sit against a *static* material parameter rather than a track (88 for the
  packaging, 40 for whether it ever changes at runtime); 2048 is a third
  mechanism again - four separate mesh nodes, each slid on and off screen by
  its own transform track (90), with no shared UV walk at all, and reaches
  only two of its own eight glyph files (plus `fx350`) across all 26 circuits
  the whole package ships. **What held across all three titles with a gantry
  is a shared three-beat master clock**, not one shared instant: something
  moves at 6.000 s, `FINAL LAP` enters at 9.333 s and the chequered flag
  enters at 12.333 s, on Pulse, HD (confirmed directly on HD's own disc) and
  2048, with a node literally named `polySurface7` carrying the identical
  closing frame pair on all three. **The gantry is placed and drawn on Pulse
  and HD** (2026-09-06 and 2026-09-13): the track's own geometry authors the
  mount, keyed by name (`billboard8`/`321backplate` on Pulse,
  `billboard8.gtf` on HD), and `oag_render::gantry`/`oag_raceplay::gantry`
  stand `321Go_StartFinish.vex` on it and play its authored timeline off the
  race clock. HD's own mount is not Pulse's flat stub - it is 1.9-3.0 units
  thick, part of a real 3D gate structure - and its model additionally
  embeds seven mesh nodes of **slot 7's own borrowed `fx350_nomip.gtf` art**;
  three of them (`polySurface155/156/157`) sit 33 units off the digit
  board's own display plane and rendered as a stray "Wipeout symbol" logo
  floating over the open road until `oag_render::gantry::strip_fx350_art`
  dropped them by texture, matching a player's own report pixel for pixel.
  HD's glyph *walk* is still unwired for **playback** - its material's
  `uvOffset`/`uvScale` are a static value on disk, not a keyframe track
  (88/40) - but 2026-09-17's patched-RPCS3 `Z2` watchpoint pass **located the
  runtime write live** (`AnimCurve_EvaluateChannels`, off
  `Billboard_UpdateInstanceUvs`/`Billboard_UpdateAndRender`), same PC on two
  boots; the authored curve data itself is the remaining gap, likely inside
  `321go_startfinish.rcsmodel`'s own undecoded bytes
  (`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s 2026-09-17 section;
  its own prior "eleven unexamined `lwz 0x834(` sites" lead was already dead
  by 2026-09-15). 2048 is untried past a first-glance read of
  the same family: four separate mesh nodes, each slid on and off screen by
  its own transform track (90), with no shared UV walk at all, and reaches
  only two of its own eight glyph files (plus `fx350`) across all 26
  circuits the whole package ships.
  [start-gantry.md](docs/rendering/start-gantry.md), one page for all four
  titles; `crates/vex/tests/start_gantry_ground_truth.rs` (4/4, Pulse),
  `start_gantry_pure_ground_truth.rs` (2/2, Pure),
  `crates/game/tests/start_gantry_report_pure_ground_truth.rs` (1/1, Pure),
  `crates/hd/tests/start_gantry_hd_ground_truth.rs` (6/6, HD),
  `crates/game/tests/start_gantry_hd_mount_ground_truth.rs` (3/3, HD),
  `start_gantry_2048_ground_truth.rs` (9/9, 2048).

**What M5 still wants**: nothing on Zone's explosion - it landed 2026-10-01 (the wreck thread below). Positional audio and the Autopilot
pickup both landed 2026-08-24; each has a row under Open threads carrying what
is still unread about it.

The force law, angular-momentum model, contact response and mag-lock
attitude hold are all recovered at instruction level and implemented, and
every fitted constant that once stood in for a recovered one has been
retired (`YAW_DRIVE_CALIBRATION`, `ALIGNMENT_INERTIA`, the `<Misc>`-derived
box inertia). Evidence: [docs/physics/](docs/physics/README.md) (both
ground-truth pages and
[angular-velocity-column.md](docs/physics/angular-velocity-column.md)) and
[docs/ghidra/functions/](docs/ghidra/functions/README.md) (`engine.md`,
`rigid-body.md`, `contact-response.md`).

**Read lap fidelity off the *seeded* comparison, never open-loop.** The
original integrates a variable frame duration
([ADR-0007](docs/architecture/adr/0007-fixed-timestep-vs-original.md)), so
two runs of the original itself, from the same pinned start pose, are 100
units apart by tick 495 - a long single-seeded comparison measures the
emulator's scheduler, not the force law. The reference lap
(`data/traces/talons-junction-clean-lap.csv`) is 96.3% wall-free, first
touching a wall at tick 256. **Inside that clean window single-seeded
position error is 4.6 units at tick 240 and never above 5** (2026-09-30).
The 44.1 recorded before was two bugs, not the force law's shape: the
replay harness never armed the speed pads (the original crosses one at tick
161), and the airbrake's forward `drag` term read `steerX` on `-1..=1` where
the original holds it on `+/-100`, 100x weak. Pinned by
`crates/trace/tests/lap_window_ground_truth.rs` and
`pad_crossing_ground_truth.rs`; the address trail is in
[engine.md](docs/ghidra/functions/psp-pulse-usa/engine.md#the-raw-steerx-is-on-the-100-scale-too).
Full numbers, the
standing-start scenario's independent cross-check (launch acceleration 32.69
against a recorded 32.52), and why `grounded` cannot be read as a headline
(`1.0` on all 3,146 of the original's own ticks) are on
[oag-trace.md](docs/tools/oag-trace.md#reseeding-and-why-a-long-comparison-needs-it)
and
[force-balance-ground-truth.md](docs/physics/force-balance-ground-truth.md).
**Always pass `--script-lead 2` against a `data/traces/` capture** - the
emulator's three-frame input latency means a script's first two states never
reach it, and replaying them anyway reads as a two-tick physics wedge;
`oag-trace.md` has the mechanism. A save-state fixed start was investigated
and not adopted (two loads of the same state disagreed by 0.031 units/1.41
degrees against `--start-heading`'s 0.0022/0.0001, which is what closed the
roadmap checkbox instead) -
[ppsspp-debugger.md](docs/reverse-engineering/ppsspp-debugger.md#save-states-and-the-input-recording-api-that-may-replace-them)
keeps the mechanism documented for a need heading-pinning cannot reach.

## Open threads

Each is a real, named next step, one file per thread under [`handover/`](handover/), filed into whichever of `rendering/`, `gameplay/`, `frontend/`, `tooling/` or `audio/` its subject matter fits best. A thread that genuinely crosses categories carries a `categories:` YAML frontmatter block naming the extras, on top of the primary its directory already gives it. Task numbers in a title are the ones the agent passes used, kept because commits and docs cite them. **When a thread's work lands, delete its file and this line** - the same rule this file always followed for a row, now for a file.

**Do not cite a `handover/*.md` thread from anywhere outside `handover/` and this file.** A thread file is deleted the moment its work lands (see above), so a citation from a doc comment, script, or test survives its target and goes dangling silently - nothing currently gates this outside `docs/` (`scripts/check-doc-links.py` only checks `docs/**/*.md`). State the fact directly instead, or point at the `docs/` page that carries it.

- [The PSN Wipeout HD download opens, races and opens its campaign; its backdrop is absent](handover/frontend/the-psn-hd-download-opens-races-and-opens-its-campaign.md) - 2026-10-07: plain HD v3.00 via RPCS3 install, `data/extracted/ps3/hd-psn-eu`, `oag_hd::psn::PSN`; exhaust and menu frame measured off the decrypted executable; campaign opens directly (no HD/Fury chooser, measured on RPCS3), race box rows plain, Zone/mount order chosen
- [The Windows build runs under Wine and Proton; what is left unchecked](handover/tooling/wine-and-proton-run-the-windows-build-open-ends.md) - 2026-10-07: `just wine-run`/`wine-check`; determinism hashes and Pulse/HD screenshots and audio match native, Proton too; frame rate, device audio, 2048/Omega unchecked
- [The Android APK builds, runs in Waydroid, and awaits its first phone run](handover/tooling/android-first-pass-builds-an-apk-nobody-has-run.md) - 2026-10-07: NativeActivity arm64 APK via `just apk` and a `release.yml` job; startup and, since the controls lane, system Back, on-screen race controls and Android pad buttons landed and driven on Waydroid; the Galaxy S24 (Xclipse 940) is untested, layout untuned, AArch64 determinism unrun
- [CI is repaired and release.yml is written, but neither has run on GitHub](handover/tooling/ci-and-release-workflows-are-unrun-on-github.md) - 2026-10-07: five root causes of the red `main`, a draft-release workflow reusing `build-appimage.sh`; the first green run is also the first Windows/macOS determinism check in 26 days
- [A player cannot set up 2048, Omega or HD without guessing](handover/tooling/a-player-cannot-set-up-2048-omega-or-hd-without-guessing.md) - 2026-10-07; Pulse and Pure drop in and boot, but a `.pkg` is invisible, a no-argument boot never finds a 2048/Omega folder, HD's encrypted ISO errors about Pulse's WADs, `$OAG_IMAGE` rejects package folders
- [Vita3K teleport needs the 2048 player's body](handover/tooling/vita3k-teleport-needs-the-2048-player-body.md) - 2026-10-07: memory read/write works, the player body is not isolated yet
- [Generic crates still name title packages by path](handover/tooling/generic-crates-still-name-title-packages-by-path.md) - 2026-10-06: `check-title-reach` ratchet landed (190 references, 51 files; `oag-hud` at zero); the per-crate counts are the backlog for moving each to `Title` fields and dropping the Cargo edges
- [Do the originals have motion blur? Never checked, and the docs contradict themselves](handover/rendering/do-the-originals-have-motion-blur.md) - 2026-10-05, queued by the maintainer, deferred to a new session: Pulse/HD/Omega leads point to yes; an `original` setting value is decided for any title whose law is recovered at 70+.
- [Motion blur grain: the tile wobble is gone, the tap dither is what is left](handover/rendering/motion-blur-grain.md) - 2026-10-05: the wobble was ~80 % of the grain energy; a bilinear tile lookup replaced it (4.12 -> 3.44 hp-RMS, no GPU cost); the dither and half-res remain.
- [Button prompts: what is left after the first landing](handover/frontend/button-prompts-follow-ups.md) - 2026-10-06: per-family prompt glyphs (PromptFont substitute, disc glyph kept for a PlayStation pad) are wired for Pulse and HD menus and EndRace; open: a live Steam/Deck check, the in-race HUD and pause overlay, Pure and Omega tables, and a legible wide key on a 12 px face
- [Omega's HUD draws off HD's five roots and its own `.gnf`; the 2048 skin set, Zone classes and the runtime tables are open](handover/frontend/omegas-hud-draws-and-these-parts-are-chosen.md) - 2026-10-06. Five roots at HD's paths (measured off the race managers), nine BC7 `.gnf` atlases top-down with no row reversal, the HUD font at half texel size; the two HUD-atlas WARN lines are closed. Open: which race reads `2048_hud\`, Zone class text, HD's runtime tables are chosen not measured
- [HUD legibility and HUD skins across titles](handover/frontend/hud-legibility-and-hud-skins.md) - 2026-10-06: Pulse's glyph body matches the original at 1x but our glyph quad is clipped to the metric box (the baked halo is cut; `borderExtendPixels` 5/3 is read by nothing), plus a 3x linear stretch; HD/Omega/2048 ship Pulse's HUD atlas at exactly 4x but not its typeface; HD/Omega skins `2097` (loyalty 6000) and `wo3` (10000) reachable, 2048's are not; config-skin design in `docs/ui/hud-skins.md`
- [Airbrake flaps: the swing sign is checked, not read, and only the player's craft moves](handover/rendering/airbrake-flaps-need-the-titles-own-handler-and-rival-craft.md) - 2026-10-05: HD, 2048, Omega and Pure swing their flaps about the hinge's local X; the direction is asserted by ground truth, HD's own handler is unread, rivals stay stowed.
- [AI fork choice: what is left after the coin](handover/gameplay/ai-fork-choice-leftovers.md) - 2026-10-05: opponents flip the original's coin per fork (`Ai_ChooseBranch`, all four titles) and drive their route on its own plan, progress read off the route; open: routes our craft cannot drive (Pulse 07, 2048 cathedral and sol) are never drawn, 2048's circuit override and re-commit, runtime confirmation.
- [Where a title still runs a chosen rule while Pulse has a recovered one](handover/gameplay/titles-without-a-measured-rule-should-inherit-pulses.md) - 2026-10-03: the fire law and the gantry clock (HD: frame 203, tick 70) are inherited; wreck/spark effects, Pure's VECTOR lap count and Zone ship handling still are not.
- [Zone hull is shared on 2048 and Omega; native-craft livery and `ship_zone.vex` open](handover/gameplay/zone-craft-livery-and-omega-2048-era-craft.md) - 2026-10-06: livery swap wired off the definition's `zone` model; Omega 2048-era craft race; native craft keep the default skin (fallback unread).
- [Pulse's cloud field: the far-camera skip and a live A/B picture](handover/rendering/pulses-cloud-field-far-skip-and-live-picture.md) - 2026-10-04: the field (scatter, cull, ramp, atlas) is built and reproduces four boots' RAM exactly from their seeds; open: the far-camera draw skip (caller of draw slot `0x08ad2a68`) and a matched PPSSPP frame (placement near the clouds kept failing)
- [Bloom has no setting, and Pulse PSP still reads 1.2-1.6x the original on a straight](handover/rendering/bloom-has-no-setting-and-pulse-reads-a-little-strong.md) - 2026-10-02: ours was 4.4-5.7x the original over a racing frame (a background wash, fixed by per-tap truncation; ours now 1.2-1.6x); no setting by decision (removed); Pulse PS2 now blooms its own way (mask and chain read off replayed GS dumps, ours 1.06x the original on one grid frame); 2026-10-04: the PSP composite now lands over the HUD, as the original's queue draws it; open: the PSP residual, a second PS2 circuit and a racing frame, HD strength
- [The status-map audit: what it could not settle, what it found stale elsewhere, and the gaps it ranked](handover/tooling/the-status-map-audit-left-these-open.md) - 2026-09-23; about 25 stale status.md cells fixed, this carries the unsettled cells, the level calls for a maintainer, about 20 stale lines outside status.md and the ranked gap list
- [Pulse's Zone announcer is a sequence; a wider set of cues is still sampled instead of played](handover/audio/pulses-zone-announcer-is-a-sequence-and-others-are-sampled.md) - 2026-09-29: `zone_N` is ZONE + number + CLEAR on authored delays and now plays as one stereo voice (master tick 258.4 Hz, measured live); 18 of Pulse's 33 wired cues now play their timeline as layered voices (2026-09-29); `~BLOWUP`, `~ROCKLOCK`, the circuit emitters and Pure/HD/2048 keep the flat pick.
- [HD races play HD's own weapon table; these blocks are authored and unread](handover/gameplay/hd-races-play-hds-own-weapon-table-and-these-blocks-are-unread.md) - 2026-10-06: the table is HD's, behaviour is Pulse's inherited law; LightBarrier, EMP, the Detonator table, Cannon `recharge_time` and the Bomb's shot count are authored and read by nothing.
- [Pulse's weapon cues are read and mostly unwired - the wiring plan](handover/audio/pulses-weapon-cues-are-read-and-mostly-unwired.md) - 2026-09-30: all four remaining triggers recovered and wired (`MISSILEEXPSHIP`, the Missile fuse's `SHURIKENEXPL`, `~QUAKETRAVEL`, `LEACHFAIL`, `SHURIKEN`), the Shuriken's cues moved onto the blade's own emitter, and the Cannon round now lives 1.0 s; still open: none of them heard by a human, `~QUAKETRAVEL` is one voice where the original has one per span.
- [HD/Fury's engine sound: level and authored mix are measured; the stereo pair, per-cue groups and the distance writer are open](handover/audio/hds-engine-sound-is-a-per-team-crossfade-table.md) - 2026-10-05; `0x400` is unity and the volume word is squared (`K = 0.295`, two boots), port levels follow it; open: HD music/ambience about 3x hotter than the original, distance factor is a fit, each layer is a two-voice pair
- [2048's cruising engine is a gearbox beside the crossfade](handover/audio/2048s-cruising-engine-is-a-gearbox-beside-the-crossfade.md) - 2026-10-05: the five `<team>2048` crossfade tables play (`Ship_NGP.bnk`, five channels, kind-2 modulator; names are FNV-1 hashes in 2048's v5 banks); open: the gearbox cues that carry cruising (251 of 254 resolve), channel 4's source and the class word (chosen), 2048's other cues still read HD's banks
- [2048's and Omega's circuits play no trackside ambience](handover/audio/2048-and-omega-circuits-play-no-trackside-ambience.md) - 2026-10-06: 2048 (Altima 40 of 41, base circuits 174 of 226, DLC 212 of 468, read off the executable) and HD (all 16 circuits, five shared banks) resolve; the rest are the disc's dangling references. Open: Omega's Wwise cue mapping, HD's Modesto Heights and Tech De Ra manifests (name Vineta K's bank), an ATRAC9 decoder for 2048's `speech_*_NGP` streams.
- [Is HD's Zone speed-class announcer voice processed at runtime?](handover/audio/is-hds-zone-class-announcer-voice-processed.md) - 2026-10-06: maintainer remembers a distorted voice; the port plays the `MR_*` waveforms raw; open: baked into the waveform or a runtime effect, settled by an objective WAV comparison, a read of HD's speech path, or an RPCS3 capture.
- [HD's hull washes white on boost and blooms off it; the original's does not](handover/rendering/hds-engine-light-washes-the-hull-on-boost.md) - 2026-10-02: live on RPCS3 the original's `Distance = 0.0` Assegai and Harimau hulls stay clean with the player's own light at 360, so the wash is ours and not per-ship-correct; blend, bloom, colour and light numbers are closed; what is left is the `EdgeGeom` per-vertex weight on a hull (space, normal, light subset), unread - do not tune the bloom
- [The HUD's message lines raise one chosen event; the original's eight have no trigger](handover/frontend/the-hud-message-lines-raise-one-event-and-eight-have-no-trigger.md) - 2026-10-06; Pulse's `Info1`-`Info4` drawn with the decompiled slot law, Zone/Speed Lap medal line chosen not measured, live frame and `MESSAGE` cue open.
- [HD's end screens: Results/Menu/Rewards/Podium read and drawn, Podium multiplayer-only](handover/frontend/hds-end-screens-are-located-and-not-read.md) - 2026-09-21; Rewards settled as never entered by the original 2026-09-25 (drawn by `--menu-page` only); `Grid{col}.{row}` is four columns by ten rows, not eight by ten.
- [The weapon-absorb effect: burst on Pulse, Pure and HD, HD's absorb shell drawn, Pulse's hull overlay drawn](handover/rendering/the-weapon-absorb-effect-plays-in-no-title.md) - 2026-09-23: burst on every title; HD's shell and Pulse's hull overlay drawn, both confirmed live; Pulse's hull lighting, glow mask and bloom now measured and ported.
- [WESL shaders: follow-ups after build-time validation](handover/rendering/wesl-shaders-semantic-errors-point-at-generated-text.md) - 2026-10-06: build-time `naga` validation landed (`oag-shader-check`). Open: split `lit_texel`, fold FSR 3 per-pass helpers.
- [The race scene build no longer freezes the loading screen, and on a desktop it no longer takes seven to eight seconds either](handover/rendering/the-race-scene-build-takes-eight-seconds.md) - 2026-09-25: built on a `race-build` thread now; a shared shader module and a pipeline cache cut the build itself from 7.2-8.1 s to 195-372 ms on desktop (Pulse PSP/PS2, HD Fury), byte-identical screenshots before/after; only the Steam Deck measurement is still open
- [PS2 Pulse's shield: the model and the tint are fixed against a GS dump; what is left](handover/rendering/ps2-pulse-shield-unmeasured-remainder.md) - 2026-10-01: the PS2 draws `<Team>\extrashield.vex` (literal prefix `extra`), not `shipshield.vex`, and never tints its vertices; both fixed. Open: a pixel-matched frame (waits on the PS2 chase camera), the cockpit sphere, `FUN_001df718`
- [Rockets, mines, the bomb and the shield look close, and none is verified frame-by-frame](handover/rendering/rockets-mines-bomb-shield-look-close-and-are-unverified.md) - verification thread from play 2026-09-17; 2026-09-24: Rocket basis, Mine and Bomb poses measured live and drawn; 2026-10-01: matched-state Rocket/Mine/Bomb/Shield side by side done, the Rocket's flight law measured (222.22 u/s cruise, 0.75 x for four frames, spawned at the craft) and four `crates/gameplay` rows open; second pass the same day: the Bomb's ring was an unplayed `Anim Transform` (now played), Mine and Bomb laid at the craft's own position, the Shield's late onset is the original's jittered frame `dt` (nothing to port); third pass: the PSP shell's brightness/banding gap was a linearly decoded pre-swizzled texture plus a missing scroll, both fixed (`vex::textures` now unswizzles every flagged node on every version), matched to the original at 0.9 correlation
- [The LeachBeam against the original, after the 2026-09-23 rebuild](handover/rendering/leachbeam-fidelity.md) - ribbon and hull overlay rebuilt and measured in play; the track-tube bend is built and measured on PPSSPP (2026-10-01, six captures; only the locator radius and its interpolation remain), the lock sight's own law is ported (2026-10-01: spinning arrowheads, replayed against live frames), bloom spread and a few smaller gaps remain (the ENERGY instance is killed at each re-spawn and on retire since 2026-09-30) 2026-10-01 (`pulse-hull-bloom`): the hull-bloom gap is closed - the original's shadow pass wipes the overlay's mask (software renderer; PPSSPP's OpenGL backend keeps it); `hull_overlay::stamps_mask`; the PSP Concept race raises `extrashield.vex`.
- [HD/Fury's weapons draw Pulse's fallbacks, not their own models and effects](handover/rendering/hd-furys-weapons-draw-pulses-fallbacks-not-their-own-models.md) - every weapon model entry is a Pulse PSP name; HD authors its own `.rcsmodel` set per weapon, Plasma's triggers already read; 2026-09-24: placement a rotation, HD bodies culled as authored; 2026-09-25: the LeachBall draws through its own program (`RIM_GLOW`), the Plasma head's program is routed (`RIM_EDGE`) but held off by the cull thread below, and the explosion sphere's `noise.gtf` is read (rides `UV_offset`, left unwired: its program is lit and reflective); 2026-10-05: `UV_offset` is the blast's age and is played on the ring and halo (`CLOCK_SCROLL_RING`/`_HALO`); Missile's pair and the Bomb's five stay unwired, no trigger read.
- [HD's Bomb blast pair has no scene block, and the LeachBeam's strips are unread](handover/rendering/hds-bomb-blast-scene-block-and-leach-strips.md) - 2026-10-06; every other HD weapon drawable now takes the circuit's scene block; the Bomb blast pair is unwritten, the beam's `0x63d0` strips undrawn, the ghost static is by design (no executable string) but still warned
- [Closing oag-trace to a second emulator: PCSX2 is done, RPCS3 has no transport, Vita3K/ShadPS4 have no tooling and ShadPS4's install is broken](handover/tooling/closing-oag-trace-to-a-second-emulator-pcsx2-is-in-reach.md) - investigated in priority order PCSX2 -> RPCS3 -> Vita3K -> ShadPS4.
- [After the finish the race keeps running under AI: the leftovers](handover/gameplay/after-the-finish-the-race-keeps-running-under-ai.md) - 2026-10-01: measured and ported for Pulse PSP (AI flies the craft, world keeps ticking, node-view spectator camera, `Race End Photo` for line finishes); 2026-10-04: the finished player's thrust law (56.7 is `AI_ComputeOpponentThrust` on its own place) and `Race End Photo`'s d-pad with camera modes 1 and 4 ported; open: what a wreck waits for (the original never leaves `InGame`), the station-camera wall, other endings and titles
- [The pre-race flyby plays with its panel; its settle and its tail do not](handover/gameplay/the-pre-race-flyby-lacks-its-panel-and-its-settle.md) - 2026-10-01: the camera is `start_grid.vex`'s `grid_camera1` animation, ported (`oag_raceplay::intro_camera`, `--no-intro`); 2026-10-02 the track-description panel landed (`oag_game::track_panel`, measured fade, pointer skip); the craft's settle, the music and the end-of-flyby wash are not
- [Split screen, multi-window and network play: one shared prerequisite, then three efforts in dependency order](handover/gameplay/split-screen-multi-window-network-play-in-order.md) - planning thread, not an RE finding; the shared N-player prerequisite landed 2026-09-16 (ADR-0052), split screen recommended first, network multiplayer last and largest.
- [The default log is quiet now; two things in it are still not right](handover/tooling/the-default-log-is-quiet-now-these-four-things-are-not.md) - 2026-10-02; the per-frame `frame: N ms` warn floods on software Vulkan, DLC problems that never reach `warn` on the race path.
- [The workspace is split into 41 crates; the next three splits are ranked and unstarted](handover/tooling/the-workspace-is-split-further-splits-are-ranked-and-unstarted.md) - 2026-10-05; `oag-game` and `oag-raceplay` are each still about 51k lines: race `load`/`scene`, a profile crate, the boot and media cluster, plus ungated loose ends
- [2048's end-race pages draw; Results' table, XP, parade laps and the way back are open](handover/frontend/2048s-end-race-pages-draw-the-rest-is-named.md) - 2026-10-06; `RaceSummary`/`ObjectiveSummary` off `NEWGUI/EndRace_Definition.xml` after a real event race, pad and pointer, retry and exit; `oag_title::FrontEnd::endrace_style` replaces three `title.name == oag_hd` branches; Omega ships the file byte-for-byte, not wired.
- [`--trace-out` still ignores `--give`/`--autopilot*`; scripts lose their analog axes in `oag-game`](handover/tooling/trace-out-drops-give-autopilot-and-script-analog-axes.md) - the `--trace-out --input-script` bug is fixed (`write_trace` now drives through `HeldButtons::advance`, pinned by a disc-backed test).
- [The EU name gap: 115 closed by exact hash, 182 functions (plus data) remain](handover/tooling/the-eu-name-gap-115-closed-by-exact-hash-182-functions-remain.md) - [ADR-0048](docs/architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md) made `psp-pulse-eu` the Ghidra target of record.
- [PPSSPP symbol bridge: export and harvest land, the Ghidra import direction does not](handover/tooling/ppsspp-symbol-bridge-import-direction-not-run.md) - `kotcrab/ghidra-allegrex` ships `PpssppImportSymFile.py`/`PpssppExportSymFile.py` to move symbol names between Ghidra and PPSSPP's Debug menu.
- [Each circuit's airtime budget is measured, and eleven of twenty-four cannot show a barrel roll to anyone](handover/gameplay/each-circuits-airtime-budget-is-now-measured.md) - the AI barrel roll is gated on a minimum airborne duration, and on a full grid only **Ace** ever armed one. The cause is the track.
- [A projectile decided wall from floor by angle; the surface class was there all along](handover/gameplay/a-projectile-rode-the-angle-not-the-surface-class.md) - player report on HD, "rockets go straight or disappear in a wavy section".
- [A craft hit by a weapon slows down; the port is in, the runtime check is not](handover/gameplay/a-craft-hit-by-a-weapon-does-not-slow-down.md) - reported from play and **missing, not mistuned**. Recovered 2026-09-06 out of the PSP executable and written up on [engine.md](docs/ghidra/functions/psp-pulse-usa/engine.md).
- [Outpost 7 loses 34-35 shield a lap to a corner no craft can hold at the speed the AI picks](handover/gameplay/outpost-7-loses-35-shield-a-lap-to-its-own-walls.md) - steps 1-6 done by 2026-09-07 and all in the thread file: the loss is kinematic (a line of curvature `k` admits `1.55 / k`, 07 peaks at 0.047), the yaw-rate cap is in `driver::pace::corner_target`.
- [Pulse craft variants are gated on loyalty; one byte and the live check are not](handover/frontend/pulse-craft-variants-wait-on-the-any-row.md) - 2026-10-02. **Landed**: circuits (`oag_game::unlock`) and craft variants (`loyalty_unlocked`: Exclusive rows OR, `any` = best single team) gate, the Loyalty block is drawn, `--unlock-all` ours. **Open**: byte `+0x99`, a live check of the `any` branch; RACE REMIX stays ungated by decision (ours).
- [The original resets a craft airborne for four seconds, and ours now does](handover/gameplay/the-original-resets-a-craft-airborne-four-seconds-and-ours-now-does.md) - 2026-09-29. **Landed**: the original's sunk-craft recovery (floor hull contacts, `Collision_AddContact`'s projection gate, `Body_StepWorld`'s pass 1), its airborne reset (four seconds with no probe touching, `0x08841d30`, watched live on `01_Track` via the dev-unlock byte), and a gap-aware AI brake (chosen, maintainer decision) so the AI flies `01_Track`'s lip as the original's field does. Survey 589 rescues to 9. **Open**: our reset's relocation and shield charge differ from the original's state 3; the 200-unit distance trigger is unported; the player's autopilot grinds a wall on `05_Track` (also on `main`). Write-up: [leaving-the-track.md](docs/gameplay/leaving-the-track.md)
- [An Ace craft should lap every circuit, in every speed class, without touching a wall](handover/gameplay/an-ace-craft-should-lap-every-circuit-without-touching-a-wall.md) - the umbrella the narrower AI threads sit under, opened 2026-09-12: a lone Ace should lap every circuit clean in every class with no wall contact. **2026-10-03: the AI follows a speed plan learned in our own physics** (`oag_ai::SpeedPlan`, ai.md "The speed plan"): `ai_clean_lap_gate` Eliminated 11 -> 1 and contact 8,828 -> 1,243; over 24 layouts x 4 classes lone rows with no contact, rescue or death 7 -> 84 of 96. 2026-10-03 later: `05_Track` forward's plans verify (the line left a magstrip on the first jump's run-up; the walls were a pit under the upper road), 91 of 96. Open: five plans that do not verify, three verified rows that still touch, the team axis, Pure/HD.

- [The race-setup previews are meshes on Pulse and HD, sprites on Pure](handover/rendering/the-race-setup-previews-differ-by-title.md) - the track and craft previews are **rendered 3D meshes** on Pulse PSP, Pulse PS2 and HD.
- [Pointer input: what the mouse and the touchscreen still cannot do](handover/frontend/pointer-input-what-is-left.md) - every front-end screen answers a mouse and a finger and each title draws its own cursor (`docs/architecture/menus.md`, "A mouse and a finger"); left open are a live pass over Pure's language picker.
- [A race box that mixes titles needs four rows off the title, not one flow](handover/frontend/a-race-box-that-mixes-titles-needs-four.md) - the architecture delta against `assets/ui/menu.toml` and [menus.md](docs/architecture/menus.md), whose "the tree is ours, the presentation is the disc's" boundary this does not move.
- [HD/Fury's flyer cards are drawn; light, bloom, the swing and the widget's own pose are open](handover/frontend/hd-flyer-cards-are-drawn-and-lack-light-bloom-and-the-swing.md) - 2026-09-30. Sixteen grid cards and `Campaign Selection`'s two, one flat-card path off each flyer's own camera; what is chosen and what is open is in `docs/ui/campaign-screens.md`.
- [Omega's menu backdrop draws behind the menus and the campaign stills and the live campaign stage; end-of-race screens and HD's own HD style are open](handover/frontend/omegas-menu-backdrop-draws-behind-the-menus.md) - 2026-09-30. The HD-style `BackgroundAnim` scene (`FrontEndScene_HD_ATG`, flown through its own camera, Roberts-cross edge and fill filter) draws on Omega only, unvalidated against the original; nine names in `menu-backdrop-scene.md`.
- [The campaign grid draws and does not launch](handover/frontend/the-campaign-grid-draws-and-does-not-launch.md) - 2026-09-21. `Grid Selection`/`Cell Selection` draw and launch, off `CellMode_Definition.xml` and the real 236-cell campaign; the tip ticker, the `Confirm`/`Back` footer legend.
- [Pulse's race box is read on both pressings, and the code side is unblocked](handover/frontend/pulses-race-box-is-read-both-pressings.md) - **2026-09-10**: the hexagonal window is a slideshow of the circuit's own stills, built on both pressings; open: the cards' transition. **2026-09-27**: the PS2 capture happened - Track Select/Ship Select match the disc exactly on PCSX2; found `RACEBOX` authors `WEAPONS`/`KILLS` rows this build has no setting for yet (shared with PSP, not a PS2 bug). **2026-09-30**: `KILLS` row landed (`race.kill_target`, disc's own `Eliminations` list); `WEAPONS` row landed the same day (`race.weapons`, disc's `Weapons` list, editable in a single race and pinned to the mode's answer elsewhere), and the blank SPEED CLASS in `--menu-page race` was the capture not supplying its list; open on the page: AI DIFFICULTY's `N/A`, Zone's text over SPEED CLASS. **2026-09-28**: the cards' and the panel's arrival transition now fades in, off `LeftLayer`'s own `transition` attribute (a duration, not a slide - `Widget_CreateFromElement`, confidence 72); open: the outline's `Mode3D` pose.
- [Pure splits the race box into a chain, and 2048 has none at all](handover/frontend/pure-splits-the-race-box-into-a-chain.md) - Pure keeps the screen *names* and inverts the *shape*: no `Racebox`, no hex grid, no grid editor, and a linear one-choice-per-screen chain where Pulse puts five rows on one page.
- [`RESIDUAL_SHARE` is one circuit, one adapter, and now points at AI/physics cost rather than `hd_bloom`](handover/rendering/drs-residual-share-is-one-circuit-one-adapter.md) - [ADR-0043](docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md) timed `hd_bloom` (the gap ADR-0042 left open) and moved it into `drs::Cost::scalable`.
- [The Omega race peak no longer holds every texture on the CPU before any is uploaded](handover/rendering/omega-race-peak-holds-every-texture-cpu-side-before-any-upload.md) - Tech De Ra 3,417 to 792 MiB peak by uploading as it decodes (`race::TextureSink`); the open ones are the sink-less routes, HD's `sky.gtf`, an unmeasured flush rule and two small `.gnf` follow-ups.
- [Texture compression only covers BC; ASTC is the unimplemented other half](handover/rendering/texture-compression-only-covers-bc-astc-is-the.md) - `optional_features` requests `TEXTURE_COMPRESSION_BC` only, so every Apple Silicon Mac (Metal, native ASTC, no BC decode) silently takes the CPU-decode fallback meant for the untested GL backend.
- [2048's GXP containers decode; the USSE stream's opcode field is located, semantics do not](handover/tooling/2048s-gxp-containers-decode-the-usse-stream.md) - **97,899 of 97,899 Vita shader containers decode** over 22,056 files, header through the named parameter table ([gxp.md](docs/formats/gxp.md), `oag_rcs::gxp`).
- [A wrecked craft draws its wreck, explodes, and the player is cut to the circuit's own camera](handover/rendering/a-wrecked-craft-explodes-and-the-player-is-cut-to-the-circuits-camera.md) - 2026-10-01: wreck swap, `WO_SHIP_EXPLOSION` placed at the wreck's matrix, the player's destroy camera (the circuit's own `Camera` node nearest the aim, zoomed to 35 units) and shakes ported and measured on PPSSPP; the white fireballs were twelve 4 bpp `.pob` sprites the reader refused; 2026-10-01 (pulse-fx-recheck): the explosion's matrix scale (0.75 on spawn offsets and velocities), the run law and the `Bomb_Shockwave.vex` ring (`ShipShockwave_Update`) landed, fire/smoke pixel counts within 0.7-1.6x of the original's; 2026-10-02 (pulse-wreck-3): a race a wreck ended steps its cosmetics under the results (`Race::tick_cosmetics`) and the Eliminator waits state 8's `1.0` s (player) / `0.8` s (others) - both live-measured, the others' was `2.0` before so the destroy camera holds through the explosion; 2026-10-02 (pulse-state6): measured live that a single race's wrecked AI craft stays in state 6 for good, so the port no longer returns it (`eliminator.rs`); open: the opponents stay frozen under a wreck's results, mode-2 emitter spread
- [The Pulse frame audit fixed texture level selection and left two differences it could not trace](handover/rendering/the-frame-audit-fixed-texture-level-selection-and-left-two.md) - 2026-09-30: three circuits matched to the original at 480x272 with no filters; a PSP `.vex` texture now uploads at one mip level (the original never leaves the base level in a race frame); the hull's `0x2000` env-map extra pass is now drawn (`oag_render::shine`, six batches per team, closes a third to a half of the spine gap; fogged to black, and a circuit's own `*_shinemap` batches drawn under the view-matrix matcap 2026-10-01; airbrake deflection on the pass closed 2026-10-01, and the wreck model drawn from `Ship_SetState` case 5 (it has no extra pass; 2026-10-01); the tunnel rims' neon strip closed 2026-10-01 - it was blended glow batches not stamping the bloom's mask, now stamped, see `docs/rendering/glow-mask.md`); the factory roofs are fixed (they were four animated slab transports the original's sections hide; the section mask now applies to moving draws, found with `scripts/psp-ge-dump.py`'s GE-list census, and the original's second tier - the section's own box against the view - is ported as `oag_render::pvs::sections_in_view`, 2026-10-01); open: whether real hardware shows level 1 past ~256 units; TEXTURE DETAIL (original/high/maximum) landed and `--anisotropy` works on a PSP model again (`textureSampleGrad` at the slope law's level)
- [Shadows are planned as four techniques, and the `Dynamic Shadow Occluder` hull is decoded and read at runtime](handover/rendering/shadows-are-planned-and-pulses-occluder-payload-closes.md) - RE only; the implementation half has its own thread, below. Pulse's `0x3c3` occluder, HD's `LiveStencilShadow`/`shadow.stencilvolume` path (all 39 real files decoded.
- [All four shadow tiers draw; `mapped` has one named gap and 2048 has none of it](handover/rendering/original-on-pulse-is-the-tier-still-unbuilt.md) - the implementation half of the thread above, and **steps 2 and 3 of [shadows.md](docs/rendering/shadows.md)'s plan landed 2026-09-04**: `graphics.shadows` (`off`/`blob`, per title.
- [Dynamic resolution wants a viewport, not an allocation](handover/rendering/dynamic-resolution-wants-a-viewport-not-an-allocation.md) - **landed 2026-09-02 and the feature works, off by default**; the file survives only for what it did not answer.
- [FSR 3.1 is ported, wired and selectable; nobody has played it](handover/rendering/fsr-3-1-is-a-seven-pass-port-and.md) - **all eight passes, wired through `upscale::Framebuffer::resolve_scene` and reachable from the UPSCALER row**, off by default.
- [HD's pads author an emissive mask and a material of their own; nothing reads either](handover/rendering/hd-pads-author-an-emissive-mask-nothing-reads.md) - the invented pad tint is gone and a pad now draws the colour its own texture paints on every title ([pads.md](docs/rendering/pads.md)); this is the half that fix did not close. **Wiring attempted 2026-09-25 and stopped, nothing changed in `oag_render`**: the `_ne`-gated glow term is confirmed, by register trace, to be multiplied by the pad's own specular scalar before it reaches output, not just added - and that scalar's own chain passes through opcode `0x3b`, which this project had read as a normalise/rsq guess at confidence ~70 and deliberately declined to name. **The opcode blocker is cleared, same day**: `0x3b`/`0x3d` are now named `DIVSQ`/`FENCT` off RPCS3's own opcode table (`rpcs3/Emu/RSX/Program/Assembler/FPOpcodes.h`), corroborated disc-wide (183,623/59,256 uses, confidence 84/90) - both pad programs' specular chains use only `0x3b`, never `0x3d`. Still nothing wired: a `mesh.wgsl` tangent frame is the remaining blocker - see the thread's Next Steps item 1.
- [The frame path allocated 1.6 MB a frame; five reasons are fixed and the rest are counted](handover/rendering/the-frame-path-allocated-16-mb-a-frame.md)
- [Race records persist across a restart, and the results table now shows one](handover/frontend/race-records-persist-and-need-an-on-screen-surface.md) - best lap, best total time, the last result and (unfed, still) a campaign medal survive a restart (`crates/game/src/records.rs`, `<config dir>/oag/records.toml`).
- [Invented UI text has no translation - the mechanism and every row's `string_id` are wired; French is the first real translation](handover/frontend/invented-ui-text-has-no-translation-and-the.md) - the mechanism landed: `oag_game::strings` merges a project-owned `assets/ui/strings/<language>.toml` over a `StringTable`, project entries winning.
- [Brazilian Portuguese: nobody has reviewed it, and a German reader artifact](handover/frontend/portuguese-br-review.md) - `PortugueseBR` is translated on Pulse, Pure, HD and 2048 (Omega ships its own), every entry `human = false`; review and the empty-continuation reader artifact remain.
- [HD/Fury rendering accuracy pass: the light cone is fixed, the frame-wide darkness is ranked first and still open](handover/rendering/hd-accuracy-pass-2026-10-05.md) - matched-pose luma gaps per circuit, the Amphiseum floor narrowed to missing light (not art), sky sign flip, and what was not compared
- [HD's sprite flare now draws by its own measured law and skips the viewing player's own craft (confirmed live, 2026-09-15); three tuning rows and the trail's own accuracy stay open](handover/rendering/hds-sprite-flare-reads-oversized-and-the-tuning.md)
- [HD's glow draws, and the coordinate it samples on is the one thing not read](handover/rendering/hds-glow-draws-and-the-coordinate-it-samples.md)
- [HD's shield hit-flash is amber, not Pulse's cyan - and the steady-state colour is a parameter this pass did not trace to its source](handover/rendering/hds-shield-hit-flash-is-amber-and-the-target-colour-is-a-parameter.md)
- [2048's four Vita eboots are decrypted and imported; loose ends from getting there](handover/tooling/2048s-vita-eboots-are-imported-re-not-started.md)
- [Race start: the countdown state machine, the launch reaction boost, and whether Zone shows a different countdown](handover/gameplay/race-start-countdown-and-launch-boost.md) - the gantry-placement blocker is resolved; the gantry is placed, drawn and playing the countdown on Pulse and HD. Pulse's after-`GO` law is read (2026-10-04, race-manager windows by lap, 85). **Open**: the lap states uncaptured on PPSSPP, one live `mesh+0x40` read, HD's stray strip past 6 s, PS2 Pulse inheriting the PSP windows unread.
- [Pulse's start pose: the grid walk is what is left](handover/gameplay/pulses-launch-boost-and-the-grid-walk-are-the-start-pose-leftovers.md) - 2026-10-01. The standing start was not off by its pose (landed, [grid-state.md](docs/physics/grid-state.md)) and the launch boost landed ([launch-boost.md](docs/physics/launch-boost.md)). The grid walk landed 2026-10-02 ([grid.md](docs/ghidra/functions/psp-pulse-usa/grid.md#the-grid-walk-read-to-the-end-2026-10-02)). Zone's four-corner epilogue was read the same day (same guard, gain `50.0`) and the flag stays off there. The perfect-start effect is wired (2026-10-04). **Open**: the AI's launch (grade 3 read at 85; the in-rule port failed ten test-data tests, cause not isolated), Zone's engine in state 0, the walk's junction hop, `25_Track` reversed, the AI board's `04_Track` row.
- [2048's PSARC and title plumbing are wiring-ready; two binary formats changed underneath a track](handover/tooling/2048s-track-vex-parses-but-two-binary-formats-changed.md)
- [Streaming decode for audio would break seek, and nothing forces the change yet](handover/audio/streaming-decode-for-audio-would-break-seek-and.md)
- [HD's cues end loud and were cut dead, and its `.COLLISIONS` is a flattened tree](handover/audio/hd-collisions-flatten-a-severity-tree.md)
- [The audio queue is ours now, and the frame stalls are what is left](handover/audio/the-device-queue-is-sized-off-the-configured-cap.md)
- [A reported skip is not in the mix we render, and the jump counter cannot see it](handover/audio/a-reported-skip-is-not-in-the-mix-we-render.md) - the PS-ADPCM run-out block was played once per loop and is now trimmed; the reporter can no longer hear the skip, and 2026-09-15's two `--tap-audio` runs (Pulse and HD, windowed) found no gap.
- [A circuit's billboard slots are a 9-entry array on the engine side, and slot 7 is not what it says it is](handover/rendering/a-circuits-billboard-slots-are-a-9-entry.md)
- [A parser cannot fail on a field it does not know about, so coverage is now measured](handover/tooling/a-parser-cannot-fail-on-a-field-it.md)
- [A chunk header is 0x20 bytes and then a *surface* record, and the byte after the layout says which space it is in](handover/tooling/a-chunk-header-is-0x20-bytes-and-then.md)
- [HD ships a per-chunk PVS, and drawing every chunk was the whole "meshes that should not be there"](handover/rendering/hd-ships-a-per-chunk-pvs-and-drawing.md)
- [HD's loading screen is recovered; the seven unnamed mode ids and one flag writer are left](handover/frontend/hds-loading-screen-is-recovered-from-its-own.md)
- [The race mix saturates; the per-voice SAS volume would settle whether it should](handover/audio/the-race-mix-saturates-the-per-voice-sas.md)
- [The Autopilot pickup is built off its own two handlers; three of its parts are still ours](handover/gameplay/the-autopilot-pickup-is-built-off-its-own.md)
- [Positional audio is recovered whole, and the pan is a table the disc computes from `cos` and `sin`](handover/audio/positional-audio-is-recovered-whole-and-the-pan.md)
- [A circuit's sound emitters parse, and nothing plays them](handover/audio/a-circuits-sound-emitters-parse-and-nothing.md) - **both `sound` and `soundcone` are audible now, and both without an emulator.** `oag_sound::sfx::TrackEmitters` opens a held voice for every in-range `sound`/`soundcone` node.
- [HD's exhaust is measured from the running game now: the trail is a 54-sample three-fin tube, the flame breathes with the throttle, and the plume scales rather than blinks](handover/rendering/hds-exhaust-is-measured-from-the-running-game.md)
- [HD's engine trail is one of four ribbons, and a craft flying through one sparks](handover/rendering/hds-engine-trail-is-one-of-four-ribbons.md)
- [HD's frame was too bright and too bloomy; the bloom chain was not what was wrong](handover/rendering/hds-frame-was-too-bright-and-too-bloomy.md)
- [HD needs a per-material shader path, and two general rules for finding one have been refuted](handover/rendering/hd-needs-a-per-material-shader-path-and.md)
- [Vineta K's glass refraction and arch glow: what is left](handover/rendering/vineta-k-glass-refraction-and-arch-glow-remainder.md)
- [A PS3 title now boots, drives and screenshots from a script - and its savestates are not worth adopting](handover/tooling/a-ps3-title-now-boots-drives-and-screenshots.md)
- [Reversed grids were putting craft off the track; fixed by walking the spline instead of extrapolating a straight line](handover/gameplay/reversed-grids-were-putting-craft-off-the-track.md)
- [2048's sky turns and its fog draws on HD's curve; the 2048 law, two sky keys and Omega's pink road are open](handover/rendering/2048s-sky-turns-and-fog-draws-on-hds-curve.md) - 2026-10-06: 2048 and Omega domes turn by `Sky rotation`, fog from `Lighting.Fog colour` on HD's inherited curve; open: 2048's GXP fog term, the sign, `Sky brightness`/`Sky height offset`, Omega altima's flat pink road
- [HD's sky, fog and lighting draw from the disc's own statements - three holds remain, each named where it lives](handover/rendering/hds-sky-fog-and-lighting-draw-from-the.md)
- [Sebenco and Vineta K: scalar maps fixed; ice water, sea colour and the sky tint stay open](handover/rendering/hd-sebenco-scalar-maps.md) - 2026-10-07: violet wall, purple ice and Vineta's red shape were scalar/normal/mask entries bound as colour; open: the sky tint is not `Sky colour/128` on Sebenco, the sea's teal, the ice water layer
- [A cue can play other cues, and that is what `.COLLISIONS` does on HD](handover/audio/a-cue-can-play-other-cues-and-that.md)
- [Every SFX trigger is a Pulse reading, applied to Pure and HD on a bet](handover/audio/every-sfx-trigger-is-a-pulse-reading-applied.md)
- [HD/Fury's particle effects parse, and a "field that disagrees" turned out to be a wrong check](handover/rendering/hd-furys-particle-effects-parse-and-a-field.md)
- [Pulse's draw order is recovered, and it is a layer sort rather than a depth sort](handover/rendering/pulses-draw-order-is-recovered-and-it-is.md)
- [Talon's Junction's "missing floor" is a glass floor drawn with the wrong texture: rendered, not absent](handover/rendering/talons-junctions-missing-floor-is-a-glass-floor.md)
- [`Material::state`'s upper bits: bit 7 is measured, mode 2 is now fully decoded](handover/rendering/material-states-upper-bits-bit-7-is-measured-mode.md)
- [HD's menus are drawn inside its own frame, and the `HD_*` palette turned out to be the FE style](handover/frontend/hds-menus-are-drawn-inside-its-own-frame.md)
- [HD's strip widget draws bare text; the real menu also draws a tab shape, an underline, and a background scene - the tab is now the executable's `Block`, the scene is the open half](handover/frontend/hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md) - **2026-09-14: the box behind every HD entry is `Block_Item.cpp`, read out of `EBOOT.elf` and drawn as it draws** ([menu-blocks.md](docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md).
- [Wipeout HD's video decodes, and the logo reel is the Studio Liverpool ident](handover/rendering/wipeout-hds-video-decodes-and-the-logo-reel.md)
- [Wipeout HD's soundtrack plays; finding it fixed a disc-wide PSARC bug](handover/audio/wipeout-hds-soundtrack-plays-finding-it-fixed-a.md)
- [Pure's soundtrack plays, and both titles' track names are recovered - Pulse's are deliberately unwired](handover/audio/pures-soundtrack-plays-and-both-titles-track-names.md)
- [HD/Fury's HUD draws, and three of Pulse's constants applied to every title were why it did not](handover/frontend/hd-furys-hud-draws-and-three-of-pulses.md)
- [Wipeout HD / Fury's HUD reads, and the reader it shares with Pulse was wrong in three ways Pulse's own data could never have shown](handover/frontend/wipeout-hd-furys-hud-reads-and-the-reader.md)
- [`.gtf` reads, so HD's textures are pixels - all 7,333 decode, and the ambient shadows they include are now drawn too](handover/rendering/gtf-reads-so-hds-textures-are-pixels.md)
- [Wipeout Pure races, on Pulse's physics, and what is still absent is a list of unrecovered entry names](handover/gameplay/wipeout-pure-races-on-pulses-physics-and-what.md) - **the entry-name list closed 2026-09-06** and both remaining rows were findings, not spellings. The Zone hull was never unrecovered after 2026-08-19.
- [Pure's particle effects decode and play, and two of five names do not resolve](handover/rendering/pures-particle-effects-decode-and-play-and-two.md)
- [Per-team hulls are drawn; which team flies which slot is recovered and ported](handover/rendering/per-team-hulls-are-drawn-which-team-flies.md)
- [A race flies a declared ship skin; what *selects* one is chosen, not measured](handover/rendering/a-race-flies-a-skin-what-selects-one-is-chosen.md) - the parser, the applier, `catalogue::Team::skins` and `--skin` are all in and verified on `pulse-psp-usa.chd` (100% of the player's texels repaint, every opponent's byte-identical).
- [The particle effects are played from the disc; 33 of the PSP's 35 play, 2 wait on a trigger](handover/rendering/the-particle-effects-are-played-from-the-disc.md) - 2026-10-04: shape 8 (half ring) and the class-6 pool bar are measured and played; the Repulser wave's one-frame `shazzam` whiteout remains (GE guard-band hypothesis).
- [Weapon blasts draw their own sprites and wash the screen; every flash kind with a caller is wired](handover/rendering/weapon-blasts-draw-their-own-sprites-and-the-screen-flash-is-next.md) - 2026-09-30: every flash kind with a caller is wired and checked live. 2026-09-24: particles draw their own `GU_TFX_MODULATE`d sprites, the sprite offsets are base-relative, and the Quake's `/ 50` is its extent co-factor. Second pass.
- [A struck craft throws its own hull sparks; they read within 10 % of the original's](handover/rendering/a-struck-craft-throws-its-own-hull-sparks.md) - 2026-09-30: the gap was 28 unparsed sprite templates (`shazam`, `glow`, ...) - now parsed and played; then matched per locator against the original and closed to within 10 %: a one-tick-late first draw and a square quad where the template is a stretched, rolling one (`+0xf0` aspect, roll channel); camera shake armed on a weapon hit. An emitter's own rotation (45 of 76 class-3) is played since 2026-09-30, read not measured live; next: measure it, re-shoot Missile/Shuriken/absorb/explosions. 2026-09-24: `Ship_Damage`'s weapon branch throws `WO_SHIP_COLL_SPARK_DAMAGE` (LeachBeam: its own variant) on one or two random hull locators per landed hit, severity 2.4, 0.8 s per locator.
- [The PS2's engine flare plays and reads as nothing on screen](handover/rendering/the-ps2s-engine-flare-plays-and-reads-as.md)
- [The default chase view is close on every title, and only Pulse PSP's default is measured](handover/rendering/the-default-chase-view-is-close-on-every-title.md) - 2026-10-01: maintainer decision, close everywhere; the discs' close and far blocks read for every title. The runtime eye scale is settled: 0.75 on every title (Pulse PSP and PS2 measured live, Pure/HD/2048 read from the binaries, HD/2048 0.7 in Detonator); default view still measured only on Pulse PSP
- [The race camera's FOV is fitted to the PSP's own aspect on every title, and the PS2's own in-game widening option has no counterpart](handover/rendering/the-race-cameras-fov-is-fitted-to-the-psps.md)
- [The AI's six-stage plan (line-follower to a field of pilots) is complete; what remains unbuilt is the residual](handover/gameplay/the-ais-six-stage-plan-line-follower-to.md)
- [The AI was never the problem: four contact-path bugs, and what is left after them](handover/gameplay/the-ai-was-never-the-problem-four-contact.md)
- [DLC packs are mounted; two things inside them are not read](handover/tooling/dlc-packs-are-mounted-two-things-inside-them.md)
- [Two recovered chase-camera behaviours are ported; `headtilt` is ported, its stick shaping and look-around are not](handover/rendering/two-recovered-chase-camera-behaviours-are-ported-headtilt.md)
- [The menus draw the disc's layout and its footer ticker; the page-transition easing curve is still invented](handover/frontend/the-menus-draw-the-discs-layout-the-chrome.md)
- [PS2 front-end layout is hardcoded to 480x272](handover/frontend/ps2-front-end-layout-is-hardcoded-to-480x272.md) - **2026-09-27**: PCSX2 pixel measurement settles the title-font question (don't flip `PS2_MENU_SKIN::title_font` - title and row glyphs measure the same 14px cap height); title position stays open with a 5px residual. Track Select/Ship Select walked on PCSX2 and match this build exactly.
- [Audio: race SFX plays on all three titles](handover/audio/audio-race-sfx-plays-on-all-three-titles.md)
- [Omega music plays 17 of 29 songs; eight-channel songs and the mix level are open](handover/audio/omega-music-remaining-eleven-songs-and-mix-level.md) - 2026-10-05: `Set_Music_Track_<location>__frontend` -> state -> switch -> segment -> stems, summed (level chosen); 11 eight-channel songs and location 0 unplayed by name; front end plays the `Menus` loop.
- [`oag-trace plan` has never been replayed into the emulator](handover/tooling/oag-trace-plan-has-never-been-replayed-into.md)
- [The ghost is drawn and raced, and not yet compared against the original](handover/rendering/the-ghost-is-drawn-and-not-compared.md)
- [M6 authored lighting: no hardware light slot found enabled, and no `DirectionalLight` consumer found anywhere](handover/rendering/m6-authored-lighting-no-hardware-light-slot-found.md)
- [`/psp-pure-usa`, `/psp-pure-eu` and `/psp-pure-eu-reimport` were missing from the project on 2026-08-10, despite the 2026-08-09 note below describing them as already imported and saved](handover/tooling/psp-pure-usa-psp-pure-eu-and-psp.md)
- [The EU/Pure imports have never been diffed](handover/tooling/the-eu-pure-imports-have-never-been-diffed.md)
- [Magstrip leftovers, none load-bearing](handover/gameplay/magstrip-leftovers-none-load-bearing.md)
- [Magfloor fx: drawn on Pulse from the recovered anchor; runtime check and Pure are open](handover/rendering/magfloor-fx-drawn-on-pulse-runtime-check-and-pure-open.md)
- [Magstrip effects on Omega, HD/Fury and 2048: the arc wake, the POB, the sound, the rumble](handover/rendering/magstrip-wake-omega-hd-2048.md) - 2026-10-05, magstrip-omega-law: the law is recovered and ready to wire - contact = surface-type-3 probe hit (our `mag_contact`), additive-RGB blend (ONE/ONE), one `arc_anchor_point` per hull (38 hulls, HD = Omega), arm/disarm sound start-on-enter stop-on-exit, three rumble XMLs on the edges. Open: the fifth probe's endpoints, the fragment program and depth state, the cue's loop flag.
- [Frame comparison: three residuals](handover/tooling/frame-comparison-three-residuals.md)
- [Weapons: thirteen of thirteen, the Repulser last (2026-10-04), and the dispatch table read whole](handover/gameplay/weapons-eight-of-thirteen-the-plasma-and-the.md) - **2026-10-04: the Repulser is read whole and built** - a blast, then two waves walking the track 5 points forward and 2 back a tick ([repulser.md](docs/ghidra/functions/psp-pulse-usa/repulser.md)); later the same day its field model is drawn, its third wave forks down a branch, and its timeline is read live on PPSSPP; open: the blast's bead ring and the wave-start whiteout differ from the original, and HD's Eliminator hands it out on Pulse's law. **2026-09-09: the Plasma's charge and detonation both closed, and the two findings point opposite ways.** The weapon *does* wind up before it fires - one second, held on the firing craft's nose.
- [A fired Cannon round: the body, its impact, its bolt streak and its muzzle flash are all drawn now](handover/gameplay/a-fired-cannon-round-is-drawn-by-nothing.md) - **2026-09-17: both hand-built quads landed.** The bolt is two crossed camera-facing ribbons between the round's last/current position, fixed white; the flash is one quad.
- [Both lockable weapons draw their reticle now, and the black background behind it is gone](handover/gameplay/the-missile-fires-without-a-lock-now-and.md)
- [Pure and HD lock on too, and it cost two axes rather than any new recovery](handover/gameplay/pure-and-hd-lock-on-too-and-two-axes.md)
- [The camera shake on impact is measured against the original and matches; only a real hard hit is still open](handover/rendering/the-originals-camera-hud-shake-on-impact-is.md)
- [PVS culling: three ceilings, one of them invented](handover/rendering/pvs-culling-three-ceilings-one-of-them-invented.md)
- [The HUD's four remaining items](handover/frontend/the-huds-four-remaining-items.md)
- [The lap count is per speed class, and three of four classes raced short](handover/gameplay/the-lap-count-is-per-speed-class.md) - 2026-09-09; `Mode::SINGLE_RACE_LAPS = 3` was a guess carried over from the time trial, retired by the campaign census: 3 Venom, 4 Flash, 4 Rapier.
- [Wipeout HD/Fury's campaign grids now parse; the Grid Selection screen is next](handover/gameplay/hd-furys-campaign-grids-parse-and-the-screen-is-next.md) - 2026-09-14, latest 2026-09-29. HD's own `Track Creation` (`Track_Selection_Definition.xml`) now opens on the RACEBOX path before `Team Selection` - emblem, name, length/distance, RECORDS, 12-wide carousel with two direction rows; open: hex grid, wireframe, fly-by movie. HD's own `Team Selection` (`DATA06`'s `Team_Selection_Definition.xml`) now opens between `Cell Selection` and the race and from RACE START - logo, per-model stat bars, pointer, Back to the same cell; open: the 3-D ship preview (no `ship_FE.vex` on HD) and the hex grid. Medals are confirmed per difficulty with a per-difficulty icon shape (`DATA04`'s `1024x768` Hexmedal atlas, three blocks); the archive-precedence and crop fixes landed, `CampaignRecord::best_difficulty` persists it. The block-to-difficulty mapping is now measured on RPCS3 (confidence 90: `easy=plain hex`, `medium=cane`, `hard=swirl`), and a real mode-dependent mismatch surfaced in the footer's own `DIFFICULTY (<rung>)` prompt text (`AI DIFFICULTY` on a `Race` cell, bare `DIFFICULTY` on an `Eliminator` one) - for `pulse-cellsel`/the lead to route. Open: whether the original stores a per-cell rung at all - see the thread's own 2026-09-28 section.
- [Pulse's EndRace screens are read and not drawn](handover/gameplay/pulses-endrace-screens-are-read-and-not-drawn.md) - 2026-09-14. A campaign race in the original ends in three authored screens, all of which now draw for Pulse. 2026-09-30: Zone and Eliminator have tables of their own (the Eliminator one confirmed on a live frame), bit `0x4` is the *visible* bit so the lap table shows `boostimg` and only its own rows, and the third column is the speedup pads entered on each lap (the writer found live in `Ship_ApplySpeedupPad`) and now draws.
- [Tournament and Head2Head are built; four RE gaps are still open](handover/gameplay/tournament-scoring-is-read-and-the-mode-is-next.md) - 2026-09-23, latest 2026-09-28. `oag_race::Mode::Tournament` races a leg exactly like a single race; the standings table (`PRO_POS`/`ER_TEAM`/`ER_POINTS`, cycling every 3s between this leg's own placings and the running total) now draws, off the copy site this pass located in `Race_BuildEndRaceResult`. `oag_race::Mode::Head2Head` fields the player plus exactly one AI opponent - measured off `AICount="1"` on all 23 authored cells - with weapons locked off and the campaign's ordinary win-or-nothing medal; the disc's own `HeadToHeadBar` HUD gap readout is fully decompiled but not wired.
- [The Race Campaign is authored, all of it - shape *and* content](handover/gameplay/the-race-campaign-is-authored-shape-not-content.md) - 2026-09-08, two passes in one day. The first read the disc's front-end XML and concluded the campaign's screens are authored and its content is not.
- [Zone ends now, and the wreck explodes; four reads are still open](handover/gameplay/zone-ends-now-what-is-missing-is-the.md) - the explosion landed 2026-10-01; open: `0x08ab10e8`, `0x08ab2120`, states 6/8, an opponent's destruction sound.
- [Zone flies its own environment now; the menu is the part still not mode-aware](handover/frontend/zone-flies-its-own-environment-now-the-menu.md)
- [Zone races run on all three titles now, and the craft axis closed the way the circuit one did](handover/gameplay/zone-races-run-on-all-three-titles-now.md)
- [Both titles' Zone grade escalates and draws now; two narrow render questions remain](handover/rendering/2048-and-hd-ship-an-unread-effectsettings-table.md)
- [HD's per-stage Zone textures are grounded now: which file feeds which sampler is read, what the shader does with them is not](handover/rendering/hd-zone-stage-textures-are-grounded.md)
- [2048's HUD draws its always-on six, the shield fill (with its threshold tint and lagging trail) and the held pickup slot now too, and the caption text arms are wired; the speed segments, thrust bar and zone lights are wired too; the rest of what the frames show is still state-gated and unwired](handover/frontend/2048s-hud-draws-its-always-on-four-the-rest-is-state.md) - **the first time this project drove the Vita title** (2026-09-16, `docs/reverse-engineering/vita3k-capture.md`).
- [2048's campaign is read from `SP.xml`, not a `PI_Grid` grid](handover/frontend/2048s-campaign-is-read-from-sp-xml.md) - 2026-09-21. `SP.xml` is a "mjolnir" typed-instance database, 288 instances across eight typedefs.
- [2048's front end is read and not wired](handover/frontend/2048s-front-end-is-read-and-not-wired.md) - 2026-09-17, updated 2026-09-27. **The boot walks it now**: `just play 2048` runs the five declared screens off their own redirects. **2026-09-27**: the campaign map draws the disc's own hex tile art and per-kind mode icon (chequered flag/stopwatch/radar/crosshair) instead of a flat colour square, decoded from `hex_{filled,outline,select}.gxt` and `callout/*_mode.gxt` and pixel-confirmed against the committed reference frames.
- [HD's Zone ladder draws and its speed-class sound now plays; a 40,000-tick run confirms the grade escalates rung for rung but nothing survives past zone 54; the class-change blend's wavefront (origin, radius, weight) is now fully measured live (2026-09-15) but still unwired in the renderer](handover/frontend/hds-zone-ladder-draws-and-the-zone-to.md)
- [The AI is authored XML, and its units are the only thing blocking a port of the original's numbers](handover/gameplay/the-ai-is-authored-xml-and-its-units.md) - 2026-09-16: the fire-or-absorb decision is read in full and its inputs named; the port sits unmerged on branch `plasma-flash-ai`.
- [The HUD shows a place, measured against the original; the caption pair and opponent place are the open half](handover/frontend/the-hud-shows-a-place-measured-against-the.md) - the shield-bar-colour half resolved 2026-09-05 (threshold plus a one-shot post-hit flash, not a gradient - `Hud_UpdateEnergyBar`, confidence 82); wiring it into `oag_hud` is its own thread now
- [The field drove in single file, and the fix is a per-craft personality off the disc's own AI corridor](handover/gameplay/the-field-drove-in-single-file-and-the.md) - **2026-09-07: two threads closed, one stays open.** The "nothing knows another craft exists" bullet is false and corrected with file:line evidence (`crates/ai/src/field.rs`.
- [Airborne ticks *have* been captured - 65 of them, and they are thin in a specific way](handover/tooling/airborne-ticks-have-been-captured-and-are-thin.md) - **the old title was false, corrected 2026-09-07.** `talons-junction-clean-lap.csv` holds **31** non-grounded ticks over three events (8 with both probes off.
- [Pulse handling after the airbrake scale fix](handover/gameplay/pulse-handling-after-the-airbrake-scale-fix.md) - 2026-09-30: the reference lap tracks to 4.6 units at tick 240 (was 43.9) after two fixes, missing speed pads in the replay and a 100x-weak airbrake drag term; the AI follow-up landed (a valley is not a corner: `05_Track` and `14_Track` RAPIER clean again); left open: `07_Track` FLASH `Eliminated`, the unrescued `05_Track` crest clip, a small no-airbrake residual, `drive` has no pads.
- [The crest lag is crest phase, not the cushion](handover/gameplay/the-crest-lag-is-phase-not-the-cushion.md) - **resolved 2026-09-07 by the pose walk, and the answer is (b).** The two candidates for the seven-tick liftoff lag on `16_Track`'s 1595 crest were our hover reach being too generous or residual crest...
- [Is there a fifth handling class?](handover/gameplay/is-there-a-fifth-handling-class.md)
- [The lap capture's own "the HUD said `Lap 2 of 3`" claim is now in tension with the finding that closed this row, and nobody has reconciled them](handover/frontend/the-lap-captures-own-the-hud-said-lap.md)
- [The 13 % roll-stiffness gap](handover/gameplay/the-13-roll-stiffness-gap.md)
- [Craft-to-craft collision is implemented; the stun is still not armed](handover/gameplay/craft-to-craft-collision-is-implemented-the-stun.md)
- [Craft collision feel: sticking vs spin](handover/gameplay/craft-collision-feel-sticking-vs-spin.md) - **2026-09-10: the spin half is measured and fixed.** The original applies the pair impulse at each body's own position (`body+0x30` as `Body_ApplyImpulseAtPoint`'s point, PSP and PS2 alike.
- [The barrel roll is playable; its visual is read but not drawn](handover/rendering/the-barrel-roll-is-read-and-unimplemented.md) - `oag_physics::barrel_roll` has the tap history, the shield-gated arm (8% of shield, refused at or below cost), the self-completing phase ramp.
- [The barrel roll has a drawing job, and the original's own recipe for it](handover/rendering/the-barrel-roll-has-a-drawing-job.md) - drawn and on screen since 2026-09-06: a symmetric quadratic ease on `ShipState::roll_phase`, then `eased * 6.28` radians about the craft's **nose** on its display matrix.
- [Sideshift has no runtime leg](handover/gameplay/sideshift-has-no-runtime-leg.md)
- [A parked-player Eliminator finishes at five, now a little faster than the original's 85 s](handover/gameplay/eliminator-cannot-finish-a-five-kill-event.md) - 2026-10-03: opponents fire forward weapons on the original's own law (`oag_ai::weapon_ai`), median 108 -> 75 s over 240 seeds (original 85 s). Open: a live count of the decision cadence (four a second would give 93 s), a live read of the never-written `+0x44`, a live check of the absorb-to-Shield.
- [Task #31 residual: the unguarded `slice(..)` in the race's draw path](handover/rendering/task-31-residual-the-unguarded-slice-in-the.md)
- [MONITOR has only ever run on a one-screen machine](handover/tooling/monitor-has-only-ever-run-on-a-one.md)
- [Where Pulse's language picker belongs is unevidenced](handover/frontend/where-pulses-language-picker-belongs-is-unevidenced.md)
- [Pure's `Title Screen` reveal wipe is implemented; the wordmark's own separate 0.1s fade is not](handover/frontend/pures-title-screen-is-missing-its-own-logo.md)
- [A *slot-resolved* record's own field layout](handover/tooling/a-slot-resolved-records-own-field-layout.md)
- [Pure's DLC trailer is RSA-shaped and holds the per-pack key at a fixed offset; not yet runnable end-to-end](handover/tooling/pure-dlc-trailer-key-and-testbin-unknowns.md)
- [A model built from several small pieces sharing one atlas](handover/rendering/a-model-built-from-several-small-pieces-sharing.md)
- [Which movie cut plays, and what plays the three 260-frame reels](handover/tooling/which-movie-cut-plays-and-what-plays-the.md)
- [The PS2's backend task table has four unread cases](handover/tooling/the-ps2s-backend-task-table-has-four-unread-cases.md) - what remains after the PAL/NTSC selector's trigger was traced 2026-09-08 to `<Values task="Switch50">` on a first-boot 60 Hz screen...
- [SteamOS's own glibc version](handover/tooling/steamoss-own-glibc-version.md)
- [The loading screen draws now, and its wave is partly runtime-verified](handover/frontend/the-loading-screen-has-no-caller.md) - internal-state recurrence matched to float-rounding precision across hundreds of columns (confidence 96).
- [`Anim Transform` `0x3c0` is read and played, and closing it fixed a placement defect](handover/rendering/anim-transform-0x3c0-is-read-and-played-and.md)
- [HD, 2048 and Omega render to an HDR target and tone-map it; we draw into an 8-bit one](handover/rendering/hdr-target-and-tone-map-per-title.md) - 2026-10-05, queued by the maintainer, not started: Omega's `Tonemap.*` consumer and an fp16 target; whether HD and 2048 do the same; the maintainer's "HD track textures feel off" as a curve-vs-regional measurement; and whether a craft should take on a red-dyed section's colour (an RPCS3 hull-RGB test decides).
- [HD animates a texture in two ways now; the rest of its shader families still draw still](handover/rendering/hd-animated-textures-still-families.md) - 2026-10-06: the vertex `uv + time * rate` scroll plays off authored rates (76 materials, 12 circuits, `AnimTrack::Scroll`); named families still still, no in-motion race frame yet.
- [Omega's glow layers scroll off 2048's rule; rates are chosen and several families still draw still](handover/rendering/omega-uv-scroll-rates-and-unread-families.md) - 2026-10-05: the PS4 material uniform/sampler table is read (float pool tiles exactly on all 34,423 materials), 317 glow layers and 19 plain scrolls play through the 2048 plan. Open: rates and sign chosen, `time`-only and HD-named families unread.
- [2048's scenery moves, the glow-layer scrolls play; the Zone circuit loads behind an off-by-default option (its road shader is unread, so the road vanished) and the start-grid clip is unwired](handover/rendering/2048s-scenery-moves-zone-and-the-start-grid-clips-are-next.md) - 2026-10-05: `trackZone.rcsmodel` can be loaded (`race::Options::zone_model`, off by default) for a Zone race, the material instance table is solved (colours, scroll rates), moving lit meshes under a non-uniform scale shade off the inverse transpose. Open: no capture of the original's Zone mode to judge it against, scroll rates and the plain scroll's sign are chosen, the start-grid clip is a pit-bot rig with no trigger.
- [Wipeout HD Fury's executable is 26,100 functions with 26 of them named, and no gameplay behaviour yet](handover/tooling/wipeout-hd-furys-executable-is-26100-functions-with.md)
- [Players should be able to build pilots in game, and a naive editor would eat their comments](handover/frontend/players-should-be-able-to-edit-pilots-in-game.md) - **requested by the maintainer 2026-09-06**: in-game CRUD over the pilot `.toml` files, and **all five operations have now landed**. List, edit, save and create-from-template first.
- [Pulse authors its own text entry, and it is not a keyboard](handover/frontend/pulse-authors-its-own-text-entry-and-it-is-not-a-keyboard.md) - a **positive** result found while checking whether the disc had a keyboard to play before this project drew its own. It has no keyboard and no key glyphs.
- [Five reference traces are missing here, and the silent skip is why nobody noticed](handover/tooling/five-reference-traces-are-gone-and-the-skip-hid-it.md) - five of the six trace captures the ground-truth tests name do not exist in this checkout, and their absence never turned a build red because a missing reference **skips**.
- [Omega Collection's PS4 build imports, decrypts and is mapped; asset-level access is not there yet](handover/tooling/omega-collections-ps4-build-imports-and-decrypts.md) - **A race starts on it (2026-09-30): real spline, collision (reversed circuits too), hull, textured and lightmapped circuit, and the skeleton and clip move its scenery; its circuits cull with their own `.pvs` and its Wwise banks and ATRAC9 `.wem` are read and identified (2026-09-30, `omega-pvs-sound`); the lightmap's combination is read off its pixel shaders and wired (2026-10-02, `omega-lightmap`; `Tonemap` read, consumer not located); next is a PS4 `.rcsmaterial` reader, multi-channel ATRAC9, and mapping Wwise events to cues.** `eboot.bin` imports clean via GhidraOrbis (20,941 functions; 38 named as of 2026-09-15 - `MagstripWake_Construct`, the ship-collision-fx pair, the boot chain, nine weapon-manager constructors.
- [The Omega Collection's PS4 patch adds four archives the base `.pkg` doesn't have](handover/tooling/omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md) - `data05`/`07`/`08`/`09` (no `data06`, no `data00`-`04`), extracted with the same `pkg_extract` verb the base uses.
- [Omega's front end is HD's `PI001` plugin, carried forward - and load order is unresolved](handover/frontend/omegas-front-end-is-hds-or-is-not.md) - swept 2026-09-21 from a player observation ("the Omega menu looks like HD/Fury's"); `FEGlobals` matches HD's to the digit and this project's own screen reader parses it unchanged. The `.gnf` reader this thread's own Next Steps named as the highest-value follow-on landed 2026-09-27 (`docs/formats/gnf.md`); five of the campaign's own hex texture names and most of the front end's small boot-time image set are still missing from this lane's own two-archive extraction, not unread names.

## Pending maintainer decision: shipped design data in tracked docs

[`handling-stats.md`](docs/formats/handling-stats.md)'s own rule, per
[ADR-0006](docs/architecture/adr/0006-no-copyrighted-content.md) - "a field name
is a description of the format, a tuning table is the content itself" - is
breached in places by earlier passes. `angular-velocity-column.md`'s shipped
`Turning.amount` has been redacted, but it still quotes the Assegai `Misc` hull
dimensions in its box-tensor refutation, and `force-balance-ground-truth.md`
carries XML-side values from the force-law derivations (`accelcap`, an
`amount`-derived figure, `ride_height`, `Physical.mass`), some of them
load-bearing in the arithmetic.

**Whether to sweep those historically is a maintainer decision, not an
agent's**, which is why it is flagged rather than done. Git history retains
everything regardless, so a sweep would clean the distributable snapshot, not
the past.

**A second instance was resolved the same way, 2026-09-06.** The Shuriken's and
Repulser's `<Pickupodds>` weights, recovered while establishing that both are
Eliminator-only (`docs/gameplay/pickups.md`), are the same kind of shipped
tuning data. Asked directly, the maintainer chose to commit them: eight values
for two weapons, load-bearing for the claim, rather than the full thirteen-by-four
table. The historical sweep above stays open and undecided.

**One new instance was resolved by the maintainer rather than flagged,
2026-08-17.** The `Weapon Pad` ready-state colour cycle - a 6-entry `[r,g,b]`
keyframe table recovered from `WeaponPad_UpdateRefreshTimer`
(`docs/ghidra/functions/psp-pulse-usa/pads.md`) - is exactly this kind of
tuning table. Asked directly, the maintainer chose to commit it
(`oag_render::weapon_pad::READY_KEYFRAMES`) rather than defer or build a
runtime BOOT.BIN reader, so it is tracked source now. Recorded here as the
precedent for the next one of these that comes up: ask, do not assume either
answer.

## Scope decisions that look like gaps

| Decision | Why |
| --- | --- |
| **The Memory Stick and profile system** | One automatic save slot, the way a modern PC game does it. Most of the disc's boot chain between `Language Selection` and the menus is deliberately out of scope. |
| **PSP over PS2 where they diverge** | User directive, now policy in [goals.md](docs/overview/goals.md#scope). PS2 stays the corroboration leg for confidence, not the target. |
| **The collision stun is not armed by track contact** | The original's is gated on a pending impulse the contact path never writes. |
| **Sweep and prune is not reimplemented** | The original's packing clamps world space rather than rebasing it, so it never drops a genuinely overlapping pair; ours does the same job without the packing. |
| **The four-corner hover variant** | Selected by `g_game_mode == 6` every frame (`Ship_UpdateHover`). Ported 2026-10-02: the epilogue's bank-to-yaw gain `50.0` (`ShipState::four_corner`). Not ported: the four-probe layout and the downforce's missing groundedness/magstrip factors, see `docs/ghidra/functions/psp-pulse-usa/zone-rest.md`. |
| **Pure asset work** | No longer deferred as a whole. [ADR-0022](docs/architecture/adr/0022-title-packages.md) opened the format and asset layers on the strength of [`pure-status.md`](docs/formats/pure-status.md)'s measurements; ADR-0009 item 2 still defers Pure *simulation* work behind M4's exit. |
| **PS3 content** | Still paused (since 2026-08-09), on the toolchain rather than on the research - see [`data/README.md`](data/README.md#the-ps3-image-is-encrypted-and-nothing-here-decrypts-it-yet). None of `PS3Dec`/`scetool` is installed and there is no RPCS3 install to borrow a decrypted copy from. |
| **Vita content** | Not paused: all four `eboot.elf` (base + patch v1.04, both regions) are decrypted end to end and imported into Ghidra as of 2026-08-26. The layer the prior entry missed - `eboot.bin` past PFS decryption is still NpDrm-encrypted under its SELF wrapper, a plaintext header and phdrs say nothing about the segments - is solved by `scripts/vita-self-decrypt.py`, which needs only the title's klicensee (itself pure-software recoverable from a zRIF, no F00D/hardware - `scripts/zrif-to-klicensee.py`). See [toolchain.md#vita](docs/reverse-engineering/toolchain.md#vita). Default target: `/2048/eboot-vita-2048-eu-v104.elf` (EU, patch v1.04); naming and RE work itself are the open thread now. |
| _(shared architectural fact)_ | **HD/Fury and 2048 both ship PSARC, not WAD**, and 2048 arrives as a PKG rather than a disc filesystem. |
| **A "CPU renderer" switch** | There is nothing to switch. wgpu ships no software rasteriser (`Backend::Noop` draws nothing), so CPU rendering exists only via a system Vulkan implementation such as lavapipe, selected outside the process. |
| **RENDERER applies on the next launch** | The device is made once at boot and everything hangs off it. Live switching means tearing down the surface, the pipelines and every GPU resource. |
| **60 Hz over 50 Hz where a title ships both** | User directive, 2026-09-08. The PS2 disc carries two cuts of each movie and the original picks between them on `g_refresh_mode` - the player's own answer to a first-boot 60 Hz question, not a region ([refresh-mode.md](docs/ghidra/functions/ps2-pulse-eu/refresh-mode.md)), so both are legitimate on the one PAL-only pressing. `oag_pulse::movies::LOOSE_MOVIES` lists the `640` cut first. **The switch itself is not reimplemented** and is not expected to be: this engine has no 50 Hz raster to be compatible with. |
| **PS2 `.PSS`/`.IPF` through GStreamer** | [ADR-0017](docs/architecture/adr/0017-gstreamer-native-video.md). |
| **`just play`'s default is not `oag-game`'s default** | `native-video`'s Cargo feature is off by default, so `cargo build`/`cargo test`/CI stay GStreamer-free; `just play` turns it on. |
| **GStreamer for ATRAC3+** | **Do not retry this.** Extending the `native-video` path to audio was the intended route and it cannot work - see [ADR-0019](docs/architecture/adr/0019-atrac3plus-out-of-process.md). |
| **Motion blur is built, and the jitter in its tile lookup is load-bearing** | 2026-08-31, replacing a row that said it was designed and had no code: it shipped with the velocity buffer ([ADR-0030](docs/architecture/adr/0030-velocity-buffer-motion-blur.md)), and both questions that row held open are answered - `Rg16Float` *is* multisample-renderable at 4x here, pinned as a tripwire by `rg16float_is_multisample_renderable_at_4x_on_this_adapter`, and the ADR owed was written. **The trap now points the other way.** `fs_reconstruct` offsets its tile lookup by up to half a tile, which reads as noise for its own sake and is not. Take it out and an axis-aligned lattice of squares comes back, 8 % of the viewport height across - 87 px at 1080p, anchored to the window rather than the world, so the scene flows through a stationary grid. Reported from play in a turn, measured at a 2.33x phase-0 peak and fixed to 1.48x; the numbers, the method and the two suspects ruled out instead of assumed are in [motion-blur.md](docs/rendering/motion-blur.md). The squares are that large because the reach cap doubles as the tile edge, so a generous cap makes coarse tiles - the same coupling that made the square tile-max starve the GPU. Still open there and *not* a suspect for this: `SOFT_Z` compares nonlinear 0..1 depths against a constant, which is dimensionally wrong and measured as contributing nothing to the lattice. |
| **The disc chooser is not a `menu::Menu`** | 2026-08-18. `oag_game::launcher` has rows, a cursor and the same abstract buttons, and it is its own module anyway: it runs before any archive is open, so it has no title, therefore no `MenuSkin` and no disc font. Making it a menu would mean choosing a skin in order to choose which disc the skin comes from. It draws with `font::Atlas::build` and `sprite::Sheet::default`, the disc-free pair the perf overlay already uses. The test for anything else moving out of `menu.rs` is whether a title can be behind it - and everything else can. [menus.md](docs/architecture/menus.md#the-one-screen-that-is-not-a-menu) |
| **The chooser has no headless capture route** | `--until` matches front-end state-machine names read off the disc's own `Skin.xml` (`capture.rs`), and this screen has none - it exists before a disc does. `--menu-page` is for our menu tree. So `--launcher` is *refused* with `--screenshot`/`--dry-run`/`--race` rather than half-working, and the screen is covered by unit tests plus `crates/game/tests/launcher_ground_truth.rs` instead. **`every_line_fits_on_screen` in `launcher/tests.rs` is the one that earns its keep**: it measures each drawn string in the 5x7 face against the 480-unit grid, and it exists because the first version's footer ran off the right edge where no test could see it. |
| **`chdman` is not installed in this sandbox; PPSSPP does not need it anyway** | 2026-08-19. `just extract-iso` (and `psp-drive.py preflight`'s own suggested recipe) calls `chdman extractdvd`, and in this sandbox that fails immediately with `command not found` - no `chdman` binary anywhere on the machine, confirmed by a full `find /`. Cost a detour into "live verification is blocked, commit the doc-only half and leave the port uncommitted" before the actual fix was noticed: `ppsspp-debugger.md` already says **PPSSPP v1.20.4 boots a `.chd` directly**, extraction is optional on that version, and the installed `PPSSPPSDL`/`PPSSPPHeadless`/`Xvfb` are all present and work. Pointing the SDL launch straight at `data/images/pulse-psp-usa.chd` (skip the `chdman` line in the docs' own recipe entirely) reached a live, driveable Single Race normally. Check `which PPSSPPSDL PPSSPPHeadless Xvfb` before assuming a live PSP session is unreachable here - the CHD-vs-ISO step is what's actually missing, not the emulator or the display. |
| **`body+0x50` is live-verified as a craft's world position; `rigid-body.md`'s own `body+0x40..0x70` inertia-tensor table says that range is something else - and a follow-up check sharpened the conflict rather than closing it** | 2026-08-19, found while live-verifying `Weapon_PostBlastImpulse_q`'s port - see the stun row above and [contact-response.md](docs/ghidra/functions/psp-pulse-usa/contact-response.md#weapon_postblastimpulse-0x0886794c-confidence-82). A breakpoint read `body+0x50` off a real craft's confirmed `RigidBody` pointer mid-tick and got a live, track-scale, per-craft value that tracked the craft's actual position - not tensor-shaped data. `rigid-body.md` independently reads `body+0x40..0x70` as one confidence-88 inverse-inertia-tensor block, backed by `Body_Integrate` visibly consuming it every sub-step via `vtfm3.t`/`vscl.t`. **Checked further, not just flagged**: `Body_SetBoxInertia`'s diagonal patch-back writes land at `+0x40`/`+0x54`/`+0x68`, stride `0x14` - exactly what a row-major 4x4 with `0x10`-byte rows at `+0x40`/`+0x50`/`+0x60`/`+0x70` predicts for its own diagonal, which *confirms* the constructor genuinely treats `+0x50` as row 1's base rather than resolving the conflict in the other direction. That makes the runtime finding sharper: an axis-aligned box's row-1 off-diagonal terms should read `0.0` forever after construction, and they visibly do not. Both readings rest on real instruction traces; neither has been withdrawn, and they still cannot both be exactly right about `+0x50` specifically. Flagged with a cross-reference in both docs, reconciled in neither - next step is tracing exactly which quad-words `Body_Integrate`'s `vtfm3.t` addresses (a `.t` transform is 3x3; which three rows from a `+0x40` base is unread) and finding what writes `body+0x50` every tick. A second, smaller thread from the same session, explicitly **not** a finding: `body+0x30` was read at the start line pre-countdown (not a controlled comparison against the mid-race `+0x50` sample) and clustered near `(0, 0.7-0.75, 0)` for all eight craft - as consistent with a parked grid in some local frame as with anything else; `rigid-body.md` already has `Body_Translate` accumulating into `+0x30` the way a real position field would. Needs a same-breakpoint mid-race re-read before it means anything. |
| **Our AI barrel-rolls and the original's never did** | 2026-09-06, **landed**, and the largest deliberate deviation in the tree - a maintainer directive rather than a fidelity gap. The original reads the roll's tap history out of the human player's pad block and nothing else (confidence 85: the `ship+0x78` early-out at `0x08846be0`, a *single-global* axis edge detector at `0x08ae4cf0`, and a complete arm-site scan), so its opponents never roll; ours do, an `Ace` whenever its energy budget allows and lower tiers less. **Four gates, and the invented two sit on top of the recovered two rather than through them**: airborne (88) and `cost < shield` (90) are untouched; the energy budget is `ShipControls::roll_shield_floor`, a per-pilot number a player retunes in a TOML; the propensity is `Difficulty::roll_appetite`/`roll_caution`. Three pilot axes carry it - `roll_chance` (a probability *per airborne window*, not a rate), `roll_floor`, `roll_airtime` - appended as draws 17-19 and pinned by `every_built_in_pilot_still_draws_what_it_drew_before_the_roll_axes_were_appended`, which is deliberately wider than the guard beside it because draws 8-16 had nothing watching them. **Two things not to quietly reverse**: provocation does *not* feed roll usage (the grudge scales what a driver does to other craft, never what it asks of its own, and wiring it closes the rubber-banding loop this project refuses to port), and the driver asks for a roll on a dedicated `ShipControls::roll_request` rather than synthesising taps on its own steering axis, so the invented intent stays distinguishable from the recovered input path. **What the measurement changed**: the invented `0.20` shield floor `oag_physics` carried as a bare constant was dormant while the AI did not roll, and the first race that exercised it put an aggressive Ace on `09_Track` on **6.5** shield - the hard floor held, what it had done was remove the buffer the walls then ate - so no built-in uses `0.20` any more. Full table and method on [ai.md](docs/gameplay/ai.md), harness in `crates/game/tests/ai_roll_ground_truth.rs`. |
| **Screen filter tunables have no menu rows, and the PSP presets are uncalibrated** | 2026-09-16, with the feature itself landed - [screen-filters.md](docs/rendering/screen-filters.md), [ADR-0053](docs/architecture/adr/0053-screen-filters-are-loadable-wgsl-after-the-composite.md). A preset's header declares `min`/`max`/`step` per tunable and the loader validates them, but the menu is a static `menu.toml` and regenerating rows off a file's header is a change to that model nobody has needed yet; tuning is copying the built-in into `<config dir>/oag/shaders/` and editing the defaults, which shadows it under the same id. Separately: `psp-3000`'s `0.035`/`0.075`/`0.12` and `psp-lcd`'s `0.4`/`0.04` are starting points from a design discussion and PPSSPP's own defaults, not measurements of a panel - the page says so per number. **Do not add a `_q`-style confidence to them**: nothing on a disc authors a display, so there is no RE claim to score. |
| Everything else milestone-scoped | Weapons, AI, netcode, the shell: [roadmap](docs/overview/roadmap.md), not this file. |

## Working rules that were learned expensively

**Most of them now live in `docs/`**, because they are properties of the method
rather than of any pass:

- [Reverse engineering](docs/reverse-engineering/methodology.md#rules-learned-the-expensive-way) -
  decode before reporting, xref counts bound authored instances only, sibling
  pages go stale, never write a fitted constant in as a recovered one, and the
  rest.
- [Reading a capture](docs/reverse-engineering/verification-protocol.md#reading-a-capture-rules-that-each-cost-a-session) -
  control columns first, unclamped ramps, clean-capture detection, ask the
  tangent not the index, Left-Up-Forward, and threshold-versus-frame stability.
- [GhidraMCP bridge quirks](docs/reverse-engineering/toolchain.md#search_instructionss-mnemonic-filter-is-exact-match-not-substring) -
  `search_instructions`'s `mnemonic` filter is exact-match despite its own
  description, so a sweep filtered on one mnemonic silently misses every
  underscore-prefixed delay-slot form of it; `run_script_inline`'s current
  failure mode.
- [Measuring a renderer change](docs/rendering/README.md#measuring-a-renderer-change) -
  aggregates rank broken changes higher, brightness thresholds measure the
  circuit, measure a screenshot before citing it.

What stays here is what is about working in *this* repository, with other
writers in it at the same time:

- **Editing this file or a `handover/` thread**: verify each cut against the
  page or evidence it cites before cutting, not after - a fitted number
  drifting from its source is the usual way this goes wrong. Compress or
  delete a thread that is closed or stale, never one still carrying open
  work. Never merge numbers into a new total that does not already appear
  merged on the page they came from.
- **`git stash push -- <pathspec>` has the same failure, one level earlier:
  it captures a listed file's *entire* working-tree diff, not the hunks one
  writer actually made.** 2026-08-25. A session isolating its own edits into a
  worktree via `git stash push -u -- <its files>` named `menu.rs` among them;
  another session was concurrently mid-refactor in that same file, uncommitted.
  The stash swept both diffs off disk in one move, and popping it in the
  worktree, then discarding the mixed result down to just the first session's
  own hunk, left the second session's work recoverable only from the stash
  commit's blob (`git show <sha>:<path>`) - not restored on disk anywhere, and
  the main checkout was left with that file's *other* uncommitted edits
  (test/data files referencing the now-stashed API) pointing at a version of
  it that no longer matched. **Before stashing a file by pathspec while
  another session might be touching it, diff it first** (`git diff -- <path>`)
  and check whether the whole diff is actually yours; if not, this needs a
  worktree isolating *that session's* edits instead, or a message to them
  before touching the file at all - not a stash-and-hope.
- **Do not run `just` or `cargo fmt --all` while anything else is editing.** A
  gate run taken mid-edit reports failures that do not exist, and `cargo fmt
  --all` rewrites files another writer holds open. Scope to `-p <crate>`.
- **A doc two parties are editing wants committing promptly**, not left dirty:
  concurrent uncommitted edits are indistinguishable from data loss to whoever
  looks second.
- **Splitting a large file (`crates/game/src/main.rs`, 4,808 to 430 lines
  across 18 modules, 2026-08-16) is safest computed as one map from the
  original applied in a single pass, not carved interactively** - each
  edit shifts every line number below it, so line-number-anchored edits
  on later slices corrupt them. Also: `impl` blocks that were
  private-to-file become `pub(crate)` once the pieces are siblings rather
  than parent/child, not `pub`.
- **If two sessions do run at once, the value is in their *disagreement*, not
  their throughput - and one thing destroys it.** On 2026-08-09 two sessions
  worked this tree simultaneously; every number in that day's four commits was
  reproduced by whoever did **not** produce it, and each caught refuted
  citations the other's sweep had missed, in both directions. What made that
  work is that neither could see the other's intermediate state, so **a wrong
  premise had to survive contact with someone who had not formed it** - which
  is precisely what six days of single-session work could not provide. Two
  sessions that agree on method up front and split strictly by file reproduce
  each other's blind spots at twice the cost. So: split by *claim* and re-derive
  each other's numbers, do not split by directory and trust the reports. And
  note the same day's counter-example - a spawned subagent went idle four times
  without ever producing a report, and its one decisive check had to be run by
  hand. **Parallelism bought nothing that day; independent re-derivation bought
  everything.** Do not read that session as an argument for more agents.

## Traps that are live

**On `ps4-omega-eu`, the tagged-object constructor idiom (vtable install,
`.cpp`-tag write to `param_1[0xb]`, callback to `param_1[1]`, plus a
linked-context-list registration walk) is shared boilerplate every class
uses, not evidence a candidate function matches a specific sibling-binary
class.** 2026-09-15. Investigating a `Collision.cpp`-tagged candidate for
`ps3-hdfury-eu/collision.md`'s own `Collision_Construct`
([`ps4-omega-eu/collision.md`](docs/ghidra/functions/ps4-omega-eu/collision.md)),
the same busy-wait chain walk, field writes (`0x3006`/`0xffffffff`) and
vtable/tag/callback triple that looked like collision-specific structural
corroboration turned out to be byte-identical to code already read the same
day in all thirteen of `ModeManager_ConstructByMode`'s branches
([`weapons.md`](docs/ghidra/functions/ps4-omega-eu/weapons.md)) - caught by
`advisor()` before the name was committed. **On this binary, a `.cpp` tag
match still needs the class-specific evidence `README.md` already asks for
(a field offset, an enum, a magic number, or - the strongest, per
`ps3-hdfury-eu/collision.md`'s own `Collision_Construct` - a loop whose
count times stride lands exactly on a separately-computed allocation size)**;
the generic constructor skeleton around the tag write proves nothing on its
own here, and won't going forward either.

**Every static negative recorded on `ps3-hdfury-eu`'s `EBOOT.elf` before 2026-09-15 09:29 is stale by construction.** Stock Ghidra could not decode the Cell `lvlx` instruction, so 851 sites across 294 functions - about 213 KB of code - held with no instructions, no xrefs and no callers; the live `ps3-hdfury-eu` program was reimported at 09:29 today with a fixed language and now carries all of it (`docs/reverse-engineering/toolchain.md#ps3`, "Some Cell vector instructions are missing"). Any "no writer", "no caller", "no consumer", "no xref", "nothing references", "no reader located" or "the sweep returned nothing" claim about that binary timestamped - or undated - before that reimport was computed over an incomplete image and needs re-running before it's trusted; three were re-run the same day: the Zone radius writer was found, `zoneOrigin`'s writer was re-confirmed against a thread that still said "none", and `craft+0x108` held as a negative. A pass on 2026-09-15 marked the still-load-bearing ones it found across `handover/` and `docs/ghidra/functions/ps3-hdfury-eu/` with `**[stale 2026-09-15: computed before the lvlx reimport; re-run per toolchain.md#ps3]**` inline rather than re-running them all; nobody has audited the rest.

**`cargo check` does not link an executable, so a `--screenshot` capture run
between a `cargo check` and the last real `cargo build` is silently running
the old binary.** 2026-09-13, wiring HD's chrome-title font role. The whole
implementation pass used `cargo check`/`cargo clippy` to verify as it went -
correctly, they are faster and catch the same type errors - but the first two
before/after screenshot pairs both came back **zero** pixels different,
which read as "the fix isn't visible" rather than what it was: `target/debug/oag-game`
had not been relinked since the one `cargo build --workspace` at the very
start of the session, so every `--menu-page` capture in between was against
pre-change code with no error or warning saying so. Rebuilding with `cargo
build` (not `cargo check`) surfaced the real picture - and a second, genuine
bug the check-only loop had never run far enough to hit: a `wgpu` validation
panic, `"the buffer bound at binding index 0 is bound with size 56 where the
shader expects 64"`, from a `Uniforms` struct whose Rust-side size did not
match WGSL's own alignment-driven padding. **A `cargo check`-verified change
to `render.rs` or any `.wgsl` file needs an actual `cargo build` plus a real
capture before trusting a before/after comparison** - `cargo check`'s silence
proves the types line up, not that the picture on screen has moved, and a
wgpu validation error only fires at draw time, never at compile time.

**Adding a weapon to `oag_weapons::pickup::IMPLEMENTED` re-rolls every
statistic downstream of `pickup::draw`, and a sparse one will flip.**
2026-09-08. Landing the LeachBeam turned
`oag-game::ai_roll_ground_truth a_full_grid_still_rolls_and_a_higher_tier_rolls_no_less`
red - `skilled armed 0 rolls total against novice's 1` - (that test is six
`a_full_grid_of_*_arms_no_fewer_rolls_than_*` tests since 2026-09-09) with nothing touching
the barrel roll, `oag_ai` or any force law. Confirmed by isolation: that one
line in `IMPLEMENTED` makes it fail, the same line out makes it pass. A longer
`IMPLEMENTED` widens the weighted walk in `draw`, so every craft draws different
pickups, so every trajectory differs, so the seven per-flight roll coin tosses
are re-rolled. The test's per-tier totals lived in the range 0-3, and a
statistic with four distinct values cannot carry a monotonicity claim across
four tiers.

**So a red in a race-wide *behaviour* statistic after a pickup change is a
sample-size finding first, not a regression.** Check whether the assertion's
statistic can absorb a reroll before looking for the bug. The fix the maintainer
ruled for is to make the sample discriminate - that test now sums twelve forward
circuits by two world seeds by seven opponents (24 grid races per tier, armed
totals 5/21/43/82, each seed monotone on its own) rather than one race on
`09_Track`. **Weakening the bound and re-baselining the number were both
explicitly rejected**, the same call made on the `craft_sticking_ground_truth`
tripwire the same day. `race::Options::seed` is the resampling axis: it reaches
`World::new`, `oag_ai::Driver::for_slot` and `oag_ai::pilot_for_slot`. The cost
is real - 350 s in debug against 17 s, which makes it the suite's longest single
test.

**It bit a second test the same day, and there the rate had not moved at all.**
`oag-game::ram_ground_truth a_ram_rarely_throws_the_rammer_out_of_the_corridor`
(merged into `a_ram_fires_only_where_there_is_room_and_rarely_throws_the_rammer_out`
on 2026-09-09) was **already red on `main`** before this pass - isolated by reverting only the
Quake lifetime fix, which changed the count from 9 to 11 and left it red either
way, so it is not a consequence of that fix. Its bound is "no worse than one in
six", set from 2 of 27 shifts over six races in August. The ram was later
narrowed to the player slot and every pickup-pool change since has reshuffled
the field, so six races now yield **nine** shifts and `2 of 9` trips a 1-in-6
bound on noise alone. Re-measured over forty races on the same tree: **11 of
123, 8.9%**, against the 7.4% the bound came from. The bound was left exactly
where it was and `RACES` went 6 -> 40 (about 110 s in debug).

**The lesson both cases share**: when a ground-truth bound goes red, check the
*denominator* before the numerator. A shrinking sample and a real regression
look identical from the assertion message, and this suite has several bounds
whose sample was sized against a field that has since changed.

**Both PS2 transcode ground-truth tests shared one scratch directory, and it
read for three days as "fails under load".** Fixed 2026-09-09 in `33913eb9`;
`transcode_scratch`'s own doc comment in
`crates/game/tests/ps2_source_ground_truth.rs` carries the mechanism. Kept here
only for the general lesson: **a deterministic temp path shared by two tests
races under `nextest`'s own parallelism, with no second worktree and no load
required** - and the load correlation that follows from it is a symptom
convincing enough to survive three separate investigations.

**A `Data\Psys\<NAME>.POB` load-path string search discriminates for weapon
and debug particle-effect names on HD's executable, and is blind to
continuous/ambient ones - a zero hit means opposite things for the two
categories.** 2026-09-08, bucketing `hds-engine-trail-is-one-of-four-ribbons.md`'s
unwired HD `.pob` inventory. For `NUMBERS`/`TEST_BOMBSPIKES`, absence from
`EBOOT.elf`'s string table was real evidence: a control scan
(`strings -a … | rg -o 'Data.Psys.[A-Za-z0-9_]+\.POB'`) finds 46 of HD's 82
disc names as a load-path string, including several this engine has not
built at all (`WO_LEACHBEAM_CHARGING`, `WO_BOMB_EXPLO_DETONATOR`), so the
executable's string table tracks the *original* game's load paths and a
genuine absence says something. Reused unchecked for four environment names
(`WO_BLUE_WELDER`, `WO_MODESTO_STEAM_A`, `WO_DustMotes`,
`WO_UNDERWATER_GODRAYS`) in the same pass, and the control silently fails
for that category: **zero** environment-flavoured name is in the 46, wired
or not (`WO_RAIN`/`WO_SNOW` included), and the *wired* `WO_SHIP_ENGINEFLARE`
is absent from the string table too, same as its two unwired HD siblings
`WO_ENGINE_FLARE`/`WO_ENGINE_JETFLARE`. A continuous or ambient effect's
trigger evidently never puts its name in a load-path string on this disc,
so a zero hit there proves nothing either way - caught only because advisor
review asked for the same control re-run scoped to the new category, one
commit after the overclaimed version had already landed
(`f496a5b2` -> corrected in `48ef5c17`). **Re-establish a control for each
new *category* of name before trusting a zero-hit search on it, not just
once per search technique** - see `docs/formats/pob.md`'s HD corpus
section and that thread's Next Steps for the worked example and the
now-open `WO_ENGINE_FLARE`/`WO_ENGINE_JETFLARE` question it left behind.

**A wired-effect count kept in prose drifts stale independently in every
place it's copied, with nothing to catch it converging on the wrong
number.** 2026-09-08, same pass. `RACE_EFFECTS` (`crates/raceplay/src/effects.rs`)
grew from 8 to 15 over two weeks (Plasma, Shuriken, Quake landed
2026-09-02) while three different prose copies of "how many are wired"
stood still at three different stale values - 7 in a handover thread, 11 in
`psys_inventory_ground_truth.rs`'s own doc comment - neither matching each
other or the real number, until cross-checked against the source directly.
**Recompute a wired/authored count from `RACE_EFFECTS.len()` (or the
equivalent const) at the point of use rather than trusting a number
written down earlier**, especially across a gap of more than a day or two;
nothing enforces that these prose counts stay in sync with the source.

**A tuning number chosen on the twelve lone-craft circuits can turn a *field*
ground-truth test red, and the whole `just` gate will stay green while it
does.** 2026-09-06, choosing `oag_ai::Tuning::curvature_span`. A cap of 10
units was the best row on `race_ground_truth`'s twelve-circuit solo board -
all twelve clean, no respawn added, `07_Track` reaching lap 4 for the first
time - and it broke both
`lap_times_ground_truth::every_opponent_that_laps_has_a_lap_time` and
`opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`.
One craft in a grid of eight wedges near the start and never completes a timed
lap; another is ground to 0.41 of its pool against a 0.45 floor. **A lone craft
on an empty circuit is a different machine from a craft being shoved by seven
others**, and no amount of solo circuits substitutes for one field run. The
sweep harness (`crates/game/tests/ai_span_sweep.rs`) now reports both boards,
using the two committed field fixtures set up exactly as they set themselves
up. The second half of the trap is why it survived review at all: **`just` does
not run `#[ignore]`d disc-backed tests**, so `just` was green throughout. Run
`just test-data` after any AI tuning change. Historical note, not current
guidance: `shuriken_ground_truth` and `stall_rescue_ground_truth` were once
the two expected reds this line named - both are fixed (2026-09-07 and
2026-09-08 respectively, see the shuriken thread below) and `just test-data`
should be all-green again barring a fresh regression.

**A start-line or grid-timing change can flip an `#[ignore]`d ground-truth
test without CI ever seeing it, because `just test` never runs `#[ignore]`d
tests at all.** 2026-09-06, bisecting `shuriken_ground_truth`'s red test (see
the shuriken thread above). `cc395862` ("hold AI opponents at the line
through the start countdown") updated the three synthetic `oag-game` tests
its own gate broke and added a dedicated countdown test - every one of those
runs under `just test` and would have caught a regression there. But it also
changed where every opponent is, tick for tick, for the rest of any race
longer than the countdown, and the disc-backed `shuriken_ground_truth` and
`stall_rescue_ground_truth` tests that measure exact opponent position and
state both broke the same day - invisibly, because both are `#[ignore]`d and
only `just test-data` (which needs `data/images/`) runs them. A change that
only touches synthetic, in-memory fixtures cannot see this class of
regression at all; only a real-track, real-grid ground-truth test can, and
those only run on demand.

**A breakpoint-driven PPSSPP capture (break, screenshot, resume) burns real
wall-clock time far faster than emulated game time, and the attract-demo
idle timer runs on the former - so a capture can silently roll into the demo
mid-run with nothing in the frame indices announcing it.** 2026-09-05,
measuring the main menu's highlight pulse. Breaking on `Gfx_PresentFrame` and
photographing between hits costs about a quarter second of wall time per
frame (the screenshot round trip), so 90 in-game frames (3 s of emulated time)
took roughly 90 real seconds - and combined with the setup time before the
capture script even started, real elapsed time since the menu was last
touched exceeded the documented ~120 s attract-demo threshold well before the
frame count did. The capture kept running and photographed the demo's own
race footage, indistinguishable from a real capture until a frame was opened
and showed a circuit rather than `Main Menu`. **Combine "walk to the state
you want" and "start capturing" in one process with no gap** - reading tool
output or writing an analysis script in between is exactly the gap that
overruns the idle window - and treat total wall-clock time since the last
input, not frame count, as the budget against the 120 s limit.

**`ghidra-mcp`'s `search_byte_patterns` accepts a `mask` parameter and
silently ignores it - a masked wildcard search reads exactly like "not
found" even when the bytes are sitting right there.** 2026-09-05, hunting
`stfs ...,0x108(rX)` for any register `X` on `ps3-hdfury-eu`. A function
disassembly-confirmed to contain three `stfs ...,0x108(r31)` instructions
(`d03f0108`) at known addresses returned **zero** matches for
`pattern=d0000108, mask=fc00ffff` (byte0 masked down to just the `stfs`
opcode bits, byte1 wildcarded to catch any register pair, byte2/3 pinned to
the `0x108` displacement) - and the identical mask against the pattern with
the *real* byte1 value already filled in (`d03f0108`) found all three, i.e.
masking degrades to an exact-match requirement rather than a wildcard.
**Use `search_instructions` with `mnemonic`/`operand_pattern` instead** - it
matches after Ghidra's own disassembly, so `mnemonic="stfs",
operand_pattern="0x108("` finds every register variant in one call (48 for
this offset) with no encoding math and no mask to get wrong. Reserve
`search_byte_patterns` for a pattern with no live-register component, and
sanity-check any use of its `mask` against a byte sequence you already know
is present before trusting a "no matches" result from it.

**`ghidra-mcp`'s `rename_data` rejects this project's own global-naming
convention outright, and `rename_function` merely warns about it - so a
data name silently doesn't land while a function name does.** 2026-09-05,
recovering `g_MaterialDrawClass` on `ps3-hdfury-eu`. The tool enforces a
Hungarian-prefix rule (`g_dw...`, `g_ab...`, `g_p...`) and returns
`{"status":"rejected","issue":"missing_hungarian_prefix"}` for a plain
`g_MaterialDrawClass` - even though `g_ChunkVisibilityMask` and every other
`data` row in `names.tsv` is spelled that way, per
[ADR-0005](docs/architecture/adr/0005-ghidra-conventions.md). **Do not
rename to satisfy the validator**; ADR-0005 and `names.tsv` are
authoritative and `just check-names` polices them. Use `create_label`
instead, which applies the same name and only warns. The same validator
also warns that `Subsystem_VerbNoun` "is not PascalCase" and "does not
start with a recognized verb" on every function rename this project makes -
those warnings are noise, and the rename does land.

**A boot chain watched on RPCS3 with savedata present is a *different, shorter* chain, and it reads as a complete one.** 2026-09-05, and it nearly cost a wrong `Provenance::Measured`. With `~/.config/rpcs3/dev_hdd0/home/00000001/savedata/BCES00664-AUTO-` in place, Wipeout HD skips `FirstPlay` entirely and goes `EpilepsyWarning` -> `Save Warning`: seven steps where its `skin.xml` declares eight, with nothing in `TTY.log` announcing an omission - only `Data BCES00664-AUTO- has been found` and `get->isNewData: 0`, several lines away and easy to read as routine. Flipping a chain to `Measured` off such a log puts a step nobody watched inside a chain labelled watched, which is [ADR-0025](docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md)'s own error mode one level down. **Move the save aside before any boot-order capture and put it back after** - `just rpcs3-bootchain`'s recipe comment says so, and `docs/formats/hd-frontend.md` has the measurement. The general form is worth carrying to the other titles: a front end's boot order is a function of profile state, so "which boot did you watch" is part of the claim and not context.


**A `--screenshot --until` run over a movie leg on a machine with a real audio
device fails with `never reached "X" in 3600 ticks`, or worse, silently
captures an early frame instead of the one asked for - cost two wrong
readings (a near-white reel frame and a black intro frame) before this was
understood.** 2026-09-05, closing
`handover/until-cannot-reach-a-late-movie-frame-on.md`. A movie with sound is
audio-clocked per ADR-0019: `Audio::movie_playhead` returns
`Some(seconds)` from the real device's own position whenever one is
attached and streaming. A headless capture has no per-tick pacing, so it
finishes in well under a second of real time while the movie itself can be
tens of seconds long - the device-driven playhead barely moves and `--until`
burns its whole tick ceiling on a picture that is still on its first
frame. This is invisible on CI or any machine with **no** device: there,
`movie_playhead` already falls back to `None` (tick-clocked), which is why
the bug never showed up there. **Fix: pass `--no-audio`.** It forces the
same null audio backend `--dump-audio` already forces (so `movie_playhead`
takes the same tick-clocked fallback a no-device machine gets for free),
without needing `--dump-audio`'s `--screenshot` pairing or its growing WAV
buffer. Verified against both Pulse's `LogoFMV` reel and Pure's
`Developer Publisher Screen` reel (which has two 2-second frame holds -
the one place tick-clock and audio-clock provably diverge, and it still
lands on the correct next state). See `crates/sound/src/lib.rs`'s
`Audio::open`/`Audio::movie_playhead` and `crates/game/src/main/cli.rs`'s
`--no-audio`.

**The `ghidra-mcp` bridge is one shared instance across every concurrent session, and `switch_program` sets a global "current program" pointer any of them can move out from under you.** 2026-09-03, reading `ps3-hdfury-eu`'s `TrackSelection_Screen` while a second, independent thread (`2048-and-hd-ship-an-unread-effectsettings-table.md`) was doing overlapping Ghidra work the same week. Three `rename_function_by_address` calls with no explicit `program` argument reported success; a follow-up `get_current_program_info` showed the active program had silently become a different title's binary mid-session, and the three renames had landed there instead - `get_function_by_address` back on `ps3-hdfury-eu` still showed plain `.opd.FUN_xxxxxxxx`. Fixed by redoing all three with an explicit `program: "/hdfury/EBOOT-ps3-hdfury-eu.elf"` and re-verifying the same way. **Pass `program` explicitly on every mutating call** (`rename_*`, `set_*`, `create_*`, not just `switch_program`), and re-verify with an explicit-`program` read immediately after any rename that matters - a `switch_program` done once at the start of a session is not durable if another agent is driving the same instance. Full write-up: `handover-scratch/hd-zone-tracklist-re-notes.md` from that session (untracked, may not survive - this paragraph is the durable copy). **The write path that does not depend on the pointer at all is `run_script_inline` with an explicit `program`**: confirmed 2026-09-15 by applying all 309 `ps3-hdfury-eu` names through one inline script while another session held the pointer on a `BOOT.BIN` and `just apply-names` was (correctly) refusing to run - see the closing paragraphs of toolchain.md#ps3.

**A live Ghidra instance's own function boundaries can drift from what a `docs/` page records, even without a fresh import.** 2026-09-05, re-reading `Mine_Init`/`Mine_Construct` on `psp-pulse-usa` before wiring their models. `mine.md` records both at specific entry addresses (`0x08859ac8`, `0x08859930`, confidence 90/92); this session's live instance decompiles those exact addresses to two *different* functions - one an unrelated teardown routine, one landing mid-instruction inside its neighbour, confirmed with `disassemble_bytes` rather than trusted from the decompiler alone. The actual matrix-copy/velocity/fuse/drift logic `mine.md` describes is reachable, just at a different entry (`0x08859954`) in this instance. Most likely cause: `just apply-names` had not been replayed into this project (CLAUDE.md already notes a fresh import starts at `FUN_08940d0c` again; this suggests boundary drift can happen independently of a full reimport too). **Byte-for-byte content at a documented address can still confirm a reading even when the address itself no longer resolves to that function's entry** - don't treat a decompile-at-address mismatch as proof the earlier finding was wrong; disassemble around the documented address first to see whether it is genuinely a different function or just mid-body of a shifted one. See `mine.md`'s 2026-09-05 section for the full worked example.

**Ghidra's decompile of a PS3 HD/Fury function above the `0x32d5e0` TOC break
names the wrong strings, disc-wide, not just for one function.** Read closing
the engine-shader-parameters thread, 2026-08-24 originally. `0x003f1300`
(`Shader_InitEngineParams`) decompiles showing sound-bank and `.vex` path
literals that are not what the function actually references - it sits above
the TOC break [memory.md](docs/ghidra/functions/ps3-hdfury-eu/memory.md)
documents, so Ghidra resolves its `lwz rX,disp(r2)` loads against the wrong
table entries. The only correct read is
[`scripts/ps3-toc.py`](scripts/ps3-toc.py) walking the function's own loads
against its own OPD TOC. Check a high-address PS3 function's decompile
against this before trusting any string or symbol name it shows - a
plausible-looking wrong string here reads exactly like a correct one until
checked.

**A stationary `--race` capture never reaches the particle path at all, so it is worthless as evidence about anything a sharpener, a filter or a compression step does to effects.** 2026-09-02, closing the FSR 1 default question. `--race` holds the throttle without steering, so the player crosses no `Weapon Pad`, fires nothing and hits nothing. Measured from the vertex side: the `psys` path costs **199 bytes** in a stationary `--race` capture and **1.99 MB** under `--autopilot --give rocket --press square` - four orders of magnitude, because the stationary run never enters it. The frame *looks* like a race and contains none of the high-frequency additive content - spark burst, rocket plume, shield shell - that the question was about. A preference for FSR 1 was carried for weeks on exactly such a capture. **Add `--give rocket --hold cross --press square` to any capture meant to judge effects**, and prefer a play-test over a still for a picture-quality judgement: a byte-diff answers "are these different", not "is this better".

**Do not read a menu or front-end capture taken with `--upscaler` as evidence about the upscaler.** The flag changes the UPSCALER row's own *text*, so two images differ for a reason that has nothing to do with resampling - that nearly produced a false conclusion once. Since [ADR-0038](docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md) the trap is worse than it was: no upscaler touches menu or front-end pixels at all any more, so the row's text is the *only* thing that can differ between the two captures.

**ADR-0036 states one consequence that is wrong, and ADR-0038 supersedes a second item in it - both ADRs are immutable, so this is the correction of record.** 2026-09-02, landing the UI-compositing restructure. (1) ADR-0036's consequence *"every race `--screenshot` changes bytes when the HUD moves"* is **false**: only `--presented` does. The plain capture path in `crates/game/src/race_capture.rs` builds no `Framebuffer` and runs no resolve, so its HUD was already drawn at native size and its bytes are untouched - verified by the capture path having no `Framebuffer` branch outside `presented`. Anyone re-baselining race screenshots on the strength of that sentence would be regenerating goldens for a change that did not reach them. (2) ADR-0036 puts a movie on the *scene* side of the UI seam, which for the front end would have meant splitting one draw list across two passes; [ADR-0038](docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md) supersedes that item, because the front end's scene side holds **nothing but the movie quad** and the launcher and loading screen have no movie at all. A stage with no 3D scene draws its whole list straight into the presentation target and `resolve_scene` is not called. Read ADR-0038 before ADR-0036's seam wording sends you to build the split.
**`RemoteDebuggerOnStartup` does not pause the CPU at boot, so a watchpoint armed "right after connecting" can already be too late for an early write.** 2026-09-02, chasing the SAP clamp globals (`docs/ghidra/functions/psp-pulse-usa/collision.md`'s clamp section). The config option opens the websocket port at boot; it does not halt execution waiting for a client. The first successful connection after a fresh `PPSSPPSDL` launch read `cpu.status` as `{"stepping": false, "ticks": 22061009}` - the CPU was already running, and 22,061,009 ticks at the PSP's 222 MHz clock is about 99 ms of *emulated* time, easily enough for an early boot-time initializer (a `.ctors` entry, a bootstrap routine) to have already run and written a value once. A write watchpoint armed at that point, then left running through an entire menu walk and track load into a live race, never fired - not because nothing writes the target, but because whatever writes it had already finished before the watchpoint existed. If a "no writer found" read from a watchpoint disagrees with a live memory read that is plainly non-default, suspect this before concluding the write is invisible to the debugger; there is no known way to make PPSSPP pause at the very first instruction over this API, so the fix is a static instruction sweep for the write (accounting for an immediate split the naive `lui`/`addiu` operand form may not match), not a faster connection race.

**Two debugger connections racing the same PPSSPP execution breakpoint corrupts the read, not just the timing - and a watchpoint armed on the garbage address that produces counts nothing while looking like a measurement.** 2026-08-31, chasing [the M6 lighting thread](handover/rendering/m6-authored-lighting-no-hardware-light-slot-found.md)'s live capture. `psp-drive.py restart`'s own internal `Ship_UpdateCraft` breakpoint and a second, independent connection's own breakpoint at the same address raced each other (PPSSPP execution breakpoints are global CPU state, shared across every client), producing a garbage field read (`craft+0x1CC` came back `0`) that got armed as a watchpoint on address `0x30`. Fix: take an address a running script already printed instead of re-deriving it with a second breakpoint, and bounds-check any computed watch address against `0x08000000`-`0x0A000000` before arming it. [`ppsspp-debugger.md`](docs/reverse-engineering/ppsspp-debugger.md#two-connections-racing-the-same-execution-breakpoint-corrupts-the-read-not-just-the-timing).

**Corrected 2026-09-05: that bad watchpoint did *not* cause the `00000030` emulator halt blamed on it, and the real cause is a one-line rule.** The halt (`E[MEMMAP] Bad memory access detected! 00000030 ... Stopping emulation`, JIT block `08872f98_z_un_08872f54`) is the emulated game loading a quad from `null + 0x30`, and it is triggered by **writing the LeachBeam fire bit `0x8000`** through `scripts/psp-fire-weapon.py`'s raw-bit mechanism - the instance lands in a weapon pool with its matrix pointer `+0xa0` never filled in. **Do not fire the LeachBeam that way**; it halts the emulator within a frame, deterministically. Reproduced on two clean boots with no watchpoint armed at all, with the null pointer read at a breakpoint one instruction before the fault, and with rocket and **Missile** controls in the same session that both fired cleanly and did not halt - so the Missile, long paired with the LeachBeam under this crash, is not implicated. Full account: [the `00000030` halt](docs/ghidra/functions/psp-pulse-usa/bad-memory-access-halt.md).

**RPCS3 with no PS3 firmware installed fails in three different, non-obvious ways depending on how you ask, and a naive liveness check lies about all of them.** 2026-08-30, first `rpcs3-drive.py browse` run in this checkout. A fresh `~/.config/rpcs3/dev_flash` is empty (firmware is a separate, user-supplied install - a disc's own bundled updater under `data/extracted/<title>/PS3_UPDATE/PS3UPDAT.PUP`, where one is present, is a legitimate source, but installing it is the user's call, not a session default). Symptoms: a normal boot prints `Firmware is missing` and exits; `rpcs3 --no-gui --installfw <path>` refuses outright (`Cannot perform installation in no-gui mode!`); `rpcs3 --installfw <path>` with no `--no-gui` needs a real GUI pass and does nothing headless. **`pgrep -x rpcs3` never matches** - the actual process `comm` is `AppRun.wrapped` (the AppImage wrapper), so any liveness or exit check built on it is silently wrong in both directions; check `/proc/<pid>` or grep `AppRun.wrapped` instead, or better, read `TTY.log` growth the way `Session.wait_for_screen` already does. Once firmware is in place, RPCS3's own first-run **"Welcome" dialog is a native Qt window that overlays every screenshot** (nothing under it is visible, including circuit-select text) and nothing the gamepad sends reaches it - fix is `infoBoxEnabledWelcome=false` in `~/.config/rpcs3/GuiConfigs/CurrentSettings.ini`, not a button. **A manual `rpcs3 ...` launch that sets `DISPLAY=127.0.0.1:77` in one Bash call and launches in a later, separate call drops the export** - shell env state does not persist across this harness's tool calls - and the emulator opens a real window on the user's desktop instead of Xvfb, interrupting their session. `scripts/rpcs3-drive.py`'s `Session` sets `DISPLAY`/`QT_QPA_PLATFORM` in-process via `emulator_env()` and is not exposed to this; prefer it over a manual `rpcs3` invocation for exactly this reason. Separately, `EpilepsyWarning`'s live path needs a `cross` press or a boot stalls there for its whole timeout - see [hd-frontend.md](docs/formats/hd-frontend.md#the-boot-chain-declared-and-then-watched) and `Session.wait_for_screen_pressing`.

**A future bit-exact comparison of the sideshift timer columns will see a tiny negative residue where a clamped port reads exactly `0.0` - that is the original, not a bug.** 2026-08-27, capturing `sideshift-double-tap.inputs` off real PPSSPP for the first time. `Ship_UpdateSideshiftInput_q`'s own countdown (`0x08846a54`, decompiled via ghidra-mcp) is `if (timer > 0.0f) timer -= dt;` - gated, not clamped - so the tick that crosses zero still subtracts a full `dt` and lands slightly negative (`ss_shift_l` froze at `-0.000149006`, `ss_tap_window_l` at `-0.0004569963`, each held bit-exact for the rest of the run), then the field is never touched again. `crates/physics/src/airbrake.rs`'s `advance_sideshift` clamps with `.max(0.0)` instead and converges to exactly `0.0`. Left that way deliberately - every reader in the tree gates on `> 0.0`/`<= 0.0`, so the residue is behaviourally invisible, and the three fields feed `ShipState::hash_state`'s committed golden hashes, so matching it costs a hash regeneration for a cosmetic difference. Full evidence and the exact decompiled block:
[`input-bindings.md`](docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-countdown-does-not-clamp-at-zero-and-the-ports-does---deliberately-left-that-way).
If a bit-exact trace comparison is ever built and wants to match this too, it is a small, mechanical change (drop the three `.max(0.0)` calls in `advance_sideshift`, regenerate the affected golden hashes) - not a data gap.

**A fragment `<LoadXML>` never resolving is not always a missing file - HD's
own disc disables one by misspelling the tag.** 2026-08-26, diffing the three
copies of `speedlap_hud.xml`
([the HUD thread](handover/frontend/wipeout-hd-furys-hud-reads-and-the-reader.md)).
`DATA05`'s copy references `HUD_lap_ghost.xml` through `<aLoadXML>`/
`</aLoadXML>` instead of `<LoadXML>`/`</LoadXML>` - one added letter per tag,
which is also exactly `DATA05`'s 2-byte size difference from the otherwise
byte-identical `DATA00`/`DATA06` copies. `LoadXML_Item`'s reader keys on the
literal tag name, so this is not a fragment that fails to resolve and logs a
miss - it is a fragment the reader never looks for at all, deliberately
authored to be skipped without deleting the reference. If a HUD or front-end
composition is ever missing a feature that a sibling copy of the same root
has, check for a misspelled tag before assuming a parser or precedence bug.

**Reaching an arbitrary circuit through the front end is gated by Race Campaign
progression, not by anything `psp-drive.py` or the debugger can skip.** 2026-08-26,
chasing [airborne-ticks-have-been-captured-and-are-thin](handover/tooling/airborne-ticks-have-been-captured-and-are-thin.md).
RaceBox's Custom Race → Track Select is a **fixed** wrapping list of three -
confirmed live, up/down only, `right`/`left` genuinely do nothing there, matching
`docs/reverse-engineering/ppsspp-debugger.md`'s existing note - and the three are
exactly the `PI_Track` entries in `Data\Plugins\PI001\Definition.xml` carrying no
`<Unlock>` element at all: `16_Track` (Talon's Junction), `03_Track` (Moa Therma),
`18_Track` (Metropia reversed). Every other circuit's `PI_Track` node carries
`<Unlock Grid="N">`, and a fresh profile has only Grid 1 of 16 open. **The XML
attribute answers "how many grids" before a single button gets pressed** - no
need to grind blind: `13_Track` needs `Grid6` (five grids of real races first),
where `10_Track` needs `Grid0` (the currently-open grid, so just its own Race
Campaign cell). Check the target's own `<Unlock Grid="N">` before planning a
capture around it, and expect an `N` above 0-1 to mean the capture is not this
session's task.

**A device test with a *uniform* depth buffer cannot exercise any
depth-weighted term - it proves the pass runs, not that the weighting is
right.** 2026-08-26, found reviewing the motion blur chain. `post::motion_blur`
gathers its taps with McGuire's three weights, two of which are gated on
`soft_depth_compare`. Its device test cleared depth to a flat `0.5`, so both
gates returned `1` for every tap in the frame and the two cone terms absorbed
the whole weight - the test was green, and it was structurally blind to the
half of the filter that classifies surfaces. What hid behind that was a real
bug: the third, *ungated* term was written against the tile neighbourhood's
dominant reach instead of the centre pixel's own, so a still surface in front
of a fast one took full-weight taps of the background and read 199 of 255 at
its silhouette. Twenty lines of a second test with two depths caught it
immediately.

Generalise it, because the next weighted gather will have the same shape:
**whatever a filter classifies by, the test has to vary.** Uniform depth, a
single velocity, one material - each turns an assertion about a *decision* into
an assertion about an average. The cheap trick that made the second test easy
is worth keeping too: the pass binds depth as `Float { filterable: false }` and
`textureLoad`s it, so an `R32Float` texture with `COPY_DST` works and takes a
plain `write_texture` - no depth-writing pipeline, and none of `Depth32Float`'s
"you may not copy into me".

The same review turned the rule on the fix and it bit twice more. Splitting
tile-max into two axes needed a frame size the tile edge does **not** divide,
because a power-of-two frame makes every tile square and full - and it needed
motion that varies in *both* axes, because a velocity that is constant down
each column lets any row of the horizontal pass answer for any other. Sizing
the split's intermediate target wrong on purpose left two green tests before
a third one, varying both, caught it. **A test that does not vary what the
code branches on is a test of the average, not of the decision.**

**A reach cap that doubles as a tile size makes a square reduction starve the
GPU.** Same review. `post::motion_blur`'s tile-max reduced each `K x K` square
in one pass, `K` being the blur's own pixel cap - 8 % of viewport height, so 87
at 1080p. That is 7,569 serial `textureLoad`s inside each of only 299
fragments: the right amount of arithmetic (every pixel once) at almost no
occupancy, and it measured 1.5 ms of the chain's 2.6 ms on an Intel Arc iGPU.
Separable - horizontal to `ceil(w / K) x h`, then vertical onto the tile grid -
is the same arithmetic in 24,840 fragments and took the whole chain to 1.2 ms.
The tell is the *fragment count*, not the loop: a fullscreen pass whose target
is much smaller than its source is where this hides.

The reproducible measurement is the table in
[`docs/rendering/motion-blur.md`](docs/rendering/motion-blur.md) - 2.6 ms
square, 1.2 ms separable, 1.1 ms with the reduction stubbed out entirely - not
the stub-the-loop isolation that first pointed at it.

**A worktree sharing `CARGO_TARGET_DIR` with the main checkout compiles against
the *other* tree's uncommitted code.** 2026-08-18, and it cost a confusing hour.
Setting `CARGO_TARGET_DIR=<repo>/target` from `.claude/worktrees/<name>` looks
like a free win - the dependency builds are already there and a workspace build
drops from minutes to seconds - and it works right up until the two trees'
sources differ. Then `just test` in the worktree fails with
`missing field 'lightmaps' in initializer of oag_mesh::mesh::Model` against a
`mesh.rs` that has no such field, because the field is in the *main* checkout's
work in progress. Nothing in the error names the other directory.

It also produces a subtler failure first: a run that behaves as though an edit
had not landed, then behaves correctly a few commands later. Do not share the
directory. A worktree's own `target/` is gitignored already (`/target/` in
`.gitignore`), and the first build is a few minutes once.

The other half of the same setup is worth knowing: **`data/` does not travel
into a worktree**, being gitignored, so `data/images/` looks empty there.
Symlink `data/images` (and `data/cache`) at the main checkout's, and do **not**
`rm -rf data` to do it - `data/README.md` is tracked.

**A grid of eight identical craft is what an *empty* roster looks like, not what
a broken one looks like - so nothing failed and nothing said so.** 2026-08-18.
Wipeout HD raced eight Assegais because two independent faults each produced an
empty team list, and `livery::teams_for_slots` did the honest thing with one:
every slot gets the player's team. Both are fixed. The general lesson: a
fallback that produces a legible picture is the hardest kind to notice - both
sites now report the *reason* rather than a count of zero, since the report
line used to read `0 team(s)` for two unrelated causes in a row and hid the
second behind the first. Full account, including that HD ships its plugin
definition **five times** across seven archives with the copies disagreeing
(`DATA00` declares twelve teams/28 circuits, `DATA02` eight/eight, precedence
serves `DATA00`):
[hd-status.md](docs/formats/hd-status.md#it-ships-five-times-and-the-copies-disagree).
The two faults, briefly: (1) `race::load`/`boot::definitions` read Pulse's
numbered plugin path (`Data\Plugins\PI001\Definition.xml`) whatever title was
open, where HD names its own (`Data\Plugins\Frontend\Definition.xml`) - fixed
via `oag_title::Title::plugin_definition`, the fourth axis moved into the
title package for this reason. (2) `fexml::expand` refuses a file with no
`<code>` dictionary, and HD's definition is plain `<?xml`; **any new
`read_name` of an XML entry should be `fexml::text`, not `fexml::expand`,
unless the file is known to be shortened** - the trap was already written
down for the PS2 in-race HUD and still caught this caller.

**A name-variant sweep is only as good as its extension list, and one whole
class of PS2 names was missing because `.pct` was not in it.** 2026-08-18.
`Data\Tex\engineFlare\Engine_noise.mip` is on the PS2 disc; the lookup was
wrong, not the disc. The PS2 texture resolver rewrites `.MIP` and `.TGA` to
`.PCT` before it hashes, so **none** of the 18 `.mip` or 32 `.tga` literals in
`SCES_547.48` resolve as spelled and 47 of 50 resolve rewritten, while every
`.pob`, `.vex`, `.bnk` and `.xml` name resolves on both discs untouched.
Mechanism: [texture-names.md](docs/ghidra/functions/ps2-pulse-eu/texture-names.md),
implemented as `oag_pulse::ps2_texture_name`. **The general lesson: when a
whole class of names misses and every other class hits, the answer is a
transformation the loader applies, not more spellings** - the sweep that
missed this tried 60 candidates across path shape and case, and any remaining
PS2 asset gap phrased as "not in the archive set" deserves the rewrite tried
first. [loading-screen.md](docs/ghidra/functions/ps2-pulse-eu/loading-screen.md)
had the `.mip`-to-`.pct` rewrite written down for the loading screen since
2026-08-07, read at the time as a quirk of that one screen. Two more instances
were found and fixed this way on 2026-08-18 - the two exhaust textures and the
loading screen's glow strip (`Data\Defaults\Loading\LoadingPulseOverlay.mip`,
`WADS2.WAD` entry 138, which had been falling back to
`GlowStrip::placeholder` every boot). The sweep behind that covered
`read_name` call sites in `crates/game/src` and `crates/render/src` only; its
two remaining texture-path hits (`livery.rs`, `race/load.rs`) are `mesh::rcs`
callbacks and so PS3-only. **No sweep outside those two crates has been
done**, so treat this as two instances found, not the complete list.

**Fixed in place, 2026-08-28 - the workaround below is no longer needed for
`ps3-hdfury-eu`.** The maintainer ran `scripts/import-ps3-eboot.sh --ps3-cspec`
(Ghidra closed first, as the script requires); verified directly afterward:
language reads `PowerPC:BE:64:A2ALT-32addr-PS3`, still 26,100 functions, real
imports, and `scripts/apply-ghidra-names.py` re-applied all 114 of this
binary's `names.tsv` rows clean. `AssignPs3R2FromOpd.java` now runs as part
of the normal import (`-postScript`), so every function's TOC-relative load
should resolve correctly without `ps3-toc.py` cross-checks going forward -
though any *pre-existing* decompile read or claim made before this date and
not yet re-verified should still be treated with the same suspicion the rest
of this section describes, since it may have been made under the old,
TOC-defective database. The history below is kept for the lesson, not as a
live workaround to repeat.

**The Ghidra bridge cannot run scripts, so the PS3 TOC defect has to be worked
around rather than fixed in place.** 2026-08-18. `GHIDRA_MCP_ALLOW_SCRIPTS` is
unset on this bridge, so both `run_ghidra_script` and `run_script_inline` refuse
with "Script execution disabled" - which means **`AssignPs3R2FromOpd.java`
cannot be applied to the live `ps3-hdfury-eu` database through MCP at all.** The
next session will reach for it, because
[memory.md](docs/ghidra/functions/ps3-hdfury-eu/memory.md) correctly names it as
the fix. The two routes that do work are the full
`scripts/import-ps3-eboot.sh` re-import (needs Ghidra closed, and is heavy), or
**`scripts/ps3-toc.py`, which resolves each function's own TOC against the file
with no Ghidra at all** - added 2026-08-18 for exactly this. Its self-check is
`scripts/ps3-toc.py resolve 0x003914b0 -0x6634`, which must print
`Small is  %3.2f MB (%d bytes)` and not `forward`. **Its `map` mode is validated
against `Collision.cpp`**, whose two attributed addresses are already in
`names.tsv` from independent work. The failure mode it avoids is not a visible
error: in module-B functions Ghidra renders TOC-loaded *floats* as string
pointers, so arithmetic decompiles as `(float)PTR_s_Emp__d_008a754c` and reads
like a nonsense variable name rather than a wrong base.

**The same defect has a second failure mode, and this one doesn't look
wrong.** 2026-08-25. Hunting for where HD's HUD resolves a texture reference's
extension to `.gtf` (see
[the HUD thread](handover/frontend/wipeout-hd-furys-hud-reads-and-the-reader.md)),
`get_xrefs_to` on the literal string `SrcRel` reported a real-looking hit:
`FUN_00550eb4`, decompiling (under Ghidra's one global TOC) as a function
that opens with a call site literally named `PTR_s_LoadXML_Item_cpp_...` and
checks `strncmp(param_2, "Values", 0xb)` - a thematically perfect match for
the LoadXML fragment loader this thread was chasing. `scripts/ps3-toc.py toc
0x00550eb4` says its real TOC is `0x008bd3c4`, not the global
`0x008ad4d8` - so every one of those names is Ghidra decompiling module-B
code against module-A's table, and it happened to land on another *real,
coherent* table (LoadXML_Item's actual attribute-name strings live a few
words further into the same data region) rather than garbage. The `0xb`
in that `strncmp` is the tell once you know to look: `"Values"` is 6
characters, but `"Set-Cookie:"` is exactly 11 - the function is
`CCookie.cpp`, confirmed by reading its real TOC's neighbouring strings
(`Set-Cookie:`, `expires`, `domain`, `path`, `secure`) directly with
`inspect_memory_content` rather than trusting Ghidra's resolution. **The
general lesson: `get_xrefs_to` and decompiled symbol names for any
TOC-relative load are only as trustworthy as the target function's TOC
matches the global one** - check first with `scripts/ps3-toc.py toc <addr>`
(`0x008ad4d8` means trust it, anything else means don't), and prefer
`scripts/ps3-toc.py map` (grep the `.cpp` list) plus raw
`inspect_memory_content` reads over `search_strings`/`get_xrefs_to` when
hunting a specific mechanism in a function whose TOC hasn't been checked.

**Even with the TOC defect fixed, a function that calls through an unknown
pointer can still make Ghidra hide a real TOC-relative global behind a fake
local variable.** 2026-08-28, past the `--ps3-cspec` reimport above, on
`Environment_LoadStageTextures` (`0x003d6dc8`,
[zone-effectsettings-loader.md](docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)).
Every ELFv1 indirect call (`lwz r2,0x4(rX)` loading a *different* object's TOC,
then `std r2` / `ld r2` round-tripping the caller's own TOC to the stack
around the call) breaks Ghidra's constant propagation even when it is
provably faithful - so a `-0x58xx(r2)` load *after* such a call decompiles as
an opaque `iStack_NNNN` read, indistinguishable at a glance from a genuine
stack variable or a missing parameter. The tell is `analyze_dataflow`
(backward, from a use of the mystery variable): it terminates at "function
input" rather than a real definition, which is what a value equal to entry
`r2` looks like once the direct chain of assignments is too indirect to
fold. Once recognised, the fix is the same as ever - resolve by hand against
the function's own already-known TOC - but the *recognition* step is new:
don't assume a Ghidra-invented local is really local just because the TOC
defect is fixed; check whether it's actually `r2` in disguise first.
This is binary-wide, not specific to this one string or this one function.

**A related but separate trap, hit later the same session: proximity to a
known-good table is not evidence of relatedness, TOC issue or not.** The
same HUD-thread session chased `get_xrefs_to` hits clustered around
`008ad9e0`, one word away from `LoadXML_Item`'s real attribute-name pointer
table at `008ad9f0` - a dozen-plus hits, in the right file's address range,
looked like exactly the read-the-table code being searched for. It wasn't:
`008ad9e0` is a generic pooled-object free-list head that many unrelated
classes' destructors happen to share, and every one of those "hits" was a
destructor releasing an unrelated smart-pointer field. **Read the actual data at the address in question** - the `.rodata` string
run itself (`0078bf00`, holding the real literal names) rather than the
nearby pointer table that references it (`008ad9f0`, one word from the
free-list head that produced the false lead) - rather than trusting that a
cluster of xrefs near a landmark means the landmark. This one doesn't need a
TOC check to avoid: both addresses were module-A/global-TOC and the xrefs
were technically real; the inference "near my table, therefore about my
table" was simply wrong.

**Deferred from the 2026-08-18 renderer sweep**, so nobody re-derives that these
are open: the **draw path** (inline command-buffer writes on PS3, so no import
census or call graph finds it - it needs a search for RSX method constants);
**`SortRoot.cpp`'s four constructors** (`0x002dbe08`, `0x002dbe50`, `0x002dbe98`,
`0x002dbf40`) - **spent as a render lead, 2026-08-25**: `0x002dbe50` turned out
to have six real callers after all (base cannot be told from complete *and
none is named* was true only for the other three), and chasing them one level
further landed in `DetonatorBomb.cpp` - a bomb weapon's constructor, now named
([`detonator-bomb.md`](docs/ghidra/functions/ps3-hdfury-eu/detonator-bomb.md)) -
tied to `RaceManager_GetInstance()`. `SortRoot`'s only located use is gameplay
code two hops from the render layer, not the "draw-order root" the table entry
guessed; treat that guess as refuted, not merely unconfirmed. **The render
draw-order question itself is now answered anyway, 2026-09-04, and not via the
RSX-method-constant search this entry used to point at**: rereading
`RenderManager_FlushDrawQueue` directly found the `qsort` call the earlier
pass missed, with a comparator (`RenderManager_CompareQueueKeys`, `0x002d4b58`)
byte-identical in shape to Pulse's `Gfx_CompareQueueKeys` - same `{item, key}`
layout, same ascending `a->key - b->key` on `+0x04`. See
[renderer.md](docs/ghidra/functions/ps3-hdfury-eu/renderer.md#the-per-eye-draw-dispatch-and-what-it-says-about-sort-order)
and [the handover thread](handover/rendering/pulses-draw-order-is-recovered-and-it-is.md).
**What HD's `+0x04` key encodes is answered too, same day**: an enqueue
idiom inlined at five confirmed producer call sites, twelve bits of layer
over twenty of back-to-front depth - Pulse's exact split, one layer constant
(`0x4d0`) even matching Pulse's own `ExhaustFlare_Submit` key byte for byte.
See [renderer.md](docs/ghidra/functions/ps3-hdfury-eu/renderer.md#the-enqueue-idiom-and-what-the-0x04-key-encodes).
What still isn't established is which class each of the five sites belongs
to, and where the per-instance depth-override field they all read
(`instance+0x11c`) gets written - and -
**now closed** - where the engine-owned shader
programs' microcode lives: 124 `SHO` blocks linked into `EBOOT.elf` against
exactly 124 registered names, same container as a `.rcsmaterial`. Two things
stay open there and are on the page. **Only 62 of the 124 name-to-blob pairings
are measured**; the rest match by count alone, because the other 62 names
register by a route a whole-image direct-branch scan does not find - and the
plausible explanation is disproved, not untested: `0x005cd700`, the registry's
second constructor, has **zero branches to it anywhere in the image**. And **no
microcode has been disassembled**, so nothing says what any program does.

**A model reports the right triangle count and still renders nothing - check
the alpha test, not the class table.** 2026-08-17. Wipeout Pure's `Speedup
Pad`/`Weapon Pad` class ids were recovered 2026-08-13, `mesh::build_pads` is
title-agnostic through `vex::classes::V4`, and `Data\Environments\01_Vineta_K\track.vex`
decoded 14 `Speedup Pad` nodes, 952 triangles, correctly - every load report
line said so. **The pad still drew zero pixels**, because `mesh.wgsl`'s
`ALPHA_TEST_THRESHOLD` was an invented `0.5` and the pad's own glow texture
(`speedup_GLOW_KEY.tga`) tops out at alpha 58/255 (~0.23): every fragment of
every cutout batch discarded. A geometry decode that reports a plausible
triangle count is not evidence a model draws anything - `oag-view --pads
--screenshot` (or the new `crates/render/tests/pad_alpha_test_ground_truth.rs`)
is the check that would have caught this, and neither ran until the picture
was asked for directly. Fixed by lowering `ALPHA_TEST_THRESHOLD` to `1/255`
(`0`, one of three real GE references `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`
recovered for this branch) - see `docs/formats/vex.md`'s materials section.

**Since 2026-09-10 the reference is per batch and read off the file, and the
pad's own value turns out to be that same `0`** - it is the one selector
pattern that separates the recovered branch from the obvious rival reading, so
the pad went from "saved by a permissive default" to "the case that settles the
branch". Selector, census over three discs and the measured pixel cost:
`mesh-draw.md`, "The alpha-test reference's selector, and the pad that
discriminates", confidence 86. The trap above is unchanged and still the point:
a decode that reports a plausible triangle count is not evidence a model draws.

**And the pad broke a second time the same day, from the other direction:
`GU_GREATER` reproduced as `<` is `GEQUAL`, and at reference `0` that is no
alpha test at all.** 2026-09-10, one commit later. `mesh.wgsl` had always
discarded at `shaded.a < alpha_test_ref`, which was indistinguishable from the
correct `<=` while Wipeout HD's `0.5` was the only reference in play - the
alpha is 8-bit and `0.5` sits between `127/255` and `128/255`, so no texel
could land on the boundary, and the shader's own comment said so. Every
reference read off a `.vex` batch lands on a texel value exactly, and
`a < 0.0` is true of no fragment: Pure's `Speedup Pad` painted its glow
texture's transparent background as a **solid square plate**. Two lessons, and
the second is the reusable one:

- A comparison that is *exact either way on today's data* is a latent bug, not
  a settled one. The rationale for `<` was true and load-bearing right up to
  the commit that made it false, and nothing re-read it.
- **`pad_alpha_test_ground_truth`'s `lit > 100` passed harder than before.** A
  one-sided count cannot see a cutout that stopped cutting - a filled quad is
  strictly *more* lit pixels than a cutout of it. The test now captures the pad
  twice, once as authored and once with the reference forced below any alpha a
  texel can carry, and asserts the first keeps materially fewer pixels than the
  second. Measured 310 against 425; under the broken discard it was 425/425.

**A one-seed race comparison cannot resolve two adjacent difficulties, and the
test that did it read as a regression on an improvement.** 2026-08-17.
`every_difficulty_is_quicker_than_the_one_below_it` ran one 3,600-tick race per
level and compared the leader's distance. Measured: novice 6,704, skilled 7,199,
elite 7,766, ace 7,810 - the first two gaps are 495 and 567 and **the last is
44**, half a percent, on a system of seven drivers trading places. A change that
made *every* level faster (the stall rescue freeing a wedged craft: elite 8,078,
ace 8,075) duly failed it by three units in eight thousand. Averaged over five
seeds the ordering is real and comfortable - 6,477 / 7,095 / 7,637 / 7,751 -
while elite still beats ace on two seeds of the five. **The test now averages and
prints the spread**, and moved to `crates/game/tests/difficulty_ground_truth.rs`
(which took `race_ground_truth.rs` from 2,464 to 2,387, ratcheted in
`check-file-size.py`). The general form is worth keeping: **an ordering that
holds on the mean and not on one run is the measurement being noisy, not the
setting being broken** - print the spread before touching the thing under test.

**A synthetic race fixture now has a horizon, and it is `STALL_TICKS` long.**
Same day. `race_with_a_grid` builds craft on `Handling::ZERO` with an empty
collision world, so no force law runs and the craft hold full throttle without
moving - which is exactly the condition the stall rescue catches. Any synthetic
race test that ticks past 120 now recovers all seven opponents and zeroes the
throttle on the tick it does. `the_opponents_are_driven_rather_than_parked` runs
`STALL_TICKS / 2` for that reason. A fixture that wants to run longer has to give
its craft handling that moves them.



**Subsystem traps have moved to their pages**:
[surveying a disc](docs/formats/wad.md#traps-when-surveying-a-disc),
[the Ghidra database versus `names.tsv`](docs/ghidra/workflow.md#the-database-can-disagree-with-namestsv-and-the-docs-win),
[PPSSPP session hygiene](docs/reverse-engineering/ppsspp-debugger.md#session-hygiene),
and the `ffmpeg`/`ffprobe`/GStreamer traps on
[frontend-boot.md](docs/architecture/frontend-boot.md). What is left is this
machine and this checkout.

- `data/` could be on `ecryptfs`. **`cp --reflink` does not work** (copying the images
  is a real multi-GB copy), and `ls`/`rg` intermittently fail with "permission
  denied" or "no such file" on directories that plainly exist. `fd`, `find` and
  absolute paths work.
- **Backticks in a justfile are command substitution at parse time**, even
  inside an `@echo` string - an echoed example command once ran a full
  trace-compare on every `just` invocation, `just --list` included.
- **`XDG_CONFIG_HOME` is how you point `oag-game` at a scratch `settings.toml`**
  for A/B config captures without touching `~/.config/oag/`.
- **A niri screenshot piped through `wl-paste` can return the PREVIOUS clipboard
  image.** One lap "verification" shot came out as a menu from an earlier
  session. `psp-trace.py` now detects this and writes `.stale.png` rather than
  citing it, but nothing else does. `niri` screenshots land in
  `~/Pictures/Screenshots/`.
- **`/tmp` here is quota-limited well below the project's own disk.** Extracting
  a large WAD into it once broke the shell entirely - every command failed,
  `echo` included, until the partial extraction was deleted. Scan through
  `oag_assets::Archive` in memory instead.
- **Shipped `section` data is not tidy, and a strict parser refuses real
  tracks.** ~2 % of PVS mask bits name a section the file does not declare, one
  id is authored three times over on four tracks, and 116 PS2 control points
  name a section their file lacks. All benign; see
  [`track.md`](docs/formats/track.md).
- **No capture will ever show the BRIGHTNESS or GAMMA settings.** They apply at
  the blit, which `--screenshot` and the race capture never reach, so a picture
  that looks ungraded is not a bug. The real check is
  `upscale::tests::the_grade_moves_the_picture_in_the_direction_the_setting_names`,
  which builds the actual pipeline on a headless target and reads the pixel back
  (mutation-checked by swapping brightness and the exponent in the shader). It
  **skips with a note where there is no adapter**, so a green CI run does not
  mean it ran.
- **Judge line width at native 480x272 only.** Anything upscaled makes a
  one-pixel authored line a magnification artefact argument rather than a
  fidelity one.
- **`--features native-video` makes the boot fifty times slower, and `just play`
  turns it on** (Linux only, `justfile:25`). Measured on `pulse-psp-eu.chd`,
  caches warm: **4.9 s** of boot with it (intro 2.7, backdrop 2.2) against
  **0.08 s** without - `GstDecoder::open` decodes every frame into memory
  before returning (~288 MiB for the two reels) where the AV1 cache path
  decodes on demand.
  [ADR-0017](docs/architecture/adr/0017-gstreamer-native-video.md) chose that
  deliberately, costed at 270 frames/53 MiB for the backdrop (the intro is
  1200 frames). The boot prints its own timings now, so this is re-measurable
  rather than folklore. Making the GStreamer path lazy would remove the wait
  rather than hide it, and is not done.
- **Two things convert into `data/cache/movies` and they must never overlap**
  - the boot's own two reels and the `--prefetch` worker both shell out to
  `ffmpeg`, and two processes writing one cache file corrupts it rather than
  racing safely. `Session::start_prefetch` is the only thing serialising them
  now that movies run on `boot::MediaWorker`. `--refresh-video` (2026-08-12)
  makes the worker stop skipping cached movies, so combined with `--prefetch`
  it re-converts the boot's own two reels too - wasteful but still ordered.
- **The loading screen's bar vanished on every ordinary boot, and the gate
  wasn't the cause.** `0 / 0 converted` beside a full `100%` bar was real: the
  boot's media phase reported no counts at all, and the display bundled bar,
  counts and percentage into one row keyed on `total > 0`. Fixed 2026-08-12 by
  giving the phase real counts (`boot::MediaPlan::loads`,
  `boot::MediaWorker::progress`) rather than loosening the gate - **a display
  gate keyed on "is there data" reads "nobody reported" and "nothing to
  report" as the same thing.** The bar is now continuous (each load fills its
  own slice from its own step, so a minute-long transcode crosses 40%->60%
  rather than holding at 40%); the invariant to preserve is monotonicity.
  [frontend boot](docs/architecture/frontend-boot.md#loading-is-not-transcoding).
- **`--prefetch` could not fill the movie cache on a `native-video` build, and
  said it had.** Found 2026-08-12. The GStreamer path writes no cache file -
  it decodes into memory with a `{key}-gst` pseudo-path - so
  `prefetch::convert`'s `report_picture` saw a picture and called it
  converted, while the planning pass found nothing cached and relisted the
  same 22 movies every run. Fixed by making `prefetch::convert` set
  `movie::Decode::prefer_cache` unconditionally (filling the cache is the
  worker's whole job, not a preference); confirmed empirically, the cache
  went from 9 to 21 `.ivf` files in three minutes. **The general trap: a
  success/failure check that asks "did this produce a picture?" cannot see
  that the *side effect* the caller wanted did not happen.**
- **An existing AV1 cache file now beats the platform decoder with no flag,
  and ADR-0017's default still stands** - compatible because 0017 is only
  about the case with nothing cached, where GStreamer still wins;
  `open_psmf` now asks `movie::cached` first. Measured on `pulse-psp-eu.chd`,
  both reels: 4.80 s of media phase through GStreamer, 36.08 s once to build
  the cache (intro 30.33, backdrop 5.74), 0.06 s on every run after.
  `--prefer-av1-cache` builds it for the boot's reels, `--prefetch` for all
  22, `--refresh-video` implies it. ADRs are immutable, so nothing edited
  0017; making the *uncached* default the cache would need a new one.

**The Ghidra bridge's `switch_program` can silently no-op, and its own reply
says success when it does.** 2026-08-19. Four of this project's binaries share
the bridge's display name `BOOT.BIN` (`psp-pulse-usa`, `psp-pulse-eu`,
`psp-pure-usa`, `psp-pure-eu`), and `rename_function_by_address` /
`create_function` both ignore the `program` field on the request and act on
whichever program the bridge currently considers *truly* active - unlike
`get_function_by_address`, which resolves `program` correctly on every call.
`switch_program` is supposed to be the fix, but confirmed live: it returns
`{"success":true}` while leaving the true active program completely
unchanged, for any of the three non-`psp-pulse-usa` `BOOT.BIN`s. Tried
switching away to a known-reachable program first in case it was
call-order-dependent - it wasn't; those three are unreachable for writes
through this bridge session regardless of sequence. `ps2-pulse-eu` and
`ps3-hdfury-eu` are unaffected (uniquely named, genuinely switchable).

The failure mode is not a loud error, it is silent cross-binary corruption:
a `just apply-names` run for, say, `psp-pulse-eu` reports success while
writing `psp-pulse-eu`'s documented names onto `psp-pulse-usa`'s addresses
instead, at whatever address happens to also be a function start in the
wrong binary. This had been happening for a while - live `psp-pulse-usa` on
2026-08-19 carried 234 stray names (91 mismatched, 143 unsanctioned) before
cleanup, every one traceable to a name legitimately documented at a nearby
address in `psp-pulse-eu` or `ps2-pulse-eu`. `scripts/apply-ghidra-names.py`'s
`Bridge.switch_program` (and `scripts/audit-ghidra-names.py`'s `--prune`, via
`ensure_active`) now distrust the reply and re-read the bridge's true active
program afterward with no `program` parameter, aborting loudly instead of
writing blind - so `just apply-names` now correctly refuses `psp-pulse-eu`,
`psp-pure-usa` and `psp-pure-eu` outright rather than corrupting whichever
program was truly active. **Those three binaries cannot be named through this
bridge at all right now** - that is a `bridge-mcp-ghidra` limitation, not
something fixable from either script. `psp-pulse-usa` was cleaned (audits
clean as of 2026-08-19); `psp-pulse-eu`/`psp-pure-usa`/`psp-pure-eu` still have
no way to apply their `names.tsv` live until the bridge can disambiguate
same-named programs, or until each is loaded in a Ghidra project without a
`BOOT.BIN` name collision.

**Correction, 2026-09-03: two of those three were never a bridge
disambiguation limit - `psp-pure-usa` and `psp-pure-eu` simply were not open
in the tool.** `list_open_programs` returned 8 programs, not the 10
`list_instances` implied (that call reports the *project's* files, not what
the tool has open); `psp-pure-usa`/`psp-pure-eu` were missing from it, which
is why `switch_program`/`get_function_by_address` reported them unreachable.
One `open_program` call each fixed it - both are reachable and writable
afterward, no bridge fix needed. `psp-pulse-eu` was never actually blocked
either: this same session's `just apply-names` run applied all 301 of its
rows cleanly with no `open_program` needed, so the `BOOT.BIN`-collision theory
above does not hold for it. **Check `list_open_programs` before believing a
program is unreachable** - a closed program and a same-name collision look
identical from `switch_program`'s side, and only the first is a one-call fix.
All four `BOOT.BIN`s (`psp-pulse-usa`, `psp-pulse-eu`, `psp-pure-usa`,
`psp-pure-eu`) are confirmed live and cleaned as of this session: a sample
function and a sample data label were checked post-`apply-names` on each via
`get_function_by_address`/`list_globals` and all read back with their
documented names rather than `FUN_`/`DAT_`.

**A duplicate address in `names.tsv` had silently broken `just apply-names`
for `psp-pulse-usa` since 2026-09-02, with nothing catching it offline.**
`apply_group`'s duplicate-address check runs before any bridge call and
refuses the whole group on the first hit, so the 2026-09-02 Plasma commit's
`plasma.md` re-documenting two functions (`WeaponStats_ParseTurbo`,
`WeaponStats_ParseAutopilot`) that `weapon-stats.md` already owned - both
rows correct, both pages genuinely support them, just one address named
twice - meant `psp-pulse-usa`'s whole group aborted with "duplicate address
0x0880c92c" and never reached a single rename. `just check-names` (the
offline gate CI actually runs) had no duplicate-address rule, only the
evidence/suffix/floor ones, so this was invisible to `just` and to review.
Fixed both ways: the two `plasma.md` rows deleted from `names.tsv` (the
`weapon-stats.md` rows already covered the same functions), and
`check-ghidra-names.py` gained a `check_duplicates` rule so a repeat trips
the gate offline instead of surfacing as a live bridge failure a session
later.

**`psp-pulse-usa`'s Ghidra DB stores every `jal` target pre-relocation, so
`get_xrefs_to`/`get_function_callers` return "none" for almost all of them.**
Found 2026-08-11, re-derived independently 2026-08-19, and it cost most of a
session before the mechanism was understood. Ghidra's *static* disassembly
bytes for a `jal` hold the unrelocated `R_MIPS_26` field - decoding them with
the ordinary MIPS rule lands on a small, bogus sub-`0x08000000` address, not
the real target; the *running* program's bytes at the same address are
already relocation-patched and decode correctly. The fix: add this program's
own image base (`0x08804000`, from `get_current_program_info`) to the
low-26-bits-shifted-left-2 value read from Ghidra's static bytes -
`target = image_base + ((word & 0x03FFFFFF) << 2)`, no `(addr+4) &
0xF0000000` term. Confirmed on every `jal` checked so far, including all six
inside `Body_ResolveContactPair`. **Practical consequence**: when an xref
tool says "no callers", it is not evidence of that - search
`search_instructions` on the zero-padded *relative* operand
(`jal 0x000645cc`, not `jal 0x0886a5cc`) instead of trusting the "none"
result. Full derivation, and confirmation this is not database corruption:
[contact-response.md](docs/ghidra/functions/psp-pulse-usa/contact-response.md#its-call-target-needed-the-image-base-added-by-hand---ghidras-own-bytes-are-pre-relocation).
**Settled 2026-08-27: it is not regional, it is every one.** A different
session had independently concluded (2026-08-17, since retracted - see
[weapon-fire.md](docs/ghidra/functions/psp-pulse-usa/weapon-fire.md#read-this-first-the-disassemblys-addresses-are-image-base-relative))
that two address ranges were exempt; `search_instructions` across the whole
program found zero `jal` anywhere - not just those two ranges - that
displays an already-relocated (`0x08`-prefixed) target, out of 524,728
instructions scanned, and the specific counter-example instruction that
claim cited does not exist in the database. Apply the workaround
unconditionally to a static `jal` reading here; there is no exempt region.
Whether this is a general PSP-ELF-relocation trait or specific to how this
project's importer handled `BOOT.BIN`, and whether it is related to the
decompiler failing on several functions in the same file, are both still
open - no evidence ties either to the mechanism above.

**It is not just `jal`: `lui`/`lw` HI16/LO16 pairs and raw 32-bit data words
carry the same pre-relocation values, and the fix generalises.** Found
2026-08-27 while chasing the three unidentified globals on
[zone-mode.md](docs/ghidra/functions/psp-pulse-usa/zone-mode.md#not-determined)'s
case-4 arm: `disassemble_bytes` on `lui a0,0x2b` / `lw a0,-0x37c8(a0)` at
`0x088444b0` gives a naive address of `0x002ac838` - small, and in no mapped
section - and `get_xrefs_to` on that naive value duly finds nothing, same
failure shape as a `jal`. Adding the image base the same way the `jal` fix
does (`0x08804000 + 0x002ac838 = 0x08ab0838`) reproduces the address the
2026-08-24 pass already had by hand, and does the same for the other two
globals on that page (`0x08ab2120`, `0x08ab10e8`) against their own naive
values. A 32-bit data word is the same story one level removed: the pointer
stored at that now-correct `0x08ab0838` slot reads `88 59 27 00` (LE
`0x00275988`), which resolves to nothing on its own but to the `"PickupBackground"`
string table entry once the same `+0x08804000` is applied
(`0x08804000 + 0x00275988 = 0x08a79988`, confirmed by `inspect_memory_content`
landing on readable text). **Same fix, three different encodings** - a
26-bit jump target, a 16+16 hi/lo pair, and a plain 32-bit word - so the
practical rule widens: any address-shaped value read off this program's
static bytes (not already surfaced via a working xref) is a pre-relocation
candidate, not just call targets. `get_xrefs_to`/`get_xrefs_from` on the
naively-read address will report nothing either way, which reads exactly
like "no reference" and is not.

**Corrected 2026-09-02: `+0x08804000` is not the whole rule for a `lui`/`lw`
HI16/LO16 pair - it is per-segment, and this binary has two.** All three
`0x08ab08xx`-shaped examples just above are `.data`, which is segment 0
(`PT_LOAD` `VirtAddr 0x00000000`) and does get `+0x08804000` - that part
holds. A fourth case (`AnimTransform_Update`'s clock fallback, found chasing
[the anim-transform thread](handover/rendering/anim-transform-0x3c0-is-read-and-played-and.md))
resolves under that base to `0x0885c018`, which is *inside a real function*,
not a global - the tell that the base is wrong for it. This binary's second
`PT_LOAD` segment (`VirtAddr 0x002d5798`, covering
`.cplinit`/`.linkonce.d`/`.ctors`/`.bss`) gets its own base,
`0x08804000 + 0x002d5798 = 0x08ad9798`; applying that instead reproduces
`texture-animation.md`'s independently-recorded `DAT_08b317b0` exactly, and
the `.rel.text` record confirms it directly - bits 16-23 of `r_info` are `1`
for this pair and `0` for every confirmed `.data` case, i.e. the segment
index. `jal` targets are unaffected (a call target is always in `.text`,
segment 0), so the "no exempt region" call above still holds for those; a
HI16/LO16 pair or a 32-bit data word that resolves into `.bss`, `.cplinit`,
`.ctors` or `.linkonce.d` needs the segment-1 base instead. Full arithmetic
and the relocation-record check:
[anim-transform.md](docs/ghidra/functions/psp-pulse-usa/anim-transform.md#the-second-relocation-base-is-found-it-is-per-segment-not-per-image).
**2026-09-05: that HI16/LO16 route did turn up for `DAT_08b32428`, and it settled
two things at once.** First, the `$gp` premise was wrong: this binary uses `$gp`
as a load/store base register **zero** times in its entire 521,965-instruction
body, so `DAT_08b32428` was never `$gp`-relative to begin with - the actual
mechanism was this naive-value trap all along. Second, applying the segment-1
base (`+0x08ad9798`) appeared not to reproduce the known address while a
different constant (`+0x08ad9450`) did.

**Retracted the same day, and this is the part to read: stop doing the
arithmetic by hand.** `scripts/psp-relocate.py` replays the PRX relocation
records instead of decoding immediates, and it shows the second half above was
a misread. `DAT_08b32428`'s base is built by `addiu ?,?,-0x7378`, not
`-0x7030`; the naive value is `0x00058c88`, not `0x00058fd0`; and it resolves
under the ordinary segment-1 base. The three naive values the "third base"
rested on have **zero** relocation records between them, and each is exactly
`+0x348` off a real one - one constant slip, cancelled by inventing a base
`0x348` lower, which is why three cross-checks all seemed to agree. **There are
two `PT_LOAD` segments and therefore exactly two bases.** The two write sites
were real but their addresses were `0x134` low; corrected in
[missile.md](docs/ghidra/functions/psp-pulse-usa/missile.md).

The script reproduces five independently-recorded addresses with no
special-casing and lands 97.85% of 108,813 resolved targets inside a loaded
segment. It also carries the two searches Ghidra cannot do here: `xrefs` on the
relocated value, and `member` for a global reached as base-plus-offset -
`DAT_08b32428` is `DAT_08b32420 + 8` and has no record of its own, which is the
zero that started this whole detour. See
[workflow.md](docs/ghidra/workflow.md#stop-doing-it-by-hand-scriptspsp-relocatepy).

**`get_xrefs_to` can work after all - on the naive address, not the relocated
one.** Found tracing `g_ingame`'s readers for the same thread: `get_xrefs_to`
on `0x08ab0818` (the relocated, correct address) finds nothing, as this whole
section predicts - but `get_xrefs_to` on `0x002ac818` (the naive, unrelocated
value Ghidra actually indexed) returned all 27 read/write sites in one call,
both known writers (`InGame_Construct`, `InGame_Destruct`) included. That is
a five-minute lookup in place of the `search_instructions` dance the rest of
this section prescribes. **Confirmed for a `lui`/`lw` data reference; not for
a `jal`** - `get_function_callers("InGame_Construct")` still returned nothing,
and finding its one caller needed `search_instructions` on the zero-padded
relative operand as before. Two data points, not a generalised rule: try the
naive-address xref first for a data reference, but don't expect it to rescue
a call-target search.

**And it does not rescue a `.rodata` string reference either - that is a
third case, and nothing in Ghidra finds it.** Found 2026-09-05 tracing
`%s\ship_eliminator.dat` (`0x08a7ff7c`, `psp-pulse-usa`) for
[`ship-skin.md`](docs/ghidra/functions/psp-pulse-usa/ship-skin.md).
`get_xrefs_to` returns nothing on the relocated address **and** nothing on the
naive `0x0027bf7c`; `search_instructions` returns nothing for `addiu` with
`-0x4084`, nothing for `lui` with `0x8a8`, and `search_byte_patterns` finds no
stored pointer either - yet the reference exists and is an ordinary
`lui`/`addiu` pair. Calibrate before believing an empty result: `%s\Ship.vex`
(`0x08a7ba64`) and `Data\Psys\%s.POB` (`0x08a884dc`), whose consumers are
known, come back just as empty on their naive forms too, so **no path template
in this binary has a working xref** and an empty result proves nothing about
whether a string is used.

**Check the naive address against the section table before you trust it**, and
this one nearly went wrong here: `0x08a884dc - 0x08804000` is `0x002844dc`, and
mis-borrowing one digit gives `0x000844dc`, which is inside `.text`.
`.rodata` runs `0x274000` to `0x2ac5e4` in `psp-pulse-usa`, so a naive address
for a `.rodata` string that lands below `0x274000` means the *subtraction* is
wrong, not that the reference is missing - and it produces a confident empty
result on an address that never could have matched. Same rule for `.data`
(`0x2ac600`) and `.text` (below `0x272a3c`): the naive value has to land in the
section the symbol actually lives in.

**What worked was scanning the ELF outside Ghidra, and it is quick.**
`BOOT.BIN` is a PRX (`e_type` `0xff80`) whose `.text` is at file offset `0x80`,
virtual `0x0`, length `0x272a3c` - read the section headers rather than
assuming, they are plain. Walk it four bytes at a time keeping each `lui`'s
immediate per register; when an `addiu`/`ori` completes a pair, compare the
combined value against `<target> - 0x08804000`. The same walk finds callers:
a `jal`'s target is `(word & 0x03ffffff) << 2`, already in naive form, so
comparing against `<function> - 0x08804000` gives the caller list
`get_function_callers` will not. Both took one short script and seconds to
run, against a long unsuccessful search inside Ghidra - reach for it as soon
as one `get_xrefs_to` comes back empty, not as a last resort.

**`psp-pulse-eu` has the same `jal` wart, not just `psp-pulse-usa`.** Found
2026-08-31 tracing `World_LoadTrack_q` (`0x088835f0`, EU) for
[the M6 lighting thread](handover/rendering/m6-authored-lighting-no-hardware-light-slot-found.md):
its call into `World_CollectMarkerLists` (`0x0887a1c4`) decompiles as
`func_0x000761c4(...)`, and `get_xrefs_to`/`get_function_callers` on
`0x0887a1c4` both return empty despite a live PPSSPP capture already having
confirmed the call executes. `0x000761c4 + 0x08804000 = 0x0887a1c4` exactly -
same fix, second binary. This nudges the still-open "general PSP-ELF trait
vs. this project's importer" question above toward the importer, since both
binaries went through the same procedure; not settled by one data point, but
worth weighing next time it comes up. Full account:
[`lighting.md`](docs/ghidra/functions/psp-pulse-eu/lighting.md#a-fourth-pass-world_loadtracks-own-body-is-now-disassembly-verified-to-hold-no-consumer-and-its-name-is-doubtful).

## Verification status: what to lean on

**The disc chooser was driven end to end on this machine** (2026-08-18), which
is worth writing down because there is no automated way to do it. `just launch`
listed all seven images under `data/images/` with the title, platform and serial
read off each disc; `hdfury-ps3-eu.iso` came up red and unselectable beside the
decrypted twin it shares a serial with; and two Down presses plus Return booted
`pure-psp-eu.chd` through the ordinary loading screen. Whole survey of seven
images: 0.85 s.

**How to repeat it, and the two dead ends.** The input has to go in over
**X11** - `xdotool` cannot reach the window under Wayland at all, so
`unset WAYLAND_DISPLAY` before the run is what makes a scripted press land, and
`xdotool key --window <id>` does not work either; focus the window and use plain
`xdotool key`. **Xvfb is not a way round having a real display here**: the run
starts and the survey prints, but Mesa reports *"vulkan: No DRI3 support
detected - required for presentation"* and nothing is ever drawn or ticked, so
no press is acted on. A headless *capture* is no help either - see the entry in
"Scope decisions" about the chooser having no `--until`.

**The cross-platform gate now runs four layers, not one** (2026-08-15). CI's
`determinism` job runs `oag-core`, `oag-physics`, `oag-gameplay` and the new
`oag-ai` scenario on Linux, Windows and macOS, in release *and* debug. The AI
one exists because the physics gate drives a **scripted** input, so no
controller decision was ever hashed: `oag_ai::Line::curvature` called the
platform's `acos` from the day it was written and nothing failed, because
nothing cross-platform ever ran a driver. It calls `oag_core::math::acos` (our
own libm) now. Measured on this machine before the swap: the two implementations
differ on 8.6 % of the domain by up to one ULP. Full account, including what is
still uncovered, on
[determinism.md](docs/architecture/determinism.md#each-gate-covers-the-layer-below-it-and-one-thing-more).


**Measured against the original running**, all with full method and numbers on
their own pages - lean on these, don't requote them: the forward force law end
to end, lateral grip (0.9985 of the disc value), the yaw accumulator term by
term (99.94 %), the inertia tensor on all three axes, the pitch step response,
the contact friction coefficient, the mag-lock probe, the authored `Start
Position` frame, and the chase camera (RMS 0.008 world units over 150 ticks) -
[`docs/physics/`](docs/physics/README.md),
`crates/game/tests/chase_camera_ground_truth.rs`. The whole-lap result has its
limit in the same breath as its number: over the clean window (first ~170
ticks) position tracks to 6.54 units worst / 2.41 mean, but nothing after tick
256 measures our physics against the current capture, and `grounded` is a
constant column, so agreeing with it says only that our ship also never left
the ground.
[oag-trace.md](docs/tools/oag-trace.md#reseeding-and-why-a-long-comparison-needs-it)
carries both numbers.

**Instruction-level reading with no runtime leg yet.** Mag-lock's blend weights
and its `|h - d| > 5.0` fallback; the swept collision path; everything airborne;
both sideshift gestures. Single-sided wall rejection is measured against the
*data* rather than against the original running: **2,076 of 2,084 wall triangles
on `16_Track` (99.6 %)** are wound toward the nearest point of the track's own
spline, which is what makes reproducing the original's
`dot(boxCentre - sample, n) > 0` rejection faithful rather than merely literal.

**The determinism gate reaches the simulation** (`dd53c1f`).
`oag_physics::probe` steps the same `oag_physics::step` the race loop steps, over
a scripted input and a synthetic corridor built in the file, so it runs in CI on
all three OSes rather than needing a disc image. Two mechanisms keep the hashed
field list from rotting and **both are needed**: exhaustive destructuring makes a
new `ShipState`/`Body` field a compile error, and a perturbation test proves each
field actually reaches the hash rather than merely being bound - that second test
caught a real hole on its first run, since `time_since_landing` defaults to `1.0`
and perturbing it *to* `1.0` was a vacuous no-op. What it does not cover is
stated in its module docs: maglock, reset, and the swept tunnelling path.
`probe::environment` now crosses a speed pad twice, once on each side of the
`<Special speedpad_jump>` tilt threshold, because until it did, step 15 of 15 was
the one force term the gate did not cover at all - and a new branch inside an
uncovered term lands with the hashes *not* moving, which is the worst outcome
available.

**Two standing rules.** When the determinism test fails, find the bug - never
update the reference constants to make it pass. And never raise a confidence
score on the strength of the simulation agreeing with itself.
