# test-data's tail is now real decode and simulation sweeps

The 2026-10-07 gate-speed lane did two things. It removed the HD render tests' per-texture disc reopen (`crates/render/tests/archive_cache/`). It also raced `ai_roll`'s and `ram`'s independent races side by side (`crates/game/tests/in_parallel/`). Members now gate with `just gate-affected` / `just test-data-affected`. The measurements are in
[`docs/architecture/workspace-layout.md`](../../docs/architecture/workspace-layout.md),
"Reopened archives, serial sweeps and an affected gate". The full `test-data` went from 465 s to 326 s, and its slowest test from 221 s to 133 s. An `oag-mesh` member gate went from about 530 s to 271 s.

## Open

- **`oag_physics::collide::TriangleSoup::raycast_all` and `raycast` are brute force.** They take 53% of an `ai_roll` grid race in perf, and they dominate every simulation test and the game itself. A spatial index has to return hits in the same order, or the state hashes move, so this is a physics lane.
- **The remaining 2048 sweeps run their races in series.** That is `ai_fork_split_ground_truth` (125-133 s under load, now the slowest test in an `oag-game` selection), `magstrip_wire_2048_ground_truth`, `zone_airbrake_flaps_ground_truth` and `fire_law_inherit_ground_truth`. If their races are independent, `in_parallel::map` applies unchanged. Check that before using it, and diff the printed output against the serial run, as was done for `ai_roll` and `ram`.
- **Decode floors:** BC7 is 87% of `omega_gnf_pixels`' CPU (`oag_texture::bcn::bc7`), and ATRAC9 dominates `omega_wem`. Both are real work in production crates already at opt-level 2.
- **Selection granularity.** A lane in `oag-mesh`, `oag-render` or `oag-fx` selects `oag-game`, which is 55% of test time on its own. That holds because every one of its ~130 test binaries links the whole library.

## Next Steps

1. Check `ai_fork_split_ground_truth` for independent races. If it has them, wrap them in `in_parallel::map` and diff the output.
2. Open a physics lane for a raycast acceleration structure, with the determinism hashes as its gate.
