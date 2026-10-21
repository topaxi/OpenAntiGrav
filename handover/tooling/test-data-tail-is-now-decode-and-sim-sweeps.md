# test-data's tail is now real decode and simulation sweeps

The 2026-10-07 gate-speed lane did two things. It removed the HD render tests' per-texture disc reopen (`crates/render/tests/archive_cache/`). It also raced `ai_roll`'s and `ram`'s independent races side by side (`crates/game/tests/in_parallel/`). Members now gate with `just gate-affected` / `just test-data-affected`. The measurements are in
[`docs/architecture/workspace-layout.md`](../../docs/architecture/workspace-layout.md),
"Reopened archives, serial sweeps and an affected gate". The full `test-data` went from 465 s to 326 s, and its slowest test from 221 s to 133 s. An `oag-mesh` member gate went from about 530 s to 271 s.

## Open

- **The suite is over its ceiling again (2026-10-21, deferred by the maintainer for that day's session).** The lead's full `test-data` at `9c1d28524` ran 9,302 tests, all passing, in **470 s**, and `check-test-budget` failed on the 450 s suite ceiling. The three gates before it, under similar load (load average 20-46), read 445 s, 421 s and 441 s. The tail was `oag-net`'s `extract_ground_truth::omega_extraction_reproduces_its_races` at **245 s** (now one test per title and race, `extract_ground_truth::<title>::<race>_reproduces`, Omega's three at 45-51 s and 2048's at 17-24 s when run alone); after it came `autopilot_original_ground_truth::an_opponent_flies_its_autopilot_by_class_and_difficulty` (148 s), `wipeout_2048_extraction_reproduces_its_races` (134 s) and `hd_extraction_reproduces_its_races` (79 s). The Autopilot test is still one loop over class and difficulty and should split the same way, with no `BASELINE` row.

- **The raycaster is no longer brute force (2026-10-13).** One BVH over every triangle of the world, same answers in the same order; see [`workspace-layout.md`](../../docs/architecture/workspace-layout.md), "The collision world's broadphase". After it, `just test-data` measured 8.4k CPU-s and 610 s against 10.4k and 738 s, and a gdb sample of a 2048 field race still puts most of its time in raycasts under **speed plan learning** (`ensure_speed_plans`: the field's plan, adapted hull plans, route plans), which every race that flies this project's driver pays again.
- **The tail is decode now.** `omega_gnf` (972 s summed under load, BC7 plus the PSARC reads), `omega_wem` (495 s, ATRAC9 in the third-party `atrac9dec`, whose `floor` goes through `compiler_builtins`' soft float on the baseline x86-64 target) and `omega_music` (196 s).
- **The remaining 2048 sweeps run their races in series.** That is `ai_fork_split_ground_truth` (125-133 s under load, now the slowest test in an `oag-game` selection), `magstrip_wire_2048_ground_truth`, `zone_airbrake_flaps_ground_truth` and `fire_law_inherit_ground_truth`. If their races are independent, `in_parallel::map` applies unchanged. Check that before using it, and diff the printed output against the serial run, as was done for `ai_roll` and `ram`.
- **Decode floors:** BC7 is 87% of `omega_gnf_pixels`' CPU (`oag_texture::bcn::bc7`), and ATRAC9 dominates `omega_wem`. Both are real work in production crates already at opt-level 2.
- **Selection granularity.** A lane in `oag-mesh`, `oag-render` or `oag-fx` selects `oag-game`, which is 55% of test time on its own. That holds because every one of its ~130 test binaries links the whole library.

## Next Steps

1. Check `ai_fork_split_ground_truth` for independent races. If it has them, wrap them in `in_parallel::map` and diff the output.
2. Speed plan learning, a sim lane: sample `ensure_speed_plans` on a Pulse and a 2048 field and find what a plan re-learns per hull that the reference plan already knows. The plans are learned in most of `oag-game`'s simulation tests, so a 2x here is roughly a fifth of `test-data`.
3. `atrac9dec` at `opt-level = 3` in the test profile, measured on one `omega_wem` shard; a Cargo profile change, so agree it with whoever owns the profiles.
