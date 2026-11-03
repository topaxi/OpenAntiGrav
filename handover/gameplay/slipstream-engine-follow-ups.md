---
status: ready-to-wire
titles: [all]
updated: 2026-11-03
summary: "Portal-weapon pieces 6-9 open: AI fire observation, mesh/fill/FOV outputs, per-title mod icons and slots, default-off mods; route exits."
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

1. **AI firing of a mod pickup (piece 6).** Today an AI craft holding `Weapon::Extension`
   raises `pickup-fire` through the Turbo's gate (`Race::spend_opponent_pickup`,
   `Driver::affords_boost`). A guest may ignore that and decide itself, but it lacks
   observation: its progress along the lap, its place, the track ahead. The craft record
   is full (16 bytes). 348 bytes of `IO_LEN` are spare after `CRAFTS_AT + 16 * 16`, enough
   for a second per-craft record (progress `f32`, place `u8`, course length in a header
   word). The clearance answer already reaches it as an event. Decide whether the Turbo
   gate stays the default `pickup-fire` for the AI or a manifest flag turns it off for a mod
   that drives its own AI. Any fire or use path built here goes through
   `Race::may_use_pickup(slot)` (main, 37dfef768), so a mod never handles a wrecked craft. Size M (40).
2. **Outputs (piece 7)**, none built:
   - a world-space mesh or sprite anchored ahead of a craft (the portal half disc);
   - a full-screen colour fill per view (`oag_fx::flash::ScreenFlash` is the one screen
     flash owner; `Race::screen_wash` reads it, so a `screen-fill{slot, rgba}` output
     belongs beside it);
   - a camera FOV curve (`camera::finish_vertical_fov` is the one FOV chain, from item 5 of
     the refactor thread; a per-slot FOV offset output enters there).

   All are outputs, never hashed. Size L (50).
3. **Per-title mod icons and the icon slot fixes (piece 8).**
   - Per-title icons: a manifest `[icons]` key per title, or a guest choosing by title.
   - Slot sizes and placement:
     - Pulse PSP: the glyph is 27 px in a 24 px slot.
     - Pulse PS2: 33 px in 26 px.
     - Enlarge the slot; do not shrink the glyph.
     - HD and Omega: the slot (34 and 38 px) is smaller than real glyphs and sits 2.5-3 px
       below the frame centre.
   - Backdrop and blank slots:
     - A 2048 mod pickup gets no disc backdrop.
     - Pure draws nothing for a mod icon. Draw it, or document why not.
   - The adopted PNGs are on official-mods branch `slipstream` (`c8dc884`) under
     `slipstream/icons/`.
   - Size M (40).
4. **Default-off mods (piece 9).** The manifest has no enable field; `settings::Mods` keeps
   only a `disabled` set (`crates/game/src/mods.rs`), so a default-off mod needs an
   `enabled` set too (or one resolver `is_enabled(manifest_default, settings)`, item 7 of
   the refactor thread). Schema and the mod-author skill change with it. Size S (30).
5. **Exit on a branch route.** `Race::move_to` re-seats the driver on the ring
   (`Race::reseat` uses `Branching::on_ring`), so an exit on a fork's route drives back to
   the ring. `Course::locate_on_route` already maps a route sample to ring progress; the
   work is seating the driver on the route, plus a test on a forked circuit
   (`branch_progress_ground_truth`'s). Size S, half a day (50). Slipstream itself uses the
   main ring (maintainer, 2026-11-03).
6. **Exit with no spline near it (a floating island).** No lap fix, no AI line, and the
   distance reset would fire. Recommended rule for our track format: the author gives the
   island its own spline path that rejoins the circuit, which makes it item 5. An engine
   notion of an off-course stretch is a design question first. Size L if built (30).
7. **Two mod pickups in one race.** A mod with `[[pickup.odds]]` owns the mod pickup from tick
   0, so with slipstream and energy-turbo both on, energy-turbo faults on its first
   `pickup-roll` ("the mod pickup belongs to mod ..."). One mod pickup a race is the rule
   today; more than one needs a pickup kind per mod.
8. **`Distance::Trigger` with a real track file.** It is tested only through the
   refusal path (a track with no trigger). A `track_props_ground_truth`-style test with a
   trigger pair is owed.

## Next Steps

1. Item 4 first (small, and the mod ships disabled by default): add the enabled set and
   resolver, then `schemas/v1/mod.tosd` and the mod-author skill.
2. Item 1: decide the default AI gate with the lead, then add the second craft record.
3. Items 2 and 3 together: they are the view half the guest draws with.
4. Quote `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
   before and after any change that touches racing.
