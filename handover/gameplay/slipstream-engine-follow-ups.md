---
status: ready-to-wire
titles: [all]
updated: 2026-11-12
summary: "slipstream, shaders, portal-view, route exits, tunnel, raise-warning landed; open: mesh portal, island exit, occlusion, loops."
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
   (`default-enabled`, ADR-0089) are on branch `slipstream`. What each left open:
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

2. ~~**Exit on a branch route.**~~ Landed. A `Distance::Trigger` whose centre is nearer a fork
   route's sample than the ring's names that route (`transit::Exit::route`, `Transit::route`, in
   the state hash only while a transit runs). `Course::route_point` maps the ring point onto the
   route (the inverse of `locate_on_route`), `Race::move_to` places the craft there, and
   `Race::seat_on_route` (shared with `locate_player_driver`) puts the driver on the route's
   line; `Course::locate_exit` decides ring or route. A reset and every other distance (units, seconds, lap share, progress) stay on the
   ring: only a trigger can name a route, and a craft already on a route that jumps forward by
   units still comes out on the ring. Proof: `transit_ground_truth::an_exit_on_a_forks_route_...`
   (Pulse `05_Track`; red with the seat dropped) and `branch_progress_ground_truth` (every
   Pulse and 2048 route's `route_point` stands at its share of the route). The portal's view is
   route-aware too: `portal-view`'s exit is a progress or a trigger (`reply.portal_view_to_trigger`),
   a trigger on a route is framed from the route's deck (`mod-shaders.md`, "The engine texture";
   proof `tests::portal_view::a_portal_view_to_a_forks_route_is_framed_from_the_route`). 2048 and
   Omega: checked, applies, not wired (their forks are in the same `Course`, the 2048 routes are
   covered by the `route_point` check, no transit or portal-view test on them). Open from it,
   small: the entrance is still a ring progress, so a firer already on a route frames its portal
   from the ring; the official `oag.slipstream` exits by progress and does not use the trigger form.
2b. **Incoming warning for a person the portal passes (lane mod-warnings-api).** Landed: a `raise-warning`
   command (37, `WarningIcon`, SDK `raise_warning`) raises the viewer's incoming-weapon warning through
   `Warnings::raise`; `oag.slipstream` raises the rear Repulser icon once, on the transit, for a person it
   goes in behind and comes out ahead of (proof: `tests::extension_view`, the guest's tests and
   `slipstream_ground_truth::a_craft_behind_that_comes_out_ahead_of_the_viewer_lights_its_rear_warning_and_no_other_does`).
   A mod's own warning icon landed (lane mod-warning-icons): `raise-warning` may name an `[icons]` entry and a
   group (ABI 1 revision 4); `oag.slipstream` raises its own `portal-warning.svg` instead of the Repulser
   (proof: `tests::extension_view`, `oag_hud::incoming::tests`, the same ground-truth test). Open, small: (b) the
   warnings are drawn on Pulse (PSP, PS2) only, one view, one person, so HD, 2048, Omega, Pure, split screen
   and network seats drop a mod's raise; HD, 2048 and Omega author the same widget names under a different
   law (`HudArt::incoming_warnings`), so wiring them is a measurement first.
3. **Exit with no spline near it (a floating island).** No lap fix, no AI line, and the
   distance reset would fire. Recommended rule for our track format: the author gives the
   island its own spline path that rejoins the circuit, which makes it item 5. An engine
   notion of an off-course stretch is a design question first. Size L if built (30).
4. ~~Two mod pickups in one race~~ (landed: a pickup kind per mod, `tests::extension_pickup_kinds`,
   `slipstream_energy_turbo_ground_truth`).
5. ~~**`Distance::Trigger` with a real track file.**~~ Landed:
   `track_props_ground_truth::a_transit_to_a_trigger_comes_out_where_the_trigger_stands`
   (Hillside with a trigger pair, a module answering `trigger-enter` with a `transit` to the
   second; the exit is the course point at the second centre, within 5 units, and the craft
   comes out on its road).

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
  gate is shut (the grid's first crossing, a craft shoved back over the line) is no lap and no
  finish. A lap entered by a jump that landed past half-way has both halves behind it, the ground
  from the line to the landing being driven ground (`Standing::jumped`, `RaceState::update`;
  `standing::jump_tests`), so the crossing after it, driven or jumped, is a lap. Before that fix
  the driven crossing earned nothing and the craft sat a lap behind where it was.
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

## portal-view, the portal's lane B (lane portal-view, 2026-11-04)

`portal-view` (command 36), the secondary view and `oag.window` landed; the contract and the numbers
are `docs/architecture/wasm-extensions/mod-shaders.md`, "The engine texture". For lane C:

- **slipstream-portal (C):** `reply.portal_view(&craft, (entrance, exit), radius, update, generation)`
  each tick the portal stands. Medium is `ViewUpdate::Once`, High `ViewUpdate::Continuous`, Low sends
  none and draws the shader's fallback. Read `engine_texture(engine_uv(input.world))`, gated on
  `engine_texture_bound()`: the second seat of a split always takes the fallback. A progress is
  `Craft::progress()` plus the units ahead, wrapped by the course length, never below zero. Check the
  exit: on Pulse's `16_Track` one lands upside down at the top of the loop.

Open from it, each small:

1. ~~**Omega's particle pool is full of `WO_TRAIL_HITSHIP`**~~ Re-measured 2026-11-09 on
   Omega EU, `oag.window` on, autopilot from the grid, 480 ticks (a temporary probe on
   `Stage::claim`, not committed): with `graphics.trail_hits` off (the default since main
   gated 2048's and Omega's burst) the pool holds the mod's held surface alone, 1 claim in the
   whole run and 0 `WO_TRAIL_HITSHIP`; with it on, 31 of 32 slots are `WO_TRAIL_HITSHIP` and
   the 32nd is the held surface (545 claims), so the cap on held slots keeps the surface drawn
   either way. The burst is a title effect and was not touched.
2. **A wall between the camera and the portal does not skip the view**: only the frustum and the
   face do. Testing the entrance's section against the main view's PVS is the cheap answer. Size S (50).
3. ~~**`Stage::orient` is a no-op on an effect from `play_held`**~~ Landed for `Fired::OwnAt`:
   `Stage::orient_surface` addresses a held surface by its generation
   (`psys::tests::a_held_surface_takes_its_orientation`, red before). `Stage::orient` itself
   stays attached-only, on purpose: the wreck debris and smoke (`wreck_fx`) `play` then `orient`
   and today ignore it, so widening `orient` would change a title visual nobody measured. Open,
   small: a mod effect fired by `fire-effect` (`Fired::Own`, `play` + `orient`) still ignores
   its `axis` (`wall-penalty`'s `sputter.toml` is `rear`); fix it on `orient_surface` and look
   at a frame.
4. **A float target's values above 1 clamp in `engine_texture`**: `fs_main` clamps a shader's output,
   so HD's and Omega's brightest highlights in a portal stop at white. Size S (40).
5. **The texture is a fixed 960x540** at every window size; a configurable rate and size are later
   if a player needs them (spec, "reduced resolution allowed").
6. **Only Pulse and HD have a frame test.** 2048, Omega, PS2 and Pure were read off screenshots.
7. ~~A snapshot was never looked at~~: looked at (lane C, below).

## slipstream-portal, lane C (done)

`oag.slipstream` draws a window (`fx/portal.wgsl`) with a QUALITY row under OPTIONS; the pick reaches the
guest as `Observation::option` (new ABI bytes, `mod-shaders.md` "A mod's quality row"). Open from it:

1. ~~The firer barely sees the window~~: landed. The guest holds the transit until the craft's
   progress reaches the entrance (see item 6).
2. **A queued fire whose clearance fails at its turn is dropped** (the pickup is kept), where a Quake's armed
   press stays armed. Chosen, kept.
3. ~~Online a server's guest has no pick~~: a `view` row reads its default there by construction; a `sim` pick has no receiver yet, as no network race runs a guest (`hooks.md`, "Picks: sim and view").
4. 2048 and HD windows look like a continuation of the scene (the exit is four seconds on, same scenery).
5. ~~`Observation::option` is not part of a race's identity~~: a menu row declares `scope = "sim"` or `"view"`; sim picks are in the state hash and the LAN fingerprint, view picks reach only a second, outputs-only call (`tests::extension_picks`). Open from it: carry sim picks in `RaceParams` once a network race loads modules; a MODDED race records no replay.
6. ~~Question: should the firer see the window?~~ Done, the guest's approach phase. The portal and
   its sound open at the press, the craft keeps flying and goes through when its progress reaches
   the entrance (about 13 ticks at 500 km/h on Pulse's `16_Track`); the pickup is spent there, and the
   fill, FOV jump and hum start there. **Chosen, not measured:** a craft that has not arrived 50 ticks
   after the press (stopped, wall-hit, wrecked, pickup lost) closes the portal and keeps the pickup;
   50 is under the portal effect's 60 tick life. The exit stays where the view looks (entrance
   plus four seconds of the press speed); a craft that arrives with the exit not ahead of it (a
   press at a standstill) closes the portal and keeps the pickup too. No course fix: through on the
   press, as before. Another
   craft cannot enter a held portal (the transit is per craft); a fire while the portal is held queues
   as before. The AI goes through the same path (`the_ai_fires_it_and_only_one_portal_is_open_at_a_time`).
   Seen at the player's size on `16_Track`: window ahead for about 13 ticks with no fill, then the
   craft is gone at the entrance and the fill ramps over 10 ticks. At 500 km/h the window is on
   screen for about 0.2 s before the fill.

## The flight through the tunnel (2026-11-10)

A person's own flight through the portal is one move with no cut, and the portal is the tunnel
(maintainer's picture, 2026-11-10, in three steps: a short tunnel whose border stretches with the
craft's speed; the tunnel has the portal's shape, is entered, the exit moves back, is driven
through and left through the portal; then by questions: half-disc arch, the ship visible
throughout, flown in through the mouth, the exit back then closed on steadily). `fx/wormhole.wgsl`
cuts the portal's half disc, drawn out round the craft, by each texel's ray; the guest's `Flight`
says how long the craft has been through and how fast it flies (README, "What you see of your
own flight"). The field of view rides one curve (24 degrees at its widest), no step at either end.
What the engine gained for it, all title-blind (`mod-shaders.md`, "A craft's own transit" and
"A backdrop"):

- **A transit comes out at flying height** (`breadcrumbs::emerge`: the reset's placement without
  its 5 unit drop). Before, the craft fell for about 15 ticks after every exit. A simulation
  change, for modded races only: a vanilla race has no transit.
- **The camera** of a craft in a transit flies on through where the craft went, along the exit's
  framing carried back to the entrance (its seat, `transit_camera`), and is seated behind the
  craft on the exit tick (`RaceView::camera_jumps`; the spring used to sweep it across the jump
  for about 15 ticks). A `portal-view` of that slot is seen from the seat, so the view through
  the craft's own pair is the camera's own picture at the exit (0.12 units and 0.0002 apart on
  Pulse `16_Track`, from 3.5 units and 17 degrees).
- **A backdrop fill** (`fill = "backdrop"` in a shader's header): drawn before the crafts and in
  place of the world where it is at least half there, with the view's own craft drawn in front
  of it, in a transit too (`Race::ship_drawn`, `Draw::Backdrop`, `Draw::Depth`).
- **`craft_frame()` and `eye_ray()`** for a shader to cut geometry round the craft,
  **`input.between`** for a scalar that moves, and **`fov-offset` drawn between ticks**.

Looked at frame by frame at 1280x720 on Pulse PSP EU `16_Track` (High and Low), HD, Omega and
2048, autopilot, fire at tick 700: entrance 714, exit 774 on Pulse. Open from it, each small:

1. **Nobody has watched it move.** Every judgement is off stills a tick or a few apart; how long
   the exit moves back for (`WH_BACK`), the rings' spacing (`WH_RINGS`: 12 units is 0.19 of a
   ring a frame at 490 km/h, and strobes above about 1,300) and the camera's swing onto the
   tunnel's line in its first 12 ticks want a pair of eyes at 60 Hz and above.
2. **The flown craft has no exhaust**: its hull and flame shapes are drawn (HD shows its flame),
   not the boost plume, the particle exhaust or the trail, which are stepped and drawn for a
   craft in the world (Pulse's plume is one). The shader lights the floor behind the craft
   instead. Drawing them is an engine piece: `advance_exhausts` and the effect stage's one draw
   for every craft.
3. **The far end's picture has no post chain** (no bloom, no shadows, 960x540), so on HD it is a
   little flatter than the frame after it.
4. **Low and the second seat of a split** have light at the far end and go from it to the road on
   the exit tick: a whiteout, not a match.
5. **Medium's far end changes picture 16 ticks in**, from the snapshot the portal opened with to
   the one taken once the camera is on its line.
6. **The portal's sprite is a billboard** and the tunnel's mouth a plane across the road: seen
   from off the road's line the two outlines differ for the tick or two before the camera is
   through. A mesh portal (the thread's oldest open item) ends it.
7. **A backdrop hides the world's later layers by depth**, so what draws with no depth test
   (the mist, the screen flash, the weather) still draws over it; not looked for.
8. **The extra seats of a split** get the same camera and backdrop; not looked at.
9. **The sounds were not touched**: the three synthesised placeholders still start and stop on
   the entrance and the exit, with no rise into the tunnel.
10. `check-size` names `crates/game/src/race_capture.rs` at 1501 lines, with and without this
    change. The `race-build` stack overflow that two commit messages of 2026-11-10 call red on
    the branch is not a defect of it: a disc-backed test that spawns the game overflows that
    thread under `--cargo-profile quick` (opt-level 0) and passes under the ground-truth
    profile (`drive_ground_truth::a_race_box_race_runs_the_simulation_mods_only_with_the_extensions_feature`
    and `pause_menu_ground_truth::a_speed_lap_pauses_into_the_speed_lap_screen`, both checked).

## SVG icons (lane svg-assets, 2026-11-04)

A mod's `[icons]` accepts `.svg` beside `.png` (`oag_hud::svg`, `Assets::rasterise_vector_icons`,
`oag_game::hud_overlay`). `oag_raceplay::extensions::mod_icons` parses an SVG at load (a torn
one is named by file and parser error, draws nothing); the pickup slot's size in layout units
comes from the same fit `mod_icon_draw` uses (`oag_hud::mod_icon_size`), times the pixels per
layout unit the renderer measures; the raster is stored in the HUD sheet with its texel density
and drawn one texel to one pixel. The race load rasterises at one pixel per unit; the overlay
rasterises again on a worker thread (`oag_thread::Task`) when its measured size or Safe Area
changes and swaps the sheet in; `--screenshot` rasterises synchronously. `resvg` is built
without `text` and `raster-images`: `<text>` draws nothing, `<image>` is never read. Example:
`oag.energy-turbo`'s `icons/energy.svg`. Open: a split-screen window rasterises for the whole
viewport, so a view scaled down samples a larger raster; windows of different sizes sharing
one overlay never start a raster (a change must be asked for in two draws running);
under `hud_scale = "integer"` at a non-integer factor (1280x800 Pulse) the title's backdrop
snaps down to a whole factor while the icon keeps its exact measured size, so it sits low and
right in the smaller hex (the raster itself stays one texel to one pixel; a PNG was not compared); the fallback-layer HUD art
(`assets/`) does not use SVG; `oag.slipstream` keeps its PNGs on purpose.

## Next Steps

1. Open item 3 (an island exit) is a design question first; the recommended rule is the author's
   own spline path that rejoins the circuit, which a trigger exit on that route already reads.
2. ~~Make `portal-view` route-aware.~~ Landed (exit by trigger). A route-aware entrance, for a firer on a route, is open and small.
3. Quote `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
   before and after any change that touches racing.
4. **The next merge of main into this branch will not compile as is.** Main's `give` replay event
   (`give-outside-tick`, origin/main 373c562d1) stores a single `Weapon::Extension` as index 14
   (`STORED_WEAPONS` in `crates/raceplay/src/replay/events.rs`); here a mod adds more than one pickup
   (`Weapon::Extension(kind)`, hashed as `u8::MAX - kind.index()`). Resolve it by storing
   `Extension(k)` as `14 + k.index()` (the event's `u8` payload holds it), extend the event's unit test
   to more than one kind, and update `docs/formats/replay-file.md` (v1 in place).
