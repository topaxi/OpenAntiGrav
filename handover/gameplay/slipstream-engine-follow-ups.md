---
status: ready-to-wire
titles: [all]
updated: 2026-11-04
summary: "oag.slipstream wired; open: mesh portal, route and island exits, Distance::Trigger on a real track."
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
     road ahead (`fire-mod-effect-ahead`), not a half disc standing across the road. A shader can
     draw it now (`docs/architecture/wasm-extensions/mod-shaders.md`); a real mesh is a later piece.
   - ~~`screen-fill` is additive~~: a fill through a shader is alpha over or added, as its header
     says. The flat fill stays additive.
   - **Pure** draws its backdrop grid model and the mod's PNG centred on it at its own size
     (`pure_psp.png`, glyph alone, variant b; checked by screenshot on `pure-psp-eu.chd`). The
     pad share there is 5.9-6.5 % (AI) and 4.3-4.5 % (person) against a Turbo's 13-15 %; the
     portal colour is Pure's glyph green `#40ff40` (chosen), near 2048's green. **2048** was not checked by a screenshot: a headless `--race` shows no
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
4. ~~Two mod pickups in one race~~ (landed: a pickup kind per mod, `tests::extension_pickup_kinds`,
   `slipstream_energy_turbo_ground_truth`).
5. **`Distance::Trigger` with a real track file.** It is tested only through the
   refusal path (a track with no trigger). A `track_props_ground_truth`-style test with a
   trigger pair is owed.

## Slipstream mod wired (lane slipstream-mod, 2026-11-03)

`oag.slipstream` (official-mods `slipstream/`) is the first user of the pieces above and is proven
by `tests::extension_slipstream` and `slipstream_ground_truth` (Pulse PSP EU `16_Track`). What it
found, for the engine:

- **A guest could not tell the title it ran on**, so a per-title portal colour was impossible.
  Added `Observation::title()` / `title!` (key after the tracking records, no offset moved).
- **A mod effect is an upright billboard centred on its point**: a portal art's flat edge belongs
  on the middle row of the PNG, or the depth test hides it under the road. An additive effect on
  Pulse's pale track is nearly invisible; the mod draws alpha-over in a deep colour.
- **Fire path**: a person's press raises `pickup-fire` and leaves the pickup, and the AI's Turbo
  gate does the same, so the guest refuses a fire by simply not clearing the pickup. The mod keeps
  the default gate (`drives_ai_pickup` false): the gate already needs a straight and a clear road,
  and the guest adds the portal-room check on the same event.
- **The half-way gate still applies to a jump**: a jump over the line from a lap whose half-way
  gate is shut (a lap entered by a late jump) is no lap and no finish. Right for a mod; a mod
  that wants "skip the lap" needs a separate command.
- **Pulse PSP and PS2 are one title to a manifest**, so one icon (PS2's, the larger) is shipped.
- 2048 and Omega: checked, the same guest, icon and fill run on both. To see it:
  `OAG_MODS=assets/mods/official oag-game <disc or folder> --race --no-audio --mode single_race --give extension --autopilot --input-script fire.inputs --ticks 730 --screenshot out.png`
  with `[mods] enabled = ["oag.slipstream"]` in the settings file and `fire.inputs` = `700 cross`,
  `1 cross square`, `200 cross`. 2048's disc icon (`2048.png`) is drawn by the mod and the frame by
  the engine; the pickup box shows with `--give extension` (it did not without a pickup).
- The pad odds are one row for every title and class, checked against each real table
  (`slipstream_ground_truth::its_share_of_the_pad_draws_is_a_fraction_of_a_turbo_on_every_titles_real_table`).

## Mod shaders, the portal's lane A (lane mod-shaders, 2026-11-04)

`[shaders]`, an emitter's `shader` key, a shaded `screen-fill`, `draw-beam` and `set-shader-param`
landed; the contract, refusals and what a hung GPU is still open to are
`docs/architecture/wasm-extensions/mod-shaders.md`. For the next two lanes:

- **portal-view (B):** fill the slot with `oag_fx::mod_shader::Stage::set_engine_texture(device,
  Some((view, view_projection)))` (rebuilds the bind groups) and
  `set_engine_view_projection` per frame; a shader reads `engine_texture(input.screen_uv)` or
  `engine_texture(engine_uv(input.world))` and `engine_texture_bound()`. The stage lives in
  `Scene::mod_shaders`. **One matrix per stage**: a split screen with a portal in two views needs
  that written per view beside the rest of `Stage::upload`.
- **slipstream-portal (C):** `shader = "<name>"` on the portal's emitter, its PNG moved to the
  shader's `texture` header; a sprite is centred on its point, as today. The mod's quality row
  reaches the shader as a `set_shader_param` scalar.

Open from it, each small:

1. **A bounded loop is refused with the rest.** Recognising `for (i < constant)` in naga's IR is
   about 100 lines (the counter's store in `continuing`, no other store, no pointer to it handed
   on, the product of nested counts capped). Portal noise can live without it. Size S (50).
2. **`.wesl` is read as WGSL**: no `import`. Runtime linking pulls `wesl` into every build. Size S (40).
3. **A device refusal on the web** is caught by nothing: `oag_gpu::shader_file::scoped` runs bare
   there, because the scope resolves through a promise the page's thread may not block on. No module
   runs on the web yet, so nothing reaches it.
4. **`attach_mist` and `attach_lens_flare` take the caller's format**, not the race pass's: a mod
   shader's stage panicked on exactly that on HD's float target (`Scene::pass_format` is the fix
   there). Whether mist or the lens flare ever run on a chain's float target was not checked.
5. **`docs/architecture/wasm-extensions/README.md` still lists no shader in its section 2 sketch**;
   its "And a mod's own shaders" points at the page.

## Next Steps

1. Open items 2, 3 and 5: a route exit, an island exit, a `Distance::Trigger` test on a
   real track file.
2. Quote `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
   before and after any change that touches racing.
