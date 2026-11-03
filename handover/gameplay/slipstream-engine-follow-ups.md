---
status: ready-to-wire
titles: [all]
updated: 2026-11-03
summary: "Portal pieces 6-9 landed on slipstream; open: wreck gate after merging main, mesh portal, route and island exits, two mod pickups."
categories: [tooling, frontend]
size: L (50)
difficulty: medium (50)
---

# Engine pieces for a portal-teleport mod: what is left

The official mod `oag.slipstream` (official-mods submodule `assets/mods/official`, branch
`slipstream`; lane 18b writes the guest) needs engine pieces 1-9 from its brief. Pieces
1-5 landed on engine branch `slipstream` (see the wasm-extensions README, "And the pieces
of a portal weapon", and `hooks.md`, "Built for a portal weapon"):

- `Race::move_to` (forward to a course point under a lap, the line crossing credited:
  `Standing::jumped`, `RaceState::jumped`), `Race::point_ahead` (`Distance`: units,
  seconds, lap share, progress, trigger).
- `transit` (command 27) with `transit-end` (event 11), `Ship::transit`, `Ship::in_world`,
  reset triggers held (`Race::reset_held` in `Race::armed`).
- `probe-clearance` (29) and `clearance` (12); `scale-speed` (28).
- `[[pickup.odds]]` per title and class (`oag_tables::pickup_look::odds_for`).
- Proof: `transit_ground_truth` (Pulse PSP EU `16_Track`), `tests::transit`,
  `tests::extension_odds`, `standing::jump_tests`, the ABI byte pins.

The engine never names the mod.

## Open

1. **Landed, see the checks below.** Item 1 (AI firing: `[extension] drives_ai_pickup`, the
   tracking record `progress`/`place` and the course length in the observation), item 2 (outputs:
   `screen-fill`, `fov-offset`, `fire-mod-effect-ahead`), item 3 (per-title icons
   `[pickup.titles.<title>]`, icon placed on the title's own Turbo glyph box) and item 4
   (`default-enabled`, ADR-0084) are on branch `slipstream`. What each left open:
   - **Wreck gate: landed** (main merged). A guest's `clear-pickup`, `scale-speed` and `transit` for a
     craft `may_use_pickup` refuses are ignored engine-side (`RaceSim::may_use_pickup`, which
     `Race::may_use_pickup` calls); proof `tests::extension_view`.
   - **The portal is a camera-facing sprite**, a mod PNG through the particle system placed at the
     road ahead (`fire-mod-effect-ahead`), not a half disc standing across the road. A real mesh
     or a custom shader is a later piece.
   - **`screen-fill` is additive**, as the title's own flash is; a subtractive or alpha-over wash
     needs a second blend.
   - **Pure** draws the pickup backdrop and no mod PNG (its icons are `<Mode3D>` models, and no
     Pure art ships). **2048** was not checked by a screenshot: a headless `--race` shows no
     pickup box for `--give turbo` either. **Pulse PSP and PS2** are one title to a manifest, so
     they share one `[pickup.titles.wipeout-pulse]` icon.
   - **Mod checks**: the 2048 and Omega checks of CLAUDE.md "A 2048 finding is checked against
     Omega": the same HUD code draws the mod icon on both (`oag_hud::pickup::mod_icon_draw`);
     Omega is screenshot-verified, 2048 is not.

2. **Exit on a branch route.** `Race::move_to` re-seats the driver on the ring
   (`Race::reseat` uses `Branching::on_ring`), so an exit on a fork's route drives back to
   the ring. `Course::locate_on_route` already maps a route sample to ring progress; the
   work is seating the driver on the route, plus a test on a forked circuit
   (`branch_progress_ground_truth`'s). Size S, half a day (50). Slipstream itself uses the
   main ring (maintainer, 2026-11-03).
3. **Exit with no spline near it (a floating island).** No lap fix, no AI line, and the
   distance reset would fire. Recommended rule for our track format: the author gives the
   island its own spline path that rejoins the circuit, which makes it item 5. An engine
   notion of an off-course stretch is a design question first. Size L if built (30).
4. **Two mod pickups in one race.** A mod with `[[pickup.odds]]` owns the mod pickup from tick
   0, so with slipstream and energy-turbo both on, energy-turbo faults on its first
   `pickup-roll` ("the mod pickup belongs to mod ..."). One mod pickup a race is the rule
   today; more than one needs a pickup kind per mod.
5. **`Distance::Trigger` with a real track file.** It is tested only through the
   refusal path (a track with no trigger). A `track_props_ground_truth`-style test with a
   trigger pair is owed.

## Next Steps

1. Open items 2-5: a route exit, an island exit, two mod pickups, a `Distance::Trigger` test on a
   real track file.
2. Quote `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
   before and after any change that touches racing.
