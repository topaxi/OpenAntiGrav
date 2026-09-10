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
    `crates/game/src/race/tests/respawn.rs::a_stopped_opponent_is_flagged_after_the_dwell`,
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
  closing frame pair on all three. **Nothing is placed on any title** - slot
  8's transform is unrecovered on Pulse and HD and untried on 2048.
  [start-gantry.md](docs/rendering/start-gantry.md), one page for all four
  titles; `crates/vex/tests/start_gantry_ground_truth.rs` (4/4, Pulse),
  `start_gantry_pure_ground_truth.rs` (2/2, Pure),
  `crates/game/tests/start_gantry_report_pure_ground_truth.rs` (1/1, Pure),
  `crates/hd/tests/start_gantry_hd_ground_truth.rs` (6/6, HD),
  `start_gantry_2048_ground_truth.rs` (9/9, 2048).

**What M5 still wants**: Zone's explosion. Positional audio and the Autopilot
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
touching a wall at tick 256, and *inside* that clean window single-seeded
position error is already 44.1 units by tick 240: the force law, not contact
geometry, is now what limits comparison length. Full numbers, the
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

- [The EU name gap: 115 closed by exact hash, 182 functions (plus data) remain](handover/tooling/the-eu-name-gap-115-closed-by-exact-hash-182-functions-remain.md) - [ADR-0048](docs/architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md) made `psp-pulse-eu` the Ghidra target of record. Measured 2026-09-07 before any transfer: 581 USA-named functions against 284 EU-named, a 297-function gap. **115 closed the same day**, via [`exact-hash-transfer.md`](docs/ghidra/functions/psp-pulse-eu/exact-hash-transfer.md): `get_bulk_function_hashes` on both binaries, matched only where a USA name's normalized opcode hash was unique on both sides, every one of the 115 individually confirmed with `diff_functions` before applying anything - all 115 came back **zero added, zero removed instructions**. Applied live and audited clean: `scripts/audit-ghidra-names.py --binary psp-pulse-eu` reports "399 named function(s) live, 416 row(s) in names.tsv... clean". **182 functions remain** (16 sharing a hash with another USA function, unfixable by hash matching at all; 166 with no exact hash match on the EU side), plus 25 `data`-kind rows this method cannot touch. **The prerequisite the original design worried about - the EU program having no functions analysed - is resolved**: all four PSP databases were reimported with the Allegrex relocation patch the same day, `/psp-pulse-eu/BOOT.BIN` now reads 10,672 functions and 108,729 relocations (zero before), and `get_xrefs_to` works for code and data alike, confirmed directly. What is left needs `bulk_fuzzy_match` at descending thresholds (0.95 -> 0.8 -> 0.6) with a collision filter *before* every individual `diff_functions` check, never renaming off a score alone, plus the structural/positional techniques (address adjacency, callee-diff correspondence, offset interpolation) the 2026-08-05 sweeps already proved out for corroboration.md's own 213 rows. **The failure mode is documented and specific**: `scripts/audit-ghidra-names.py`'s own docstring records a fuzzy sweep on this project that mis-assigned 54 of 64 `_q` names, including `Ship_UpdateCameraRigs` displaying as `Ship_UpdateSideshiftInput_q` - the GE state-cache wrappers (9-22 instructions, structurally identical) are exactly the shape a fuzzy matcher permutes. A transferred name needs its own EU-side evidence page, not a citation of the USA page its match came from - `exact-hash-transfer.md` and `corroboration.md` both already do this, and either is reusable for what remains. Full method, confidence tiers, and the specific do-not-re-run exclusions are in the thread file
- [PPSSPP symbol bridge: export and harvest land, the Ghidra import direction does not](handover/tooling/ppsspp-symbol-bridge-import-direction-not-run.md) - `kotcrab/ghidra-allegrex` ships `PpssppImportSymFile.py`/`PpssppExportSymFile.py` to move symbol names between Ghidra and PPSSPP's Debug menu, at offset `0` (every `names.tsv` header already states image base `0x08804000`). **Export** (`names.tsv` -> `.sym`, offline, no Ghidra/PPSSPP needed): `scripts/export-ppsspp-sym.py`, tested against all four PSP binaries. Size is the gap to the next `names.tsv` row's address, not a naive `0000` - checked live and against PPSSPP's own `SymbolMap.cpp` source, a zero-size entry gets a name only at its exact start address (`AddFunction` always calls `AddLabel`) with **zero function-body grouping** in `memory.disasm` (`GetFunctionStart`'s `start+size > address` check is never true at `size=0`), so a real extent was worth deriving. **Harvest** (what PPSSPP's own analysis already knows, live over its websocket debugger's `hle.func.list`, no GUI, works right after boot): `scripts/harvest-ppsspp-symbols.py`, run against both Pulse pressings and committed as `docs/ghidra/captures/psp-pulse-{usa,eu}/ppsspp-detected.tsv` - a sibling of that directory's Ghidra-capture files, never merged into them (PPSSPP state, not Ghidra state). **The honest headline is 60 (USA) / 63 (EU) real, useful names out of 13,372 functions PPSSPP finds** - of 408/407 named, 335/334 are `zz_` HLE stubs mostly duplicating a live `psp-imports.tsv`, and the ~73 real detections per binary are dominated by libc/newlib and the transcendental math routines `sinf`/`cosf`/`atan2f`/`sqrtf`/`pow`/... that `just check-determinism` polices. **Neither export nor harvest ever feeds `names.tsv`** - PPSSPP's detections have no evidence page, so they carry their own provenance instead. **New, live-verified addition to `ppsspp-debugger.md`**: `hle.func.rename` renames an entry in place correctly (full function-body grouping, not just the label), but **`hle.func.remove` immediately followed by `hle.func.add` at the same address crashes PPSSPP**, reproduced twice independently - a documented trap, not a suggested technique. Full writeup and the offline overwrite-risk table (13 USA / 10 EU addresses where PPSSPP's autodetection collides with a documented name, `skipZun` does not protect any of them since they're not `z_un_` placeholders) in [`ppsspp-symbol-bridge.md`](docs/reverse-engineering/ppsspp-symbol-bridge.md). Open: the import direction itself was not run this session (Ghidra bridge was another lane's) - offset and the fix (re-run `just apply-names` right after import) are both already worked out in the doc
- [Each circuit's airtime budget is measured, and eleven of twenty-four cannot show a barrel roll to anyone](handover/gameplay/each-circuits-airtime-budget-is-now-measured.md) - the AI barrel roll is gated on a minimum airborne duration, and on a full grid only **Ace** ever armed one. The cause is the track, not the propensity: **Moa Therma (`03_Track`) offers a longest airborne window of 11 ticks / 0.18 s** against Ace+`AGGRESSIVE`'s floor of 24 t / 0.40 s - 46 % of the minimum, in either direction, either speed class, any tier. That **agrees with the maintainer's own play report** ("very difficult if possible at all to squeeze in a barrel roll") and rules out a flight-model gap. The lockout is **directional, not a circuit property**: `09_Track` clears 31-36 t while its own reverse `25_Track` clears 103-104 t. Eleven of twenty-four circuit-directions clear nothing at any tier (03/19, 04/20, 07/23, 13/29, 16/32), and `25_Track` is the sole clean Novice candidate. **Nothing was tuned** - relaxing `roll_caution` is one constant across sixteen circuits with wildly different budgets, so it would make Ace roll constantly on a jump-heavy track and still change nothing on a flat one. The table is `crates/game/tests/airtime_budget_ground_truth.rs`; the design decision is the maintainer's
- [A craft hit by a weapon slows down; the port is in, the runtime check is not](handover/gameplay/a-craft-hit-by-a-weapon-does-not-slow-down.md) - reported from play and **missing, not mistuned**. Recovered 2026-09-06 out of the PSP executable and written up on [engine.md](docs/ghidra/functions/psp-pulse-usa/engine.md), then **ported the same day**: a hit accumulates the weapon's `slowdown_time` into `entity+0x130`; once a tick the craft's own update drains that into a timer at `craft+0x2e0` through `Ship_AddSlowdown` (`0x08848690`, 90, fifteen instructions, one caller) which **clamps the running total to `<Global slowdown_limit>`**, so the global is a *ceiling on seconds of slowdown outstanding*, not a speed floor and not a total-slow budget. While that timer is positive the victim gets **no engine thrust and a zeroed throttle state**, **no lateral grip**, and a **hover target height lowered by `min(timer, 4.0)`**; the timer decays by `dt` with no clamp to zero. **A shielded craft takes no slowdown and its pending slot is zeroed anyway**, so a hit one tick before a shield lapses is lost. The port is `oag_physics::slowdown` (the clamp and the timer), `oag_gameplay::slowdown` (the pending slot's one consumer, with the shield gate), `oag_gameplay::projectile::blast` (the credit) and one call from `oag_game::race::Race::tick`. **The trap for anyone reading a row like this one: three of the four effects were already implemented** against the same field under a hypothesised name, `ShipState::leap_timer` - `craft+0x2e0` was believed to be a leap timer and its own doc said so - so the port was a rename plus the two ends nobody had, and a second gate on one original field was one commit away. The decay is **deliberately not clamped to zero** unlike `airbrake::advance_sideshift`'s timers: that function's justification ("every reader gates on `> 0.0`") is false here, because `Ship_AddSlowdown` adds into the field before clamping, so the one-`dt` residue is worth that much less slowdown on the next hit. Measured on the disc in `crates/game/tests/weapon_slowdown_ground_truth.rs`: a Plasma slows a craft for 79 ticks on Talon's Junction in Venom, to 58.6 units/s against an identically seeded control run's 122.4, recovering to 104.1; a second Plasma inside the window leaves the timer exactly where the first did. Open: **nothing is runtime-verified** - every claim is static and a PPSSPP capture of `timer_2e0` through a real hit is the one step left worth taking, with `scripts/psp-trace.py` already carrying the field; the `4.0` clamp is unreachable on the shipped disc and is ported anyway; the subtraction takes *seconds* off a *height* with no conversion and is ported verbatim rather than reconciled; `entity+0x138` (a weapon-type id that does not match the class-name pool's order) is unnamed; five of the nine writers of the pending slot are unidentified (and of the four that are, the Mine/Bomb blast is *inherited* from `mine.md`, resting on a `0x0885bff0` reading this work's own `Ship_Damage` evidence reopens); and PS2 is unchecked
- [Outpost 7 loses 34-35 shield a lap to a corner no craft can hold at the speed the AI picks](handover/gameplay/outpost-7-loses-35-shield-a-lap-to-its-own-walls.md) - one lone Ace craft, `Mode::SingleRace`, `07_Track`, **no rolls armed at all**: it sheds 34.1 then 35.0 shield a lap purely to wall contact and is destroyed on lap 3, the only circuit on the disc measured on that scale. **Steps 1 and 2 are both done as of 2026-09-06.** Step 1: the line is neither into a wall nor above the floor and the collision load agrees with the track file - the *craft* leaves the line, 30.85 units left of centre where the authored left half-width is 27.3, at 101 units/s. Step 2: **the steering is not saturated and the craft is not sliding.** `Tuning::max_turn_rate` (1.8) clamps the rate pure pursuit *requests*; 1.55-1.62 is what the *hull* achieves, and that is `steer * Turning.amount / (5 * I_yy)` = `100 * 1.68 / 108` = 1.556 - within a few per cent of the 1.42-1.51 [engine.md](docs/ghidra/functions/psp-pulse-usa/engine.md) measures on the original. Slip against the craft's own forward is a flat **-8.4 degrees** and `grounded` is **1.00 on every tick**, so the `grip_ground`/`grip_air` split is never crossed. What binds is kinematics: a craft must yaw at `v * k`, so a line of curvature `k` admits at most `1.55 / k`, and 07's local curvature peaks at **0.047 at index 2,119** - radius 21 units, **33 units/s** - while the craft is doing 94. The departure starts within three samples of the speed crossing that limit. **The driver never sees it**: `Line::curvature`'s span is 24 at that speed against a ~50-unit arc, so it reports 0.0155-0.0186 for a 0.047 corner, understating it 2.5-3x - which is also why the `lateral_accel` probe was null, since `v_grip(90)` is 43.8 against a `v_yaw` of 33. **Steps 3 and 4 are done too, and both probes landed.** The yaw-rate term is in `driver::pace::corner_target` as `min(sqrt(lateral_accel * commitment / k), max_turn_rate / k)` - kinematics off recovered constants, worth 51 shield across the twelve and reaching 07 not at all. `Line::curvature`'s seam bug is fixed, so 06 is no longer the confusing row (69.27 shield and zero respawns under the cap, against 56.66 baseline). The span is `Tuning::curvature_span`, **11 units, swept twice** - and the second sweep is the finding: **a cap of 10 is the best solo row and turns two committed *field* tests red**, one craft in a grid of eight wedging near the start and never completing a timed lap, another ground to 0.41 of its pool against a 0.45 floor. Twelve lone-craft circuits cannot see that and `just` does not run the disc-backed suite, so the ordinary gate was green on it; the sweep now reports a field board beside the solo one (`crates/game/tests/ai_span_sweep.rs`) and the trap is in "Traps that are live". Board after: 562.04 -> 705.68 shield retained summed over twelve, all twelve still clean, 01's single respawn pre-existing and unchanged, 13 improved from 2.90 to 6.52 end shield (its ~11 a lap untouched - **still not this bug**), and **07 reaching lap 4 at 28-30 a lap** instead of destroyed on lap 3. That 11 passes the field gate and 10 does not is a property of where one craft wedged, not of the estimator. **Step 5 is done as of 2026-09-06, and it closes the item that ordering was waiting on**: `oag_physics::wall::resolve`'s tangential bleed is **exactly `0.035`**, agreeing with the value [contact-response.md](docs/ghidra/functions/psp-pulse-usa/contact-response.md) recovers off two literals - measured at 100 units/s grazing a flat wall at 28 degrees, one contact a tick, tangential factor `0.965` on every tick at every angle and the *total* loss converging **down** onto `3.5000 %` from `12.84 %` as `v_n` is spent, the same one-sided shape the original's capture has. **Nothing changed, and `curvature_span` does not want re-sweeping on account of the wall.** What stops a craft is the **bounce, not the friction**: with the heading held 28 degrees off, one contact a tick and no thrust at all, 100 units/s becomes **7.34 in 19 ticks** at a flat `12.84 %` a tick, of which `2.72 %` is the friction and `9.82 %` is `-(1 + 0.4) * v_n / D` - **73 % bounce, 27 % friction**. Both are pinned in `crates/physics/src/wall/tests.rs`. The `101 -> 2.7 in 18 ticks` event has its mechanism measured but **no longer reproduces**: on the span-11 tree no `07` contact tick after tick 200 has the craft below 30 units/s, and `14` has **zero contacts anywhere in 740-760** over four laps. Two invented caps retired with it - `dropped_contacts` is `0` on all 24,000 instrumented ticks, so `MAX_HITS_PER_PROBE`/`HULL_CONTACTS` never bind on real geometry - and `FUN_0883e64c` is `Ship_State`, so the original's hull-damage gate is a racing-state test and not a local-player one: opponents do take contact damage there, and `damage::subtract` already reproduces it. The pre-fix probe notes below are kept because they are what the reasoning was built on: **B's damage is two *located* events, not conservative cornering**: 06 loses **34.72 shield in one bucket on one lap** (indices 1,200-1,249, three respawns, where baseline loses nothing) which is its worst corner, `k` 0.0547 at index 1204, `v_yaw` 28 units/s; and 14 loses 7.60 on lap 3 alone at indices 750-799, overlapping the **82 unsupported racing-line samples at 684-765** `race_ground_truth.rs` already records - while 14 is otherwise a large net win under B (29.09 total against 53.59). So the fix is a design call - which limit law and at what resolution - and the sweep cannot be anchored on 06's shield until somebody finds out what happens at **06:1204**. `13_Track` is next thinnest at **2.7** shield and is **not** this bug: it is 12 % yaw-limited but its worst corner still admits 90 units/s. Do not trim `07_Track` from the test's known-good list, and do not move `lateral_accel`, `grip_ground` or `grip_air`. **Step 6, 2026-09-07, answers what "upstream" meant.** Instrumented `Driver::drive` (reverted) over the two remaining crash windows (idx 2,158 and 2,395, both laps): the differential trail-brake's own documented corner-entry gate (`pace::trail`'s `speed < target`) is confirmed live here too, `gated=true` on every sampled tick, but it is inert - `throttle`'s identical `speed <= target` branch never leaves `(1.0, 0.0)` either, so the craft is at full thrust with **zero symmetric brake** all the way into both walls; there is no overspeed for a differential to correct. The actual cause: `corner_target`'s windowed curvature (`max_curvature` at `span = curvature_span = 11`) understates the true local peak (`Line::curvature` at `span = 4`) by **1.66x at the first corner (0.02305 vs 0.01387) and 1.85x at the second (0.02359 vs 0.01273)** - smaller than step 2's 2.5-3x at idx 2,119, same mechanism, not closed by the span-11 compromise chosen in step 4 for unrelated field-test reasons. A further span change to resolve these two apexes without re-breaking the field board is unmeasured and is its own piece of work, not attempted. Full data: `~/.claude/projects/-home-topaxi-projects-OpenAntiGrav/scratch/outpost7-corner-entry.md`
- [The differential airbrake needs a real-corner threshold, not a straight/not-straight one](handover/gameplay/the-differential-airbrake-needs-a-real-corner.md) - **the "AI never airbrakes" report is measured and its cause confirmed**: `driver::trail`'s `speed < target` gate blocks the differential on corner *entry*, not only at corner exit as its own doc comment claimed - measured on Talon's Junction, steering pinned at full lock for over ten ticks while speed collapsed 111->39.5 and `target` stayed at 160-180 throughout, so `differential` was zero on every one of them. `airbrakes()`'s `floor`/`limit` gate, the thread's original suspect, is refuted algebraically - `brake == 0` still lets one side rise from zero. The obvious fix (`!target.is_finite()`) passed `closed_loop.rs` in full, including its stability guard, and still ground one opponent's shield to zero on the real seven-craft `opponent_weapons_ground_truth` suite: a real racing line is never *exactly* collinear, so `target` is never literally infinite on disc geometry, and the replacement gate was inert there - `closed_loop.rs`'s own recoveries only happen on its perfectly-straight synthetic segments, which is why it could not see the regression. Reverted in full; only `trail`'s doc comment kept, describing the finding rather than reverted code. Open: a gate that actually discriminates on real curvature needs fitting against the 12-circuit benchmark, not derived by inspection, and `closed_loop.rs` needs a real-geometry fixture before a green run there means anything about disc behaviour again. Also open: the wall-bumping half of the original report was not reproduced (VENOM/Ace solo, zero respawns) - unclear whether the same fix closes it

- [The race-setup previews are meshes, and nothing draws them](handover/rendering/the-race-setup-previews-are-meshes-and-nothing.md) - the track and craft previews on every race-setup screen are **rendered 3D meshes**, not 2D maps and not prerendered images, on Pulse PSP, Pulse PS2 and HD alike. Verified paths, not inferred ones: `<location>\FE\{forward,reverse}.vex` for a circuit (`VEXX` magic, 10-23 KB against a real `track.vex`'s 4.25 MB, resolving on both pressings) and `<team>\<variant>_FE.vex` for a craft, with HD authoring both as explicit `<Model name="TrackModel">` / `<Model name="ShipModel">` widgets. Recovered by reading each PSP screen class's contiguous `.rodata` string block, since every cross-reference route into those classes is blocked by the unapplied-relocation defect. Split out because it needs nothing from the flow plan - `oag-render` already draws `.vex`. Open: nothing in this build draws either preview; Pure's two are unresolved (its craft candidate rests on string adjacency alone, confidence 45); and the *camera* the PSP titles frame these meshes with is unmeasured, so a framing picked by eye would be ours. See [race-setup.md](docs/formats/race-setup.md)
- [A race box that mixes titles needs four rows off the title, not one flow](handover/frontend/a-race-box-that-mixes-titles-needs-four.md) - the architecture delta against `assets/ui/menu.toml` and [menus.md](docs/architecture/menus.md), whose "the tree is ours, the presentation is the disc's" boundary this does not move. **SPEED CLASS is now a title axis and Pure's fifth rung is raceable** (`oag_title::SpeedClasses`, 2026-09-05): a race carries the rung as the name the disc spells and resolves it against the file that authored it, so `oag_physics::SpeedClass` **stayed at four variants** rather than growing a fifth that would have forced a fabricated entry on Pulse and HD. VECTOR appears exactly when a Pure source is mounted, with no flag - the union is built from the mounted titles' ladders. **VECTOR is confined to RACE REMIX** (`SpeedClasses::is_offered_outside_remix`, 2026-09-05): making the rung selectable at all briefly made a Pure-only boot offer five on the ordinary RACE page too, wider than what was asked for, and the maintainer settled it as "confined to remix" - the ordinary page offers four on every title, RACE REMIX still offers five with Pure mounted, and the engine keeps naming and racing VECTOR either way. Recorded in `docs/architecture/menus.md`'s "SPEED CLASS: `VECTOR` is confined to RACE REMIX" section. **The row shrinking alone was not enough** - `race.class` is one setting shared with RACE REMIX, so a class settled to `"vector"` there could sit in storage while the ordinary RACE page's own row no longer listed it, and `Menu::supply` only ever moves a widget's display index, never the stored setting; the ordinary page's `LaunchRace` handler would have launched the stale `"vector"` while its row displayed `"venom"`. `resolve_race_page_class` (`crates/game/src/main/session/menus.rs`) clamps a stored class the page's row does not offer to the row's own first entry before launch, the same fallback CIRCUIT and TEAM already use. AI difficulty differs in cardinality *and* existence - ours 4, Pulse/PS2/HD 3 (`global="SkillLevel"`, a key that differs from the widget name on all three), Pure none at all. Three rows the disc has and we do not (WEAPONS on/off; ELIMINATIONS, which is a **false friend** - a kill count on Pulse, a disabled *score* target on HD; split-screen player count). Unlock gating is settled (the box gates on nothing; `docs/architecture/menus.md`'s `## Unlocks`). Still open: AI difficulty's count on a title axis, the WEAPONS row, and that the flow must be re-entrant, since Pulse's `Track Creation`/`Team Selection` serve custom race, tournament and the grid editor from one pair of screens
- [Pulse's race box is read on both pressings, and the code side is unblocked](handover/frontend/pulses-race-box-is-read-both-pressings.md) - **2026-09-10: the hexagonal window is a slideshow of the circuit's own stills, authored in a per-circuit `screen.xml` that `TrackDefinition_EnterScreenState` loads - built on both pressings, with the hex grid tiled, the PS2's `0`-suffixed widgets and untextured previews fixed, and the RACE page's TEAM/VARIANT/TRACK rows dropped on a title with these screens.** Open: the cards' transition and the outline's authored `Mode3D` pose, a PS2 capture to read against. Earlier, 2026-09-09: both selection screens are built and drawn off the disc's own XML** (`oag_ui::picker`, `--menu-page track-select`/`ship-select`), measured against a 54-frame scripted PPSSPP walk (`scripts/psp-frontend-capture.py`, [selection-screens.md](docs/ui/selection-screens.md)); the `FE\*.vex` file turned out to be the panel's outline ribbon, not the flythrough, and the rating table is `Definition.xml`'s own `<FE>` element. Open: the flythrough, the distance, loyalty, the skin-vs-variant row, and a played run of the live flow. Earlier: the three-page flow is read straight off the disc: `Single Player` (five lists - Mode, Class, Weapons, Difficulty, Eliminations) -> `Track Creation` -> `Team Selection` -> `Launch Game`, with the PS2 pressing identical on the page itself and diverging only around it (`Racebox` bypassed with the file saying so in its own comments, split screen added, `Team Selection` widgets `0`-suffixed as a player index, `Tournament C` widened from four track slots to twelve). Two unlock axes, not one: circuits gate on `<Unlock Grid="Grid0">` - a **name**, not an index, so an unlock stored as an integer will not round-trip - and craft variants gate on per-team loyalty. Exactly three `PI_Track` entries carry no `<Unlock>` at all, which is why Custom Race offers three circuits. **Both previews are now captured live in PPSSPP (2026-09-05)** rather than read only out of `.rodata`: `Track Creation` is an animated first-person flythrough of the circuit corridor in a hexagonal frame, `Team Selection` a turntable-rotating craft on black, the three-entry track list walked all the way round with no locked circuit ever reachable from that screen, and `Black`/`White` confirmed as the two circuit runs off the screen's own Help text. **2026-09-08: both screen classes are now decompiled, fifteen functions named** ([`race-box-screens.md`](docs/ghidra/functions/psp-pulse-usa/race-box-screens.md)), unblocked by the 2026-09-07 PSP relocation patch - `TrackSelection_PopulateList` closes "what populates the track list" (the `<Unlock>` filter plus a second, mode-gated per-track flag), `TrackSelection_ApplySelection` closes `Top->Ship` (a node inside each track's own dynamically-created preview scene, not a static widget - which is why the live capture's button probe never found it), and `TeamSelection_Update` traces the stat bars and the Eliminator/Normal variant pick to code. One function (`Definition_IsUnlocked`) cross-checked exactly against `psp-pulse-eu`; the other fourteen had no exact-hash match there, front-end code diverging more between pressings than the physics code an earlier EU-transfer pass covered. Open: the nine unlock-predicate functions behind `Definition_IsUnlocked` are still untraced; `GSDisableEntriesBitField` is undecoded; the locked-circuit info panel needs `Tournament C` or campaign progression, neither tried; six fields in the serialised race record (`Locked`, `damage`, `AICount`, `laps`, `ship`, `ShipChoice`) have no authored row; and none of the newly-decompiled functions are runtime-verified
- [Pure splits the race box into a chain, and 2048 has none at all](handover/frontend/pure-splits-the-race-box-into-a-chain.md) - Pure keeps the screen *names* and inverts the *shape*: no `Racebox`, no hex grid, no grid editor, and a linear one-choice-per-screen chain where Pulse puts five rows on one page. **Pure authors a fifth speed class** (Vector), which is a data point for the open `is-there-a-fifth-handling-class` thread and explicitly not an answer to it - Pure having five says nothing about Pulse having five, and [handling-stats.md](docs/formats/handling-stats.md)'s conclusion should not be edited on the strength of it. Pure authors **no AI difficulty anywhere** and **zero unlock machinery** (no `<Unlock>`, no `Grid=`, no `GSDisableEntriesBitField` across all eleven files), and its one authored preview is a **2D stat graph of the speed class** - the only flat-2D preview found in any title. 2048 has no racebox and no track-select screen at all: racing is entered from a code-drawn campaign event grid, which is a property of the title rather than a gap in the measurement. Open: Pure's two previews, its ordering cap of 88 (redirect targets only, never captured), and a localised-path rewrite that hides two of its twelve definition files
- [`RESIDUAL_SHARE` is one circuit, one adapter, and now points at AI/physics cost rather than `hd_bloom`](handover/rendering/drs-residual-share-is-one-circuit-one-adapter.md) - [ADR-0043](docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md) timed `hd_bloom` (the gap ADR-0042 left open) and moved it into `drs::Cost::scalable`; the `dev` overlay's `GPU` row gained a `BLOOM` field and a computed `OTHER` row (frame time minus everything timed) so the residual is visible rather than assumed. Measured: on the calibration circuit `hd_bloom` costs ~0.36 ms and does not explain a multi-ms gap on its own; a full AI grid (`--mode single_race`, the mode every real race runs) leaves ~2.4 ms of a ~8.7 ms frame still unaccounted for, which is why `RESIDUAL_SHARE` rose to `0.20` rather than shrinking. Then [ADR-0044](docs/architecture/adr/0044-the-residual-is-a-learned-upper-bound.md) stopped the constant being the reserve at all: `drs::Residual` opens each session at `RESIDUAL_SHARE * period` and is tightened by frames that can prove a smaller number, gated on the one condition that makes a paced loop's wall clock trustworthy - `reading <= reserve` is the same statement as `scalable >= budget`, a frame with no slack to have slept away - and capped at the constant, so no budget is ever smaller than the one that shipped. Open: nobody has run it on the Steam Deck that motivated it (every number in ADR-0044 is a model, `drs::tests::machines`), and a mis-scaled `TIMESTAMP_QUERY` period is still an unruled-out competing explanation for that report; under vsync at exactly the target rate the learning stops as soon as the controller is comfortable, so closing that gap still needs the CPU-side work actually timed
- [Texture compression only covers BC; ASTC is the unimplemented other half](handover/rendering/texture-compression-only-covers-bc-astc-is-the.md) - `optional_features` requests `TEXTURE_COMPRESSION_BC` only, so every Apple Silicon Mac (Metal, native ASTC, no BC decode) silently takes the CPU-decode fallback meant for the untested GL backend. No disc authors ASTC (standardized after both consoles' GPUs shipped), so this is an encode of this project's own already-faithful RGBA output, not new content. Open: no encoder is wired, no cache location decided, and "pick whichever performs better as the default when both are supported" is unmeasured - needs a real benchmark on hardware that offers both, not a spec-sheet assumption
- [2048's GXP containers decode; the USSE stream's opcode field is located, semantics do not](handover/tooling/2048s-gxp-containers-decode-the-usse-stream.md) - **97,899 of 97,899 Vita shader containers decode** over 22,056 files, header through the named parameter table ([gxp.md](docs/formats/gxp.md), `oag_rcs::gxp`), which unblocks the 2048 half of the shadow plan: `shadowMap` is a **single-channel sampler** at unit 3 beside `lightmap`, so the track shadow is a one-channel texture and not a projected depth map. **2026-09-04**: `scripts/vita-gxp.py opcodes` swept both regions' full corpus (323,937 programs, 19.3M instructions) and found the op1 field is a **variable-width prefix**, not a fixed one - 8 of 9 dominant 5-bit groups close cleanly but the largest (10.1% of primary instructions) keeps splitting past 8 bits - plus a corpus-wide (247,049 of 247,049) secondary-code end bit at bit 50, holding even excluding single-instruction blocks. Checked against Vita3K's own GPL-licensed USSE decoder as an oracle only, never transcribed, per this project's license bar. (A same-day advisor check caught a first-pass overclaim that 5 bits closed the field outright - a monotone coverage statistic hid a real 2:1 split; corrected before landing.) Still open: no opcode is *named* - the field's position is found, its meaning is not - and which of `eboot.elf`'s 67 programs carries which `_vp`/`_fp` name (counts and the 32/35 split match, the ordinal correspondence breaks around the twentieth, and the SCE relocation segment that would settle it is unparsed)
- [Shadows are planned as four techniques, and the `Dynamic Shadow Occluder` hull is decoded and read at runtime](handover/rendering/shadows-are-planned-and-pulses-occluder-payload-closes.md) - RE only; the implementation half now has its own thread, below. `0x3c3` closes at `0x50 + 32n + 16m` on 129/129 Pulse nodes and 6/6 on 2048, and both arrays are read: `n` planes, `m` vertices. **2026-09-03**: the runtime reader and its class registration are both found and statically linked - `Shadow_RenderOccluderVolume` (`0x089038c8`, confidence 84) reads exactly those fields, derives its shadow direction from the occluder's own local axis rather than any light, and turns out to be the same function that renders the craft's own drop shadow; `DynamicShadowOccluder_RegisterClass` (`0x0890446c`, confidence 84) registers class `0x3c3` and installs the method table naming it, byte for byte. Step 4 of the plan is closed. **A same-day trap worth carrying**: a first pass at the padded-bbox field (below `+0x30`) generalized a rule from the 32 mismatching nodes alone and was falsified the moment it was checked against the matching 97 - corrected, with exact counts, not a clean rule. Also here: 2048 replaced HD's `Ile` bake with `Nova` and relit all thirteen ported circuits; 2048 authors a shadow direction per circuit in plain text; and two traps - `mesh_batches(..).is_ok()` proves nothing, and a repeated string count in an executable can be a linker artifact. **2026-09-04**: HD's own `LiveStencilShadow`/`shadow.stencilvolume` path is traced too - a real registered shader technique (`Shader_ResolveLiveStencilShadowConstants` et al., confidence 82, all five constant/shader names individually TOC-resolved rather than trusted from Ghidra) with a named `lightDirection` constant Pulse's equivalent has none of, and a per-model flag bit that builds a path ending in the fixed leaf `shadow.stencilvolume` and loads it (`Resource_FindOrLoadByHashedPath`, deliberately not `Shadow_`-prefixed since nothing in it is shadow-specific) into an explicit vertex-buffer/index-buffer pair plus a computed bbox - the same coarse shape as Pulse's `.vex` payload but a genuinely different topology encoding. **The draw call is found too, same day**: `Shadow_DrawOccluderStencilVolumes` runs a colour-mask-bracketed two-sided depth-fail stencil test (later found to be over a rigidly-shifted box proxy, not a true silhouette volume - see below) (`Shadow_AccumulateStencilVolume`, then a generic scene-chunk submit whose geometry is unconfirmed, then `Shadow_ClearStencilVolume` - a stencil reset with no colour output, not a "resolve"), every RSX register identity (`0x304`/`0x310`/`0x324`/`0x328`/`0x348`/`0x183c`, the func/op enum values) cross-checked against a local `rpcs3`'s own `gcm_enums.h` rather than assumed. **And the record format itself closed the same day**: `Shadow_AllocateStencilVolumeBuffers` allocates an `n`-vertex (24-byte stride) buffer and an `m`-index buffer, and its index-width computation for the literal `param_2=0` this call site passes comes out to exactly `4` bytes - matching `Shadow_ParseStencilVolumeGeometry`'s own `m<<2`-byte `memcpy` without either function citing the other, an exact arithmetic invariant between two independently-read functions (confidence 82, up from the 50 an earlier same-day draft gave "an unrelated word-blob"). See `shadow-stencilvolume.md`. Four wrong first-pass reads caught before being written down as findings, all in the same investigation: unrelated strings that only lined up in decompiled-text order, not instruction order; a data xref that looked like a shared vtable slot but is an ordinary PPC64 OPD descriptor every function has; a middle draw call guessed as "the caster's own mesh" from parameter shape alone without checking; and a primitive-type citation (`TRIANGLE_STRIP`) that named the wrong GCM enum because GCM's primitive list is 1-based, not OpenGL's 0-based one. **All 39 real `shadow.stencilvolume` files found and decoded, same day**: `scripts/psarc.py` against all seven PSARC archives turns up one entry per `data/ships/<name>/`, every one matching an identical `(n=24, m=108)` header and byte-verifying the record format (confidence 82 -> 92) - a fixed, unwelded six-face box, only vertex positions varying per ship. This forced one more correction: the min/max fields at `param_1+0x20`/`+0x30`, previously called a "bounding box", actually reduce over the vertex *normal* (not position) per this closed layout, so "bbox" is retracted. **The vertex shader is disassembled too, same day**: `LiveStencilShadow_vp` (`scripts/ps3-microcode.py`, resolved through `ShaderRegistry_Register`'s own arguments to `SHO` block `0x00929600`) applies a *uniform* `lightDirection * extrusionDistance` shift to every vertex regardless of facing - the per-vertex normal is dotted with `lightDirection` but that result is never read again - so this is a rigid-body shift of the sealed box, not a per-vertex silhouette extrusion the way Pulse's runtime reader builds one. All three parameter names are confirmed by exact hash preimage (confidence 84). **Checked directly against both draw functions, same day**: neither `Shadow_AccumulateStencilVolume` nor `Shadow_ClearStencilVolume` re-uploads a different `extrusionDistance` between passes, so there is no near-cap/far-cap pair either - both draws submit the same, once-shifted box, meaning HD's technique is closer to "does this pixel's depth sample fall inside a fixed box template, shifted toward the light" than to a true silhouette-derived shadow volume. One loose thread: `data/ships/detonator/`'s box is ship-scale despite `detonator-bomb.md` naming a `DetonatorBomb` weapon class - unresolved.
- [All four shadow tiers draw; `mapped` has one named gap and 2048 has none of it](handover/rendering/original-on-pulse-is-the-tier-still-unbuilt.md) - the implementation half of the thread above, and **steps 2 and 3 of [shadows.md](docs/rendering/shadows.md)'s plan landed 2026-09-04**: `graphics.shadows` (`off`/`blob`, per title, with a GRAPHICS row and a `--shadows` override) and the `blob` tier itself - a ground-aligned quad per craft, cast along the craft's own down axis against the circuit's collision geometry and filtered to `Floor`/`MagFloor` (a bare `raycast` puts the shadow up the barrier a craft is scraping). HD's own nine `ambient_shadow.gtf` are the silhouette where a source ships them, with the **coverage polarity measured** - corner texel 0, outline to 212 of 255, and the `remap` broadcasts it into red with alpha forced opaque, so reading `.a` draws a full-strength rectangle - and a generated falloff elsewhere, reported per slot. **Diffed `off` against `blob` on four titles** rather than eyeballed: Pulse PSP 8,595 pixels changed at a worst delta of 43, Pulse PS2 4,637 at 75, Pure 16,924 at 166, HD/Fury 25,022 at 77 - though every judgement about how it *looks* is still from a headless capture. **A trap worth the reading time**: the first fade faded from the ground, so a craft resting at its own ride height drew at strength `0.030` - 5,262 pixels changed by a maximum of 2, which reads exactly like a pass that never ran; an `off`-versus-`blob` PNG diff at threshold **zero** is what told them apart. **Step 5 landed the same day**: `original` draws the craft's own `Dynamic Shadow Occluder` hull, silhouette taken in hull space against `oag_pulse::shadow::AUTHORED_AXIS` - `(1, -10, 2)` normalized, carried by all three Pulse builds and neither Pure one - and projected onto the surface its own cast found; `blob` against `original` changes 5,720 pixels at a worst delta of 126, which is what says the two tiers draw different things. **HD's `original` landed the same day and is a different mechanism**: the craft render into a *coverage* map - `TXP R1.x, f[TC0] unit2` then `ADD H0.w, -R1.xxxx, {1}`, nothing compared anywhere, so no depth attachment and no comparison sampler - and the track samples it projectively, 1,289 pixels against `off` at a worst delta of 58. Proved inert where it should be: Pulse's `original` is byte-identical before and after. **`mapped` landed the same day** - a depth map everything casts into and every lit surface reads, this project's own, one cascade - and found two bugs that shadowed *something* and so read as working: the lookup was mirrored vertically (glam's `rh::proj::directx` projections are **Y-down**, and HD's coverage tier had the same bug hidden by a caster near its map's centre), and the shadow term reached one fragment entry point of five, where a race draws through the `_velocity` pair. Open: a craft's own shadow does not appear on the road under `mapped` - five things ruled out by separate runs and the caster shader's missing node transform named as the first thing to check; `original` on 2048 (needs 2048 to race); HD's separate `LiveStencilShadow` path; and three darkness constants that are ours because the compositing pass is unread, nothing shadows anything but the road, nobody has seen either tier in a window, and HD's own `ambientShadowBlendFactor` has never been read
- [Dynamic resolution wants a viewport, not an allocation](handover/rendering/dynamic-resolution-wants-a-viewport-not-an-allocation.md) - **landed 2026-09-02 and the feature works, off by default**; the file survives only for what it did not answer. `[render_profiles.<title>] target_fps` names a target rate and `minimum_resolution` bounds the fall, `render_scale` is the ceiling, every scene pass takes a resource size and a viewport separately, and the budget is a share of the target frame period rather than anything off the wall clock ([ADR-0040](docs/architecture/adr/0040-the-dynamic-resolution-budget-is-a-share-of-a-frame.md)). **`render_scale` being a ceiling makes every warning that read it as the drawn size wrong** - three on the graphics page were, and each is now two entries. Still open: what each title's defaults should be, the Steam Deck memory reading, `SCENE_SHARE` being a choice rather than a measurement, and whether the scale stepping is visible to a player
- [FSR 3.1 is ported, wired and selectable; nobody has played it](handover/rendering/fsr-3-1-is-a-seven-pass-port-and.md) - **all eight passes, wired through `upscale::Framebuffer::resolve_scene` and reachable from the UPSCALER row**, off by default. Captured once and never played: the frame is clean and sits between bilinear and FSR 1 in sharpness, which is not yet known to be right. The spine is [fsr3.md](docs/rendering/fsr3.md); the reference is pinned to FidelityFX-SDK `v1.1.4` (`just fsr-reference`, nothing vendored). Findings worth the reading time: `oag_render::jitter::offset_pixels` was already `ffxFsr3UpscalerGetJitterOffset` line for line; the port needs **no `wgpu::Features` bit at all**, which is not what ADR-0012 expected; `accumulation` and `luma_history` are **render**-sized despite their names, and getting that wrong produces a blurry picture with no error anywhere; and `SceneAverageLuma` and `SampleInputDepth` are declared in v1.1.4 and called from nowhere. Also fixed on the way, and pre-existing: **a headless `--race` capture discarded `--render-scale`, `--upscaler`, `--anti-aliasing` and `--motion-blur` entirely**, because `main.rs` applies them by walking `settings.render_profiles` and that path built a `RenderProfile::default()` instead - so `just compare-upscalers` had been producing three byte-identical images while reporting nothing wrong. Open: accumulation runs in gamma rather than linear per ADR-0020 and the cost is unmeasured, and a camera cut does not reset the history. **A full review on 2026-09-03 found nine more and all nine are fixed**: the chain is timed now (its own `PassTimer` ring, not a share of the scene pass's, reported on the `dev` overlay as `GPU SCENE x.xx MS  FSR3 x.xx MS` - which incidentally gave `Session::scene_cost` the reader its doc had been claiming); **FXAA/SMAA is skipped rather than run and discarded** when FSR 3.1 resolves, which needed the resolve encoded *above* the anti-aliasing match so the skip keys on what actually ran; the ~15 per-frame bind groups are now two cached parities behind `perfprobe::bind_group`, since nothing in them is per-frame; `accumulate.wgsl`'s history sample went from 40 `sin` per presentation pixel to 16 and from 20 taps to 16, both arithmetically identical and both recorded as deviations because this port's premise is diffability against upstream; and `Fsr3::sizes` finally has a caller - **61.3 MiB measured at 1440x816/50 %** on a real headless run, logged once per allocation. That is a different size from the 93.8-against-upstream's-70.2 the review computed at 1080p/50 %, not a confirmation of it - the two agree by area, and only the first is a reading. A cache hit and a rebuild produce identical pixels, as do a discarded AA pass and a skipped one, so both needed an observable built for them (`Fsr3::group_rebuilds`, `Framebuffer::built_spatial_anti_aliasing`) and both have a test. **Still unmeasured**: the timer is wired and nobody has read it, on this machine or a Steam Deck
- [HD's pads author an emissive mask and a material of their own; nothing reads either](handover/rendering/hd-pads-author-an-emissive-mask-nothing-reads.md) - the invented pad tint is gone and a pad now draws the colour its own texture paints on every title ([pads.md](docs/rendering/pads.md)); this is the half that fix did not close. **What binds the `_ne` file is now answered, asset-side, 2026-09-07**: the pad material is an inline `.rcsmodel` record, not a separate file, and it genuinely names `ds_speedup_ne.gtf`/`ds_weaponup_ne.gtf` as its own `second_texture` - but the record carries a *third* sampler entry (the lightmap), and `skin::picks`'s "the lightmap wins the second binding, wherever it sits" rule grabs that instead, so the `_ne` entry is never bound. Measured off the actual shader too: the `_ne` sampler binds fragment unit 1 and `Program::accumulates(1)` is `true` on 18 of 18 pad chunks on `12_sol_2`, unconditionally - no parameter patch gates it. `speedup_material.rcsmaterial` turned out to be an authored orphan on this circuit; the `Speedup Pad` chunks name a generic material instead. **Wiring it is not cheap**: the renderer has room for only one second texture per material, so the naive fix trades the lightmap away for the glow - a near-black regression the existing pixel-count guard would not catch, since an added glow layer adds lit pixels even while losing more of them to the dropped lightmap. A real fix needs a third texture slot reaching every model's bind group. **Maintainer report from play, 2026-09-07, more specific than before**: HD weapon pads light up red, go dark on grant, relight on refresh - only the light bars, not the whole pad. **Checked asset-side the same day, and it is a negative result**: the `_ne` file's RGB is a tangent-space normal map (whole-texture mean `[127,129,243]`/`[127,128,244]`, confirmed by the shader's own `x*2-1` normal-decode right after the sample), not paint, and neither pad's fragment program carries a tint anywhere - baked or parameter-driven. So nothing measured explains "red" for the light bars; that hypothesis is closed, not open. **Which alpha channel gates the accumulate is now answered too, same day**: `_ne`'s own alpha, not the diffuse's - traced with the fragment program's own swizzles and write masks, which the probe now prints. `_ne`'s raw alpha (untouched from its `TEX` sample to instruction 41/43) gates an additive term identically on both pad classes; the diffuse's alpha only multiplies that already-accumulated sum one step later. Neither channel runs the renderer's generic `mesh.wgsl` shape at all - the pad's own program runs two power-curve chains (one against the lightmap sample directly, one a genuine `N·H` specular exponent) as well as `_cs` and `_ne`; the "second texture" those curves read is unit 2, which is the material's own already-known lightmap (`LIGHTMAP_SAMPLER`), already bound today via `skin::picks` - not a fourth, unbound texture, so the slot count stays three. `Program::output_texels()` independently agrees with the hand trace on both programs' output-alpha lane, and a `Weapon Pad`-only register-aliasing caveat (a `TEX H0` sits between `R0`'s write and its use, unlike `Speedup Pad`'s `TEX H5`) is recorded rather than glossed over. The colour patched into the `_ne`-gated term is now read too: `[0.0, 0.768628, 0.992157, 0.0]`, a light cyan/blue, identical between the `Speedup Pad` and `Weapon Pad` material files - not red, and shared rather than pad-specific, which reads as a circuit rim tint. What is still open is where the red actually comes from, which needs the Ghidra-side vtable diff this thread's Next Steps already named, not another asset probe - and wiring anything is now known to need reproducing that arithmetic, not just a third texture slot, since the disc's own formula is not the generic emissive shape either
- [The frame path allocated 1.6 MB a frame; five reasons are fixed and the rest are counted](handover/rendering/the-frame-path-allocated-16-mb-a-frame.md)
- [Race records persist across a restart, and the results table now shows one](handover/frontend/race-records-persist-and-need-an-on-screen-surface.md) - best lap, best total time, the last result and (unfed, still) a campaign medal survive a restart (`crates/game/src/records.rs`, `<config dir>/oag/records.toml`), captured outside `Race::tick` at two sites - `Session::frame`'s finish transition and `Session::escape`. `crate::scoreboard`'s results table now draws a "PERSONAL BEST LAP"/"BEST MEDAL" line off `records::PersonalBest::compare`, verified live off a real disc, three ways (no record, a beaten record, a standing record plus medal). Schema is chosen, not measured, carries no confidence score: [ADR-0049](docs/architecture/adr/0049-race-records-are-a-chosen-schema-captured-outside-the-tick.md), [persistence.md](docs/architecture/persistence.md). Open: no records browser yet, no wiring from a real race to a campaign cell (so the medal line never draws outside a hand-seeded demo file), and `Session::escape`'s own capture site is still not runtime-verified on a real desktop
- [Invented UI text has no translation - the mechanism and every row's `string_id` are wired; French is the first real translation](handover/frontend/invented-ui-text-has-no-translation-and-the.md) - the mechanism landed: `oag_game::strings` merges a project-owned `assets/ui/strings/<language>.toml` over a `StringTable`, project entries winning, either onto the disc's own (`boot::load_strings`) or, for a caller with no disc open yet, a project-only one (`strings::project_table`). Every `menu.toml` row/page now names a `string_id`/`title_string_id` (`just check-strings` reports 0 baselined either way), and `assets/ui/strings/french.toml` is the first second-language file: 86 of 109 ids translated, 23 honestly named under its own `[untranslated]` rather than guessed at, proved on screen with headless `--menu-page options`/`--menu-page audio` screenshots against `pulse-psp-usa.chd` (the EU disc ships no English plugin at all, a real disc-content gotcha named in the thread). Still open: `english.toml` duplicates the `hints.rs`/`wording.rs` literals by hand rather than being their one source of truth, and only one of the four other languages the disc offers (German, Spanish, Italian, ...) has a project translation yet
- [HD's sprite flare reads oversized against the original, and the tuning file's other 19 rows are read](handover/rendering/hds-sprite-flare-reads-oversized-and-the-tuning.md)
- [HD's glow draws, and the coordinate it samples on is the one thing not read](handover/rendering/hds-glow-draws-and-the-coordinate-it-samples.md)
- [2048's four Vita eboots are decrypted and imported; loose ends from getting there](handover/tooling/2048s-vita-eboots-are-imported-re-not-started.md)
- [Race start: the countdown state machine, the launch reaction boost, and whether Zone shows a different countdown](handover/gameplay/race-start-countdown-and-launch-boost.md) - **The gantry-placement blocker's own reframed next step is resolved, with a negative-for-placement result: `Billboard_ConstructResource_q`'s type search at `param_1+0x40` finds `321Go_StartFinish.vex`'s own authored `Camera` node (class id `0xf7`, confirmed against this project's ground-truthed Maya class-ID table and a `Vex_RegisterClass` call site), not a generic placement mount point.** This retires the prior pass's leading candidate rather than confirming it - a Camera node is a poor fit for carrying a circuit's own gantry coordinates. Five functions renamed (`Camera_GetTypeId`/`Construct`/`AttachChild`, `Camera_RegisterClass`, `GridCamera_RegisterClass`), and a wrong-turn Transform/Locator reading this same pass first reached for is retracted in the doc rather than left standing. **No static lead remains untried on slot 8's transform**; the concrete next step is a live breakpoint on the child-list insert (`func_0x00140bd4`) during an actual track load. Full account in `docs/ghidra/functions/psp-pulse-usa/billboards.md`. Unrelated to this pass: `ReadyGo`'s own gate is closed, negative (see prior entry, still standing) - `Hud_UpdateCountdownFade` gives it a real code-supplied fade envelope gated on a byte pair with no recovered writer, so it stays undrawn rather than shown without its envelope
- [2048's PSARC and title plumbing are wiring-ready; two binary formats changed underneath a track](handover/tooling/2048s-track-vex-parses-but-two-binary-formats-changed.md)
- [Streaming decode for audio would break seek, and nothing forces the change yet](handover/audio/streaming-decode-for-audio-would-break-seek-and.md)
- [HD's cues end loud and were cut dead, and its `.COLLISIONS` is a flattened tree](handover/audio/hd-collisions-flatten-a-severity-tree.md)
- [The audio queue is ours now, and the frame stalls are what is left](handover/audio/the-device-queue-is-sized-off-the-configured-cap.md)
- [A reported skip is not in the mix we render, and the jump counter cannot see it](handover/audio/a-reported-skip-is-not-in-the-mix-we-render.md) - the PS-ADPCM run-out block was played once per loop and is now trimmed; the reporter can no longer hear the skip, though it was never reproduced under an instrument - `--tap-audio` from a windowed run is what would settle it if it returns
- [A circuit's billboard slots are a 9-entry array on the engine side, and slot 7 is not what it says it is](handover/rendering/a-circuits-billboard-slots-are-a-9-entry.md)
- [A parser cannot fail on a field it does not know about, so coverage is now measured](handover/tooling/a-parser-cannot-fail-on-a-field-it.md)
- [A chunk header is 0x20 bytes and then a *surface* record, and the byte after the layout says which space it is in](handover/tooling/a-chunk-header-is-0x20-bytes-and-then.md)
- [HD ships a per-chunk PVS, and drawing every chunk was the whole "meshes that should not be there"](handover/rendering/hd-ships-a-per-chunk-pvs-and-drawing.md)
- [HD's loading screen is recovered from its own constructor; three threads left](handover/frontend/hds-loading-screen-is-recovered-from-its-own.md)
- [The race mix saturates; the per-voice SAS volume would settle whether it should](handover/audio/the-race-mix-saturates-the-per-voice-sas.md)
- [The Autopilot pickup is built off its own two handlers; three of its parts are still ours](handover/gameplay/the-autopilot-pickup-is-built-off-its-own.md)
- [Positional audio is recovered whole, and the pan is a table the disc computes from `cos` and `sin`](handover/audio/positional-audio-is-recovered-whole-and-the-pan.md)
- [A circuit's sound emitters parse, and nothing plays them](handover/audio/a-circuits-sound-emitters-parse-and-nothing.md) - **both `sound` and `soundcone` are audible now, and both without an emulator.** `oag_game::audio::sfx::TrackEmitters` opens a held voice for every in-range `sound`/`soundcone` node, off `oag_audio::spatial::Emitter::cone`'s law. 2026-09-09 closed the two open reversals: `VexSoundCone_Init` (`0x08925ff4`, confidence 90) settles which of a cone's two authored angles is its half-angle, and tracing it found that `VexSound_Update`'s radius curve has exactly two static call sites, both gated on a byte that is `0` on all 1,298 authored nodes - so the curve never actually runs and `+0x10` is the one radius any node ever plays, which is also why a cone's own curve key reads `0` and is safe to ignore. Pure's own `woSound` class (`0x393`) was found the same pass, without the Ghidra bridge the thread expected it to need: a scene node's own class ID and name are enough, no exporter table required. Earlier finds still stand: PS2 shares Pulse's class IDs and counts exactly; a circuit's own sound bank is named by its `trackstartup.xml` and lives beside it in the circuit directory; opcode `0x14` is a decoded no-op, corroborated on `ps3-hdfury-eu`; the mix's clip rate is down sharply since `Scream_PanVolumePair` landed, and no limiter was invented to chase the rest. Open: a cone in radius but outside its angle still holds a silent voice rather than being budgeted away, unmeasured past two circuits; Pure's own payload field layout past the class ID.
- [HD's exhaust is measured from the running game now: the trail is a 54-sample three-fin tube, the flame breathes with the throttle, and the plume scales rather than blinks](handover/rendering/hds-exhaust-is-measured-from-the-running-game.md)
- [HD's engine trail is one of four ribbons, and a craft flying through one sparks](handover/rendering/hds-engine-trail-is-one-of-four-ribbons.md)
- [The PS2 boost plume draws, and the PS2 file is not the PSP file wearing the same name](handover/rendering/the-ps2-boost-plume-draws-and-the-ps2.md)
- [HD's frame was too bright and too bloomy; the bloom chain was not what was wrong](handover/rendering/hds-frame-was-too-bright-and-too-bloomy.md)
- [HD needs a per-material shader path, and two general rules for finding one have been refuted](handover/rendering/hd-needs-a-per-material-shader-path-and.md)
- [A PS3 title now boots, drives and screenshots from a script - and its savestates are not worth adopting](handover/tooling/a-ps3-title-now-boots-drives-and-screenshots.md)
- [Reversed grids were putting craft off the track; fixed by walking the spline instead of extrapolating a straight line](handover/gameplay/reversed-grids-were-putting-craft-off-the-track.md)
- [HD's sky, fog and lighting draw from the disc's own statements - three holds remain, each named where it lives](handover/rendering/hds-sky-fog-and-lighting-draw-from-the.md)
- [A cue can play other cues, and that is what `.COLLISIONS` does on HD](handover/audio/a-cue-can-play-other-cues-and-that.md)
- [Every SFX trigger is a Pulse reading, applied to Pure and HD on a bet](handover/audio/every-sfx-trigger-is-a-pulse-reading-applied.md)
- [HD/Fury's particle effects parse, and a "field that disagrees" turned out to be a wrong check](handover/rendering/hd-furys-particle-effects-parse-and-a-field.md)
- [Pulse's draw order is recovered, and it is a layer sort rather than a depth sort](handover/rendering/pulses-draw-order-is-recovered-and-it-is.md)
- [Talon's Junction's "missing floor" is a glass floor drawn with the wrong texture: rendered, not absent](handover/rendering/talons-junctions-missing-floor-is-a-glass-floor.md)
- [`viewProj` is located; the capture harness still reports the wrong camera](handover/rendering/viewproj-is-located-the-capture-harness-still.md)
- [`Material::state`'s upper bits: bit 7 is measured, mode 2 is now fully decoded](handover/rendering/material-states-upper-bits-bit-7-is-measured-mode.md)
- [Is HD's language picker ever *shown*, or only entered?](handover/frontend/is-hds-language-picker-ever-shown-to-a-player.md)
- [HD's menus are drawn inside its own frame, and the `HD_*` palette turned out to be the FE style](handover/frontend/hds-menus-are-drawn-inside-its-own-frame.md)
- [HD's strip widget draws bare text; the real menu also draws a tab shape, an underline, and a background scene - the tab now draws its measured corner, the scene is the open half](handover/frontend/hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md)
- [Wipeout HD's video decodes, and the logo reel is the Studio Liverpool ident](handover/rendering/wipeout-hds-video-decodes-and-the-logo-reel.md)
- [Wipeout HD's soundtrack plays; finding it fixed a disc-wide PSARC bug](handover/audio/wipeout-hds-soundtrack-plays-finding-it-fixed-a.md)
- [Pure's soundtrack plays, and both titles' track names are recovered - Pulse's are deliberately unwired](handover/audio/pures-soundtrack-plays-and-both-titles-track-names.md)
- [HD/Fury's HUD draws, and three of Pulse's constants applied to every title were why it did not](handover/frontend/hd-furys-hud-draws-and-three-of-pulses.md)
- [Wipeout HD / Fury's HUD reads, and the reader it shares with Pulse was wrong in three ways Pulse's own data could never have shown](handover/frontend/wipeout-hd-furys-hud-reads-and-the-reader.md)
- [`.gtf` reads, so HD's textures are pixels - all 7,333 decode, and the ambient shadows they include are now drawn too](handover/rendering/gtf-reads-so-hds-textures-are-pixels.md)
- [Wipeout Pure races, on Pulse's physics, and what is still absent is a list of unrecovered entry names](handover/gameplay/wipeout-pure-races-on-pulses-physics-and-what.md) - **the entry-name list closed 2026-09-06** and both remaining rows were findings, not spellings. The Zone hull was never unrecovered after 2026-08-19; `crates/pure/src/race::ZONE_TEAM` had spelled it `Zone_01` for three weeks and the thread lagged the code (re-verified against the disc's own `<PI_Team type="Zone">`; no change needed and none made). **Pure ships no boost-plume asset at all** - confidence 93 from four probes across three sources, each with its Pulse positive control beside it: no `%s\%sboost.vex`-shaped template (Pulse's is at `0x08a84ccc`; Pure's whole executable holds two `boost` strings and neither is a path), no archive entry under any of the eleven declared `<PI_Team>` locations on either pressing, and no `boost_flare` anchor in any Pure hull where every Pulse hull carries two. `race::ships`' old "Pure certainly has one" is retracted. **Four per-craft model names came out of the same block and nobody was looking for them**: `Shipwreck`, `Phantom`, `Phantom_shipwreck` and `VR\Ship`, contiguous with `Ship` at `0x08a7a5d4`..`0x08a7a61c`, confidence 96 and self-certifying (each blob carries its own `Z:/Data/Ships/<Team>/<Leaf>.mb` back). **`Phantom` is the top speed-class rung getting its own model**, which **Pulse** does not do - its template set is mode-keyed, not class-keyed; HD and 2048 unchecked, so that is a two-title comparison. Eight core racing teams only, not `Medievil`/`Zone`/`Zone_01`. Pinned by `crates/pure/tests/ship_models_ground_truth.rs` (3/3, both discs); evidence on [ship-models.md](docs/ghidra/functions/psp-pure-usa/ship-models.md). Open: what *selects* `Phantom.vex`, what `VR` is, whether Pure's boost is visually inert (the flare billboard is present and its draw path unread), and a `WO_SHIP_ENGINEFLARE.POB` named in Feisar's hull that resolves to nothing while all 26 `WO_*.POB` literals in the binary do
- [`boost_entry_name` composes Pulse's spelling on every title, and Pure has no plume to compose](handover/rendering/pure-boost-entry-name-is-pulses-on-every-title.md) - the `crates/game` half of the finding above, split out because the recovery was `crates/pure`. `race::assets::boost_entry_name` hands back `oag_pulse::race::ships::BOOST` unconditionally, so every Pure race reports a missing boost model for a file that does not exist - a standing false alarm now that the absence is measured. Wants the title axis the hull already has (`ShipPaths`), and a report that still distinguishes "this title ships none" from "this title ships one and it did not resolve". **Must not decide** whether Pure's boost is visually inert: drawing nothing extra is the honest state, and an invented intensification is exactly the stand-in the project rule forbids
- [Pure's particle effects decode and play, and two of five names do not resolve](handover/rendering/pures-particle-effects-decode-and-play-and-two.md)
- [Per-team hulls are drawn; which team flies which slot is not recovered](handover/rendering/per-team-hulls-are-drawn-which-team-flies.md)
- [A race flies a declared ship skin; what *selects* one is chosen, not measured](handover/rendering/a-race-flies-a-skin-what-selects-one-is-chosen.md) - the parser, the applier, `catalogue::Team::skins` and `--skin` are all in and verified on `pulse-psp-usa.chd` (100% of the player's texels repaint, every opponent's byte-identical). **Which skin a race flies is chosen, not measured, and no unlock is checked** - the original selects `ship_eliminator.dat` on a global equal to `0x12`, confidence 55 on what that number denotes, so a mode gate on it would be a guess dressed as a reproduction. Settled by `scripts/psp-relocate.py resolve` on the comparison or by tracing the six non-lobby callers of `Skin_ApplyToModel`; both need the bridge. Still open: the composed fourth slot (new asset-side corroboration for the linear layout, not proof), `zone01.vex`, the `%s\%s.dat` fallback, and a RACE-page SKIN row that needs its own `string_id`
- [The particle effects are played from the disc, and most of them have no recovered trigger](handover/rendering/the-particle-effects-are-played-from-the-disc.md)
- [The PS2's engine flare plays and reads as nothing on screen](handover/rendering/the-ps2s-engine-flare-plays-and-reads-as.md)
- [The race camera's FOV is fitted to the PSP's own aspect on every title, and the PS2's own in-game widening option has no counterpart](handover/rendering/the-race-cameras-fov-is-fitted-to-the-psps.md)
- [The AI's six-stage plan (line-follower to a field of pilots) is complete; what remains unbuilt is the residual](handover/gameplay/the-ais-six-stage-plan-line-follower-to.md)
- [The AI was never the problem: four contact-path bugs, and what is left after them](handover/gameplay/the-ai-was-never-the-problem-four-contact.md)
- [DLC packs are mounted; two things inside them are not read](handover/tooling/dlc-packs-are-mounted-two-things-inside-them.md)
- [Two recovered chase-camera behaviours are ported; `headtilt` is not](handover/rendering/two-recovered-chase-camera-behaviours-are-ported-headtilt.md)
- [The menus draw the disc's layout; the footer's ticker and prompts are still unbuilt](handover/frontend/the-menus-draw-the-discs-layout-the-chrome.md)
- [PS2 front-end layout is hardcoded to 480x272](handover/frontend/ps2-front-end-layout-is-hardcoded-to-480x272.md)
- [Audio: race SFX plays on all three titles](handover/audio/audio-race-sfx-plays-on-all-three-titles.md)
- [`oag-trace plan` has never been replayed into the emulator](handover/tooling/oag-trace-plan-has-never-been-replayed-into.md)
- [M6 authored lighting: no hardware light slot found enabled, and no `DirectionalLight` consumer found anywhere](handover/rendering/m6-authored-lighting-no-hardware-light-slot-found.md)
- [`/psp-pure-usa`, `/psp-pure-eu` and `/psp-pure-eu-reimport` were missing from the project on 2026-08-10, despite the 2026-08-09 note below describing them as already imported and saved](handover/tooling/psp-pure-usa-psp-pure-eu-and-psp.md)
- [The EU/Pure imports have never been diffed](handover/tooling/the-eu-pure-imports-have-never-been-diffed.md)
- [The ghost-ship renderer is read and written down nowhere else](handover/rendering/the-ghost-ship-renderer-is-read-and-written.md)
- [Magstrip leftovers, none load-bearing](handover/gameplay/magstrip-leftovers-none-load-bearing.md)
- [Magfloor gfx/sfx: two real `.vex` effects found, the trigger is read, sfx and the other two titles are open](handover/rendering/magfloor-gfxsfx-two-real-vex-effects-found-trigger-read-sfx-open.md)
- [Frame comparison: three residuals](handover/tooling/frame-comparison-three-residuals.md)
- [Weapons: nine of thirteen, and the dispatch table read whole](handover/gameplay/weapons-eight-of-thirteen-the-plasma-and-the.md) - **2026-09-09: the Plasma's charge and detonation both closed, and the two findings point opposite ways.** The weapon *does* wind up before it fires - one second, held on the firing craft's nose, from `Plasma_Init`'s `+0x4c`/`+0x50` spent by the pool walker `Plasmas_Update` (`0x0886b490`) - so the from-play oracle was right for the third time. And `<Plasma charge_time>` is **still** read by nothing: the duration is a hardcoded `1.0f` in `.rodata`, not the authored `3`, and **Pure hard-codes the same second in the same two stores**. A three-second charge added to match the attribute would have tripled it. The negative is measured rather than assumed: across all 69 functions that reach the weapon-stats table, every access at `charge_time`'s `+0x9c` comes to zero against fourteen at the `venomspeed` control offset - **run the control offset or the negative is worthless**. `WO_PLASMA_FLASH` is the detonation, off `Plasma_SpawnDetonation` in the same walker's teardown. **The general lesson: in this engine a weapon's per-tick state machine lives in its pool walker, not in its fire handler** - the earlier pass read the whole press-to-flight path and the charge was never going to be in it. **the LeachBeam landed 2026-09-08, so every weapon a player can get now does something** and only the deferred Repulser is out. Two findings worth the row. First, **the reason the previous pass could not build it was a wrong address, not a hard function**: the page's two drain-rate functions were rebased `0x4000` low (`0x08804000 + 0x6eedc` is `0x08872edc`, not `0x0886eedc`) and the stated addresses decompile *cleanly into an unrelated function*, which is why it survived - the second such slip on that one page. Second, the item that page called "the single largest remaining gap" - what consumes `entity+0x120`/`+0x128` - was two functions away from one already in `names.tsv`: `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) and its four-line neighbour `FUN_0883f228`, both called from `FUN_0883f540`, which nobody had decompiled. **When a field's consumer is missing, decompile the caller of the functions you already know.** Still open: `slowShipFactor` is parsed and unspent - it lands on `craft+0x31c`, the one-shot thrust scale `oag_physics::engine` documents and does not implement - and the beam's own ribbon draws nothing, its geometry recovered but its texture unlocated. **A separate defect found in the landed Quake: the wave never retires**, so a Quake can be fired once per race and then circles the ring hitting everyone every lap; see the thread for the mechanism and why the fix is a chosen lifetime rather than a measured one.
- [The AI Cannon's gate is an uninitialised byte, and its value is chosen](handover/gameplay/the-ai-cannons-gate-is-an-uninitialised-byte.md) - landed 2026-09-08, replacing "An AI craft cannot fire a Cannon here". `field 0x16` was blind by construction: no owner addresses the control record at offset `0x16`, because `PlayerInput` embeds it at `controller+0x44` and `Ai` embeds it at **`Ai+0x08`**, so the held byte is `controller+0x5a` and **`Ai+0x1e`**. Nothing writes `Ai+0x1e` at any width, and the `Ai` object is allocated with no zero-fill while the craft object beside it is explicitly memset - the gate is uninitialised heap. `Race::advance_cannons` now runs every slot, and the trigger is **tuned rather than copied** under the standing ruling that opponent behaviour is a design axis: an opponent holds fire while `oag_ai::Driver::holds_fire` finds a target ahead, so the weapon reads as an opponent leaning on you down a straight rather than spraying thirty rounds at empty track. Chosen, not measured, no confidence score. `Driver::social` gained `CONTACT_FLOOR` in the same pass - the minimum yield every driver gives a rival it is already alongside, which the neutral personality used to skip entirely. `craft_sticking_ground_truth`'s statistic was **replaced** (worst single streak no longer separated bug from fix - 90 against 95; total overlapped pair-ticks gives 1,756 against 978) and it is green.
- [A fired Cannon round: the body and its impact are drawn; the two hand-built quads are not](handover/gameplay/a-fired-cannon-round-is-drawn-by-nothing.md) - the round is **not** a particle effect; `BOOT.BIN` names its assets outright and all three resolve in `Data.wad` on both PSP pressings. The round draws as its own mesh (a 1.2x1.2x3.6 dart), confirmed on a real disc screenshot, and (2026-09-09) a wall hit now throws `WO_CANNON_SPARKS` - `Cannon_UpdateRound`'s own raycast branch, distinct from the separate craft-hit path that applies damage but no spark. Which GU display list is the bolt and which the muzzle flash is settled at confidence 88 (an instruction-level bind-then-call-list read, not the load-order guess this thread carried at 40), and the base speed is measured at a flat `500.0`, not per-class. Still open: the two display lists themselves are undrawn (needs a textured-billboard path outside this pass's `mesh_render`-adjacent ownership), and no frame has caught the spark's own moment, only the round in flight.
- [Projectiles follow the floor, and the km/h fix - both now landed](handover/gameplay/projectiles-follow-the-floor-and-the-km-h.md)
- [Both lockable weapons draw their reticle now, and the black background behind it is gone](handover/gameplay/the-missile-fires-without-a-lock-now-and.md)
- [Pure and HD lock on too, and it cost two axes rather than any new recovery](handover/gameplay/pure-and-hd-lock-on-too-and-two-axes.md)
- [The camera shake on impact reproduces its two-rotation shape now; two composition details are open](handover/rendering/the-originals-camera-hud-shake-on-impact-is.md)
- [PVS culling: three ceilings, one of them invented](handover/rendering/pvs-culling-three-ceilings-one-of-them-invented.md)
- [The HUD's four remaining items](handover/frontend/the-huds-four-remaining-items.md)
- [The lap count is per speed class, and three of four classes raced short](handover/gameplay/the-lap-count-is-per-speed-class.md) - 2026-09-09. `Mode::SINGLE_RACE_LAPS = 3` was the module's own self-declared weakest number, a guess carried over from the time trial; the campaign census retired it. **3 Venom, 4 Flash, 4 Rapier, 5 Phantom**, confidence **90** over all 236 authored `PI_Cell` records. Flash, Rapier and Phantom single races were finishing one or two laps early. **The plumbing now exists**: `Mode::laps_target(class)` indexes `Mode::SINGLE_RACE_LAPS_BY_CLASS` and `RaceState::new(mode, class)` takes the class, so no site defaults it silently. The class reaches `oag_race` as `oag_tables::handling::SpeedClass` (already a dependency); `oag-title` was deliberately not added, since it carries class *names* and resolving a name is `oag_game::race::Race::start`'s job - Pure's `VECTOR` rung has no measured lap count and falls back to Venom's `3` with a warning, **chosen, not measured**. Venom is the default and its count is `3`, so the committed determinism reference did not move. `MAX_RECORDED_LAPS` stayed `4` on purpose: **a Phantom race's fifth lap is counted and ends the race but its split is not recorded**, because four rows is what the disc authors and a fifth would change `World`'s size and so the state hash to store a number nothing can display. **2026-09-09, second pass, both settled live under PPSSPP.** Time trial is per class too, confirmed rather than left on the census: one Custom Race per rung (no campaign cell in play) reads `Lap 1 of 3/4/4/5` on Venom/Flash/Rapier/Phantom, matching the census exactly - `Mode::TIME_TRIAL_LAPS_BY_CLASS` replaces the old flat `TIME_TRIAL_LAPS`, confidence **90**. Speed Lap's `7`-vs-`None` conflict resolved in `None`'s favour, and not via the hypothesis this thread first wrote down (campaign-bounded vs Custom-Race-unbounded - false, both read `7`): a Custom Race speed lap also shows `Lap 1 of 7`, but its own in-race pause menu carries an `END SESSION` row that neither Time Trial's nor Single Race's otherwise-identical pause menu has - a mode that offers a dedicated way to end an open run is a mode that doesn't end one on its own, confidence **75**. Zone's `0` is still untouched. Open: **the table is a fallback, not the authority** - a campaign race should read its own cell's `laps` (`oag_tables::race_campaign::Cell` already parses it) once a launch path is traced. [race-modes.md](docs/gameplay/race-modes.md)
- [The Race Campaign is authored, all of it - shape *and* content](handover/gameplay/the-race-campaign-is-authored-shape-not-content.md) - 2026-09-08, two passes in one day. The first read the disc's front-end XML and concluded the campaign's screens are authored and its content is not; the second held the Ghidra bridge and **found the content in full, in sixteen files the first pass never opened**: `Data\Plugins\grids\grid_00.xml` ..`grid_15.xml` inside `Data.wad`, sixteen `PI_Grid` records holding **236 `PI_Cell` records**, every one authoring its track, mode, speed class, lap count, weapons and damage switches, AI count, AI skill and **its own gold, silver and bronze targets**. The law is decompiled: **a medal is worth gold 3, silver 2, bronze 1** (`Cell_MedalPoints`, `0x088bf530`); **a medal is a three-way threshold compare** against the cell's own targets with the direction flipped for Zone and Elimination (`Cell_EvaluateMedal`, `0x088bf620`, ordinal `0 = gold`); **`<Unlock Grid="Grid0"/>` means "you scored at least grid0's `RequiredPoints` in grid0"** (`Unlock_GridPointsMet`, `0x0888ebd8`), a 12/16/20/24/28 ladder against 24/30/36/42/48 maxima. **The campaign reads `FEData.wad`'s 24 per-track records for exactly one thing, AI difficulty** - a cell's `skill`/`skillEasy`/`skillHard` is a position on the track's own `SkillScaleValue` curve, interpolated by `AI_ResolveSkillScale` (`0x08834df4`), which also closes `ai-stats.md`'s unchased-string item. Also measured: **lap counts are per speed class and exact** (3 Venom, 4 Flash, 4 Rapier, 5 Phantom across all 236 cells, Speed Lap always 7, Zone 0), the **mode enum is nine values** off the table at `0x08ab062c` (adding `Custom Grid` and `AI Race` to the seven already known, `7` absent), and **Eliminator's kill target is the cell's gold target** (10/7/5 on all 22 cells) while Zone's campaign targets are 18-24 per track and are *not* `FEData`'s `Zone="25"`. 39 names landed against [`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`](docs/ghidra/functions/psp-pulse-usa/race-campaign.md), the authoritative page. **Trap worth keeping**: the first pass's negative was a negative over the WAD names it thought to try - `Data\Plugins\grids\*` resolves only if you already know the path, and the string that gives it away is `"%s\Definition.xml"` at `0x08a7d6a8` applied to `"Data\Plugins\grids"` at `0x08a7d38c`. Open: the **in-race HUD medal tier is still unread and is a different ordinal** from the campaign's (`hud.md` runs `0 = BRONZE`..`3 = RECORD`; the field is `*(hud + 0x3c) + 0x34`, and `Hud_BindWidgets`' `DAT_08b30ffc` reads, `Eliminator_UpdateKillTarget` and `AI_ResolveSkillScale` are ruled out as its writer) - confidence 50, not renamed, and `hud.md`'s medal-target item stays blocked; **how a campaign event actually launches was not traced**, which is what an implementation needs first; nothing is runtime-verified and only the USA pressing was read
- [Race modes: the start line is measured on one circuit only](handover/gameplay/race-modes-the-start-line-is-measured-on.md)
- [Zone ends now; what is missing is the explosion, not the rule](handover/gameplay/zone-ends-now-what-is-missing-is-the.md)
- [Zone flies its own environment now; the menu is the part still not mode-aware](handover/frontend/zone-flies-its-own-environment-now-the-menu.md)
- [Zone races run on all three titles now, and the craft axis closed the way the circuit one did](handover/gameplay/zone-races-run-on-all-three-titles-now.md)
- [Both titles' Zone grade escalates and draws now; two narrow render questions remain](handover/rendering/2048-and-hd-ship-an-unread-effectsettings-table.md)
- [HD's per-stage Zone textures are grounded now: which file feeds which sampler is read, what the shader does with them is not](handover/rendering/hd-zone-stage-textures-are-grounded.md)
- [HD's Zone ladder draws and its speed-class sound now plays; a 40,000-tick run confirms the grade escalates rung for rung but nothing survives past zone 54, and the class-change blend stays unwired on three unrecovered inputs](handover/frontend/hds-zone-ladder-draws-and-the-zone-to.md)
- [The AI is authored XML, and its units are the only thing blocking a port of the original's numbers](handover/gameplay/the-ai-is-authored-xml-and-its-units.md)
- [The HUD shows a place, measured against the original; the caption pair and opponent place are the open half](handover/frontend/the-hud-shows-a-place-measured-against-the.md) - the shield-bar-colour half resolved 2026-09-05 (threshold plus a one-shot post-hit flash, not a gradient - `Hud_UpdateEnergyBar`, confidence 82); wiring it into `oag_game::hud` is its own thread now
- [The field drove in single file, and the fix is a per-craft personality off the disc's own AI corridor](handover/gameplay/the-field-drove-in-single-file-and-the.md) - **2026-09-07: two threads closed, one stays open.** The "nothing knows another craft exists" bullet is false and corrected with file:line evidence (`crates/ai/src/field.rs`, `Driver::social`/`caution`/`ram`/`avoidance` - landed 2026-08-24/26, still being maintained as of today's `2e255a37`); `docs/gameplay/ai.md` had one stale paragraph saying otherwise, now fixed. `turbo_lookahead_sweep` widened to `07_Track` and `09_Track` (chosen for already-marginal cornering/shield, `13_Track` excluded as confounded by its own jump pathology): zero escapes on both, 672 craft-minutes and 134 Turbos across all three circuits now. Still open: three circuits is evidence the escape is rare, not proof it is gone.
- [Airborne ticks *have* been captured - 65 of them, and they are thin in a specific way](handover/tooling/airborne-ticks-have-been-captured-and-are-thin.md) - **the old title was false, corrected 2026-09-07.** `talons-junction-clean-lap.csv` holds **31** non-grounded ticks over three events (8 with both probes off, longest event 14 ticks) and `talons-junction-autopilot.csv` another **34** - both on `16_Track`, both already committed. `head-pad*.csv` read `grounded 0` on all 110 of their rows, which is a constant column, not airborne data. They already paid for themselves: `crates/trace/tests/hover_contact_ground_truth.rs` uses them to show our contact test reproduces the whole `grounded` column, 2,976 of 2,976. What they **cannot** settle is more specific than "they are thin": `pitch_air` is a gain on an input axis and **no capture records a pitch axis at all** (`scripts/psp_trace_fields.py` has throttle/brake/steer/airbrakes and nothing else), so that one is blocked on locating the craft's pitch-input offset, not on reaching `13_Track`. The `-0.3` weathervane is blocked on a capture that **centres the stick** while airborne - which `16_Track` can give, no unlock needed - not on a longer jump. `grip_air` is genuinely excited (right airbrake pinned at 100 through 1602-1608, lateral velocity -9.3 to -4.7) but confounded by 1 rad/s of yaw across seven ticks: falsifiable off what exists, not fittable
- [The crest lag is crest phase, not the cushion](handover/gameplay/the-crest-lag-is-phase-not-the-cushion.md) - **resolved 2026-09-07 by the pose walk, and the answer is (b).** The two candidates for the seven-tick liftoff lag on `16_Track`'s 1595 crest were our hover reach being too generous or residual crest phase. `crates/trace/tests/hover_contact_ground_truth.rs` walks the **recorded** poses - the original's own position and basis, no integration, so trajectory phase cannot enter - and asks `hover::evaluate` what it finds: **0 disagreements over all 2,976 comparable ticks**, the whole `grounded` column reproduced, all 23 `0.5` ticks and both airborne events included, ours at pose `t` against the recording at `t + 1` (the ordering `replay` already emits under, the same reason `speed_cached` is the previous frame's at 199/199). Confirmed with the integrator and sweep in the loop: `oag-trace run --reseed 2` reports `grounded max error 0.000e0 ... exact`. **The reach is not generous; the craft is simply elsewhere on the crest by the time it gets there**, which is the force-law drift `oag-trace.md` documents. Correction that fell out: `docs/gameplay/ai.md` said `grounded` "cannot read `0.5`" above `FAST_PROBE_SPEED` - all 23 of the capture's `0.5` ticks are above it, `speed_cached` 76.4 to 111.6, because the fast path's flag copy only fires when the front ray *hits* and a front miss still casts the rear for real. Page corrected; branch, flag copy and the `6.0` untouched. Written up permanently in `docs/physics/README.md`. **2026-09-10**: the same technique generalised to the force terms (`crates/trace/tests/force_term_pose_walk_ground_truth.rs`) found none of `forces::evaluate`'s eleven terms explains the crest-approach residual - rolling resistance, the largest, cuts 3.1% of it. One discrete event at tick 1583 (near-constant speed, a sharp direction change - the signature of a projection like `wall::resolve`, unconfirmed against the actual collision geometry) dominates the raw residual by two orders of magnitude over every other tick, but bridged to a position-error yardstick and projected along-track it accounts for only ~12% of the eight-tick shift, wrong-signed - so it does not close the gap on its own by this arithmetic. Force terms are ruled out; whether the wall event or something outside ticks 1560-1600 accounts for the rest is still open.
- [Is there a fifth handling class?](handover/gameplay/is-there-a-fifth-handling-class.md)
- [The lap capture's own "the HUD said `Lap 2 of 3`" claim is now in tension with the finding that closed this row, and nobody has reconciled them](handover/frontend/the-lap-captures-own-the-hud-said-lap.md)
- [The 13 % roll-stiffness gap](handover/gameplay/the-13-roll-stiffness-gap.md)
- [Craft-to-craft collision is implemented; the stun is still not armed](handover/gameplay/craft-to-craft-collision-is-implemented-the-stun.md)
- [Craft collision feel: sticking vs spin](handover/gameplay/craft-collision-feel-sticking-vs-spin.md) - **2026-09-10: the spin half is measured and fixed.** The original applies the pair impulse at each body's own position (`body+0x30` as `Body_ApplyImpulseAtPoint`'s point, PSP and PS2 alike, caught live on six contacts), so a craft-to-craft hit never spins either craft; `oag_physics::pair` now does the same and the `-267` deg/s first contacts are gone. Weapons-live pair-ticks moved 1,558 -> 1,134 untuned. **The wall-path follow-up is also closed, 2026-09-10, with a negative result**: `Body_ResolveContact` does use the same `cross(r, R^T omega)` idiom, and that *is* the textbook `omega x r`, because `body+0x150` is negated body-local rather than world-space (fitted at a 2 % residual on four captures against 200 % for world-space). `wall.rs` was already right; implementing the literal reading cost `07_Track` its clean lap. The defect it left in the *pair* path is **fixed, 2026-09-10**: `pair::respond` builds `vn` through `Body::velocity_at` like every other path, `j` came back bit-identical from the original's own captured bytes, and weapons-live pair-ticks moved 1,134 -> **1,052** with sustained (streak > 10) 808 -> **625**. The sticking half: the `alongside`/`CONTACT_FLOOR` fix landed and closed the sticking question; **2026-09-09 reopened it**. Correcting the Cannon's base speed from a chosen `400.0` to a measured `500.0` moves a weapons-live full grid from 1,051 overlapped pair-ticks to **1,558**, and `MAX_OVERLAPPED_PAIR_TICKS` was re-baselined 1,300 -> 2,000 to keep `main` green. Both of the figures the old bound's "midway between the regimes" rule used predate that fix, so the pathology ceiling is stale. The discriminating check is a play session with weapons live, not another statistic.
- [The corrected craft-pair narrowphase moves a full grid's trajectories enough to fail a race-length guard](handover/gameplay/the-corrected-craft-pair-narrowphase-moves-a-full-grids-trajectories.md)
- [The barrel roll is playable; its visual is read but not drawn](handover/rendering/the-barrel-roll-is-read-and-unimplemented.md) - `oag_physics::barrel_roll` has the tap history, the shield-gated arm (8% of shield, refused at or below cost), the self-completing phase ramp, and the landing payout wired into all three of `craft+0x1c0 & 0x400`'s handling consumers (1.5x lateral grip, the hover rebound forced to 1.0, the turbo add), all off values read live from both PSP pressings. **Reachable from a real race as of 2026-09-06**: `ShipControls` carries the d-pad tap edges, `barrel_roll::advance_gesture` reads those and the steering axis crossing `+-90` against the new `ShipState::roll_axis_zone` latch, and `crates/gameplay/tests/barrel_roll.rs` arms a roll from an `InputSnapshot` through `ship_controls` and `oag_physics::step`. **The render search landed 2026-09-06 and found a consumer** - `+0x87c` eases into `+0x880`, which rolls the ship about its nose and the internal camera's up vector with it (confidence 88); drawing it moved to its own thread. **The airborne gate is recovered and ported 2026-09-06** (confidence 88): `Ship_UpdateSideshiftInput_q` branches past every tap when the contact bit `craft+0x1c0 & 1` is set (`0x08846bd0`) and zeroes the three-slot tap history outright (`0x08847018`-`0x08847030`), so a grounded craft cannot hold a partial gesture and an alternation cannot span a takeoff - which closed the `07_Track` regression, taking an Ace opponent from 4-8 arms and 30-54 shield a race to **zero arms on all twelve circuits**, and took `14`/`09`/`10` from 3.9/9.5/10.8 shield left to 41.4/59.2/57.0. The same read made the release event a reading rather than a guess (it is the landing, as guessed) and added the phase levelling on any completed alternation. **The original's AI almost certainly never rolls at all** (confidence 85): the tap leg early-outs on a null `ship+0x78` - the player's pad, its buttons indexed by `Options_ButtonForAction` - the axis edge detector's previous sample is a **single global** at `0x08ae4cf0` referenced from that one function, and a complete scan for arm-bit stores finds only the two in that block plus a network/replay applier. This port lets every craft reach the gesture anyway, on a maintainer's ruling and as a documented deviation. **One invented number now exists here, and it is the mechanic's first deviation**: `oag_physics::barrel_roll::AI_ROLL_SHIELD_FLOOR` = 20%, an AI-only shield floor the human player's path does not carry, chosen by the maintainer on 2026-09-06 who stated plainly that what the original does is unknown - **no confidence score, on purpose**, and it is replaced rather than reconciled if the original's gate is ever recovered. It measures as **dormant**: at 0.20 and at 0.00 all twelve circuits report identical arms, spend and finishing shield, because the grounded gate already accounts for everything. Also fixed 2026-09-06, reported from play: a second roll the other way drew **two** rotations, because the phase parks at `+-1.0` and the next roll travelled 2.0 of it - the original's phase levelling on any completed alternation is the fix, pinned on the traversal rather than the endpoint. Still open: no capture, so the whole chain is a static read; `<Special turbo_jump>` has a name and no reader; PS2 unchecked
- [The barrel roll has a drawing job, and the original's own recipe for it](handover/rendering/the-barrel-roll-has-a-drawing-job.md) - drawn and on screen since 2026-09-06: a symmetric quadratic ease on `ShipState::roll_phase`, then `eased * 6.28` radians about the craft's **nose** on its display matrix, and the same angle on the internal camera's up vector, all at confidence 88. **The sign is now measured too (2026-09-08, confidence 92)**: a memory-read capture against a real, booted `pulse-psp-usa.chd` settled the VFPU `sin`/`-sin` ambiguity the axis survived but the sign didn't, and closed `vmmul.q`'s operand order and `[0x002ace1c] = 0.75 = g_craft_scale` in the same pass - `oag_render::roll::ROLL_DIRECTION` is `-1.0`, flipped from the `+1.0` a player's own gesture would have suggested. Only the internal camera's own site (`FUN_088455ec`) was not independently captured; it inherits the ship's measured sign as a labelled inference. `craft+0x854`'s lean term shares the angle and must not be transcribed - confidence 0 upstream. **Also found this pass**: every offset in this section is on the **entity**, not the craft `Ship_UpdateCraft` itself takes - the third time this exact trap has bitten the project
- [Sideshift has no runtime leg](handover/gameplay/sideshift-has-no-runtime-leg.md)
- [Task #31 residual: the unguarded `slice(..)` in the race's draw path](handover/rendering/task-31-residual-the-unguarded-slice-in-the.md)
- [Menus: no pause overlay while a race is suspended](handover/frontend/menus-no-pause-overlay-while-suspended.md) - the CONTROLS page's key-capture prompt landed (2026-09-07) and stays landed. **The pause overlay itself did not actually show on a live run, found and fixed in the same pass (2026-09-09):** a real `Escape` from a running race parked the state correctly, but Pulse's own `FE_SCREEN` authors a full-screen `<Image>` mark that `draw_list` drew unconditionally, painting straight over both the pause overlay's fill and the parked race underneath it - the docs' "Both PSP titles' frames are unread" was stale. `Frame::backdrops` (`crates/game/src/menu/frame.rs`) now drops any full-screen clear/mark behind a parked race while keeping structural chrome (top bar, footers); live-reproduced before and after, full `just` gate green including the ground-truth regression under `OAG_REQUIRE_GAME_DATA=1`. This also closes the `<ScreenClear>`/`MenuSkin::background` gap the same way. `docs/architecture/menus.md`'s two stale paragraphs (line ~62's "frames are unread", line ~679's "now draws it, dimmed") still need their owner to fix - both are `docs/architecture/`, out of this thread's `docs/frontend/` lane
- [MONITOR has only ever run on a one-screen machine](handover/tooling/monitor-has-only-ever-run-on-a-one.md)
- [Pure's dev/pub hold duration, and how the original picks a regional cut](handover/tooling/pures-dev-pub-hold-duration-and-how-the.md)
- [`Movie::entry_name` hardcodes `_US`, so a European Pure disc shows the American card](handover/tooling/movie-entry-name-hardcodes-us-so-a-european.md)
- [Where Pulse's language picker belongs is unevidenced](handover/frontend/where-pulses-language-picker-belongs-is-unevidenced.md)
- [Pure's `Title Screen` logo wordmark is found; `FE Screen`'s own backdrop is not](handover/frontend/pures-title-screen-is-missing-its-own-logo.md)
- [A *slot-resolved* record's own field layout](handover/tooling/a-slot-resolved-records-own-field-layout.md)
- [Pure's DLC trailer key and TEST.bin, still unknown](handover/tooling/pure-dlc-trailer-key-and-testbin-unknowns.md)
- [A model built from several small pieces sharing one atlas](handover/rendering/a-model-built-from-several-small-pieces-sharing.md)
- [Which movie cut plays, and what plays the three 260-frame reels](handover/tooling/which-movie-cut-plays-and-what-plays-the.md)
- [The PS2's backend task table has four unread cases](handover/tooling/the-ps2s-backend-task-table-has-four-unread-cases.md) - what remains after the PAL/NTSC selector's trigger was traced 2026-09-08 to `<Values task="Switch50">` on a first-boot 60 Hz screen ([refresh-mode.md](docs/ghidra/functions/ps2-pulse-eu/refresh-mode.md)): four task names the archive never authors, and whether the player's answer is persisted
- [SteamOS's own glibc version](handover/tooling/steamoss-own-glibc-version.md)
- [The loading screen draws now, and its wave is partly runtime-verified](handover/frontend/the-loading-screen-has-no-caller.md) - internal-state recurrence matched to float-rounding precision across hundreds of columns (confidence 96); the emitted draw arguments also matched (confidence 90) but every capture stayed inside `x=0..14`, undeveloped amplitude, and hit an unexplained ~130-140-hit ceiling that leaves `PPSSPPHeadless` running but unresponsive
- [`Anim Transform` `0x3c0` is read and played, and closing it fixed a placement defect](handover/rendering/anim-transform-0x3c0-is-read-and-played-and.md)
- [The PS3 Ghidra path works; the cspec-only fork is done, `lvlx` is what's left](handover/tooling/the-ps3-ghidra-path-works-two-improvements-to.md)
- [Wipeout HD Fury's executable is 26,100 functions with 26 of them named, and no gameplay behaviour yet](handover/tooling/wipeout-hd-furys-executable-is-26100-functions-with.md)
- [Players should be able to build pilots in game, and a naive editor would eat their comments](handover/frontend/players-should-be-able-to-edit-pilots-in-game.md) - **requested by the maintainer 2026-09-06**: in-game CRUD over the pilot `.toml` files, and **all five operations have now landed**. List, edit, save and create-from-template first; then rename and delete, once `crate::prompt` gave this project the text entry it had none of. Both traps were paid: `set_axis` goes through `toml_edit` so a hand-written comment survives an edit, and `rename_pilot` is a *move*, so the file arrives at its new name byte for byte. **Deleting `aggressive.toml` restores the built-in rather than removing a pilot**, and the confirm says so by name. What is left in the thread is the axis preview, `string_id` for the remaining rows, and the one-axis-at-a-time silence pinned by a test
- [Pulse authors its own text entry, and it is not a keyboard](handover/frontend/pulse-authors-its-own-text-entry-and-it-is-not-a-keyboard.md) - a **positive** result found while checking whether the disc had a keyboard to play before this project drew its own. It has no keyboard and no key glyphs, and the standing `sceUtilityOsk` assumption is **refuted at 92** - all four OSK NIDs are absent from both Pulse executables, while Pure links them. What Pulse has instead is a `<TagInput>`: a row of `length` character cells scrolled one glyph at a time, 15 instances in `Data.wad` (entry **#1083**, hash `b94fe6f9`, name unresolved), geometry authored per screen, and a **70-character alphabet in `BOOT.BIN`** at file offset 2,808,976 / vaddr `0x08AB1C10`, at 85. Nothing renders it yet; `docs/formats/fexml.md` has no `TagInput` row. Open at 65: the input mapping, one PPSSPP capture away
- [Five reference traces are missing here, and the silent skip is why nobody noticed](handover/tooling/five-reference-traces-are-gone-and-the-skip-hid-it.md) - five of the six trace captures the ground-truth tests name do not exist in this checkout, and their absence never turned a build red because a missing reference **skips**. `just test-data` reports 2 failures; the same suite under `OAG_REQUIRE_GAME_DATA=1` reports 14, eleven of them missing-file panics rather than behaviour. [ADR-0046](docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md) now tracks a trace a test names, per file by name, so this cannot recur - it does not recover the five. Open: removing the skip path, recapturing, and two docs that assert a capture this checkout cannot demonstrate

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
| **The four-corner hover variant** | Not implemented. Its selector is known (`DAT_08ab07e3 == 0 && DAT_08b31048 == 6`, which also disables the brakes and the weapons) and it is not the racing configuration. |
| **Pure asset work** | No longer deferred as a whole. [ADR-0022](docs/architecture/adr/0022-title-packages.md) opened the format and asset layers on the strength of [`pure-status.md`](docs/formats/pure-status.md)'s measurements; ADR-0009 item 2 still defers Pure *simulation* work behind M4's exit. |
| **PS3 content** | Still paused (since 2026-08-09), on the toolchain rather than on the research - see [`data/README.md`](data/README.md#the-ps3-image-is-encrypted-and-nothing-here-decrypts-it-yet). None of `PS3Dec`/`scetool` is installed and there is no RPCS3 install to borrow a decrypted copy from. |
| **Vita content** | Not paused: all four `eboot.elf` (base + patch v1.04, both regions) are decrypted end to end and imported into Ghidra as of 2026-08-26. The layer the prior entry missed - `eboot.bin` past PFS decryption is still NpDrm-encrypted under its SELF wrapper, a plaintext header and phdrs say nothing about the segments - is solved by `scripts/vita-self-decrypt.py`, which needs only the title's klicensee (itself pure-software recoverable from a zRIF, no F00D/hardware - `scripts/zrif-to-klicensee.py`). See [toolchain.md#vita](docs/reverse-engineering/toolchain.md#vita). Default target: `/vita-2048-eu-v104/eboot.elf` (EU, patch v1.04); naming and RE work itself are the open thread now. |
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

**Adding a weapon to `oag_gameplay::pickup::IMPLEMENTED` re-rolls every
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
number.** 2026-09-08, same pass. `RACE_EFFECTS` (`crates/game/src/race/effects.rs`)
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
lands on the correct next state). See `crates/game/src/audio.rs`'s
`Audio::open`/`Audio::movie_playhead` and `crates/game/src/main/cli.rs`'s
`--no-audio`.

**The `ghidra-mcp` bridge is one shared instance across every concurrent session, and `switch_program` sets a global "current program" pointer any of them can move out from under you.** 2026-09-03, reading `ps3-hdfury-eu`'s `TrackSelection_Screen` while a second, independent thread (`2048-and-hd-ship-an-unread-effectsettings-table.md`) was doing overlapping Ghidra work the same week. Three `rename_function_by_address` calls with no explicit `program` argument reported success; a follow-up `get_current_program_info` showed the active program had silently become a different title's binary mid-session, and the three renames had landed there instead - `get_function_by_address` back on `ps3-hdfury-eu` still showed plain `.opd.FUN_xxxxxxxx`. Fixed by redoing all three with an explicit `program: "/ps3-hdfury-eu/EBOOT.elf"` and re-verifying the same way. **Pass `program` explicitly on every mutating call** (`rename_*`, `set_*`, `create_*`, not just `switch_program`), and re-verify with an explicit-`program` read immediately after any rename that matters - a `switch_program` done once at the start of a session is not durable if another agent is driving the same instance. Full write-up: `handover-scratch/hd-zone-tracklist-re-notes.md` from that session (untracked, may not survive - this paragraph is the durable copy).

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

**ADR-0036 states one consequence that is wrong, and ADR-0038 supersedes a second item in it - both ADRs are immutable, so this is the correction of record.** 2026-09-02, landing the UI-compositing restructure. (1) ADR-0036's consequence *"every race `--screenshot` changes bytes when the HUD moves"* is **false**: only `--presented` does. The plain capture path in `crates/game/src/race/capture.rs` builds no `Framebuffer` and runs no resolve, so its HUD was already drawn at native size and its bytes are untouched - verified by the capture path having no `Framebuffer` branch outside `presented`. Anyone re-baselining race screenshots on the strength of that sentence would be regenerating goldens for a change that did not reach them. (2) ADR-0036 puts a movie on the *scene* side of the UI seam, which for the front end would have meant splitting one draw list across two passes; [ADR-0038](docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md) supersedes that item, because the front end's scene side holds **nothing but the movie quad** and the launcher and loading screen have no movie at all. A stage with no 3D scene draws its whole list straight into the presentation target and `resolve_scene` is not called. Read ADR-0038 before ADR-0036's seam wording sends you to build the split.
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
`missing field 'lightmaps' in initializer of oag_render::mesh::Model` against a
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
