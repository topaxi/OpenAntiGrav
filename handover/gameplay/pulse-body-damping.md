---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-11
summary: "Pulse and Pure write 0.01 body damping and do not fly it; turning it on moves 5 hashes and 8 Pulse trajectory tests"
size: M (45)
difficulty: medium (45)
---

# Pulse and Pure: the rigid body's velocity damping is read, measured on HD, and not flown

2026-11-10, `hd-late-speed`. The law, its addresses and the HD measurement:
`docs/ghidra/functions/ps3-hdfury-eu/body-damping.md`. Pulse's own read:
`docs/ghidra/functions/psp-pulse-usa/rigid-body.md`, "The linear damping really is `0.01`".

`Body_Integrate` ends each step with `velocity -= velocity * h * 0.01` and the angular twin, on
every executable read (Pulse PSP `0x0884e368`/`0x0884e374`, PS2 `0x0015d088`, Pure
`0x089345e8`, HD `0x000f6fd8`/`0x000f70f8`, Omega `0x013467d9`). HD's is measured live to
`0.0100` and flown, with Omega and 2048. The integrator takes it as
`oag_physics::laws::BodyDamping`; `Laws::PULSE` and Pure's `CRAFT_LAWS` carry `NONE`.

## Open

- **Pulse flies no body damping.** Evidence it should (confidence 88): replaying
  `data/traces/talons-junction-standing-start.csv` through `oag-trace run` with
  `Laws::PULSE.body_damping` set to `0.01`/`0.01`, the forward speed error over the clean
  launch decays from `-1.15` (tick 60) to `-0.19` (tick 185); without it it drifts from `-0.91`
  to `+0.73`, and the replay's largest speed error falls from `3.88` to `1.25`.
  `docs/physics/cornering-ground-truth.md` already needed the term in its forward-axis probe.
  A lone craft still gets round all twelve circuits with it on
  (`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`: twelve
  clean, laps `0.1`-`1.3` s slower, `07_Track` `0.2` s faster, `01_Track` 0 respawns).
- **What moved with it on for every title** (full `test-data` run, 2026-11-10, 11,653 tests):
  - committed hashes: `oag-physics::determinism the_simulation_matches_the_committed_reference`,
    `oag-ai::determinism` (`the_drivers_match...`, `the_speed_plan_matches...`), and
    `oag-raceplay tests::extensions` (`VANILLA_FOLD`, two tests);
  - `oag-trace replay::tests`: three harness self-checks that replay a constant-velocity
    recording through a "free" body (give them an environment with `BodyDamping::NONE`);
  - Pulse trajectories: `ai_clean_lap_gate::track_14_phantom`,
    `ai_dekonstruct_symptoms_ground_truth::de_konstruct_symptoms_phantom`,
    `autopilot_original_ground_truth::..._without_a_rescue_05`,
    `eliminator_finish_ground_truth::the_chosen_kill_target_is_what_the_eliminator_ends_on`,
    `plasma_timing_ground_truth::the_plasma_releases_flies_and_detonates_on_the_originals_ticks`,
    `speed_plan_ground_truth::speed_plan_phantom_forward`,
    `original_ai_ground_truth::at_easy_the_lower_stop_reads_what_the_original_logged`
    (`oag-game`), `hover_contact_ground_truth::a_reseeded_replay_lifts_off_the_crest_on_time_with_the_cage`
    (`oag-trace`).
  All eight pass with Pulse's damping off. None was diagnosed.
- **The angular half is measured live on Pulse** (2026-11-11, `roll-stiffness`, confidence 85):
  four roll step captures in PPSSPP decay by `0.3337`-`0.3346` per period, this engine by
  `0.3366` with the term off and `0.3345` with `body_damping.angular = 0.01`
  (`docs/physics/roll-step-response.md`, "The damping"). Still not flown. The same page's "Open"
  has one unexplained residual: the original leads our step response by a quarter of a frame
  from the release on (next read named there).
- **Pure** follows Pulse: its constructor passes `0.01`, `0.01` and its integrator is Pulse's
  loop (`docs/physics/pure-physics.md`), read and not measured live.

## Next Steps

1. Set `Laws::PULSE.body_damping` to `0.01`/`0.01`, give the three `oag-trace` self-checks a
   free body, and diagnose the eight Pulse tests one at a time (which of them compares against
   an original's number, which samples a point on an Autopilot line). 3-5 hours.
2. A per-frame check on Pulse like HD's: one step of ours from each row of a PSP held-throttle
   capture, forward residual over `speed x dt` with the term off and on (HD read `0.01002` and
   `0.00003`). `hd_air_pose_walk.rs` is the shape; `psp-trace.py` writes the rows. 1-2 hours.
3. Regenerate the committed hashes in a commit of their own, naming this law.
4. Pure: the same switch in `oag_pure::race::CRAFT_LAWS`, then its own gates.
